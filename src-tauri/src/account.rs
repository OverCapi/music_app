//! Account on the listening-group server, persisted as `account.json` in the
//! (profile) data dir.
//!
//! The token never leaves the backend: the frontend only sees [`Account`].

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use group_protocol::{AuthResponse, Credentials, ErrorResponse};
use serde::{Deserialize, Serialize};

use crate::storage::{load_json, save_json};

pub const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:8787";
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

/// What the frontend sees.
#[derive(Clone, Serialize)]
pub struct Account {
    pub server_url: String,
    /// `None` when logged out.
    pub username: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Stored {
    #[serde(default = "default_server_url")]
    server_url: String,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    token: Option<String>,
}

impl Default for Stored {
    fn default() -> Self {
        Self {
            server_url: default_server_url(),
            username: None,
            token: None,
        }
    }
}

fn default_server_url() -> String {
    DEFAULT_SERVER_URL.to_owned()
}

/// Everything the group client needs to open its WebSocket.
#[derive(Clone)]
pub struct Session {
    pub server_url: String,
    pub username: String,
    pub token: String,
}

pub struct AccountStore {
    file: PathBuf,
    stored: Mutex<Stored>,
    http: reqwest::Client,
}

impl AccountStore {
    pub fn load(file: PathBuf) -> Self {
        let mut stored: Stored = load_json(&file);
        // A hand-edited or corrupt URL falls back to the default rather than failing later.
        stored.server_url =
            normalize_server_url(&stored.server_url).unwrap_or_else(|_| default_server_url());
        if stored.username.is_none() || stored.token.is_none() {
            stored.username = None;
            stored.token = None;
        }
        let http = reqwest::Client::builder()
            .timeout(HTTP_TIMEOUT)
            .build()
            .unwrap_or_default();
        Self {
            file,
            stored: Mutex::new(stored),
            http,
        }
    }

    pub fn account(&self) -> Account {
        let s = self.stored.lock().unwrap();
        Account {
            server_url: s.server_url.clone(),
            username: s.username.clone(),
        }
    }

    pub fn session(&self) -> Option<Session> {
        let s = self.stored.lock().unwrap();
        Some(Session {
            server_url: s.server_url.clone(),
            username: s.username.clone()?,
            token: s.token.clone()?,
        })
    }

    /// Changing the server logs out: the token belongs to the old server.
    /// Returns whether the URL changed.
    pub fn set_server_url(&self, url: &str) -> Result<bool, String> {
        let url = normalize_server_url(url)?;
        let mut s = self.stored.lock().unwrap();
        if s.server_url == url {
            return Ok(false);
        }
        let mut next = s.clone();
        next.server_url = url;
        next.username = None;
        next.token = None;
        save_json(&self.file, &next)?;
        *s = next;
        Ok(true)
    }

    pub async fn register(&self, username: &str, password: &str) -> Result<Account, String> {
        self.authenticate("register", username, password).await
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<Account, String> {
        self.authenticate("login", username, password).await
    }

    pub fn logout(&self) -> Result<Account, String> {
        {
            let mut s = self.stored.lock().unwrap();
            let mut next = s.clone();
            next.username = None;
            next.token = None;
            save_json(&self.file, &next)?;
            *s = next;
        }
        Ok(self.account())
    }

    async fn authenticate(
        &self,
        endpoint: &str,
        username: &str,
        password: &str,
    ) -> Result<Account, String> {
        let server_url = self.stored.lock().unwrap().server_url.clone();
        let body = Credentials {
            username: username.trim().to_owned(),
            password: password.to_owned(),
        };
        let response = self
            .http
            .post(format!("{server_url}/{endpoint}"))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Serveur injoignable : {}", error_chain(&e)))?;
        let status = response.status();
        if !status.is_success() {
            let message = response
                .json::<ErrorResponse>()
                .await
                .map(|e| e.error)
                .unwrap_or_else(|_| format!("Erreur du serveur ({status})"));
            return Err(message);
        }
        let auth: AuthResponse = response
            .json()
            .await
            .map_err(|e| format!("Réponse du serveur invalide : {e}"))?;
        {
            let mut s = self.stored.lock().unwrap();
            // The URL may have changed while the request was in flight.
            if s.server_url != server_url {
                return Err("L'adresse du serveur a changé entre-temps".to_owned());
            }
            let mut next = s.clone();
            next.username = Some(auth.username);
            next.token = Some(auth.token);
            save_json(&self.file, &next)?;
            *s = next;
        }
        Ok(self.account())
    }
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
pub fn normalize_server_url(url: &str) -> Result<String, String> {
    let url = url.trim().trim_end_matches('/');
    let rest = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .ok_or("L'adresse du serveur doit commencer par http:// ou https://")?;
    let host = rest.split('/').next().unwrap_or_default();
    if host.is_empty() || rest.contains(['?', '#', ' ']) {
        return Err("Adresse du serveur invalide".to_owned());
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
}
