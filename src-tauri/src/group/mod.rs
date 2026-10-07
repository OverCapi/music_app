//! Listening-group client: one WebSocket to the group server (see the
//! `group-protocol` crate for the playback model), local downloads of the tracks
//! about to play, and the audio engine in group mode.
//!
//! The server is the source of truth. Every `State` it sends is reconciled:
//! - downloads: the current entry and the next two, into `<cache>/groups/<CODE>/`,
//!   with URLs built only from validated YouTube ids (never from member input);
//! - `Ready` for every downloaded track (again after each reconnection: the server
//!   forgets a member's ready set when its connection drops);
//! - the audio engine, re-commanded only when the entry or the playback changes,
//!   so unrelated updates (a member becoming ready…) never cause glitches.
//!
//! Submodules: `clock` (NTP-style offset, pure), `reconcile` (state → actions,
//! pure), `connection` (WebSocket + reconnect), `files` (group cache dir).
//! The UI gets the whole picture as [`GroupView`] in [`GroupChanged`] events.

mod clock;
pub mod commands;
mod connection;
mod files;
mod reconcile;

use std::collections::HashMap;
use std::path::PathBuf;

use group_protocol::{ClientMessage, GroupState};
use serde::Serialize;
use tauri::AppHandle;

use crate::account::Session;
use crate::error::AppResult;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum TrackStatus {
    Queued,
    Downloading,
    Ready,
    Error,
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
pub struct TrackDownload {
    pub status: TrackStatus,
    /// 0.0–1.0 while downloading, when the size is known.
    pub progress: Option<f64>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
pub struct GroupView {
    pub code: String,
    /// `musicapp://join/<code>`.
    pub link: String,
    /// My username.
    pub me: String,
    /// False while reconnecting.
    pub connected: bool,
    pub state: GroupState,
    /// By YouTube id.
    pub downloads: HashMap<String, TrackDownload>,
}

/// `None` when not in a group.
#[derive(Clone, Debug, Serialize, specta::Type, tauri_specta::Event)]
pub struct GroupChanged(pub Option<GroupView>);

/// Error to show to the user (server error message, failed download…).
#[derive(Clone, Debug, Serialize, specta::Type, tauri_specta::Event)]
pub struct GroupError {
    pub message: String,
}

pub enum Intent {
    Create,
    /// Uppercase code (already parsed with `parse_invite`).
    Join(String),
}

/// Managed state.
pub struct GroupClient {
    // Filled in by the implementation.
}

impl GroupClient {
    /// `dir`: `<cache>/groups`, wiped here (leftovers of a previous run).
    pub fn new(_app: AppHandle, _dir: PathBuf) -> Self {
        todo!()
    }

    pub fn view(&self) -> Option<GroupView> {
        todo!()
    }

    /// Connects (leaving the current group, if any) and resolves once the first
    /// `State` arrives.
    pub async fn start(&self, _intent: Intent, _session: Session) -> AppResult<GroupView> {
        todo!()
    }

    /// Leaves the group (no-op if not in one), restores local playback, deletes files.
    pub fn leave(&self) {
        todo!()
    }

    /// Called on app exit: like `leave`, synchronously removes the cache dir.
    pub fn shutdown(&self) {
        todo!()
    }

    pub fn send(&self, _msg: ClientMessage) -> AppResult<()> {
        todo!()
    }
}
