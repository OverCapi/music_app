use std::collections::HashSet;

use tauri::State;

use super::{Storage, StorageReport, StorageSettings, StorageTrack};
use crate::audio::AudioHandle;
use crate::downloads::Downloads;
use crate::error::AppResult;
use crate::library::{Library, TrackId};

/// Tracks of the play queue (including the current one): their file stays.
pub fn queued_tracks(audio: &AudioHandle) -> HashSet<TrackId> {
    audio.queue().items.iter().map(|i| i.track_id).collect()
}

#[tauri::command]
#[specta::specta]
pub fn get_storage_settings(storage: State<'_, Storage>) -> AppResult<StorageSettings> {
    storage.settings()
}

#[tauri::command]
#[specta::specta]
pub fn set_storage_settings(
    storage: State<'_, Storage>,
    settings: StorageSettings,
) -> AppResult<StorageSettings> {
    storage.set_settings(&settings)
}

/// Async: stats every downloaded file.
#[tauri::command]
#[specta::specta]
pub async fn list_storage_tracks(
    storage: State<'_, Storage>,
    downloads: State<'_, Downloads>,
    audio: State<'_, AudioHandle>,
) -> AppResult<Vec<StorageTrack>> {
    storage.list(&downloads, &queued_tracks(&audio))
}

/// Deletes the audio files of the tracks not played for more than the configured days.
#[tauri::command]
#[specta::specta]
pub async fn optimize_storage(
    storage: State<'_, Storage>,
    downloads: State<'_, Downloads>,
    audio: State<'_, AudioHandle>,
) -> AppResult<StorageReport> {
    storage.optimize(&downloads, &queued_tracks(&audio))
}

/// Deletes the audio file of the chosen tracks (they stay in the library).
#[tauri::command]
#[specta::specta]
pub async fn delete_track_files(
    storage: State<'_, Storage>,
    downloads: State<'_, Downloads>,
    audio: State<'_, AudioHandle>,
    ids: Vec<TrackId>,
) -> AppResult<StorageReport> {
    storage.delete_files(&ids, &downloads, &queued_tracks(&audio))
}

#[tauri::command]
#[specta::specta]
pub fn set_favorite(library: State<'_, Library>, id: TrackId, favorite: bool) -> AppResult<()> {
    library.set_favorite(id, favorite)
}
