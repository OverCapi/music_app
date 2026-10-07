//! Audio engine: a dedicated thread owns the output device and the rodio `Player`.
//! The rest of the app talks to it through `AudioHandle` (command channel + shared state).
//!
//! The play queue lives in the shared state so the audio thread can advance to the
//! next track on its own when one ends.
//!
//! In group mode (listening group, see `group.rs`) the group client drives playback
//! instead: the local queue is frozen, tracks never advance on their own and the end
//! of a track is reported back through a callback.

use std::fs::File;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use lofty::prelude::*;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use serde::Serialize;

use crate::opus::OpusSource;

type TrackSource = Box<dyn Source + Send>;

/// Picks a random library track, avoiding the given path when possible.
pub type RandomPicker = Arc<dyn Fn(Option<&str>) -> Option<String> + Send + Sync>;

/// Called on the audio thread with the group token of the track that just finished.
pub type GroupEndedCallback = Arc<dyn Fn(u64) + Send + Sync>;

/// "Previous" restarts the current track instead when we're past this point.
const RESTART_THRESHOLD_SECS: f64 = 3.0;
/// Upper bound of the engine loop's wait (state refresh rate for the UI).
const TICK: Duration = Duration::from_millis(100);
/// A seek past the end that fails within this margin counts as "track over".
const END_MARGIN_SECS: f64 = 0.5;
const GROUP_MODE_ERROR: &str = "Tu es dans un groupe d'écoute : utilise les contrôles du groupe";

/// When a group track should start.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GroupStart {
    /// Stay paused at the position.
    Paused,
    /// Start at this instant. If it is already past, start right away further in
    /// the track, as if it had started on time.
    At(Instant),
}

enum AudioCommand {
    /// Load and play `queue[index]` (skipping unreadable files).
    PlayIndex(usize),
    Play,
    Pause,
    Stop,
    Seek(f64),
    SetVolume(f32),
    /// Freezes the local queue and hands playback over to the group client.
    EnterGroup(GroupEndedCallback),
    /// Stops group playback and restores the local queue (stopped).
    LeaveGroup,
    /// Loads `path` (unless `token` is already loaded), seeks to `position` and
    /// starts as `start` says. `token` identifies the group entry.
    GroupLoad {
        token: u64,
        path: String,
        position: f64,
        start: GroupStart,
    },
    GroupStop,
}

#[derive(Clone, Serialize)]
pub struct PlayerState {
    pub path: Option<String>,
    pub playing: bool,
    /// False when nothing is loaded (never played, stopped, or queue finished).
    pub loaded: bool,
    pub position: f64,
    pub duration: f64,
    pub volume: f32,
    /// Index of the current track in the queue.
    pub current: Option<usize>,
    /// Last playback error (e.g. a queued file that could not be read).
    pub error: Option<String>,
    /// Bumped on every queue change, so the UI knows when to refetch it
    /// (the engine appends random tracks on its own).
    pub queue_version: u64,
    /// A listening group drives playback; local queue controls are disabled.
    pub group_mode: bool,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            path: None,
            playing: false,
            loaded: false,
            position: 0.0,
            duration: 0.0,
            volume: 1.0,
            current: None,
            error: None,
            queue_version: 0,
            group_mode: false,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct QueueState {
    pub tracks: Vec<String>,
    pub current: Option<usize>,
    pub version: u64,
}

#[derive(Default)]
struct Shared {
    status: PlayerState,
    queue: Vec<String>,
}

impl Shared {
    fn queue_state(&self) -> QueueState {
        QueueState {
            tracks: self.queue.clone(),
            current: self.status.current,
            version: self.status.queue_version,
        }
    }

    fn queue_changed(&mut self) {
        self.status.queue_version += 1;
    }

    /// Queue exhausted: appends a random library track and returns its index.
    fn append_random(&mut self, pick: &RandomPicker) -> Option<usize> {
        let path = pick(self.status.path.as_deref())?;
        self.queue.push(path);
        self.queue_changed();
        Some(self.queue.len() - 1)
    }
}

pub struct AudioHandle {
    tx: Sender<AudioCommand>,
    shared: Arc<Mutex<Shared>>,
    pick_random: RandomPicker,
}

impl AudioHandle {
    /// `pick_random` supplies a library track whenever the queue runs out.
    pub fn spawn(pick_random: RandomPicker) -> Self {
        let (tx, rx) = mpsc::channel();
        let shared = Arc::new(Mutex::new(Shared::default()));
        let thread_shared = Arc::clone(&shared);
        let thread_pick = Arc::clone(&pick_random);
        thread::Builder::new()
            .name("audio".into())
            .spawn(move || audio_thread(rx, thread_shared, thread_pick))
            .expect("failed to spawn audio thread");
        Self {
            tx,
            shared,
            pick_random,
        }
    }

    pub fn state(&self) -> PlayerState {
        self.shared.lock().unwrap().status.clone()
    }

    pub fn queue(&self) -> QueueState {
        self.shared.lock().unwrap().queue_state()
    }

    /// Replaces the queue with `tracks` and starts playing `tracks[start]`.
    pub fn play_queue(&self, tracks: Vec<String>, start: usize) -> Result<QueueState, String> {
        self.ensure_local()?;
        if start >= tracks.len() {
            return Err("Index de départ invalide".to_owned());
        }
        let queue = {
            let mut s = self.shared.lock().unwrap();
            s.queue = tracks;
            s.status.current = Some(start);
            s.queue_changed();
            s.queue_state()
        };
        self.send(AudioCommand::PlayIndex(start))?;
        Ok(queue)
    }

    pub fn enqueue(&self, tracks: Vec<String>) -> Result<QueueState, String> {
        let mut s = self.shared.lock().unwrap();
        ensure_local(&s)?;
        s.queue.extend(tracks);
        s.queue_changed();
        Ok(s.queue_state())
    }

    pub fn play_index(&self, index: usize) -> Result<(), String> {
        self.ensure_local()?;
        if index >= self.shared.lock().unwrap().queue.len() {
            return Err("Ce titre n'est plus dans la file".to_owned());
        }
        self.send(AudioCommand::PlayIndex(index))
    }

    pub fn remove_from_queue(&self, index: usize) -> Result<QueueState, String> {
        let mut s = self.shared.lock().unwrap();
        ensure_local(&s)?;
        if index >= s.queue.len() {
            return Err("Ce titre n'est plus dans la file".to_owned());
        }
        match s.status.current {
            Some(c) if c == index => {
                return Err("Impossible de retirer le titre en cours".to_owned());
            }
            Some(c) if c > index => s.status.current = Some(c - 1),
            _ => {}
        }
        s.queue.remove(index);
        s.queue_changed();
        Ok(s.queue_state())
    }

    /// Removes every track except the current one.
    pub fn clear_queue(&self) -> Result<QueueState, String> {
        let mut s = self.shared.lock().unwrap();
        ensure_local(&s)?;
        match s.status.current {
            Some(c) => {
                let current = s.queue.swap_remove(c);
                s.queue = vec![current];
                s.status.current = Some(0);
            }
            None => s.queue.clear(),
        }
        s.queue_changed();
        Ok(s.queue_state())
    }

    pub fn play(&self) -> Result<(), String> {
        let cmd = {
            let mut s = self.shared.lock().unwrap();
            ensure_local(&s)?;
            match s.status.current {
                _ if s.status.loaded => AudioCommand::Play,
                // Stopped or queue finished: restart the current track.
                Some(c) => AudioCommand::PlayIndex(c),
                None if !s.queue.is_empty() => AudioCommand::PlayIndex(0),
                None => match s.append_random(&self.pick_random) {
                    Some(index) => AudioCommand::PlayIndex(index),
                    None => return Ok(()),
                },
            }
        };
        self.send(cmd)
    }

    pub fn next(&self) -> Result<(), String> {
        let cmd = {
            let mut s = self.shared.lock().unwrap();
            ensure_local(&s)?;
            let next = s.status.current.map_or(0, |c| c + 1);
            if next < s.queue.len() {
                AudioCommand::PlayIndex(next)
            } else {
                match s.append_random(&self.pick_random) {
                    Some(index) => AudioCommand::PlayIndex(index),
                    None => return Ok(()),
                }
            }
        };
        self.send(cmd)
    }

    pub fn previous(&self) -> Result<(), String> {
        let cmd = {
            let s = self.shared.lock().unwrap();
            ensure_local(&s)?;
            match s.status.current {
                Some(c) if c > 0 && s.status.position <= RESTART_THRESHOLD_SECS => {
                    AudioCommand::PlayIndex(c - 1)
                }
                Some(_) if s.status.loaded => AudioCommand::Seek(0.0),
                Some(c) => AudioCommand::PlayIndex(c),
                None => return Ok(()),
            }
        };
        self.send(cmd)
    }

    pub fn pause(&self) -> Result<(), String> {
        self.ensure_local()?;
        self.send(AudioCommand::Pause)
    }

    pub fn stop(&self) -> Result<(), String> {
        self.ensure_local()?;
        self.send(AudioCommand::Stop)
    }

    pub fn seek(&self, secs: f64) -> Result<(), String> {
        self.ensure_local()?;
        self.send(AudioCommand::Seek(secs.max(0.0)))
    }

    /// Hands playback over to the group client. The flag is set right away so local
    /// commands are refused even before the audio thread processes the switch.
    pub fn enter_group(&self, on_ended: GroupEndedCallback) -> Result<(), String> {
        self.shared.lock().unwrap().status.group_mode = true;
        self.send(AudioCommand::EnterGroup(on_ended))
    }

    pub fn leave_group(&self) -> Result<(), String> {
        self.send(AudioCommand::LeaveGroup)
    }

    pub fn group_load(
        &self,
        token: u64,
        path: String,
        position: f64,
        start: GroupStart,
    ) -> Result<(), String> {
        self.send(AudioCommand::GroupLoad {
            token,
            path,
            position: position.max(0.0),
            start,
        })
    }

    pub fn group_stop(&self) -> Result<(), String> {
        self.send(AudioCommand::GroupStop)
    }

    fn ensure_local(&self) -> Result<(), String> {
        ensure_local(&self.shared.lock().unwrap())
    }

    pub fn set_volume(&self, volume: f32) -> Result<(), String> {
        self.send(AudioCommand::SetVolume(volume.clamp(0.0, 1.0)))
    }

    fn send(&self, cmd: AudioCommand) -> Result<(), String> {
        self.tx
            .send(cmd)
            .map_err(|_| "Le moteur audio n'est plus disponible".to_owned())
    }
}

fn ensure_local(s: &Shared) -> Result<(), String> {
    if s.status.group_mode {
        Err(GROUP_MODE_ERROR.to_owned())
    } else {
        Ok(())
    }
}

fn open_track(path: &str) -> Result<(TrackSource, f64), String> {
    let file = File::open(path).map_err(|e| format!("impossible d'ouvrir le fichier ({e})"))?;
    let is_opus = Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("opus"));
    let source: TrackSource = if is_opus {
        Box::new(OpusSource::open(file)?)
    } else {
        Box::new(Decoder::try_from(file).map_err(|e| format!("format non supporté ({e})"))?)
    };
    let duration = lofty::read_from_path(Path::new(path))
        .map(|f| f.properties().duration().as_secs_f64())
        .unwrap_or(0.0);
    Ok((source, duration))
}

fn audio_thread(rx: Receiver<AudioCommand>, shared: Arc<Mutex<Shared>>, pick_random: RandomPicker) {
    let opened = DeviceSinkBuilder::from_default_device()
        .map(|b| b.with_error_callback(log_stream_error))
        .and_then(|b| b.open_sink_or_fallback());
    let mut device = match opened {
        Ok(d) => d,
        Err(e) => {
            eprintln!("audio: no output device: {e}");
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
    }
}

fn log_stream_error(err: rodio::cpal::StreamError) {
    let msg = err.to_string();
    // Known harmless ALSA timestamp jitter (cpal on PipeWire/Pulse), emitted on every callback.
    if msg.contains("get_htstamp") {
        return;
    }
    eprintln!("audio stream error: {msg}");
}

struct Engine {
    device: MixerDeviceSink,
    player: Option<Player>,
    shared: Arc<Mutex<Shared>>,
    pick_random: RandomPicker,
    /// Set while a listening group drives playback.
    group: Option<GroupMode>,
}

struct GroupMode {
    on_ended: GroupEndedCallback,
    /// Group entry currently loaded in `player`.
    token: Option<u64>,
    /// Pending scheduled start (the player waits paused until then).
    start_at: Option<Instant>,
    /// Local track shown before entering group mode, restored when leaving.
    saved_path: Option<String>,
    saved_duration: f64,
}

impl Engine {
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
                self.shared.lock().unwrap().status.error = None;
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
                if let Some(p) = &self.player {
                    if let Err(e) = p.try_seek(Duration::from_secs_f64(secs)) {
                        eprintln!("audio: seek failed: {e}");
                    }
                }
            }
            AudioCommand::SetVolume(volume) => {
                self.shared.lock().unwrap().status.volume = volume;
                if let Some(p) = &self.player {
                    p.set_volume(volume);
                }
            }
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
            saved_path: s.status.path.take(),
            saved_duration: std::mem::take(&mut s.status.duration),
        });
    }

    /// Back to the local queue, stopped on the track it was on.
    fn leave_group(&mut self) {
        let Some(group) = self.group.take() else {
            self.shared.lock().unwrap().status.group_mode = false;
            return;
        };
        self.unload();
        let mut s = self.shared.lock().unwrap();
        s.status.group_mode = false;
        s.status.error = None;
        s.status.path = group.saved_path;
        s.status.duration = group.saved_duration;
    }

    fn group_stop(&mut self) {
        if let Some(group) = &mut self.group {
            group.token = None;
            group.start_at = None;
            self.unload();
            self.shared.lock().unwrap().status.path = None;
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
                    s.status.path = Some(path);
                    s.status.duration = duration;
                    s.status.position = 0.0;
                    s.status.playing = false;
                    s.status.loaded = true;
                    s.status.error = None;
                }
                Err(e) => {
                    self.unload();
                    self.shared.lock().unwrap().status.error = Some(e);
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
                let now = Instant::now();
                if at > now {
                    // Seek now (it blocks a few ms), start exactly at `at`.
                    if self.group_seek(position) {
                        if let Some(group) = &mut self.group {
                            group.start_at = Some(at);
                        }
                    }
                } else {
                    // Late (slow download, reconnection…): join where the others are.
                    // Measured after opening the file, which can take a while.
                    let late = Instant::now().duration_since(at).as_secs_f64();
                    if self.group_seek(position + late) {
                        if let Some(p) = &self.player {
                            p.play();
                        }
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
                let duration = self.shared.lock().unwrap().status.duration;
                if duration > 0.0 && position >= duration - END_MARGIN_SECS {
                    self.group_track_ended();
                    false
                } else {
                    eprintln!("audio: group seek failed: {e}");
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
            let Some(path) = self.shared.lock().unwrap().queue.get(index).cloned() else {
                self.unload();
                return;
            };
            match open_track(&path) {
                Ok((source, duration)) => {
                    // A fresh Player per track resets the position counter.
                    let player = Player::connect_new(self.device.mixer());
                    let mut s = self.shared.lock().unwrap();
                    player.set_volume(s.status.volume);
                    player.append(source);
                    self.player = Some(player);
                    s.status.current = Some(index);
                    s.status.path = Some(path);
                    s.status.duration = duration;
                    s.status.position = 0.0;
                    s.status.playing = true;
                    s.status.loaded = true;
                    return;
                }
                Err(e) => {
                    let name = Path::new(&path)
                        .file_name()
                        .map_or(path.clone(), |n| n.to_string_lossy().into_owned());
                    self.shared.lock().unwrap().status.error = Some(format!("{name} : {e}"));
                    index += 1;
                }
            }
        }
    }

    fn unload(&mut self) {
        self.player = None;
        let mut s = self.shared.lock().unwrap();
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
                let next = s.status.current.map_or(0, |c| c + 1);
                if next < s.queue.len() {
                    Some(next)
                } else {
                    s.append_random(&self.pick_random)
                }
            };
            match next {
                Some(index) => self.load_from(index),
                None => self.unload(),
            }
        } else {
            let mut s = self.shared.lock().unwrap();
            s.status.playing = !p.is_paused();
            s.status.position = p.get_pos().as_secs_f64();
        }
    }
}
