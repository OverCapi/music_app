//! YouTube / YouTube Music search through yt-dlp, page by page.
//!
//! - YouTube: a "flat" search already returns channel, duration and views.
//! - YouTube Music: the flat search (~1.5 s) only returns ids and titles, so the
//!   results are returned right away, then one full extraction per result (a few
//!   in parallel) fetches artist/album/duration, pushed as [`SearchDetails`] events.
//!   A newer search cancels the detail fetches of the previous one (`search_id`).

pub mod commands;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use group_protocol::is_valid_youtube_id;
use serde::{Deserialize, Serialize};
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Url};
use tauri_specta::Event;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::error::{AppError, AppResult};
use crate::ytdlp;

const PAGE_SIZE: u32 = 10;
const MAX_PARALLEL_DETAILS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum SearchSource {
    Music,
    Youtube,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
pub struct SearchResult {
    /// YouTube video id (validated).
    pub id: String,
    pub title: String,
    pub thumbnail: String,
    /// Artist (YouTube Music) or channel (YouTube).
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<f64>,
    pub views: Option<u64>,
    /// True while artist/album/duration are still being fetched (YouTube Music).
    pub details_pending: bool,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
pub struct SearchResponse {
    pub search_id: u64,
    pub results: Vec<SearchResult>,
    pub has_more: bool,
}

#[derive(Clone, Debug, Serialize, specta::Type, tauri_specta::Event)]
pub struct SearchDetails {
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
    channel: Option<String>,
    uploader: Option<String>,
    duration: Option<f64>,
    view_count: Option<u64>,
}

#[derive(Deserialize)]
struct Details {
    artist: Option<String>,
    album: Option<String>,
    duration: Option<f64>,
}

/// Managed state.
#[derive(Default)]
pub struct Search {
    latest: AtomicU64,
    /// Background detail fetches of the latest search (one per loaded page).
    details_tasks: Mutex<Vec<JoinHandle<()>>>,
}

impl Search {
    /// `search_id`: pass the id of the first page to load more of the same search;
    /// `None` starts a new search.
    pub async fn search(
        &self,
        app: AppHandle,
        query: &str,
        source: SearchSource,
        offset: u32,
        search_id: Option<u64>,
    ) -> AppResult<SearchResponse> {
        let query = query.trim();
        if query.is_empty() {
            return Err(AppError::invalid("La recherche est vide"));
        }
        let search_id = match search_id {
            Some(id) if offset > 0 => {
                if id != self.latest.load(Ordering::SeqCst) {
                    return Err(AppError::invalid("Cette recherche a été remplacée"));
                }
                id
            }
            _ => {
                let id = self.latest.fetch_add(1, Ordering::SeqCst) + 1;
                for task in self.details_tasks.lock().unwrap().drain(..) {
                    // Dropping the task's JoinSet kills its yt-dlp processes.
                    task.abort();
                }
                id
            }
        };

        let end = offset + PAGE_SIZE;
        let target = match source {
            SearchSource::Music => {
                let mut url =
                    Url::parse_with_params("https://music.youtube.com/search", [("q", query)])
                        .map_err(|e| AppError::Internal(e.to_string()))?;
                // Only the "Songs" section of the results page.
                url.set_fragment(Some("songs"));
                url.to_string()
            }
            SearchSource::Youtube => format!("ytsearch{end}:{query}"),
        };
        let entries: Vec<FlatEntry> = ytdlp::run_json(&[
            &target,
            "--flat-playlist",
            "--dump-json",
            "--playlist-start",
            &(offset + 1).to_string(),
            "--playlist-end",
            &end.to_string(),
        ])
        .await?;
        let has_more = entries.len() as u32 == PAGE_SIZE;
        let results: Vec<SearchResult> = entries
            .into_iter()
            .filter(|e| is_valid_youtube_id(&e.id))
            .map(|e| to_result(e, source))
            .collect();

        // If a newer search started meanwhile, these results will be discarded.
        if source == SearchSource::Music && self.latest.load(Ordering::SeqCst) == search_id {
            let ids = results.iter().map(|r| r.id.clone()).collect();
            let task = tauri::async_runtime::spawn(fetch_details(app, search_id, ids));
            self.details_tasks.lock().unwrap().push(task);
        }

        Ok(SearchResponse {
            search_id,
            results,
            has_more,
        })
    }
}

fn to_result(e: FlatEntry, source: SearchSource) -> SearchResult {
    SearchResult {
        thumbnail: format!("https://i.ytimg.com/vi/{}/mqdefault.jpg", e.id),
        title: e.title.unwrap_or_else(|| e.id.clone()),
        artist: e.channel.or(e.uploader),
        album: None,
        duration: e.duration,
        views: e.view_count,
        details_pending: source == SearchSource::Music,
        id: e.id,
    }
}

/// One full extraction per result, at most [`MAX_PARALLEL_DETAILS`] at a time.
async fn fetch_details(app: AppHandle, search_id: u64, ids: Vec<String>) {
    let permits = Arc::new(Semaphore::new(MAX_PARALLEL_DETAILS));
    let mut tasks = JoinSet::new();
    for id in ids {
        let permits = Arc::clone(&permits);
        tasks.spawn(async move {
            let _permit = permits.acquire_owned().await.ok()?;
            let url = ytdlp::video_url(&id).ok()?;
            let details = ytdlp::run_json::<Details>(&[
                &url,
                "--skip-download",
                "--no-playlist",
                "-O",
                "%(.{artist,album,duration})j",
            ])
            .await
            .map_err(|e| tracing::warn!("yt-dlp details for {id}: {e}"))
            .ok()?
            .pop()?;
            Some(SearchDetails {
                search_id,
                id,
                artist: details.artist,
                album: details.album,
                duration: details.duration,
            })
        });
    }
    while let Some(done) = tasks.join_next().await {
        if let Ok(Some(details)) = done {
            let _ = details.emit(&app);
        }
    }
}
