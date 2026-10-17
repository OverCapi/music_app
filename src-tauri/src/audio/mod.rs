//! Audio engine: a dedicated thread owns the output device and the rodio `Player`
//! (not `Send`). The rest of the app talks to it through [`AudioHandle`]
//! (command channel + shared state).
//!
//! The play queue lives in the engine so tracks advance without the UI; when it
//! runs out the engine appends a random library track ([`RandomPicker`]).
//!
//! In group mode the listening-group client drives playback with scheduled starts
//! ([`GroupStart`]): local queue commands return [`AppError::GroupMode`], tracks never
//! advance on their own and the end of a track is reported through a callback.
//!
//! The engine pushes [`PlayerChanged`] on every state change (and a few times per
//! second while playing) and [`QueueChanged`] on every queue change: the UI never polls.

pub mod commands;
mod engine;
mod listen;
mod opus;
mod queue;

use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use group_protocol::MAX_POSITION_SECS;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::error::{AppError, AppResult};
use crate::history::{PlayRecorder, PlaySource, PlayedTrack};
use crate::library::TrackId;
use engine::AudioCommand;
use queue::Queue;

/// "Previous" restarts the current track instead when we're past this point.
const RESTART_THRESHOLD_SECS: f64 = 3.0;

/// One queue entry: the library track, its resolved path and where it came from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct QueueItem {
    pub track_id: TrackId,
    pub path: String,
    pub source: PlaySource,
}

/// Picks a random library track, avoiding the given one when possible.
pub type RandomPicker = Arc<dyn Fn(Option<TrackId>) -> Option<QueueItem> + Send + Sync>;

/// Called on the audio thread with the group token of the track that just finished.
pub type GroupEndedCallback = Arc<dyn Fn(u64) + Send + Sync>;

/// When a group track should start.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GroupStart {
    /// Stay paused at the position.
    Paused,
    /// Start at this instant. If it is already past, start right away further in
    /// the track, as if it had started on time.
    At(Instant),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum RepeatMode {
    #[default]
    Off,
    /// Loop the whole queue.
    All,
    /// Replay the current track.
    One,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, specta::Type)]
pub struct PlayerState {
    /// Library track being played (`None` in group mode or when nothing is loaded).
    pub track_id: Option<TrackId>,
    /// File being played, also set in group mode (temp file).
    pub path: Option<String>,
    pub playing: bool,
    /// False when nothing is loaded (never played, stopped, or queue finished).
    pub loaded: bool,
    /// Seconds.
    pub position: f64,
    /// Seconds (0 when unknown).
    pub duration: f64,
    /// 0.0–1.0.
    pub volume: f32,
    /// Index of the current track in the queue.
    pub current: Option<u32>,
    /// Last playback error (e.g. a queued file that could not be read).
    pub error: Option<String>,
    /// A listening group drives playback; local queue controls are disabled.
    pub group_mode: bool,
    /// Next track is picked at random in the queue.
    pub shuffle: bool,
    pub repeat: RepeatMode,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, specta::Type)]
pub struct QueueState {
    pub items: Vec<QueueItem>,
    pub current: Option<u32>,
}

#[derive(Clone, Debug, Serialize, specta::Type, tauri_specta::Event)]
pub struct PlayerChanged(pub PlayerState);

#[derive(Clone, Debug, Serialize, specta::Type, tauri_specta::Event)]
pub struct QueueChanged(pub QueueState);

/// State shared by the handle and the engine thread.
struct Shared {
    /// `status.current` is unused: [`Shared::player_state`] fills it from the queue.
    status: PlayerState,
    queue: Queue,
    /// Set on every queue change; the engine emits [`QueueChanged`] and clears it.
    queue_dirty: bool,
}

impl Shared {
    fn new() -> Self {
        Self {
            status: PlayerState {
                volume: 1.0,
                ..Default::default()
            },
            queue: Queue::default(),
            queue_dirty: false,
        }
    }

    fn player_state(&self) -> PlayerState {
        PlayerState {
            current: self.queue.current().map(|c| c as u32),
            ..self.status.clone()
        }
    }

    fn queue_changed(&mut self) {
        self.queue_dirty = true;
    }

    /// Index of the track to play after the current one; `None` when the queue is
    /// exhausted (see [`Shared::append_random`]).
    fn next_index(&self, auto: bool) -> Option<usize> {
        self.queue.next_index(
            self.status.shuffle,
            self.status.repeat,
            auto,
            fastrand::u64(..),
        )
    }

    /// Queue exhausted: appends a random library track and returns its index.
    fn append_random(&mut self, pick: &RandomPicker) -> Option<usize> {
        let item = pick(self.status.track_id)?;
        self.queue.append([item]);
        self.queue_changed();
        Some(self.queue.len() - 1)
    }
}

/// Managed state: the only way to talk to the engine.
pub struct AudioHandle {
    tx: Sender<AudioCommand>,
    shared: Arc<Mutex<Shared>>,
    pick_random: RandomPicker,
}

impl AudioHandle {
    /// Spawns the audio thread. `app` is used to emit [`PlayerChanged`] / [`QueueChanged`];
    /// every listen is handed to `record` when it ends.
    pub fn spawn(app: AppHandle, pick_random: RandomPicker, record: PlayRecorder) -> Self {
        let (tx, rx) = mpsc::channel();
        let shared = Arc::new(Mutex::new(Shared::new()));
        engine::spawn(
            app,
            rx,
            Arc::clone(&shared),
            Arc::clone(&pick_random),
            record,
        );
        Self {
            tx,
            shared,
            pick_random,
        }
    }

    pub fn state(&self) -> PlayerState {
        self.lock().player_state()
    }

    pub fn queue(&self) -> QueueState {
        self.lock().queue.state()
    }

    /// Replaces the queue with `items` and starts playing `items[start]`.
    pub fn play_queue(&self, items: Vec<QueueItem>, start: usize) -> AppResult<()> {
        {
            let mut s = self.local()?;
            s.queue.replace(items, start)?;
            s.queue_changed();
        }
        self.send(AudioCommand::PlayIndex(start))
    }

    pub fn enqueue(&self, items: Vec<QueueItem>) -> AppResult<()> {
        self.edit_queue(|q| {
            q.append(items);
            Ok(())
        })
    }

    pub fn play_index(&self, index: usize) -> AppResult<()> {
        self.local()?.queue.check_index(index)?;
        self.send(AudioCommand::PlayIndex(index))
    }

    /// The current track cannot be removed.
    pub fn remove_from_queue(&self, index: usize) -> AppResult<()> {
        self.edit_queue(|q| q.remove(index))
    }

    /// Removes every track except the current one.
    pub fn clear_queue(&self) -> AppResult<()> {
        self.edit_queue(|q| {
            q.clear_except_current();
            Ok(())
        })
    }

    /// Drops every queue entry of a track removed from the library (except the current one).
    pub fn forget_track(&self, track_id: TrackId) {
        let changed = {
            let mut s = self.lock();
            let changed = s.queue.forget_track(track_id);
            if changed {
                s.queue_changed();
            }
            changed
        };
        if changed {
            // Nothing to report if the engine is gone.
            let _ = self.send(AudioCommand::Publish);
        }
    }

    /// Resumes, or restarts the current/first track when stopped, or picks a random one.
    pub fn play(&self) -> AppResult<()> {
        let cmd = {
            let mut s = self.local()?;
            match s.queue.current() {
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

    pub fn pause(&self) -> AppResult<()> {
        self.local().map(drop)?;
        self.send(AudioCommand::Pause)
    }

    pub fn stop(&self) -> AppResult<()> {
        self.local().map(drop)?;
        self.send(AudioCommand::Stop)
    }

    /// Next queue track, or a random library track at the end of the queue.
    pub fn next(&self) -> AppResult<()> {
        let next = {
            let mut s = self.local()?;
            match s.next_index(false) {
                Some(next) => Some(next),
                None => s.append_random(&self.pick_random),
            }
        };
        match next {
            Some(index) => self.send(AudioCommand::PlayIndex(index)),
            None => Ok(()),
        }
    }

    /// Previous track, or restart the current one when past 3 s.
    pub fn previous(&self) -> AppResult<()> {
        let cmd = {
            let s = self.local()?;
            match s.queue.current() {
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

    pub fn set_shuffle(&self, shuffle: bool) -> AppResult<()> {
        self.local()?.status.shuffle = shuffle;
        self.send(AudioCommand::Publish)
    }

    pub fn set_repeat(&self, repeat: RepeatMode) -> AppResult<()> {
        self.local()?.status.repeat = repeat;
        self.send(AudioCommand::Publish)
    }

    pub fn seek(&self, secs: f64) -> AppResult<()> {
        self.local().map(drop)?;
        self.send(AudioCommand::Seek(clamp_position(secs)))
    }

    pub fn set_volume(&self, volume: f32) -> AppResult<()> {
        self.send(AudioCommand::SetVolume(volume.clamp(0.0, 1.0)))
    }

    /// Hands playback over to the group client. Local commands are refused right away.
    pub fn enter_group(&self, on_ended: GroupEndedCallback) -> AppResult<()> {
        // Set before the engine processes the switch so local commands fail at once.
        self.lock().status.group_mode = true;
        self.send(AudioCommand::EnterGroup(on_ended))
    }

    /// Stops group playback and restores the local queue (stopped).
    pub fn leave_group(&self) -> AppResult<()> {
        self.send(AudioCommand::LeaveGroup)
    }

    /// Loads `path` (unless `token` is already loaded), seeks to `position` and
    /// starts as `start` says. `token` identifies the group entry; `track` (the
    /// `PlayedTrack::Group` variant) describes it for the listening history.
    pub fn group_load(
        &self,
        token: u64,
        path: String,
        position: f64,
        start: GroupStart,
        track: PlayedTrack,
    ) -> AppResult<()> {
        self.send(AudioCommand::GroupLoad {
            token,
            path,
            track,
            position: clamp_position(position),
            start,
        })
    }

    pub fn group_stop(&self) -> AppResult<()> {
        self.send(AudioCommand::GroupStop)
    }

    fn lock(&self) -> MutexGuard<'_, Shared> {
        self.shared.lock().unwrap()
    }

    /// The shared state, or [`AppError::GroupMode`] while a group drives playback.
    fn local(&self) -> AppResult<MutexGuard<'_, Shared>> {
        let s = self.lock();
        if s.status.group_mode {
            return Err(AppError::group_mode());
        }
        Ok(s)
    }

    /// Applies a local queue edit and wakes the engine so it emits [`QueueChanged`].
    fn edit_queue(&self, f: impl FnOnce(&mut Queue) -> AppResult<()>) -> AppResult<()> {
        {
            let mut s = self.local()?;
            f(&mut s.queue)?;
            s.queue_changed();
        }
        self.send(AudioCommand::Publish)
    }

    fn send(&self, cmd: AudioCommand) -> AppResult<()> {
        self.tx
            .send(cmd)
            .map_err(|_| AppError::Internal("Le moteur audio n'est plus disponible".into()))
    }
}

/// Keeps a position (possibly from another group member) in a range `Duration` can
/// hold: NaN → 0, negative → 0, huge or infinite → [`MAX_POSITION_SECS`].
fn clamp_position(secs: f64) -> f64 {
    if secs.is_nan() {
        0.0
    } else {
        secs.clamp(0.0, MAX_POSITION_SECS)
    }
}
