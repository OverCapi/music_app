//! SQLite storage for the library, playlists and settings.
//!
//! Schema changes are numbered files in `migrations/`, applied in order at startup
//! (`PRAGMA user_version` tracks the current one). To change the schema, add a new
//! file and append it to [`MIGRATIONS`]; never edit a migration that has shipped.

mod import_legacy;

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

use crate::error::{AppError, AppResult};

const MIGRATIONS: &[M<'static>] = &[
    M::up(include_str!("migrations/001_initial.sql")),
    M::up(include_str!("migrations/002_plays.sql")),
    M::up(include_str!("migrations/003_remote_tracks.sql")).foreign_key_check(),
    M::up(include_str!("migrations/004_sync.sql")),
];

/// Shared handle to the database. Cheap to clone; queries are short so one
/// connection behind a mutex is enough.
// ponytail: single connection, a pool if long queries ever block the UI.
#[derive(Clone)]
pub struct Db(Arc<Mutex<Connection>>);

impl Db {
    /// Opens (or creates) the database, applies pending migrations, then imports the
    /// JSON files of older versions found in `data_dir` (once).
    pub fn open(file: &Path, data_dir: &Path) -> AppResult<Self> {
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut conn = Connection::open(file)?;
        Self::init(&mut conn)?;
        import_legacy::run(&mut conn, data_dir)?;
        crate::library::fill_match_keys(&conn)?;
        Ok(Self(Arc::new(Mutex::new(conn))))
    }

    /// In-memory database with the full schema, for tests.
    #[cfg(test)]
    pub fn in_memory() -> Self {
        let mut conn = Connection::open_in_memory().unwrap();
        Self::init(&mut conn).unwrap();
        Self(Arc::new(Mutex::new(conn)))
    }

    fn init(conn: &mut Connection) -> AppResult<()> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        // Foreign keys are off during migrations (the bundled SQLite enables them by
        // default): rebuilding a table (drop + rename) would otherwise
        // cascade-delete the rows that reference it. Migrations that rebuild one
        // check foreign keys before committing.
        conn.pragma_update(None, "foreign_keys", "OFF")?;
        Migrations::from_slice(MIGRATIONS)
            .to_latest(conn)
            .map_err(|e| AppError::Db(e.to_string()))?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Ok(())
    }

    pub fn conn(&self) -> MutexGuard<'_, Connection> {
        self.0.lock().unwrap()
    }
}

/// Unix timestamp in seconds.
pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_valid() {
        Migrations::from_slice(MIGRATIONS).validate().unwrap();
    }

    #[test]
    fn remote_tracks_migration_merges_youtube_duplicates() {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrations::from_slice(&MIGRATIONS[..2])
            .to_latest(&mut conn)
            .unwrap();
        conn.execute_batch(
            "INSERT INTO tracks (id, path, title, youtube_id, added_at) VALUES
                 (1, '/a.opus', 'A', 'aaaaaaaaaaa', 1),
                 (2, '/a2.opus', 'A', 'aaaaaaaaaaa', 2),
                 (3, '/b.mp3', 'B', NULL, 3);
             INSERT INTO playlists (id, name, created_at) VALUES (1, 'P', 1), (2, 'Q', 1);
             INSERT INTO playlist_tracks VALUES (1, 2, 0), (1, 3, 1), (2, 1, 0), (2, 2, 1);
             INSERT INTO plays (track_id, title, source, started_at, listened, completed)
                 VALUES (2, 'A', 'library', 1, 1.0, 1);",
        )
        .unwrap();
        Db::init(&mut conn).unwrap();

        let ids = |sql: &str| -> Vec<i64> {
            let mut stmt = conn.prepare(sql).unwrap();
            stmt.query_map([], |r| r.get(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };
        assert_eq!(ids("SELECT id FROM tracks ORDER BY id"), [1, 3]);
        assert_eq!(
            ids("SELECT track_id FROM playlist_tracks WHERE playlist_id = 1 ORDER BY position"),
            [1, 3]
        );
        assert_eq!(
            ids("SELECT track_id FROM playlist_tracks WHERE playlist_id = 2"),
            [1]
        );
        assert_eq!(ids("SELECT track_id FROM plays"), [1]);
        // Foreign keys are back on: deleting a track still cleans playlists.
        conn.execute("DELETE FROM tracks WHERE id = 3", []).unwrap();
        assert_eq!(
            ids("SELECT track_id FROM playlist_tracks WHERE playlist_id = 1"),
            [1]
        );
        // Tracks without file are allowed, a YouTube id only once.
        conn.execute(
            "INSERT INTO tracks (title, youtube_id, added_at) VALUES ('C', 'ccccccccccc', 1)",
            [],
        )
        .unwrap();
        assert!(
            conn.execute(
                "INSERT INTO tracks (title, youtube_id, added_at) VALUES ('A', 'aaaaaaaaaaa', 1)",
                [],
            )
            .is_err()
        );
    }
}
