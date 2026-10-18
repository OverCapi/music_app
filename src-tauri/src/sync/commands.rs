use tauri::State;

use super::{LibrarySync, SpotifyImportReport, SyncReport, SyncSettings};
use crate::error::AppResult;

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

/// Imports a Spotify liked-songs TSV (`n, titre, artiste, durée`); progress comes as
/// `SyncProgress` events.
#[tauri::command]
#[specta::specta]
pub async fn import_spotify_tsv(
    sync: State<'_, LibrarySync>,
    path: String,
) -> AppResult<SpotifyImportReport> {
    let sync = sync.inner().clone();
    sync.import_spotify_tsv(&path).await
}
