//! Timing of one listen, from the moment a track is loaded until it is replaced,
//! stopped or over. Pure (instants are passed in) so it is tested without rodio;
//! the engine calls it on every play/pause transition.

use std::time::{Duration, Instant};

use crate::history::{FinishedPlay, PlaySource, PlayedTrack};

pub(super) struct Listen {
    track: PlayedTrack,
    source: PlaySource,
    /// Unix timestamp (seconds) of the load.
    started_at: i64,
    duration: f64,
    /// Time heard before the current playing stretch.
    heard: Duration,
    /// Start of the current playing stretch; `None` while paused.
    playing_since: Option<Instant>,
}

impl Listen {
    /// A listen of a freshly loaded track, paused until [`Listen::resume`].
    pub fn new(track: PlayedTrack, source: PlaySource, started_at: i64, duration: f64) -> Self {
        Self {
            track,
            source,
            started_at,
            duration,
            heard: Duration::ZERO,
            playing_since: None,
        }
    }

    /// The track is playing from `now` on (no-op if it already was).
    pub fn resume(&mut self, now: Instant) {
        self.playing_since.get_or_insert(now);
    }

    /// The track stopped playing at `now` (no-op if it already was paused).
    pub fn pause(&mut self, now: Instant) {
        if let Some(since) = self.playing_since.take() {
            self.heard += now.saturating_duration_since(since);
        }
    }

    /// Ends the listen at `now`; `completed` when the track reached its end.
    pub fn finish(mut self, now: Instant, completed: bool) -> FinishedPlay {
        self.pause(now);
        FinishedPlay {
            track: self.track,
            source: self.source,
            started_at: self.started_at,
            listened: self.heard.as_secs_f64(),
            duration: self.duration,
            completed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listen() -> Listen {
        Listen::new(PlayedTrack::Library(7), PlaySource::Playlist, 1000, 180.0)
    }

    fn secs(s: u64) -> Duration {
        Duration::from_secs(s)
    }

    #[test]
    fn finish_copies_the_track_info() {
        let t0 = Instant::now();
        let mut l = listen();
        l.resume(t0);
        let play = l.finish(t0 + secs(30), true);
        assert_eq!(
            play,
            FinishedPlay {
                track: PlayedTrack::Library(7),
                source: PlaySource::Playlist,
                started_at: 1000,
                listened: 30.0,
                duration: 180.0,
                completed: true,
            }
        );
    }

    #[test]
    fn never_resumed_counts_nothing() {
        let t0 = Instant::now();
        let play = listen().finish(t0 + secs(60), false);
        assert_eq!(play.listened, 0.0);
        assert!(!play.completed);
    }

    #[test]
    fn pauses_are_excluded() {
        let t0 = Instant::now();
        let mut l = listen();
        l.resume(t0);
        l.pause(t0 + secs(10));
        l.resume(t0 + secs(25));
        l.pause(t0 + secs(30));
        assert_eq!(l.finish(t0 + secs(100), false).listened, 15.0);
    }

    #[test]
    fn finish_while_playing_counts_up_to_now() {
        let t0 = Instant::now();
        let mut l = listen();
        l.resume(t0);
        l.pause(t0 + secs(5));
        l.resume(t0 + secs(20));
        assert_eq!(l.finish(t0 + secs(27), false).listened, 12.0);
    }

    #[test]
    fn resume_twice_and_pause_while_paused_are_no_ops() {
        let t0 = Instant::now();
        let mut l = listen();
        l.pause(t0);
        l.resume(t0 + secs(1));
        // Already playing: keeps counting from the first resume.
        l.resume(t0 + secs(5));
        l.pause(t0 + secs(11));
        l.pause(t0 + secs(50));
        assert_eq!(l.finish(t0 + secs(60), false).listened, 10.0);
    }

    #[test]
    fn instants_out_of_order_never_go_negative() {
        let t0 = Instant::now();
        let mut l = listen();
        l.resume(t0 + secs(10));
        l.pause(t0);
        assert_eq!(l.finish(t0, false).listened, 0.0);
    }
}
