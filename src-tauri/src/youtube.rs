//! YouTube / YouTube Music search through the yt-dlp binary, page by page.
//!
//! - YouTube: a "flat" search already returns channel, duration and views.
//! - YouTube Music: the flat search (~1.5 s) only returns ids and titles, so the
//!   results are returned right away, then one full extraction per result (a few
//!   in parallel) fetches artist/album/duration, pushed to the UI as
//!   `search-details` events as they arrive.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Url};
use tokio::process::Command;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

pub const DETAILS_EVENT: &str = "search-details";
const PAGE_SIZE: u32 = 10;
const MAX_PARALLEL_DETAILS: usize = 4;

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SearchSource {
    Music,
    Youtube,
}

#[derive(Clone, Serialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub url: String,
    pub thumbnail: String,
    /// Artist (YouTube Music) or channel (YouTube).
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<f64>,
    pub views: Option<u64>,
    /// True while artist/album/duration are still being fetched (YouTube Music).
    pub details_pending: bool,
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub search_id: u64,
    pub results: Vec<SearchResult>,
    pub has_more: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SearchDetails {
    #[serde(default)]
    pub search_id: u64,
    pub id: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<f64>,
}

#[derive(Deserialize)]
struct FlatEntry {
    id: String,
    title: Option<String>,
    url: Option<String>,
    channel: Option<String>,
    uploader: Option<String>,
    duration: Option<f64>,
    view_count: Option<u64>,
}

#[derive(Default)]
pub struct YoutubeSearch {
    latest: AtomicU64,
    /// Background detail fetches of the latest search (one per loaded page).
    details_tasks: Mutex<Vec<JoinHandle<()>>>,
}

impl YoutubeSearch {
    /// `offset == 0` starts a new search (cancelling the previous one); otherwise loads
    /// the next page of `search_id`, which must still be the latest search.
    pub async fn search(
        &self,
        app: AppHandle,
        query: &str,
        source: SearchSource,
        offset: u32,
        search_id: Option<u64>,
    ) -> Result<SearchResponse, String> {
        let query = query.trim();
        if query.is_empty() {
            return Err("La recherche est vide".to_owned());
        }
        let search_id = match search_id {
            Some(id) if offset > 0 => {
                if id != self.latest.load(Ordering::SeqCst) {
                    return Err("Cette recherche a été remplacée".to_owned());
                }
                id
            }
            _ => {
                let id = self.latest.fetch_add(1, Ordering::SeqCst) + 1;
                self.cancel_details();
                id
            }
        };

        let start = (offset + 1).to_string();
        let end = offset + PAGE_SIZE;
        let target = match source {
            SearchSource::Music => {
                let mut url =
                    Url::parse_with_params("https://music.youtube.com/search", [("q", query)])
                        .map_err(|e| e.to_string())?;
                // Only the "Songs" section of the results page.
                url.set_fragment(Some("songs"));
                url.to_string()
            }
            SearchSource::Youtube => format!("ytsearch{end}:{query}"),
        };
        let stdout = run_ytdlp(&[
            &target,
            "--flat-playlist",
            "--dump-json",
            "--playlist-start",
            &start,
            "--playlist-end",
            &end.to_string(),
        ])
        .await?;

        let results: Vec<SearchResult> = stdout
            .lines()
            .filter_map(|line| serde_json::from_str::<FlatEntry>(line).ok())
            .map(|e| to_result(e, source))
            .collect();
        let has_more = results.len() as u32 == PAGE_SIZE;

        // If a newer search started meanwhile, these results will be discarded.
        if source == SearchSource::Music && self.latest.load(Ordering::SeqCst) == search_id {
            let task = tauri::async_runtime::spawn(fetch_details(app, search_id, results.clone()));
            self.details_tasks.lock().unwrap().push(task);
        }

        Ok(SearchResponse {
            search_id,
            results,
            has_more,
        })
    }

    fn cancel_details(&self) {
        for task in self.details_tasks.lock().unwrap().drain(..) {
            // Dropping the task's JoinSet kills its yt-dlp processes (kill_on_drop).
            task.abort();
        }
    }
}

fn to_result(e: FlatEntry, source: SearchSource) -> SearchResult {
    let music = source == SearchSource::Music;
    let default_url = if music {
        format!("https://music.youtube.com/watch?v={}", e.id)
    } else {
        format!("https://www.youtube.com/watch?v={}", e.id)
    };
    SearchResult {
        thumbnail: format!("https://i.ytimg.com/vi/{}/mqdefault.jpg", e.id),
        url: e.url.unwrap_or(default_url),
        title: e.title.unwrap_or_else(|| e.id.clone()),
        artist: e.channel.or(e.uploader),
        album: None,
        duration: e.duration,
        views: e.view_count,
        details_pending: music,
        id: e.id,
    }
}

async fn fetch_details(app: AppHandle, search_id: u64, results: Vec<SearchResult>) {
    let permits = Arc::new(Semaphore::new(MAX_PARALLEL_DETAILS));
    let mut tasks = JoinSet::new();
    for result in results {
        let permits = Arc::clone(&permits);
        tasks.spawn(async move {
            let _permit = permits.acquire_owned().await.ok()?;
            let stdout = run_ytdlp(&[
                &result.url,
                "--skip-download",
                "--no-playlist",
                "-O",
                "%(.{id,artist,album,duration})j",
            ])
            .await
            .map_err(|e| eprintln!("yt-dlp details for {}: {e}", result.id))
            .ok()?;
            serde_json::from_str::<SearchDetails>(stdout.trim()).ok()
        });
    }
    while let Some(done) = tasks.join_next().await {
        if let Ok(Some(mut details)) = done {
            details.search_id = search_id;
            let _ = app.emit(DETAILS_EVENT, details);
        }
    }
}

async fn run_ytdlp(args: &[&str]) -> Result<String, String> {
    // Arguments are passed directly (no shell), so the query cannot inject anything.
    let output = Command::new(ytdlp_path())
        .args(args)
        .arg("--no-warnings")
        .stdin(Stdio::null())
        .kill_on_drop(true)
        .output()
        .await
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                "yt-dlp est introuvable (ni dans le PATH, ni dans ~/Applications/yt-dlp)".to_owned()
            }
            _ => format!("Impossible de lancer yt-dlp : {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let reason = stderr.lines().rev().find(|l| !l.trim().is_empty());
        return Err(format!(
            "La recherche a échoué : {}",
            reason.unwrap_or("erreur inconnue de yt-dlp")
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// `yt-dlp` from PATH, else the standalone binary in ~/Applications
/// (apps started from the desktop don't always have ~/.local/bin in PATH).
pub(crate) fn ytdlp_path() -> PathBuf {
    find_in_path("yt-dlp")
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join("Applications/yt-dlp/yt-dlp"))
        })
        .unwrap_or_else(|| PathBuf::from("yt-dlp"))
}

pub(crate) fn find_in_path(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .map(|dir| dir.join(name))
        .find(|p| p.is_file())
}
