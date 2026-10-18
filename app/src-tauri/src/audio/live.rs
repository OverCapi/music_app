//! Live stream playback. A live has no end and no file, so it is streamed:
//! `yt-dlp -o -` writes the stream to a pipe, `ffmpeg` decodes it to raw f32 PCM, and
//! a thread feeds a small buffer (a few seconds) that [`LiveSource`] plays. An empty
//! buffer plays silence rather than ending the source.
//!
//! The processes die when the source is dropped (pause, another track, app exit). If
//! the stream breaks (network, expiring HLS URL) it is restarted a few times before
//! giving up with an error.

use std::io::Read;
use std::process::{Child, ChildStdout, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use rodio::{ChannelCount, Sample, SampleRate, Source};

use crate::error::{AppError, AppResult};
use crate::ytdlp;

const SAMPLE_RATE: u32 = 48_000;
const CHANNELS: u16 = 2;
/// 0.1 s of stereo f32 PCM.
const CHUNK_BYTES: usize = SAMPLE_RATE as usize / 10 * CHANNELS as usize * 4;
/// Buffered chunks: 4 s. When full, the pipe fills up and yt-dlp waits.
const BUFFERED_CHUNKS: usize = 40;
/// Restarts in a row without receiving any audio before giving up.
const MAX_FAILURES: u32 = 3;
const RETRY_DELAY: Duration = Duration::from_secs(2);

/// Why the stream stopped for good; set by the feeder thread, read by the engine.
pub type LiveError = Arc<Mutex<Option<String>>>;

#[derive(Default)]
struct Control {
    stop: AtomicBool,
    /// The running yt-dlp and ffmpeg processes.
    children: Mutex<Vec<Child>>,
}

impl Control {
    /// Registers a process, unless the source was dropped meanwhile (then it is killed).
    fn track(&self, child: Child) -> bool {
        let mut children = self.children.lock().unwrap();
        if self.stop.load(Ordering::SeqCst) {
            kill(child);
            return false;
        }
        children.push(child);
        true
    }

    fn kill_all(&self) {
        let children = std::mem::take(&mut *self.children.lock().unwrap());
        children.into_iter().for_each(kill);
    }
}

/// Kills a process and what it started (yt-dlp has its own process group, because it
/// may run ffmpeg itself).
fn kill(mut child: Child) {
    // Also kills ffmpeg when yt-dlp started it: the whole process group on unix, the tree on Windows.
    #[cfg(unix)]
    let _ = std::process::Command::new("kill")
        .args(["-KILL", "--", &format!("-{}", child.id())])
        .stderr(Stdio::null())
        .status();
    #[cfg(windows)]
    let _ = ytdlp::tool_command("taskkill")
        .args(["/F", "/T", "/PID", &child.id().to_string()])
        .stderr(Stdio::null())
        .status();
    let _ = child.kill();
    let _ = child.wait();
}

pub struct LiveSource {
    rx: Receiver<Vec<Sample>>,
    chunk: Vec<Sample>,
    pos: usize,
    control: Arc<Control>,
}

impl LiveSource {
    /// Starts streaming the live `youtube_id` (validated). Never blocks: the processes
    /// are started by a thread, whose final failure shows up in the returned [`LiveError`].
    pub fn start(youtube_id: &str) -> AppResult<(Self, LiveError)> {
        // Fails on an invalid id before anything is spawned.
        ytdlp::live_audio_command(youtube_id)?;
        let (tx, rx) = sync_channel(BUFFERED_CHUNKS);
        let control = Arc::new(Control::default());
        let error = LiveError::default();
        let id = youtube_id.to_owned();
        thread::Builder::new()
            .name("live".into())
            .spawn({
                let (control, error) = (Arc::clone(&control), Arc::clone(&error));
                move || {
                    if let Err(message) = feed(&id, &tx, &control) {
                        *error.lock().unwrap() = Some(message);
                    }
                }
            })
            .map_err(|e| AppError::Internal(format!("impossible de lancer le direct : {e}")))?;
        let source = Self {
            rx,
            chunk: Vec::new(),
            pos: 0,
            control,
        };
        Ok((source, error))
    }
}

impl Drop for LiveSource {
    fn drop(&mut self) {
        self.control.stop.store(true, Ordering::SeqCst);
        self.control.kill_all();
    }
}

/// Streams until the source is dropped; `Err` is the message to show when it can't continue.
fn feed(id: &str, tx: &SyncSender<Vec<Sample>>, control: &Control) -> Result<(), String> {
    let mut failures = 0;
    while !control.stop.load(Ordering::SeqCst) {
        let Some(received) = stream_once(id, tx, control)? else {
            return Ok(());
        };
        failures = if received { 0 } else { failures + 1 };
        if failures >= MAX_FAILURES {
            return Err("Le direct est interrompu".into());
        }
        tracing::warn!("live {id} interrupted, reconnecting");
        thread::sleep(RETRY_DELAY);
    }
    Ok(())
}

/// Runs the pipeline until it ends. `Ok(Some(received_audio))`, or `Ok(None)` when the
/// source is gone.
fn stream_once(
    id: &str,
    tx: &SyncSender<Vec<Sample>>,
    control: &Control,
) -> Result<Option<bool>, String> {
    let mut ytdlp = ytdlp::live_audio_command(id).map_err(|e| e.to_string())?;
    ytdlp
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null());
    // Own process group, so `kill` takes ffmpeg down with yt-dlp.
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut ytdlp, 0);
    let mut ytdlp = ytdlp
        .spawn()
        .map_err(|e| ytdlp::spawn_error(e).to_string())?;
    let audio = ytdlp.stdout.take().expect("stdout is piped");
    if !control.track(ytdlp) {
        return Ok(None);
    }
    let mut ffmpeg =
        ytdlp::tool_command(ytdlp::find_in_path("ffmpeg").unwrap_or_else(|| "ffmpeg".into()))
            .args(["-v", "quiet", "-i", "pipe:0", "-vn", "-f", "f32le"])
            .args([
                "-ac",
                &CHANNELS.to_string(),
                "-ar",
                &SAMPLE_RATE.to_string(),
            ])
            .arg("pipe:1")
            .stdin(Stdio::from(audio))
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => "ffmpeg est introuvable".to_owned(),
                _ => format!("Impossible de lancer ffmpeg : {e}"),
            })?;
    let pcm = ffmpeg.stdout.take().expect("stdout is piped");
    if !control.track(ffmpeg) {
        return Ok(None);
    }
    let received = forward(pcm, tx);
    control.kill_all();
    Ok((!control.stop.load(Ordering::SeqCst)).then_some(received))
}

/// Cuts the PCM stream into chunks for the player; true if any audio arrived.
fn forward(mut pcm: ChildStdout, tx: &SyncSender<Vec<Sample>>) -> bool {
    let mut bytes = vec![0; CHUNK_BYTES];
    let mut received = false;
    loop {
        let mut filled = 0;
        while filled < CHUNK_BYTES {
            match pcm.read(&mut bytes[filled..]) {
                Ok(0) | Err(_) => return received,
                Ok(n) => filled += n,
            }
        }
        received = true;
        let samples = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| Sample::from_le_bytes(*b))
            .collect();
        if tx.send(samples).is_err() {
            return received;
        }
    }
}

impl Iterator for LiveSource {
    type Item = Sample;

    fn next(&mut self) -> Option<Sample> {
        if self.pos >= self.chunk.len() {
            match self.rx.try_recv() {
                Ok(chunk) => {
                    self.chunk = chunk;
                    self.pos = 0;
                }
                // Buffer empty (or stream down): silence, never the end of the source.
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return Some(0.0),
            }
        }
        let sample = self.chunk[self.pos];
        self.pos += 1;
        Some(sample)
    }
}

impl Source for LiveSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        ChannelCount::new(CHANNELS).expect("non-zero")
    }

    fn sample_rate(&self) -> SampleRate {
        SampleRate::new(SAMPLE_RATE).expect("non-zero")
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> (LiveSource, SyncSender<Vec<Sample>>) {
        let (tx, rx) = sync_channel(2);
        let source = LiveSource {
            rx,
            chunk: Vec::new(),
            pos: 0,
            control: Arc::default(),
        };
        (source, tx)
    }

    #[test]
    fn plays_chunks_then_silence_when_empty() {
        let (mut s, tx) = source();
        assert_eq!(s.next(), Some(0.0));
        tx.send(vec![0.5, 0.25]).unwrap();
        tx.send(vec![1.0]).unwrap();
        let heard: Vec<_> = s.by_ref().take(4).collect();
        assert_eq!(heard, [0.5, 0.25, 1.0, 0.0]);
        drop(tx);
        // A dead feeder never ends the source: the engine reacts to the error instead.
        assert_eq!(s.next(), Some(0.0));
    }

    #[test]
    fn invalid_id_does_not_start() {
        assert!(LiveSource::start("not an id").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn dropping_kills_the_processes() {
        use std::process::Command;
        let control = Arc::new(Control::default());
        let child = Command::new("sleep").arg("30").spawn().unwrap();
        let pid = child.id();
        assert!(control.track(child));
        let (mut s, _tx) = source();
        s.control = Arc::clone(&control);
        drop(s);
        let alive = Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success();
        assert!(!alive);
        // Too late to register another process.
        let late = Command::new("sleep").arg("30").spawn().unwrap();
        assert!(!control.track(late));
    }
}
