//! User playlists: named, ordered lists of library tracks (tables `playlists`,
//! `playlist_tracks`). A track appears at most once per playlist.
//!
//! Every change emits [`PlaylistsChanged`] with the full list.

pub mod commands;

use std::collections::HashMap;

use rusqlite::{Connection, params};
use serde::Serialize;
use tauri::AppHandle;
use tauri_specta::Event;

use crate::db::{Db, now_secs};
use crate::error::{AppError, AppResult};
use crate::library::TrackId;

pub type PlaylistId = i64;

const MAX_NAME_LEN: usize = 100;

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
pub struct Playlist {
    pub id: PlaylistId,
    pub name: String,
    /// In play order.
    pub track_ids: Vec<TrackId>,
    /// Unix timestamp (seconds).
    pub created_at: i64,
    /// Display order within the pinned / unpinned groups.
    pub position: u32,
    /// Pinned playlists are listed first.
    pub pinned: bool,
}

#[derive(Clone, Debug, Serialize, specta::Type, tauri_specta::Event)]
pub struct PlaylistsChanged {
    pub playlists: Vec<Playlist>,
}

/// Managed state.
#[derive(Clone)]
pub struct Playlists {
    db: Db,
    app: Option<AppHandle>,
}

impl Playlists {
    /// `app` is `None` in tests (no events emitted).
    pub fn new(db: Db, app: Option<AppHandle>) -> Self {
        Self { db, app }
    }

    /// All playlists: pinned first, then by position.
    pub fn all(&self) -> AppResult<Vec<Playlist>> {
        let conn = self.db.conn();
        let mut tracks: HashMap<PlaylistId, Vec<TrackId>> = HashMap::new();
        let mut stmt = conn.prepare(
            "SELECT playlist_id, track_id FROM playlist_tracks ORDER BY playlist_id, position",
        )?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            tracks.entry(row.get(0)?).or_default().push(row.get(1)?);
        }
        let mut stmt = conn.prepare(
            "SELECT id, name, created_at, position, pinned FROM playlists
             ORDER BY pinned DESC, position, id",
        )?;
        let playlists = stmt
            .query_map([], |row| {
                let id = row.get(0)?;
                Ok(Playlist {
                    id,
                    name: row.get(1)?,
                    track_ids: tracks.remove(&id).unwrap_or_default(),
                    created_at: row.get(2)?,
                    position: row.get(3)?,
                    pinned: row.get(4)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(playlists)
    }

    /// Name is trimmed, non-empty, at most 100 chars.
    pub fn create(&self, name: &str) -> AppResult<Playlist> {
        let name = clean_name(name)?;
        let created_at = now_secs();
        let id = {
            let conn = self.db.conn();
            conn.execute(
                "INSERT INTO playlists (name, created_at, position)
                 VALUES (?1, ?2, (SELECT COALESCE(MAX(position) + 1, 0) FROM playlists))",
                params![name, created_at],
            )?;
            conn.last_insert_rowid()
        };
        self.changed()?;
        let position = self.db.conn().query_row(
            "SELECT position FROM playlists WHERE id = ?1",
            [id],
            |row| row.get(0),
        )?;
        Ok(Playlist {
            id,
            name,
            track_ids: Vec::new(),
            created_at,
            position,
            pinned: false,
        })
    }

    pub fn rename(&self, id: PlaylistId, name: &str) -> AppResult<()> {
        let name = clean_name(name)?;
        let updated = self.db.conn().execute(
            "UPDATE playlists SET name = ?1 WHERE id = ?2",
            params![name, id],
        )?;
        if updated == 0 {
            return Err(gone());
        }
        self.changed()
    }

    /// Rewrites the display order: `ids` lists playlists in their new order.
    /// Unknown ids are ignored; playlists not listed keep their position.
    pub fn reorder(&self, ids: &[PlaylistId]) -> AppResult<()> {
        {
            let mut conn = self.db.conn();
            let tx = conn.transaction()?;
            {
                let mut update = tx.prepare("UPDATE playlists SET position = ?1 WHERE id = ?2")?;
                for (position, id) in ids.iter().enumerate() {
                    update.execute(params![position as i64, id])?;
                }
            }
            tx.commit()?;
        }
        self.changed()
    }

    pub fn set_pinned(&self, id: PlaylistId, pinned: bool) -> AppResult<()> {
        let updated = self.db.conn().execute(
            "UPDATE playlists SET pinned = ?1 WHERE id = ?2",
            params![pinned, id],
        )?;
        if updated == 0 {
            return Err(gone());
        }
        self.changed()
    }

    pub fn delete(&self, id: PlaylistId) -> AppResult<()> {
        self.db
            .conn()
            .execute("DELETE FROM playlists WHERE id = ?1", [id])?;
        self.changed()
    }

    /// Appends tracks, skipping ones already in the playlist.
    pub fn add_tracks(&self, id: PlaylistId, track_ids: &[TrackId]) -> AppResult<()> {
        {
            let mut conn = self.db.conn();
            let tx = conn.transaction()?;
            ensure_exists(&tx, id)?;
            let mut position: i64 = tx.query_row(
                "SELECT COALESCE(MAX(position) + 1, 0) FROM playlist_tracks WHERE playlist_id = ?1",
                [id],
                |row| row.get(0),
            )?;
            {
                let mut insert = tx.prepare(
                    "INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position)
                     VALUES (?1, ?2, ?3)",
                )?;
                for track_id in track_ids {
                    position += insert.execute(params![id, track_id, position])? as i64;
                }
            }
            tx.commit()?;
        }
        self.changed()
    }

    /// Replaces the content of a playlist (duplicates keep their first position).
    pub fn set_tracks(&self, id: PlaylistId, track_ids: &[TrackId]) -> AppResult<()> {
        {
            let mut conn = self.db.conn();
            let tx = conn.transaction()?;
            ensure_exists(&tx, id)?;
            tx.execute("DELETE FROM playlist_tracks WHERE playlist_id = ?1", [id])?;
            {
                let mut insert = tx.prepare(
                    "INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position)
                     VALUES (?1, ?2, ?3)",
                )?;
                let mut position: i64 = 0;
                for track_id in track_ids {
                    position += insert.execute(params![id, track_id, position])? as i64;
                }
            }
            tx.commit()?;
        }
        self.changed()
    }

    pub fn remove_track(&self, id: PlaylistId, track_id: TrackId) -> AppResult<()> {
        {
            let conn = self.db.conn();
            ensure_exists(&conn, id)?;
            conn.execute(
                "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
                params![id, track_id],
            )?;
        }
        self.changed()
    }

    /// Re-emits [`PlaylistsChanged`] (after a library change cascaded into playlists).
    pub fn notify_changed(&self) -> AppResult<()> {
        self.changed()
    }

    fn changed(&self) -> AppResult<()> {
        if let Some(app) = &self.app {
            let playlists = self.all()?;
            if let Err(e) = (PlaylistsChanged { playlists }).emit(app) {
                tracing::warn!("emit PlaylistsChanged failed: {e}");
            }
        }
        Ok(())
    }
}

fn gone() -> AppError {
    AppError::not_found("Cette playlist n'existe plus")
}

fn ensure_exists(conn: &Connection, id: PlaylistId) -> AppResult<()> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM playlists WHERE id = ?1)",
        [id],
        |row| row.get(0),
    )?;
    if exists { Ok(()) } else { Err(gone()) }
}

fn clean_name(name: &str) -> AppResult<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::invalid("Le nom de la playlist est vide"));
    }
    if name.chars().count() > MAX_NAME_LEN {
        return Err(AppError::invalid(format!(
            "Le nom de la playlist est trop long ({MAX_NAME_LEN} caractères maximum)"
        )));
    }
    Ok(name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::Library;
    use crate::library::tests::insert_track;

    fn setup() -> (Library, Playlists) {
        let db = Db::in_memory();
        (Library::new(db.clone(), None), Playlists::new(db, None))
    }

    #[test]
    fn names_are_validated() {
        let (_, pl) = setup();
        assert_eq!(pl.create("  Rock  ").unwrap().name, "Rock");
        assert!(pl.create("   ").is_err());
        assert!(pl.create(&"x".repeat(101)).is_err());
        assert!(pl.create(&"é".repeat(100)).is_ok());
    }

    #[test]
    fn set_tracks_replaces_the_content() {
        let (lib, pl) = setup();
        let [a, b, c] = ["/a", "/b", "/c"].map(|p| insert_track(&lib, p));
        let p = pl.create("P").unwrap();
        pl.add_tracks(p.id, &[a, b]).unwrap();
        pl.set_tracks(p.id, &[c, a, c]).unwrap();
        assert_eq!(pl.all().unwrap()[0].track_ids, [c, a]);
        assert!(pl.set_tracks(p.id + 1, &[a]).is_err());
    }

    #[test]
    fn crud_and_ordering() {
        let (lib, pl) = setup();
        let [a, b, c] = ["/a", "/b", "/c"].map(|p| insert_track(&lib, p));
        let p1 = pl.create("One").unwrap();
        let p2 = pl.create("Two").unwrap();

        pl.add_tracks(p1.id, &[b, a]).unwrap();
        pl.add_tracks(p1.id, &[a, c, c]).unwrap();
        pl.rename(p2.id, "Deux").unwrap();
        let all = pl.all().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].track_ids, [b, a, c]);
        assert_eq!(all[1].name, "Deux");

        pl.remove_track(p1.id, a).unwrap();
        pl.add_tracks(p1.id, &[a]).unwrap();
        assert_eq!(pl.all().unwrap()[0].track_ids, [b, c, a]);

        pl.delete(p2.id).unwrap();
        assert!(pl.rename(p2.id, "x").is_err());
        assert!(pl.add_tracks(p2.id, &[a]).is_err());
        assert_eq!(pl.all().unwrap().len(), 1);
    }

    #[test]
    fn reorder_and_pin() {
        let (_, pl) = setup();
        let [a, b, c] = ["A", "B", "C"].map(|n| pl.create(n).unwrap().id);
        let names = |pl: &Playlists| -> Vec<String> {
            pl.all().unwrap().into_iter().map(|p| p.name).collect()
        };
        assert_eq!(names(&pl), ["A", "B", "C"]);
        pl.reorder(&[c, a, b]).unwrap();
        assert_eq!(names(&pl), ["C", "A", "B"]);
        pl.set_pinned(b, true).unwrap();
        assert_eq!(names(&pl), ["B", "C", "A"]);
        assert!(pl.all().unwrap()[0].pinned);
        // New playlists go last.
        pl.create("D").unwrap();
        assert_eq!(names(&pl), ["B", "C", "A", "D"]);
        assert!(pl.set_pinned(999, true).is_err());
    }

    #[test]
    fn removing_a_library_track_cascades() {
        let (lib, pl) = setup();
        let a = insert_track(&lib, "/a");
        let b = insert_track(&lib, "/b");
        let p = pl.create("P").unwrap();
        pl.add_tracks(p.id, &[a, b]).unwrap();
        lib.remove(a).unwrap();
        assert_eq!(pl.all().unwrap()[0].track_ids, [b]);
    }
}
