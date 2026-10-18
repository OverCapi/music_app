//! `live://localhost/...`: lets the webview play the video of a live. YouTube's HLS
//! servers send no CORS headers, so hls.js could not fetch them directly; this
//! scheme relays the playlist and the segments with `Access-Control-Allow-Origin`.
//!
//! - `/<youtube id>.m3u8`: the video-only playlist, its segment URLs rewritten to go
//!   through `/seg`. The yt-dlp-resolved URL is cached until shortly before it expires.
//! - `/seg?u=<url>`: one segment. Only `https` URLs on `*.googlevideo.com` are relayed,
//!   or the page could use the app as an open proxy.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use group_protocol::is_valid_youtube_id;
use tauri::Url;
use tauri::http::{Request, Response};

use crate::ytdlp;

const SEGMENT_HOST_SUFFIX: &str = ".googlevideo.com";
/// A cached playlist URL is renewed when it expires within this many seconds.
const EXPIRY_MARGIN_SECS: u64 = 300;

#[derive(Clone)]
pub struct LiveProxy {
    client: reqwest::Client,
    /// Playlist URL per live id, with its expiry (unix seconds, 0 when unknown).
    playlists: Arc<Mutex<HashMap<String, (Url, u64)>>>,
}

impl LiveProxy {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            playlists: Arc::default(),
        }
    }

    /// Answers one `live://` request.
    pub async fn handle(&self, request: Request<Vec<u8>>) -> Response<Vec<u8>> {
        let path = request.uri().path().trim_start_matches('/').to_owned();
        let result = if path == "seg" {
            self.segment(request.uri().query().unwrap_or_default())
                .await
        } else if let Some(id) = path.strip_suffix(".m3u8") {
            self.playlist(id).await
        } else {
            Err(Status::NotFound)
        };
        match result {
            Ok((content_type, body, cache)) => Response::builder()
                .header("Content-Type", content_type)
                .header("Access-Control-Allow-Origin", "*")
                .header("Cache-Control", cache)
                .body(body),
            Err(status) => {
                tracing::warn!("live proxy: {path}: {status:?}");
                Response::builder()
                    .status(status.code())
                    .header("Access-Control-Allow-Origin", "*")
                    .body(Vec::new())
            }
        }
        .expect("static response parts are valid")
    }

    async fn playlist(&self, id: &str) -> Result<Body, Status> {
        if !is_valid_youtube_id(id) {
            return Err(Status::Forbidden);
        }
        let mut fresh = false;
        loop {
            let url = self.playlist_url(id, fresh).await?;
            let response = self.client.get(url.clone()).send().await;
            match response {
                // The signed URL is no longer valid: resolve it once more.
                Ok(r) if r.status().as_u16() == 403 && !fresh => fresh = true,
                Ok(r) if r.status().is_success() => {
                    let text = r.text().await.map_err(|_| Status::BadGateway)?;
                    return Ok((
                        "application/vnd.apple.mpegurl".into(),
                        rewrite_manifest(&text, &url).into_bytes(),
                        "no-store",
                    ));
                }
                _ => return Err(Status::BadGateway),
            }
        }
    }

    async fn playlist_url(&self, id: &str, force: bool) -> Result<Url, Status> {
        if !force
            && let Some((url, expire)) = self.playlists.lock().unwrap().get(id)
            && (*expire == 0 || *expire > now_secs() + EXPIRY_MARGIN_SECS)
        {
            return Ok(url.clone());
        }
        let url = ytdlp::live_video_playlist_url(id)
            .await
            .map_err(|_| Status::BadGateway)?;
        let url = Url::parse(&url).map_err(|_| Status::BadGateway)?;
        let expire = expiry(&url);
        self.playlists
            .lock()
            .unwrap()
            .insert(id.to_owned(), (url.clone(), expire));
        Ok(url)
    }

    async fn segment(&self, query: &str) -> Result<Body, Status> {
        let target = Url::parse(&format!("live://localhost/seg?{query}"))
            .ok()
            .and_then(|u| {
                u.query_pairs()
                    .find(|(k, _)| k == "u")
                    .map(|(_, v)| v.into_owned())
            })
            .and_then(|u| Url::parse(&u).ok())
            .filter(allowed_segment_url)
            .ok_or(Status::Forbidden)?;
        let response = self
            .client
            .get(target)
            .send()
            .await
            .map_err(|_| Status::BadGateway)?;
        if !response.status().is_success() {
            return Err(Status::BadGateway);
        }
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("video/mp4")
            .to_owned();
        let bytes = response.bytes().await.map_err(|_| Status::BadGateway)?;
        Ok((content_type, bytes.to_vec(), "max-age=60"))
    }
}

type Body = (String, Vec<u8>, &'static str);

#[derive(Debug)]
enum Status {
    Forbidden,
    NotFound,
    BadGateway,
}

impl Status {
    fn code(&self) -> u16 {
        match self {
            Self::Forbidden => 403,
            Self::NotFound => 404,
            Self::BadGateway => 502,
        }
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// The `expire/<unix seconds>` path segment of a googlevideo URL (0 when absent).
fn expiry(url: &Url) -> u64 {
    let segments: Vec<&str> = url.path_segments().map_or_else(Vec::new, |s| s.collect());
    segments
        .windows(2)
        .find(|w| w[0] == "expire")
        .and_then(|w| w[1].parse().ok())
        .unwrap_or(0)
}

/// Only HTTPS media servers of YouTube.
fn allowed_segment_url(url: &Url) -> bool {
    url.scheme() == "https"
        && url
            .host_str()
            .is_some_and(|h| h.ends_with(SEGMENT_HOST_SUFFIX))
}

/// Points every URI of the playlist (segment lines, `URI="..."` attributes) to `/seg`.
fn rewrite_manifest(manifest: &str, base: &Url) -> String {
    let proxied = |uri: &str| -> String {
        let Ok(absolute) = base.join(uri) else {
            return uri.to_owned();
        };
        let mut out = Url::parse("live://localhost/seg").expect("static url");
        out.query_pairs_mut().append_pair("u", absolute.as_str());
        out.to_string()
    };
    manifest
        .lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else if !line.starts_with('#') {
                proxied(line)
            } else if let Some((head, rest)) = line.split_once("URI=\"") {
                match rest.split_once('"') {
                    Some((uri, tail)) => format!("{head}URI=\"{}\"{tail}", proxied(uri)),
                    None => line.to_owned(),
                }
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Url {
        Url::parse("https://manifest.googlevideo.com/api/manifest/hls_playlist/expire/1792355255/id/x.m3u8").unwrap()
    }

    #[test]
    fn segments_are_rewritten_through_the_proxy() {
        let m = "#EXTM3U\n#EXTINF:5.0,\nhttps://rr4.googlevideo.com/videoplayback/a%3Db\n#EXT-X-MAP:URI=\"init.mp4\"\n";
        let out = rewrite_manifest(m, &base());
        assert!(out.contains("\nlive://localhost/seg?u=https%3A%2F%2Frr4.googlevideo.com%2Fvideoplayback%2Fa%253Db\n"), "{out}");
        assert!(
            out.contains("URI=\"live://localhost/seg?u=https%3A%2F%2Fmanifest.googlevideo.com"),
            "{out}"
        );
        assert!(out.starts_with("#EXTM3U\n#EXTINF:5.0,\n"));
    }

    #[test]
    fn only_youtube_media_hosts_are_relayed() {
        let ok = |u: &str| allowed_segment_url(&Url::parse(u).unwrap());
        assert!(ok(
            "https://rr4---sn-u125g5-5b.googlevideo.com/videoplayback/x"
        ));
        assert!(!ok("http://rr4.googlevideo.com/x"));
        assert!(!ok("https://example.com/x"));
        assert!(!ok("https://googlevideo.com.evil.com/x"));
        assert!(!ok("https://evilgooglevideo.com/x"));
    }

    #[test]
    fn expiry_is_read_from_the_url() {
        assert_eq!(expiry(&base()), 1_792_355_255);
        assert_eq!(
            expiry(&Url::parse("https://a.googlevideo.com/x").unwrap()),
            0
        );
    }
}
