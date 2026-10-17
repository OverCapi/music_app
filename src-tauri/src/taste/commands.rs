use tauri::State;

use super::{Taste, TasteProfile};
use crate::error::AppResult;

/// What the user likes, computed from the listening history and the feedback.
#[tauri::command]
#[specta::specta]
pub fn get_taste_profile(taste: State<'_, Taste>) -> AppResult<TasteProfile> {
    taste.profile()
}
