use tauri::{AppHandle, State};

use super::Downloads;
use crate::error::AppResult;
use crate::library::Library;

#[tauri::command]
#[specta::specta]
pub fn download_track(
    app: AppHandle,
    downloads: State<'_, Downloads>,
    library: State<'_, Library>,
    youtube_id: String,
) -> AppResult<()> {
    downloads.start(app, library.inner().clone(), youtube_id)
}

#[tauri::command]
#[specta::specta]
pub fn cancel_download(app: AppHandle, downloads: State<'_, Downloads>, youtube_id: String) {
    downloads.cancel(&app, &youtube_id);
}
