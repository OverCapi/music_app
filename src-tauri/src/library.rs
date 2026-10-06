//! Music library: every track the user has added, persisted as JSON in the app data dir.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use lofty::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Track {
    /// Absolute path; also the track's unique id.
    pub path: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: f64,
    /// Unix timestamp (seconds).
    pub added_at: u64,
}

pub struct Library {
    file: PathBuf,
    tracks: Mutex<Vec<Track>>,
}

impl Library {
    /// Loads the library from `file`, starting empty if it is missing or unreadable.
    pub fn load(file: PathBuf) -> Self {
        let tracks = fs::read_to_string(&file)
            .ok()
            .and_then(|json| match serde_json::from_str(&json) {
                Ok(tracks) => Some(tracks),
                Err(e) => {
                    eprintln!("library: ignoring corrupt {}: {e}", file.display());
                    None
                }
            })
            .unwrap_or_default();
        Self {
            file,
            tracks: Mutex::new(tracks),
        }
    }

    pub fn tracks(&self) -> Vec<Track> {
        self.tracks.lock().unwrap().clone()
    }

    /// Adds the given files (skipping ones already present) and returns the new full list.
    pub fn add(&self, paths: &[String]) -> Result<Vec<Track>, String> {
        let mut tracks = self.tracks.lock().unwrap();
        let mut errors = Vec::new();
        for path in paths {
            if tracks.iter().any(|t| &t.path == path) {
                continue;
            }
            match read_track(Path::new(path)) {
                Ok(track) => tracks.push(track),
                Err(e) => errors.push(format!("{path} : {e}")),
            }
        }
        self.save(&tracks)?;
        if errors.is_empty() {
            Ok(tracks.clone())
        } else {
            Err(format!(
                "Certains fichiers n'ont pas pu être ajoutés :\n{}",
                errors.join("\n")
            ))
        }
    }

    pub fn remove(&self, path: &str) -> Result<Vec<Track>, String> {
        let mut tracks = self.tracks.lock().unwrap();
        tracks.retain(|t| t.path != path);
        self.save(&tracks)?;
        Ok(tracks.clone())
    }

    /// Writes to a temp file then renames, so a crash never leaves a half-written library.
    fn save(&self, tracks: &[Track]) -> Result<(), String> {
        let err = |e: std::io::Error| format!("Impossible d'enregistrer la bibliothèque : {e}");
        if let Some(dir) = self.file.parent() {
            fs::create_dir_all(dir).map_err(err)?;
        }
        let json = serde_json::to_string_pretty(tracks).map_err(|e| e.to_string())?;
        let tmp = self.file.with_extension("json.tmp");
        fs::write(&tmp, json).map_err(err)?;
        fs::rename(&tmp, &self.file).map_err(err)
    }
}

fn read_track(path: &Path) -> Result<Track, String> {
    let tagged = lofty::read_from_path(path).map_err(|e| e.to_string())?;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
    let text = |value: Option<std::borrow::Cow<'_, str>>| {
        value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
    };

    let file_stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    Ok(Track {
        path: path.to_string_lossy().into_owned(),
        title: text(tag.and_then(|t| t.title())).unwrap_or(file_stem),
        artist: text(tag.and_then(|t| t.artist())),
        album: text(tag.and_then(|t| t.album())),
        duration: tagged.properties().duration().as_secs_f64(),
        added_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    })
}
