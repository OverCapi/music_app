//! Downloads songs from YouTube with yt-dlp (`-x --audio-format opus
//! --embed-metadata --embed-thumbnail`) into the app music folder, then adds them
//! to the library. At most 2 in parallel, one retry (YouTube sometimes answers 403).
//!
//! Progress is pushed as [`DownloadProgress`] events; the library emits its own
//! change event once the file is added. [`download`] is reused by the group client.

pub mod commands;

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::AppHandle;

use crate::error::AppResult;
use crate::library::Library;

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
    #[allow(dead_code)]
    dir: PathBuf,
    // Filled in by the implementation (semaphore, active tasks).
}

impl Downloads {
    /// `dir`: the app music folder (`~/Music/Music App`).
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// Starts downloading `youtube_id` (validated) into the music folder; when done
    /// the file is added to `library`. No-op if it is already downloading.
    pub fn start(&self, _app: AppHandle, _library: Library, _youtube_id: String) -> AppResult<()> {
        todo!()
    }

    pub fn cancel(&self, _app: &AppHandle, _youtube_id: &str) {
        todo!()
    }

    /// Moves `path` to the trash if it is inside the music folder. Returns whether
    /// it did.
    pub fn trash_if_owned(&self, _path: &str) -> AppResult<bool> {
        todo!()
    }
}

/// Downloads `youtube_id` as Opus into `dir`, retrying once, reporting progress
/// (0.0–1.0) through `on_progress`. Returns the path of the file.
pub async fn download(
    _youtube_id: &str,
    _dir: &Path,
    _on_progress: impl FnMut(DownloadStatus, Option<f64>) + Send,
) -> AppResult<PathBuf> {
    todo!()
}
