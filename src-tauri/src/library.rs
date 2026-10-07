//! Music library: every track the user has added, persisted as JSON in the app data dir.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use lofty::prelude::*;
use serde::{Deserialize, Serialize};

use crate::storage::{load_json, save_json};

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
    /// Set for tracks downloaded from YouTube / YouTube Music.
    #[serde(default)]
    pub youtube_id: Option<String>,
}

pub struct Library {
    file: PathBuf,
    tracks: Arc<Mutex<Vec<Track>>>,
}

impl Library {
    /// Loads the library from `file`, starting empty if it is missing or unreadable.
    pub fn load(file: PathBuf) -> Self {
        let tracks = load_json(&file);
        Self {
            file,
            tracks: Arc::new(Mutex::new(tracks)),
        }
    }

    /// A picker that returns a random track path, avoiding `exclude` unless it's the only one.
    pub fn random_picker(&self) -> impl Fn(Option<&str>) -> Option<String> + Send + Sync + 'static {
        let tracks = Arc::clone(&self.tracks);
        move |exclude| {
            let tracks = tracks.lock().unwrap();
            let candidates: Vec<&Track> = tracks
                .iter()
                .filter(|t| Some(t.path.as_str()) != exclude)
                .collect();
            let pool = if candidates.is_empty() {
                tracks.iter().collect()
            } else {
                candidates
            };
            fastrand::choice(pool).map(|t| t.path.clone())
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

    /// Adds (or updates) a file downloaded from YouTube.
    pub fn add_downloaded(&self, path: &str, youtube_id: &str) -> Result<(), String> {
        let mut track = read_track(Path::new(path))?;
        track.youtube_id = Some(youtube_id.to_owned());
        let mut tracks = self.tracks.lock().unwrap();
        tracks.retain(|t| t.path != path);
        tracks.push(track);
        self.save(&tracks)
    }

    pub fn has_youtube_id(&self, youtube_id: &str) -> bool {
        self.tracks
            .lock()
            .unwrap()
            .iter()
            .any(|t| t.youtube_id.as_deref() == Some(youtube_id))
    }

    /// Path of the library copy of this video, if it is still on disk
    /// (listening groups play it instead of downloading it again).
    pub fn youtube_path(&self, youtube_id: &str) -> Option<String> {
        self.tracks
            .lock()
            .unwrap()
            .iter()
            .find(|t| t.youtube_id.as_deref() == Some(youtube_id))
            .map(|t| t.path.clone())
            .filter(|path| Path::new(path).is_file())
    }

    pub fn remove(&self, path: &str) -> Result<Vec<Track>, String> {
        let mut tracks = self.tracks.lock().unwrap();
        tracks.retain(|t| t.path != path);
        self.save(&tracks)?;
        Ok(tracks.clone())
    }

    fn save(&self, tracks: &[Track]) -> Result<(), String> {
        save_json(&self.file, tracks)
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
        youtube_id: None,
    })
}
