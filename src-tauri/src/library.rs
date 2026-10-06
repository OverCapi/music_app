use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use lofty::prelude::*;
use serde::Serialize;
use tauri::{AppHandle, Manager};
use walkdir::WalkDir;

const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "m4a", "aac", "wav", "ogg", "opus"];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub track_no: Option<u32>,
    pub duration: f64,
    pub cover: Option<String>,
}

fn hash_str(s: &str) -> String {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    format!("{:016x}", h.finish())
}

fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| AUDIO_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// Writes the embedded cover to the cache dir once per artist/album and returns its path.
fn extract_cover(tag: &lofty::tag::Tag, key: &str, covers_dir: &Path) -> Option<String> {
    let name = format!("{}.img", hash_str(key));
    let dest = covers_dir.join(name);
    if dest.exists() {
        return Some(dest.to_string_lossy().into_owned());
    }
    let picture = tag.pictures().first()?;
    fs::write(&dest, picture.data()).ok()?;
    Some(dest.to_string_lossy().into_owned())
}

fn read_track(path: &Path, covers_dir: &Path) -> Track {
    let path_str = path.to_string_lossy().into_owned();
    let file_stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path_str.clone());

    let mut track = Track {
        id: hash_str(&path_str),
        path: path_str,
        title: file_stem,
        artist: String::new(),
        album: String::new(),
        track_no: None,
        duration: 0.0,
        cover: None,
    };

    let Ok(tagged) = lofty::read_from_path(path) else {
        return track;
    };
    track.duration = tagged.properties().duration().as_secs_f64();

    if let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) {
        if let Some(t) = tag.title().filter(|t| !t.trim().is_empty()) {
            track.title = t.into_owned();
        }
        track.artist = tag.artist().map(|s| s.into_owned()).unwrap_or_default();
        track.album = tag.album().map(|s| s.into_owned()).unwrap_or_default();
        track.track_no = tag.track();
        let cover_key = if track.album.is_empty() {
            track.path.clone()
        } else {
            format!("{}\u{0}{}", track.artist, track.album)
        };
        track.cover = extract_cover(tag, &cover_key, covers_dir);
    }
    track
}

fn covers_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("covers");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

#[tauri::command]
pub async fn scan_folders(app: AppHandle, paths: Vec<String>) -> Result<Vec<Track>, String> {
    let covers = covers_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        paths
            .iter()
            .flat_map(|root| WalkDir::new(root).follow_links(true).into_iter().flatten())
            .filter(|e| e.file_type().is_file() && is_audio(e.path()))
            .map(|e| read_track(e.path(), &covers))
            .collect()
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn read_files(app: AppHandle, paths: Vec<String>) -> Result<Vec<Track>, String> {
    let covers = covers_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        paths
            .iter()
            .map(PathBuf::from)
            .filter(|p| p.is_file() && is_audio(p))
            .map(|p| read_track(&p, &covers))
            .collect()
    })
    .await
    .map_err(|e| e.to_string())
}

/// Tells whether each path is a directory, so drag & drop can route folders vs files.
#[tauri::command]
pub fn classify_paths(paths: Vec<String>) -> Vec<bool> {
    paths.iter().map(|p| Path::new(p).is_dir()).collect()
}
