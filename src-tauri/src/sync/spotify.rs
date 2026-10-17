//! Reads the Spotify account data export ("Download your data" → "Account data"):
//! `YourLibrary.json` (liked songs, followed artists) and `Playlist1.json`,
//! `Playlist2.json`… (playlists the user created). The export has no YouTube id
//! and no duration: tracks are matched later.
//!
//! Pure parsing, no I/O except [`read_export`].

use std::io::Read;
use std::path::Path;

use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::library::RemoteTrack;

/// What an export contains, merged over all the files given.
#[derive(Debug, Default, PartialEq)]
pub struct SpotifyExport {
    pub liked: Vec<RemoteTrack>,
    pub playlists: Vec<SpotifyPlaylist>,
    pub artists: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct SpotifyPlaylist {
    pub name: String,
    pub tracks: Vec<RemoteTrack>,
}

#[derive(Deserialize)]
struct Library {
    #[serde(default)]
    tracks: Vec<LibraryTrack>,
    #[serde(default)]
    artists: Vec<LibraryArtist>,
}

#[derive(Deserialize)]
struct LibraryTrack {
    artist: Option<String>,
    album: Option<String>,
    track: Option<String>,
}

#[derive(Deserialize)]
struct LibraryArtist {
    name: Option<String>,
}

#[derive(Deserialize)]
struct Playlists {
    #[serde(default)]
    playlists: Vec<Playlist>,
}

#[derive(Deserialize)]
struct Playlist {
    name: Option<String>,
    #[serde(default)]
    items: Vec<PlaylistItem>,
}

/// Episodes, local files and audiobooks have `track: null`.
#[derive(Deserialize)]
struct PlaylistItem {
    track: Option<PlaylistTrack>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlaylistTrack {
    track_name: Option<String>,
    artist_name: Option<String>,
    album_name: Option<String>,
}

/// Reads the given files: the export `.zip` itself, or JSON files extracted from it.
/// Files that are not part of the export are ignored; it is an error if none is.
pub fn read_export(paths: &[String]) -> AppResult<SpotifyExport> {
    let mut export = SpotifyExport::default();
    let mut found = false;
    for path in paths {
        let path = Path::new(path);
        if has_extension(path, "zip") {
            let file = std::fs::File::open(path)?;
            let mut zip = zip::ZipArchive::new(file).map_err(|e| invalid_zip(path, e))?;
            for i in 0..zip.len() {
                let mut entry = zip.by_index(i).map_err(|e| invalid_zip(path, e))?;
                let name = entry.name().to_owned();
                if kind(&name).is_none() {
                    continue;
                }
                let mut json = String::new();
                entry.read_to_string(&mut json)?;
                found |= add_file(&mut export, &name, &json)?;
            }
        } else {
            let name = path.to_string_lossy();
            if kind(&name).is_some() {
                let json = std::fs::read_to_string(path)?;
                found |= add_file(&mut export, &name, &json)?;
            }
        }
    }
    if !found {
        return Err(AppError::invalid(
            "Aucun fichier de l'export Spotify trouvé (YourLibrary.json ou Playlist1.json)",
        ));
    }
    Ok(export)
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Library,
    Playlists,
}

/// `…/YourLibrary.json` or `…/Playlist<n>.json` (any folder, as in the zip).
fn kind(path: &str) -> Option<Kind> {
    let name = path.rsplit(['/', '\\']).next()?;
    if name == "YourLibrary.json" {
        return Some(Kind::Library);
    }
    let number = name.strip_prefix("Playlist")?.strip_suffix(".json")?;
    (!number.is_empty() && number.bytes().all(|b| b.is_ascii_digit())).then_some(Kind::Playlists)
}

/// Adds one file of the export; returns whether it was one.
fn add_file(export: &mut SpotifyExport, name: &str, json: &str) -> AppResult<bool> {
    let file_name = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let bad = |e: serde_json::Error| AppError::invalid(format!("{file_name} illisible : {e}"));
    match kind(name) {
        Some(Kind::Library) => {
            let library: Library = serde_json::from_str(json).map_err(bad)?;
            export.liked.extend(
                library
                    .tracks
                    .into_iter()
                    .filter_map(|t| track(t.track, t.artist, t.album)),
            );
            export.artists.extend(
                library
                    .artists
                    .into_iter()
                    .filter_map(|a| a.name.map(|n| n.trim().to_owned()))
                    .filter(|n| !n.is_empty()),
            );
            Ok(true)
        }
        Some(Kind::Playlists) => {
            let playlists: Playlists = serde_json::from_str(json).map_err(bad)?;
            export
                .playlists
                .extend(playlists.playlists.into_iter().filter_map(|p| {
                    let name = p
                        .name
                        .map(|n| n.trim().to_owned())
                        .filter(|n| !n.is_empty())?;
                    let tracks = p
                        .items
                        .into_iter()
                        .filter_map(|i| i.track)
                        .filter_map(|t| track(t.track_name, t.artist_name, t.album_name))
                        .collect();
                    Some(SpotifyPlaylist { name, tracks })
                }));
            Ok(true)
        }
        None => Ok(false),
    }
}

fn track(
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
) -> Option<RemoteTrack> {
    let clean = |s: Option<String>| s.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
    Some(RemoteTrack {
        title: clean(title)?,
        artist: clean(artist),
        album: clean(album),
        duration: 0.0,
        youtube_id: None,
    })
}

fn has_extension(path: &Path, ext: &str) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

fn invalid_zip(path: &Path, e: zip::result::ZipError) -> AppError {
    AppError::invalid(format!(
        "{} n'est pas une archive lisible : {e}",
        path.file_name().unwrap_or_default().to_string_lossy()
    ))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;
    use crate::library::tests::temp_dir;

    const LIBRARY: &str = r#"{
        "tracks": [
            {"artist": "Daft Punk", "album": "Random Access Memories", "track": "Get Lucky",
             "uri": "spotify:track:69kOkLUCkxIZYexIgSG8rq"},
            {"artist": "Nobody", "album": "", "track": "  "}
        ],
        "albums": [],
        "artists": [{"name": "Daft Punk", "uri": "spotify:artist:4tZwfgrHOc3mvqYlEYSvVi"}],
        "shows": [], "episodes": [], "bannedTracks": [], "other": []
    }"#;

    const PLAYLISTS: &str = r#"{
        "playlists": [{
            "name": "Rock",
            "lastModifiedDate": "2024-01-01",
            "items": [
                {"track": {"trackName": "Bohemian Rhapsody - Remastered 2011",
                           "artistName": "Queen", "albumName": "A Night at the Opera",
                           "trackUri": "spotify:track:x"},
                 "episode": null, "localTrack": null, "audiobook": null,
                 "addedDate": "2024-01-01"},
                {"track": null, "episode": {"episodeName": "Podcast"}, "localTrack": null}
            ],
            "description": null,
            "numberOfFollowers": 0
        }, {"name": "", "items": []}]
    }"#;

    #[test]
    fn export_files_are_recognized() {
        assert_eq!(
            kind("Spotify Account Data/YourLibrary.json"),
            Some(Kind::Library)
        );
        assert_eq!(kind("Playlist12.json"), Some(Kind::Playlists));
        assert_eq!(kind("C:\\x\\Playlist1.json"), Some(Kind::Playlists));
        assert_eq!(kind("Playlist.json"), None);
        assert_eq!(kind("StreamingHistory0.json"), None);
    }

    #[test]
    fn parses_liked_tracks_artists_and_playlists() {
        let mut export = SpotifyExport::default();
        assert!(add_file(&mut export, "YourLibrary.json", LIBRARY).unwrap());
        assert!(add_file(&mut export, "Playlist1.json", PLAYLISTS).unwrap());
        assert!(!add_file(&mut export, "Userdata.json", "{}").unwrap());
        assert!(add_file(&mut export, "Playlist2.json", "not json").is_err());

        assert_eq!(export.liked.len(), 1);
        assert_eq!(export.liked[0].title, "Get Lucky");
        assert_eq!(export.liked[0].artist.as_deref(), Some("Daft Punk"));
        assert_eq!(export.liked[0].youtube_id, None);
        assert_eq!(export.artists, ["Daft Punk"]);
        assert_eq!(export.playlists.len(), 1);
        assert_eq!(export.playlists[0].name, "Rock");
        assert_eq!(export.playlists[0].tracks.len(), 1);
        assert_eq!(
            export.playlists[0].tracks[0].artist.as_deref(),
            Some("Queen")
        );
    }

    #[test]
    fn reads_the_zip_or_extracted_files() {
        let dir = temp_dir("spotify");
        let zip_path = dir.join("my_spotify_data.zip");
        {
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
            let options = zip::write::SimpleFileOptions::default();
            zip.start_file("Spotify Account Data/YourLibrary.json", options)
                .unwrap();
            zip.write_all(LIBRARY.as_bytes()).unwrap();
            zip.start_file("Spotify Account Data/Playlist1.json", options)
                .unwrap();
            zip.write_all(PLAYLISTS.as_bytes()).unwrap();
            zip.start_file("Spotify Account Data/ReadMeFirst.pdf", options)
                .unwrap();
            zip.finish().unwrap();
        }
        let from_zip = read_export(&[zip_path.to_string_lossy().into_owned()]).unwrap();
        assert_eq!(from_zip.liked.len(), 1);
        assert_eq!(from_zip.playlists.len(), 1);

        let json = dir.join("Playlist1.json");
        std::fs::write(&json, PLAYLISTS).unwrap();
        let from_json = read_export(&[json.to_string_lossy().into_owned()]).unwrap();
        assert_eq!(from_json.playlists.len(), 1);

        let other = dir.join("notes.json");
        std::fs::write(&other, "{}").unwrap();
        assert!(read_export(&[other.to_string_lossy().into_owned()]).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
