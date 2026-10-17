//! Downloads songs from YouTube with yt-dlp (`-x --audio-format opus
//! --embed-metadata --embed-thumbnail`) into the app music folder, then adds them
//! to the library. At most 2 in parallel, one retry (YouTube sometimes answers 403).
//!
//! Progress is pushed as [`DownloadProgress`] events; the library emits its own
//! change event once the file is added. [`download`] is reused by the group client.

pub mod commands;

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::AppHandle;
use tauri::async_runtime::JoinHandle;
use tauri_specta::Event;
use tokio::sync::Semaphore;

use crate::error::{AppError, AppResult};
use crate::library::Library;
use crate::ytdlp::{self, Progress};

const MAX_PARALLEL: usize = 2;
/// YouTube sometimes answers 403 on the first try; one retry is usually enough.
const ATTEMPTS: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStatus {
    Queued,
    Downloading,
    /// Extracting audio and embedding tags/cover (ffmpeg).
    Converting,
    Done,
    Error,
    Cancelled,
}

#[derive(Clone, Debug, Serialize, specta::Type, tauri_specta::Event)]
pub struct DownloadProgress {
    /// YouTube id.
    pub id: String,
    pub status: DownloadStatus,
    /// 0.0–1.0 while downloading, when the size is known.
    pub progress: Option<f64>,
    pub error: Option<String>,
}

/// Managed state.
pub struct Downloads {
    dir: PathBuf,
    permits: Arc<Semaphore>,
    active: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
}

impl Downloads {
    /// `dir`: the app music folder (`~/Music/Music App`).
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            permits: Arc::new(Semaphore::new(MAX_PARALLEL)),
            active: Arc::default(),
        }
    }

    /// Starts downloading `youtube_id` (validated) into the music folder; when done
    /// the file is added to `library`. No-op if it is already downloading.
    pub fn start(&self, app: AppHandle, library: Library, youtube_id: String) -> AppResult<()> {
        ytdlp::video_url(&youtube_id)?;
        // A track whose file is gone may be downloaded again (`add_downloaded` updates it).
        let existing = library.find_by_youtube_id(&youtube_id)?;
        if existing.is_some_and(|t| t.path.is_some_and(|p| Path::new(&p).is_file())) {
            return Err(AppError::invalid("Ce titre est déjà dans la bibliothèque"));
        }
        // Held while spawning, so the task can't remove its entry before it is inserted.
        let mut active = self.active.lock().unwrap();
        if active.contains_key(&youtube_id) {
            return Ok(());
        }
        emit(&app, &youtube_id, DownloadStatus::Queued, None, None);

        let task = tauri::async_runtime::spawn({
            let (id, dir) = (youtube_id.clone(), self.dir.clone());
            let (permits, active) = (Arc::clone(&self.permits), Arc::clone(&self.active));
            async move {
                let _permit = permits.acquire_owned().await;
                let result = download(&id, &dir, |status, progress| {
                    emit(&app, &id, status, progress, None)
                })
                .await
                .and_then(|path| library.add_downloaded(&path.to_string_lossy(), &id));
                match result {
                    Ok(_) => emit(&app, &id, DownloadStatus::Done, None, None),
                    Err(e) => emit(&app, &id, DownloadStatus::Error, None, Some(e.to_string())),
                }
                active.lock().unwrap().remove(&id);
            }
        });
        active.insert(youtube_id, task);
        Ok(())
    }

    /// Path of the file of the library track with `youtube_id`, downloading it
    /// first if needed (with progress events, like [`Downloads::start`]). Used to
    /// play a track that is in the library without a file.
    pub fn fetch(
        &self,
        app: AppHandle,
        library: Library,
        youtube_id: String,
    ) -> impl Future<Output = AppResult<String>> + Send + 'static {
        let (dir, permits) = (self.dir.clone(), Arc::clone(&self.permits));
        async move {
            if let Some(path) = library
                .find_by_youtube_id(&youtube_id)?
                .and_then(|t| t.path)
                .filter(|p| Path::new(p).is_file())
            {
                return Ok(path);
            }
            emit(&app, &youtube_id, DownloadStatus::Queued, None, None);
            let _permit = permits.acquire_owned().await;
            let result = download(&youtube_id, &dir, |status, progress| {
                emit(&app, &youtube_id, status, progress, None)
            })
            .await
            .and_then(|path| library.add_downloaded(&path.to_string_lossy(), &youtube_id));
            match result {
                Ok(track) => {
                    emit(&app, &youtube_id, DownloadStatus::Done, None, None);
                    track
                        .path
                        .ok_or_else(|| AppError::Internal("downloaded track has no path".into()))
                }
                Err(e) => {
                    emit(
                        &app,
                        &youtube_id,
                        DownloadStatus::Error,
                        None,
                        Some(e.to_string()),
                    );
                    Err(e)
                }
            }
        }
    }

    pub fn cancel(&self, app: &AppHandle, youtube_id: &str) {
        let Some(task) = self.active.lock().unwrap().remove(youtube_id) else {
            return;
        };
        // Aborting drops the yt-dlp child process, which kills it (kill_on_drop).
        task.abort();
        emit(app, youtube_id, DownloadStatus::Cancelled, None, None);
        let (dir, id) = (self.dir.clone(), youtube_id.to_owned());
        tauri::async_runtime::spawn(async move {
            let _ = task.await;
            remove_partial_files(&dir, &id);
        });
    }

    /// Moves `path` to the trash if it is inside the music folder. Returns whether
    /// it did.
    pub fn trash_if_owned(&self, path: &str) -> AppResult<bool> {
        let (Ok(file), Ok(dir)) = (Path::new(path).canonicalize(), self.dir.canonicalize()) else {
            // Missing file (or music folder): nothing to delete.
            return Ok(false);
        };
        if !file.starts_with(&dir) {
            return Ok(false);
        }
        trash::delete(&file)
            .map(|()| true)
            .map_err(|e| AppError::Io(format!("impossible de supprimer le fichier : {e}")))
    }
}

/// Downloads `youtube_id` as Opus into `dir`, retrying once, reporting progress
/// (0.0–1.0) through `on_progress`. Returns the path of the file.
pub async fn download(
    youtube_id: &str,
    dir: &Path,
    mut on_progress: impl FnMut(DownloadStatus, Option<f64>) + Send,
) -> AppResult<PathBuf> {
    let url = ytdlp::video_url(youtube_id)?;
    let args: [&OsStr; 11] = [
        url.as_ref(),
        "--no-playlist".as_ref(),
        "-x".as_ref(),
        "--audio-format".as_ref(),
        "opus".as_ref(),
        "--embed-metadata".as_ref(),
        "--embed-thumbnail".as_ref(),
        "-P".as_ref(),
        dir.as_os_str(),
        "-o".as_ref(),
        "%(title)s [%(id)s].%(ext)s".as_ref(),
    ];
    let mut result = Err(AppError::Internal(String::new()));
    for attempt in 1..=ATTEMPTS {
        result = ytdlp::download(&args, |p| match p {
            Progress::Downloading(f) => on_progress(DownloadStatus::Downloading, f),
            Progress::Converting => on_progress(DownloadStatus::Converting, None),
        })
        .await;
        match &result {
            Ok(_) => break,
            Err(e) => tracing::warn!("download {youtube_id} (attempt {attempt}): {e}"),
        }
    }
    result
}

/// Leftovers of an interrupted download of `youtube_id` (the output template puts
/// `[<id>]` in every file name).
fn remove_partial_files(dir: &Path, youtube_id: &str) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let marker = format!("[{youtube_id}].");
    for path in entries.flatten().map(|e| e.path()) {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let partial = name.ends_with(".part") || name.ends_with(".ytdl") || name.contains(".part-");
        if name.contains(&marker)
            && partial
            && let Err(e) = std::fs::remove_file(&path)
        {
            tracing::warn!("cannot remove {}: {e}", path.display());
        }
    }
}

fn emit(
    app: &AppHandle,
    id: &str,
    status: DownloadStatus,
    progress: Option<f64>,
    error: Option<String>,
) {
    let _ = DownloadProgress {
        id: id.to_owned(),
        status,
        progress,
        error,
    }
    .emit(app);
}
