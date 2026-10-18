//! Finding YouTube live streams (searched by keyword, or from a pasted link) so the
//! audio engine can play them as a radio (see `audio::live`).
//!
//! A pasted link is parsed here and only the validated video id or channel handle is
//! kept: the URL given to yt-dlp is rebuilt from it ([`ytdlp::watch_url`]).

pub mod commands;
pub mod proxy;

use group_protocol::is_valid_youtube_id;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::Url;

use crate::db::{Db, now_secs};
use crate::error::{AppError, AppResult};
use crate::ytdlp;

/// How many lives a search or a channel lists at most.
const MAX_LIVES: &str = "20";
/// How many channels a search lists at most.
const MAX_CHANNELS: &str = "6";
/// YouTube's "Channel" filter of the search results page.
const CHANNEL_FILTER: &str = "EgIQAg==";
/// YouTube's "Live" filter of the search results page.
const LIVE_FILTER: &str = "EgJAAQ==";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct LiveStream {
    /// YouTube video id (validated).
    pub id: String,
    pub title: String,
    pub channel: Option<String>,
    pub thumbnail: String,
    pub viewers: Option<u64>,
}

/// One line of yt-dlp's JSON output about a video.
#[derive(Deserialize)]
struct Entry {
    id: String,
    title: Option<String>,
    channel: Option<String>,
    uploader: Option<String>,
    view_count: Option<u64>,
    concurrent_view_count: Option<u64>,
    live_status: Option<String>,
}

impl Entry {
    fn is_live(&self) -> bool {
        self.live_status.as_deref() == Some("is_live")
    }

    fn into_stream(self) -> LiveStream {
        LiveStream {
            thumbnail: format!("https://i.ytimg.com/vi/{}/mqdefault.jpg", self.id),
            title: self.title.unwrap_or_else(|| self.id.clone()),
            channel: self.channel.or(self.uploader),
            viewers: self.concurrent_view_count.or(self.view_count),
            id: self.id,
        }
    }
}

/// Lives matching `query`, with the viewer count when YouTube gives it.
pub async fn search(query: &str) -> AppResult<Vec<LiveStream>> {
    let query = query.trim();
    if query.is_empty() {
        return Err(AppError::invalid("La recherche est vide"));
    }
    let url = Url::parse_with_params(
        "https://www.youtube.com/results",
        [("search_query", query), ("sp", LIVE_FILTER)],
    )
    .map_err(|e| AppError::Internal(e.to_string()))?;
    // The search filter only returns lives; `live_status` is absent from these entries.
    let entries = list(url.as_str()).await?;
    Ok(streams(entries, |e| e.live_status.is_none() || e.is_live()))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct LiveChannel {
    /// YouTube channel id (`UC…`, validated).
    pub id: String,
    pub name: String,
    /// `@handle`, when known.
    pub handle: Option<String>,
    pub thumbnail: String,
    pub followers: Option<u64>,
}

#[derive(Deserialize)]
struct ChannelEntry {
    id: String,
    title: Option<String>,
    uploader_id: Option<String>,
    channel_follower_count: Option<u64>,
    thumbnails: Option<Vec<Thumbnail>>,
}

#[derive(Deserialize)]
struct Thumbnail {
    url: String,
}

/// Channels matching `query`.
pub async fn search_channels(query: &str) -> AppResult<Vec<LiveChannel>> {
    let query = query.trim();
    if query.is_empty() {
        return Err(AppError::invalid("La recherche est vide"));
    }
    let url = Url::parse_with_params(
        "https://www.youtube.com/results",
        [("search_query", query), ("sp", CHANNEL_FILTER)],
    )
    .map_err(|e| AppError::Internal(e.to_string()))?;
    let entries: Vec<ChannelEntry> = ytdlp::run_json(&[
        url.as_str(),
        "--flat-playlist",
        "--dump-json",
        "--playlist-end",
        MAX_CHANNELS,
    ])
    .await?;
    Ok(entries
        .into_iter()
        .filter(|e| ytdlp::is_valid_channel_id(&e.id))
        .map(|e| LiveChannel {
            name: e.title.unwrap_or_else(|| e.id.clone()),
            handle: e.uploader_id.filter(|h| h.starts_with('@')),
            // The largest picture is the last; some URLs have no scheme.
            thumbnail: e
                .thumbnails
                .and_then(|t| t.into_iter().last())
                .map(|t| t.url)
                .map(|u| {
                    if u.starts_with("//") {
                        format!("https:{u}")
                    } else {
                        u
                    }
                })
                .unwrap_or_default(),
            followers: e.channel_follower_count,
            id: e.id,
        })
        .collect())
}

/// Lives running now on a channel.
pub async fn channel_lives(channel_id: &str) -> AppResult<Vec<LiveStream>> {
    let entries = list(&ytdlp::channel_id_streams_url(channel_id)?).await?;
    Ok(streams(entries, Entry::is_live))
}

/// The favorite channels, in the `live_channels` table (most recently added first).
#[derive(Clone)]
pub struct Favorites {
    db: Db,
}

impl Favorites {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub fn list(&self) -> AppResult<Vec<LiveChannel>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, handle, thumbnail, followers FROM live_channels
             ORDER BY added_at DESC, rowid DESC",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(LiveChannel {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    handle: r.get(2)?,
                    thumbnail: r.get(3)?,
                    followers: r.get::<_, Option<i64>>(4)?.map(|n| n as u64),
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    /// Adds (or refreshes) a favorite; returns the new list.
    pub fn add(&self, channel: &LiveChannel) -> AppResult<Vec<LiveChannel>> {
        if !ytdlp::is_valid_channel_id(&channel.id) {
            return Err(AppError::invalid("Identifiant de chaîne invalide"));
        }
        self.db.conn().execute(
            "INSERT INTO live_channels (id, name, handle, thumbnail, followers, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET name = ?2, handle = ?3, thumbnail = ?4, followers = ?5",
            params![
                channel.id,
                channel.name,
                channel.handle,
                channel.thumbnail,
                channel.followers.map(|n| n as i64),
                now_secs()
            ],
        )?;
        self.list()
    }

    pub fn remove(&self, id: &str) -> AppResult<Vec<LiveChannel>> {
        self.db
            .conn()
            .execute("DELETE FROM live_channels WHERE id = ?1", [id])?;
        self.list()
    }
}

/// Lives behind a pasted YouTube link: the video itself, or the lives running now on a
/// channel (`@handle`).
pub async fn resolve_url(input: &str) -> AppResult<Vec<LiveStream>> {
    match parse_target(input)? {
        Target::Channel(handle) => {
            let lives = streams(
                list(&ytdlp::channel_streams_url(&handle)?).await?,
                Entry::is_live,
            );
            if lives.is_empty() {
                return Err(AppError::invalid("Cette chaîne n'a aucun direct en cours"));
            }
            Ok(lives)
        }
        Target::Video(id) => {
            let entry = ytdlp::run_json::<Entry>(&[
                &ytdlp::watch_url(&id)?,
                "--skip-download",
                "--no-playlist",
                "-O",
                "%(.{id,title,channel,uploader,concurrent_view_count,live_status})j",
            ])
            .await?
            .pop()
            .filter(Entry::is_live)
            .ok_or_else(|| AppError::invalid("Cette vidéo n'est pas un direct en cours"))?;
            Ok(vec![entry.into_stream()])
        }
    }
}

async fn list(url: &str) -> AppResult<Vec<Entry>> {
    ytdlp::run_json(&[
        url,
        "--flat-playlist",
        "--dump-json",
        "--playlist-end",
        MAX_LIVES,
    ])
    .await
}

fn streams(entries: Vec<Entry>, keep: impl Fn(&Entry) -> bool) -> Vec<LiveStream> {
    entries
        .into_iter()
        .filter(|e| is_valid_youtube_id(&e.id) && keep(e))
        .map(Entry::into_stream)
        .collect()
}

#[derive(Debug, PartialEq)]
enum Target {
    Video(String),
    /// Channel handle, without the `@`.
    Channel(String),
}

/// Understands `watch?v=`, `youtu.be/`, `/live/<id>` and `@channel[/streams]` links.
fn parse_target(input: &str) -> AppResult<Target> {
    let unknown = || AppError::invalid("Lien YouTube non reconnu");
    let input = input.trim();
    // A bare handle (`@name`) is not a URL: `https://@name` would parse as a user name.
    if let Some(rest) = input.strip_prefix('@') {
        return channel(rest.split('/').next().unwrap_or_default());
    }
    let url = if input.contains("://") {
        Url::parse(input)
    } else {
        Url::parse(&format!("https://{input}"))
    }
    .map_err(|_| unknown())?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(unknown());
    }
    let host = url.host_str().ok_or_else(unknown)?;
    let host = host
        .strip_prefix("www.")
        .or_else(|| host.strip_prefix("m."))
        .or_else(|| host.strip_prefix("music."))
        .unwrap_or(host);
    let segments: Vec<&str> = url.path_segments().map_or_else(Vec::new, |s| s.collect());
    let video = |id: Option<&str>| match id {
        Some(id) if is_valid_youtube_id(id) => Ok(Target::Video(id.to_owned())),
        _ => Err(unknown()),
    };
    match (host, segments.as_slice()) {
        ("youtu.be", [id, ..]) => video(Some(id)),
        ("youtube.com", ["watch", ..]) => video(
            url.query_pairs()
                .find(|(k, _)| k == "v")
                .as_ref()
                .map(|(_, v)| &**v),
        ),
        ("youtube.com", ["live", id, ..]) => video(Some(id)),
        ("youtube.com", [first, ..]) if first.starts_with('@') => channel(&first[1..]),
        _ => Err(unknown()),
    }
}

fn channel(handle: &str) -> AppResult<Target> {
    // Same validation as the URL builder, so a bad handle fails before any yt-dlp call.
    ytdlp::channel_streams_url(handle)?;
    Ok(Target::Channel(handle.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn video(id: &str) -> Target {
        Target::Video(id.into())
    }

    #[test]
    fn video_links() {
        let id = "jfKfPfyJRdk";
        for url in [
            "https://www.youtube.com/watch?v=jfKfPfyJRdk",
            "https://www.youtube.com/watch?feature=share&v=jfKfPfyJRdk&t=3",
            "https://m.youtube.com/watch?v=jfKfPfyJRdk",
            "https://music.youtube.com/watch?v=jfKfPfyJRdk",
            "youtube.com/watch?v=jfKfPfyJRdk",
            "https://youtu.be/jfKfPfyJRdk?si=abc",
            "https://www.youtube.com/live/jfKfPfyJRdk",
        ] {
            assert_eq!(parse_target(url).unwrap(), video(id), "{url}");
        }
    }

    #[test]
    fn channel_links() {
        for url in [
            "https://www.youtube.com/@LofiGirl",
            "https://www.youtube.com/@LofiGirl/streams",
            "@LofiGirl",
            "@LofiGirl/streams",
            " @LofiGirl ",
        ] {
            assert_eq!(
                parse_target(url).unwrap(),
                Target::Channel("LofiGirl".into()),
                "{url}"
            );
        }
    }

    #[test]
    fn rejects_everything_else() {
        for url in [
            "",
            "lofi girl",
            "https://evil.example/watch?v=jfKfPfyJRdk",
            "https://youtube.com.evil.example/watch?v=jfKfPfyJRdk",
            "ftp://youtube.com/watch?v=jfKfPfyJRdk",
            "https://www.youtube.com/watch?v=short",
            "https://www.youtube.com/watch",
            "https://www.youtube.com/playlist?list=PL123456",
            "https://www.youtube.com/@a",
            "@a b",
        ] {
            assert!(parse_target(url).is_err(), "{url:?}");
        }
    }

    #[test]
    fn only_running_lives_are_kept() {
        let entry = |id: &str, status: Option<&str>| Entry {
            id: id.into(),
            title: None,
            channel: None,
            uploader: Some("Up".into()),
            view_count: Some(5),
            concurrent_view_count: None,
            live_status: status.map(Into::into),
        };
        let list = || {
            vec![
                entry("jfKfPfyJRdk", Some("is_live")),
                entry("aaaaaaaaaaa", Some("was_live")),
                entry("bbbbbbbbbbb", None),
                entry("bad id", Some("is_live")),
            ]
        };
        let strict = streams(list(), Entry::is_live);
        assert_eq!(strict.len(), 1);
        assert_eq!(strict[0].channel.as_deref(), Some("Up"));
        assert_eq!(strict[0].viewers, Some(5));
        assert_eq!(strict[0].title, "jfKfPfyJRdk");
        assert_eq!(
            streams(list(), |e| e.live_status.is_none() || e.is_live()).len(),
            2
        );
    }

    #[test]
    fn favorites_add_refresh_remove() {
        let fav = Favorites::new(Db::in_memory());
        let channel = |name: &str| LiveChannel {
            id: "UCSJ4gkVC6NrvII8umztf0Ow".into(),
            name: name.into(),
            handle: Some("@LofiGirl".into()),
            thumbnail: "https://x/y.jpg".into(),
            followers: Some(5),
        };
        assert_eq!(fav.add(&channel("Lofi")).unwrap().len(), 1);
        let list = fav.add(&channel("Lofi Girl")).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Lofi Girl");
        let bad = LiveChannel {
            id: "nope".into(),
            ..channel("x")
        };
        assert!(fav.add(&bad).is_err());
        assert!(fav.remove("UCSJ4gkVC6NrvII8umztf0Ow").unwrap().is_empty());
    }
}
