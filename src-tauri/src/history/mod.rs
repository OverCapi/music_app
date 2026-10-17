//! Listening history: one row per listen (table `plays`), the raw material of the
//! recommendation system. Title/artist/youtube_id are copied into the row so the
//! history survives the track being removed from the library (`track_id` becomes NULL).
//!
//! The audio engine hands a [`FinishedPlay`] over through a [`PlayRecorder`] when a
//! listen ends.

pub mod commands;

use std::sync::Arc;

use rusqlite::{Row, params};
use serde::{Deserialize, Serialize};

use crate::db::Db;
use crate::error::AppResult;
use crate::library::TrackId;

/// Listens shorter than this (seconds) are accidental clicks and are not recorded.
pub const MIN_LISTENED_SECS: f64 = 2.0;

/// Where a listen came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum PlaySource {
    Library,
    Playlist,
    /// Random track picked because the queue ran out.
    Autoplay,
    Group,
    /// Listen started from a generated mix or from the discoveries.
    Mix,
}

impl PlaySource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Library => "library",
            Self::Playlist => "playlist",
            Self::Autoplay => "autoplay",
            Self::Group => "group",
            Self::Mix => "mix",
        }
    }

    /// Unknown strings map to [`PlaySource::Library`] rather than failing.
    pub fn parse(s: &str) -> Self {
        match s {
            "playlist" => Self::Playlist,
            "autoplay" => Self::Autoplay,
            "group" => Self::Group,
            "mix" => Self::Mix,
            _ => Self::Library,
        }
    }
}

/// The track that was listened to.
#[derive(Clone, Debug, PartialEq)]
pub enum PlayedTrack {
    Library(TrackId),
    /// Group tracks are temp files, not in the library.
    Group {
        youtube_id: String,
        title: String,
        artist: Option<String>,
    },
}

/// What the audio engine hands over when a listen ends.
#[derive(Clone, Debug, PartialEq)]
pub struct FinishedPlay {
    pub track: PlayedTrack,
    pub source: PlaySource,
    /// Unix timestamp (seconds).
    pub started_at: i64,
    /// Seconds actually heard.
    pub listened: f64,
    /// Track length in seconds (0 when unknown).
    pub duration: f64,
    /// Reached the end naturally.
    pub completed: bool,
}

/// A recorded listen.
#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
pub struct Play {
    pub id: i64,
    /// `None` for group tracks and for tracks removed from the library since.
    pub track_id: Option<TrackId>,
    pub youtube_id: Option<String>,
    pub title: String,
    pub artist: Option<String>,
    pub source: PlaySource,
    /// Unix timestamp (seconds).
    pub started_at: i64,
    /// Seconds actually heard.
    pub listened: f64,
    /// Seconds (0 when unknown).
    pub duration: f64,
    pub completed: bool,
}

/// Callback given to the audio thread; never panics, logs failures.
pub type PlayRecorder = Arc<dyn Fn(FinishedPlay) + Send + Sync>;

/// Managed state.
#[derive(Clone)]
pub struct History {
    db: Db,
}

const COLUMNS: &str =
    "id, track_id, youtube_id, title, artist, source, started_at, listened, duration, completed";

impl History {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Records a finished listen. Ignored (still `Ok`) when it was shorter than
    /// [`MIN_LISTENED_SECS`] or when a library track no longer exists.
    pub fn record(&self, play: &FinishedPlay) -> AppResult<()> {
        if play.listened < MIN_LISTENED_SECS {
            return Ok(());
        }
        let conn = self.db.conn();
        let (track_id, youtube_id, title, artist) = match &play.track {
            PlayedTrack::Library(id) => {
                let row = conn.query_row(
                    "SELECT youtube_id, title, artist FROM tracks WHERE id = ?1",
                    [id],
                    |row| {
                        Ok((
                            row.get::<_, Option<String>>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, Option<String>>(2)?,
                        ))
                    },
                );
                match row {
                    Ok((youtube_id, title, artist)) => (Some(*id), youtube_id, title, artist),
                    Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(()),
                    Err(e) => return Err(e.into()),
                }
            }
            PlayedTrack::Group {
                youtube_id,
                title,
                artist,
            } => (
                None,
                Some(youtube_id.clone()),
                title.clone(),
                artist.clone(),
            ),
        };
        conn.execute(
            "INSERT INTO plays (track_id, youtube_id, title, artist, source, started_at,
                                listened, duration, completed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                track_id,
                youtube_id,
                title,
                artist,
                play.source.as_str(),
                play.started_at,
                play.listened,
                play.duration,
                play.completed
            ],
        )?;
        Ok(())
    }

    /// The latest listens, newest first.
    pub fn recent(&self, limit: u32) -> AppResult<Vec<Play>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(&format!(
            "SELECT {COLUMNS} FROM plays ORDER BY started_at DESC, id DESC LIMIT ?1"
        ))?;
        let plays = stmt
            .query_map([limit], play_from_row)?
            .collect::<Result<_, _>>()?;
        Ok(plays)
    }

    /// Recorder handed to the audio engine (one insert per call).
    pub fn recorder(&self) -> PlayRecorder {
        let history = self.clone();
        Arc::new(move |play| {
            if let Err(e) = history.record(&play) {
                tracing::error!("recording play failed: {e}");
            }
        })
    }
}

fn play_from_row(row: &Row<'_>) -> rusqlite::Result<Play> {
    Ok(Play {
        id: row.get(0)?,
        track_id: row.get(1)?,
        youtube_id: row.get(2)?,
        title: row.get(3)?,
        artist: row.get(4)?,
        source: PlaySource::parse(&row.get::<_, String>(5)?),
        started_at: row.get(6)?,
        listened: row.get(7)?,
        duration: row.get(8)?,
        completed: row.get(9)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::Library;
    use crate::library::tests::insert_track;

    fn setup() -> (Db, Library, History) {
        let db = Db::in_memory();
        (db.clone(), Library::new(db.clone(), None), History::new(db))
    }

    fn finished(track: PlayedTrack, started_at: i64, listened: f64) -> FinishedPlay {
        FinishedPlay {
            track,
            source: PlaySource::Library,
            started_at,
            listened,
            duration: 200.0,
            completed: false,
        }
    }

    #[test]
    fn library_play_copies_track_metadata() {
        let (db, lib, history) = setup();
        let id = insert_track(&lib, "/a.mp3");
        db.conn()
            .execute(
                "UPDATE tracks SET artist = 'Art', youtube_id = 'yt1' WHERE id = ?1",
                [id],
            )
            .unwrap();
        let mut play = finished(PlayedTrack::Library(id), 10, 30.5);
        play.source = PlaySource::Playlist;
        play.completed = true;
        history.record(&play).unwrap();

        let plays = history.recent(10).unwrap();
        assert_eq!(plays.len(), 1);
        let p = &plays[0];
        assert_eq!(p.track_id, Some(id));
        assert_eq!(p.title, "/a.mp3");
        assert_eq!(p.artist.as_deref(), Some("Art"));
        assert_eq!(p.youtube_id.as_deref(), Some("yt1"));
        assert_eq!(p.source, PlaySource::Playlist);
        assert_eq!((p.started_at, p.listened, p.duration), (10, 30.5, 200.0));
        assert!(p.completed);
    }

    #[test]
    fn group_play_has_no_track_id() {
        let (_, _, history) = setup();
        let mut play = finished(
            PlayedTrack::Group {
                youtube_id: "yt9".into(),
                title: "Song".into(),
                artist: Some("Band".into()),
            },
            5,
            60.0,
        );
        play.source = PlaySource::Group;
        history.record(&play).unwrap();

        let p = &history.recent(1).unwrap()[0];
        assert_eq!(p.track_id, None);
        assert_eq!(p.youtube_id.as_deref(), Some("yt9"));
        assert_eq!(p.title, "Song");
        assert_eq!(p.artist.as_deref(), Some("Band"));
        assert_eq!(p.source, PlaySource::Group);
    }

    #[test]
    fn short_plays_are_ignored() {
        let (_, lib, history) = setup();
        let id = insert_track(&lib, "/a.mp3");
        history
            .record(&finished(PlayedTrack::Library(id), 1, 1.99))
            .unwrap();
        assert!(history.recent(10).unwrap().is_empty());
        history
            .record(&finished(PlayedTrack::Library(id), 1, MIN_LISTENED_SECS))
            .unwrap();
        assert_eq!(history.recent(10).unwrap().len(), 1);
    }

    #[test]
    fn play_of_a_deleted_track_is_ignored() {
        let (_, lib, history) = setup();
        let id = insert_track(&lib, "/a.mp3");
        lib.remove(id).unwrap();
        history
            .record(&finished(PlayedTrack::Library(id), 1, 30.0))
            .unwrap();
        assert!(history.recent(10).unwrap().is_empty());
    }

    #[test]
    fn recent_is_newest_first_and_limited() {
        let (_, lib, history) = setup();
        let id = insert_track(&lib, "/a.mp3");
        for started_at in [10, 30, 20, 30] {
            history
                .record(&finished(PlayedTrack::Library(id), started_at, 5.0))
                .unwrap();
        }
        let all = history.recent(10).unwrap();
        let order: Vec<(i64, i64)> = all.iter().map(|p| (p.started_at, p.id)).collect();
        assert_eq!(order, [(30, 4), (30, 2), (20, 3), (10, 1)]);
        assert_eq!(history.recent(2).unwrap().len(), 2);
        assert_eq!(history.recent(2).unwrap()[0].id, 4);
        assert!(history.recent(0).unwrap().is_empty());
    }

    #[test]
    fn deleting_a_track_keeps_its_plays() {
        let (_, lib, history) = setup();
        let id = insert_track(&lib, "/a.mp3");
        history
            .record(&finished(PlayedTrack::Library(id), 1, 30.0))
            .unwrap();
        lib.remove(id).unwrap();
        let plays = history.recent(10).unwrap();
        assert_eq!(plays.len(), 1);
        assert_eq!(plays[0].track_id, None);
        assert_eq!(plays[0].title, "/a.mp3");
    }

    #[test]
    fn recorder_records_and_source_parse_is_lenient() {
        let (_, lib, history) = setup();
        let id = insert_track(&lib, "/a.mp3");
        let record = history.recorder();
        record(finished(PlayedTrack::Library(id), 1, 30.0));
        assert_eq!(history.recent(10).unwrap().len(), 1);

        for s in [
            PlaySource::Library,
            PlaySource::Playlist,
            PlaySource::Autoplay,
            PlaySource::Group,
            PlaySource::Mix,
        ] {
            assert_eq!(PlaySource::parse(s.as_str()), s);
        }
        assert_eq!(PlaySource::parse("bogus"), PlaySource::Library);
    }
}
