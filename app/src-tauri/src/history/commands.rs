use tauri::State;

use super::{History, Play};
use crate::error::AppResult;

/// Most recent listens first.
#[tauri::command]
#[specta::specta]
pub fn get_recent_plays(history: State<'_, History>, limit: u32) -> AppResult<Vec<Play>> {
    history.recent(limit)
}
