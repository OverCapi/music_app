use tauri::State;

use super::{Feedback, FeedbackKind, FeedbackState, FeedbackTarget};
use crate::downloads::Downloads;
use crate::error::AppResult;
use crate::library::Library;

/// Initial load; later changes are pushed by `FeedbackChanged`.
#[tauri::command]
#[specta::specta]
pub fn get_feedback(feedback: State<'_, Feedback>) -> AppResult<FeedbackState> {
    feedback.state()
}

#[tauri::command]
#[specta::specta]
pub fn set_feedback(
    feedback: State<'_, Feedback>,
    downloads: State<'_, Downloads>,
    library: State<'_, Library>,
    target: FeedbackTarget,
    kind: FeedbackKind,
) -> AppResult<()> {
    feedback.set(&target, kind)?;
    // Liking a recommended track keeps it: it joins the library for good.
    if let (FeedbackKind::Like, FeedbackTarget::Track { youtube_id, .. }) = (kind, &target)
        && let Err(e) = downloads.keep(&library, youtube_id)
    {
        tracing::warn!("keep liked track {youtube_id}: {e}");
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn clear_feedback(feedback: State<'_, Feedback>, target: FeedbackTarget) -> AppResult<()> {
    feedback.clear(&target)
}
