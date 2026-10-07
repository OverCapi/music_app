//! YouTube / YouTube Music search through yt-dlp, page by page.
//!
//! - YouTube: a "flat" search already returns channel, duration and views.
//! - YouTube Music: the flat search (~1.5 s) only returns ids and titles, so the
//!   results are returned right away, then one full extraction per result (a few
//!   in parallel) fetches artist/album/duration, pushed as [`SearchDetails`] events.
//!   A newer search cancels the detail fetches of the previous one (`search_id`).

pub mod commands;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::error::AppResult;

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

/// Managed state.
#[derive(Default)]
pub struct Search {
    // Filled in by the implementation (latest search id, detail tasks).
}

impl Search {
    /// `search_id`: pass the id of the first page to load more of the same search;
    /// `None` starts a new search.
    pub async fn search(
        &self,
        _app: AppHandle,
        _query: &str,
        _source: SearchSource,
        _offset: u32,
        _search_id: Option<u64>,
    ) -> AppResult<SearchResponse> {
        todo!()
    }
}
