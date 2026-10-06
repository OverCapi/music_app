//! Audio engine: a dedicated thread owns the output device and the rodio `Player`.
//! The rest of the app talks to it through `AudioHandle` (command channel + shared state).
//!
//! The play queue lives in the shared state so the audio thread can advance to the
//! next track on its own when one ends.

use std::fs::File;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use lofty::prelude::*;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use serde::Serialize;

use crate::opus::OpusSource;

type TrackSource = Box<dyn Source + Send>;

/// "Previous" restarts the current track instead when we're past this point.
const RESTART_THRESHOLD_SECS: f64 = 3.0;

enum AudioCommand {
    /// Load and play `queue[index]` (skipping unreadable files).
    PlayIndex(usize),
    Play,
    Pause,
    Stop,
    Seek(f64),
    SetVolume(f32),
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
        }
    }
}

#[derive(Clone, Serialize)]
pub struct QueueState {
    pub tracks: Vec<String>,
    pub current: Option<usize>,
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
        }
    }
}

pub struct AudioHandle {
    tx: Sender<AudioCommand>,
    shared: Arc<Mutex<Shared>>,
}

impl AudioHandle {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel();
        let shared = Arc::new(Mutex::new(Shared::default()));
        let thread_shared = Arc::clone(&shared);
        thread::Builder::new()
            .name("audio".into())
            .spawn(move || audio_thread(rx, thread_shared))
            .expect("failed to spawn audio thread");
        Self { tx, shared }
    }

    pub fn state(&self) -> PlayerState {
        self.shared.lock().unwrap().status.clone()
    }

    pub fn queue(&self) -> QueueState {
        self.shared.lock().unwrap().queue_state()
    }

    /// Replaces the queue with `tracks` and starts playing `tracks[start]`.
    pub fn play_queue(&self, tracks: Vec<String>, start: usize) -> Result<QueueState, String> {
        if start >= tracks.len() {
            return Err("Index de départ invalide".to_owned());
        }
        let queue = {
            let mut s = self.shared.lock().unwrap();
            s.queue = tracks;
            s.status.current = Some(start);
            s.queue_state()
        };
        self.send(AudioCommand::PlayIndex(start))?;
        Ok(queue)
    }

    pub fn enqueue(&self, tracks: Vec<String>) -> QueueState {
        let mut s = self.shared.lock().unwrap();
        s.queue.extend(tracks);
        s.queue_state()
    }

    pub fn play_index(&self, index: usize) -> Result<(), String> {
        if index >= self.shared.lock().unwrap().queue.len() {
            return Err("Ce titre n'est plus dans la file".to_owned());
        }
        self.send(AudioCommand::PlayIndex(index))
    }

    pub fn remove_from_queue(&self, index: usize) -> Result<QueueState, String> {
        let mut s = self.shared.lock().unwrap();
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
        Ok(s.queue_state())
    }

    /// Removes every track except the current one.
    pub fn clear_queue(&self) -> QueueState {
        let mut s = self.shared.lock().unwrap();
        match s.status.current {
            Some(c) => {
                let current = s.queue.swap_remove(c);
                s.queue = vec![current];
                s.status.current = Some(0);
            }
            None => s.queue.clear(),
        }
        s.queue_state()
    }

    pub fn play(&self) -> Result<(), String> {
        let cmd = {
            let s = self.shared.lock().unwrap();
            match s.status.current {
                _ if s.status.loaded => AudioCommand::Play,
                // Stopped or queue finished: restart the current track.
                Some(c) => AudioCommand::PlayIndex(c),
                None if !s.queue.is_empty() => AudioCommand::PlayIndex(0),
                None => return Ok(()),
            }
        };
        self.send(cmd)
    }

    pub fn next(&self) -> Result<(), String> {
        let cmd = {
            let s = self.shared.lock().unwrap();
            let next = s.status.current.map_or(0, |c| c + 1);
            if next >= s.queue.len() {
                return Ok(());
            }
            AudioCommand::PlayIndex(next)
        };
        self.send(cmd)
    }

    pub fn previous(&self) -> Result<(), String> {
        let cmd = {
            let s = self.shared.lock().unwrap();
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
        self.send(AudioCommand::Pause)
    }

    pub fn stop(&self) -> Result<(), String> {
        self.send(AudioCommand::Stop)
    }

    pub fn seek(&self, secs: f64) -> Result<(), String> {
        self.send(AudioCommand::Seek(secs.max(0.0)))
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

fn audio_thread(rx: Receiver<AudioCommand>, shared: Arc<Mutex<Shared>>) {
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
    };

    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(cmd) => engine.handle(cmd),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
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
}

impl Engine {
    fn handle(&mut self, cmd: AudioCommand) {
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
        if p.empty() {
            // End of track: advance through the queue.
            let next = self
                .shared
                .lock()
                .unwrap()
                .status
                .current
                .map_or(0, |c| c + 1);
            self.load_from(next);
        } else {
            let mut s = self.shared.lock().unwrap();
            s.status.playing = !p.is_paused();
            s.status.position = p.get_pos().as_secs_f64();
        }
    }
}
