//! Maps server time to local time from Ping/Pong round trips (median offset). Pure, unit-tested.

use std::collections::VecDeque;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAX_SAMPLES: usize = 15;

/// Local times are milliseconds since `base` (monotonic, so wall-clock jumps on
/// this machine don't matter); `offset` = server ms − local ms.
pub struct Clock {
    base: Instant,
    samples: VecDeque<f64>,
    /// Used until the first pong: assumes both wall clocks agree.
    fallback: f64,
}

impl Clock {
    pub fn new() -> Self {
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

    pub fn local_ms(&self, at: Instant) -> f64 {
        at.saturating_duration_since(self.base).as_secs_f64() * 1000.0
    }

    /// A pong for a ping sent at local `sent_ms`, received at local `received_ms`.
    /// The server read its clock roughly halfway through the round trip.
    pub fn add_sample(&mut self, sent_ms: f64, received_ms: f64, server_ms: f64) {
        if received_ms < sent_ms {
            return;
        }
        self.samples
            .push_back(server_ms - (sent_ms + received_ms) / 2.0);
        if self.samples.len() > MAX_SAMPLES {
            self.samples.pop_front();
        }
    }

    fn offset(&self) -> f64 {
        median(self.samples.iter().copied()).unwrap_or(self.fallback)
    }

    /// The local instant at which the server clock reads `server_ms`.
    pub fn to_instant(&self, server_ms: u64) -> Instant {
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
    Some(if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_of_samples() {
        assert_eq!(median([].into_iter()), None);
        assert_eq!(median([5.0].into_iter()), Some(5.0));
        assert_eq!(median([9.0, 1.0, 5.0].into_iter()), Some(5.0));
        assert_eq!(median([4.0, 1.0, 3.0, 2.0].into_iter()), Some(2.5));
        assert_eq!(median([1.0, f64::NAN, 3.0].into_iter()), Some(2.0));
    }

    #[test]
    fn offset_ignores_outliers() {
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
    fn keeps_the_latest_samples() {
        let mut clock = Clock::new();
        for i in 0..20 {
            clock.add_sample(0.0, 0.0, f64::from(i));
        }
        assert_eq!(clock.samples.len(), MAX_SAMPLES);
        assert_eq!(clock.samples.front(), Some(&5.0));
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
    fn falls_back_to_wall_clock() {
        let clock = Clock::new();
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let at = clock.to_instant(now_ms + 1000);
        let delta = at.saturating_duration_since(Instant::now());
        assert!(delta > Duration::from_millis(900) && delta <= Duration::from_millis(1001));
    }
}
