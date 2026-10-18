//! Engine thread: owns the rodio output and `Player`, runs the command loop and
//! pushes [`PlayerChanged`] / [`QueueChanged`] to the UI.

use std::collections::HashSet;
use std::fs::File;
use std::path::Path;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use lofty::prelude::*;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use tauri::AppHandle;
use tauri_specta::Event;

use super::listen::Listen;
use super::live::{LiveError, LiveSource};
use super::opus::OpusSource;
use super::{
    GroupEndedCallback, GroupStart, PlayerChanged, PlayerState, QueueChanged, QueueItem,
    RandomPicker, Shared, TrackFetcher,
};
use crate::db::now_secs;
use crate::error::{AppError, AppResult};
use crate::history::{PlayRecorder, PlaySource, PlayedTrack};
use crate::library::TrackId;
use crate::lives::LiveStream;

type TrackSource = Box<dyn Source + Send>;

/// Upper bound of the engine loop's wait (position refresh rate).
const TICK: Duration = Duration::from_millis(100);
/// While playing, position-only updates are pushed at most this often.
const POSITION_INTERVAL: Duration = Duration::from_millis(250);
/// A seek past the end that fails within this margin counts as "track over".
const END_MARGIN_SECS: f64 = 0.5;

pub(super) enum AudioCommand {
    /// Load and play `queue[index]` (skipping unreadable files).
    PlayIndex(usize),
    /// Stream a live, outside the queue.
    PlayLive(LiveStream),
    Play,
    Pause,
    Stop,
    Seek(f64),
    SetVolume(f32),
    /// The handle changed the queue: emit it.
    Publish,
    /// A [`TrackFetcher`] call ended: the file path, or an error message.
    Fetched {
        track_id: TrackId,
        result: Result<String, String>,
    },
    EnterGroup(GroupEndedCallback),
    LeaveGroup,
    GroupLoad {
        token: u64,
        path: String,
        position: f64,
        start: GroupStart,
        track: PlayedTrack,
    },
    GroupStop,
}

/// What the engine thread is built from.
pub(super) struct EngineParts {
    pub app: AppHandle,
    pub rx: Receiver<AudioCommand>,
    /// To send [`AudioCommand::Fetched`] back to the engine.
    pub tx: Sender<AudioCommand>,
    pub shared: Arc<Mutex<Shared>>,
    pub pick_random: RandomPicker,
    pub record: PlayRecorder,
    pub fetch: TrackFetcher,
}

pub(super) fn spawn(parts: EngineParts) {
    thread::Builder::new()
        .name("audio".into())
        .spawn(move || run(parts))
        .expect("failed to spawn audio thread");
}

fn run(parts: EngineParts) {
    let EngineParts {
        app,
        rx,
        tx,
        shared,
        pick_random,
        record,
        fetch,
    } = parts;
    let opened = DeviceSinkBuilder::from_default_device()
        .map(|b| b.with_error_callback(log_stream_error))
        .and_then(|b| b.open_sink_or_fallback());
    let mut device = match opened {
        Ok(d) => d,
        Err(e) => {
            tracing::error!("no audio output device: {e}");
            return;
        }
    };
    device.log_on_drop(false);

    let mut engine = Engine {
        device,
        player: None,
        shared,
        pick_random,
        record,
        fetch,
        tx,
        waiting: None,
        fetching: HashSet::new(),
        listen: None,
        live_error: None,
        group: None,
        app,
        published: PlayerState::default(),
        published_at: Instant::now(),
    };

    loop {
        // Wake up exactly at a scheduled group start rather than on the next tick:
        // members must start within a few milliseconds of each other.
        let timeout = engine
            .group
            .as_ref()
            .and_then(|g| g.start_at)
            .map_or(TICK, |at| {
                at.saturating_duration_since(Instant::now()).min(TICK)
            });
        match rx.recv_timeout(timeout) {
            Ok(cmd) => engine.handle(cmd),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        engine.start_if_due();
        engine.sync_state();
        engine.publish();
    }
    // App shutting down: the current listen still counts.
    engine.finish_listen(false);
}

fn log_stream_error(err: rodio::cpal::StreamError) {
    let msg = err.to_string();
    // Known harmless ALSA timestamp jitter (cpal on PipeWire/Pulse), emitted on every callback.
    if msg.contains("get_htstamp") {
        return;
    }
    tracing::warn!("audio stream error: {msg}");
}

/// Opens a file for playback; returns the source and its duration (0 when unknown).
/// Opus goes through [`OpusSource`], everything else through rodio's decoder.
fn open_track(path: &str) -> AppResult<(TrackSource, f64)> {
    let file = File::open(path)
        .map_err(|e| AppError::invalid(format!("impossible d'ouvrir le fichier ({e})")))?;
    let is_opus = Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("opus"));
    let source: TrackSource = if is_opus {
        Box::new(OpusSource::open(file)?)
    } else {
        Box::new(
            Decoder::try_from(file)
                .map_err(|e| AppError::invalid(format!("format non supporté ({e})")))?,
        )
    };
    let duration = lofty::read_from_path(Path::new(path))
        .map(|f| f.properties().duration().as_secs_f64())
        .unwrap_or(0.0);
    Ok((source, duration))
}

struct Engine {
    device: MixerDeviceSink,
    player: Option<Player>,
    shared: Arc<Mutex<Shared>>,
    pick_random: RandomPicker,
    record: PlayRecorder,
    fetch: TrackFetcher,
    tx: Sender<AudioCommand>,
    /// Current queue track being downloaded; it plays when its file arrives.
    waiting: Option<TrackId>,
    /// Tracks with a fetch in flight (current or prefetched).
    fetching: HashSet<TrackId>,
    /// Listen of the track loaded in `player`, handed to `record` when it ends.
    listen: Option<Listen>,
    /// Set by the feeder thread of the live being streamed when it gives up.
    live_error: Option<LiveError>,
    /// Set while a listening group drives playback.
    group: Option<GroupMode>,
    app: AppHandle,
    /// Last [`PlayerChanged`] payload sent, and when.
    published: PlayerState,
    published_at: Instant,
}

struct GroupMode {
    on_ended: GroupEndedCallback,
    /// Group entry currently loaded in `player`.
    token: Option<u64>,
    /// Pending scheduled start (the player waits paused until then).
    start_at: Option<Instant>,
    /// Local track shown before entering group mode, restored when leaving.
    saved_track_id: Option<TrackId>,
    saved_path: Option<String>,
    saved_duration: f64,
}

impl Engine {
    fn shared(&self) -> MutexGuard<'_, Shared> {
        self.shared.lock().unwrap()
    }

    fn handle(&mut self, cmd: AudioCommand) {
        let is_local = matches!(
            cmd,
            AudioCommand::PlayIndex(_)
                | AudioCommand::PlayLive(_)
                | AudioCommand::Play
                | AudioCommand::Pause
                | AudioCommand::Stop
                | AudioCommand::Seek(_)
        );
        // A local command that raced with entering group mode.
        if is_local && self.group.is_some() {
            return;
        }
        match cmd {
            AudioCommand::PlayIndex(index) => {
                self.shared().status.error = None;
                self.load_from(index);
            }
            AudioCommand::PlayLive(live) => self.play_live(live),
            AudioCommand::Play => {
                if self.shared().status.live.is_some() && self.player.is_none() {
                    // Paused live: rejoin the stream where it is now.
                    self.start_live_stream();
                } else if let Some(p) = &self.player {
                    p.play();
                    self.resume_listen();
                }
            }
            AudioCommand::Pause => {
                if self.shared().status.live.is_some() {
                    // No pausing a live: stop the stream (and its processes).
                    self.player = None;
                    self.live_error = None;
                    self.shared().status.playing = false;
                } else if let Some(p) = &self.player {
                    p.pause();
                }
                self.pause_listen();
                // Paused while downloading: don't start when the file arrives.
                self.stop_waiting();
            }
            AudioCommand::Stop => self.unload(),
            AudioCommand::Seek(secs) => {
                if let Some(p) = &self.player
                    && let Err(e) = p.try_seek(Duration::from_secs_f64(secs))
                {
                    tracing::warn!("seek failed: {e}");
                }
            }
            AudioCommand::SetVolume(volume) => {
                self.shared().status.volume = volume;
                if let Some(p) = &self.player {
                    p.set_volume(volume);
                }
            }
            // The queue may have a new next track to fetch.
            AudioCommand::Publish => self.prefetch_next(),
            AudioCommand::Fetched { track_id, result } => self.fetched(track_id, result),
            AudioCommand::EnterGroup(on_ended) => self.enter_group(on_ended),
            AudioCommand::LeaveGroup => self.leave_group(),
            AudioCommand::GroupLoad {
                token,
                path,
                position,
                start,
                track,
            } => self.group_load(token, path, position, start, track),
            AudioCommand::GroupStop => self.group_stop(),
        }
    }

    /// Emits [`QueueChanged`] if the queue changed, and [`PlayerChanged`] if the state
    /// changed (position-only changes at most every [`POSITION_INTERVAL`]).
    fn publish(&mut self) {
        let (state, queue) = {
            let mut s = self.shared();
            let queue = std::mem::take(&mut s.queue_dirty).then(|| s.queue.state());
            (s.player_state(), queue)
        };
        if let Some(queue) = queue
            && let Err(e) = QueueChanged(queue).emit(&self.app)
        {
            tracing::warn!("emit QueueChanged failed: {e}");
        }
        if state == self.published {
            return;
        }
        let position_only = PlayerState {
            position: self.published.position,
            ..state.clone()
        } == self.published;
        if position_only && self.published_at.elapsed() < POSITION_INTERVAL {
            return;
        }
        if let Err(e) = PlayerChanged(state.clone()).emit(&self.app) {
            tracing::warn!("emit PlayerChanged failed: {e}");
        }
        self.published = state;
        self.published_at = Instant::now();
    }

    fn enter_group(&mut self, on_ended: GroupEndedCallback) {
        if let Some(group) = &mut self.group {
            // Switching groups: keep what was saved from the local queue.
            group.on_ended = on_ended;
            self.group_stop();
            return;
        }
        self.unload();
        let mut s = self.shared.lock().unwrap();
        s.status.group_mode = true;
        s.status.error = None;
        self.group = Some(GroupMode {
            on_ended,
            token: None,
            start_at: None,
            saved_track_id: s.status.track_id.take(),
            saved_path: s.status.path.take(),
            saved_duration: std::mem::take(&mut s.status.duration),
        });
    }

    /// Back to the local queue, stopped on the track it was on.
    fn leave_group(&mut self) {
        let Some(group) = self.group.take() else {
            self.shared().status.group_mode = false;
            return;
        };
        self.unload();
        let mut s = self.shared();
        s.status.group_mode = false;
        s.status.error = None;
        s.status.track_id = group.saved_track_id;
        s.status.path = group.saved_path;
        s.status.duration = group.saved_duration;
    }

    fn group_stop(&mut self) {
        if let Some(group) = &mut self.group {
            group.token = None;
            group.start_at = None;
            self.unload();
            self.shared().status.path = None;
        }
    }

    fn group_load(
        &mut self,
        token: u64,
        path: String,
        position: f64,
        start: GroupStart,
        track: PlayedTrack,
    ) {
        let Some(group) = &mut self.group else { return };
        group.start_at = None;
        let reuse = group.token == Some(token) && self.player.is_some();
        if !reuse {
            group.token = None;
            // Another entry: the previous listen ends here, even if the file fails.
            self.finish_listen(false);
            match open_track(&path) {
                Ok((source, duration)) => {
                    // Paused before appending, so not a single sample plays early.
                    let player = Player::connect_new(self.device.mixer());
                    player.pause();
                    let mut s = self.shared.lock().unwrap();
                    player.set_volume(s.status.volume);
                    player.append(source);
                    self.player = Some(player);
                    self.listen = Some(Listen::new(track, PlaySource::Group, now_secs(), duration));
                    if let Some(group) = &mut self.group {
                        group.token = Some(token);
                    }
                    s.status.track_id = None;
                    s.status.path = Some(path);
                    s.status.duration = duration;
                    s.status.position = 0.0;
                    s.status.playing = false;
                    s.status.loaded = true;
                    s.status.error = None;
                }
                Err(e) => {
                    self.unload();
                    self.shared().status.error = Some(e.to_string());
                    return;
                }
            }
        }
        let Some(player) = &self.player else { return };
        player.pause();
        self.pause_listen();
        match start {
            GroupStart::Paused => {
                self.group_seek(position);
            }
            GroupStart::At(at) => {
                if at > Instant::now() {
                    // Seek now (it blocks a few ms), start exactly at `at`.
                    if self.group_seek(position)
                        && let Some(group) = &mut self.group
                    {
                        group.start_at = Some(at);
                    }
                } else {
                    // Late (slow download, reconnection…): join where the others are.
                    // Measured after opening the file, which can take a while.
                    let late = Instant::now().duration_since(at).as_secs_f64();
                    if self.group_seek(position + late)
                        && let Some(p) = &self.player
                    {
                        p.play();
                        self.resume_listen();
                    }
                }
            }
        }
    }

    /// Seeks the group player. A failed seek past the end means the track is over
    /// for this group (reported as ended); returns false in that case.
    fn group_seek(&mut self, position: f64) -> bool {
        let Some(player) = &self.player else {
            return false;
        };
        if position <= 0.0 {
            // A fresh player is already at the start; a reused one needs the seek.
            let _ = player.try_seek(Duration::ZERO);
            return true;
        }
        match player.try_seek(Duration::from_secs_f64(position)) {
            Ok(()) => true,
            Err(e) => {
                let duration = self.shared().status.duration;
                if duration > 0.0 && position >= duration - END_MARGIN_SECS {
                    self.group_track_ended();
                    false
                } else {
                    tracing::warn!("group seek failed: {e}");
                    true
                }
            }
        }
    }

    /// Starts the group player if its scheduled start time has come.
    fn start_if_due(&mut self) {
        let Some(group) = &mut self.group else { return };
        if group.start_at.is_some_and(|at| Instant::now() >= at) {
            group.start_at = None;
            if let Some(p) = &self.player {
                p.play();
            }
            self.resume_listen();
        }
    }

    fn group_track_ended(&mut self) {
        let Some(group) = &mut self.group else { return };
        let token = group.token.take();
        group.start_at = None;
        let on_ended = Arc::clone(&group.on_ended);
        self.finish_listen(true);
        self.unload();
        if let Some(token) = token {
            on_ended(token);
        }
    }

    /// Starts a live. The queue and its current track are left as they are.
    fn play_live(&mut self, live: LiveStream) {
        // Ends the previous listen.
        self.unload();
        self.listen = Some(Listen::new(
            PlayedTrack::Live {
                youtube_id: live.id.clone(),
                title: live.title.clone(),
                artist: live.channel.clone(),
            },
            PlaySource::Live,
            now_secs(),
            0.0,
        ));
        {
            let mut s = self.shared();
            s.status.live = Some(live);
            s.status.track_id = None;
            s.status.path = None;
            s.status.duration = 0.0;
            s.status.error = None;
            s.status.loaded = true;
        }
        self.start_live_stream();
    }

    /// Streams the live of `status.live` (new start, or resume after a pause).
    fn start_live_stream(&mut self) {
        let Some(id) = self.shared().status.live.as_ref().map(|l| l.id.clone()) else {
            return;
        };
        match LiveSource::start(&id) {
            Ok((source, error)) => {
                let player = Player::connect_new(self.device.mixer());
                let mut s = self.shared.lock().unwrap();
                player.set_volume(s.status.volume);
                player.append(source);
                s.status.playing = true;
                s.status.position = 0.0;
                drop(s);
                self.player = Some(player);
                self.live_error = Some(error);
                self.resume_listen();
            }
            Err(e) => {
                self.unload();
                self.shared().status.error = Some(e.to_string());
            }
        }
    }

    /// Plays `queue[index]`, skipping forward past files that fail to open. A track
    /// without a file is downloaded first (the player shows it as loading).
    fn load_from(&mut self, mut index: usize) {
        loop {
            let Some(item) = self.shared().queue.get(index).cloned() else {
                self.unload();
                return;
            };
            let Some(path) = item.path.clone() else {
                if let Some(youtube_id) = item.youtube_id.clone() {
                    self.wait_for(index, &item, youtube_id);
                    return;
                }
                // `queue_items` refuses those; only here if the track changed since.
                self.shared().status.error = Some("Aucune vidéo YouTube associée".into());
                index += 1;
                continue;
            };
            match open_track(&path) {
                Ok((source, duration)) => {
                    // Only now: a file that fails to open must not end the current listen.
                    self.finish_listen(false);
                    let mut listen = Listen::new(
                        PlayedTrack::Library(item.track_id),
                        item.source,
                        now_secs(),
                        duration,
                    );
                    listen.resume(Instant::now());
                    self.listen = Some(listen);
                    // A fresh Player per track resets the position counter.
                    let player = Player::connect_new(self.device.mixer());
                    let mut s = self.shared.lock().unwrap();
                    player.set_volume(s.status.volume);
                    player.append(source);
                    self.player = Some(player);
                    self.live_error = None;
                    s.status.live = None;
                    s.queue.set_current(index);
                    s.queue_changed();
                    s.status.track_id = Some(item.track_id);
                    s.status.path = Some(path);
                    s.status.duration = duration;
                    s.status.position = 0.0;
                    s.status.playing = true;
                    s.status.loaded = true;
                    s.status.loading = false;
                    drop(s);
                    self.waiting = None;
                    self.prefetch_next();
                    return;
                }
                Err(e) => {
                    let name = Path::new(&path)
                        .file_name()
                        .map_or(path.clone(), |n| n.to_string_lossy().into_owned());
                    tracing::warn!("cannot play {path}: {e}");
                    self.shared().status.error = Some(format!("{name} : {e}"));
                    index += 1;
                }
            }
        }
    }

    fn unload(&mut self) {
        self.finish_listen(false);
        self.player = None;
        self.live_error = None;
        self.waiting = None;
        let mut s = self.shared();
        s.status.live = None;
        s.status.playing = false;
        s.status.loaded = false;
        s.status.loading = false;
        s.status.position = 0.0;
    }

    /// Makes `queue[index]` current, shown as loading, and downloads it.
    fn wait_for(&mut self, index: usize, item: &QueueItem, youtube_id: String) {
        // The previous track stops now.
        self.unload();
        {
            let mut s = self.shared();
            s.queue.set_current(index);
            s.queue_changed();
            s.status.track_id = Some(item.track_id);
            s.status.path = None;
            s.status.duration = 0.0;
            s.status.loading = true;
        }
        self.waiting = Some(item.track_id);
        self.request(item.track_id, youtube_id);
    }

    fn stop_waiting(&mut self) {
        if self.waiting.take().is_some() {
            self.shared().status.loading = false;
        }
    }

    /// Starts a fetch unless one is already in flight for this track.
    fn request(&mut self, track_id: TrackId, youtube_id: String) {
        if !self.fetching.insert(track_id) {
            return;
        }
        let tx = self.tx.clone();
        (self.fetch)(
            track_id,
            youtube_id,
            Box::new(move |result| {
                // Nothing to do if the engine is gone.
                let _ = tx.send(AudioCommand::Fetched { track_id, result });
            }),
        );
    }

    /// Downloads the track after the current one in advance, so it starts without
    /// a gap. Not when shuffling: the next track isn't known yet.
    fn prefetch_next(&mut self) {
        let next = {
            let s = self.shared();
            if self.group.is_some() || !s.status.loaded || s.status.shuffle {
                return;
            }
            s.next_index(false)
                .and_then(|i| s.queue.get(i))
                .filter(|item| item.path.is_none())
                .and_then(|item| Some((item.track_id, item.youtube_id.clone()?)))
        };
        if let Some((track_id, youtube_id)) = next {
            self.request(track_id, youtube_id);
        }
    }

    fn fetched(&mut self, track_id: TrackId, result: Result<String, String>) {
        self.fetching.remove(&track_id);
        let waited = self.waiting == Some(track_id) && self.group.is_none();
        let current = {
            let mut s = self.shared();
            if let Ok(path) = &result
                && s.queue.set_path(track_id, path)
            {
                s.queue_changed();
            }
            s.queue
                .current()
                .filter(|&c| s.queue.get(c).is_some_and(|i| i.track_id == track_id))
        };
        if !waited {
            return;
        }
        // The current track changed meanwhile (queue edited): stay stopped.
        let Some(index) = current else {
            self.stop_waiting();
            return;
        };
        self.waiting = None;
        match result {
            Ok(_) => self.load_from(index),
            Err(e) => {
                tracing::warn!("cannot download track {track_id}: {e}");
                self.shared().status.error = Some(format!("Téléchargement impossible : {e}"));
                self.load_from(index + 1);
            }
        }
    }

    /// Hands the current listen over to the history (never with `shared` locked:
    /// the recorder writes to the database).
    fn finish_listen(&mut self, completed: bool) {
        if let Some(listen) = self.listen.take() {
            (self.record)(listen.finish(Instant::now(), completed));
        }
    }

    fn resume_listen(&mut self) {
        if let Some(listen) = &mut self.listen {
            listen.resume(Instant::now());
        }
    }

    fn pause_listen(&mut self) {
        if let Some(listen) = &mut self.listen {
            listen.pause(Instant::now());
        }
    }

    fn sync_state(&mut self) {
        let live_error = self
            .live_error
            .as_ref()
            .and_then(|e| e.lock().unwrap().take());
        if let Some(message) = live_error {
            self.unload();
            self.shared().status.error = Some(message);
            return;
        }
        let Some(p) = &self.player else { return };
        if p.empty() && self.group.is_some() {
            // Group mode: no advance, the group client decides what comes next.
            self.group_track_ended();
        } else if p.empty() {
            // End of track: advance through the queue, or pick a random library track.
            // Repeat-one replays start a new listen.
            self.finish_listen(true);
            let next = {
                let mut s = self.shared.lock().unwrap();
                match s.next_index(true) {
                    Some(next) => Some(next),
                    None => s.append_random(&self.pick_random),
                }
            };
            match next {
                Some(index) => self.load_from(index),
                None => self.unload(),
            }
        } else {
            let playing = !p.is_paused();
            let position = p.get_pos().as_secs_f64();
            let mut s = self.shared.lock().unwrap();
            s.status.playing = playing;
            if s.status.live.is_none() {
                s.status.position = position;
            }
        }
    }
}
