//! User playlists: named, ordered lists of library tracks (tables `playlists`,
//! `playlist_tracks`). A track appears at most once per playlist.
//!
//! Every change emits [`PlaylistsChanged`] with the full list.

pub mod commands;

use serde::Serialize;
use tauri::AppHandle;

use crate::db::Db;
use crate::error::AppResult;
use crate::library::TrackId;

pub type PlaylistId = i64;

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
pub struct Playlist {
    pub id: PlaylistId,
    pub name: String,
    /// In play order.
    pub track_ids: Vec<TrackId>,
    /// Unix timestamp (seconds).
    pub created_at: i64,
}

#[derive(Clone, Debug, Serialize, specta::Type, tauri_specta::Event)]
pub struct PlaylistsChanged {
    pub playlists: Vec<Playlist>,
}

/// Managed state.
#[derive(Clone)]
pub struct Playlists {
    #[allow(dead_code)]
    db: Db,
    #[allow(dead_code)]
    app: Option<AppHandle>,
}

impl Playlists {
    /// `app` is `None` in tests (no events emitted).
    pub fn new(db: Db, app: Option<AppHandle>) -> Self {
        Self { db, app }
    }

    /// All playlists, in creation order.
    pub fn all(&self) -> AppResult<Vec<Playlist>> {
        todo!()
    }

    /// Name is trimmed, non-empty, at most 100 chars.
    pub fn create(&self, _name: &str) -> AppResult<Playlist> {
        todo!()
    }

    pub fn rename(&self, _id: PlaylistId, _name: &str) -> AppResult<()> {
        todo!()
    }

    pub fn delete(&self, _id: PlaylistId) -> AppResult<()> {
        todo!()
    }

    /// Appends tracks, skipping ones already in the playlist.
    pub fn add_tracks(&self, _id: PlaylistId, _track_ids: &[TrackId]) -> AppResult<()> {
        todo!()
    }

    pub fn remove_track(&self, _id: PlaylistId, _track_id: TrackId) -> AppResult<()> {
        todo!()
    }

    /// Re-emits [`PlaylistsChanged`] (after a library change cascaded into playlists).
    pub fn notify_changed(&self) -> AppResult<()> {
        todo!()
    }
}
