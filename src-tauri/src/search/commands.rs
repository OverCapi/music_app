use tauri::{AppHandle, State};

use super::{Search, SearchResponse};
use crate::error::AppResult;

#[tauri::command]
#[specta::specta]
pub async fn search_youtube(
    app: AppHandle,
    search: State<'_, Search>,
    query: String,
    offset: u32,
    search_id: Option<u64>,
) -> AppResult<SearchResponse> {
    search.search(app, &query, offset, search_id).await
}
