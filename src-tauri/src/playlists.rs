//! User playlists: named, ordered lists of library tracks (by path),
//! persisted as JSON in the app data dir.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::storage::{load_json, save_json};

const MAX_NAME_LEN: usize = 100;

#[derive(Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    /// Track paths, in play order (no duplicates).
    pub tracks: Vec<String>,
    /// Unix timestamp (seconds).
    pub created_at: u64,
}

pub struct Playlists {
    file: PathBuf,
    playlists: Mutex<Vec<Playlist>>,
}

impl Playlists {
    pub fn load(file: PathBuf) -> Self {
        let playlists = load_json(&file);
        Self {
            file,
            playlists: Mutex::new(playlists),
        }
    }

    pub fn all(&self) -> Vec<Playlist> {
        self.playlists.lock().unwrap().clone()
    }

    pub fn create(&self, name: &str) -> Result<(Playlist, Vec<Playlist>), String> {
        let playlist = Playlist {
            id: new_id(),
            name: clean_name(name)?,
            tracks: Vec::new(),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };
        let all = self.update(|playlists| {
            playlists.push(playlist.clone());
            Ok(())
        })?;
        Ok((playlist, all))
    }

    pub fn rename(&self, id: &str, name: &str) -> Result<Vec<Playlist>, String> {
        let name = clean_name(name)?;
        self.update_one(id, |p| p.name = name)
    }

    pub fn delete(&self, id: &str) -> Result<Vec<Playlist>, String> {
        self.update(|playlists| {
            playlists.retain(|p| p.id != id);
            Ok(())
        })
    }

    /// Appends `paths`, skipping tracks already in the playlist.
    pub fn add_tracks(&self, id: &str, paths: Vec<String>) -> Result<Vec<Playlist>, String> {
        self.update_one(id, |p| {
            for path in paths {
                if !p.tracks.contains(&path) {
                    p.tracks.push(path);
                }
            }
        })
    }

    pub fn remove_track(&self, id: &str, path: &str) -> Result<Vec<Playlist>, String> {
        self.update_one(id, |p| p.tracks.retain(|t| t != path))
    }

    /// Drops a track from every playlist (called when it leaves the library).
    pub fn forget_track(&self, path: &str) -> Result<Vec<Playlist>, String> {
        self.update(|playlists| {
            for p in playlists.iter_mut() {
                p.tracks.retain(|t| t != path);
            }
            Ok(())
        })
    }

    fn update_one(&self, id: &str, f: impl FnOnce(&mut Playlist)) -> Result<Vec<Playlist>, String> {
        self.update(|playlists| {
            let playlist = playlists
                .iter_mut()
                .find(|p| p.id == id)
                .ok_or("Cette playlist n'existe plus")?;
            f(playlist);
            Ok(())
        })
    }

    /// Applies `f`, saves, and returns the new list. Nothing changes if `f` or the save fails.
    fn update(
        &self,
        f: impl FnOnce(&mut Vec<Playlist>) -> Result<(), String>,
    ) -> Result<Vec<Playlist>, String> {
        let mut playlists = self.playlists.lock().unwrap();
        let mut next = playlists.clone();
        f(&mut next)?;
        save_json(&self.file, &next)?;
        *playlists = next;
        Ok(playlists.clone())
    }
}

fn clean_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Le nom de la playlist est vide".to_owned());
    }
    Ok(name.chars().take(MAX_NAME_LEN).collect())
}

fn new_id() -> String {
    std::iter::repeat_with(fastrand::alphanumeric)
        .take(12)
        .collect()
}
