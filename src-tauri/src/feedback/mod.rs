//! Explicit feedback on what the app suggests: a like or a "never suggest again" on
//! one track (by `youtube_id`) or one artist (normalized name). Table `feedback`.
//!
//! A target has at most one feedback: setting a new one replaces the old one.
//! Rows keep their `label` and have no foreign key to `tracks`, so feedback
//! survives the track leaving the library.
//!
//! Every change emits [`FeedbackChanged`] with the full state.

use std::collections::HashSet;

use group_protocol::is_valid_youtube_id;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_specta::Event;

use crate::db::{Db, now_secs};
use crate::error::{AppError, AppResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum FeedbackKind {
    Like,
    Dislike,
}

impl FeedbackKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Like => "like",
            Self::Dislike => "dislike",
        }
    }

    fn from_db(s: &str) -> AppResult<Self> {
        match s {
            "like" => Ok(Self::Like),
            "dislike" => Ok(Self::Dislike),
            other => Err(AppError::Db(format!("type de retour inconnu : {other}"))),
        }
    }
}

/// What a feedback applies to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FeedbackTarget {
    /// `label` is the track title, for display.
    Track {
        youtube_id: String,
        label: String,
    },
    Artist {
        name: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
pub struct TrackFeedback {
    pub youtube_id: String,
    pub label: String,
    pub kind: FeedbackKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
pub struct ArtistFeedback {
    /// Normalized key (see [`normalize_artist`]).
    pub artist: String,
    pub label: String,
    pub kind: FeedbackKind,
}

/// Both lists, newest first.
#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
pub struct FeedbackState {
    pub tracks: Vec<TrackFeedback>,
    pub artists: Vec<ArtistFeedback>,
}

#[derive(Clone, Debug, Serialize, specta::Type, tauri_specta::Event)]
pub struct FeedbackChanged(pub FeedbackState);

/// Managed state.
#[derive(Clone)]
pub struct Feedback {
    db: Db,
    app: Option<AppHandle>,
}

const BAD_ID: &str = "Identifiant YouTube invalide";
const EMPTY_ARTIST: &str = "Nom d'artiste vide";

/// Key under which an artist is stored: trimmed and lowercased.
pub fn normalize_artist(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Validated form of a target, ready for SQL.
enum Key {
    Track(String),
    Artist(String),
}

impl Key {
    fn of(target: &FeedbackTarget) -> AppResult<(Self, String)> {
        match target {
            FeedbackTarget::Track { youtube_id, label } => {
                if !is_valid_youtube_id(youtube_id) {
                    return Err(AppError::invalid(BAD_ID));
                }
                Ok((Self::Track(youtube_id.clone()), label.clone()))
            }
            FeedbackTarget::Artist { name } => {
                let key = normalize_artist(name);
                if key.is_empty() {
                    return Err(AppError::invalid(EMPTY_ARTIST));
                }
                Ok((Self::Artist(key), name.trim().to_owned()))
            }
        }
    }
}

impl Feedback {
    /// `app` is `None` in tests (no events emitted).
    pub fn new(db: Db, app: Option<AppHandle>) -> Self {
        Self { db, app }
    }

    /// Sets (or replaces) the feedback on `target`.
    pub fn set(&self, target: &FeedbackTarget, kind: FeedbackKind) -> AppResult<()> {
        let (key, label) = Key::of(target)?;
        {
            // A single upsert statement: atomic, one row per target.
            let conn = self.db.conn();
            match key {
                Key::Track(id) => conn.execute(
                    "INSERT INTO feedback (kind, youtube_id, label, created_at)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT (youtube_id) WHERE youtube_id IS NOT NULL
                     DO UPDATE SET kind = excluded.kind, label = excluded.label,
                                   created_at = excluded.created_at",
                    params![kind.as_str(), id, label, now_secs()],
                )?,
                Key::Artist(artist) => conn.execute(
                    "INSERT INTO feedback (kind, artist, label, created_at)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT (artist) WHERE artist IS NOT NULL
                     DO UPDATE SET kind = excluded.kind, label = excluded.label,
                                   created_at = excluded.created_at",
                    params![kind.as_str(), artist, label, now_secs()],
                )?,
            };
        }
        self.changed()
    }

    /// Removes the feedback on `target`; fine if there was none.
    pub fn clear(&self, target: &FeedbackTarget) -> AppResult<()> {
        let (key, _) = Key::of(target)?;
        {
            let conn = self.db.conn();
            match key {
                Key::Track(id) => {
                    conn.execute("DELETE FROM feedback WHERE youtube_id = ?1", [id])?
                }
                Key::Artist(artist) => {
                    conn.execute("DELETE FROM feedback WHERE artist = ?1", [artist])?
                }
            };
        }
        self.changed()
    }

    pub fn state(&self) -> AppResult<FeedbackState> {
        let conn = self.db.conn();
        let mut tracks = Vec::new();
        let mut stmt = conn.prepare(
            "SELECT youtube_id, label, kind FROM feedback
             WHERE youtube_id IS NOT NULL ORDER BY created_at DESC, id DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        for row in rows {
            let (youtube_id, label, kind) = row?;
            tracks.push(TrackFeedback {
                youtube_id,
                label,
                kind: FeedbackKind::from_db(&kind)?,
            });
        }
        let mut artists = Vec::new();
        let mut stmt = conn.prepare(
            "SELECT artist, label, kind FROM feedback
             WHERE artist IS NOT NULL ORDER BY created_at DESC, id DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        for row in rows {
            let (artist, label, kind) = row?;
            artists.push(ArtistFeedback {
                artist,
                label,
                kind: FeedbackKind::from_db(&kind)?,
            });
        }
        Ok(FeedbackState { tracks, artists })
    }

    #[allow(dead_code)] // used by the recommendation scoring (later step)
    pub fn disliked_youtube_ids(&self) -> AppResult<HashSet<String>> {
        self.keys("youtube_id", FeedbackKind::Dislike)
    }

    #[allow(dead_code)] // used by the recommendation scoring (later step)
    pub fn disliked_artists(&self) -> AppResult<HashSet<String>> {
        self.keys("artist", FeedbackKind::Dislike)
    }

    #[allow(dead_code)] // used by the recommendation scoring (later step)
    pub fn liked_youtube_ids(&self) -> AppResult<HashSet<String>> {
        self.keys("youtube_id", FeedbackKind::Like)
    }

    #[allow(dead_code)] // used by the recommendation scoring (later step)
    pub fn liked_artists(&self) -> AppResult<HashSet<String>> {
        self.keys("artist", FeedbackKind::Like)
    }

    /// `column` is one of two internal constants, never user input.
    fn keys(&self, column: &str, kind: FeedbackKind) -> AppResult<HashSet<String>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(&format!(
            "SELECT {column} FROM feedback WHERE kind = ?1 AND {column} IS NOT NULL"
        ))?;
        let keys = stmt
            .query_map([kind.as_str()], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        Ok(keys)
    }

    /// Emits [`FeedbackChanged`] with the full state.
    fn changed(&self) -> AppResult<()> {
        if let Some(app) = &self.app {
            let state = self.state()?;
            if let Err(e) = FeedbackChanged(state).emit(app) {
                tracing::warn!("emit FeedbackChanged failed: {e}");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID_A: &str = "dQw4w9WgXcQ";
    const ID_B: &str = "abcdefghijk";

    fn feedback() -> (Feedback, Db) {
        let db = Db::in_memory();
        (Feedback::new(db.clone(), None), db)
    }

    fn track(id: &str, label: &str) -> FeedbackTarget {
        FeedbackTarget::Track {
            youtube_id: id.into(),
            label: label.into(),
        }
    }

    fn artist(name: &str) -> FeedbackTarget {
        FeedbackTarget::Artist { name: name.into() }
    }

    fn count(db: &Db) -> i64 {
        db.conn()
            .query_row("SELECT COUNT(*) FROM feedback", [], |r| r.get(0))
            .unwrap()
    }

    fn set_of<const N: usize>(items: [&str; N]) -> HashSet<String> {
        items.into_iter().map(String::from).collect()
    }

    #[test]
    fn normalize_trims_and_lowercases() {
        assert_eq!(normalize_artist("  Daft Punk "), "daft punk");
        assert_eq!(normalize_artist("ÉLUARD"), "éluard");
        assert_eq!(normalize_artist("   "), "");
    }

    #[test]
    fn set_then_state() {
        let (fb, _) = feedback();
        fb.set(&track(ID_A, "Song"), FeedbackKind::Like).unwrap();
        fb.set(&artist(" Daft Punk "), FeedbackKind::Dislike)
            .unwrap();
        let state = fb.state().unwrap();
        assert_eq!(
            state.tracks,
            vec![TrackFeedback {
                youtube_id: ID_A.into(),
                label: "Song".into(),
                kind: FeedbackKind::Like
            }]
        );
        assert_eq!(
            state.artists,
            vec![ArtistFeedback {
                artist: "daft punk".into(),
                label: "Daft Punk".into(),
                kind: FeedbackKind::Dislike
            }]
        );
    }

    #[test]
    fn replacing_keeps_one_row_with_new_kind_and_label() {
        let (fb, db) = feedback();
        fb.set(&track(ID_A, "Old"), FeedbackKind::Like).unwrap();
        fb.set(&track(ID_A, "New"), FeedbackKind::Dislike).unwrap();
        assert_eq!(count(&db), 1);
        let t = fb.state().unwrap().tracks;
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].kind, FeedbackKind::Dislike);
        assert_eq!(t[0].label, "New");

        fb.set(&artist("X"), FeedbackKind::Dislike).unwrap();
        fb.set(&artist("X"), FeedbackKind::Like).unwrap();
        assert_eq!(count(&db), 2);
        assert_eq!(fb.state().unwrap().artists[0].kind, FeedbackKind::Like);
    }

    #[test]
    fn artist_names_are_normalized() {
        let (fb, db) = feedback();
        fb.set(&artist(" Daft Punk "), FeedbackKind::Like).unwrap();
        fb.set(&artist("daft punk"), FeedbackKind::Dislike).unwrap();
        assert_eq!(count(&db), 1);
        let a = fb.state().unwrap().artists;
        assert_eq!(a[0].artist, "daft punk");
        assert_eq!(a[0].label, "daft punk");
        assert_eq!(a[0].kind, FeedbackKind::Dislike);
    }

    #[test]
    fn clear_removes_and_absent_is_ok() {
        let (fb, db) = feedback();
        fb.clear(&track(ID_A, "")).unwrap();
        fb.clear(&artist("nobody")).unwrap();
        fb.set(&track(ID_A, "Song"), FeedbackKind::Like).unwrap();
        fb.set(&artist("Daft Punk"), FeedbackKind::Like).unwrap();
        fb.clear(&track(ID_A, "")).unwrap();
        fb.clear(&artist(" DAFT PUNK")).unwrap();
        assert_eq!(count(&db), 0);
    }

    #[test]
    fn invalid_input_is_rejected_and_writes_nothing() {
        let (fb, db) = feedback();
        for bad in ["", "short", "has space!!", "waytoolongidentifier"] {
            assert!(matches!(
                fb.set(&track(bad, "x"), FeedbackKind::Like),
                Err(AppError::Invalid(_))
            ));
        }
        for bad in ["", "   "] {
            assert!(matches!(
                fb.set(&artist(bad), FeedbackKind::Dislike),
                Err(AppError::Invalid(_))
            ));
        }
        assert!(fb.clear(&artist("  ")).is_err());
        assert_eq!(count(&db), 0);
    }

    #[test]
    fn state_is_newest_first() {
        let (fb, db) = feedback();
        fb.set(&track(ID_A, "A"), FeedbackKind::Like).unwrap();
        fb.set(&track(ID_B, "B"), FeedbackKind::Like).unwrap();
        fb.set(&artist("one"), FeedbackKind::Like).unwrap();
        fb.set(&artist("two"), FeedbackKind::Like).unwrap();
        // Same second: `id DESC` breaks the tie.
        let ids: Vec<_> = fb
            .state()
            .unwrap()
            .tracks
            .iter()
            .map(|t| t.label.clone())
            .collect();
        assert_eq!(ids, ["B", "A"]);
        // Explicit timestamps win over ids.
        db.conn()
            .execute(
                "UPDATE feedback SET created_at = 1000 WHERE label = 'B'",
                [],
            )
            .unwrap();
        db.conn()
            .execute(
                "UPDATE feedback SET created_at = 1000 WHERE label = 'two'",
                [],
            )
            .unwrap();
        let state = fb.state().unwrap();
        assert_eq!(state.tracks[0].label, "A");
        assert_eq!(state.artists[0].label, "one");
    }

    #[test]
    fn helper_queries() {
        let (fb, _) = feedback();
        fb.set(&track(ID_A, "A"), FeedbackKind::Like).unwrap();
        fb.set(&track(ID_B, "B"), FeedbackKind::Dislike).unwrap();
        fb.set(&artist("Liked One"), FeedbackKind::Like).unwrap();
        fb.set(&artist("Bad One"), FeedbackKind::Dislike).unwrap();
        assert_eq!(fb.liked_youtube_ids().unwrap(), set_of([ID_A]));
        assert_eq!(fb.disliked_youtube_ids().unwrap(), set_of([ID_B]));
        assert_eq!(fb.liked_artists().unwrap(), set_of(["liked one"]));
        assert_eq!(fb.disliked_artists().unwrap(), set_of(["bad one"]));
    }

    #[test]
    fn feedback_survives_track_deletion() {
        let (fb, db) = feedback();
        db.conn()
            .execute(
                "INSERT INTO tracks (title, added_at, youtube_id) VALUES ('Song', 1, ?1)",
                [ID_A],
            )
            .unwrap();
        fb.set(&track(ID_A, "Song"), FeedbackKind::Dislike).unwrap();
        db.conn().execute("DELETE FROM tracks", []).unwrap();
        assert_eq!(fb.disliked_youtube_ids().unwrap(), set_of([ID_A]));
    }
}
