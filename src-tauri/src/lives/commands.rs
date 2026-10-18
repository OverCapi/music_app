use tauri::State;

use super::{Favorites, LiveChannel, LiveStream};
use crate::error::AppResult;

#[tauri::command]
#[specta::specta]
pub async fn search_lives(query: String) -> AppResult<Vec<LiveStream>> {
    super::search(&query).await
}

#[tauri::command]
#[specta::specta]
pub async fn resolve_live_url(url: String) -> AppResult<Vec<LiveStream>> {
    super::resolve_url(&url).await
}

#[tauri::command]
#[specta::specta]
pub async fn search_live_channels(query: String) -> AppResult<Vec<LiveChannel>> {
    super::search_channels(&query).await
}

#[tauri::command]
#[specta::specta]
pub async fn get_channel_lives(channel_id: String) -> AppResult<Vec<LiveStream>> {
    super::channel_lives(&channel_id).await
}

#[tauri::command]
#[specta::specta]
pub fn get_favorite_channels(favorites: State<'_, Favorites>) -> AppResult<Vec<LiveChannel>> {
    favorites.list()
}

/// Returns the updated list.
#[tauri::command]
#[specta::specta]
pub fn add_favorite_channel(
    favorites: State<'_, Favorites>,
    channel: LiveChannel,
) -> AppResult<Vec<LiveChannel>> {
    favorites.add(&channel)
}

/// Returns the updated list.
#[tauri::command]
#[specta::specta]
pub fn remove_favorite_channel(
    favorites: State<'_, Favorites>,
    id: String,
) -> AppResult<Vec<LiveChannel>> {
    favorites.remove(&id)
}
