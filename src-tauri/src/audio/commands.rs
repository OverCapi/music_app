use tauri::State;

use super::{AudioHandle, PlayerState, QueueState};
use crate::error::AppResult;
use crate::library::{Library, TrackId};

/// Replaces the queue with `track_ids` and plays `track_ids[start]`.
#[tauri::command]
#[specta::specta]
pub fn play_queue(
    audio: State<'_, AudioHandle>,
    library: State<'_, Library>,
    track_ids: Vec<TrackId>,
    start: u32,
) -> AppResult<()> {
    audio.play_queue(library.queue_items(&track_ids)?, start as usize)
}

#[tauri::command]
#[specta::specta]
pub fn enqueue(
    audio: State<'_, AudioHandle>,
    library: State<'_, Library>,
    track_ids: Vec<TrackId>,
) -> AppResult<()> {
    audio.enqueue(library.queue_items(&track_ids)?)
}

#[tauri::command]
#[specta::specta]
pub fn play_index(audio: State<'_, AudioHandle>, index: u32) -> AppResult<()> {
    audio.play_index(index as usize)
}

#[tauri::command]
#[specta::specta]
pub fn remove_from_queue(audio: State<'_, AudioHandle>, index: u32) -> AppResult<()> {
    audio.remove_from_queue(index as usize)
}

#[tauri::command]
#[specta::specta]
pub fn clear_queue(audio: State<'_, AudioHandle>) -> AppResult<()> {
    audio.clear_queue()
}

#[tauri::command]
#[specta::specta]
pub fn get_queue(audio: State<'_, AudioHandle>) -> QueueState {
    audio.queue()
}

#[tauri::command]
#[specta::specta]
pub fn get_player_state(audio: State<'_, AudioHandle>) -> PlayerState {
    audio.state()
}

#[tauri::command]
#[specta::specta]
pub fn play(audio: State<'_, AudioHandle>) -> AppResult<()> {
    audio.play()
}

#[tauri::command]
#[specta::specta]
pub fn pause(audio: State<'_, AudioHandle>) -> AppResult<()> {
    audio.pause()
}

#[tauri::command]
#[specta::specta]
pub fn stop(audio: State<'_, AudioHandle>) -> AppResult<()> {
    audio.stop()
}

#[tauri::command]
#[specta::specta]
pub fn next(audio: State<'_, AudioHandle>) -> AppResult<()> {
    audio.next()
}

#[tauri::command]
#[specta::specta]
pub fn previous(audio: State<'_, AudioHandle>) -> AppResult<()> {
    audio.previous()
}

#[tauri::command]
#[specta::specta]
pub fn seek(audio: State<'_, AudioHandle>, secs: f64) -> AppResult<()> {
    audio.seek(secs)
}

#[tauri::command]
#[specta::specta]
pub fn set_volume(audio: State<'_, AudioHandle>, volume: f32) -> AppResult<()> {
    audio.set_volume(volume)
}
