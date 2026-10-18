//! Group cache dir `<cache>/groups/<CODE>/`: wiped at startup, deleted on leave/quit.

use std::path::{Path, PathBuf};
use std::time::Duration;

/// Left to yt-dlp's children (ffmpeg) to die before the folder is removed again.
const CLEANUP_DELAY: Duration = Duration::from_secs(3);

pub fn remove_dir(dir: &Path) {
    match std::fs::remove_dir_all(dir) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            tracing::warn!("group: cannot remove {}: {e}", dir.display());
        }
        _ => {}
    }
}

/// Removes `dir` now and again a bit later (a killed download may still write for
/// a moment), unless `keep()` says it is in use again by then.
pub fn remove_dir_twice(dir: PathBuf, keep: impl FnOnce() -> bool + Send + 'static) {
    remove_dir(&dir);
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(CLEANUP_DELAY).await;
        if !keep() {
            remove_dir(&dir);
        }
    });
}
