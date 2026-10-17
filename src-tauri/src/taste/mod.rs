//! Taste profile: what the user likes, derived from the listening history
//! ([`crate::history`]) and the explicit feedback ([`crate::feedback`]).
//!
//! [`compute`] is a pure function (no DB, no clock: `now` is injected). Each listen
//! is worth `base * decay * source weight`, summed per track and per artist. Likes
//! add a flat bonus; dislikes exclude the track or the artist (and all its tracks).

pub mod commands;

use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::db::now_secs;
use crate::error::AppResult;
use crate::feedback::{Feedback, FeedbackKind, FeedbackState, normalize_artist};
use crate::history::{History, Play, PlaySource};

/// A listen loses half of its value every this many days.
pub const HALF_LIFE_DAYS: f64 = 14.0;
/// Value of a listen interrupted before [`SKIP_SECS`] (a negative signal).
pub const SKIP_PENALTY: f64 = -0.5;
/// Under this many seconds heard (and not completed), a listen is a skip.
pub const SKIP_SECS: f64 = 30.0;
/// Fraction of the track heard from which a listen counts as a full one.
pub const FULL_LISTEN_RATIO: f64 = 0.5;
/// Value of a partial listen when the duration is unknown.
pub const UNKNOWN_DURATION_VALUE: f64 = 0.5;
/// Bonus for a liked track (also given to its artist) or a liked artist.
pub const LIKE_BONUS: f64 = 3.0;
/// Source weights.
pub const GROUP_WEIGHT: f64 = 0.5;
pub const AUTOPLAY_WEIGHT: f64 = 0.5;
pub const DEFAULT_WEIGHT: f64 = 1.0;
pub const MAX_TRACKS: usize = 50;
pub const MAX_ARTISTS: usize = 20;
/// How many recent listens [`Taste::profile`] looks at.
pub const HISTORY_LIMIT: u32 = 500;

const SECS_PER_DAY: f64 = 86_400.0;

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
pub struct TasteTrack {
    pub youtube_id: String,
    pub title: String,
    pub artist: Option<String>,
    pub score: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
pub struct TasteArtist {
    /// Display name, as in the most recent listen.
    pub artist: String,
    pub score: f64,
}

/// Both lists sorted by score, best first; only positive scores.
#[derive(Clone, Debug, Default, PartialEq, Serialize, specta::Type)]
pub struct TasteProfile {
    pub tracks: Vec<TasteTrack>,
    pub artists: Vec<TasteArtist>,
}

fn source_weight(source: PlaySource) -> f64 {
    match source {
        PlaySource::Group => GROUP_WEIGHT,
        PlaySource::Autoplay => AUTOPLAY_WEIGHT,
        PlaySource::Library | PlaySource::Playlist => DEFAULT_WEIGHT,
    }
}

fn base_value(play: &Play) -> f64 {
    let ratio = (play.duration > 0.0).then(|| play.listened / play.duration);
    if play.completed || ratio.is_some_and(|r| r >= FULL_LISTEN_RATIO) {
        1.0
    } else if play.listened < SKIP_SECS {
        SKIP_PENALTY
    } else {
        ratio.unwrap_or(UNKNOWN_DURATION_VALUE)
    }
}

fn play_value(play: &Play, now: i64) -> f64 {
    let age_days = ((now - play.started_at).max(0) as f64) / SECS_PER_DAY;
    let decay = 0.5_f64.powf(age_days / HALF_LIFE_DAYS);
    base_value(play) * decay * source_weight(play.source)
}

struct TrackAcc {
    title: String,
    artist: Option<String>,
    latest: i64,
    score: f64,
}

struct ArtistAcc {
    display: String,
    latest: i64,
    score: f64,
}

/// Pure scoring; see the module doc.
pub fn compute(plays: &[Play], feedback: &FeedbackState, now: i64) -> TasteProfile {
    let disliked_ids: HashSet<&str> = feedback
        .tracks
        .iter()
        .filter(|f| f.kind == FeedbackKind::Dislike)
        .map(|f| f.youtube_id.as_str())
        .collect();
    let disliked_artists: HashSet<&str> = feedback
        .artists
        .iter()
        .filter(|f| f.kind == FeedbackKind::Dislike)
        .map(|f| f.artist.as_str())
        .collect();

    let mut tracks: HashMap<&str, TrackAcc> = HashMap::new();
    let mut artists: HashMap<String, ArtistAcc> = HashMap::new();

    for play in plays {
        let value = play_value(play, now);
        if let Some(id) = play.youtube_id.as_deref() {
            let acc = tracks.entry(id).or_insert_with(|| TrackAcc {
                title: play.title.clone(),
                artist: play.artist.clone(),
                latest: i64::MIN,
                score: 0.0,
            });
            acc.score += value;
            if play.started_at > acc.latest {
                acc.latest = play.started_at;
                acc.title.clone_from(&play.title);
                acc.artist.clone_from(&play.artist);
            }
        }
        if let Some(name) = play.artist.as_deref() {
            let key = normalize_artist(name);
            if key.is_empty() {
                continue;
            }
            let acc = artists.entry(key).or_insert_with(|| ArtistAcc {
                display: name.trim().to_owned(),
                latest: i64::MIN,
                score: 0.0,
            });
            acc.score += value;
            if play.started_at > acc.latest {
                acc.latest = play.started_at;
                name.trim().clone_into(&mut acc.display);
            }
        }
    }

    for f in feedback
        .tracks
        .iter()
        .filter(|f| f.kind == FeedbackKind::Like)
    {
        let acc = tracks.entry(&f.youtube_id).or_insert_with(|| TrackAcc {
            title: f.label.clone(),
            artist: None,
            latest: i64::MIN,
            score: 0.0,
        });
        acc.score += LIKE_BONUS;
        // The liked track's artist gets the bonus too, when known.
        if let Some(key) = acc.artist.as_deref().map(normalize_artist)
            && let Some(artist) = artists.get_mut(&key)
        {
            artist.score += LIKE_BONUS;
        }
    }
    for f in feedback
        .artists
        .iter()
        .filter(|f| f.kind == FeedbackKind::Like)
    {
        artists
            .entry(f.artist.clone())
            .or_insert_with(|| ArtistAcc {
                display: f.label.clone(),
                latest: i64::MIN,
                score: 0.0,
            })
            .score += LIKE_BONUS;
    }

    let mut track_list: Vec<TasteTrack> = tracks
        .into_iter()
        .filter(|(id, acc)| {
            acc.score > 0.0
                && !disliked_ids.contains(id)
                && !acc
                    .artist
                    .as_deref()
                    .is_some_and(|a| disliked_artists.contains(normalize_artist(a).as_str()))
        })
        .map(|(id, acc)| TasteTrack {
            youtube_id: id.to_owned(),
            title: acc.title,
            artist: acc.artist,
            score: acc.score,
        })
        .collect();
    // Ties broken by id so the output is deterministic.
    track_list.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.youtube_id.cmp(&b.youtube_id))
    });
    track_list.truncate(MAX_TRACKS);

    let mut artist_list: Vec<(String, ArtistAcc)> = artists
        .into_iter()
        .filter(|(key, acc)| acc.score > 0.0 && !disliked_artists.contains(key.as_str()))
        .collect();
    artist_list.sort_by(|a, b| b.1.score.total_cmp(&a.1.score).then_with(|| a.0.cmp(&b.0)));
    artist_list.truncate(MAX_ARTISTS);

    TasteProfile {
        tracks: track_list,
        artists: artist_list
            .into_iter()
            .map(|(_, acc)| TasteArtist {
                artist: acc.display,
                score: acc.score,
            })
            .collect(),
    }
}

/// Managed state.
#[derive(Clone)]
pub struct Taste {
    history: History,
    feedback: Feedback,
}

impl Taste {
    pub fn new(history: History, feedback: Feedback) -> Self {
        Self { history, feedback }
    }

    pub fn profile(&self) -> AppResult<TasteProfile> {
        let plays = self.history.recent(HISTORY_LIMIT)?;
        let feedback = self.feedback.state()?;
        Ok(compute(&plays, &feedback, now_secs()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feedback::{ArtistFeedback, TrackFeedback};

    const NOW: i64 = 1_000_000_000;
    const DAY: i64 = 86_400;

    fn play(id: &str, artist: &str, age_days: i64) -> Play {
        Play {
            id: 0,
            track_id: None,
            youtube_id: Some(id.into()),
            title: format!("title {id}"),
            artist: Some(artist.into()),
            source: PlaySource::Library,
            started_at: NOW - age_days * DAY,
            listened: 200.0,
            duration: 200.0,
            completed: true,
        }
    }

    fn no_feedback() -> FeedbackState {
        FeedbackState {
            tracks: vec![],
            artists: vec![],
        }
    }

    fn track_fb(id: &str, kind: FeedbackKind) -> TrackFeedback {
        TrackFeedback {
            youtube_id: id.into(),
            label: "label".into(),
            kind,
        }
    }

    fn artist_fb(name: &str, kind: FeedbackKind) -> ArtistFeedback {
        ArtistFeedback {
            artist: normalize_artist(name),
            label: name.into(),
            kind,
        }
    }

    fn approx(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} != {b}");
    }

    #[test]
    fn empty_input() {
        assert_eq!(compute(&[], &no_feedback(), NOW), TasteProfile::default());
    }

    #[test]
    fn decay_halves_after_half_life() {
        let p = compute(
            &[play("aaaaaaaaaaa", "A", 0), play("bbbbbbbbbbb", "B", 14)],
            &no_feedback(),
            NOW,
        );
        approx(p.tracks[0].score, 1.0);
        approx(p.tracks[1].score, 0.5);
        // A play "in the future" is not boosted.
        let p = compute(&[play("aaaaaaaaaaa", "A", -3)], &no_feedback(), NOW);
        approx(p.tracks[0].score, 1.0);
    }

    #[test]
    fn skip_is_negative() {
        let mut skip = play("aaaaaaaaaaa", "A", 0);
        skip.completed = false;
        skip.listened = 5.0;
        let p = compute(&[skip.clone()], &no_feedback(), NOW);
        assert!(p.tracks.is_empty() && p.artists.is_empty());
        // A skip cancels part of a full listen.
        let p = compute(&[play("aaaaaaaaaaa", "A", 0), skip], &no_feedback(), NOW);
        approx(p.tracks[0].score, 0.5);
        approx(p.artists[0].score, 0.5);
    }

    #[test]
    fn partial_and_unknown_duration() {
        let mut partial = play("aaaaaaaaaaa", "A", 0);
        partial.completed = false;
        partial.listened = 60.0; // 30% of 200 s, past the skip threshold
        let mut unknown = play("bbbbbbbbbbb", "B", 0);
        unknown.completed = false;
        unknown.duration = 0.0;
        unknown.listened = 40.0;
        let mut half = play("ccccccccccc", "C", 0);
        half.completed = false;
        half.listened = 100.0;
        let p = compute(&[partial, unknown, half], &no_feedback(), NOW);
        let score = |id: &str| p.tracks.iter().find(|t| t.youtube_id == id).unwrap().score;
        approx(score("aaaaaaaaaaa"), 0.3);
        approx(score("bbbbbbbbbbb"), 0.5);
        approx(score("ccccccccccc"), 1.0);
    }

    #[test]
    fn group_and_autoplay_weigh_half() {
        let mut g = play("aaaaaaaaaaa", "A", 0);
        g.source = PlaySource::Group;
        let mut a = play("bbbbbbbbbbb", "B", 0);
        a.source = PlaySource::Autoplay;
        let mut pl = play("ccccccccccc", "C", 0);
        pl.source = PlaySource::Playlist;
        let p = compute(&[g, a, pl], &no_feedback(), NOW);
        assert_eq!(p.tracks[0].youtube_id, "ccccccccccc");
        approx(p.tracks[0].score, 1.0);
        approx(p.tracks[1].score, 0.5);
        approx(p.tracks[2].score, 0.5);
    }

    #[test]
    fn plays_without_youtube_id_count_for_artists_only() {
        let mut p0 = play("aaaaaaaaaaa", "Daft Punk", 1);
        p0.youtube_id = None;
        let p1 = play("bbbbbbbbbbb", " daft punk ", 0);
        let p = compute(&[p0, p1], &no_feedback(), NOW);
        assert_eq!(p.tracks.len(), 1);
        assert_eq!(p.artists.len(), 1);
        assert_eq!(p.artists[0].artist, "daft punk");
        assert!(p.artists[0].score > 1.9);
    }

    #[test]
    fn likes_add_a_bonus() {
        let mut fb = no_feedback();
        fb.tracks.push(track_fb("aaaaaaaaaaa", FeedbackKind::Like));
        fb.artists.push(artist_fb("Other", FeedbackKind::Like));
        let p = compute(&[play("aaaaaaaaaaa", "Art", 0)], &fb, NOW);
        approx(p.tracks[0].score, 1.0 + LIKE_BONUS);
        let art = p.artists.iter().find(|a| a.artist == "Art").unwrap();
        approx(art.score, 1.0 + LIKE_BONUS);
        let other = p.artists.iter().find(|a| a.artist == "Other").unwrap();
        approx(other.score, LIKE_BONUS);
    }

    #[test]
    fn track_dislike_excludes_only_that_track() {
        let mut fb = no_feedback();
        fb.tracks
            .push(track_fb("aaaaaaaaaaa", FeedbackKind::Dislike));
        let p = compute(
            &[play("aaaaaaaaaaa", "Art", 0), play("bbbbbbbbbbb", "Art", 0)],
            &fb,
            NOW,
        );
        assert_eq!(p.tracks.len(), 1);
        assert_eq!(p.tracks[0].youtube_id, "bbbbbbbbbbb");
        assert_eq!(p.artists.len(), 1);
    }

    #[test]
    fn artist_dislike_excludes_artist_and_its_tracks() {
        let mut fb = no_feedback();
        fb.artists.push(artist_fb("ART", FeedbackKind::Dislike));
        let p = compute(
            &[
                play("aaaaaaaaaaa", " art ", 0),
                play("bbbbbbbbbbb", "Fine", 0),
            ],
            &fb,
            NOW,
        );
        assert_eq!(p.tracks.len(), 1);
        assert_eq!(p.tracks[0].youtube_id, "bbbbbbbbbbb");
        assert_eq!(p.artists.len(), 1);
        assert_eq!(p.artists[0].artist, "Fine");
    }

    #[test]
    fn sorted_and_capped() {
        let plays: Vec<Play> = (0..70)
            .map(|i| {
                let mut p = play(&format!("{i:011}"), &format!("artist {i}"), i);
                p.started_at = NOW - i * DAY;
                p
            })
            .collect();
        let p = compute(&plays, &no_feedback(), NOW);
        assert_eq!(p.tracks.len(), MAX_TRACKS);
        assert_eq!(p.artists.len(), MAX_ARTISTS);
        assert!(p.tracks.windows(2).all(|w| w[0].score >= w[1].score));
        assert!(p.artists.windows(2).all(|w| w[0].score >= w[1].score));
        assert_eq!(p.tracks[0].youtube_id, format!("{:011}", 0));
    }

    #[test]
    fn profile_reads_history_and_feedback() {
        use crate::db::Db;
        use crate::history::{FinishedPlay, PlayedTrack};
        let db = Db::in_memory();
        let history = History::new(db.clone());
        let feedback = Feedback::new(db, None);
        history
            .record(&FinishedPlay {
                track: PlayedTrack::Group {
                    youtube_id: "aaaaaaaaaaa".into(),
                    title: "T".into(),
                    artist: Some("Art".into()),
                },
                source: PlaySource::Library,
                started_at: now_secs(),
                listened: 100.0,
                duration: 100.0,
                completed: true,
            })
            .unwrap();
        let p = Taste::new(history, feedback).profile().unwrap();
        assert_eq!(p.tracks.len(), 1);
        assert_eq!(p.artists[0].artist, "Art");
    }
}
