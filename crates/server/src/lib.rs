//! Listening-group server: accounts over HTTP, group sync over a WebSocket.
//! The wire contract lives in the `group-protocol` crate.

mod db;
pub mod group;
mod hub;

use std::fmt::Display;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::body::Bytes;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use group_protocol::{AuthResponse, ClientMessage, Credentials, ErrorResponse, READY_TIMEOUT_MS};
use serde::Deserialize;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use db::{AuthError, Db};
use hub::Hub;

/// Keeps idle NAT mappings open and detects peers that vanished without a close frame.
const PING_INTERVAL: Duration = Duration::from_secs(20);
const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_FRAME_BYTES: usize = 64 * 1024;

/// Startup failures (the server itself never stops on a request error).
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{var} {value:?}: {reason}")]
    Env {
        var: &'static str,
        value: String,
        reason: String,
    },
    #[error("cannot open database {}: {source}", path.display())]
    Database {
        path: PathBuf,
        source: rusqlite_migration::Error,
    },
    #[error("cannot listen on {addr}: {source}")]
    Listen {
        addr: SocketAddr,
        source: std::io::Error,
    },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Clone, Debug)]
pub struct Config {
    pub addr: SocketAddr,
    pub db_path: PathBuf,
    /// How long a group waits for slow members before starting a track anyway.
    pub ready_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            addr: SocketAddr::from(([127, 0, 0, 1], 8787)),
            db_path: PathBuf::from("./server.db"),
            ready_timeout: Duration::from_millis(READY_TIMEOUT_MS),
        }
    }
}

impl Config {
    /// Reads `MUSIC_SERVER_ADDR`, `MUSIC_SERVER_DB` and `MUSIC_SERVER_READY_TIMEOUT_MS`.
    pub fn from_env() -> Result<Self, Error> {
        let mut config = Self::default();
        if let Some(addr) = env("MUSIC_SERVER_ADDR")? {
            config.addr = addr;
        }
        if let Some(path) = env("MUSIC_SERVER_DB")? {
            config.db_path = path;
        }
        if let Some(ms) = env("MUSIC_SERVER_READY_TIMEOUT_MS")? {
            config.ready_timeout = Duration::from_millis(ms);
        }
        Ok(config)
    }
}

/// Parses an environment variable, `None` if unset.
fn env<T: FromStr>(var: &'static str) -> Result<Option<T>, Error>
where
    T::Err: Display,
{
    let Ok(value) = std::env::var(var) else {
        return Ok(None);
    };
    match value.parse() {
        Ok(parsed) => Ok(Some(parsed)),
        Err(e) => Err(Error::Env {
            var,
            value,
            reason: e.to_string(),
        }),
    }
}

#[derive(Clone)]
struct AppState {
    db: Db,
    hub: Arc<Hub>,
}

/// Serves until the listener fails. `config.addr` is ignored (the listener is already bound).
pub async fn serve(listener: TcpListener, config: Config) -> Result<(), Error> {
    let db = Db::open(&config.db_path).map_err(|source| Error::Database {
        path: config.db_path.clone(),
        source,
    })?;
    let state = AppState {
        db,
        hub: Hub::new(config.ready_timeout),
    };
    let app = Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/ws", get(ws))
        // Auth uses bearer tokens in bodies/queries, never cookies, so any origin is safe.
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);
    axum::serve(listener, app).await?;
    Ok(())
}

fn error(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(ErrorResponse {
            error: message.to_owned(),
        }),
    )
        .into_response()
}

fn internal(e: impl Display) -> Response {
    tracing::error!("internal error: {e}");
    error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Erreur interne du serveur",
    )
}

/// Parses and validates the body ourselves so every failure is a JSON [`ErrorResponse`].
fn credentials(body: &[u8]) -> Result<Credentials, &'static str> {
    let creds: Credentials = serde_json::from_slice(body).map_err(|_| "Requête invalide")?;
    let username_ok = (3..=32).contains(&creds.username.len())
        && creds
            .username
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-'));
    if !username_ok {
        return Err("Pseudo invalide (3 à 32 caractères : lettres, chiffres, _ . -)");
    }
    if !(6..=128).contains(&creds.password.chars().count()) {
        return Err("Mot de passe invalide (6 à 128 caractères)");
    }
    Ok(creds)
}

fn auth_error(e: AuthError) -> Response {
    match e {
        AuthError::UsernameTaken => error(StatusCode::CONFLICT, "Ce pseudo est déjà pris"),
        AuthError::BadCredentials => {
            error(StatusCode::UNAUTHORIZED, "Pseudo ou mot de passe incorrect")
        }
        e => internal(e),
    }
}

async fn register(State(state): State<AppState>, body: Bytes) -> Response {
    let creds = match credentials(&body) {
        Ok(c) => c,
        Err(message) => return error(StatusCode::BAD_REQUEST, message),
    };
    let username = creds.username.clone();
    match state.db.register(creds.username, creds.password).await {
        Ok(token) => Json(AuthResponse { username, token }).into_response(),
        Err(e) => auth_error(e),
    }
}

async fn login(State(state): State<AppState>, body: Bytes) -> Response {
    let creds = match credentials(&body) {
        Ok(c) => c,
        Err(message) => return error(StatusCode::BAD_REQUEST, message),
    };
    match state.db.login(creds.username, creds.password).await {
        Ok((username, token)) => Json(AuthResponse { username, token }).into_response(),
        Err(e) => auth_error(e),
    }
}

#[derive(Deserialize)]
struct WsQuery {
    token: Option<String>,
}

async fn ws(
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
    upgrade: WebSocketUpgrade,
) -> Response {
    let user = match query.token {
        Some(token) => state.db.user_for_token(token).await,
        None => Ok(None),
    };
    let username = match user {
        Ok(Some(username)) => username,
        Ok(None) => return error(StatusCode::UNAUTHORIZED, "Session invalide"),
        Err(e) => return internal(e),
    };
    upgrade
        .max_message_size(MAX_FRAME_BYTES)
        .on_upgrade(move |socket| run_socket(socket, state.hub, username))
}

/// One task per connection: reads client frames and writes queued server frames.
async fn run_socket(mut socket: WebSocket, hub: Arc<Hub>, username: String) {
    let (outbox, mut queued) = mpsc::unbounded_channel::<String>();
    tracing::debug!(%username, "websocket connected");
    let conn = hub.connect(username, outbox);
    let mut ping =
        tokio::time::interval_at(tokio::time::Instant::now() + PING_INTERVAL, PING_INTERVAL);
    let mut last_seen = Instant::now();
    loop {
        tokio::select! {
            frame = socket.recv() => {
                let Some(Ok(frame)) = frame else { break };
                last_seen = Instant::now();
                match frame {
                    Message::Text(text) => match serde_json::from_str::<ClientMessage>(&text) {
                        Ok(msg) => hub.handle(conn, msg),
                        Err(_) => hub.invalid_message(conn),
                    },
                    Message::Binary(_) => hub.invalid_message(conn),
                    Message::Close(_) => break,
                    Message::Ping(_) | Message::Pong(_) => {}
                }
            }
            Some(text) = queued.recv() => {
                if socket.send(Message::Text(text.into())).await.is_err() {
                    break;
                }
            }
            _ = ping.tick() => {
                if last_seen.elapsed() > IDLE_TIMEOUT
                    || socket.send(Message::Ping(Bytes::new())).await.is_err()
                {
                    break;
                }
            }
        }
    }
    hub.disconnect(conn);
    tracing::debug!(conn, "websocket closed");
}
