use tauri::State;

use super::{Playlist, PlaylistId, Playlists};
use crate::error::AppResult;
use crate::library::TrackId;

#[tauri::command]
#[specta::specta]
pub fn get_playlists(playlists: State<'_, Playlists>) -> AppResult<Vec<Playlist>> {
    playlists.all()
}

#[tauri::command]
#[specta::specta]
pub fn create_playlist(playlists: State<'_, Playlists>, name: String) -> AppResult<Playlist> {
    playlists.create(&name)
}

#[tauri::command]
#[specta::specta]
pub fn rename_playlist(
    playlists: State<'_, Playlists>,
    id: PlaylistId,
    name: String,
) -> AppResult<()> {
    playlists.rename(id, &name)
}

#[tauri::command]
#[specta::specta]
pub fn delete_playlist(playlists: State<'_, Playlists>, id: PlaylistId) -> AppResult<()> {
    playlists.delete(id)
}

#[tauri::command]
#[specta::specta]
pub fn add_to_playlist(
    playlists: State<'_, Playlists>,
    id: PlaylistId,
    track_ids: Vec<TrackId>,
) -> AppResult<()> {
    playlists.add_tracks(id, &track_ids)
}

#[tauri::command]
#[specta::specta]
pub fn remove_from_playlist(
    playlists: State<'_, Playlists>,
    id: PlaylistId,
    track_id: TrackId,
) -> AppResult<()> {
    playlists.remove_track(id, track_id)
}

#[tauri::command]
#[specta::specta]
pub fn reorder_playlists(playlists: State<'_, Playlists>, ids: Vec<PlaylistId>) -> AppResult<()> {
    playlists.reorder(&ids)
}

#[tauri::command]
#[specta::specta]
pub fn set_playlist_pinned(
    playlists: State<'_, Playlists>,
    id: PlaylistId,
    pinned: bool,
) -> AppResult<()> {
    playlists.set_pinned(id, pinned)
}
