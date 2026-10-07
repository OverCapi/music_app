//! Engine thread: owns the rodio output and `Player`, runs the command loop and
//! pushes [`PlayerChanged`] / [`QueueChanged`] to the UI.

use std::fs::File;
use std::path::Path;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use lofty::prelude::*;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use tauri::AppHandle;
use tauri_specta::Event;

use super::opus::OpusSource;
use super::{
    GroupEndedCallback, GroupStart, PlayerChanged, PlayerState, QueueChanged, RandomPicker, Shared,
};
use crate::error::{AppError, AppResult};
use crate::library::TrackId;

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
    Play,
    Pause,
    Stop,
    Seek(f64),
    SetVolume(f32),
    /// The handle changed the queue: emit it.
    Publish,
    EnterGroup(GroupEndedCallback),
    LeaveGroup,
    GroupLoad {
        token: u64,
        path: String,
        position: f64,
        start: GroupStart,
    },
    GroupStop,
}

pub(super) fn spawn(
    app: AppHandle,
    rx: Receiver<AudioCommand>,
    shared: Arc<Mutex<Shared>>,
    pick_random: RandomPicker,
) {
    thread::Builder::new()
        .name("audio".into())
        .spawn(move || run(app, rx, shared, pick_random))
        .expect("failed to spawn audio thread");
}

fn run(
    app: AppHandle,
    rx: Receiver<AudioCommand>,
    shared: Arc<Mutex<Shared>>,
    pick_random: RandomPicker,
) {
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
            AudioCommand::Play => {
                if let Some(p) = &self.player {
                    p.play();
                }
            }
            AudioCommand::Pause => {
                if let Some(p) = &self.player {
                    p.pause();
                }
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
            AudioCommand::Publish => {}
            AudioCommand::EnterGroup(on_ended) => self.enter_group(on_ended),
            AudioCommand::LeaveGroup => self.leave_group(),
            AudioCommand::GroupLoad {
                token,
                path,
                position,
                start,
            } => self.group_load(token, path, position, start),
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

    fn group_load(&mut self, token: u64, path: String, position: f64, start: GroupStart) {
        let Some(group) = &mut self.group else { return };
        group.start_at = None;
        let reuse = group.token == Some(token) && self.player.is_some();
        if !reuse {
            group.token = None;
            match open_track(&path) {
                Ok((source, duration)) => {
                    // Paused before appending, so not a single sample plays early.
                    let player = Player::connect_new(self.device.mixer());
                    player.pause();
                    let mut s = self.shared.lock().unwrap();
                    player.set_volume(s.status.volume);
                    player.append(source);
                    self.player = Some(player);
                    group.token = Some(token);
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
        }
    }

    fn group_track_ended(&mut self) {
        let Some(group) = &mut self.group else { return };
        let token = group.token.take();
        group.start_at = None;
        let on_ended = Arc::clone(&group.on_ended);
        self.unload();
        if let Some(token) = token {
            on_ended(token);
        }
    }

    /// Plays `queue[index]`, skipping forward past files that fail to open.
    fn load_from(&mut self, mut index: usize) {
        loop {
            let Some(item) = self.shared().queue.get(index).cloned() else {
                self.unload();
                return;
            };
            match open_track(&item.path) {
                Ok((source, duration)) => {
                    // A fresh Player per track resets the position counter.
                    let player = Player::connect_new(self.device.mixer());
                    let mut s = self.shared.lock().unwrap();
                    player.set_volume(s.status.volume);
                    player.append(source);
                    self.player = Some(player);
                    s.queue.set_current(index);
                    s.queue_changed();
                    s.status.track_id = Some(item.track_id);
                    s.status.path = Some(item.path);
                    s.status.duration = duration;
                    s.status.position = 0.0;
                    s.status.playing = true;
                    s.status.loaded = true;
                    return;
                }
                Err(e) => {
                    let name = Path::new(&item.path)
                        .file_name()
                        .map_or(item.path.clone(), |n| n.to_string_lossy().into_owned());
                    tracing::warn!("cannot play {}: {e}", item.path);
                    self.shared().status.error = Some(format!("{name} : {e}"));
                    index += 1;
                }
            }
        }
    }

    fn unload(&mut self) {
        self.player = None;
        let mut s = self.shared();
        s.status.playing = false;
        s.status.loaded = false;
        s.status.position = 0.0;
    }

    fn sync_state(&mut self) {
        let Some(p) = &self.player else { return };
        if p.empty() && self.group.is_some() {
            // Group mode: no advance, the group client decides what comes next.
            self.group_track_ended();
        } else if p.empty() {
            // End of track: advance through the queue, or pick a random library track.
            let next = {
                let mut s = self.shared.lock().unwrap();
                match s.queue.next_index() {
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
            s.status.position = position;
        }
    }
}
