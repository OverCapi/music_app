//! Music library: every track the user has added (table `tracks`).
//! Downloaded tracks carry their `youtube_id`. Tracks imported from another service
//! can be in the library before their file is downloaded (`path` is `None`).
//!
//! A YouTube id identifies at most one track, and [`match_key`] (normalized artist
//! and title) recognizes the same song coming from elsewhere, so imports from
//! several services don't create duplicates.
//!
//! Every change emits [`LibraryChanged`] with the full list.

pub mod commands;
mod match_key;

use std::path::Path;
use std::sync::Arc;

use group_protocol::is_valid_youtube_id;
use lofty::prelude::*;
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::Serialize;
use tauri::AppHandle;
use tauri_specta::Event;

use crate::audio::{QueueItem, RandomPicker};
use crate::db::{Db, now_secs};
use crate::error::{AppError, AppResult};
use crate::history::PlaySource;

pub type TrackId = i64;

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
pub struct Track {
    pub id: TrackId,
    /// Absolute path (unique); `None` until the file is downloaded.
    pub path: Option<String>,
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

/// A track known from another service, not downloaded yet.
#[cfg_attr(not(test), expect(dead_code, reason = "used by the service imports"))]
#[derive(Clone, Debug, PartialEq)]
pub struct RemoteTrack {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    /// Seconds (0 when unknown).
    pub duration: f64,
    /// When already known (YouTube Music), else found later by matching.
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
const NOT_DOWNLOADED: &str = "Ce titre n'est pas encore téléchargé";

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
        let Some(path) = self.get(id)?.path else {
            return Ok(None);
        };
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

    /// Resolves ids to queue items, in order (error if one is unknown or not
    /// downloaded yet).
    pub fn queue_items(&self, ids: &[TrackId], source: PlaySource) -> AppResult<Vec<QueueItem>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT path FROM tracks WHERE id = ?1")?;
        ids.iter()
            .map(|&track_id| {
                let path: Option<String> = stmt
                    .query_row([track_id], |row| row.get(0))
                    .optional()?
                    .ok_or_else(|| AppError::not_found(GONE))?;
                let path = path.ok_or_else(|| AppError::invalid(NOT_DOWNLOADED))?;
                Ok(QueueItem {
                    track_id,
                    path,
                    source,
                })
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

    /// Adds a file downloaded from YouTube. A track that already has this YouTube id
    /// (not downloaded yet, or whose file is gone) gets the file and keeps its id,
    /// title, artist and album.
    pub fn add_downloaded(&self, path: &str, youtube_id: &str) -> AppResult<Track> {
        let mut track = read_track(Path::new(path)).map_err(AppError::invalid)?;
        let updated = self
            .db
            .conn()
            .query_row(
                &format!(
                    "UPDATE tracks SET path = ?1, duration = ?2 WHERE youtube_id = ?3
                     RETURNING {COLUMNS}"
                ),
                params![path, track.duration, youtube_id],
                track_from_row,
            )
            .optional()?;
        let track = match updated {
            Some(track) => track,
            None => {
                track.youtube_id = Some(youtube_id.to_owned());
                self.insert(track)?
            }
        };
        self.changed()?;
        Ok(track)
    }

    /// Adds tracks from another service without downloading them, reusing the
    /// library track when it is the same song: same YouTube id, else same
    /// [`match_key`] and a close duration (a matched track without YouTube id gets
    /// this one). Returns the track id of each entry, in order.
    #[cfg_attr(not(test), expect(dead_code, reason = "used by the service imports"))]
    pub fn add_remote(&self, tracks: &[RemoteTrack]) -> AppResult<Vec<TrackId>> {
        let mut ids = Vec::with_capacity(tracks.len());
        let mut added = false;
        for t in tracks {
            if let Some(youtube_id) = &t.youtube_id
                && !is_valid_youtube_id(youtube_id)
            {
                return Err(AppError::invalid("Identifiant YouTube invalide"));
            }
            let existing = match &t.youtube_id {
                Some(youtube_id) => self.find_by_youtube_id(youtube_id)?,
                None => None,
            };
            let existing = match existing {
                Some(track) => Some(track),
                None => self
                    .find_similar(t.artist.as_deref(), &t.title, t.duration)?
                    .into_iter()
                    .find(|track| track.youtube_id.is_none() || t.youtube_id.is_none()),
            };
            let id = match existing {
                Some(track) => {
                    if track.youtube_id.is_none() && t.youtube_id.is_some() {
                        self.db.conn().execute(
                            "UPDATE tracks SET youtube_id = ?1 WHERE id = ?2",
                            params![t.youtube_id, track.id],
                        )?;
                        added = true;
                    }
                    track.id
                }
                None => {
                    added = true;
                    self.insert(Track {
                        id: 0,
                        path: None,
                        title: t.title.clone(),
                        artist: t.artist.clone(),
                        album: t.album.clone(),
                        duration: t.duration,
                        added_at: now_secs(),
                        youtube_id: t.youtube_id.clone(),
                    })?
                    .id
                }
            };
            ids.push(id);
        }
        if added {
            self.changed()?;
        }
        Ok(ids)
    }

    /// Library tracks with the same [`match_key`] and a close duration.
    #[cfg_attr(not(test), expect(dead_code, reason = "used by the service imports"))]
    pub fn find_similar(
        &self,
        artist: Option<&str>,
        title: &str,
        duration: f64,
    ) -> AppResult<Vec<Track>> {
        let Some(key) = match_key::match_key(artist, title) else {
            return Ok(Vec::new());
        };
        let conn = self.db.conn();
        let mut stmt = conn.prepare(&format!(
            "SELECT {COLUMNS} FROM tracks WHERE match_key = ?1 ORDER BY id"
        ))?;
        let tracks = stmt
            .query_map([key], track_from_row)?
            .filter(|t| {
                t.as_ref()
                    .map_or(true, |t| match_key::same_duration(t.duration, duration))
            })
            .collect::<Result<_, _>>()?;
        Ok(tracks)
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
                    "SELECT id, path FROM tracks WHERE path IS NOT NULL
                     ORDER BY id IS ?1, random() LIMIT 1",
                    [avoid],
                    |row| {
                        Ok(QueueItem {
                            track_id: row.get(0)?,
                            path: row.get(1)?,
                            source: PlaySource::Autoplay,
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
                "INSERT INTO tracks (path, title, artist, album, duration, added_at,
                     youtube_id, match_key)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT (path) DO UPDATE SET title = excluded.title,
                     artist = excluded.artist, album = excluded.album,
                     duration = excluded.duration, added_at = excluded.added_at,
                     youtube_id = excluded.youtube_id, match_key = excluded.match_key
                 RETURNING {COLUMNS}"
            ),
            params![
                t.path,
                t.title,
                t.artist,
                t.album,
                t.duration,
                t.added_at,
                t.youtube_id,
                match_key::match_key(t.artist.as_deref(), &t.title)
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

/// Computes the missing `match_key`s (rows inserted by migrations or the legacy
/// import, which can't run Rust code).
pub(crate) fn fill_match_keys(conn: &Connection) -> AppResult<()> {
    let mut select =
        conn.prepare("SELECT id, artist, title FROM tracks WHERE match_key IS NULL")?;
    let rows = select
        .query_map([], |row| {
            Ok((
                row.get::<_, TrackId>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut update = conn.prepare("UPDATE tracks SET match_key = ?1 WHERE id = ?2")?;
    for (id, artist, title) in rows {
        // An empty key marks "nothing to match on" so the row isn't retried.
        let key = match_key::match_key(artist.as_deref(), &title).unwrap_or_default();
        update.execute(params![key, id])?;
    }
    Ok(())
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
        path: Some(path.to_string_lossy().into_owned()),
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
            path: Some(path.into()),
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
        let items = lib.queue_items(&[b, a], PlaySource::Library).unwrap();
        assert_eq!(items[0].path, "/b.mp3");
        assert_eq!(items[1].track_id, a);

        assert_eq!(lib.remove(a).unwrap().path.as_deref(), Some("/a.mp3"));
        assert!(lib.get(a).is_err());
        assert!(lib.queue_items(&[a], PlaySource::Library).is_err());
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

    fn remote(title: &str, artist: &str, duration: f64, youtube_id: Option<&str>) -> RemoteTrack {
        RemoteTrack {
            title: title.into(),
            artist: Some(artist.into()),
            album: None,
            duration,
            youtube_id: youtube_id.map(Into::into),
        }
    }

    #[test]
    fn add_remote_reuses_the_same_song() {
        let lib = Library::new(Db::in_memory(), None);
        let ids = lib
            .add_remote(&[
                remote("Get Lucky", "Daft Punk", 248.0, Some("aaaaaaaaaaa")),
                remote("Get Lucky", "Daft Punk", 248.0, Some("aaaaaaaaaaa")),
                remote("Other", "Daft Punk", 200.0, None),
            ])
            .unwrap();
        assert_eq!(ids[0], ids[1]);
        assert_ne!(ids[0], ids[2]);
        let track = lib.get(ids[0]).unwrap();
        assert_eq!(track.path, None);
        assert_eq!(track.youtube_id.as_deref(), Some("aaaaaaaaaaa"));

        // Same song from Spotify (no YouTube id, other spelling): reused.
        let spotify = lib
            .add_remote(&[remote(
                "Get Lucky (feat. Pharrell Williams)",
                "Daft Punk, Pharrell Williams",
                250.0,
                None,
            )])
            .unwrap();
        assert_eq!(spotify, vec![ids[0]]);
        // A matched track without YouTube id receives it.
        let found = lib
            .add_remote(&[remote("Other", "Daft Punk", 201.0, Some("bbbbbbbbbbb"))])
            .unwrap();
        assert_eq!(found, vec![ids[2]]);
        assert_eq!(
            lib.get(ids[2]).unwrap().youtube_id.as_deref(),
            Some("bbbbbbbbbbb")
        );
        // Very different duration (live version...): a separate track.
        let live = lib
            .add_remote(&[remote("Get Lucky", "Daft Punk", 600.0, None)])
            .unwrap();
        assert_ne!(live[0], ids[0]);
        assert_eq!(lib.tracks().unwrap().len(), 3);

        assert!(
            lib.add_remote(&[remote("X", "Y", 1.0, Some("bad id"))])
                .is_err()
        );
    }

    #[test]
    fn remote_tracks_are_not_playable_until_downloaded() {
        let dir = temp_dir("remote");
        let wav = dir.join("Get Lucky.wav");
        write_wav(&wav);
        let wav = wav.to_string_lossy().into_owned();
        let lib = Library::new(Db::in_memory(), None);
        let id = lib
            .add_remote(&[remote("Get Lucky", "Daft Punk", 248.0, Some("aaaaaaaaaaa"))])
            .unwrap()[0];

        assert!(lib.queue_items(&[id], PlaySource::Library).is_err());
        assert!(lib.random_picker()(None).is_none());
        assert_eq!(lib.cover(id).unwrap(), None);

        // The download fills the existing track and keeps its metadata.
        let downloaded = lib.add_downloaded(&wav, "aaaaaaaaaaa").unwrap();
        assert_eq!(downloaded.id, id);
        assert_eq!(downloaded.title, "Get Lucky");
        assert_eq!(downloaded.path.as_deref(), Some(wav.as_str()));
        assert_eq!(
            lib.queue_items(&[id], PlaySource::Library).unwrap()[0].path,
            wav
        );
        assert_eq!(lib.tracks().unwrap().len(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn fills_missing_match_keys() {
        let lib = Library::new(Db::in_memory(), None);
        lib.db
            .conn()
            .execute(
                "INSERT INTO tracks (path, title, artist, added_at)
                 VALUES ('/a.mp3', 'Get Lucky', 'Daft Punk', 1)",
                [],
            )
            .unwrap();
        fill_match_keys(&lib.db.conn()).unwrap();
        assert_eq!(
            lib.find_similar(Some("Daft Punk"), "Get Lucky", 0.0)
                .unwrap()
                .len(),
            1
        );
    }
}
