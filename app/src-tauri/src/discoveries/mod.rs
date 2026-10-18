//! "Découvertes de la semaine": a playlist of ~[`DISCOVERY_SIZE`] tracks the user has
//! never heard, regenerated once a week and frozen in between.
//!
//! [`discover`] is pure and deterministic: it reuses the mixes' input and candidate
//! scoring ([`crate::mixes`]) with every seed treated as one cluster, and keeps only
//! unknown tracks (not in the library, never played, not liked, not disliked, not a
//! seed, not in the exclusion set), at most [`DISCOVERY_ARTIST_CAP`] per artist.
//!
//! [`Discoveries`] stores the result in the `discoveries` table under the start of the
//! current week ([`week_start`], Monday 00:00 UTC). Within the week the stored rows are
//! returned as is (no network). A new week generates a new list that excludes the
//! previous week's tracks; weeks older than the previous one are deleted. A generation
//! that yields nothing stores nothing, so the next call tries again.

pub mod commands;

use std::collections::{HashMap, HashSet};

use rusqlite::params;
use serde::Serialize;

use crate::db::{Db, now_secs};
use crate::error::AppResult;
use crate::mixes::{MixInput, MixTrack, Mixes, Seed, candidates};

/// Tracks in the weekly playlist.
pub const DISCOVERY_SIZE: usize = 30;
/// Most tracks by one artist.
pub const DISCOVERY_ARTIST_CAP: usize = 2;

const SECS_PER_DAY: i64 = 86_400;

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
pub struct Discovery {
    /// Unix timestamp (seconds) of the start of the week the tracks belong to.
    pub week_start: i64,
    pub tracks: Vec<MixTrack>,
}

/// Start (Monday 00:00 UTC, unix seconds) of the week containing `now`.
pub fn week_start(now: i64) -> i64 {
    let days = now.div_euclid(SECS_PER_DAY);
    // 1970-01-01 was a Thursday: 3 days after a Monday.
    let since_monday = (days + 3).rem_euclid(7);
    (days - since_monday) * SECS_PER_DAY
}

/// Pure core; see the module doc. Same input, same output.
pub fn discover(input: &MixInput, exclude: &HashSet<String>) -> Vec<MixTrack> {
    let seeds: Vec<&Seed> = input.seeds.iter().collect();
    let mut per_artist: HashMap<String, usize> = HashMap::new();
    let mut tracks = Vec::new();
    for c in candidates(input, &seeds) {
        if tracks.len() >= DISCOVERY_SIZE {
            break;
        }
        if input.track_known(&c.youtube_id) || exclude.contains(&c.youtube_id) {
            continue;
        }
        if let Some(key) = c.artist_key {
            let count = per_artist.entry(key).or_default();
            if *count >= DISCOVERY_ARTIST_CAP {
                continue;
            }
            *count += 1;
        }
        tracks.push(MixTrack {
            youtube_id: c.youtube_id,
            title: c.title,
            artist: c.artist,
            duration: c.duration,
            known: false,
        });
    }
    tracks
}

/// Managed state.
#[derive(Clone)]
pub struct Discoveries {
    db: Db,
    mixes: Mixes,
}

impl Discoveries {
    pub fn new(db: Db, mixes: Mixes) -> Self {
        Self { db, mixes }
    }

    /// The current week's discoveries: the stored ones, else freshly generated.
    pub async fn get(&self) -> AppResult<Discovery> {
        let week = week_start(now_secs());
        let tracks = self.stored(week)?;
        if !tracks.is_empty() {
            return Ok(Discovery {
                week_start: week,
                tracks,
            });
        }
        self.generate(week).await
    }

    /// Drops the current week's discoveries and generates new ones.
    pub async fn refresh(&self) -> AppResult<Discovery> {
        let week = week_start(now_secs());
        self.db
            .conn()
            .execute("DELETE FROM discoveries WHERE week_start = ?1", [week])?;
        self.generate(week).await
    }

    /// Builds the list for `week` (excluding the previous week's tracks), stores it
    /// and prunes old weeks. Nothing is stored when the list is empty.
    async fn generate(&self, week: i64) -> AppResult<Discovery> {
        let exclude = self.previous_ids(week)?;
        let input = self.mixes.load_input().await?;
        let tracks = discover(&input, &exclude);
        if !tracks.is_empty() {
            self.store(week, &tracks)?;
            self.prune(week)?;
        }
        Ok(Discovery {
            week_start: week,
            tracks,
        })
    }

    /// Tracks stored for `week`, in order.
    fn stored(&self, week: i64) -> AppResult<Vec<MixTrack>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT youtube_id, title, artist, duration FROM discoveries
             WHERE week_start = ?1 ORDER BY position",
        )?;
        let tracks = stmt
            .query_map([week], |row| {
                Ok(MixTrack {
                    youtube_id: row.get(0)?,
                    title: row.get(1)?,
                    artist: row.get(2)?,
                    duration: row.get(3)?,
                    known: false,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tracks)
    }

    /// Replaces the tracks of `week` in one transaction.
    fn store(&self, week: i64, tracks: &[MixTrack]) -> AppResult<()> {
        let mut conn = self.db.conn();
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM discoveries WHERE week_start = ?1", [week])?;
        {
            let mut insert = tx.prepare(
                "INSERT INTO discoveries (week_start, position, youtube_id, title, artist, duration)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;
            for (position, t) in tracks.iter().enumerate() {
                insert.execute(params![
                    week,
                    position as i64,
                    t.youtube_id,
                    t.title,
                    t.artist,
                    t.duration
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Ids of the latest stored week strictly before `week` (empty if none).
    fn previous_ids(&self, week: i64) -> AppResult<HashSet<String>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT youtube_id FROM discoveries WHERE week_start =
                 (SELECT MAX(week_start) FROM discoveries WHERE week_start < ?1)",
        )?;
        let ids = stmt
            .query_map([week], |row| row.get(0))?
            .collect::<Result<HashSet<String>, _>>()?;
        Ok(ids)
    }

    /// Deletes the weeks older than the one before `week`.
    fn prune(&self, week: i64) -> AppResult<()> {
        self.db.conn().execute(
            "DELETE FROM discoveries WHERE week_start < COALESCE(
                 (SELECT MAX(week_start) FROM discoveries WHERE week_start < ?1), ?1)",
            [week],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feedback::Feedback;
    use crate::history::History;
    use crate::library::Library;
    use crate::radio::{Radio, RadioTrack};
    use crate::taste::Taste;

    fn rt(id: &str, artist: &str) -> RadioTrack {
        RadioTrack {
            youtube_id: id.into(),
            title: format!("title {id}"),
            artist: Some(artist.into()),
            duration: 180.0,
        }
    }

    fn seed(id: &str, artist: &str, weight: f64) -> Seed {
        Seed {
            youtube_id: id.into(),
            title: format!("title {id}"),
            artist: Some(artist.into()),
            duration: 200.0,
            weight,
        }
    }

    /// `n` tracks `<prefix>0..`, each by its own artist.
    fn radio(prefix: &str, n: usize) -> Vec<RadioTrack> {
        (0..n)
            .map(|i| rt(&format!("{prefix}{i}"), &format!("{prefix}-art{i}")))
            .collect()
    }

    fn input() -> MixInput {
        let mut input = MixInput {
            seeds: vec![seed("sa", "Alpha", 3.0), seed("sb", "Beta", 2.0)],
            ..Default::default()
        };
        input.radios.insert("sa".into(), radio("a", 30));
        input.radios.insert("sb".into(), radio("b", 30));
        input
    }

    fn ids(tracks: &[MixTrack]) -> Vec<&str> {
        tracks.iter().map(|t| t.youtube_id.as_str()).collect()
    }

    #[test]
    fn week_start_is_monday_midnight_utc() {
        // 2024-01-01 is a Monday.
        let monday = 1_704_067_200;
        assert_eq!(week_start(monday), monday);
        assert_eq!(week_start(monday + 3 * 86_400 + 12_345), monday);
        // Sunday 23:59:59 still belongs to the week; the next second does not.
        assert_eq!(week_start(monday + 7 * 86_400 - 1), monday);
        assert_eq!(week_start(monday + 7 * 86_400), monday + 7 * 86_400);
        // Before the epoch's first Monday (1969-12-29).
        assert_eq!(week_start(0), -3 * 86_400);
        assert_eq!(week_start(-1), -3 * 86_400);
        assert_eq!(week_start(-3 * 86_400 - 1), -10 * 86_400);
    }

    #[test]
    fn empty_input_gives_nothing() {
        assert!(discover(&MixInput::default(), &HashSet::new()).is_empty());
    }

    #[test]
    fn only_unknown_tracks() {
        let mut input = input();
        input.library_ids.insert("a0".into());
        input.play_counts.insert("a1".into(), 3);
        input.liked_ids.insert("a2".into());
        input.disliked_ids.insert("a3".into());
        // A seed appearing in the other seed's radio.
        input.radios.get_mut("sb").unwrap().push(rt("sa", "Alpha"));
        let tracks = discover(&input, &HashSet::new());
        assert_eq!(tracks.len(), DISCOVERY_SIZE);
        let got = ids(&tracks);
        for bad in ["a0", "a1", "a2", "a3", "sa", "sb"] {
            assert!(!got.contains(&bad), "{bad}");
        }
        assert!(tracks.iter().all(|t| !t.known));
    }

    #[test]
    fn exclude_is_honoured() {
        let input = input();
        let first = discover(&input, &HashSet::new());
        let exclude: HashSet<String> = ids(&first[..10]).iter().map(|s| (*s).into()).collect();
        let second = discover(&input, &exclude);
        assert!(ids(&second).iter().all(|id| !exclude.contains(*id)));
        assert_eq!(second.len(), DISCOVERY_SIZE);
    }

    #[test]
    fn artist_cap_and_size_cap() {
        let mut input = MixInput {
            seeds: vec![seed("s", "Seed", 1.0)],
            ..Default::default()
        };
        let mut r: Vec<RadioTrack> = (0..10).map(|i| rt(&format!("same{i}"), "Same")).collect();
        r.extend(radio("o", 50));
        input.radios.insert("s".into(), r);
        let tracks = discover(&input, &HashSet::new());
        let same = tracks
            .iter()
            .filter(|t| t.artist.as_deref() == Some("Same"))
            .count();
        assert_eq!(same, DISCOVERY_ARTIST_CAP);
        assert_eq!(tracks.len(), DISCOVERY_SIZE);
    }

    #[test]
    fn all_seeds_count_as_one_cluster() {
        let mut input = input();
        input.radios.get_mut("sa").unwrap().push(rt("shared", "S"));
        input.radios.get_mut("sb").unwrap().push(rt("shared", "S"));
        // Overlap across unrelated seeds ranks first.
        assert_eq!(discover(&input, &HashSet::new())[0].youtube_id, "shared");
    }

    #[test]
    fn deterministic() {
        let input = input();
        let first = discover(&input, &HashSet::new());
        assert!(!first.is_empty());
        for _ in 0..5 {
            assert_eq!(discover(&input.clone(), &HashSet::new()), first);
        }
    }

    fn discoveries() -> Discoveries {
        let db = Db::in_memory();
        let history = History::new(db.clone());
        let feedback = Feedback::new(db.clone(), None);
        let mixes = Mixes::new(
            Taste::new(history.clone(), feedback.clone()),
            Radio::new(db.clone()),
            Library::new(db.clone(), None),
            feedback,
            history,
        );
        Discoveries::new(db, mixes)
    }

    fn tracks(prefix: &str, n: usize) -> Vec<MixTrack> {
        (0..n)
            .map(|i| MixTrack {
                youtube_id: format!("{prefix}{i}"),
                title: format!("t{i}"),
                artist: (i % 2 == 0).then(|| "Art".to_owned()),
                duration: i as f64,
                known: false,
            })
            .collect()
    }

    #[test]
    fn store_and_read_back_in_order() {
        let d = discoveries();
        assert!(d.stored(100).unwrap().is_empty());
        let t = tracks("x", 5);
        d.store(100, &t).unwrap();
        assert_eq!(d.stored(100).unwrap(), t);
        // Storing again replaces the week.
        let t2 = tracks("y", 3);
        d.store(100, &t2).unwrap();
        assert_eq!(d.stored(100).unwrap(), t2);
        assert!(d.stored(200).unwrap().is_empty());
    }

    #[test]
    fn previous_week_rotation() {
        let d = discoveries();
        assert!(d.previous_ids(300).unwrap().is_empty());
        d.store(100, &tracks("a", 2)).unwrap();
        d.store(200, &tracks("b", 2)).unwrap();
        d.store(300, &tracks("c", 2)).unwrap();
        // Latest earlier week only; the current one is not its own predecessor.
        let prev = d.previous_ids(300).unwrap();
        assert_eq!(prev, HashSet::from(["b0".to_owned(), "b1".to_owned()]));
        assert_eq!(d.previous_ids(100).unwrap(), HashSet::new());

        d.prune(300).unwrap();
        assert!(d.stored(100).unwrap().is_empty());
        assert_eq!(d.stored(200).unwrap().len(), 2);
        assert_eq!(d.stored(300).unwrap().len(), 2);

        // No previous week: prune keeps the current one.
        let d = discoveries();
        d.store(300, &tracks("c", 2)).unwrap();
        d.prune(300).unwrap();
        assert_eq!(d.stored(300).unwrap().len(), 2);
    }

    #[tokio::test]
    async fn get_returns_stored_week_without_network() {
        let d = discoveries();
        let week = week_start(now_secs());
        let t = tracks("x", 4);
        d.store(week, &t).unwrap();
        let got = d.get().await.unwrap();
        assert_eq!(
            got,
            Discovery {
                week_start: week,
                tracks: t
            }
        );
    }

    #[tokio::test]
    async fn empty_generation_stores_nothing() {
        let d = discoveries();
        let got = d.get().await.unwrap();
        assert!(got.tracks.is_empty());
        assert!(d.stored(got.week_start).unwrap().is_empty());
        let got = d.refresh().await.unwrap();
        assert!(got.tracks.is_empty());
    }
}
