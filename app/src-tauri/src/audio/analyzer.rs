//! Spectrum of what is playing, for the visualizer. [`Tap`] wraps every source appended to
//! the output and copies the mono mix into a lock-free [`Ring`]; the engine thread reads the
//! last [`WINDOW`] samples a few times per second and turns them into bands with [`Ring::bands`].

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering::Relaxed};
use std::time::Duration;

use rodio::source::SeekError;
use rodio::{ChannelCount, SampleRate, Source};

/// Samples analysed per refresh (power of two).
const WINDOW: usize = 1024;
const RING: usize = 4096;
/// Lowest / highest frequency shown, in Hz.
const LOW_HZ: f32 = 40.0;
const HIGH_HZ: f32 = 14_000.0;

pub(super) struct Ring {
    buf: Vec<AtomicU32>,
    pos: AtomicUsize,
    sample_rate: AtomicU32,
}

impl Ring {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            buf: (0..RING).map(|_| AtomicU32::new(0)).collect(),
            pos: AtomicUsize::new(0),
            sample_rate: AtomicU32::new(44_100),
        })
    }

    fn push(&self, sample: f32) {
        let pos = self.pos.load(Relaxed);
        self.buf[pos % RING].store(sample.to_bits(), Relaxed);
        self.pos.store(pos.wrapping_add(1), Relaxed);
    }

    /// `n` levels in 0..=1, log-spaced from [`LOW_HZ`] to [`HIGH_HZ`], of the latest samples.
    pub fn bands(&self, n: usize) -> Vec<f32> {
        let end = self.pos.load(Relaxed);
        let mut re = [0f32; WINDOW];
        let mut im = [0f32; WINDOW];
        let mut window_sum = 0.0;
        for (i, r) in re.iter_mut().enumerate() {
            let w = 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / WINDOW as f32).cos();
            let idx = end.wrapping_add(RING - WINDOW + i) % RING;
            *r = f32::from_bits(self.buf[idx].load(Relaxed)) * w;
            window_sum += w;
        }
        fft(&mut re, &mut im);

        let bin_hz = self.sample_rate.load(Relaxed) as f32 / WINDOW as f32;
        let edge = |k: usize| LOW_HZ * (HIGH_HZ / LOW_HZ).powf(k as f32 / n as f32);
        (0..n)
            .map(|k| {
                let lo = ((edge(k) / bin_hz) as usize).clamp(1, WINDOW / 2 - 1);
                let hi = ((edge(k + 1) / bin_hz) as usize).clamp(lo + 1, WINDOW / 2);
                let peak = (lo..hi)
                    .map(|b| 2.0 * re[b].hypot(im[b]) / window_sum)
                    .fold(0.0, f32::max);
                // dB, with a slight tilt: highs carry less energy than bass in music.
                let db = 20.0 * (peak + 1e-9).log10() + 18.0 * k as f32 / n as f32;
                ((db + 80.0) / 70.0).clamp(0.0, 1.0)
            })
            .collect()
    }
}

/// In-place radix-2 FFT (`re.len()` a power of two).
fn fft(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -std::f32::consts::TAU / len as f32;
        for start in (0..n).step_by(len) {
            for k in 0..len / 2 {
                let (s, c) = (ang * k as f32).sin_cos();
                let (a, b) = (start + k, start + k + len / 2);
                let (tr, ti) = (re[b] * c - im[b] * s, re[b] * s + im[b] * c);
                (re[b], im[b]) = (re[a] - tr, im[a] - ti);
                (re[a], im[a]) = (re[a] + tr, im[a] + ti);
            }
        }
        len <<= 1;
    }
}

/// Passes `inner` through unchanged while feeding its mono mix to a [`Ring`].
pub(super) struct Tap<S> {
    inner: S,
    ring: Arc<Ring>,
    channels: usize,
    acc: f32,
    count: usize,
}

impl<S: Source> Tap<S> {
    pub fn new(inner: S, ring: Arc<Ring>) -> Self {
        ring.sample_rate.store(inner.sample_rate().get(), Relaxed);
        Self {
            channels: inner.channels().get() as usize,
            inner,
            ring,
            acc: 0.0,
            count: 0,
        }
    }
}

impl<S: Source> Iterator for Tap<S> {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let sample = self.inner.next()?;
        self.acc += sample;
        self.count += 1;
        if self.count == self.channels {
            self.ring.push(self.acc / self.channels as f32);
            self.acc = 0.0;
            self.count = 0;
        }
        Some(sample)
    }
}

impl<S: Source> Source for Tap<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len()
    }

    fn channels(&self) -> ChannelCount {
        self.inner.channels()
    }

    fn sample_rate(&self) -> SampleRate {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.inner.try_seek(pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_lights_the_matching_band() {
        let ring = Ring::new();
        for i in 0..RING {
            let t = i as f32 / 44_100.0;
            ring.push(0.5 * (std::f32::consts::TAU * 1000.0 * t).sin());
        }
        let n = 48;
        let bands = ring.bands(n);
        let top = (0..n)
            .max_by(|&a, &b| bands[a].total_cmp(&bands[b]))
            .unwrap();
        let k = (1000f32 / LOW_HZ).ln() / (HIGH_HZ / LOW_HZ).ln() * n as f32;
        assert!(
            (top as f32 - k).abs() <= 2.0,
            "top band {top}, expected ~{k}"
        );
        let silent = Ring::new().bands(n);
        assert!(silent.iter().all(|&l| l == 0.0));
    }
}
