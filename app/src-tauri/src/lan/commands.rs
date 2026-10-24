use tauri::State;

use super::{Direction, Lan, LanState};
use crate::error::AppResult;

#[tauri::command]
#[specta::specta]
pub fn get_lan_state(lan: State<'_, Lan>) -> LanState {
    lan.device_state()
}

#[tauri::command]
#[specta::specta]
pub fn set_device_name(lan: State<'_, Lan>, name: String) -> AppResult<String> {
    lan.set_device_name(&name)
}

/// Asks a device of the network for a sync; `code` (4 digits) is shown there too.
/// Returns whether its user accepted; progress and the outcome come as
/// `LanProgress` / `LanDone` events.
#[tauri::command]
#[specta::specta]
pub async fn lan_request_sync(
    lan: State<'_, Lan>,
    peer_id: String,
    direction: Direction,
    mirror: bool,
    code: String,
) -> AppResult<bool> {
    let lan = lan.inner().clone();
    lan.request_sync(&peer_id, direction, mirror, &code).await
}

/// Answers the sync request shown to the user.
#[tauri::command]
#[specta::specta]
pub fn lan_answer(lan: State<'_, Lan>, accept: bool) {
    lan.answer(accept);
}
