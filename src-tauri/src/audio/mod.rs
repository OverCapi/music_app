//! Audio engine: a dedicated thread owns the output device and the rodio `Player`
//! (not `Send`). The rest of the app talks to it through [`AudioHandle`]
//! (command channel + shared state).
//!
//! The play queue lives in the engine so tracks advance without the UI; when it
//! runs out the engine appends a random library track ([`RandomPicker`]).
//!
//! In group mode the listening-group client drives playback with scheduled starts
//! ([`GroupStart`]): local queue commands return [`AppError::GroupMode`], tracks never
//! advance on their own and the end of a track is reported through a callback.
//!
//! The engine pushes [`PlayerChanged`] on every state change (and a few times per
//! second while playing) and [`QueueChanged`] on every queue change: the UI never polls.

pub mod commands;
mod engine;
mod opus;
mod queue;

use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::error::AppResult;
use crate::library::TrackId;

/// One queue entry: the library track and its resolved path.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct QueueItem {
    pub track_id: TrackId,
    pub path: String,
}

/// Picks a random library track, avoiding the given one when possible.
pub type RandomPicker = Arc<dyn Fn(Option<TrackId>) -> Option<QueueItem> + Send + Sync>;

/// Called on the audio thread with the group token of the track that just finished.
pub type GroupEndedCallback = Arc<dyn Fn(u64) + Send + Sync>;

/// When a group track should start.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GroupStart {
    /// Stay paused at the position.
    Paused,
    /// Start at this instant. If it is already past, start right away further in
    /// the track, as if it had started on time.
    At(Instant),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, specta::Type)]
pub struct PlayerState {
    /// Library track being played (`None` in group mode or when nothing is loaded).
    pub track_id: Option<TrackId>,
    /// File being played, also set in group mode (temp file).
    pub path: Option<String>,
    pub playing: bool,
    /// False when nothing is loaded (never played, stopped, or queue finished).
    pub loaded: bool,
    /// Seconds.
    pub position: f64,
    /// Seconds (0 when unknown).
    pub duration: f64,
    /// 0.0–1.0.
    pub volume: f32,
    /// Index of the current track in the queue.
    pub current: Option<u32>,
    /// Last playback error (e.g. a queued file that could not be read).
    pub error: Option<String>,
    /// A listening group drives playback; local queue controls are disabled.
    pub group_mode: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, specta::Type)]
pub struct QueueState {
    pub items: Vec<QueueItem>,
    pub current: Option<u32>,
}

#[derive(Clone, Debug, Serialize, specta::Type, tauri_specta::Event)]
pub struct PlayerChanged(pub PlayerState);

#[derive(Clone, Debug, Serialize, specta::Type, tauri_specta::Event)]
pub struct QueueChanged(pub QueueState);

/// Managed state: the only way to talk to the engine.
pub struct AudioHandle {
    // Filled in by the implementation (command sender + shared state).
}

impl AudioHandle {
    /// Spawns the audio thread. `app` is used to emit [`PlayerChanged`] / [`QueueChanged`].
    pub fn spawn(_app: AppHandle, _pick_random: RandomPicker) -> Self {
        todo!()
    }

    pub fn state(&self) -> PlayerState {
        todo!()
    }

    pub fn queue(&self) -> QueueState {
        todo!()
    }

    /// Replaces the queue with `items` and starts playing `items[start]`.
    pub fn play_queue(&self, _items: Vec<QueueItem>, _start: usize) -> AppResult<()> {
        todo!()
    }

    pub fn enqueue(&self, _items: Vec<QueueItem>) -> AppResult<()> {
        todo!()
    }

    pub fn play_index(&self, _index: usize) -> AppResult<()> {
        todo!()
    }

    /// The current track cannot be removed.
    pub fn remove_from_queue(&self, _index: usize) -> AppResult<()> {
        todo!()
    }

    /// Removes every track except the current one.
    pub fn clear_queue(&self) -> AppResult<()> {
        todo!()
    }

    /// Drops every queue entry of a track removed from the library (except the current one).
    pub fn forget_track(&self, _track_id: TrackId) {
        todo!()
    }

    /// Resumes, or restarts the current/first track when stopped, or picks a random one.
    pub fn play(&self) -> AppResult<()> {
        todo!()
    }

    pub fn pause(&self) -> AppResult<()> {
        todo!()
    }

    pub fn stop(&self) -> AppResult<()> {
        todo!()
    }

    /// Next queue track, or a random library track at the end of the queue.
    pub fn next(&self) -> AppResult<()> {
        todo!()
    }

    /// Previous track, or restart the current one when past 3 s.
    pub fn previous(&self) -> AppResult<()> {
        todo!()
    }

    pub fn seek(&self, _secs: f64) -> AppResult<()> {
        todo!()
    }

    pub fn set_volume(&self, _volume: f32) -> AppResult<()> {
        todo!()
    }

    /// Hands playback over to the group client. Local commands are refused right away.
    pub fn enter_group(&self, _on_ended: GroupEndedCallback) -> AppResult<()> {
        todo!()
    }

    /// Stops group playback and restores the local queue (stopped).
    pub fn leave_group(&self) -> AppResult<()> {
        todo!()
    }

    /// Loads `path` (unless `token` is already loaded), seeks to `position` and
    /// starts as `start` says. `token` identifies the group entry.
    pub fn group_load(
        &self,
        _token: u64,
        _path: String,
        _position: f64,
        _start: GroupStart,
    ) -> AppResult<()> {
        todo!()
    }

    pub fn group_stop(&self) -> AppResult<()> {
        todo!()
    }
}
