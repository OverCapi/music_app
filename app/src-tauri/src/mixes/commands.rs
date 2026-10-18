use tauri::State;

use super::{Mix, Mixes};
use crate::error::AppResult;

/// Mixes generated from what the user listens to (recomputed on each call).
#[tauri::command]
#[specta::specta]
pub async fn get_mixes(mixes: State<'_, Mixes>) -> AppResult<Vec<Mix>> {
    mixes.generate().await
}
