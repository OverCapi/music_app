//! One-time import of the JSON files written by older versions
//! (`library.json`, `playlists.json`, `account.json`). Each imported file is renamed
//! to `*.json.bak`; the `legacy_imported` setting, written in the same transaction,
//! guarantees the import never runs twice even if a rename fails.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::error::AppResult;

#[derive(Deserialize)]
struct LegacyTrack {
    path: String,
    title: String,
    artist: Option<String>,
    album: Option<String>,
    duration: f64,
    added_at: i64,
    #[serde(default)]
    youtube_id: Option<String>,
}

#[derive(Deserialize)]
struct LegacyPlaylist {
    name: String,
    /// Track paths, in order.
    tracks: Vec<String>,
    created_at: i64,
}

#[derive(Deserialize)]
struct LegacyAccount {
    server_url: Option<String>,
    username: Option<String>,
    token: Option<String>,
}

pub fn run(conn: &mut Connection, data_dir: &Path) -> AppResult<()> {
    let done: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'legacy_imported'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if done.is_some() {
        return Ok(());
    }
    let library = read::<Vec<LegacyTrack>>(data_dir, "library.json");
    let playlists = read::<Vec<LegacyPlaylist>>(data_dir, "playlists.json");
    let account = read::<LegacyAccount>(data_dir, "account.json");
    if library.is_none() && playlists.is_none() && account.is_none() {
        return Ok(());
    }

    let tx = conn.transaction()?;
    let mut imported = Vec::new();
    if let Some((file, tracks)) = library {
        let mut insert = tx.prepare(
            "INSERT OR IGNORE INTO tracks (path, title, artist, album, duration, added_at, youtube_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        for t in &tracks {
            insert.execute(params![
                t.path,
                t.title,
                t.artist,
                t.album,
                t.duration,
                t.added_at,
                t.youtube_id
            ])?;
        }
        tracing::info!("imported {} tracks from {}", tracks.len(), file.display());
        imported.push(file);
    }
    if let Some((file, playlists)) = playlists {
        let mut track_id = tx.prepare("SELECT id FROM tracks WHERE path = ?1")?;
        let mut insert_entry = tx.prepare(
            "INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position)
             VALUES (?1, ?2, ?3)",
        )?;
        for p in &playlists {
            tx.execute(
                "INSERT INTO playlists (name, created_at) VALUES (?1, ?2)",
                params![p.name, p.created_at],
            )?;
            let playlist_id = tx.last_insert_rowid();
            for (position, path) in p.tracks.iter().enumerate() {
                // Paths no longer in the library are dropped.
                let id: Option<i64> = track_id.query_row([path], |row| row.get(0)).optional()?;
                if let Some(id) = id {
                    insert_entry.execute(params![playlist_id, id, position as i64])?;
                }
            }
        }
        tracing::info!(
            "imported {} playlists from {}",
            playlists.len(),
            file.display()
        );
        imported.push(file);
    }
    if let Some((file, account)) = account {
        let settings = [
            ("server_url", account.server_url),
            ("username", account.username),
            ("token", account.token),
        ];
        for (key, value) in settings {
            if let Some(value) = value {
                tx.execute(
                    "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                    params![key, value],
                )?;
            }
        }
        imported.push(file);
    }
    tx.execute(
        "INSERT INTO settings (key, value) VALUES ('legacy_imported', '1')",
        [],
    )?;
    tx.commit()?;

    for file in imported {
        let mut bak = file.clone().into_os_string();
        bak.push(".bak");
        if let Err(e) = std::fs::rename(&file, bak) {
            tracing::warn!("could not rename {}: {e}", file.display());
        }
    }
    Ok(())
}

/// Parses `data_dir/name`. Missing → `None`; corrupt → logged and `None` (left in place).
fn read<T: DeserializeOwned>(data_dir: &Path, name: &str) -> Option<(PathBuf, T)> {
    let file = data_dir.join(name);
    let json = std::fs::read_to_string(&file).ok()?;
    match serde_json::from_str(&json) {
        Ok(value) => Some((file, value)),
        Err(e) => {
            tracing::warn!("ignoring corrupt {}: {e}", file.display());
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::db::Db;
    use crate::library::Library;
    use crate::library::tests::temp_dir;
    use crate::playlists::Playlists;

    #[test]
    fn imports_and_renames_legacy_files() {
        let dir = temp_dir("import");
        std::fs::write(
            dir.join("library.json"),
            r#"[
                {"path": "/m/a.mp3", "title": "A", "artist": "X", "album": null,
                 "duration": 12.5, "added_at": 100},
                {"path": "/m/b.opus", "title": "B", "artist": null, "album": null,
                 "duration": 3.0, "added_at": 200, "youtube_id": "yt1"}
            ]"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("playlists.json"),
            r#"[{"id": "abc", "name": "Mix", "created_at": 5,
                 "tracks": ["/m/b.opus", "/m/unknown.mp3", "/m/a.mp3"]}]"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("account.json"),
            r#"{"server_url": "http://s", "username": "bob", "token": null}"#,
        )
        .unwrap();

        let db = Db::open(&dir.join("music.db"), &dir).unwrap();
        let lib = Library::new(db.clone(), None);
        let tracks = lib.tracks().unwrap();
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].youtube_id.as_deref(), Some("yt1"));
        let (b, a) = (tracks[0].id, tracks[1].id);

        let playlists = Playlists::new(db.clone(), None).all().unwrap();
        assert_eq!(playlists.len(), 1);
        assert_eq!(playlists[0].name, "Mix");
        assert_eq!(playlists[0].track_ids, [b, a]);

        let settings: Vec<(String, String)> = db
            .conn()
            .prepare("SELECT key, value FROM settings ORDER BY key")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            settings,
            [
                ("legacy_imported".to_owned(), "1".to_owned()),
                ("server_url".to_owned(), "http://s".to_owned()),
                ("username".to_owned(), "bob".to_owned())
            ]
        );

        for name in ["library", "playlists", "account"] {
            assert!(!dir.join(format!("{name}.json")).exists());
            assert!(dir.join(format!("{name}.json.bak")).exists());
        }
        drop(db);
        // Reopening imports nothing twice.
        let db = Db::open(&dir.join("music.db"), &dir).unwrap();
        assert_eq!(Playlists::new(db, None).all().unwrap().len(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
