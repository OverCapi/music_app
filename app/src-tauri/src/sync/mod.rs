//! Imports the user's library from other services into the app library, one way
//! (service → app; nothing is written to the service).
//!
//! YouTube Music: yt-dlp reads the liked songs (playlist `LM`) and the library
//! playlists (`/feed/playlists`) with the cookies of the user's browser
//! (`--cookies-from-browser`, browser chosen in the settings, cookies never stored by
//! the app). Tracks are added without downloading them ([`Library::add_remote`],
//! which reuses tracks already in the library), and each service playlist fills a
//! local playlist linked in `sync_playlists`; a sync replaces its content.
//!
//! Progress is pushed as [`SyncProgress`] events.

pub mod commands;
mod spotify;

pub use spotify::SpotifyImportReport;

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use group_protocol::is_valid_youtube_id;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_specta::Event;

use crate::db::{Db, now_secs};
use crate::error::{AppError, AppResult};
use crate::library::{Library, RemoteTrack};
use crate::playlists::{PlaylistId, Playlists};
use crate::ytdlp;

const YOUTUBE_MUSIC: &str = "youtube_music";
/// YouTube's id of the "liked music" playlist.
const LIKED_ID: &str = "LM";
const LIKED_NAME: &str = "Titres likés YouTube Music";
/// Playlists of the signed-in user (needs the cookies).
const LIBRARY_PLAYLISTS_URL: &str = "https://www.youtube.com/feed/playlists";
/// Auto-generated playlists that are not the user's (liked videos, watch later…).
const SKIPPED_PLAYLISTS: &[&str] = &["LL", "WL", LIKED_ID];
const BROWSER_KEY: &str = "ytmusic_browser";
const SYNCED_AT_KEY: &str = "ytmusic_synced_at";

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
pub struct SyncSettings {
    /// Browser whose cookies yt-dlp uses for YouTube Music.
    pub ytmusic_browser: Option<String>,
    /// Unix timestamp (seconds) of the last successful YouTube Music sync.
    pub ytmusic_synced_at: Option<i64>,
    /// Browsers that can be chosen.
    pub browsers: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, specta::Type)]
pub struct SyncReport {
    pub playlists: u32,
    /// Tracks seen in the service (a track in several playlists counts once).
    pub tracks: u32,
    /// Tracks that were not in the library before.
    pub new_tracks: u32,
    /// Playlists that could not be read (the others were imported).
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize, specta::Type, tauri_specta::Event)]
pub struct SyncProgress {
    /// What is being read ("Titres likés", a playlist name…).
    pub step: String,
    /// Playlists done so far, out of `total` (0 while the list isn't known).
    pub done: u32,
    pub total: u32,
}

/// Managed state.
#[derive(Clone)]
pub struct LibrarySync {
    db: Db,
    library: Library,
    playlists: Playlists,
    app: Option<AppHandle>,
    running: Arc<AtomicBool>,
}

/// One `--flat-playlist --dump-json` line (a video of a playlist, or a playlist of
/// the library feed).
#[derive(Clone, Debug, Default, Deserialize)]
struct FlatEntry {
    id: String,
    title: Option<String>,
    channel: Option<String>,
    uploader: Option<String>,
    duration: Option<f64>,
}

impl LibrarySync {
    /// `app` is `None` in tests (no events emitted).
    pub fn new(db: Db, library: Library, playlists: Playlists, app: Option<AppHandle>) -> Self {
        Self {
            db,
            library,
            playlists,
            app,
            running: Arc::default(),
        }
    }

    pub fn settings(&self) -> AppResult<SyncSettings> {
        Ok(SyncSettings {
            ytmusic_browser: self.db.setting(BROWSER_KEY)?,
            ytmusic_synced_at: self.db.setting(SYNCED_AT_KEY)?.and_then(|v| v.parse().ok()),
            browsers: ytdlp::COOKIE_BROWSERS
                .iter()
                .map(|b| (*b).to_owned())
                .collect(),
        })
    }

    pub fn set_ytmusic_browser(&self, browser: &str) -> AppResult<SyncSettings> {
        if !ytdlp::COOKIE_BROWSERS.contains(&browser) {
            return Err(AppError::invalid("Navigateur non pris en charge"));
        }
        self.db.set_setting(BROWSER_KEY, browser)?;
        self.settings()
    }

    /// Imports the liked songs and the library playlists of YouTube Music.
    pub async fn sync_youtube_music(&self) -> AppResult<SyncReport> {
        let browser = self.db.setting(BROWSER_KEY)?.ok_or_else(|| {
            AppError::invalid("Choisis d'abord le navigateur connecté à YouTube Music")
        })?;
        if self.running.swap(true, Ordering::SeqCst) {
            return Err(AppError::invalid("Une synchronisation est déjà en cours"));
        }
        let result = self.run_youtube_music(&browser).await;
        self.running.store(false, Ordering::SeqCst);
        if result.is_ok() {
            self.db
                .set_setting(SYNCED_AT_KEY, &now_secs().to_string())?;
        }
        result
    }

    async fn run_youtube_music(&self, browser: &str) -> AppResult<SyncReport> {
        let before = self.library.tracks()?.len();
        let mut report = SyncReport::default();
        let mut seen = HashSet::new();

        // Liked songs first: if the cookies don't work, this is where it shows.
        self.progress("Titres likés", 0, 0);
        let liked = read_playlist(browser, LIKED_ID).await?;
        self.import_playlist(LIKED_ID, LIKED_NAME, &liked, &mut seen)?;
        report.playlists += 1;

        self.progress("Liste des playlists", 0, 0);
        // The liked songs are already imported: a failure here is only reported.
        let lists: Vec<FlatEntry> = match ytdlp::run_json(&[
            LIBRARY_PLAYLISTS_URL,
            "--flat-playlist",
            "--dump-json",
            "--cookies-from-browser",
            browser,
        ])
        .await
        {
            Ok(lists) => lists,
            Err(e) => {
                report.errors.push(format!("Liste des playlists : {e}"));
                Vec::new()
            }
        };
        let lists: Vec<_> = lists
            .into_iter()
            .filter(|p| !SKIPPED_PLAYLISTS.contains(&p.id.as_str()))
            .filter(|p| ytdlp::playlist_url(&p.id).is_ok())
            .collect();
        let total = lists.len() as u32;
        for (done, list) in lists.iter().enumerate() {
            let name = list
                .title
                .clone()
                .filter(|t| !t.trim().is_empty())
                .unwrap_or_else(|| "Playlist YouTube Music".into());
            self.progress(&name, done as u32, total);
            match read_playlist(browser, &list.id).await {
                Ok(entries) => {
                    self.import_playlist(&list.id, &name, &entries, &mut seen)?;
                    report.playlists += 1;
                }
                Err(e) => report.errors.push(format!("{name} : {e}")),
            }
        }

        report.tracks = seen.len() as u32;
        report.new_tracks = self.library.tracks()?.len().saturating_sub(before) as u32;
        Ok(report)
    }

    /// Adds the playlist's tracks to the library and replaces the content of the
    /// linked local playlist (created, or renamed when the service name changed).
    fn import_playlist(
        &self,
        external_id: &str,
        name: &str,
        entries: &[FlatEntry],
        seen: &mut HashSet<String>,
    ) -> AppResult<PlaylistId> {
        let tracks: Vec<RemoteTrack> = entries.iter().filter_map(remote_track).collect();
        seen.extend(tracks.iter().filter_map(|t| t.youtube_id.clone()));
        let track_ids = self.library.add_remote(&tracks)?;
        let playlist_id = self.linked_playlist(external_id, name)?;
        self.playlists.set_tracks(playlist_id, &track_ids)?;
        Ok(playlist_id)
    }

    fn linked_playlist(&self, external_id: &str, name: &str) -> AppResult<PlaylistId> {
        let linked: Option<PlaylistId> = self
            .db
            .conn()
            .query_row(
                "SELECT playlist_id FROM sync_playlists WHERE service = ?1 AND external_id = ?2",
                params![YOUTUBE_MUSIC, external_id],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(id) = linked {
            let current = self.playlists.all()?.into_iter().find(|p| p.id == id);
            if current.is_some_and(|p| p.name != name.trim()) {
                // A name the playlists module refuses (too long…) keeps the old one.
                let _ = self.playlists.rename(id, name);
            }
            return Ok(id);
        }
        let name: String = name.trim().chars().take(100).collect();
        let id = self.playlists.create(&name)?.id;
        self.db.conn().execute(
            "INSERT INTO sync_playlists (service, external_id, playlist_id) VALUES (?1, ?2, ?3)",
            params![YOUTUBE_MUSIC, external_id, id],
        )?;
        Ok(id)
    }

    fn progress(&self, step: &str, done: u32, total: u32) {
        if let Some(app) = &self.app {
            let event = SyncProgress {
                step: step.to_owned(),
                done,
                total,
            };
            if let Err(e) = event.emit(app) {
                tracing::warn!("emit SyncProgress failed: {e}");
            }
        }
    }
}

async fn read_playlist(browser: &str, playlist_id: &str) -> AppResult<Vec<FlatEntry>> {
    let url = ytdlp::playlist_url(playlist_id)?;
    ytdlp::run_json(&[
        url.as_str(),
        "--flat-playlist",
        "--dump-json",
        "--cookies-from-browser",
        browser,
    ])
    .await
}

/// A playlist video as a library track. Artist: the channel ("X - Topic" for
/// YouTube Music tracks); a music video titled "Artist - Title" by a channel named
/// after the artist ("ArtistVEVO") is split. Entries without a valid id or title
/// (deleted or private videos) are skipped.
fn remote_track(e: &FlatEntry) -> Option<RemoteTrack> {
    if !is_valid_youtube_id(&e.id) {
        return None;
    }
    let title = e.title.as_deref()?.trim();
    if title.is_empty() || matches!(title, "[Deleted video]" | "[Private video]") {
        return None;
    }
    let channel = e
        .channel
        .as_deref()
        .or(e.uploader.as_deref())
        .map(str::trim);
    let (artist, title) = match channel {
        Some(c) if c.ends_with(" - Topic") => (Some(c.trim_end_matches(" - Topic")), title),
        _ => match title.split_once(" - ") {
            Some((artist, rest))
                if !rest.trim().is_empty()
                    && channel.is_none_or(|c| squash(c).contains(&squash(artist)))
                    && !squash(artist).is_empty() =>
            {
                (Some(artist.trim()), rest.trim())
            }
            _ => (channel, title),
        },
    };
    Some(RemoteTrack {
        title: title.to_owned(),
        artist: artist.filter(|a| !a.is_empty()).map(str::to_owned),
        album: None,
        duration: e.duration.unwrap_or(0.0),
        youtube_id: Some(e.id.clone()),
    })
}

/// Lowercase letters and digits only ("Daft Punk" → "daftpunk").
fn squash(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, title: &str, channel: &str, duration: f64) -> FlatEntry {
        FlatEntry {
            id: id.into(),
            title: Some(title.into()),
            channel: Some(channel.into()),
            uploader: None,
            duration: Some(duration),
        }
    }

    fn setup() -> (LibrarySync, Library, Playlists) {
        let db = Db::in_memory();
        let library = Library::new(db.clone(), None);
        let playlists = Playlists::new(db.clone(), None);
        (
            LibrarySync::new(db, library.clone(), playlists.clone(), None),
            library,
            playlists,
        )
    }

    #[test]
    fn entries_to_tracks() {
        let topic = remote_track(&entry(
            "aaaaaaaaaaa",
            "Get Lucky",
            "Daft Punk - Topic",
            248.0,
        ))
        .unwrap();
        assert_eq!(topic.artist.as_deref(), Some("Daft Punk"));
        assert_eq!(topic.title, "Get Lucky");
        assert_eq!(topic.youtube_id.as_deref(), Some("aaaaaaaaaaa"));

        let clip = remote_track(&entry(
            "bbbbbbbbbbb",
            "Daft Punk - Get Lucky (Official Video)",
            "DaftPunkVEVO",
            369.0,
        ))
        .unwrap();
        assert_eq!(clip.artist.as_deref(), Some("Daft Punk"));
        assert_eq!(clip.title, "Get Lucky (Official Video)");

        let plain = remote_track(&entry("ccccccccccc", "Song", "Someone", 1.0)).unwrap();
        assert_eq!(plain.artist.as_deref(), Some("Someone"));
        // The dash isn't the artist when the channel is someone else.
        let remix = remote_track(&entry("eeeeeeeeeee", "Song - Remix", "Someone", 1.0)).unwrap();
        assert_eq!(remix.artist.as_deref(), Some("Someone"));
        assert_eq!(remix.title, "Song - Remix");

        assert!(remote_track(&entry("bad", "Song", "A", 1.0)).is_none());
        assert!(remote_track(&entry("ddddddddddd", "[Deleted video]", "", 0.0)).is_none());
    }

    #[test]
    fn import_links_and_replaces_playlists() {
        let (sync, library, playlists) = setup();
        let mut seen = HashSet::new();
        let a = entry("aaaaaaaaaaa", "A", "X - Topic", 100.0);
        let b = entry("bbbbbbbbbbb", "B", "X - Topic", 100.0);
        let c = entry("ccccccccccc", "C", "X - Topic", 100.0);

        let liked = sync
            .import_playlist("LM", LIKED_NAME, &[a.clone(), b.clone()], &mut seen)
            .unwrap();
        let other = sync
            .import_playlist("PL1", "Rock", &[b.clone(), c.clone()], &mut seen)
            .unwrap();
        assert_ne!(liked, other);
        assert_eq!(library.tracks().unwrap().len(), 3);
        assert_eq!(seen.len(), 3);

        // Second sync: same playlist, new content and name; B removed from it.
        let again = sync
            .import_playlist("PL1", "Rock & Roll", &[c.clone(), a.clone()], &mut seen)
            .unwrap();
        assert_eq!(again, other);
        let all = playlists.all().unwrap();
        assert_eq!(all.len(), 2);
        let rock = all.iter().find(|p| p.id == other).unwrap();
        assert_eq!(rock.name, "Rock & Roll");
        let ids = |e: &FlatEntry| library.find_by_youtube_id(&e.id).unwrap().unwrap().id;
        assert_eq!(rock.track_ids, [ids(&c), ids(&a)]);
        // Tracks are never removed from the library by a sync.
        assert_eq!(library.tracks().unwrap().len(), 3);

        // A deleted local playlist is recreated by the next sync.
        playlists.delete(other).unwrap();
        let recreated = sync
            .import_playlist("PL1", "Rock & Roll", std::slice::from_ref(&c), &mut seen)
            .unwrap();
        let all = playlists.all().unwrap();
        assert_eq!(all.len(), 2);
        let rock = all.iter().find(|p| p.id == recreated).unwrap();
        assert_eq!(rock.track_ids, [ids(&c)]);
    }

    #[test]
    fn browser_setting() {
        let (sync, _, _) = setup();
        assert_eq!(sync.settings().unwrap().ytmusic_browser, None);
        assert!(sync.set_ytmusic_browser("netscape").is_err());
        let settings = sync.set_ytmusic_browser("firefox").unwrap();
        assert_eq!(settings.ytmusic_browser.as_deref(), Some("firefox"));
        assert!(settings.browsers.contains(&"chrome".to_owned()));
    }
}
