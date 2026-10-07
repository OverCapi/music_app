use tauri::{AppHandle, State};

use super::{Search, SearchResponse, SearchSource};
use crate::error::AppResult;

#[tauri::command]
#[specta::specta]
pub async fn search_youtube(
    app: AppHandle,
    search: State<'_, Search>,
    query: String,
    source: SearchSource,
    offset: u32,
    search_id: Option<u64>,
) -> AppResult<SearchResponse> {
    search.search(app, &query, source, offset, search_id).await
}
