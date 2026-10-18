//! Storage optimizer: deletes the audio *file* of library tracks that were not
//! played for a while, to save disk space. The track stays in the library
//! (`path` becomes NULL) and is downloaded again when played
//! (`TrackFetcher` → `Downloads::fetch`).
//!
//! Never touched: favorites, tracks without YouTube id (they cannot be downloaded
//! again), files outside the app music folder, and tracks in the play queue.
//!
//! "Last played" is the latest play of the track, else the date it was added.

pub mod commands;

use std::collections::HashSet;

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_specta::Event;

use crate::db::{Db, now_secs};
use crate::downloads::Downloads;
use crate::error::AppResult;
use crate::library::{Library, TrackId};

const AUTO_KEY: &str = "storage_auto";
const DAYS_KEY: &str = "storage_days";
const DEFAULT_DAYS: u32 = 30;
const DAY_SECS: i64 = 86_400;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct StorageSettings {
    /// Run the optimizer in the background at every app start.
    pub auto: bool,
    /// A track not played for more than this many days is expired.
    pub days: u32,
}

/// A track whose audio file can be deleted.
#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
pub struct StorageTrack {
    pub track_id: TrackId,
    pub title: String,
    pub artist: Option<String>,
    /// File size in bytes.
    pub size: u64,
    /// Unix timestamp (seconds) of the last play, else of the date it was added.
    pub last_played: i64,
    /// Not played for more than the configured number of days.
    pub expired: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, specta::Type)]
pub struct StorageReport {
    pub deleted: u32,
    pub bytes_freed: u64,
}

/// Emitted after a deletion that removed at least one file.
#[derive(Clone, Debug, Serialize, specta::Type, tauri_specta::Event)]
pub struct StorageOptimized(pub StorageReport);

/// Managed state.
#[derive(Clone)]
pub struct Storage {
    db: Db,
    library: Library,
    app: Option<AppHandle>,
}

impl Storage {
    /// `app` is `None` in tests (no events emitted).
    pub fn new(db: Db, library: Library, app: Option<AppHandle>) -> Self {
        Self { db, library, app }
    }

    pub fn settings(&self) -> AppResult<StorageSettings> {
        Ok(StorageSettings {
            auto: self.setting(AUTO_KEY)?.is_none_or(|v| v == "1"),
            days: self
                .setting(DAYS_KEY)?
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_DAYS),
        })
    }

    pub fn set_settings(&self, settings: &StorageSettings) -> AppResult<StorageSettings> {
        self.set_setting(AUTO_KEY, if settings.auto { "1" } else { "0" })?;
        self.set_setting(DAYS_KEY, &settings.days.max(1).to_string())?;
        self.settings()
    }

    /// Tracks whose file can be deleted, oldest play first. `protected` are never
    /// listed (tracks of the play queue).
    pub fn list(
        &self,
        downloads: &Downloads,
        protected: &HashSet<TrackId>,
    ) -> AppResult<Vec<StorageTrack>> {
        let limit = now_secs() - i64::from(self.settings()?.days) * DAY_SECS;
        let rows = {
            let conn = self.db.conn();
            let mut stmt = conn.prepare(
                "SELECT t.id, t.path, t.title, t.artist, COALESCE(MAX(p.started_at), t.added_at)
                 FROM tracks t LEFT JOIN plays p ON p.track_id = t.id
                 WHERE t.path IS NOT NULL AND t.youtube_id IS NOT NULL
                   AND t.favorite = 0 AND t.suggested = 0
                 GROUP BY t.id ORDER BY 5",
            )?;
            stmt.query_map([], |row| {
                Ok((
                    row.get::<_, TrackId>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?
        };
        Ok(rows
            .into_iter()
            .filter(|(id, ..)| !protected.contains(id))
            .filter_map(|(track_id, path, title, artist, last_played)| {
                let size = std::fs::metadata(downloads.owned_file(&path)?).ok()?.len();
                Some(StorageTrack {
                    track_id,
                    title,
                    artist,
                    size,
                    last_played,
                    expired: last_played < limit,
                })
            })
            .collect())
    }

    /// Deletes the audio files of the expired tracks.
    pub fn optimize(
        &self,
        downloads: &Downloads,
        protected: &HashSet<TrackId>,
    ) -> AppResult<StorageReport> {
        let ids: Vec<_> = self
            .list(downloads, protected)?
            .into_iter()
            .filter(|t| t.expired)
            .map(|t| t.track_id)
            .collect();
        self.delete_files(&ids, downloads, protected)
    }

    /// Deletes the audio file of each track (the eligibility is checked again, the
    /// ids may come from a stale list). Emits [`StorageOptimized`] and
    /// `LibraryChanged` once.
    pub fn delete_files(
        &self,
        ids: &[TrackId],
        downloads: &Downloads,
        protected: &HashSet<TrackId>,
    ) -> AppResult<StorageReport> {
        let mut report = StorageReport::default();
        for &id in ids {
            let track = self.library.get(id)?;
            if track.favorite || track.youtube_id.is_none() || protected.contains(&id) {
                continue;
            }
            let Some(file) = track.path.as_deref().and_then(|p| downloads.owned_file(p)) else {
                continue;
            };
            let size = std::fs::metadata(&file).map(|m| m.len()).unwrap_or(0);
            if let Err(e) = std::fs::remove_file(&file) {
                tracing::warn!("cannot delete {}: {e}", file.display());
                continue;
            }
            self.library.clear_path(id)?;
            report.deleted += 1;
            report.bytes_freed += size;
        }
        if report.deleted > 0 {
            self.library.notify_changed()?;
            if let Some(app) = &self.app
                && let Err(e) = StorageOptimized(report.clone()).emit(app)
            {
                tracing::warn!("emit StorageOptimized failed: {e}");
            }
        }
        Ok(report)
    }

    fn setting(&self, key: &str) -> AppResult<Option<String>> {
        Ok(self
            .db
            .conn()
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()?)
    }

    fn set_setting(&self, key: &str, value: &str) -> AppResult<()> {
        self.db.conn().execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::tests::temp_dir;

    struct Fixture {
        storage: Storage,
        library: Library,
        downloads: Downloads,
        dir: std::path::PathBuf,
    }

    fn setup() -> Fixture {
        let db = Db::in_memory();
        let library = Library::new(db.clone(), None);
        let dir = temp_dir("storage");
        let downloads = Downloads::new(dir.clone(), dir.join("cache"));
        Fixture {
            storage: Storage::new(db, library.clone(), None),
            library,
            downloads,
            dir,
        }
    }

    impl Fixture {
        /// A downloaded track whose last activity was `age_days` ago.
        fn track(&self, name: &str, age_days: i64, youtube_id: Option<&str>) -> TrackId {
            let path = self.dir.join(name);
            std::fs::write(&path, b"audio").unwrap();
            let id = crate::library::tests::insert_track(&self.library, &path.to_string_lossy());
            let added = now_secs() - age_days * DAY_SECS;
            self.storage
                .db
                .conn()
                .execute(
                    "UPDATE tracks SET added_at = ?1, youtube_id = ?2 WHERE id = ?3",
                    params![added, youtube_id, id],
                )
                .unwrap();
            id
        }
    }

    #[test]
    fn deletes_only_old_unprotected_files_and_keeps_tracks() {
        let f = setup();
        let old = f.track("old.opus", 40, Some("aaaaaaaaaaa"));
        let recent = f.track("recent.opus", 2, Some("bbbbbbbbbbb"));
        let fav = f.track("fav.opus", 400, Some("ccccccccccc"));
        let local = f.track("local.opus", 400, None);
        let queued = f.track("queued.opus", 400, Some("ddddddddddd"));
        f.library.set_favorite(fav, true).unwrap();

        let protected = HashSet::from([queued]);
        let report = f.storage.optimize(&f.downloads, &protected).unwrap();

        assert_eq!(report.deleted, 1);
        assert_eq!(report.bytes_freed, 5);
        assert!(!f.dir.join("old.opus").exists());
        for kept in ["recent.opus", "fav.opus", "local.opus", "queued.opus"] {
            assert!(f.dir.join(kept).exists(), "{kept}");
        }
        assert_eq!(f.library.get(old).unwrap().path, None);
        assert!(f.library.get(recent).unwrap().path.is_some());
        assert!(f.library.get(local).unwrap().path.is_some());
    }

    #[test]
    fn last_play_resets_the_clock_and_manual_delete_ignores_age() {
        let f = setup();
        let played = f.track("played.opus", 100, Some("aaaaaaaaaaa"));
        let fresh = f.track("fresh.opus", 1, Some("bbbbbbbbbbb"));
        f.storage
            .db
            .conn()
            .execute(
                "INSERT INTO plays (track_id, title, source, started_at, listened, completed)
                 VALUES (?1, 't', 'library', ?2, 10, 1)",
                params![played, now_secs() - DAY_SECS],
            )
            .unwrap();

        let none = HashSet::new();
        assert_eq!(f.storage.optimize(&f.downloads, &none).unwrap().deleted, 0);
        let listed = f.storage.list(&f.downloads, &none).unwrap();
        assert_eq!(listed.len(), 2);
        assert!(listed.iter().all(|t| !t.expired));

        let report = f
            .storage
            .delete_files(&[fresh], &f.downloads, &none)
            .unwrap();
        assert_eq!(report.deleted, 1);
        assert!(!f.dir.join("fresh.opus").exists());
    }

    #[test]
    fn files_outside_the_music_folder_are_ignored() {
        let f = setup();
        let outside = temp_dir("storage_outside");
        let path = outside.join("x.opus");
        std::fs::write(&path, b"audio").unwrap();
        let id = crate::library::tests::insert_track(&f.library, &path.to_string_lossy());
        f.storage
            .db
            .conn()
            .execute(
                "UPDATE tracks SET added_at = 0, youtube_id = 'eeeeeeeeeee' WHERE id = ?1",
                [id],
            )
            .unwrap();
        let none = HashSet::new();
        assert!(f.storage.list(&f.downloads, &none).unwrap().is_empty());
        assert_eq!(
            f.storage
                .delete_files(&[id], &f.downloads, &none)
                .unwrap()
                .deleted,
            0
        );
        assert!(path.exists());
    }

    #[test]
    fn settings_default_and_roundtrip() {
        let f = setup();
        assert_eq!(
            f.storage.settings().unwrap(),
            StorageSettings {
                auto: true,
                days: 30
            }
        );
        let s = f
            .storage
            .set_settings(&StorageSettings {
                auto: false,
                days: 0,
            })
            .unwrap();
        assert_eq!(
            s,
            StorageSettings {
                auto: false,
                days: 1
            }
        );
    }
}
