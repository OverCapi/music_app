//! Account on the listening-group server (REST login/register with reqwest +
//! rustls), persisted in the `settings` table (`server_url`, `username`, `token`).
//!
//! The token never leaves the backend: the frontend only sees [`Account`].

pub mod commands;

use serde::Serialize;

use crate::db::Db;
use crate::error::AppResult;

pub const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:8787";

/// What the frontend sees.
#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
pub struct Account {
    pub server_url: String,
    /// `None` when logged out.
    pub username: Option<String>,
}

/// Everything the group client needs to open its WebSocket.
#[derive(Clone, Debug)]
pub struct Session {
    pub server_url: String,
    pub username: String,
    pub token: String,
}

/// Managed state.
pub struct AccountStore {
    #[allow(dead_code)]
    db: Db,
}

impl AccountStore {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub fn account(&self) -> AppResult<Account> {
        todo!()
    }

    /// `None` when logged out.
    pub fn session(&self) -> AppResult<Option<Session>> {
        todo!()
    }

    /// Normalizes and saves the URL. Changing server logs out; returns true if it did.
    pub fn set_server_url(&self, _url: &str) -> AppResult<bool> {
        todo!()
    }

    pub async fn register(&self, _username: &str, _password: &str) -> AppResult<Account> {
        todo!()
    }

    pub async fn login(&self, _username: &str, _password: &str) -> AppResult<Account> {
        todo!()
    }

    pub fn logout(&self) -> AppResult<Account> {
        todo!()
    }
}
