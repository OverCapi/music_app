use tauri::State;

use super::{Feedback, FeedbackKind, FeedbackState, FeedbackTarget};
use crate::error::AppResult;

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
    target: FeedbackTarget,
    kind: FeedbackKind,
) -> AppResult<()> {
    feedback.set(&target, kind)
}

#[tauri::command]
#[specta::specta]
pub fn clear_feedback(feedback: State<'_, Feedback>, target: FeedbackTarget) -> AppResult<()> {
    feedback.clear(&target)
}
