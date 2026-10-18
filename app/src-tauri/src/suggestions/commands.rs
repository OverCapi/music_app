use tauri::State;

use super::{SuggestedTrack, queue_items};
use crate::audio::AudioHandle;
use crate::error::AppResult;
use crate::library::Library;

/// Replaces the queue with recommended tracks (a mix, the discoveries) and plays
/// `tracks[start]`. The tracks are not added to the library: they are downloaded
/// into a cache when played and forgotten unless the user likes them.
#[tauri::command]
#[specta::specta]
pub fn play_suggestions(
    audio: State<'_, AudioHandle>,
    library: State<'_, Library>,
    tracks: Vec<SuggestedTrack>,
    start: u32,
) -> AppResult<()> {
    audio.play_queue(queue_items(&library, tracks)?, start as usize)
}
