//! Music library: every track the user has added (table `tracks`).
//! Downloaded tracks carry their `youtube_id`.
//!
//! Every change emits [`LibraryChanged`] with the full list.

pub mod commands;

use std::path::Path;
use std::sync::Arc;

use lofty::prelude::*;
use rusqlite::{OptionalExtension, Row, params};
use serde::Serialize;
use tauri::AppHandle;
use tauri_specta::Event;

use crate::audio::{QueueItem, RandomPicker};
use crate::db::{Db, now_secs};
use crate::error::{AppError, AppResult};

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
    db: Db,
    app: Option<AppHandle>,
}

const COLUMNS: &str = "id, path, title, artist, album, duration, added_at, youtube_id";
const GONE: &str = "Ce titre n'est plus dans la bibliothèque";

impl Library {
    /// `app` is `None` in tests (no events emitted).
    pub fn new(db: Db, app: Option<AppHandle>) -> Self {
        Self { db, app }
    }

    /// All tracks, most recently added first.
    pub fn tracks(&self) -> AppResult<Vec<Track>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(&format!(
            "SELECT {COLUMNS} FROM tracks ORDER BY added_at DESC, id DESC"
        ))?;
        let tracks = stmt
            .query_map([], track_from_row)?
            .collect::<Result<_, _>>()?;
        Ok(tracks)
    }

    pub fn get(&self, id: TrackId) -> AppResult<Track> {
        self.db
            .conn()
            .query_row(
                &format!("SELECT {COLUMNS} FROM tracks WHERE id = ?1"),
                [id],
                track_from_row,
            )
            .optional()?
            .ok_or_else(|| AppError::not_found(GONE))
    }

    /// Embedded cover art (bytes, MIME type), if the file has one.
    pub fn cover(&self, id: TrackId) -> AppResult<Option<(Vec<u8>, String)>> {
        let path = self.get(id)?.path;
        let tagged = lofty::read_from_path(path).map_err(|e| AppError::Internal(e.to_string()))?;
        let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
        let pictures = tag.map(|t| t.pictures()).unwrap_or_default();
        let picture = pictures
            .iter()
            .find(|p| p.pic_type() == lofty::picture::PictureType::CoverFront)
            .or_else(|| pictures.first());
        Ok(picture.map(|p| {
            let mime = p.mime_type().map_or("image/jpeg", |m| m.as_str());
            (p.data().to_vec(), mime.to_owned())
        }))
    }

    /// Resolves ids to queue items, in order (error if one is unknown).
    pub fn queue_items(&self, ids: &[TrackId]) -> AppResult<Vec<QueueItem>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT path FROM tracks WHERE id = ?1")?;
        ids.iter()
            .map(|&track_id| {
                let path = stmt
                    .query_row([track_id], |row| row.get(0))
                    .optional()?
                    .ok_or_else(|| AppError::not_found(GONE))?;
                Ok(QueueItem { track_id, path })
            })
            .collect()
    }

    /// Adds files (tags read with lofty); already-present paths are skipped.
    /// Returns the tracks actually added. Files that can't be read are reported in
    /// one error, after the others have been added.
    pub fn add_paths(&self, paths: &[String]) -> AppResult<Vec<Track>> {
        let mut added = Vec::new();
        let mut errors = Vec::new();
        for path in paths {
            let exists = self
                .db
                .conn()
                .query_row("SELECT 1 FROM tracks WHERE path = ?1", [path], |_| Ok(()))
                .optional()?
                .is_some();
            if exists {
                continue;
            }
            // Tags are read without holding the database lock.
            match read_track(Path::new(path)) {
                Ok(track) => added.push(self.insert(track)?),
                Err(e) => errors.push(format!("{path} : {e}")),
            }
        }
        if !added.is_empty() {
            self.changed()?;
        }
        if errors.is_empty() {
            Ok(added)
        } else {
            Err(AppError::invalid(format!(
                "Certains fichiers n'ont pas pu être ajoutés :\n{}",
                errors.join("\n")
            )))
        }
    }

    /// Adds (or updates) a file downloaded from YouTube.
    pub fn add_downloaded(&self, path: &str, youtube_id: &str) -> AppResult<Track> {
        let mut track = read_track(Path::new(path)).map_err(AppError::invalid)?;
        track.youtube_id = Some(youtube_id.to_owned());
        let track = self.insert(track)?;
        self.changed()?;
        Ok(track)
    }

    pub fn find_by_youtube_id(&self, youtube_id: &str) -> AppResult<Option<Track>> {
        Ok(self
            .db
            .conn()
            .query_row(
                &format!("SELECT {COLUMNS} FROM tracks WHERE youtube_id = ?1 LIMIT 1"),
                [youtube_id],
                track_from_row,
            )
            .optional()?)
    }

    /// Removes the row (playlist entries go with it, `ON DELETE CASCADE`).
    /// Returns the removed track.
    pub fn remove(&self, id: TrackId) -> AppResult<Track> {
        let track = self.get(id)?;
        self.db
            .conn()
            .execute("DELETE FROM tracks WHERE id = ?1", [id])?;
        self.changed()?;
        Ok(track)
    }

    /// Picker handed to the audio engine for autoplay.
    pub fn random_picker(&self) -> RandomPicker {
        let db = self.db.clone();
        Arc::new(move |avoid| {
            // `id IS ?1` sorts the avoided track last, so it is only picked when alone.
            db.conn()
                .query_row(
                    "SELECT id, path FROM tracks ORDER BY id IS ?1, random() LIMIT 1",
                    [avoid],
                    |row| {
                        Ok(QueueItem {
                            track_id: row.get(0)?,
                            path: row.get(1)?,
                        })
                    },
                )
                .optional()
                .unwrap_or_else(|e| {
                    tracing::error!("random pick failed: {e}");
                    None
                })
        })
    }

    /// Inserts a track (`id` ignored); a known path is updated in place and keeps its id.
    fn insert(&self, t: Track) -> AppResult<Track> {
        Ok(self.db.conn().query_row(
            &format!(
                "INSERT INTO tracks (path, title, artist, album, duration, added_at, youtube_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT (path) DO UPDATE SET title = excluded.title,
                     artist = excluded.artist, album = excluded.album,
                     duration = excluded.duration, added_at = excluded.added_at,
                     youtube_id = excluded.youtube_id
                 RETURNING {COLUMNS}"
            ),
            params![
                t.path,
                t.title,
                t.artist,
                t.album,
                t.duration,
                t.added_at,
                t.youtube_id
            ],
            track_from_row,
        )?)
    }

    /// Emits [`LibraryChanged`] with the full list.
    fn changed(&self) -> AppResult<()> {
        if let Some(app) = &self.app {
            let tracks = self.tracks()?;
            if let Err(e) = (LibraryChanged { tracks }).emit(app) {
                tracing::warn!("emit LibraryChanged failed: {e}");
            }
        }
        Ok(())
    }
}

fn track_from_row(row: &Row<'_>) -> rusqlite::Result<Track> {
    Ok(Track {
        id: row.get(0)?,
        path: row.get(1)?,
        title: row.get(2)?,
        artist: row.get(3)?,
        album: row.get(4)?,
        duration: row.get(5)?,
        added_at: row.get(6)?,
        youtube_id: row.get(7)?,
    })
}

/// Reads tags and duration; the title falls back to the file name.
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
        id: 0,
        path: path.to_string_lossy().into_owned(),
        title: text(tag.and_then(|t| t.title())).unwrap_or(file_stem),
        artist: text(tag.and_then(|t| t.artist())),
        album: text(tag.and_then(|t| t.album())),
        duration: tagged.properties().duration().as_secs_f64(),
        added_at: now_secs(),
        youtube_id: None,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Writes a 0.1 s silent mono WAV (readable by lofty, no tags).
    fn write_wav(path: &Path) {
        let data_len: u32 = 800 * 2; // 800 16-bit samples at 8 kHz
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_len).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav.extend_from_slice(&1u16.to_le_bytes()); // mono
        wav.extend_from_slice(&8000u32.to_le_bytes());
        wav.extend_from_slice(&16000u32.to_le_bytes()); // byte rate
        wav.extend_from_slice(&2u16.to_le_bytes()); // block align
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        wav.resize(wav.len() + data_len as usize, 0);
        std::fs::write(path, wav).unwrap();
    }

    /// A fresh empty directory under the system temp dir.
    pub fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "music_app_test_{name}_{}_{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Inserts a track row directly (no file needed).
    pub fn insert_track(lib: &Library, path: &str) -> TrackId {
        lib.insert(Track {
            id: 0,
            path: path.into(),
            title: path.into(),
            artist: None,
            album: None,
            duration: 1.0,
            added_at: 1,
            youtube_id: None,
        })
        .unwrap()
        .id
    }

    #[test]
    fn add_paths_reads_files_and_skips_known_ones() {
        let dir = temp_dir("library");
        let wav = dir.join("My Song.wav");
        write_wav(&wav);
        let wav = wav.to_string_lossy().into_owned();
        let lib = Library::new(Db::in_memory(), None);

        let added = lib.add_paths(std::slice::from_ref(&wav)).unwrap();
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].title, "My Song");
        assert!(added[0].duration > 0.0);
        assert!(
            lib.add_paths(std::slice::from_ref(&wav))
                .unwrap()
                .is_empty()
        );

        let missing = dir.join("missing.mp3").to_string_lossy().into_owned();
        assert!(lib.add_paths(&[missing]).is_err());
        assert_eq!(lib.tracks().unwrap().len(), 1);

        // A download of a known path updates it in place.
        let downloaded = lib.add_downloaded(&wav, "abc").unwrap();
        assert_eq!(downloaded.id, added[0].id);
        assert_eq!(
            lib.find_by_youtube_id("abc").unwrap().map(|t| t.id),
            Some(added[0].id)
        );
        assert!(lib.find_by_youtube_id("zzz").unwrap().is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn queue_items_and_remove() {
        let lib = Library::new(Db::in_memory(), None);
        let a = insert_track(&lib, "/a.mp3");
        let b = insert_track(&lib, "/b.mp3");
        let items = lib.queue_items(&[b, a]).unwrap();
        assert_eq!(items[0].path, "/b.mp3");
        assert_eq!(items[1].track_id, a);

        assert_eq!(lib.remove(a).unwrap().path, "/a.mp3");
        assert!(lib.get(a).is_err());
        assert!(lib.queue_items(&[a]).is_err());
        assert!(lib.remove(a).is_err());
    }

    #[test]
    fn random_picker_avoids_given_track() {
        let lib = Library::new(Db::in_memory(), None);
        let pick = lib.random_picker();
        assert!(pick(None).is_none());
        let a = insert_track(&lib, "/a.mp3");
        assert_eq!(pick(Some(a)).unwrap().track_id, a);
        let b = insert_track(&lib, "/b.mp3");
        for _ in 0..20 {
            assert_eq!(pick(Some(a)).unwrap().track_id, b);
        }
    }
}
