use group_protocol::{ClientMessage, is_valid_youtube_id, parse_invite};
use tauri::State;

use super::{GroupClient, GroupView, Intent};
use crate::account::AccountStore;
use crate::error::{AppError, AppResult};

fn session(account: &AccountStore) -> AppResult<crate::account::Session> {
    account
        .session()?
        .ok_or_else(|| AppError::invalid("Connecte-toi d'abord"))
}

#[tauri::command]
#[specta::specta]
pub fn get_group(group: State<'_, GroupClient>) -> Option<GroupView> {
    group.view()
}

#[tauri::command]
#[specta::specta]
pub async fn group_create(
    account: State<'_, AccountStore>,
    group: State<'_, GroupClient>,
) -> AppResult<GroupView> {
    group.start(Intent::Create, session(&account)?).await
}

/// `invite`: a bare code or a `musicapp://join/…` link.
#[tauri::command]
#[specta::specta]
pub async fn group_join(
    account: State<'_, AccountStore>,
    group: State<'_, GroupClient>,
    invite: String,
) -> AppResult<GroupView> {
    let code =
        parse_invite(&invite).ok_or_else(|| AppError::invalid("Code d'invitation invalide"))?;
    group.start(Intent::Join(code), session(&account)?).await
}

/// Async (like `logout`): leaving deletes the group's files.
#[tauri::command]
#[specta::specta]
pub async fn group_leave(group: State<'_, GroupClient>) -> AppResult<()> {
    group.leave();
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn group_add(
    group: State<'_, GroupClient>,
    youtube_id: String,
    title: String,
    artist: Option<String>,
    duration: Option<f64>,
) -> AppResult<()> {
    if !is_valid_youtube_id(&youtube_id) {
        return Err(AppError::invalid("Identifiant YouTube invalide"));
    }
    group.send(ClientMessage::Add {
        youtube_id,
        title,
        artist,
        duration: duration.filter(|d| d.is_finite() && *d > 0.0),
    })
}

#[tauri::command]
#[specta::specta]
pub fn group_remove(group: State<'_, GroupClient>, entry_id: u64) -> AppResult<()> {
    group.send(ClientMessage::Remove { entry_id })
}

#[tauri::command]
#[specta::specta]
pub fn group_play_entry(group: State<'_, GroupClient>, entry_id: u64) -> AppResult<()> {
    group.send(ClientMessage::PlayEntry { entry_id })
}

#[tauri::command]
#[specta::specta]
pub fn group_play(group: State<'_, GroupClient>) -> AppResult<()> {
    group.send(ClientMessage::Play)
}

#[tauri::command]
#[specta::specta]
pub fn group_pause(group: State<'_, GroupClient>) -> AppResult<()> {
    group.send(ClientMessage::Pause)
}

#[tauri::command]
#[specta::specta]
pub fn group_seek(group: State<'_, GroupClient>, position: f64) -> AppResult<()> {
    if !position.is_finite() {
        return Err(AppError::invalid("Position invalide"));
    }
    group.send(ClientMessage::Seek {
        position: position.max(0.0),
    })
}

#[tauri::command]
#[specta::specta]
pub fn group_skip(group: State<'_, GroupClient>) -> AppResult<()> {
    group.send(ClientMessage::Skip)
}
