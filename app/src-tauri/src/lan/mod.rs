//! Library sync between devices on the same local network, device to device: no
//! third-party server (the group server is not involved).
//!
//! Each running app advertises itself over mDNS (`_musicapp._tcp`, TXT `id` and
//! `name`) and browses for the others ([`discovery`]); the list of peers is pushed
//! as [`LanPeers`]. It also runs a small HTTP server ([`server`]).
//!
//! A sync is requested by one device (A) and must be accepted on the other (B):
//! 1. A `POST /request`s B with the direction, its port, a 4-digit code shown on
//!    both screens (B's user checks the request really comes from A) and, if B is
//!    to pull A, a session token for A's server. The request waits for B's user
//!    ([`LanRequest`] event, [`Lan::answer`]), 60 s at most.
//! 2. If accepted, B answers with a session token of its own if A is to pull B.
//! 3. The device that *receives* pulls the other one's `GET /manifest` and
//!    `GET /file/<key>` with the token it was given ([`merge`] applies it), then
//!    `POST /done` ends the session.
//!    `Send`: B pulls A. `Receive`: A pulls B. `Both`: each pulls the other.
//!
//! A device is busy (refuses other syncs) while it requests, pulls, or is pulled
//! (a live session token).
//!
//! `mirror` (one direction only) makes the receiver a copy of the sender.
//! Progress is pushed as [`LanProgress`], the outcome as [`LanDone`].

pub mod commands;
mod discovery;
mod merge;
mod server;

use std::collections::{BTreeMap, HashMap};
use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use tokio::io::AsyncWriteExt;
use tokio::sync::oneshot;

use crate::db::Db;
use crate::downloads::Downloads;
use crate::error::{AppError, AppResult};
use crate::library::Library;
use crate::playlists::Playlists;
use merge::{Manifest, ManifestTrack};

const DEVICE_ID_KEY: &str = "lan_device_id";
const DEVICE_NAME_KEY: &str = "lan_device_name";
const MAX_NAME_LEN: usize = 50;
/// How long the user of the other device has to answer a request.
const ANSWER_TIMEOUT: Duration = Duration::from_secs(60);
/// A session token stops working after this long without being used.
const SESSION_IDLE: Duration = Duration::from_secs(300);
/// A peer that sends nothing for this long is gone. Above [`ANSWER_TIMEOUT`]:
/// `POST /request` stays silent while the other user decides.
const READ_TIMEOUT: Duration = Duration::from_secs(75);

/// Who sends and who receives, from the requesting device's point of view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// The requester sends its library to the other device.
    Send,
    /// The requester gets the other device's library.
    Receive,
    Both,
}

impl Direction {
    fn requester_pulls(self) -> bool {
        self != Self::Send
    }

    fn accepter_pulls(self) -> bool {
        self != Self::Receive
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
pub struct Peer {
    pub id: String,
    pub name: String,
    /// IPv4 addresses, those on a network of this device first.
    pub addrs: Vec<String>,
    pub port: u16,
}

/// A sync request waiting for the user's answer.
#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
pub struct IncomingRequest {
    pub name: String,
    pub direction: Direction,
    pub mirror: bool,
    /// Shown on the requesting device too.
    pub code: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, specta::Type)]
pub struct LanReport {
    /// Tracks that were not in the library before.
    pub new_tracks: u32,
    /// Audio files copied from the other device.
    pub files: u32,
    pub playlists: u32,
    /// Tracks of the other device that cannot be played here (no YouTube id, no file).
    pub skipped: u32,
    pub removed_tracks: u32,
    pub removed_playlists: u32,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
pub struct LanState {
    pub device_name: String,
    pub peers: Vec<Peer>,
    pub incoming: Option<IncomingRequest>,
}

#[derive(Clone, Debug, Serialize, specta::Type, tauri_specta::Event)]
pub struct LanPeers {
    pub peers: Vec<Peer>,
}

/// A request to answer, or `None` when it is over (answered, timed out, cancelled).
#[derive(Clone, Debug, Serialize, specta::Type, tauri_specta::Event)]
pub struct LanRequest {
    pub request: Option<IncomingRequest>,
}

#[derive(Clone, Debug, Serialize, specta::Type, tauri_specta::Event)]
pub struct LanProgress {
    pub step: String,
    pub done: u32,
    pub total: u32,
}

#[derive(Clone, Debug, Serialize, specta::Type, tauri_specta::Event)]
pub struct LanDone {
    /// Name of the other device.
    pub device: String,
    pub report: Option<LanReport>,
    pub error: Option<String>,
}

/// Body of `POST /request`.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct RequestBody {
    name: String,
    direction: Direction,
    mirror: bool,
    /// Port of the requester's server and the token to use on it (only when the
    /// other device pulls the requester).
    port: u16,
    token: Option<String>,
    code: String,
}

/// Answer to `POST /request`.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Answer {
    accepted: bool,
    token: Option<String>,
}

struct Pending {
    id: u64,
    request: IncomingRequest,
    tx: oneshot::Sender<bool>,
}

#[derive(Default)]
struct State {
    name: String,
    port: u16,
    /// By mDNS full name.
    peers: BTreeMap<String, Peer>,
    pending: Option<Pending>,
    /// Session tokens → last use.
    sessions: HashMap<String, Instant>,
    daemon: Option<mdns_sd::ServiceDaemon>,
}

struct Inner {
    db: Db,
    library: Library,
    playlists: Playlists,
    /// Where copied audio files go (the app music folder).
    music_dir: std::path::PathBuf,
    app: Option<AppHandle>,
    device_id: String,
    state: Mutex<State>,
    /// A request or a pull is in progress.
    busy: AtomicBool,
    next_request: AtomicU64,
    http: reqwest::Client,
}

/// Managed state.
#[derive(Clone)]
pub struct Lan {
    inner: Arc<Inner>,
}

/// Marks the device busy until dropped.
struct BusyGuard(Arc<Inner>);

impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.busy.store(false, Ordering::SeqCst);
    }
}

/// Clears the pending request when the HTTP request is dropped before it was answered.
struct PendingGuard<'a>(&'a Lan, u64);

impl Drop for PendingGuard<'_> {
    fn drop(&mut self) {
        let cleared = {
            let mut state = self.0.state();
            if state.pending.as_ref().is_some_and(|p| p.id == self.1) {
                state.pending = None;
                true
            } else {
                false
            }
        };
        if cleared {
            self.0.emit(LanRequest { request: None });
        }
    }
}

impl Lan {
    /// `app` is `None` in tests (no events emitted).
    pub fn new(
        db: Db,
        library: Library,
        playlists: Playlists,
        music_dir: std::path::PathBuf,
        app: Option<AppHandle>,
    ) -> AppResult<Self> {
        let device_id = match db.setting(DEVICE_ID_KEY)? {
            Some(id) => id,
            None => {
                let id = random_hex(8)?;
                db.set_setting(DEVICE_ID_KEY, &id)?;
                id
            }
        };
        let name = db
            .setting(DEVICE_NAME_KEY)?
            .and_then(|n| clean_name(&n))
            .unwrap_or_else(default_device_name);
        let http = reqwest::Client::builder()
            // Short: the next address of the peer is tried on failure.
            .connect_timeout(Duration::from_secs(3))
            .read_timeout(READ_TIMEOUT)
            .build()?;
        Ok(Self {
            inner: Arc::new(Inner {
                db,
                library,
                playlists,
                music_dir,
                app,
                device_id,
                state: Mutex::new(State {
                    name,
                    ..State::default()
                }),
                busy: AtomicBool::new(false),
                next_request: AtomicU64::new(1),
                http,
            }),
        })
    }

    pub fn device_state(&self) -> LanState {
        let state = self.state();
        LanState {
            device_name: state.name.clone(),
            peers: state.peers.values().cloned().collect(),
            incoming: state.pending.as_ref().map(|p| p.request.clone()),
        }
    }

    pub fn set_device_name(&self, name: &str) -> AppResult<String> {
        let name = clean_name(name).ok_or_else(|| {
            AppError::invalid(format!(
                "Le nom de l'appareil doit faire entre 1 et {MAX_NAME_LEN} caractères"
            ))
        })?;
        self.inner.db.set_setting(DEVICE_NAME_KEY, &name)?;
        self.state().name = name.clone();
        self.advertise();
        Ok(name)
    }

    /// The user's answer to the pending request. Does nothing if there is none.
    pub fn answer(&self, accept: bool) {
        let pending = self.state().pending.take();
        if let Some(pending) = pending {
            // The request handler has gone away if this fails: nothing to answer.
            let _ = pending.tx.send(accept);
            self.emit(LanRequest { request: None });
        }
    }

    /// Asks `peer_id` for a sync, `code` being shown on both devices. Returns whether
    /// the other user accepted; when it did, the pull (if this device receives) runs
    /// to the end before returning, with the outcome pushed as [`LanDone`].
    pub async fn request_sync(
        &self,
        peer_id: &str,
        direction: Direction,
        mirror: bool,
        code: &str,
    ) -> AppResult<bool> {
        if !valid_code(code) {
            return Err(AppError::invalid("Code de vérification invalide"));
        }
        let peer = self
            .state()
            .peers
            .values()
            .find(|p| p.id == peer_id)
            .cloned()
            .ok_or_else(|| AppError::not_found("Cet appareil n'est plus visible sur le réseau"))?;
        let _busy = self.begin()?;
        let (token, port, name) = {
            let mut state = self.state();
            // Access to this device only for a peer that pulls it.
            let token = direction
                .accepter_pulls()
                .then(|| new_session(&mut state))
                .transpose()?;
            (token, state.port, state.name.clone())
        };
        let body = RequestBody {
            name,
            direction,
            // The mirror applies to the receiver only: never when both receive.
            mirror: mirror && direction != Direction::Both,
            port,
            token,
            code: code.to_owned(),
        };
        let answer = self.ask(&peer, &body).await;
        if !matches!(answer, Ok((_, Answer { accepted: true, .. })))
            && let Some(token) = &body.token
        {
            // Nobody will pull this device.
            self.state().sessions.remove(token);
        }
        let (base, answer) = answer?;
        if !answer.accepted {
            return Ok(false);
        }
        if direction.requester_pulls() {
            let token = answer.token.ok_or_else(|| {
                AppError::Network(format!(
                    "{} n'a pas donné accès à sa bibliothèque",
                    peer.name
                ))
            })?;
            // Only a mirror on the receiving side: here, the requester.
            self.run_pull(
                &peer.name,
                &base,
                &token,
                body.mirror && direction == Direction::Receive,
            )
            .await;
        }
        Ok(true)
    }

    /// Posts `body` to the first address of `peer` that can be reached. Returns the
    /// base URL used and the answer.
    async fn ask(&self, peer: &Peer, body: &RequestBody) -> AppResult<(String, Answer)> {
        let mut last_error = None;
        for addr in &peer.addrs {
            let base = format!("http://{addr}:{}", peer.port);
            let sent = self
                .inner
                .http
                .post(format!("{base}/request"))
                // A bit more than the other user's time to answer.
                .timeout(ANSWER_TIMEOUT + Duration::from_secs(15))
                .json(body)
                .send()
                .await;
            let response = match sent {
                Ok(response) => response,
                // Another address may be on a network this device can reach.
                Err(e) if e.is_connect() => {
                    last_error = Some(e);
                    continue;
                }
                Err(e) => {
                    return Err(AppError::Network(format!(
                        "{} ne répond pas : {e}",
                        peer.name
                    )));
                }
            };
            if response.status() == reqwest::StatusCode::CONFLICT {
                return Err(AppError::invalid(format!("{} est occupé", peer.name)));
            }
            let answer: Answer = response.error_for_status()?.json().await?;
            return Ok((base, answer));
        }
        Err(AppError::Network(match last_error {
            Some(e) => format!("{} ne répond pas : {e}", peer.name),
            None => format!("{} n'a pas d'adresse IPv4", peer.name),
        }))
    }

    /// Handles `POST /request` from `ip`: waits for the user, then answers.
    async fn incoming(&self, body: RequestBody, ip: IpAddr) -> Result<Answer, IncomingError> {
        let name = clean_name(&body.name).unwrap_or_else(|| "Appareil inconnu".into());
        // A token exactly when this device is to pull the requester.
        if body.port == 0
            || !valid_code(&body.code)
            || body.token.is_some() != body.direction.accepter_pulls()
            || body.token.as_deref().is_some_and(|t| !valid_token(t))
        {
            return Err(IncomingError::Invalid);
        }
        let busy = self.begin().map_err(|_| IncomingError::Busy)?;
        let (tx, rx) = oneshot::channel();
        let id = self.inner.next_request.fetch_add(1, Ordering::SeqCst);
        let request = IncomingRequest {
            name: name.clone(),
            direction: body.direction,
            // A mirror only makes sense when this device receives alone.
            mirror: body.mirror && body.direction == Direction::Send,
            code: body.code.clone(),
        };
        self.state().pending = Some(Pending {
            id,
            request: request.clone(),
            tx,
        });
        self.emit(LanRequest {
            request: Some(request.clone()),
        });
        let guard = PendingGuard(self, id);
        let accepted = matches!(tokio::time::timeout(ANSWER_TIMEOUT, rx).await, Ok(Ok(true)));
        drop(guard);
        if !accepted {
            return Ok(Answer {
                accepted: false,
                token: None,
            });
        }
        let token = if body.direction.requester_pulls() {
            Some(new_session(&mut self.state()).map_err(|_| IncomingError::Invalid)?)
        } else {
            None
        };
        if let Some(peer_token) = body.token {
            let lan = self.clone();
            let base = format!("http://{ip}:{}", body.port);
            tauri::async_runtime::spawn(async move {
                let _busy = busy;
                lan.run_pull(&name, &base, &peer_token, request.mirror)
                    .await;
            });
        }
        Ok(Answer {
            accepted: true,
            token,
        })
    }

    /// Pulls the other device's library and pushes the outcome as [`LanDone`].
    async fn run_pull(&self, device: &str, base: &str, token: &str, mirror: bool) {
        let result = self.pull(base, token, mirror).await;
        // Frees the other device now rather than when the session expires.
        let done = self
            .inner
            .http
            .post(format!("{base}/done"))
            .bearer_auth(token)
            .timeout(Duration::from_secs(5))
            .send()
            .await;
        if let Err(e) = done {
            tracing::warn!("cannot end the LAN session: {e}");
        }
        let (report, error) = match result {
            Ok(report) => (Some(report), None),
            Err(e) => (None, Some(e.to_string())),
        };
        self.emit(LanDone {
            device: device.to_owned(),
            report,
            error,
        });
    }

    async fn pull(&self, base: &str, token: &str, mirror: bool) -> AppResult<LanReport> {
        self.progress("Lecture de la bibliothèque", 0, 0);
        let manifest: Manifest = self
            .inner
            .http
            .get(format!("{base}/manifest"))
            .bearer_auth(token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let needed = merge::files_needed(&self.inner.library, &manifest)?;
        let mut files = HashMap::new();
        let mut errors = Vec::new();
        for (done, track) in needed.iter().enumerate() {
            self.progress(&track.title, done as u32, needed.len() as u32);
            match self.download_file(base, token, track).await {
                Ok(path) => {
                    files.insert(track.key, path);
                }
                Err(e) => errors.push(format!("{} : {e}", track.title)),
            }
        }

        self.progress("Fusion", 0, 0);
        let app = self.inner.app.clone();
        let trash = move |path: &str| {
            if let Some(downloads) = app.as_ref().and_then(|a| a.try_state::<Downloads>())
                && let Err(e) = downloads.trash_if_owned(path)
            {
                tracing::warn!("cannot trash {path}: {e}");
            }
        };
        let result = merge::apply(
            &self.inner.library,
            &self.inner.playlists,
            &manifest,
            &mut files,
            mirror,
            &trash,
        );
        // Copies the merge did not keep (similar track found, unreadable file, error).
        for path in files.values() {
            let _ = tokio::fs::remove_file(path).await;
        }
        let mut report = result?;
        report.errors.extend(errors);
        Ok(report)
    }

    /// Copies the file of `track` into the music folder; returns its path.
    async fn download_file(
        &self,
        base: &str,
        token: &str,
        track: &ManifestTrack,
    ) -> AppResult<String> {
        let ext = track
            .ext
            .as_deref()
            .filter(|e| e.bytes().all(|b| b.is_ascii_alphanumeric()))
            .ok_or_else(|| AppError::invalid("Format de fichier inconnu"))?;
        // Own name: nothing from the peer is used as a path, only sanitized text.
        let stem: String = track
            .title
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == ' ' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .take(60)
            .collect();
        let path =
            self.inner
                .music_dir
                .join(format!("{} (lan {}).{ext}", stem.trim(), random_hex(3)?));
        let part = path.with_extension(format!("{ext}.part"));
        tokio::fs::create_dir_all(&self.inner.music_dir).await?;

        let mut response = self
            .inner
            .http
            .get(format!("{base}/file/{}", track.key))
            .bearer_auth(token)
            .send()
            .await?
            .error_for_status()?;
        let copy = async {
            let mut file = tokio::fs::File::create(&part).await?;
            while let Some(chunk) = response.chunk().await? {
                file.write_all(&chunk).await?;
            }
            file.flush().await?;
            tokio::fs::rename(&part, &path).await?;
            Ok::<_, AppError>(())
        }
        .await;
        if let Err(e) = copy {
            let _ = tokio::fs::remove_file(&part).await;
            return Err(e);
        }
        Ok(path.to_string_lossy().into_owned())
    }

    /// Whether `token` is a live session token (and refreshes it).
    fn check_token(&self, token: &str) -> bool {
        let mut state = self.state();
        match state.sessions.get_mut(token) {
            Some(last_used) if last_used.elapsed() < SESSION_IDLE => {
                *last_used = Instant::now();
                true
            }
            _ => false,
        }
    }

    /// One sync at a time per device. A device still pulling this one (live session)
    /// counts as a sync.
    fn begin(&self) -> AppResult<BusyGuard> {
        let served = {
            let mut state = self.state();
            drop_expired(&mut state);
            !state.sessions.is_empty()
        };
        if served || self.inner.busy.swap(true, Ordering::SeqCst) {
            return Err(AppError::invalid("Une synchronisation est déjà en cours"));
        }
        Ok(BusyGuard(Arc::clone(&self.inner)))
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.inner.state.lock().unwrap()
    }

    fn progress(&self, step: &str, done: u32, total: u32) {
        self.emit(LanProgress {
            step: step.to_owned(),
            done,
            total,
        });
    }

    fn emit<E: Event + Serialize + Clone>(&self, event: E) {
        if let Some(app) = &self.inner.app
            && let Err(e) = event.emit(app)
        {
            tracing::warn!("emit LAN event failed: {e}");
        }
    }
}

enum IncomingError {
    Invalid,
    Busy,
}

fn drop_expired(state: &mut State) {
    state
        .sessions
        .retain(|_, last_used| last_used.elapsed() < SESSION_IDLE);
}

/// Creates a session token and drops the expired ones.
fn new_session(state: &mut State) -> AppResult<String> {
    drop_expired(state);
    let token = random_hex(32)?;
    state.sessions.insert(token.clone(), Instant::now());
    Ok(token)
}

fn valid_token(token: &str) -> bool {
    (16..=128).contains(&token.len()) && token.bytes().all(|b| b.is_ascii_alphanumeric())
}

fn valid_code(code: &str) -> bool {
    code.len() == 4 && code.bytes().all(|b| b.is_ascii_digit())
}

/// `bytes` random bytes from the OS, hex-encoded.
fn random_hex(bytes: usize) -> AppResult<String> {
    let mut buf = vec![0u8; bytes];
    getrandom::fill(&mut buf).map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
}

/// Device name from a user or a peer: no control characters, 1–50 chars.
fn clean_name(name: &str) -> Option<String> {
    let name: String = name
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .chars()
        .take(MAX_NAME_LEN)
        .collect();
    (!name.is_empty()).then_some(name)
}

/// The computer's name, else a generic one.
fn default_device_name() -> String {
    let command = |program: &str, args: &[&str]| {
        std::process::Command::new(program)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
    };
    std::env::var("COMPUTERNAME")
        .ok()
        .or_else(|| command("scutil", &["--get", "ComputerName"]))
        .or_else(|| command("hostname", &[]))
        .and_then(|n| clean_name(&n))
        .unwrap_or_else(|| "Mon appareil".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lan() -> Lan {
        let db = Db::in_memory();
        Lan::new(
            db.clone(),
            Library::new(db.clone(), None),
            Playlists::new(db, None),
            std::env::temp_dir(),
            None,
        )
        .unwrap()
    }

    #[test]
    fn names_are_cleaned() {
        assert_eq!(
            clean_name("  Mac\u{7} de Louis "),
            Some("Mac de Louis".into())
        );
        assert_eq!(clean_name(" \n "), None);
        assert_eq!(
            clean_name(&"x".repeat(80)).unwrap().chars().count(),
            MAX_NAME_LEN
        );
    }

    #[test]
    fn device_identity_is_stable_and_name_is_saved() {
        let lan = lan();
        assert_eq!(lan.inner.device_id.len(), 16);
        assert!(lan.set_device_name("").is_err());
        assert_eq!(lan.set_device_name(" Salon ").unwrap(), "Salon");
        assert_eq!(lan.device_state().device_name, "Salon");
        assert_eq!(
            lan.inner.db.setting(DEVICE_NAME_KEY).unwrap().as_deref(),
            Some("Salon")
        );
    }

    #[test]
    fn session_tokens() {
        let lan = lan();
        let token = new_session(&mut lan.state()).unwrap();
        assert!(valid_token(&token));
        assert!(lan.check_token(&token));
        assert!(!lan.check_token("nope"));
        // An expired token is refused.
        lan.state().sessions.insert(
            "old".into(),
            Instant::now() - SESSION_IDLE - Duration::from_secs(1),
        );
        assert!(!lan.check_token("old"));
    }

    #[test]
    fn one_sync_at_a_time() {
        let lan = lan();
        let guard = lan.begin().unwrap();
        assert!(lan.begin().is_err());
        drop(guard);
        assert!(lan.begin().is_ok());
    }

    /// A device with its own database and a running server, plus handles on its data.
    async fn node(tracks: &[&str], playlist: &str) -> (Lan, Library, Playlists) {
        let db = Db::in_memory();
        let (library, playlists) = (
            Library::new(db.clone(), None),
            Playlists::new(db.clone(), None),
        );
        let lan = Lan::new(
            db,
            library.clone(),
            playlists.clone(),
            std::env::temp_dir(),
            None,
        )
        .unwrap();
        let remote: Vec<_> = tracks
            .iter()
            .map(|id| crate::library::RemoteTrack {
                title: format!("Titre {id}"),
                artist: Some("Artiste".into()),
                album: None,
                duration: 100.0,
                youtube_id: Some((*id).to_owned()),
            })
            .collect();
        let ids = library.add_remote(&remote).unwrap();
        if !ids.is_empty() {
            let list = playlists.create(playlist).unwrap().id;
            playlists.add_tracks(list, &ids).unwrap();
        }
        let port = lan.serve().await.unwrap();
        lan.state().port = port;
        (lan, library, playlists)
    }

    fn see(lan: &Lan, other: &Lan) {
        let (port, name) = {
            let state = other.state();
            (state.port, state.name.clone())
        };
        lan.state().peers.insert(
            "other".into(),
            Peer {
                id: other.inner.device_id.clone(),
                name,
                // An unreachable address first: the next one is tried.
                addrs: vec!["127.0.0.2".into(), "127.0.0.1".into()],
                port,
            },
        );
    }

    /// Accepts (or refuses) the next request received by `lan`.
    fn answer_next(lan: &Lan, accept: bool) {
        let lan = lan.clone();
        tokio::spawn(async move {
            while lan.device_state().incoming.is_none() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            lan.answer(accept);
        });
    }

    async fn until(mut done: impl FnMut() -> bool) {
        for _ in 0..400 {
            if done() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        panic!("timed out");
    }

    #[tokio::test]
    async fn syncs_between_two_devices() {
        let (a, a_library, a_playlists) = node(&["aaaaaaaaaaa"], "Rock").await;
        let (b, b_library, b_playlists) = node(&["bbbbbbbbbbb", "ccccccccccc"], "rock").await;
        see(&a, &b);
        see(&b, &a);

        // Refused: nothing changes.
        answer_next(&b, false);
        let id = b.inner.device_id.clone();
        assert!(
            !a.request_sync(&id, Direction::Receive, false, "1234")
                .await
                .unwrap()
        );
        assert_eq!(a_library.tracks().unwrap().len(), 1);

        // A receives B's library.
        answer_next(&b, true);
        assert!(
            a.request_sync(&id, Direction::Receive, false, "1234")
                .await
                .unwrap()
        );
        assert_eq!(a_library.tracks().unwrap().len(), 3);
        assert_eq!(b_library.tracks().unwrap().len(), 2);
        // Same playlist name (other case): merged, not duplicated.
        let lists = a_playlists.all().unwrap();
        assert_eq!(lists.len(), 1);
        assert_eq!(lists[0].track_ids.len(), 3);

        // A sends to B, as a mirror: B becomes a copy of A.
        answer_next(&b, true);
        assert!(
            a.request_sync(&id, Direction::Send, true, "1234")
                .await
                .unwrap()
        );
        until(|| b_library.tracks().unwrap().len() == 3 && !b.inner.busy.load(Ordering::SeqCst))
            .await;
        assert_eq!(b_playlists.all().unwrap()[0].track_ids.len(), 3);
        assert!(!a.inner.busy.load(Ordering::SeqCst));
    }

    /// Needs a network interface with multicast: `cargo test -p music_app -- --ignored`.
    #[tokio::test]
    #[ignore]
    async fn devices_find_each_other() {
        let (a, _, _) = node(&[], "").await;
        let (b, _, _) = node(&[], "").await;
        a.set_device_name("Salon").unwrap();
        a.start().await.unwrap();
        b.start().await.unwrap();
        until(|| {
            b.device_state().peers.iter().any(|p| p.name == "Salon")
                && a.device_state().peers.len() == 1
        })
        .await;
        let peer = b.device_state().peers.remove(0);
        assert_eq!(peer.id, a.inner.device_id);
        assert_eq!(peer.port, a.state().port);
        a.shutdown();
        b.shutdown();
    }

    #[tokio::test]
    async fn files_need_a_valid_session() {
        let (a, _, _) = node(&["aaaaaaaaaaa"], "L").await;
        let port = a.state().port;
        let client = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{port}/manifest");
        assert_eq!(client.get(&url).send().await.unwrap().status(), 401);
        let bad = client.get(&url).bearer_auth("nope").send().await.unwrap();
        assert_eq!(bad.status(), 401);
        let token = new_session(&mut a.state()).unwrap();
        let ok = client.get(&url).bearer_auth(&token).send().await.unwrap();
        assert_eq!(ok.status(), 200);
        let file = format!("http://127.0.0.1:{port}/file/1");
        // A track without a file is not served.
        let missing = client.get(&file).bearer_auth(&token).send().await.unwrap();
        assert_eq!(missing.status(), 404);
    }

    #[tokio::test]
    async fn request_is_answered_or_times_out() {
        let lan = lan();
        let body = RequestBody {
            name: "Autre".into(),
            direction: Direction::Receive,
            mirror: false,
            port: 1234,
            // `Receive`: this device does not pull the requester.
            token: None,
            code: "1234".into(),
        };
        let ip: IpAddr = "127.0.0.1".parse().unwrap();

        let answering = lan.clone();
        let waiting = tokio::spawn({
            let (lan, body) = (lan.clone(), body.clone());
            async move { lan.incoming(body, ip).await }
        });
        // Wait for the request to be pending, then refuse.
        while answering.device_state().incoming.is_none() {
            tokio::task::yield_now().await;
        }
        assert_eq!(answering.device_state().incoming.unwrap().name, "Autre");
        answering.answer(false);
        assert!(!waiting.await.unwrap().ok().unwrap().accepted);
        assert!(lan.device_state().incoming.is_none());

        // Accepted: a session token for the requester, and nothing to pull (`Receive`).
        let waiting = tokio::spawn({
            let (lan, body) = (lan.clone(), body.clone());
            async move { lan.incoming(body, ip).await }
        });
        while lan.device_state().incoming.is_none() {
            tokio::task::yield_now().await;
        }
        lan.answer(true);
        let answer = waiting.await.unwrap().ok().unwrap();
        assert!(answer.accepted);
        let token = answer.token.unwrap();
        assert!(lan.check_token(&token));
        // Busy while the requester pulls, free once it is done.
        assert!(lan.begin().is_err());
        lan.state().sessions.remove(&token);
        assert!(lan.begin().is_ok());

        // A token that does not match the direction is refused.
        let wrong = RequestBody {
            token: Some("a".repeat(32)),
            ..body
        };
        assert!(matches!(
            lan.incoming(wrong, ip).await,
            Err(IncomingError::Invalid)
        ));
    }
}
