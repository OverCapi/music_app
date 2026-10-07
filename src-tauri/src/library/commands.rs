use tauri::State;

use super::{Library, Track, TrackId};
use crate::audio::AudioHandle;
use crate::downloads::Downloads;
use crate::error::AppResult;
use crate::playlists::Playlists;

#[tauri::command]
#[specta::specta]
pub fn get_library(library: State<'_, Library>) -> AppResult<Vec<Track>> {
    library.tracks()
}

/// Async so reading the tags of many files doesn't block the main thread.
#[tauri::command]
#[specta::specta]
pub async fn add_tracks(library: State<'_, Library>, paths: Vec<String>) -> AppResult<Vec<Track>> {
    library.add_paths(&paths)
}

/// Removes a track from the library, the playlists and the queue. A file inside
/// the app music folder is moved to the trash first (if that fails, nothing changes).
#[tauri::command]
#[specta::specta]
pub fn remove_track(
    library: State<'_, Library>,
    playlists: State<'_, Playlists>,
    downloads: State<'_, Downloads>,
    audio: State<'_, AudioHandle>,
    id: TrackId,
) -> AppResult<()> {
    let track = library.get(id)?;
    downloads.trash_if_owned(&track.path)?;
    library.remove(id)?;
    audio.forget_track(id);
    playlists.notify_changed()
}
