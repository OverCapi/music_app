//! Downloads songs from YouTube / YouTube Music with yt-dlp (audio only, Opus,
//! with tags and cover art), then adds them to the library.
//!
//! Progress is pushed to the UI as `download-progress` events, and
//! `library-changed` is emitted once a file has been added.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Semaphore;

use crate::library::Library;
use crate::youtube::{find_in_path, ytdlp_path};

pub const PROGRESS_EVENT: &str = "download-progress";
pub const LIBRARY_EVENT: &str = "library-changed";
const MAX_PARALLEL: usize = 2;
/// YouTube sometimes answers 403 on the first try; one retry is usually enough.
const ATTEMPTS: usize = 2;

#[derive(Clone, Copy, Serialize)]
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

#[derive(Clone, Serialize)]
struct DownloadProgress {
    id: String,
    status: DownloadStatus,
    /// 0.0–1.0 while downloading, when the size is known.
    progress: Option<f64>,
    error: Option<String>,
}

pub struct Downloads {
    dir: PathBuf,
    permits: Arc<Semaphore>,
    active: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
}

impl Downloads {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            permits: Arc::new(Semaphore::new(MAX_PARALLEL)),
            active: Arc::default(),
        }
    }

    pub fn start(&self, app: AppHandle, id: String, url: String) -> Result<(), String> {
        if !is_youtube_url(&url) {
            return Err("Lien YouTube invalide".to_owned());
        }
        if app.state::<Library>().has_youtube_id(&id) {
            return Err("Ce titre est déjà dans la bibliothèque".to_owned());
        }
        // Held while spawning, so the task can't remove its entry before it is inserted.
        let mut active = self.active.lock().unwrap();
        if active.contains_key(&id) {
            return Ok(());
        }
        emit(&app, &id, DownloadStatus::Queued, None, None);

        let task = tauri::async_runtime::spawn({
            let (app, id, dir) = (app.clone(), id.clone(), self.dir.clone());
            let (permits, active) = (Arc::clone(&self.permits), Arc::clone(&self.active));
            async move {
                let _permit = permits.acquire_owned().await;
                let mut result = Err(String::new());
                for _ in 0..ATTEMPTS {
                    result = download(&url, &dir, |status, progress| {
                        emit(&app, &id, status, progress, None)
                    })
                    .await;
                    if result.is_ok() {
                        break;
                    }
                }
                let result =
                    result.and_then(|path| app.state::<Library>().add_downloaded(&path, &id));
                match result {
                    Ok(()) => {
                        let _ = app.emit(LIBRARY_EVENT, ());
                        emit(&app, &id, DownloadStatus::Done, None, None);
                    }
                    Err(e) => emit(&app, &id, DownloadStatus::Error, None, Some(e)),
                }
                active.lock().unwrap().remove(&id);
            }
        });
        active.insert(id, task);
        Ok(())
    }

    pub fn cancel(&self, app: &AppHandle, id: &str) {
        if let Some(task) = self.active.lock().unwrap().remove(id) {
            // Aborting drops the yt-dlp child process, which kills it (kill_on_drop).
            task.abort();
            emit(app, id, DownloadStatus::Cancelled, None, None);
        }
    }
}

/// Runs one yt-dlp download and returns the final file path.
async fn download(
    url: &str,
    dir: &Path,
    on_progress: impl Fn(DownloadStatus, Option<f64>),
) -> Result<String, String> {
    let mut cmd = Command::new(ytdlp_path());
    cmd.arg(url)
        .args(["--no-playlist", "--no-warnings", "--progress", "--newline"])
        .args(["-x", "--audio-format", "opus", "--embed-metadata", "--embed-thumbnail"])
        .arg("-P")
        .arg(dir)
        .args(["-o", "%(title)s [%(id)s].%(ext)s"])
        .args([
            "--progress-template",
            "download:PROGRESS %(progress.downloaded_bytes)s %(progress.total_bytes,progress.total_bytes_estimate)s %(progress.status)s",
            "--print",
            "after_move:FILE %(filepath)s",
        ]);
    // Recent YouTube extraction wants a JS runtime; yt-dlp only looks for deno by default.
    if let Some(node) = find_in_path("node") {
        cmd.arg("--js-runtimes")
            .arg(format!("node:{}", node.display()));
    }
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Impossible de lancer yt-dlp : {e}"))?;

    let mut stderr = child.stderr.take().expect("stderr is piped");
    let stderr_task = tauri::async_runtime::spawn(async move {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text).await;
        text
    });

    let mut lines = BufReader::new(child.stdout.take().expect("stdout is piped")).lines();
    let mut file = None;
    let mut last_emitted = -1.0;
    while let Ok(Some(line)) = lines.next_line().await {
        if let Some(path) = line.strip_prefix("FILE ") {
            file = Some(path.to_owned());
        } else if let Some(progress) = line.strip_prefix("PROGRESS ") {
            let mut parts = progress.split_whitespace();
            let done = parts.next().and_then(|v| v.parse::<f64>().ok());
            let total = parts.next().and_then(|v| v.parse::<f64>().ok());
            if parts.next() == Some("finished") {
                on_progress(DownloadStatus::Converting, None);
                continue;
            }
            let fraction = done.zip(total).map(|(d, t)| (d / t).clamp(0.0, 1.0));
            // Throttle: one event per percent at most.
            if fraction.is_none_or(|f| f - last_emitted >= 0.01) {
                last_emitted = fraction.unwrap_or(last_emitted);
                on_progress(DownloadStatus::Downloading, fraction);
            }
        }
    }

    let status = child.wait().await.map_err(|e| e.to_string())?;
    let stderr = stderr_task.await.unwrap_or_default();
    if !status.success() {
        let reason = stderr.lines().rev().find(|l| !l.trim().is_empty());
        return Err(format!(
            "Le téléchargement a échoué : {}",
            reason.unwrap_or("erreur inconnue de yt-dlp")
        ));
    }
    file.ok_or_else(|| "yt-dlp n'a pas indiqué le fichier téléchargé".to_owned())
}

fn is_youtube_url(url: &str) -> bool {
    [
        "https://www.youtube.com/",
        "https://music.youtube.com/",
        "https://youtube.com/",
    ]
    .iter()
    .any(|prefix| url.starts_with(prefix))
}

fn emit(
    app: &AppHandle,
    id: &str,
    status: DownloadStatus,
    progress: Option<f64>,
    error: Option<String>,
) {
    let _ = app.emit(
        PROGRESS_EVENT,
        DownloadProgress {
            id: id.to_owned(),
            status,
            progress,
            error,
        },
    );
}
