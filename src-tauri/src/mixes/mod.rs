//! Mixes: playlists of ~[`MIX_SIZE`] tracks generated from what the user listens to,
//! recomputed on demand (nothing is stored).
//!
//! Three categories ([`MixCategory`]): `Known` (only tracks or artists the user
//! already knows), `New` (no track from the library, the history or the likes) and
//! `Mixed` (known and new tracks interleaved).
//!
//! [`Mixes::generate`] gathers the seeds (best tracks of the taste profile, completed
//! with library tracks when the profile is thin, see [`select_seeds`]), fetches their
//! radios ([`crate::radio`]) and hands everything to [`generate`], a pure and
//! deterministic function (ties broken by id):
//! 1. seeds are clustered ([`cluster_seeds`]): same artist, or radios that overlap
//!    enough, so that distinct tastes give distinct mixes;
//! 2. in a cluster, a radio track scores the sum, over the seeds whose radio contains
//!    it, of the seed weight (with a small bonus for appearing early in the radio);
//! 3. each cluster yields up to one mix per category, at most [`MAX_PER_ARTIST`]
//!    tracks per artist, disliked tracks and artists always excluded; mixes shorter
//!    than [`MIN_MIX_TRACKS`] are dropped and at most [`MAX_MIXES`] are kept, at
//!    least one per category when possible.

pub mod commands;

use std::collections::{HashMap, HashSet};

use futures_util::StreamExt;
use futures_util::stream;
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::feedback::{Feedback, normalize_artist};
use crate::history::History;
use crate::library::{Library, Track};
use crate::radio::{Radio, RadioTrack};
use crate::taste::{HISTORY_LIMIT, Taste, TasteTrack};

/// Target length of a mix.
pub const MIX_SIZE: usize = 25;
/// Shorter mixes are dropped.
pub const MIN_MIX_TRACKS: usize = 8;
/// Most mixes returned.
pub const MAX_MIXES: usize = 6;
/// Most tracks by one artist in a mix.
pub const MAX_PER_ARTIST: usize = 3;
/// Most seeds (one radio fetched per seed).
pub const MAX_SEEDS: usize = 8;
/// Under this many taste tracks, the seeds are completed with library tracks.
pub const MIN_SEEDS: usize = 5;
/// Weight of a seed taken from the library (no taste score).
pub const FALLBACK_SEED_WEIGHT: f64 = 0.5;
/// Two seeds whose radios (seed included) have at least this Jaccard index are clustered.
pub const CLUSTER_JACCARD: f64 = 0.2;
/// Extra weight of a radio's first track; decreases linearly to 0 at its end.
pub const POSITION_BONUS: f64 = 0.5;
/// In `Mixed` mixes, a known track's score is divided by `1 + PLAY_DAMPING * plays`.
pub const PLAY_DAMPING: f64 = 0.5;
/// In a `Mixed` mix, once one side runs out the other one is only appended while the
/// short side keeps at least this share of the mix.
pub const MIXED_MIN_SHARE: f64 = 0.3;
/// Radios fetched at the same time.
pub const RADIO_PARALLELISM: usize = 3;

const NO_RADIO: &str = "Impossible de générer des mix pour le moment";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum MixCategory {
    Known,
    New,
    Mixed,
}

impl MixCategory {
    const ALL: [Self; 3] = [Self::Known, Self::New, Self::Mixed];

    fn slug(self) -> &'static str {
        match self {
            Self::Known => "known",
            Self::New => "new",
            Self::Mixed => "mixed",
        }
    }

    fn title(self, name: &str) -> String {
        match self {
            Self::Known => format!("Mix {name}"),
            Self::New => format!("Découvertes autour de {name}"),
            Self::Mixed => format!("Mix mélangé autour de {name}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
pub struct MixTrack {
    pub youtube_id: String,
    pub title: String,
    pub artist: Option<String>,
    /// Seconds, 0 when unknown.
    pub duration: f64,
    /// In the library, already played or liked.
    pub known: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
pub struct Mix {
    /// Stable for the same input: `<category>-<cluster rank>-<main seed id>`.
    pub id: String,
    pub title: String,
    pub category: MixCategory,
    /// Seeds of the cluster the mix comes from, main one first.
    pub seed_ids: Vec<String>,
    pub tracks: Vec<MixTrack>,
}

/// A track the mixes are built around.
#[derive(Clone, Debug, PartialEq)]
pub struct Seed {
    pub youtube_id: String,
    pub title: String,
    pub artist: Option<String>,
    /// Seconds, 0 when unknown.
    pub duration: f64,
    /// Taste score (or [`FALLBACK_SEED_WEIGHT`]), > 0.
    pub weight: f64,
}

/// Everything [`generate`] needs. Artist keys are normalized ([`normalize_artist`]).
#[derive(Clone, Debug, Default)]
pub struct MixInput {
    pub seeds: Vec<Seed>,
    /// Radio of each seed, by seed id; seeds whose radio failed are absent.
    pub radios: HashMap<String, Vec<RadioTrack>>,
    pub library_ids: HashSet<String>,
    pub liked_ids: HashSet<String>,
    /// Number of listens per track.
    pub play_counts: HashMap<String, u32>,
    pub disliked_ids: HashSet<String>,
    pub disliked_artists: HashSet<String>,
    /// Artists of the taste profile, the library, the history and the likes.
    pub known_artists: HashSet<String>,
}

impl MixInput {
    /// The track itself is known: in the library, played or liked.
    pub(crate) fn track_known(&self, id: &str) -> bool {
        self.library_ids.contains(id)
            || self.liked_ids.contains(id)
            || self.play_counts.contains_key(id)
    }

    fn excluded(&self, id: &str, artist_key: Option<&str>) -> bool {
        self.disliked_ids.contains(id)
            || artist_key.is_some_and(|a| self.disliked_artists.contains(a))
    }
}

/// Normalized artist, `None` when absent or blank.
pub(crate) fn artist_key(artist: Option<&str>) -> Option<String> {
    artist.map(normalize_artist).filter(|a| !a.is_empty())
}

/// The seeds: the taste tracks (best first, already free of dislikes), at most
/// [`MAX_SEEDS`]. With fewer than [`MIN_SEEDS`] of them, library tracks that have a
/// youtube id are added (in library order, all of them if needed, up to
/// [`MAX_SEEDS`]) with [`FALLBACK_SEED_WEIGHT`]. Durations come from the library.
pub fn select_seeds(
    taste: &[TasteTrack],
    library: &[Track],
    disliked_ids: &HashSet<String>,
    disliked_artists: &HashSet<String>,
) -> Vec<Seed> {
    let durations: HashMap<&str, f64> = library
        .iter()
        .filter_map(|t| Some((t.youtube_id.as_deref()?, t.duration)))
        .collect();
    let mut seeds: Vec<Seed> = taste
        .iter()
        .filter(|t| t.score > 0.0)
        .take(MAX_SEEDS)
        .map(|t| Seed {
            youtube_id: t.youtube_id.clone(),
            title: t.title.clone(),
            artist: t.artist.clone(),
            duration: durations.get(t.youtube_id.as_str()).copied().unwrap_or(0.0),
            weight: t.score,
        })
        .collect();
    if seeds.len() >= MIN_SEEDS {
        return seeds;
    }
    for t in library {
        if seeds.len() >= MAX_SEEDS {
            break;
        }
        let Some(id) = t.youtube_id.as_deref() else {
            continue;
        };
        let key = artist_key(t.artist.as_deref());
        if seeds.iter().any(|s| s.youtube_id == id)
            || disliked_ids.contains(id)
            || key.is_some_and(|k| disliked_artists.contains(&k))
        {
            continue;
        }
        seeds.push(Seed {
            youtube_id: id.to_owned(),
            title: t.title.clone(),
            artist: t.artist.clone(),
            duration: t.duration,
            weight: FALLBACK_SEED_WEIGHT,
        });
    }
    seeds
}

fn jaccard(a: &HashSet<&str>, b: &HashSet<&str>) -> f64 {
    let union = a.union(b).count();
    if union == 0 {
        return 0.0;
    }
    a.intersection(b).count() as f64 / union as f64
}

/// Groups of seed indexes: two seeds are linked when they share a (normalized)
/// artist or when their radios, seed included, have a Jaccard index of at least
/// [`CLUSTER_JACCARD`]; groups are the connected components. Inside a group seeds
/// are sorted by weight then id; groups by total weight then first seed id.
pub fn cluster_seeds(seeds: &[Seed], radios: &HashMap<String, Vec<RadioTrack>>) -> Vec<Vec<usize>> {
    let keys: Vec<Option<String>> = seeds
        .iter()
        .map(|s| artist_key(s.artist.as_deref()))
        .collect();
    let sets: Vec<Option<HashSet<&str>>> = seeds
        .iter()
        .map(|s| {
            radios.get(&s.youtube_id).map(|r| {
                r.iter()
                    .map(|t| t.youtube_id.as_str())
                    .chain([s.youtube_id.as_str()])
                    .collect()
            })
        })
        .collect();

    // Union-find (a handful of seeds: no rank/compression tricks needed).
    let mut parent: Vec<usize> = (0..seeds.len()).collect();
    fn root(parent: &[usize], mut i: usize) -> usize {
        while parent[i] != i {
            i = parent[i];
        }
        i
    }
    for i in 0..seeds.len() {
        for j in i + 1..seeds.len() {
            let same_artist = keys[i].is_some() && keys[i] == keys[j];
            let overlap = match (&sets[i], &sets[j]) {
                (Some(a), Some(b)) => jaccard(a, b) >= CLUSTER_JACCARD,
                _ => false,
            };
            if same_artist || overlap {
                let (ri, rj) = (root(&parent, i), root(&parent, j));
                parent[ri.max(rj)] = ri.min(rj);
            }
        }
    }

    let by_weight = |a: &usize, b: &usize| {
        seeds[*b]
            .weight
            .total_cmp(&seeds[*a].weight)
            .then_with(|| seeds[*a].youtube_id.cmp(&seeds[*b].youtube_id))
    };
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..seeds.len() {
        groups.entry(root(&parent, i)).or_default().push(i);
    }
    let mut groups: Vec<Vec<usize>> = groups.into_values().collect();
    for g in &mut groups {
        g.sort_by(by_weight);
    }
    let total = |g: &[usize]| g.iter().map(|&i| seeds[i].weight).sum::<f64>();
    groups.sort_by(|a, b| {
        total(b)
            .total_cmp(&total(a))
            .then_with(|| seeds[a[0]].youtube_id.cmp(&seeds[b[0]].youtube_id))
    });
    groups
}

#[derive(Clone, Debug)]
pub(crate) struct Candidate {
    pub(crate) youtube_id: String,
    pub(crate) title: String,
    pub(crate) artist: Option<String>,
    pub(crate) artist_key: Option<String>,
    pub(crate) duration: f64,
    pub(crate) score: f64,
}

/// Radio tracks of a cluster's seeds, scored (see the module doc), best first.
/// Disliked tracks/artists and the cluster's own seeds are left out.
pub(crate) fn candidates(input: &MixInput, seeds: &[&Seed]) -> Vec<Candidate> {
    let mut by_id: HashMap<&str, Candidate> = HashMap::new();
    for seed in seeds {
        let Some(radio) = input.radios.get(&seed.youtube_id) else {
            continue;
        };
        let len = radio.len() as f64;
        for (pos, t) in radio.iter().enumerate() {
            if seeds.iter().any(|s| s.youtube_id == t.youtube_id) {
                continue;
            }
            let key = artist_key(t.artist.as_deref());
            if input.excluded(&t.youtube_id, key.as_deref()) {
                continue;
            }
            let value = seed.weight * (1.0 + POSITION_BONUS * (1.0 - pos as f64 / len));
            by_id
                .entry(&t.youtube_id)
                .or_insert_with(|| Candidate {
                    youtube_id: t.youtube_id.clone(),
                    title: t.title.clone(),
                    artist: t.artist.clone(),
                    artist_key: key,
                    duration: t.duration,
                    score: 0.0,
                })
                .score += value;
        }
    }
    let mut list: Vec<Candidate> = by_id.into_values().collect();
    sort_candidates(&mut list);
    list
}

fn sort_candidates(list: &mut [Candidate]) {
    list.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.youtube_id.cmp(&b.youtube_id))
    });
}

/// Accumulates a mix: no duplicate, artist cap, [`MIX_SIZE`] at most.
struct MixBuilder<'a> {
    input: &'a MixInput,
    tracks: Vec<MixTrack>,
    seen: HashSet<String>,
    per_artist: HashMap<String, usize>,
}

impl<'a> MixBuilder<'a> {
    fn new(input: &'a MixInput) -> Self {
        Self {
            input,
            tracks: Vec::new(),
            seen: HashSet::new(),
            per_artist: HashMap::new(),
        }
    }

    fn full(&self) -> bool {
        self.tracks.len() >= MIX_SIZE
    }

    /// Adds `c` if allowed; `true` when added.
    fn push(&mut self, c: &Candidate) -> bool {
        if self.full()
            || self.seen.contains(&c.youtube_id)
            || self.input.excluded(&c.youtube_id, c.artist_key.as_deref())
        {
            return false;
        }
        if let Some(key) = &c.artist_key {
            let count = self.per_artist.entry(key.clone()).or_default();
            if *count >= MAX_PER_ARTIST {
                return false;
            }
            *count += 1;
        }
        self.seen.insert(c.youtube_id.clone());
        self.tracks.push(MixTrack {
            youtube_id: c.youtube_id.clone(),
            title: c.title.clone(),
            artist: c.artist.clone(),
            duration: c.duration,
            known: self.input.track_known(&c.youtube_id),
        });
        true
    }

    /// Pushes the next acceptable candidate of `iter`; `false` when exhausted.
    fn push_next<'c>(&mut self, iter: &mut impl Iterator<Item = &'c Candidate>) -> bool {
        iter.any(|c| self.push(c))
    }
}

/// The seeds first (by weight), then the candidates the user knows (track or artist).
fn known_mix(input: &MixInput, seeds: &[&Seed], pool: &[Candidate]) -> Vec<MixTrack> {
    let mut b = MixBuilder::new(input);
    for s in seeds {
        b.push(&Candidate {
            youtube_id: s.youtube_id.clone(),
            title: s.title.clone(),
            artist: s.artist.clone(),
            artist_key: artist_key(s.artist.as_deref()),
            duration: s.duration,
            score: s.weight,
        });
    }
    for c in pool {
        let artist_known = c
            .artist_key
            .as_deref()
            .is_some_and(|k| input.known_artists.contains(k));
        if input.track_known(&c.youtube_id) || artist_known {
            b.push(c);
        }
    }
    b.tracks
}

/// Only tracks the user has never had (artists may be known).
fn new_mix(input: &MixInput, pool: &[Candidate]) -> Vec<MixTrack> {
    let mut b = MixBuilder::new(input);
    for c in pool.iter().filter(|c| !input.track_known(&c.youtube_id)) {
        b.push(c);
    }
    b.tracks
}

/// Known and new tracks alternated (known first). Known ones are damped by their
/// play count so the same favourites do not come back every time. When one side runs
/// out, the other is appended only while the short side keeps [`MIXED_MIN_SHARE`].
fn mixed_mix(input: &MixInput, pool: &[Candidate]) -> Vec<MixTrack> {
    let (mut known, new): (Vec<Candidate>, Vec<Candidate>) = pool
        .iter()
        .cloned()
        .partition(|c| input.track_known(&c.youtube_id));
    for c in &mut known {
        let plays = input.play_counts.get(&c.youtube_id).copied().unwrap_or(0);
        c.score /= 1.0 + PLAY_DAMPING * f64::from(plays);
    }
    sort_candidates(&mut known);

    let mut b = MixBuilder::new(input);
    let mut sides = [known.iter(), new.iter()];
    let mut counts = [0usize; 2];
    let mut exhausted = [false; 2];
    let mut turn = 0;
    while !b.full() && !(exhausted[0] && exhausted[1]) {
        let other = 1 - turn;
        if exhausted[turn] {
            turn = other;
            continue;
        }
        if exhausted[other] {
            let share = counts[other] as f64 / (b.tracks.len() + 1) as f64;
            if share < MIXED_MIN_SHARE {
                break;
            }
        }
        if b.push_next(&mut sides[turn]) {
            counts[turn] += 1;
        } else {
            exhausted[turn] = true;
        }
        turn = other;
    }
    b.tracks
}

/// Pure core; see the module doc. Same input, same output.
pub fn generate(input: &MixInput) -> Vec<Mix> {
    let clusters = cluster_seeds(&input.seeds, &input.radios);
    // Valid mixes of each category, in cluster order.
    let mut by_category: HashMap<MixCategory, Vec<Mix>> = HashMap::new();
    for (rank, cluster) in clusters.iter().enumerate() {
        let seeds: Vec<&Seed> = cluster.iter().map(|&i| &input.seeds[i]).collect();
        let main = seeds[0];
        let name = main
            .artist
            .as_deref()
            .map(str::trim)
            .filter(|a| !a.is_empty())
            .unwrap_or(main.title.trim());
        let pool = candidates(input, &seeds);
        for category in MixCategory::ALL {
            let tracks = match category {
                MixCategory::Known => known_mix(input, &seeds, &pool),
                MixCategory::New => new_mix(input, &pool),
                MixCategory::Mixed => mixed_mix(input, &pool),
            };
            if tracks.len() < MIN_MIX_TRACKS {
                continue;
            }
            by_category.entry(category).or_default().push(Mix {
                id: format!("{}-{rank}-{}", category.slug(), main.youtube_id),
                title: category.title(name),
                category,
                seed_ids: seeds.iter().map(|s| s.youtube_id.clone()).collect(),
                tracks,
            });
        }
    }
    // Rounds: the best mix of each category, then the second of each, ...
    let mut queues: Vec<std::vec::IntoIter<Mix>> = MixCategory::ALL
        .iter()
        .map(|c| by_category.remove(c).unwrap_or_default().into_iter())
        .collect();
    let mut mixes = Vec::new();
    loop {
        let before = mixes.len();
        for q in &mut queues {
            if mixes.len() >= MAX_MIXES {
                return mixes;
            }
            mixes.extend(q.next());
        }
        if mixes.len() == before {
            return mixes;
        }
    }
}

/// Managed state.
#[derive(Clone)]
pub struct Mixes {
    taste: Taste,
    radio: Radio,
    library: Library,
    feedback: Feedback,
    history: History,
}

impl Mixes {
    pub fn new(
        taste: Taste,
        radio: Radio,
        library: Library,
        feedback: Feedback,
        history: History,
    ) -> Self {
        Self {
            taste,
            radio,
            library,
            feedback,
            history,
        }
    }

    /// Everything but the radios.
    fn input(&self) -> AppResult<MixInput> {
        let profile = self.taste.profile()?;
        let library = self.library.tracks()?;
        let plays = self.history.recent_music(HISTORY_LIMIT)?;
        let disliked_ids = self.feedback.disliked_youtube_ids()?;
        let disliked_artists = self.feedback.disliked_artists()?;
        let liked_ids = self.feedback.liked_youtube_ids()?;

        let mut play_counts: HashMap<String, u32> = HashMap::new();
        for id in plays.iter().filter_map(|p| p.youtube_id.clone()) {
            *play_counts.entry(id).or_default() += 1;
        }
        let mut known_artists = self.feedback.liked_artists()?;
        known_artists.extend(
            profile
                .artists
                .iter()
                .map(|a| a.artist.as_str())
                .chain(library.iter().filter_map(|t| t.artist.as_deref()))
                .chain(plays.iter().filter_map(|p| p.artist.as_deref()))
                .filter_map(|a| artist_key(Some(a))),
        );
        for disliked in &disliked_artists {
            known_artists.remove(disliked);
        }

        Ok(MixInput {
            seeds: select_seeds(&profile.tracks, &library, &disliked_ids, &disliked_artists),
            radios: HashMap::new(),
            library_ids: library
                .iter()
                .filter_map(|t| t.youtube_id.clone())
                .collect(),
            liked_ids,
            play_counts,
            disliked_ids,
            disliked_artists,
            known_artists,
        })
    }

    /// Builds the seeds and fetches their radios ([`RADIO_PARALLELISM`] at a time; a
    /// failing seed is skipped). Without any seed, returns the input as is (no radio).
    /// Fails only if no radio at all could be read.
    pub(crate) async fn load_input(&self) -> AppResult<MixInput> {
        let mut input = self.input()?;
        if input.seeds.is_empty() {
            return Ok(input);
        }
        // Owned ids and futures (no borrowing closures): keeps the future `Send`.
        let ids: Vec<String> = input.seeds.iter().map(|s| s.youtube_id.clone()).collect();
        let fetches = ids.into_iter().map(|id| {
            let radio = self.radio.clone();
            async move {
                let tracks = radio.get(&id).await;
                (id, tracks)
            }
        });
        let results: Vec<(String, AppResult<Vec<RadioTrack>>)> = stream::iter(fetches)
            .buffered(RADIO_PARALLELISM)
            .collect()
            .await;
        for (id, radio) in results {
            match radio {
                Ok(tracks) => {
                    input.radios.insert(id, tracks);
                }
                Err(e) => tracing::warn!("mix: radio de {id} indisponible : {e}"),
            }
        }
        if input.radios.is_empty() {
            return Err(AppError::Tool(NO_RADIO.into()));
        }
        Ok(input)
    }

    /// Loads the input ([`Self::load_input`]) then builds the mixes.
    pub async fn generate(&self) -> AppResult<Vec<Mix>> {
        Ok(generate(&self.load_input().await?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// `n` tracks `<prefix>0..`, each by its own artist `<prefix>-art<i>`.
    fn radio(prefix: &str, n: usize) -> Vec<RadioTrack> {
        (0..n)
            .map(|i| rt(&format!("{prefix}{i}"), &format!("{prefix}-art{i}")))
            .collect()
    }

    fn ids(mix: &Mix) -> Vec<&str> {
        mix.tracks.iter().map(|t| t.youtube_id.as_str()).collect()
    }

    fn find(mixes: &[Mix], category: MixCategory) -> &Mix {
        mixes.iter().find(|m| m.category == category).unwrap()
    }

    /// Two unrelated seeds, each with a 30-track radio of distinct artists.
    fn two_tastes() -> MixInput {
        let mut input = MixInput {
            seeds: vec![seed("sa", "Alpha", 3.0), seed("sb", "Beta", 2.0)],
            ..Default::default()
        };
        input.radios.insert("sa".into(), radio("a", 30));
        input.radios.insert("sb".into(), radio("b", 30));
        input
    }

    #[test]
    fn empty_input_gives_nothing() {
        assert!(generate(&MixInput::default()).is_empty());
    }

    #[test]
    fn clustering_splits_two_tastes_and_joins_overlaps() {
        let mut input = two_tastes();
        // Same artist as `sa`, no radio: joins its cluster.
        input.seeds.push(seed("sc", " alpha ", 1.0));
        // Radio overlapping `sb`'s: joins it.
        input.seeds.push(seed("sd", "Delta", 0.5));
        input.radios.insert("sd".into(), radio("b", 20));
        let groups = cluster_seeds(&input.seeds, &input.radios);
        assert_eq!(groups, vec![vec![0, 2], vec![1, 3]]);

        let mixes = generate(&input);
        let a = mixes.iter().find(|m| m.seed_ids[0] == "sa").unwrap();
        assert!(ids(a).iter().all(|id| !id.starts_with('b')));
        assert_eq!(a.seed_ids, ["sa", "sc"]);
    }

    #[test]
    fn multi_seed_overlap_ranks_first() {
        let mut input = MixInput {
            // Same artist: one cluster.
            seeds: vec![seed("s1", "One", 1.0), seed("s2", "one", 1.0)],
            ..Default::default()
        };
        let mut r1 = radio("x", 12);
        let mut r2 = radio("y", 12);
        // `shared` is near the end of both radios but proposed twice.
        r1.push(rt("shared", "Shared"));
        r2.push(rt("shared", "Shared"));
        input.radios.insert("s1".into(), r1);
        input.radios.insert("s2".into(), r2);
        let mixes = generate(&input);
        let new = find(&mixes, MixCategory::New);
        assert_eq!(new.tracks[0].youtube_id, "shared");
    }

    #[test]
    fn new_has_no_known_track_and_known_only_known() {
        let mut input = two_tastes();
        // Known tracks of the `a` radio, by library, history and like.
        for i in 0..4 {
            input.library_ids.insert(format!("a{i}"));
        }
        input.play_counts.insert("a10".into(), 2);
        input.liked_ids.insert("a11".into());
        // Unheard tracks by a known artist.
        for i in 20..25 {
            input.known_artists.insert(format!("a-art{i}"));
        }
        let mixes = generate(&input);
        let new = mixes
            .iter()
            .find(|m| m.category == MixCategory::New && m.seed_ids[0] == "sa")
            .unwrap();
        assert!(new.tracks.iter().all(|t| !t.known));
        assert!(!ids(new).contains(&"sa"));

        let known = find(&mixes, MixCategory::Known);
        assert_eq!(known.seed_ids[0], "sa");
        assert_eq!(known.tracks[0].youtube_id, "sa");
        assert_eq!(known.title, "Mix Alpha");
        for t in &known.tracks {
            let artist_known = input
                .known_artists
                .contains(&normalize_artist(t.artist.as_deref().unwrap()));
            assert!(t.known || artist_known || t.youtube_id == "sa", "{t:?}");
        }
        assert_eq!(known.tracks.len(), 1 + 6 + 5);
        // Cluster `b` knows nothing: no Known mix for it.
        assert_eq!(
            mixes
                .iter()
                .filter(|m| m.category == MixCategory::Known)
                .count(),
            1
        );
    }

    #[test]
    fn mixed_interleaves_and_excludes_seed() {
        let mut input = two_tastes();
        for i in 0..10 {
            input.library_ids.insert(format!("a{}", i * 2));
        }
        let mixes = generate(&input);
        let mixed = find(&mixes, MixCategory::Mixed);
        assert_eq!(mixed.title, "Mix mélangé autour de Alpha");
        assert!(!ids(mixed).contains(&"sa"));
        let flags: Vec<bool> = mixed.tracks.iter().map(|t| t.known).collect();
        // 10 known, 20 new: alternate, then new ones while known keep >= 30%.
        assert_eq!(&flags[..20], [true, false].repeat(10));
        let known = flags.iter().filter(|k| **k).count();
        assert_eq!(known, 10);
        assert!(known as f64 / flags.len() as f64 >= MIXED_MIN_SHARE);
        assert!(flags.len() > 20 && flags.len() <= MIX_SIZE);
    }

    #[test]
    fn mixed_damps_heavily_played_tracks() {
        let mut input = two_tastes();
        for i in 0..10 {
            input.library_ids.insert(format!("a{i}"));
        }
        // a0 is first in the radio but played a lot.
        input.play_counts.insert("a0".into(), 20);
        let mixes = generate(&input);
        let mixed = find(&mixes, MixCategory::Mixed);
        assert_eq!(mixed.tracks[0].youtube_id, "a1");
        assert!(ids(mixed).contains(&"a0"));
    }

    #[test]
    fn artist_cap_applies() {
        let mut input = MixInput {
            seeds: vec![seed("s", "Seed", 1.0)],
            ..Default::default()
        };
        let mut r: Vec<RadioTrack> = (0..10).map(|i| rt(&format!("same{i}"), "Same")).collect();
        r.extend(radio("o", 30));
        input.radios.insert("s".into(), r);
        let new = find(&generate(&input), MixCategory::New).clone();
        let same = new
            .tracks
            .iter()
            .filter(|t| t.artist.as_deref() == Some("Same"))
            .count();
        assert_eq!(same, MAX_PER_ARTIST);
        assert_eq!(new.tracks.len(), MIX_SIZE);
    }

    #[test]
    fn dislikes_are_excluded() {
        let mut input = two_tastes();
        input.disliked_ids.insert("a0".into());
        input.disliked_artists.insert("a-art1".into());
        for i in 0..5 {
            input.library_ids.insert(format!("a{i}"));
        }
        for m in generate(&input) {
            assert!(!ids(&m).contains(&"a0") && !ids(&m).contains(&"a1"));
        }
    }

    #[test]
    fn short_mixes_are_dropped() {
        let mut input = MixInput {
            seeds: vec![seed("s", "Seed", 1.0)],
            ..Default::default()
        };
        input
            .radios
            .insert("s".into(), radio("r", MIN_MIX_TRACKS - 1));
        assert!(generate(&input).is_empty());
        input.radios.insert("s".into(), radio("r", MIN_MIX_TRACKS));
        let mixes = generate(&input);
        assert_eq!(mixes.len(), 1);
        assert_eq!(mixes[0].category, MixCategory::New);
        assert_eq!(mixes[0].id, "new-0-s");
        assert_eq!(mixes[0].title, "Découvertes autour de Seed");
    }

    #[test]
    fn at_most_max_mixes_with_every_category_first() {
        let mut input = MixInput::default();
        for (n, name) in ["p", "q", "r"].iter().enumerate() {
            let s = format!("s{name}");
            input.seeds.push(seed(&s, name, 3.0 - n as f64));
            input.radios.insert(s, radio(name, 30));
            for i in 0..12 {
                input.library_ids.insert(format!("{name}{i}"));
            }
        }
        let mixes = generate(&input);
        assert_eq!(mixes.len(), MAX_MIXES);
        let cats: Vec<MixCategory> = mixes.iter().map(|m| m.category).collect();
        assert_eq!(&cats[..3], MixCategory::ALL);
        assert_eq!(mixes[0].seed_ids[0], "sp");
        assert_eq!(mixes[3].seed_ids[0], "sq");
    }

    #[test]
    fn deterministic() {
        let mut input = two_tastes();
        for i in 0..8 {
            input.library_ids.insert(format!("b{i}"));
        }
        let first = generate(&input);
        assert!(!first.is_empty());
        for _ in 0..5 {
            assert_eq!(generate(&input.clone()), first);
        }
    }

    fn taste_track(id: &str, score: f64) -> TasteTrack {
        TasteTrack {
            youtube_id: id.into(),
            title: id.into(),
            artist: Some("A".into()),
            score,
        }
    }

    fn lib_track(id: i64, youtube_id: Option<&str>, artist: &str) -> Track {
        Track {
            id,
            path: None,
            title: format!("t{id}"),
            artist: Some(artist.into()),
            album: None,
            duration: 100.0,
            added_at: 0,
            youtube_id: youtube_id.map(Into::into),
        }
    }

    #[test]
    fn seeds_fall_back_to_library() {
        let none = HashSet::new();
        let library = vec![
            lib_track(1, Some("t1"), "A"),
            lib_track(2, None, "B"),
            lib_track(3, Some("l3"), "Bad"),
            lib_track(4, Some("l4"), "C"),
            lib_track(5, Some("l5"), "D"),
            lib_track(6, Some("l6"), "E"),
        ];
        let taste = vec![taste_track("t1", 2.0), taste_track("t2", 1.0)];
        let bad: HashSet<String> = ["bad".to_owned()].into();
        let seeds = select_seeds(&taste, &library, &none, &bad);
        let ids: Vec<&str> = seeds.iter().map(|s| s.youtube_id.as_str()).collect();
        // Taste first, then every usable library track (no id, duplicate, disliked skipped).
        assert_eq!(ids, ["t1", "t2", "l4", "l5", "l6"]);
        assert_eq!(seeds[0].duration, 100.0);
        assert_eq!(seeds[1].duration, 0.0);
        assert_eq!(seeds[2].weight, FALLBACK_SEED_WEIGHT);

        // Enough taste tracks: no fallback; capped at MAX_SEEDS.
        let taste: Vec<TasteTrack> = (0..10)
            .map(|i| taste_track(&format!("t{i}"), 10.0 - i as f64))
            .collect();
        let seeds = select_seeds(&taste, &library, &none, &none);
        assert_eq!(seeds.len(), MAX_SEEDS);
        assert!(seeds.iter().all(|s| s.youtube_id.starts_with('t')));
        let seeds = select_seeds(&taste[..MIN_SEEDS], &library, &none, &none);
        assert_eq!(seeds.len(), MIN_SEEDS);

        // Nothing at all.
        assert!(select_seeds(&[], &[], &none, &none).is_empty());
    }

    #[tokio::test]
    async fn generate_without_seeds_is_empty() {
        let db = crate::db::Db::in_memory();
        let history = History::new(db.clone());
        let feedback = Feedback::new(db.clone(), None);
        let mixes = Mixes::new(
            Taste::new(history.clone(), feedback.clone()),
            Radio::new(db.clone()),
            Library::new(db, None),
            feedback,
            history,
        );
        assert!(mixes.generate().await.unwrap().is_empty());
    }
}
