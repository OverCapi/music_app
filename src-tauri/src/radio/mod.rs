//! "Radio" of a seed track: the tracks YouTube suggests around it, cached in table
//! `radio_cache` for [`CACHE_TTL_SECS`]. Dislikes are not applied here (the scoring
//! step filters with `Feedback`).
//!
//! The YouTube Music radio (`RDAMVM<id>`) is tried first, then the classic Mix
//! (`RD<id>`), because it is not verified that the flat output of either contains
//! entries. An empty answer is never cached.
//!
//! Every time-dependent method takes `now` (Unix seconds, see `db::now_secs`).

use std::collections::HashSet;

use group_protocol::is_valid_youtube_id;
use rusqlite::params;
use serde::Serialize;

use crate::db::{Db, now_secs};
use crate::error::{AppError, AppResult};
use crate::search::FlatEntry;
use crate::ytdlp;

pub mod commands;

/// Most tracks kept from one radio.
pub const MAX_RADIO_TRACKS: usize = 50;
/// A cached radio is used for 3 days.
pub const CACHE_TTL_SECS: i64 = 3 * 24 * 3600;
/// How many entries yt-dlp is asked for (some are dropped by `parse_entries`).
const FETCH_LIMIT: &str = "60";

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
pub struct RadioTrack {
    /// YouTube video id (validated).
    pub youtube_id: String,
    pub title: String,
    pub artist: Option<String>,
    /// Seconds, 0 when unknown.
    pub duration: f64,
}

/// Radio entries → tracks: invalid ids, the seed, duplicates and untitled entries
/// are dropped, YouTube's order is kept, at most [`MAX_RADIO_TRACKS`].
pub(crate) fn parse_entries(seed_id: &str, entries: Vec<FlatEntry>) -> Vec<RadioTrack> {
    let mut seen = HashSet::new();
    let mut tracks = Vec::new();
    for e in entries {
        if tracks.len() >= MAX_RADIO_TRACKS {
            break;
        }
        if !is_valid_youtube_id(&e.id) || e.id == seed_id || seen.contains(&e.id) {
            continue;
        }
        let Some(title) = e
            .title
            .map(|t| t.trim().to_owned())
            .filter(|t| !t.is_empty())
        else {
            continue;
        };
        seen.insert(e.id.clone());
        let artist = e
            .channel
            .or(e.uploader)
            .map(|a| {
                let a = a.trim();
                a.strip_suffix(" - Topic").unwrap_or(a).trim().to_owned()
            })
            .filter(|a| !a.is_empty());
        tracks.push(RadioTrack {
            youtube_id: e.id,
            title,
            artist,
            duration: e
                .duration
                .filter(|d| d.is_finite() && *d > 0.0)
                .unwrap_or(0.0),
        });
    }
    tracks
}

#[derive(Clone)]
pub struct Radio {
    db: Db,
}

impl Radio {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// The cached radio of `seed_id`, in order; `None` when absent or older than the TTL.
    pub fn cached(&self, seed_id: &str, now: i64) -> AppResult<Option<Vec<RadioTrack>>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT youtube_id, title, artist, duration FROM radio_cache
             WHERE seed_id = ?1 AND ?2 - fetched_at < ?3 ORDER BY position",
        )?;
        let tracks = stmt
            .query_map(params![seed_id, now, CACHE_TTL_SECS], |r| {
                Ok(RadioTrack {
                    youtube_id: r.get(0)?,
                    title: r.get(1)?,
                    artist: r.get(2)?,
                    duration: r.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok((!tracks.is_empty()).then_some(tracks))
    }

    /// Replaces the cached radio of `seed_id` in one transaction. An empty list
    /// stores nothing (the previous rows are kept).
    pub fn store(&self, seed_id: &str, tracks: &[RadioTrack], now: i64) -> AppResult<()> {
        if tracks.is_empty() {
            return Ok(());
        }
        let mut conn = self.db.conn();
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM radio_cache WHERE seed_id = ?1", [seed_id])?;
        {
            let mut insert = tx.prepare(
                "INSERT INTO radio_cache (seed_id, position, youtube_id, title, artist, duration, fetched_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?;
            for (position, t) in tracks.iter().enumerate() {
                insert.execute(params![
                    seed_id,
                    position as i64,
                    t.youtube_id,
                    t.title,
                    t.artist,
                    t.duration,
                    now
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Deletes the rows older than the TTL.
    pub fn purge_expired(&self, now: i64) -> AppResult<()> {
        self.db.conn().execute(
            "DELETE FROM radio_cache WHERE ?1 - fetched_at >= ?2",
            params![now, CACHE_TTL_SECS],
        )?;
        Ok(())
    }

    /// The radio around `seed_id`: from the cache, else from yt-dlp (music radio,
    /// then classic mix), then cached.
    pub async fn get(&self, seed_id: &str) -> AppResult<Vec<RadioTrack>> {
        let urls = ytdlp::radio_urls(seed_id)?;
        self.purge_expired(now_secs())?;
        if let Some(tracks) = self.cached(seed_id, now_secs())? {
            return Ok(tracks);
        }
        let mut last_error = None;
        for url in &urls {
            let result: AppResult<Vec<FlatEntry>> = ytdlp::run_json(&[
                url.as_str(),
                "--flat-playlist",
                "--dump-json",
                "--playlist-end",
                FETCH_LIMIT,
            ])
            .await;
            match result {
                Ok(entries) => {
                    let tracks = parse_entries(seed_id, entries);
                    if !tracks.is_empty() {
                        self.store(seed_id, &tracks, now_secs())?;
                        return Ok(tracks);
                    }
                    tracing::warn!("radio {url}: aucune piste exploitable");
                    last_error = None;
                }
                Err(e) => {
                    tracing::warn!("radio {url}: {e}");
                    last_error = Some(e);
                }
            }
        }
        Err(last_error
            .unwrap_or_else(|| AppError::Tool("Aucune radio trouvée pour ce titre".into())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Hand-written fixtures in the shape `search::FlatEntry` reads; not real yt-dlp output.
    fn entry(id: &str, title: Option<&str>, channel: Option<&str>) -> FlatEntry {
        FlatEntry {
            id: id.into(),
            title: title.map(Into::into),
            channel: channel.map(Into::into),
            uploader: None,
            duration: Some(200.0),
            view_count: None,
        }
    }

    fn id(n: usize) -> String {
        format!("{n:011}")
    }

    fn track(n: usize) -> RadioTrack {
        RadioTrack {
            youtube_id: id(n),
            title: format!("T{n}"),
            artist: n.is_multiple_of(2).then(|| format!("A{n}")),
            duration: n as f64,
        }
    }

    #[test]
    fn parse_drops_seed_invalid_duplicates_and_untitled() {
        let seed = id(0);
        let entries = vec![
            entry(&seed, Some("Seed"), Some("X")),
            entry("bad id", Some("Bad"), None),
            entry(&id(1), Some("One"), Some("A")),
            entry(&id(1), Some("One again"), Some("B")),
            entry(&id(2), None, Some("A")),
            entry(&id(3), Some("  "), Some("A")),
            entry(&id(4), Some("Four"), None),
        ];
        let tracks = parse_entries(&seed, entries);
        let ids: Vec<_> = tracks.iter().map(|t| t.youtube_id.clone()).collect();
        assert_eq!(ids, vec![id(1), id(4)]);
        assert_eq!(tracks[0].title, "One");
        assert_eq!(tracks[0].artist.as_deref(), Some("A"));
        assert_eq!(tracks[1].artist, None);
    }

    #[test]
    fn parse_artist_fallback_and_topic() {
        let mut e = entry(&id(1), Some("S"), Some("Daft Punk - Topic"));
        let mut f = entry(&id(2), Some("S"), None);
        f.uploader = Some("Uploader".into());
        e.duration = None;
        let tracks = parse_entries(&id(0), vec![e, f]);
        assert_eq!(tracks[0].artist.as_deref(), Some("Daft Punk"));
        assert_eq!(tracks[0].duration, 0.0);
        assert_eq!(tracks[1].artist.as_deref(), Some("Uploader"));
    }

    #[test]
    fn parse_caps_at_max() {
        let entries = (1..=80).map(|n| entry(&id(n), Some("T"), None)).collect();
        let tracks = parse_entries(&id(0), entries);
        assert_eq!(tracks.len(), MAX_RADIO_TRACKS);
        assert_eq!(tracks[0].youtube_id, id(1));
    }

    #[test]
    fn cache_roundtrip_keeps_order() {
        let radio = Radio::new(Db::in_memory());
        let tracks: Vec<_> = [5, 1, 9, 3].into_iter().map(track).collect();
        radio.store("seed", &tracks, 1000).unwrap();
        assert_eq!(radio.cached("seed", 1000).unwrap(), Some(tracks));
        assert_eq!(radio.cached("other", 1000).unwrap(), None);
    }

    #[test]
    fn cache_expires_after_ttl() {
        let radio = Radio::new(Db::in_memory());
        radio.store("seed", &[track(1)], 1000).unwrap();
        assert!(
            radio
                .cached("seed", 1000 + CACHE_TTL_SECS - 1)
                .unwrap()
                .is_some()
        );
        assert!(
            radio
                .cached("seed", 1000 + CACHE_TTL_SECS)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn empty_store_is_not_cached() {
        let radio = Radio::new(Db::in_memory());
        radio.store("seed", &[], 1000).unwrap();
        assert_eq!(radio.cached("seed", 1000).unwrap(), None);
    }

    #[test]
    fn store_replaces_previous_rows() {
        let radio = Radio::new(Db::in_memory());
        radio
            .store("seed", &[track(1), track(2), track(3)], 1000)
            .unwrap();
        radio.store("seed", &[track(7)], 2000).unwrap();
        assert_eq!(radio.cached("seed", 2000).unwrap(), Some(vec![track(7)]));
    }

    #[test]
    fn purge_removes_only_expired() {
        let radio = Radio::new(Db::in_memory());
        radio.store("old", &[track(1)], 0).unwrap();
        radio.store("new", &[track(2)], CACHE_TTL_SECS).unwrap();
        radio.purge_expired(CACHE_TTL_SECS).unwrap();
        let count: i64 = radio
            .db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM radio_cache WHERE seed_id = 'old'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
        assert!(radio.cached("new", CACHE_TTL_SECS).unwrap().is_some());
    }

    #[tokio::test]
    #[ignore = "needs network and yt-dlp: cargo test -p music_app radio_live -- --ignored --nocapture"]
    async fn radio_live() {
        let seed = "dQw4w9WgXcQ";
        let tracks = Radio::new(Db::in_memory()).get(seed).await.unwrap();
        println!("{} pistes", tracks.len());
        for t in tracks.iter().take(15) {
            println!("{} — {}", t.title, t.artist.as_deref().unwrap_or("?"));
        }
        assert!(!tracks.is_empty());
        assert!(tracks.iter().all(|t| t.youtube_id != seed));
    }
}
