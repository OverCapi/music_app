//! Merging a peer's library ([`Manifest`]) into the local one.
//!
//! Tracks with a YouTube id are added without file ([`Library::add_remote`] reuses
//! the same song when it is already there) and downloaded when played. Tracks
//! without a YouTube id are only playable with their file, so those the library
//! does not already have are copied from the peer first (the caller downloads
//! [`files_needed`] and hands the paths to [`apply`]).
//!
//! Playlists are merged by name (case-insensitive). In `mirror` mode the library
//! becomes a copy of the peer's: playlists get its content, and tracks and playlists
//! it lacks are removed.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use group_protocol::is_valid_youtube_id;
use serde::{Deserialize, Serialize};

use super::LanReport;
use crate::error::{AppError, AppResult};
use crate::library::{Library, RemoteTrack, TrackId};
use crate::playlists::{PlaylistId, Playlists};

const MAX_TITLE_LEN: usize = 300;
const MAX_NAME_LEN: usize = 100;

/// A library track as the peer describes it. `key` is the peer's track id.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ManifestTrack {
    pub key: i64,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: f64,
    pub youtube_id: Option<String>,
    /// The peer can send the audio file (`GET /file/<key>`).
    pub has_file: bool,
    /// Extension of that file (to name the copy).
    pub ext: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ManifestPlaylist {
    pub name: String,
    pub keys: Vec<i64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Manifest {
    pub tracks: Vec<ManifestTrack>,
    pub playlists: Vec<ManifestPlaylist>,
}

impl Manifest {
    /// The local library as a manifest: every track that can be played elsewhere
    /// (YouTube id or existing file).
    pub fn of(library: &Library, playlists: &Playlists) -> AppResult<Self> {
        let mut keys = HashSet::new();
        let tracks: Vec<_> = library
            .tracks()?
            .into_iter()
            .filter_map(|t| {
                let file = t.path.as_deref().filter(|p| Path::new(p).is_file());
                if t.youtube_id.is_none() && file.is_none() {
                    return None;
                }
                keys.insert(t.id);
                Some(ManifestTrack {
                    key: t.id,
                    title: t.title,
                    artist: t.artist,
                    album: t.album,
                    duration: t.duration,
                    youtube_id: t.youtube_id,
                    has_file: file.is_some(),
                    ext: file.and_then(file_ext),
                })
            })
            .collect();
        let playlists = playlists
            .all()?
            .into_iter()
            .map(|p| ManifestPlaylist {
                name: p.name,
                keys: p
                    .track_ids
                    .into_iter()
                    .filter(|id| keys.contains(id))
                    .collect(),
            })
            .collect();
        Ok(Self { tracks, playlists })
    }
}

/// Lowercase alphanumeric extension of at most 5 chars (`mp3`, `opus`…).
pub fn file_ext(path: &str) -> Option<String> {
    let ext = Path::new(path).extension()?.to_str()?.to_ascii_lowercase();
    (!ext.is_empty() && ext.len() <= 5 && ext.bytes().all(|b| b.is_ascii_alphanumeric()))
        .then_some(ext)
}

/// Tracks to copy as files: no YouTube id, a file on the peer, and nothing similar
/// in the library yet.
pub fn files_needed<'a>(
    library: &Library,
    manifest: &'a Manifest,
) -> AppResult<Vec<&'a ManifestTrack>> {
    let mut needed = Vec::new();
    for t in manifest
        .tracks
        .iter()
        .filter(|t| t.youtube_id.is_none() && t.has_file)
    {
        if library
            .find_similar(t.artist.as_deref(), &t.title, clean_duration(t.duration))?
            .is_empty()
        {
            needed.push(t);
        }
    }
    Ok(needed)
}

/// Merges `manifest` into the library. `files` maps a [`ManifestTrack::key`] to the
/// local copy of its file; `trash` disposes of the file of a track removed by a
/// mirror. Tracks that cannot be played here (no YouTube id, no file) are skipped.
pub fn apply(
    library: &Library,
    playlists: &Playlists,
    manifest: &Manifest,
    files: &HashMap<i64, String>,
    mirror: bool,
    trash: &dyn Fn(&str),
) -> AppResult<LanReport> {
    if mirror && manifest.tracks.is_empty() {
        return Err(AppError::invalid(
            "La bibliothèque de l'autre appareil est vide : rien à copier",
        ));
    }
    let mut report = LanReport::default();
    let before = library.tracks()?.len();
    let mut local: HashMap<i64, TrackId> = HashMap::new();

    // Tracks with a YouTube id: added without file.
    let (mut keys, mut remote) = (Vec::new(), Vec::new());
    for t in &manifest.tracks {
        let Some(youtube_id) = t.youtube_id.as_ref().filter(|y| is_valid_youtube_id(y)) else {
            continue;
        };
        let Some(title) = clean_title(&t.title) else {
            continue;
        };
        keys.push(t.key);
        remote.push(RemoteTrack {
            title,
            artist: clean_opt(&t.artist),
            album: clean_opt(&t.album),
            duration: clean_duration(t.duration),
            youtube_id: Some(youtube_id.clone()),
        });
    }
    local.extend(keys.into_iter().zip(library.add_remote(&remote)?));

    // Tracks without YouTube id: the similar one already here, else the copied file.
    for t in manifest
        .tracks
        .iter()
        .filter(|t| t.youtube_id.is_none() && t.has_file)
    {
        let Some(title) = clean_title(&t.title) else {
            continue;
        };
        let artist = clean_opt(&t.artist);
        let duration = clean_duration(t.duration);
        if let Some(existing) = library
            .find_similar(artist.as_deref(), &title, duration)?
            .first()
        {
            local.insert(t.key, existing.id);
        } else if let Some(path) = files.get(&t.key) {
            match library.add_paths(std::slice::from_ref(path)) {
                Ok(added) => {
                    if let Some(track) = added.first() {
                        local.insert(t.key, track.id);
                        report.files += 1;
                    }
                }
                Err(e) => report.errors.push(format!("{title} : {e}")),
            }
        }
    }
    report.new_tracks = library.tracks()?.len().saturating_sub(before) as u32;
    report.skipped = manifest.tracks.len().saturating_sub(local.len()) as u32;

    // Playlists, merged by name.
    let mut by_name: HashMap<String, PlaylistId> = playlists
        .all()?
        .into_iter()
        .map(|p| (p.name.to_lowercase(), p.id))
        .collect();
    let mut source_names = HashSet::new();
    for p in &manifest.playlists {
        let name: String = p.name.trim().chars().take(MAX_NAME_LEN).collect();
        if name.is_empty() {
            continue;
        }
        let lower = name.to_lowercase();
        let ids: Vec<TrackId> = p
            .keys
            .iter()
            .filter_map(|k| local.get(k).copied())
            .collect();
        let id = match by_name.get(&lower) {
            Some(&id) => id,
            None => {
                let id = playlists.create(&name)?.id;
                by_name.insert(lower.clone(), id);
                id
            }
        };
        // A mirror replaces the content, except when the name came twice from the peer.
        if mirror && !source_names.contains(&lower) {
            playlists.set_tracks(id, &ids)?;
        } else {
            playlists.add_tracks(id, &ids)?;
        }
        source_names.insert(lower);
    }
    report.playlists = source_names.len() as u32;

    if mirror {
        // ponytail: one LibraryChanged per removal; batch them if mirrors of
        // thousands of tracks turn out slow.
        let keep: HashSet<TrackId> = local.values().copied().collect();
        for t in library
            .tracks()?
            .into_iter()
            .filter(|t| !keep.contains(&t.id))
        {
            library.remove(t.id)?;
            if let Some(path) = &t.path {
                trash(path);
            }
            report.removed_tracks += 1;
        }
        for p in playlists.all()? {
            if !source_names.contains(&p.name.to_lowercase()) {
                playlists.delete(p.id)?;
                report.removed_playlists += 1;
            }
        }
    }
    Ok(report)
}

/// Text from a peer: trimmed, non-empty, bounded.
fn clean_title(text: &str) -> Option<String> {
    let text: String = text.trim().chars().take(MAX_TITLE_LEN).collect();
    (!text.is_empty()).then_some(text)
}

fn clean_opt(text: &Option<String>) -> Option<String> {
    text.as_deref().and_then(clean_title)
}

fn clean_duration(duration: f64) -> f64 {
    if duration.is_finite() && duration > 0.0 {
        duration
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    fn setup() -> (Library, Playlists) {
        let db = Db::in_memory();
        (Library::new(db.clone(), None), Playlists::new(db, None))
    }

    fn yt(key: i64, id: &str, title: &str) -> ManifestTrack {
        ManifestTrack {
            key,
            title: title.into(),
            artist: Some("X".into()),
            album: None,
            duration: 100.0,
            youtube_id: Some(id.into()),
            has_file: false,
            ext: None,
        }
    }

    fn list(name: &str, keys: &[i64]) -> ManifestPlaylist {
        ManifestPlaylist {
            name: name.into(),
            keys: keys.to_vec(),
        }
    }

    fn manifest() -> Manifest {
        Manifest {
            tracks: vec![
                yt(1, "aaaaaaaaaaa", "A"),
                yt(2, "bbbbbbbbbbb", "B"),
                yt(3, "bad", "Invalid id"),
            ],
            playlists: vec![list("Rock", &[1, 2, 3])],
        }
    }

    fn no_trash(_: &str) {}

    #[test]
    fn merges_tracks_and_playlists_by_name() {
        let (library, playlists) = setup();
        let existing = playlists.create("rock").unwrap().id;
        let own = library
            .add_remote(&[RemoteTrack {
                title: "A".into(),
                artist: Some("X".into()),
                album: None,
                duration: 100.0,
                youtube_id: None,
            }])
            .unwrap()[0];

        let report = apply(
            &library,
            &playlists,
            &manifest(),
            &HashMap::new(),
            false,
            &no_trash,
        )
        .unwrap();
        // "A" matched the local song (and got its YouTube id); only "B" is new.
        assert_eq!(report.new_tracks, 1);
        assert_eq!(report.skipped, 1);
        assert_eq!(
            library.get(own).unwrap().youtube_id.as_deref(),
            Some("aaaaaaaaaaa")
        );
        // The playlist with the same name (other case) was reused.
        let all = playlists.all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, existing);
        assert_eq!(all[0].track_ids.len(), 2);

        // Syncing again changes nothing.
        let again = apply(
            &library,
            &playlists,
            &manifest(),
            &HashMap::new(),
            false,
            &no_trash,
        )
        .unwrap();
        assert_eq!(again.new_tracks, 0);
        assert_eq!(library.tracks().unwrap().len(), 2);
        assert_eq!(playlists.all().unwrap()[0].track_ids.len(), 2);
    }

    #[test]
    fn without_mirror_nothing_is_removed() {
        let (library, playlists) = setup();
        library
            .add_remote(&[RemoteTrack {
                title: "Mine".into(),
                artist: None,
                album: None,
                duration: 10.0,
                youtube_id: Some("ccccccccccc".into()),
            }])
            .unwrap();
        playlists.create("Mienne").unwrap();
        let report = apply(
            &library,
            &playlists,
            &manifest(),
            &HashMap::new(),
            false,
            &no_trash,
        )
        .unwrap();
        assert_eq!((report.removed_tracks, report.removed_playlists), (0, 0));
        assert_eq!(library.tracks().unwrap().len(), 3);
        assert_eq!(playlists.all().unwrap().len(), 2);
    }

    #[test]
    fn mirror_makes_a_copy() {
        let (library, playlists) = setup();
        let mine = library
            .add_remote(&[RemoteTrack {
                title: "Mine".into(),
                artist: None,
                album: None,
                duration: 10.0,
                youtube_id: Some("ccccccccccc".into()),
            }])
            .unwrap()[0];
        let rock = playlists.create("Rock").unwrap().id;
        playlists.add_tracks(rock, &[mine]).unwrap();
        playlists.create("Mienne").unwrap();

        let trashed = std::cell::RefCell::new(Vec::new());
        let report = apply(
            &library,
            &playlists,
            &manifest(),
            &HashMap::new(),
            true,
            &|p| {
                trashed.borrow_mut().push(p.to_owned());
            },
        )
        .unwrap();
        assert_eq!((report.removed_tracks, report.removed_playlists), (1, 1));
        assert!(library.get(mine).is_err());
        let titles: Vec<_> = library
            .tracks()
            .unwrap()
            .into_iter()
            .map(|t| t.title)
            .collect();
        assert_eq!(titles.len(), 2);
        let all = playlists.all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].track_ids.len(), 2);
        // No file for the removed track: nothing to trash.
        assert!(trashed.borrow().is_empty());
    }

    #[test]
    fn mirror_of_an_empty_library_is_refused() {
        let (library, playlists) = setup();
        let empty = Manifest::default();
        assert!(
            apply(
                &library,
                &playlists,
                &empty,
                &HashMap::new(),
                true,
                &no_trash
            )
            .is_err()
        );
        assert!(
            apply(
                &library,
                &playlists,
                &empty,
                &HashMap::new(),
                false,
                &no_trash
            )
            .is_ok()
        );
    }

    #[test]
    fn manifest_lists_playable_tracks_only() {
        let (library, playlists) = setup();
        let ids = library
            .add_remote(&[
                RemoteTrack {
                    title: "Online".into(),
                    artist: None,
                    album: None,
                    duration: 10.0,
                    youtube_id: Some("aaaaaaaaaaa".into()),
                },
                RemoteTrack {
                    title: "Nothing".into(),
                    artist: None,
                    album: None,
                    duration: 10.0,
                    youtube_id: None,
                },
            ])
            .unwrap();
        let list = playlists.create("L").unwrap().id;
        playlists.add_tracks(list, &ids).unwrap();
        let m = Manifest::of(&library, &playlists).unwrap();
        assert_eq!(m.tracks.len(), 1);
        assert_eq!(m.playlists[0].keys, [ids[0]]);
    }

    #[test]
    fn extensions() {
        assert_eq!(file_ext("/a/b.MP3").as_deref(), Some("mp3"));
        assert_eq!(file_ext("/a/b"), None);
        assert_eq!(file_ext("/a/b.toolongext"), None);
    }
}
