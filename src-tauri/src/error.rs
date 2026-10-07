//! The single error type of the app backend.
//!
//! Every command returns [`AppResult`]. Messages are French and shown as-is to the
//! user; on the wire an error is `{ "kind": "...", "message": "..." }`.

use serde::Serialize;

#[derive(Debug, thiserror::Error, Serialize, specta::Type)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum AppError {
    /// Bad input or an action that is not allowed in the current state.
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    NotFound(String),
    /// Local playback controls are disabled while a listening group drives playback.
    #[error("{0}")]
    GroupMode(String),
    #[error("{0}")]
    Network(String),
    /// An external tool (yt-dlp, ffmpeg) failed or is missing.
    #[error("{0}")]
    Tool(String),
    #[error("Erreur de fichier : {0}")]
    Io(String),
    #[error("Erreur de base de données : {0}")]
    Db(String),
    #[error("{0}")]
    Internal(String),
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn invalid(msg: impl Into<String>) -> Self {
        Self::Invalid(msg.into())
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    pub fn group_mode() -> Self {
        Self::GroupMode("Tu es dans un groupe d'écoute : utilise les contrôles du groupe".into())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Db(e.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        Self::Network(e.to_string())
    }
}
