//! Listening-group client: one WebSocket to the group server (see the
//! `group-protocol` crate for the playback model), local downloads of the tracks
//! about to play, and the audio engine in group mode.
//!
//! The server is the source of truth. Every `State` it sends is reconciled:
//! - downloads: the current entry and the next two, into `<cache>/groups/<CODE>/`,
//!   with URLs built only from validated YouTube ids (never from member input);
//! - `Ready` for every downloaded track (again after each reconnection: the server
//!   forgets a member's ready set when its connection drops);
//! - the audio engine, re-commanded only when the entry or the playback changes,
//!   so unrelated updates (a member becoming ready…) never cause glitches.
//!
//! Submodules: `clock` (NTP-style offset, pure), `reconcile` (state → actions,
//! pure), `connection` (WebSocket + reconnect), `files` (group cache dir).
//! The UI gets the whole picture as [`GroupView`] in [`GroupChanged`] events.

mod clock;
pub mod commands;
mod connection;
mod files;
mod reconcile;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

use group_protocol::{ClientMessage, GroupState, ServerMessage, invite_link, is_valid_youtube_id};
use serde::Serialize;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use tokio::sync::{Semaphore, mpsc, oneshot};

use crate::account::{Session, websocket_url};
use crate::audio::{AudioHandle, GroupEndedCallback, GroupStart};
use crate::downloads::{self, DownloadStatus};
use crate::error::{AppError, AppResult};
use crate::library::Library;
use clock::Clock;
use connection::Outgoing;
use reconcile::Target;

/// How long `group_create` / `group_join` wait for the first `State`.
const JOIN_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_PARALLEL_DOWNLOADS: usize = 2;

const NOT_IN_GROUP: &str = "Tu n'es dans aucun groupe";
const DISCONNECTED: &str = "Connexion au serveur perdue, reconnexion en cours…";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum TrackStatus {
    Queued,
    Downloading,
    Ready,
    Error,
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
pub struct TrackDownload {
    pub status: TrackStatus,
    /// 0.0–1.0 while downloading, when the size is known.
    pub progress: Option<f64>,
    pub error: Option<String>,
}

impl TrackDownload {
    fn new(status: TrackStatus) -> Self {
        Self {
            status,
            progress: None,
            error: None,
        }
    }

    fn failed(error: String) -> Self {
        Self {
            error: Some(error),
            ..Self::new(TrackStatus::Error)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, specta::Type)]
pub struct GroupView {
    pub code: String,
    /// `musicapp://join/<code>`.
    pub link: String,
    /// My username.
    pub me: String,
    /// False while reconnecting.
    pub connected: bool,
    pub state: GroupState,
    /// By YouTube id.
    pub downloads: HashMap<String, TrackDownload>,
}

/// `None` when not in a group.
#[derive(Clone, Debug, Serialize, specta::Type, tauri_specta::Event)]
pub struct GroupChanged(pub Option<GroupView>);

/// Error to show to the user (server error message, failed download…).
#[derive(Clone, Debug, Serialize, specta::Type, tauri_specta::Event)]
pub struct GroupError {
    pub message: String,
}

pub enum Intent {
    Create,
    /// Uppercase code (already parsed with `parse_invite`).
    Join(String),
}

/// One group membership (from create/join until leave). Survives reconnections.
struct Membership {
    id: u64,
    me: String,
    /// `None` until the server has created the group.
    code: Option<String>,
    connected: bool,
    state: Option<GroupState>,
    downloads: HashMap<String, TrackDownload>,
    files: HashMap<String, PathBuf>,
    download_tasks: Vec<JoinHandle<()>>,
    permits: Arc<Semaphore>,
    out: mpsc::UnboundedSender<Outgoing>,
    clock: Clock,
    /// Resolves the pending create/join command on the first `State` (or failure).
    joined: Option<oneshot::Sender<AppResult<()>>>,
    in_audio_group: bool,
    applied: Option<Target>,
    /// Entry that was current at the last reconcile.
    current_entry: Option<u64>,
    /// YouTube ids announced with `Ready` on the current connection.
    announced: HashSet<String>,
}

impl Membership {
    fn view(&self) -> Option<GroupView> {
        let state = self.state.clone()?;
        Some(GroupView {
            link: invite_link(&state.code),
            code: state.code.clone(),
            me: self.me.clone(),
            connected: self.connected,
            state,
            downloads: self.downloads.clone(),
        })
    }

    fn send(&self, msg: ClientMessage) {
        let _ = self.out.send(Outgoing::Msg(msg));
    }
}

struct Inner {
    app: AppHandle,
    /// `<cache>/groups`
    dir: PathBuf,
    next_id: AtomicU64,
    current: Mutex<Option<Membership>>,
}

/// Managed state.
pub struct GroupClient {
    inner: Arc<Inner>,
}

impl GroupClient {
    /// `dir`: `<cache>/groups`, wiped here (leftovers of a previous run).
    pub fn new(app: AppHandle, dir: PathBuf) -> Self {
        files::remove_dir(&dir);
        Self {
            inner: Arc::new(Inner {
                app,
                dir,
                next_id: AtomicU64::new(1),
                current: Mutex::new(None),
            }),
        }
    }

    pub fn view(&self) -> Option<GroupView> {
        self.inner.current.lock().unwrap().as_ref()?.view()
    }

    /// Connects (leaving the current group, if any) and resolves once the first
    /// `State` arrives.
    pub async fn start(&self, intent: Intent, session: Session) -> AppResult<GroupView> {
        self.leave();
        let inner = &self.inner;
        let id = inner.next_id.fetch_add(1, Ordering::Relaxed);
        let (out, rx) = mpsc::unbounded_channel();
        let (joined_tx, joined_rx) = oneshot::channel();
        *inner.current.lock().unwrap() = Some(Membership {
            id,
            me: session.username.clone(),
            code: match intent {
                Intent::Create => None,
                Intent::Join(code) => Some(code),
            },
            connected: false,
            state: None,
            downloads: HashMap::new(),
            files: HashMap::new(),
            download_tasks: Vec::new(),
            permits: Arc::new(Semaphore::new(MAX_PARALLEL_DOWNLOADS)),
            out,
            clock: Clock::new(),
            joined: Some(joined_tx),
            in_audio_group: false,
            applied: None,
            current_entry: None,
            announced: HashSet::new(),
        });
        let url = websocket_url(&session.server_url, &session.token);
        tauri::async_runtime::spawn(connection::run(Arc::clone(inner), id, url, rx));

        match tokio::time::timeout(JOIN_TIMEOUT, joined_rx).await {
            Ok(Ok(Ok(()))) => self.view().ok_or_else(|| AppError::invalid(NOT_IN_GROUP)),
            Ok(Ok(Err(e))) => Err(e),
            Ok(Err(_)) => Err(AppError::invalid("Connexion au groupe annulée")),
            Err(_) => {
                inner.end(id, true);
                Err(AppError::Network("Le serveur ne répond pas".into()))
            }
        }
    }

    /// Leaves the group (no-op if not in one), restores local playback, deletes files.
    pub fn leave(&self) {
        let id = self.inner.current.lock().unwrap().as_ref().map(|m| m.id);
        if let Some(id) = id {
            self.inner.end(id, true);
        }
    }

    /// Called on app exit: like `leave`, synchronously removes the cache dir.
    pub fn shutdown(&self) {
        let membership = self.inner.current.lock().unwrap().take();
        if let Some(m) = membership {
            let _ = m.out.send(Outgoing::Close);
            for task in &m.download_tasks {
                task.abort();
            }
        }
        files::remove_dir(&self.inner.dir);
    }

    pub fn send(&self, msg: ClientMessage) -> AppResult<()> {
        let current = self.inner.current.lock().unwrap();
        let m = current
            .as_ref()
            .filter(|m| m.state.is_some())
            .ok_or_else(|| AppError::invalid(NOT_IN_GROUP))?;
        if !m.connected {
            return Err(AppError::Network(DISCONNECTED.into()));
        }
        m.out
            .send(Outgoing::Msg(msg))
            .map_err(|_| AppError::Network(DISCONNECTED.into()))
    }
}

impl Inner {
    fn audio(&self) -> tauri::State<'_, AudioHandle> {
        self.app.state::<AudioHandle>()
    }

    fn emit(&self, m: &Membership) {
        let _ = GroupChanged(m.view()).emit(&self.app);
    }

    fn emit_error(&self, error: &AppError) {
        let _ = GroupError {
            message: error.to_string(),
        }
        .emit(&self.app);
    }

    /// Runs `f` on the membership if it is still the one with this id.
    fn with<R>(&self, id: u64, f: impl FnOnce(&mut Membership) -> R) -> Option<R> {
        let mut current = self.current.lock().unwrap();
        current.as_mut().filter(|m| m.id == id).map(f)
    }

    /// Ends membership `id`. `close` asks the connection task to send `Leave`
    /// (otherwise it is already gone, e.g. after a fatal error).
    fn end(self: &Arc<Self>, id: u64, close: bool) -> Option<Membership> {
        let m = {
            let mut current = self.current.lock().unwrap();
            if current.as_ref().is_none_or(|m| m.id != id) {
                return None;
            }
            current.take()?
        };
        if close {
            let _ = m.out.send(Outgoing::Close);
        }
        for task in &m.download_tasks {
            // Drops yt-dlp, which kills it (kill_on_drop).
            task.abort();
        }
        if m.in_audio_group
            && let Err(e) = self.audio().leave_group()
        {
            tracing::warn!("group: leaving audio group mode failed: {e}");
        }
        if let Some(code) = m.code.clone() {
            // Kept if we rejoined the same group meanwhile.
            let inner = Arc::clone(self);
            files::remove_dir_twice(self.dir.join(&code), move || {
                inner
                    .current
                    .lock()
                    .unwrap()
                    .as_ref()
                    .is_some_and(|m| m.code.as_deref() == Some(code.as_str()))
            });
        }
        if m.state.is_some() {
            let _ = GroupChanged(None).emit(&self.app);
        }
        Some(m)
    }

    /// Fatal error from the connection: fails the pending create/join command, or
    /// reports it to the UI if we were already in the group.
    fn fail(self: &Arc<Self>, id: u64, error: AppError) {
        if let Some(mut m) = self.end(id, false) {
            match m.joined.take() {
                Some(joined) => {
                    let _ = joined.send(Err(error));
                }
                None => self.emit_error(&error),
            }
        }
    }

    /// While still waiting for the first `State`, a connection failure is reported
    /// right away instead of retrying in the background. Returns true if it failed.
    fn fail_if_joining(self: &Arc<Self>, id: u64, error: AppError) -> bool {
        let joining = self.with(id, |m| m.joined.is_some()).unwrap_or(false);
        if joining {
            self.fail(id, error);
        }
        joining
    }

    /// Returns the first message to send (create or re-join), or `None` if this
    /// membership is over.
    fn on_connected(&self, id: u64) -> Option<ClientMessage> {
        self.with(id, |m| {
            m.connected = true;
            m.announced.clear();
            // Re-command the engine on the next state: a track may have ended while
            // offline (its `Ended` was dropped), and the clock offset is re-measured.
            m.applied = None;
            self.emit(m);
            match &m.code {
                Some(code) => ClientMessage::Join { code: code.clone() },
                None => ClientMessage::Create,
            }
        })
    }

    fn on_disconnected(&self, id: u64) {
        self.with(id, |m| {
            if m.connected {
                m.connected = false;
                self.emit(m);
            }
        });
    }

    fn ping(&self, id: u64) -> Option<ClientMessage> {
        self.with(id, |m| ClientMessage::Ping {
            // Floored so the round trip measured at the pong is never negative.
            client_ms: m.clock.local_ms(Instant::now()).floor() as u64,
        })
    }

    /// Returns false when the connection should stop.
    fn on_message(self: &Arc<Self>, id: u64, msg: ServerMessage, awaiting_join: &mut bool) -> bool {
        match msg {
            ServerMessage::State { state } => {
                *awaiting_join = false;
                self.with(id, |m| {
                    m.code = Some(state.code.clone());
                    m.state = Some(state);
                    if let Some(joined) = m.joined.take() {
                        let _ = joined.send(Ok(()));
                    }
                    if !m.in_audio_group {
                        match self.audio().enter_group(self.ended_callback()) {
                            Ok(()) => m.in_audio_group = true,
                            Err(e) => tracing::warn!("group: entering audio group mode: {e}"),
                        }
                    }
                    self.reconcile(m);
                    self.emit(m);
                })
                .is_some()
            }
            ServerMessage::Pong {
                client_ms,
                server_ms,
            } => self
                .with(id, |m| {
                    let received = m.clock.local_ms(Instant::now());
                    m.clock
                        .add_sample(client_ms as f64, received, server_ms as f64);
                })
                .is_some(),
            ServerMessage::Error { message } => {
                if *awaiting_join {
                    // The create/join itself was refused: unknown code, or the group
                    // was deleted while we were offline (everyone dropped). Either
                    // way the membership is over.
                    self.fail(id, AppError::Invalid(message));
                    return false;
                }
                self.emit_error(&AppError::Invalid(message));
                true
            }
            ServerMessage::Left => {
                // We only send Leave when closing, so the server removed us.
                self.fail(id, AppError::invalid("Tu as quitté le groupe"));
                false
            }
        }
    }

    fn ended_callback(self: &Arc<Self>) -> GroupEndedCallback {
        let inner: Weak<Self> = Arc::downgrade(self);
        Arc::new(move |entry_id| {
            let inner = inner.clone();
            // Off the audio thread: it must never wait on our lock (we command the
            // engine while holding it).
            tauri::async_runtime::spawn(async move {
                let Some(inner) = inner.upgrade() else { return };
                let current = inner.current.lock().unwrap();
                if let Some(m) = current.as_ref() {
                    let still_current = m
                        .state
                        .as_ref()
                        .and_then(reconcile::current_entry)
                        .is_some_and(|e| e.entry_id == entry_id);
                    if still_current && m.connected {
                        m.send(ClientMessage::Ended { entry_id });
                    }
                }
            });
        })
    }

    /// Brings downloads, `Ready` and the audio engine in line with the state.
    fn reconcile(self: &Arc<Self>, m: &mut Membership) {
        let Some(state) = &m.state else { return };
        let current = reconcile::current_entry(state).map(|e| (e.entry_id, e.youtube_id.clone()));
        if current.as_ref().map(|c| c.0) != m.current_entry {
            m.current_entry = current.as_ref().map(|c| c.0);
            // A failed download gets another try each time its track becomes current.
            if let Some((_, youtube_id)) = &current
                && m.downloads
                    .get(youtube_id)
                    .is_some_and(|d| d.status == TrackStatus::Error)
                && is_valid_youtube_id(youtube_id)
            {
                m.downloads.remove(youtube_id);
            }
        }
        let Some(state) = &m.state else { return };
        for youtube_id in reconcile::prefetch(state) {
            self.ensure_download(m, &youtube_id);
        }
        let Some(state) = &m.state else { return };
        let actions = reconcile::plan(state, &m.files, &m.announced, m.applied.as_ref());

        // Sent after the Join on the same socket, so the server handles it once joined.
        if m.connected {
            for youtube_id in actions.ready {
                m.announced.insert(youtube_id.clone());
                m.send(ClientMessage::Ready { youtube_id });
            }
        }

        let Some(target) = actions.target.filter(|_| m.in_audio_group) else {
            return;
        };
        let audio = self.audio();
        let path = |p: &PathBuf| p.to_string_lossy().into_owned();
        let result = match &target {
            Target::Stopped => audio.group_stop(),
            Target::Play {
                entry_id,
                path: file,
                position,
                at_ms,
            } => audio.group_load(
                *entry_id,
                path(file),
                *position,
                GroupStart::At(m.clock.to_instant(*at_ms)),
            ),
            Target::Pause {
                entry_id,
                path: file,
                position,
            } => audio.group_load(*entry_id, path(file), *position, GroupStart::Paused),
        };
        if let Err(e) = result {
            tracing::warn!("group: audio command failed: {e}");
        }
        m.applied = Some(target);
    }

    fn ensure_download(self: &Arc<Self>, m: &mut Membership, youtube_id: &str) {
        if m.downloads.contains_key(youtube_id) {
            return;
        }
        if !is_valid_youtube_id(youtube_id) {
            m.downloads.insert(
                youtube_id.to_owned(),
                TrackDownload::failed("Identifiant YouTube invalide".into()),
            );
            return;
        }
        // Already in the library: no need to download it again.
        let in_library = self
            .app
            .try_state::<Library>()
            .and_then(|library| library.find_by_youtube_id(youtube_id).ok().flatten())
            .filter(|track| Path::new(&track.path).is_file());
        if let Some(track) = in_library {
            m.files
                .insert(youtube_id.to_owned(), PathBuf::from(track.path));
            m.downloads.insert(
                youtube_id.to_owned(),
                TrackDownload::new(TrackStatus::Ready),
            );
            return;
        }
        let Some(code) = m.code.clone() else { return };
        m.downloads.insert(
            youtube_id.to_owned(),
            TrackDownload::new(TrackStatus::Queued),
        );

        let inner = Arc::clone(self);
        let (id, youtube_id) = (m.id, youtube_id.to_owned());
        let dir = self.dir.join(code);
        let permits = Arc::clone(&m.permits);
        let task = tauri::async_runtime::spawn(async move {
            let Ok(_permit) = permits.acquire_owned().await else {
                return;
            };
            inner.set_download(
                id,
                &youtube_id,
                TrackDownload::new(TrackStatus::Downloading),
            );
            let result = match tokio::fs::create_dir_all(&dir).await {
                Ok(()) => {
                    downloads::download(&youtube_id, &dir, |status, progress| {
                        let progress = match status {
                            DownloadStatus::Downloading => progress,
                            // Extracting/tagging: the download itself is done.
                            DownloadStatus::Converting => Some(1.0),
                            _ => return,
                        };
                        let download = TrackDownload {
                            progress,
                            ..TrackDownload::new(TrackStatus::Downloading)
                        };
                        inner.set_download(id, &youtube_id, download);
                    })
                    .await
                }
                Err(e) => Err(e.into()),
            };
            inner.with(id, |m| {
                match result {
                    Ok(path) => {
                        m.files.insert(youtube_id.clone(), path);
                        m.downloads
                            .insert(youtube_id.clone(), TrackDownload::new(TrackStatus::Ready));
                        inner.reconcile(m);
                    }
                    Err(e) => {
                        m.downloads
                            .insert(youtube_id.clone(), TrackDownload::failed(e.to_string()));
                    }
                }
                inner.emit(m);
            });
        });
        m.download_tasks.retain(|t| !t.inner().is_finished());
        m.download_tasks.push(task);
    }

    fn set_download(&self, id: u64, youtube_id: &str, download: TrackDownload) {
        self.with(id, |m| {
            m.downloads.insert(youtube_id.to_owned(), download);
            self.emit(m);
        });
    }
}
