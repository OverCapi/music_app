use tauri::State;

use super::{AudioHandle, PlayerState, QueueState, RepeatMode};
use crate::error::AppResult;
use crate::history::PlaySource;
use crate::library::{Library, TrackId};
use crate::lives::LiveStream;

/// Replaces the queue with `track_ids` and plays `track_ids[start]`; `source` is
/// where the listens come from (for the history).
#[tauri::command]
#[specta::specta]
pub fn play_queue(
    audio: State<'_, AudioHandle>,
    library: State<'_, Library>,
    track_ids: Vec<TrackId>,
    start: u32,
    source: PlaySource,
) -> AppResult<()> {
    audio.play_queue(library.queue_items(&track_ids, source)?, start as usize)
}

#[tauri::command]
#[specta::specta]
pub fn enqueue(
    audio: State<'_, AudioHandle>,
    library: State<'_, Library>,
    track_ids: Vec<TrackId>,
    source: PlaySource,
) -> AppResult<()> {
    audio.enqueue(library.queue_items(&track_ids, source)?)
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

/// Plays a live found with `search_lives` / `resolve_live_url`, outside the queue.
#[tauri::command]
#[specta::specta]
pub fn play_live(audio: State<'_, AudioHandle>, stream: LiveStream) -> AppResult<()> {
    audio.play_live(stream)
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

#[tauri::command]
#[specta::specta]
pub fn set_shuffle(audio: State<'_, AudioHandle>, shuffle: bool) -> AppResult<()> {
    audio.set_shuffle(shuffle)
}

#[tauri::command]
#[specta::specta]
pub fn set_repeat(audio: State<'_, AudioHandle>, repeat: RepeatMode) -> AppResult<()> {
    audio.set_repeat(repeat)
}
