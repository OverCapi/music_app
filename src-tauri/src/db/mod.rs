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

const MIGRATIONS: &[M<'static>] = &[M::up(include_str!("migrations/001_initial.sql"))];

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
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Migrations::from_slice(MIGRATIONS)
            .to_latest(conn)
            .map_err(|e| AppError::Db(e.to_string()))
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
}
