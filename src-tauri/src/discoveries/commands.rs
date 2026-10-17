use tauri::State;

use super::{Discoveries, Discovery};
use crate::error::AppResult;

/// The week's discoveries: stored ones when they exist, generated otherwise.
#[tauri::command]
#[specta::specta]
pub async fn get_discoveries(discoveries: State<'_, Discoveries>) -> AppResult<Discovery> {
    discoveries.get().await
}

/// Throws the week's discoveries away and generates new ones.
#[tauri::command]
#[specta::specta]
pub async fn refresh_discoveries(discoveries: State<'_, Discoveries>) -> AppResult<Discovery> {
    discoveries.refresh().await
}
