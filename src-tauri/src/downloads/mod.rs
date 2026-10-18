//! Downloads songs from YouTube with yt-dlp (`-x --audio-format opus
//! --embed-metadata --embed-thumbnail`) into the app music folder, then adds them
//! to the library. At most 2 in parallel, one retry (YouTube sometimes answers 403).
//!
//! Progress is pushed as [`DownloadProgress`] events; the library emits its own
//! change event once the file is added. [`download`] is reused by the group client.
//!
//! Suggested (recommended, not kept) tracks are downloaded into a cache folder
//! instead; liking one ([`Downloads::keep`]) moves its file into the music folder.
//! [`Downloads::purge_suggestions`] empties the cache at startup.

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
use crate::library::{Library, Track};
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
    /// Files of suggested tracks, wiped at startup.
    cache_dir: PathBuf,
    permits: Arc<Semaphore>,
    active: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
}

impl Downloads {
    /// `dir`: the app music folder (`~/Music/Music App`); `cache_dir`: where suggested
    /// tracks are downloaded.
    pub fn new(dir: PathBuf, cache_dir: PathBuf) -> Self {
        Self {
            dir,
            cache_dir,
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
                // An explicit download means the user wants to keep the track.
                let result = result.and_then(|track| library.promote(&id).map(|_| track));
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
        let (dir, cache_dir) = (self.dir.clone(), self.cache_dir.clone());
        let permits = Arc::clone(&self.permits);
        async move {
            if let Some(path) = library
                .find_by_youtube_id(&youtube_id)?
                .and_then(|t| t.path)
                .filter(|p| Path::new(p).is_file())
            {
                return Ok(path);
            }
            let suggested = match library.find_by_youtube_id(&youtube_id)? {
                Some(track) => library.is_suggested(track.id)?,
                None => false,
            };
            let target = if suggested { &cache_dir } else { &dir };
            emit(&app, &youtube_id, DownloadStatus::Queued, None, None);
            let _permit = permits.acquire_owned().await;
            let result: AppResult<Track> = async {
                std::fs::create_dir_all(target)?;
                let path = download(&youtube_id, target, |status, progress| {
                    emit(&app, &youtube_id, status, progress, None)
                })
                .await?;
                let mut track = library.add_downloaded(&path.to_string_lossy(), &youtube_id)?;
                // Liked while downloading: the file belongs in the music folder.
                if suggested && !library.is_suggested(track.id)? {
                    move_out_of_cache(&library, &track, &dir, &cache_dir)?;
                    track = library.get(track.id)?;
                }
                Ok(track)
            }
            .await;
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
        let (dirs, id) = (
            [self.dir.clone(), self.cache_dir.clone()],
            youtube_id.to_owned(),
        );
        tauri::async_runtime::spawn(async move {
            let _ = task.await;
            for dir in &dirs {
                remove_partial_files(dir, &id);
            }
        });
    }

    /// The user likes the track `youtube_id`: it becomes a normal library track and
    /// its cached file, if any, moves into the music folder. No-op for a track that
    /// is not in the library. The track is promoted even if the move fails.
    pub fn keep(&self, library: &Library, youtube_id: &str) -> AppResult<()> {
        let Some(track) = library.find_by_youtube_id(youtube_id)? else {
            return Ok(());
        };
        let moved = move_out_of_cache(library, &track, &self.dir, &self.cache_dir);
        library.promote(youtube_id)?;
        moved
    }

    /// Startup cleanup: empties the suggestion cache and deletes every suggested
    /// track that was never kept, so suggestions don't accumulate across sessions.
    pub fn purge_suggestions(&self, library: &Library) -> AppResult<()> {
        match std::fs::remove_dir_all(&self.cache_dir) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => tracing::warn!("cannot empty {}: {e}", self.cache_dir.display()),
        }
        for (id, _, _) in library.suggested_tracks()? {
            library.remove_suggested(id)?;
        }
        Ok(())
    }

    /// The canonical path of `path` if it is an existing file inside the music folder.
    pub fn owned_file(&self, path: &str) -> Option<PathBuf> {
        let file = Path::new(path).canonicalize().ok()?;
        let dir = self.dir.canonicalize().ok()?;
        (file.starts_with(dir) && file.is_file()).then_some(file)
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

/// Moves the file of `track` into `dir` if it lives in `cache_dir` (rename, else
/// copy + delete) and points the track at the new path.
fn move_out_of_cache(
    library: &Library,
    track: &Track,
    dir: &Path,
    cache_dir: &Path,
) -> AppResult<()> {
    let Some(path) = track.path.as_deref().map(Path::new) else {
        return Ok(());
    };
    if !path.starts_with(cache_dir) || !path.is_file() {
        return Ok(());
    }
    let io = |e: std::io::Error| AppError::Io(format!("impossible de conserver le fichier : {e}"));
    let name = path
        .file_name()
        .ok_or_else(|| AppError::Io("nom de fichier invalide".into()))?;
    std::fs::create_dir_all(dir).map_err(io)?;
    let dest = dir.join(name);
    if dest.exists() {
        // Already in the music folder: just drop the cached copy.
        std::fs::remove_file(path).map_err(io)?;
    } else if std::fs::rename(path, &dest).is_err() {
        std::fs::copy(path, &dest).map_err(io)?;
        if let Err(e) = std::fs::remove_file(path) {
            tracing::warn!("cannot remove {}: {e}", path.display());
        }
    }
    library.set_path(track.id, &dest.to_string_lossy())
}

/// Downloads `youtube_id` as Opus into `dir`, retrying once, reporting progress
/// (0.0–1.0) through `on_progress`. Returns the path of the file.
pub async fn download(
    youtube_id: &str,
    dir: &Path,
    mut on_progress: impl FnMut(DownloadStatus, Option<f64>) + Send,
) -> AppResult<PathBuf> {
    let url = ytdlp::video_url(youtube_id)?;
    let args: [&OsStr; 13] = [
        url.as_ref(),
        "--no-playlist".as_ref(),
        // A live never ends, so it could never finish downloading.
        "--break-match-filters".as_ref(),
        "!is_live".as_ref(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use crate::library::RemoteTrack;
    use crate::library::tests::temp_dir;

    const ID: &str = "aaaaaaaaaaa";

    fn suggest(lib: &Library, id: &str) -> Track {
        let ids = lib
            .add_suggested(&[RemoteTrack {
                title: id.into(),
                artist: None,
                album: None,
                duration: 1.0,
                youtube_id: Some(id.into()),
            }])
            .unwrap();
        lib.get(ids[0]).unwrap()
    }

    fn setup() -> (Downloads, Library, PathBuf, PathBuf) {
        let root = temp_dir("downloads");
        let (music, cache) = (root.join("music"), root.join("cache"));
        std::fs::create_dir_all(&cache).unwrap();
        (
            Downloads::new(music.clone(), cache.clone()),
            Library::new(Db::in_memory(), None),
            music,
            cache,
        )
    }

    #[test]
    fn keep_moves_cached_file_and_promotes() {
        let (downloads, lib, music, cache) = setup();
        let track = suggest(&lib, ID);
        let cached = cache.join("Song [aaaaaaaaaaa].opus");
        std::fs::write(&cached, b"audio").unwrap();
        lib.set_path(track.id, &cached.to_string_lossy()).unwrap();

        downloads.keep(&lib, ID).unwrap();

        let kept = music.join("Song [aaaaaaaaaaa].opus");
        assert!(kept.is_file() && !cached.exists());
        assert!(!lib.is_suggested(track.id).unwrap());
        let tracks = lib.tracks().unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].path.as_deref(), Some(&*kept.to_string_lossy()));
    }

    #[test]
    fn keep_promotes_a_track_without_file_and_ignores_unknown_ones() {
        let (downloads, lib, music, _) = setup();
        let track = suggest(&lib, ID);
        downloads.keep(&lib, ID).unwrap();
        downloads.keep(&lib, "bbbbbbbbbbb").unwrap();
        assert_eq!(lib.tracks().unwrap().len(), 1);
        assert!(lib.get(track.id).unwrap().path.is_none());
        assert!(!music.exists());
    }

    #[test]
    fn purge_removes_unkept_suggestions_and_the_cache() {
        let (downloads, lib, _, cache) = setup();
        let dropped = suggest(&lib, ID);
        let kept = suggest(&lib, "bbbbbbbbbbb");
        let normal = crate::library::tests::insert_track(&lib, "/a.opus");
        std::fs::write(cache.join("x.opus"), b"audio").unwrap();
        lib.promote("bbbbbbbbbbb").unwrap();

        downloads.purge_suggestions(&lib).unwrap();

        assert!(!cache.exists());
        assert!(lib.get(dropped.id).is_err());
        assert!(lib.get(kept.id).is_ok());
        assert!(lib.get(normal).is_ok());
        assert!(lib.suggested_tracks().unwrap().is_empty());
    }
}
