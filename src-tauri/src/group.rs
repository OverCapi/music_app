//! Listening-group client: one WebSocket to the group server (see the
//! `group-protocol` crate for the playback model), local downloads of the tracks
//! about to play, and the audio engine in group mode.
//!
//! The server is the source of truth. Every `State` it sends is reconciled here:
//! - downloads: the current entry and the next two, into `<cache>/groups/<CODE>/`,
//!   with URLs built only from validated YouTube ids (never from member input);
//! - `Ready` for every downloaded track (again after each reconnection: the server
//!   forgets a member's ready set when its connection drops);
//! - the audio engine, re-commanded only when the entry or the playback changes,
//!   so unrelated updates (a member becoming ready…) never cause glitches.
//!
//! The UI gets the whole picture as [`GroupView`] in `group-state` events.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use group_protocol::{
    invite_link, is_valid_youtube_id, ClientMessage, GroupState, Playback, QueueEntry,
    ServerMessage,
};
use serde::Serialize;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{mpsc, oneshot, Semaphore};
use tokio_tungstenite::tungstenite::{self, Message};

use crate::account::{self, websocket_url};
use crate::audio::{AudioHandle, GroupEndedCallback, GroupStart};
use crate::downloads::{download_with_retry, DownloadStatus};
use crate::library::Library;

pub const STATE_EVENT: &str = "group-state";
pub const ERROR_EVENT: &str = "group-error";

/// How long `group_create` / `group_join` wait for the first `State`.
const JOIN_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Reconnection delays; the last one repeats.
const BACKOFF_SECS: [u64; 4] = [1, 2, 5, 10];
/// Clock sync: a quick burst at connection, then a periodic refresh.
const INITIAL_PINGS: u32 = 5;
const INITIAL_PING_INTERVAL: Duration = Duration::from_millis(200);
const PING_INTERVAL: Duration = Duration::from_secs(30);
/// Nothing received for this long (not even a pong) → the connection is dead.
const SILENCE_TIMEOUT: Duration = Duration::from_secs(75);
const MAX_CLOCK_SAMPLES: usize = 15;
/// Current entry + the next two.
const PREFETCH: usize = 3;
const MAX_PARALLEL_DOWNLOADS: usize = 2;
/// Left to yt-dlp's children to die before the temp folder is removed again.
const CLEANUP_DELAY: Duration = Duration::from_secs(3);

const NOT_IN_GROUP: &str = "Tu n'es dans aucun groupe";
const DISCONNECTED: &str = "Connexion au serveur perdue, reconnexion en cours…";

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TrackStatus {
    Queued,
    Downloading,
    Ready,
    Error,
}

#[derive(Clone, Debug, Serialize)]
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
}

#[derive(Clone, Serialize)]
pub struct GroupView {
    pub code: String,
    pub link: String,
    pub me: String,
    pub connected: bool,
    pub state: GroupState,
    /// By YouTube id.
    pub downloads: HashMap<String, TrackDownload>,
}

pub enum Intent {
    Create,
    /// Uppercase code (already parsed with `parse_invite`).
    Join(String),
}

enum Outgoing {
    Msg(ClientMessage),
    /// Leave the group and close the connection.
    Close,
}

/// Maps server time to local time from Ping/Pong round trips.
///
/// Local times are milliseconds since `base` (monotonic, so wall-clock jumps on
/// this machine don't matter); `offset` = server ms − local ms.
struct Clock {
    base: Instant,
    samples: VecDeque<f64>,
    /// Used until the first pong: assumes both wall clocks agree.
    fallback: f64,
}

impl Clock {
    fn new() -> Self {
        let base = Instant::now();
        let wall = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64()
            * 1000.0;
        Self {
            base,
            samples: VecDeque::new(),
            fallback: wall,
        }
    }

    fn local_ms(&self, at: Instant) -> f64 {
        at.saturating_duration_since(self.base).as_secs_f64() * 1000.0
    }

    /// A pong for a ping sent at local `sent_ms`, received at local `received_ms`.
    /// The server read its clock roughly halfway through the round trip.
    fn add_sample(&mut self, sent_ms: f64, received_ms: f64, server_ms: f64) {
        if received_ms < sent_ms {
            return;
        }
        self.samples
            .push_back(server_ms - (sent_ms + received_ms) / 2.0);
        if self.samples.len() > MAX_CLOCK_SAMPLES {
            self.samples.pop_front();
        }
    }

    fn offset(&self) -> f64 {
        median(self.samples.iter().copied()).unwrap_or(self.fallback)
    }

    /// The local instant at which the server clock reads `server_ms`.
    fn to_instant(&self, server_ms: u64) -> Instant {
        let local_ms = server_ms as f64 - self.offset();
        if local_ms >= 0.0 {
            self.base + Duration::from_secs_f64(local_ms / 1000.0)
        } else {
            self.base
                .checked_sub(Duration::from_secs_f64(-local_ms / 1000.0))
                .unwrap_or(self.base)
        }
    }
}

/// Median, robust to the occasional slow round trip.
fn median(values: impl Iterator<Item = f64>) -> Option<f64> {
    let mut values: Vec<f64> = values.filter(|v| v.is_finite()).collect();
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let mid = values.len() / 2;
    Some(if values.len().is_multiple_of(2) {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    })
}

/// What the audio engine should be doing for a given group state. The engine is
/// only re-commanded when this changes.
#[derive(Clone, Debug, PartialEq)]
enum Target {
    /// Nothing to play, still preparing, or our file isn't there yet.
    Stopped,
    Play {
        entry_id: u64,
        path: PathBuf,
        position: f64,
        at_ms: u64,
    },
    Pause {
        entry_id: u64,
        path: PathBuf,
        position: f64,
    },
}

fn current_entry(state: &GroupState) -> Option<&QueueEntry> {
    state.queue.get(state.current?)
}

fn target(state: &GroupState, files: &HashMap<String, PathBuf>) -> Target {
    let Some(entry) = current_entry(state) else {
        return Target::Stopped;
    };
    let Some(path) = files.get(&entry.youtube_id) else {
        return Target::Stopped;
    };
    match state.playback {
        Playback::Playing { position, at_ms } => Target::Play {
            entry_id: entry.entry_id,
            path: path.clone(),
            position,
            at_ms,
        },
        Playback::Paused { position } => Target::Pause {
            entry_id: entry.entry_id,
            path: path.clone(),
            position,
        },
        Playback::Idle | Playback::Preparing { .. } => Target::Stopped,
    }
}

/// The entries whose files should be on disk: the current one and the next two.
fn prefetch(state: &GroupState) -> impl Iterator<Item = &QueueEntry> {
    let skip = state.current.unwrap_or(state.queue.len());
    state.queue.iter().skip(skip).take(PREFETCH)
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
    joined: Option<oneshot::Sender<Result<(), String>>>,
    in_audio_group: bool,
    applied: Option<Target>,
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

pub struct GroupClient {
    inner: Arc<Inner>,
}

impl GroupClient {
    /// `dir` is `<cache>/groups`; anything left there by a crash is deleted.
    pub fn new(app: AppHandle, dir: PathBuf) -> Self {
        remove_dir(&dir);
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

    /// Connects, creates or joins, and resolves once the server sent the group state.
    /// Leaves the current group first, if any.
    pub async fn start(
        &self,
        intent: Intent,
        session: account::Session,
    ) -> Result<GroupView, String> {
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
            announced: HashSet::new(),
        });
        let url = websocket_url(&session.server_url, &session.token);
        tauri::async_runtime::spawn(run_connection(Arc::clone(inner), id, url, rx));

        match tokio::time::timeout(JOIN_TIMEOUT, joined_rx).await {
            Ok(Ok(Ok(()))) => self.view().ok_or_else(|| NOT_IN_GROUP.to_owned()),
            Ok(Ok(Err(e))) => Err(e),
            Ok(Err(_)) => Err("Connexion au groupe annulée".to_owned()),
            Err(_) => {
                inner.end(id, true);
                Err("Le serveur ne répond pas".to_owned())
            }
        }
    }

    /// Leaves the group (if any): tells the server, stops group playback and
    /// deletes the downloaded files.
    pub fn leave(&self) {
        let id = self.inner.current.lock().unwrap().as_ref().map(|m| m.id);
        if let Some(id) = id {
            self.inner.end(id, true);
        }
    }

    /// App exit: drop everything synchronously.
    pub fn shutdown(&self) {
        let membership = self.inner.current.lock().unwrap().take();
        if let Some(m) = membership {
            let _ = m.out.send(Outgoing::Close);
            for task in &m.download_tasks {
                task.abort();
            }
        }
        remove_dir(&self.inner.dir);
    }

    pub fn send(&self, msg: ClientMessage) -> Result<(), String> {
        let current = self.inner.current.lock().unwrap();
        let m = current
            .as_ref()
            .filter(|m| m.state.is_some())
            .ok_or(NOT_IN_GROUP)?;
        if !m.connected {
            return Err(DISCONNECTED.to_owned());
        }
        m.out
            .send(Outgoing::Msg(msg))
            .map_err(|_| DISCONNECTED.to_owned())
    }
}

impl Inner {
    fn audio(&self) -> tauri::State<'_, AudioHandle> {
        self.app.state::<AudioHandle>()
    }

    fn emit(&self, m: &Membership) {
        let _ = self.app.emit(STATE_EVENT, m.view());
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
        if m.in_audio_group {
            let _ = self.audio().leave_group();
        }
        if let Some(code) = &m.code {
            let dir = self.dir.join(code);
            remove_dir(&dir);
            // yt-dlp's ffmpeg may still write for a moment: clean again a bit later,
            // unless we rejoined the same group meanwhile.
            let inner = Arc::clone(self);
            let code = code.clone();
            std::thread::spawn(move || {
                std::thread::sleep(CLEANUP_DELAY);
                let rejoined = inner
                    .current
                    .lock()
                    .unwrap()
                    .as_ref()
                    .is_some_and(|m| m.code.as_deref() == Some(code.as_str()));
                if !rejoined {
                    remove_dir(&dir);
                }
            });
        }
        if m.state.is_some() {
            let _ = self.app.emit(STATE_EVENT, None::<GroupView>);
        }
        Some(m)
    }

    /// Fatal error from the connection: fails the pending create/join command, or
    /// reports it to the UI if we were already in the group.
    fn fail(self: &Arc<Self>, id: u64, message: String) {
        if let Some(mut m) = self.end(id, false) {
            match m.joined.take() {
                Some(joined) => {
                    let _ = joined.send(Err(message));
                }
                None => {
                    let _ = self.app.emit(ERROR_EVENT, message);
                }
            }
        }
    }

    /// While still waiting for the first `State`, a connection failure is reported
    /// right away instead of retrying in the background. Returns true if it failed.
    fn fail_if_joining(self: &Arc<Self>, id: u64, message: &str) -> bool {
        let joining = self.with(id, |m| m.joined.is_some()).unwrap_or(false);
        if joining {
            self.fail(id, message.to_owned());
        }
        joining
    }

    /// Returns the first message to send (create or re-join), or `None` if this
    /// membership is over.
    fn on_connected(&self, id: u64) -> Option<ClientMessage> {
        self.with(id, |m| {
            m.connected = true;
            m.announced.clear();
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
                        m.in_audio_group = self.audio().enter_group(self.ended_callback()).is_ok();
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
                    self.fail(id, message);
                    return false;
                }
                let _ = self.app.emit(ERROR_EVENT, message);
                true
            }
            ServerMessage::Left => {
                // We only send Leave when closing, so the server removed us.
                self.fail(id, "Tu as quitté le groupe".to_owned());
                false
            }
        }
    }

    fn ended_callback(self: &Arc<Self>) -> GroupEndedCallback {
        let inner: Weak<Self> = Arc::downgrade(self);
        Arc::new(move |entry_id| {
            let Some(inner) = inner.upgrade() else { return };
            let current = inner.current.lock().unwrap();
            if let Some(m) = current.as_ref() {
                let still_current = m
                    .state
                    .as_ref()
                    .and_then(current_entry)
                    .is_some_and(|e| e.entry_id == entry_id);
                if still_current && m.connected {
                    m.send(ClientMessage::Ended { entry_id });
                }
            }
        })
    }

    /// Brings downloads, `Ready` and the audio engine in line with the state.
    fn reconcile(self: &Arc<Self>, m: &mut Membership) {
        let Some(state) = m.state.clone() else { return };
        for entry in prefetch(&state) {
            self.ensure_download(m, &entry.youtube_id);
        }

        // Sent after the Join on the same socket, so the server handles it once joined.
        if m.connected {
            for youtube_id in m.files.keys() {
                if m.announced.insert(youtube_id.clone()) {
                    let _ = m.out.send(Outgoing::Msg(ClientMessage::Ready {
                        youtube_id: youtube_id.clone(),
                    }));
                }
            }
        }

        let target = target(&state, &m.files);
        if m.in_audio_group && m.applied.as_ref() != Some(&target) {
            let audio = self.audio();
            let path_str = |p: &Path| p.to_string_lossy().into_owned();
            let result = match &target {
                Target::Stopped => audio.group_stop(),
                Target::Play {
                    entry_id,
                    path,
                    position,
                    at_ms,
                } => audio.group_load(
                    *entry_id,
                    path_str(path),
                    *position,
                    GroupStart::At(m.clock.to_instant(*at_ms)),
                ),
                Target::Pause {
                    entry_id,
                    path,
                    position,
                } => audio.group_load(*entry_id, path_str(path), *position, GroupStart::Paused),
            };
            if let Err(e) = result {
                eprintln!("group: audio command failed: {e}");
            }
            m.applied = Some(target);
        }
    }

    fn ensure_download(self: &Arc<Self>, m: &mut Membership, youtube_id: &str) {
        if m.downloads.contains_key(youtube_id) {
            return;
        }
        if !is_valid_youtube_id(youtube_id) {
            let mut failed = TrackDownload::new(TrackStatus::Error);
            failed.error = Some("Identifiant YouTube invalide".to_owned());
            m.downloads.insert(youtube_id.to_owned(), failed);
            return;
        }
        // Already in the library: no need to download it again.
        let in_library = self
            .app
            .try_state::<Library>()
            .and_then(|library| library.youtube_path(youtube_id));
        if let Some(path) = in_library {
            m.files.insert(youtube_id.to_owned(), PathBuf::from(path));
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
            inner.set_download(id, &youtube_id, TrackStatus::Downloading, None, None);
            if let Err(e) = std::fs::create_dir_all(&dir) {
                let error = format!("Impossible de créer le dossier temporaire : {e}");
                inner.set_download(id, &youtube_id, TrackStatus::Error, None, Some(error));
                return;
            }
            // Built from the validated id only, never from what another member sent.
            let url = format!("https://music.youtube.com/watch?v={youtube_id}");
            let result = download_with_retry(&url, &dir, |status, progress| {
                let (status, progress) = match status {
                    DownloadStatus::Queued => (TrackStatus::Queued, None),
                    DownloadStatus::Downloading => (TrackStatus::Downloading, progress),
                    // Extracting/tagging: the download itself is done.
                    DownloadStatus::Converting => (TrackStatus::Downloading, Some(1.0)),
                    _ => return,
                };
                inner.set_download(id, &youtube_id, status, progress, None);
            })
            .await;
            inner.with(id, |m| {
                match result {
                    Ok(path) => {
                        m.files.insert(youtube_id.clone(), PathBuf::from(path));
                        m.downloads
                            .insert(youtube_id.clone(), TrackDownload::new(TrackStatus::Ready));
                        inner.reconcile(m);
                    }
                    Err(e) => {
                        let mut failed = TrackDownload::new(TrackStatus::Error);
                        failed.error = Some(e);
                        m.downloads.insert(youtube_id.clone(), failed);
                    }
                }
                inner.emit(m);
            });
        });
        m.download_tasks.retain(|t| !t.inner().is_finished());
        m.download_tasks.push(task);
    }

    fn set_download(
        &self,
        id: u64,
        youtube_id: &str,
        status: TrackStatus,
        progress: Option<f64>,
        error: Option<String>,
    ) {
        self.with(id, |m| {
            m.downloads.insert(
                youtube_id.to_owned(),
                TrackDownload {
                    status,
                    progress,
                    error,
                },
            );
            self.emit(m);
        });
    }
}

enum SocketEnd {
    /// Leave requested (or the membership is over): stop for good.
    Closed,
    /// Unexpected disconnection: reconnect.
    Lost,
}

/// Owns the WebSocket for one membership, reconnecting with backoff until the
/// group is left.
async fn run_connection(
    inner: Arc<Inner>,
    id: u64,
    url: String,
    mut rx: mpsc::UnboundedReceiver<Outgoing>,
) {
    let mut failures = 0usize;
    loop {
        let connect = tokio::time::timeout(CONNECT_TIMEOUT, tokio_tungstenite::connect_async(&url));
        let result = tokio::select! {
            result = connect => result,
            () = wait_for_close(&mut rx) => return,
        };
        let error = match result {
            Ok(Ok((socket, _))) => {
                failures = 0;
                match run_socket(&inner, id, socket, &mut rx).await {
                    SocketEnd::Closed => return,
                    SocketEnd::Lost => "connexion perdue".to_owned(),
                }
            }
            Ok(Err(tungstenite::Error::Http(response))) if response.status() == 401 => {
                // Bad or expired token: retrying would never work.
                inner.fail(id, "Session invalide, reconnecte-toi".to_owned());
                return;
            }
            Ok(Err(e)) => e.to_string(),
            Err(_) => "délai dépassé".to_owned(),
        };
        inner.on_disconnected(id);
        if inner.fail_if_joining(id, &format!("Serveur injoignable : {error}")) {
            return;
        }
        let delay = BACKOFF_SECS[failures.min(BACKOFF_SECS.len() - 1)];
        failures += 1;
        tokio::select! {
            () = tokio::time::sleep(Duration::from_secs(delay)) => {}
            () = wait_for_close(&mut rx) => return,
        }
    }
}

/// While offline: drops intents (commands already refuse them) until `Close`.
async fn wait_for_close(rx: &mut mpsc::UnboundedReceiver<Outgoing>) {
    while let Some(Outgoing::Msg(_)) = rx.recv().await {}
}

type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn run_socket(
    inner: &Arc<Inner>,
    id: u64,
    socket: Socket,
    rx: &mut mpsc::UnboundedReceiver<Outgoing>,
) -> SocketEnd {
    let (mut sink, mut stream) = socket.split();
    let send = |msg: &ClientMessage| {
        Message::text(serde_json::to_string(msg).expect("client messages serialize"))
    };

    let Some(hello) = inner.on_connected(id) else {
        let _ = sink.close().await;
        return SocketEnd::Closed;
    };
    if sink.send(send(&hello)).await.is_err() {
        return SocketEnd::Lost;
    }
    let mut awaiting_join = true;
    let mut pings = 0;
    let mut next_ping = tokio::time::Instant::now();
    let mut last_received = tokio::time::Instant::now();

    loop {
        tokio::select! {
            frame = stream.next() => {
                last_received = tokio::time::Instant::now();
                match frame {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<ServerMessage>(text.as_str()) {
                            Ok(msg) => {
                                if !inner.on_message(id, msg, &mut awaiting_join) {
                                    let _ = sink.close().await;
                                    return SocketEnd::Closed;
                                }
                            }
                            Err(e) => eprintln!("group: ignoring unknown message: {e}"),
                        }
                    }
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => return SocketEnd::Lost,
                    Some(Ok(_)) => {}
                }
            }
            out = rx.recv() => match out {
                Some(Outgoing::Msg(msg)) => {
                    if sink.send(send(&msg)).await.is_err() {
                        return SocketEnd::Lost;
                    }
                }
                Some(Outgoing::Close) | None => {
                    let _ = sink.send(send(&ClientMessage::Leave)).await;
                    let _ = sink.close().await;
                    return SocketEnd::Closed;
                }
            },
            () = tokio::time::sleep_until(next_ping) => {
                if last_received.elapsed() > SILENCE_TIMEOUT {
                    return SocketEnd::Lost;
                }
                let Some(ping) = inner.ping(id) else {
                    let _ = sink.close().await;
                    return SocketEnd::Closed;
                };
                if sink.send(send(&ping)).await.is_err() {
                    return SocketEnd::Lost;
                }
                pings += 1;
                next_ping += if pings < INITIAL_PINGS {
                    INITIAL_PING_INTERVAL
                } else {
                    PING_INTERVAL
                };
            }
        }
    }
}

fn remove_dir(dir: &Path) {
    if let Err(e) = std::fs::remove_dir_all(dir) {
        if e.kind() != std::io::ErrorKind::NotFound {
            eprintln!("group: cannot remove {}: {e}", dir.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use group_protocol::{parse_invite, Member};

    fn entry(entry_id: u64, youtube_id: &str) -> QueueEntry {
        QueueEntry {
            entry_id,
            youtube_id: youtube_id.to_owned(),
            title: format!("Titre {entry_id}"),
            artist: None,
            duration: Some(200.0),
            added_by: "alice".to_owned(),
        }
    }

    fn state(current: Option<usize>, playback: Playback) -> GroupState {
        GroupState {
            code: "ABC123".to_owned(),
            members: vec![Member {
                username: "alice".to_owned(),
                ready: false,
            }],
            queue: vec![
                entry(1, "aaaaaaaaaaa"),
                entry(2, "bbbbbbbbbbb"),
                entry(3, "ccccccccccc"),
                entry(4, "ddddddddddd"),
            ],
            current,
            playback,
        }
    }

    #[test]
    fn median_of_samples() {
        assert_eq!(median([].into_iter()), None);
        assert_eq!(median([5.0].into_iter()), Some(5.0));
        assert_eq!(median([9.0, 1.0, 5.0].into_iter()), Some(5.0));
        assert_eq!(median([4.0, 1.0, 3.0, 2.0].into_iter()), Some(2.5));
        assert_eq!(median([1.0, f64::NAN, 3.0].into_iter()), Some(2.0));
    }

    #[test]
    fn clock_offset_ignores_outliers() {
        let mut clock = Clock::new();
        // Server is 1000 ms ahead; symmetric 20 ms round trips…
        clock.add_sample(100.0, 120.0, 1110.0);
        clock.add_sample(200.0, 220.0, 1210.0);
        // …and one slow, asymmetric round trip.
        clock.add_sample(300.0, 700.0, 1350.0);
        assert_eq!(clock.offset(), 1000.0);
        // A pong "before" its ping is ignored.
        clock.add_sample(500.0, 400.0, 0.0);
        assert_eq!(clock.samples.len(), 3);
    }

    #[test]
    fn server_time_to_instant() {
        let mut clock = Clock::new();
        clock.add_sample(0.0, 0.0, 50_000.0);
        let at = clock.to_instant(52_500);
        assert_eq!(at - clock.base, Duration::from_millis(2500));
        // Before the local base: clamped rather than panicking.
        assert!(clock.to_instant(0) <= clock.base);
    }

    #[test]
    fn clock_falls_back_to_wall_clock() {
        let clock = Clock::new();
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let at = clock.to_instant(now_ms + 1000);
        let delta = at.saturating_duration_since(Instant::now());
        assert!(delta > Duration::from_millis(900) && delta <= Duration::from_millis(1001));
    }

    #[test]
    fn prefetches_current_and_next_two() {
        let ids = |s: &GroupState| prefetch(s).map(|e| e.entry_id).collect::<Vec<_>>();
        assert_eq!(ids(&state(Some(0), Playback::Idle)), [1, 2, 3]);
        assert_eq!(ids(&state(Some(2), Playback::Idle)), [3, 4]);
        assert!(ids(&state(None, Playback::Idle)).is_empty());
    }

    #[test]
    fn target_follows_playback() {
        let mut files = HashMap::new();
        let playing = Playback::Playing {
            position: 12.0,
            at_ms: 1000,
        };
        // Not downloaded yet: wait.
        assert_eq!(
            target(&state(Some(1), playing.clone()), &files),
            Target::Stopped
        );
        files.insert("bbbbbbbbbbb".to_owned(), PathBuf::from("/tmp/b.opus"));
        assert_eq!(
            target(&state(Some(1), playing.clone()), &files),
            Target::Play {
                entry_id: 2,
                path: PathBuf::from("/tmp/b.opus"),
                position: 12.0,
                at_ms: 1000,
            }
        );
        assert_eq!(
            target(&state(Some(1), Playback::Paused { position: 3.0 }), &files),
            Target::Pause {
                entry_id: 2,
                path: PathBuf::from("/tmp/b.opus"),
                position: 3.0,
            }
        );
        let preparing = Playback::Preparing { deadline_ms: 5 };
        assert_eq!(target(&state(Some(1), preparing), &files), Target::Stopped);
        assert_eq!(
            target(&state(None, Playback::Idle), &files),
            Target::Stopped
        );
    }

    #[test]
    fn unrelated_changes_keep_the_target() {
        let files = HashMap::from([("aaaaaaaaaaa".to_owned(), PathBuf::from("/a.opus"))]);
        let playing = Playback::Playing {
            position: 0.0,
            at_ms: 42,
        };
        let before = state(Some(0), playing.clone());
        let mut after = before.clone();
        after.members[0].ready = true;
        after.queue.push(entry(5, "eeeeeeeeeee"));
        after.queue.remove(2);
        assert_eq!(target(&before, &files), target(&after, &files));

        // A seek (new position/time) or another entry does change it.
        let mut seeked = before.clone();
        seeked.playback = Playback::Playing {
            position: 30.0,
            at_ms: 99,
        };
        assert_ne!(target(&before, &files), target(&seeked, &files));
    }

    #[test]
    fn invites_from_ui() {
        assert_eq!(
            parse_invite("musicapp://join/abc123"),
            Some("ABC123".into())
        );
        assert_eq!(parse_invite("abc"), None);
    }
}
