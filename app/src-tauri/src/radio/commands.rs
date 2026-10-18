use tauri::State;

use super::{Radio, RadioTrack};
use crate::error::AppResult;

#[tauri::command]
#[specta::specta]
pub async fn get_radio(youtube_id: String, radio: State<'_, Radio>) -> AppResult<Vec<RadioTrack>> {
    radio.get(&youtube_id).await
}
