//! Music library: every track the user has added (table `tracks`).
//! Downloaded tracks carry their `youtube_id`.
//!
//! Every change emits [`LibraryChanged`] with the full list.

pub mod commands;

use serde::Serialize;
use tauri::AppHandle;

use crate::audio::{QueueItem, RandomPicker};
use crate::db::Db;
use crate::error::AppResult;

pub type TrackId = i64;

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
pub struct Track {
    pub id: TrackId,
    /// Absolute path (unique).
    pub path: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    /// Seconds (0 when unknown).
    pub duration: f64,
    /// Unix timestamp (seconds).
    pub added_at: i64,
    /// Set for tracks downloaded from YouTube / YouTube Music.
    pub youtube_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, specta::Type, tauri_specta::Event)]
pub struct LibraryChanged {
    pub tracks: Vec<Track>,
}

/// Managed state.
#[derive(Clone)]
pub struct Library {
    #[allow(dead_code)]
    db: Db,
    #[allow(dead_code)]
    app: Option<AppHandle>,
}

impl Library {
    /// `app` is `None` in tests (no events emitted).
    pub fn new(db: Db, app: Option<AppHandle>) -> Self {
        Self { db, app }
    }

    /// All tracks, most recently added first.
    pub fn tracks(&self) -> AppResult<Vec<Track>> {
        todo!()
    }

    pub fn get(&self, _id: TrackId) -> AppResult<Track> {
        todo!()
    }

    /// Resolves ids to queue items, in order (error if one is unknown).
    pub fn queue_items(&self, _ids: &[TrackId]) -> AppResult<Vec<QueueItem>> {
        todo!()
    }

    /// Adds files (tags read with lofty); already-present paths are skipped.
    /// Returns the tracks actually added.
    pub fn add_paths(&self, _paths: &[String]) -> AppResult<Vec<Track>> {
        todo!()
    }

    /// Adds (or updates) a file downloaded from YouTube.
    pub fn add_downloaded(&self, _path: &str, _youtube_id: &str) -> AppResult<Track> {
        todo!()
    }

    pub fn find_by_youtube_id(&self, _youtube_id: &str) -> AppResult<Option<Track>> {
        todo!()
    }

    /// Removes the row (playlist entries go with it, `ON DELETE CASCADE`).
    /// Returns the removed track.
    pub fn remove(&self, _id: TrackId) -> AppResult<Track> {
        todo!()
    }

    /// Picker handed to the audio engine for autoplay.
    pub fn random_picker(&self) -> RandomPicker {
        todo!()
    }
}
