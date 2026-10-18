//! Account on the listening-group server (REST login/register with reqwest +
//! rustls), persisted in the `settings` table (`server_url`, `username`, `token`).
//!
//! The token never leaves the backend: the frontend only sees [`Account`].

pub mod commands;

use std::time::Duration;

use group_protocol::{AuthResponse, Credentials, ErrorResponse};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use crate::db::Db;
use crate::error::{AppError, AppResult};

pub const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:8787";
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

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
    db: Db,
    http: reqwest::Client,
}

impl AccountStore {
    pub fn new(db: Db) -> Self {
        let http = reqwest::Client::builder()
            .timeout(HTTP_TIMEOUT)
            .build()
            .unwrap_or_default();
        Self { db, http }
    }

    pub fn account(&self) -> AppResult<Account> {
        let conn = self.db.conn();
        Ok(Account {
            server_url: server_url(&conn)?,
            username: stored_session(&conn)?.map(|s| s.username),
        })
    }

    /// `None` when logged out.
    pub fn session(&self) -> AppResult<Option<Session>> {
        stored_session(&self.db.conn())
    }

    /// Normalizes and saves the URL. Changing server logs out; returns true if it did.
    pub fn set_server_url(&self, url: &str) -> AppResult<bool> {
        let url = normalize_server_url(url)?;
        let mut conn = self.db.conn();
        if server_url(&conn)? == url {
            return Ok(false);
        }
        // The token belongs to the old server.
        let tx = conn.transaction()?;
        set(&tx, "server_url", &url)?;
        clear_credentials(&tx)?;
        tx.commit()?;
        Ok(true)
    }

    pub async fn register(&self, username: &str, password: &str) -> AppResult<Account> {
        self.authenticate("register", username, password).await
    }

    pub async fn login(&self, username: &str, password: &str) -> AppResult<Account> {
        self.authenticate("login", username, password).await
    }

    pub fn logout(&self) -> AppResult<Account> {
        clear_credentials(&self.db.conn())?;
        self.account()
    }

    async fn authenticate(
        &self,
        endpoint: &str,
        username: &str,
        password: &str,
    ) -> AppResult<Account> {
        let server = server_url(&self.db.conn())?;
        let body = Credentials {
            username: username.trim().to_owned(),
            password: password.to_owned(),
        };
        let response = self
            .http
            .post(format!("{server}/{endpoint}"))
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Network(format!("Serveur injoignable : {}", error_chain(&e))))?;
        let status = response.status();
        if !status.is_success() {
            let message = response
                .json::<ErrorResponse>()
                .await
                .map(|e| e.error)
                .unwrap_or_else(|_| format!("Erreur du serveur ({status})"));
            return Err(AppError::invalid(message));
        }
        let auth: AuthResponse = response
            .json()
            .await
            .map_err(|e| AppError::Network(format!("Réponse du serveur invalide : {e}")))?;
        {
            let mut conn = self.db.conn();
            // The URL may have changed while the request was in flight.
            if server_url(&conn)? != server {
                return Err(AppError::invalid(
                    "L'adresse du serveur a changé entre-temps",
                ));
            }
            let tx = conn.transaction()?;
            set(&tx, "username", &auth.username)?;
            set(&tx, "token", &auth.token)?;
            tx.commit()?;
        }
        self.account()
    }
}

fn stored_session(conn: &Connection) -> AppResult<Option<Session>> {
    let (Some(username), Some(token)) = (get(conn, "username")?, get(conn, "token")?) else {
        return Ok(None);
    };
    Ok(Some(Session {
        server_url: server_url(conn)?,
        username,
        token,
    }))
}

fn get(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
            row.get(0)
        })
        .optional()?)
}

fn set(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

fn clear_credentials(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "DELETE FROM settings WHERE key IN ('username', 'token')",
        [],
    )?;
    Ok(())
}

/// A hand-edited or corrupt URL falls back to the default rather than failing later.
fn server_url(conn: &Connection) -> AppResult<String> {
    Ok(get(conn, "server_url")?
        .and_then(|url| normalize_server_url(&url).ok())
        .unwrap_or_else(|| DEFAULT_SERVER_URL.to_owned()))
}

/// reqwest's top-level message ("error sending request") hides the useful part.
fn error_chain(e: &dyn std::error::Error) -> String {
    let mut message = e.to_string();
    let mut source = e.source();
    while let Some(cause) = source {
        message = format!("{message} ({cause})");
        source = cause.source();
    }
    message
}

/// Accepts `http(s)://host[:port][/path]`, without the trailing slash.
pub fn normalize_server_url(url: &str) -> AppResult<String> {
    let url = url.trim().trim_end_matches('/');
    let rest = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .ok_or_else(|| {
            AppError::invalid("L'adresse du serveur doit commencer par http:// ou https://")
        })?;
    let host = rest.split('/').next().unwrap_or_default();
    if host.is_empty() || rest.contains(['?', '#', ' ']) {
        return Err(AppError::invalid("Adresse du serveur invalide"));
    }
    Ok(url.to_owned())
}

/// `http://host` → `ws://host/ws?token=…` (`https` → `wss`).
pub fn websocket_url(server_url: &str, token: &str) -> String {
    let base = match server_url.strip_prefix("https://") {
        Some(rest) => format!("wss://{rest}"),
        None => format!("ws://{}", server_url.trim_start_matches("http://")),
    };
    format!("{base}/ws?token={}", percent_encode(token))
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_urls() {
        assert_eq!(
            normalize_server_url(" http://127.0.0.1:8787/ ").unwrap(),
            "http://127.0.0.1:8787"
        );
        assert_eq!(
            normalize_server_url("https://example.org/music").unwrap(),
            "https://example.org/music"
        );
        assert!(normalize_server_url("ftp://example.org").is_err());
        assert!(normalize_server_url("example.org").is_err());
        assert!(normalize_server_url("http://").is_err());
        assert!(normalize_server_url("http://a.b/?x=1").is_err());
    }

    #[test]
    fn websocket_urls() {
        assert_eq!(
            websocket_url("http://127.0.0.1:8787", "abc"),
            "ws://127.0.0.1:8787/ws?token=abc"
        );
        assert_eq!(
            websocket_url("https://example.org/music", "a+b/c="),
            "wss://example.org/music/ws?token=a%2Bb%2Fc%3D"
        );
    }

    #[test]
    fn settings_round_trip() {
        let store = AccountStore::new(Db::in_memory());
        assert_eq!(
            store.account().unwrap(),
            Account {
                server_url: DEFAULT_SERVER_URL.into(),
                username: None
            }
        );
        {
            let conn = store.db.conn();
            set(&conn, "username", "alice").unwrap();
            set(&conn, "token", "t").unwrap();
        }
        assert_eq!(store.session().unwrap().unwrap().token, "t");
        // Same server: stays logged in. Another one: logged out.
        assert!(!store.set_server_url("http://127.0.0.1:8787/").unwrap());
        assert!(store.set_server_url("https://example.org").unwrap());
        assert!(store.session().unwrap().is_none());
        assert_eq!(store.account().unwrap().server_url, "https://example.org");
    }
}
