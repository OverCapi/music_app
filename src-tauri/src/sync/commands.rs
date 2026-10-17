use tauri::State;

use super::{LibrarySync, SyncReport, SyncSettings};
use crate::error::{AppError, AppResult};

#[tauri::command]
#[specta::specta]
pub fn get_sync_settings(sync: State<'_, LibrarySync>) -> AppResult<SyncSettings> {
    sync.settings()
}

#[tauri::command]
#[specta::specta]
pub fn set_ytmusic_browser(
    sync: State<'_, LibrarySync>,
    browser: String,
) -> AppResult<SyncSettings> {
    sync.set_ytmusic_browser(&browser)
}

/// Imports the YouTube Music liked songs and playlists; progress comes as
/// `SyncProgress` events.
#[tauri::command]
#[specta::specta]
pub async fn sync_youtube_music(sync: State<'_, LibrarySync>) -> AppResult<SyncReport> {
    let sync = sync.inner().clone();
    sync.sync_youtube_music().await
}

/// Imports the Spotify data export (`paths`: the `.zip` or JSON files from it);
/// progress comes as `SyncProgress` events.
#[tauri::command]
#[specta::specta]
pub async fn import_spotify(
    sync: State<'_, LibrarySync>,
    paths: Vec<String>,
) -> AppResult<SyncReport> {
    let sync = sync.inner().clone();
    tauri::async_runtime::spawn_blocking(move || sync.import_spotify(&paths))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
}
