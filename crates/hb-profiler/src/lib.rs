//! Lightweight, dependency-free frame-time statistics for Horizon Boost.
//!
//! This crate measures durations supplied by an integration layer. It does not
//! hook a renderer, infer GPU time, or equate rendered frames with presented
//! frames. The caller is responsible for supplying valid frame durations.

use std::collections::VecDeque;
use std::time::Duration;

/// A bounded collection of recent frame durations.
///
/// Once full, recording a frame discards the oldest one. Recording is O(1);
/// computing [`FrameStats`] is O(n log n) in the number of retained samples,
/// so call [`FrameWindow::stats`] when reporting, not on every frame.
#[derive(Debug, Clone)]
pub struct FrameWindow {
    samples: VecDeque<Duration>,
    capacity: usize,
}

impl FrameWindow {
    /// Create a frame window with room for at least one sample.
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            samples: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Record one completed frame duration. Zero durations are ignored.
    pub fn record(&mut self, frame_time: Duration) {
        if frame_time.is_zero() {
            return;
        }

        if self.samples.len() == self.capacity {
            self.samples.pop_front();
        }
        self.samples.push_back(frame_time);
    }

    /// Maximum number of samples the window retains.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Number of samples currently retained.
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Whether the window contains no samples.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Discard all samples, keeping the capacity.
    pub fn clear(&mut self) {
        self.samples.clear();
    }

    /// Calculate statistics for the current window.
    pub fn stats(&self) -> Option<FrameStats> {
        if self.samples.is_empty() {
            return None;
        }

        let mut sorted: Vec<Duration> = self.samples.iter().copied().collect();
        sorted.sort_unstable();

        let average =
            self.samples.iter().map(Duration::as_secs_f64).sum::<f64>() / self.samples.len() as f64;

        // The slowest 1% of frames (at least one), averaged as frame time and
        // reported as FPS. This is the "1% low" commonly quoted for frame
        // spikes; definitions vary between tools, so this one is fixed here.
        let slowest = (sorted.len() as f64 * 0.01).ceil().max(1.0) as usize;
        let tail = &sorted[sorted.len() - slowest..];
        let tail_average = tail.iter().map(Duration::as_secs_f64).sum::<f64>() / tail.len() as f64;

        Some(FrameStats {
            samples: self.samples.len(),
            average_frame_time_ms: average * 1_000.0,
            fps_from_average: if average > 0.0 { 1.0 / average } else { 0.0 },
            p50_frame_time_ms: percentile(&sorted, 0.50).as_secs_f64() * 1_000.0,
            p95_frame_time_ms: percentile(&sorted, 0.95).as_secs_f64() * 1_000.0,
            p99_frame_time_ms: percentile(&sorted, 0.99).as_secs_f64() * 1_000.0,
            max_frame_time_ms: sorted[sorted.len() - 1].as_secs_f64() * 1_000.0,
            one_percent_low_fps: if tail_average > 0.0 {
                1.0 / tail_average
            } else {
                0.0
            },
        })
    }
}

/// Nearest-rank style percentile: the sample at index `ceil((n - 1) * p)`.
fn percentile(sorted: &[Duration], percentile: f64) -> Duration {
    let index = ((sorted.len() - 1) as f64 * percentile).ceil() as usize;
    sorted[index.min(sorted.len() - 1)]
}

/// Summary of measured frame durations.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct FrameStats {
    pub samples: usize,
    pub average_frame_time_ms: f64,
    pub fps_from_average: f64,
    pub p50_frame_time_ms: f64,
    pub p95_frame_time_ms: f64,
    pub p99_frame_time_ms: f64,
    /// The slowest frame in the window.
    pub max_frame_time_ms: f64,
    /// FPS equivalent of the mean frame time of the slowest 1% of frames.
    pub one_percent_low_fps: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    #[test]
    fn computes_average_fps_and_percentiles() {
        let mut window = FrameWindow::new(10);
        for value in [10, 20, 30, 40, 50] {
            window.record(ms(value));
        }

        let stats = window.stats().unwrap();
        assert_eq!(stats.samples, 5);
        assert!((stats.average_frame_time_ms - 30.0).abs() < 0.001);
        assert!((stats.fps_from_average - (1.0 / 0.03)).abs() < 0.001);
        assert_eq!(stats.p50_frame_time_ms, 30.0);
        assert_eq!(stats.p95_frame_time_ms, 50.0);
        assert_eq!(stats.p99_frame_time_ms, 50.0);
    }

    #[test]
    fn keeps_only_the_newest_samples() {
        let mut window = FrameWindow::new(2);
        window.record(ms(10));
        window.record(ms(20));
        window.record(ms(30));

        let stats = window.stats().unwrap();
        assert_eq!(stats.samples, 2);
        assert_eq!(stats.average_frame_time_ms, 25.0);
    }

    #[test]
    fn ignores_zero_duration_samples() {
        let mut window = FrameWindow::new(4);
        window.record(Duration::ZERO);
        assert!(window.is_empty());
        assert!(window.stats().is_none());
    }

    #[test]
    fn zero_capacity_still_holds_one_sample() {
        let mut window = FrameWindow::new(0);
        assert_eq!(window.capacity(), 1);
        window.record(ms(5));
        window.record(ms(8));
        assert_eq!(window.len(), 1);
        assert_eq!(window.stats().unwrap().max_frame_time_ms, 8.0);
    }

    #[test]
    fn clear_empties_the_window_and_keeps_capacity() {
        let mut window = FrameWindow::new(3);
        window.record(ms(5));
        window.clear();
        assert!(window.is_empty());
        assert_eq!(window.capacity(), 3);
        window.record(ms(7));
        assert_eq!(window.len(), 1);
    }

    #[test]
    fn matches_a_naive_reference_across_many_wraparounds() {
        // Deterministic pseudo-random durations, no dependency needed.
        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            1 + state % 50_000
        };

        for capacity in [1, 2, 7, 64] {
            let mut window = FrameWindow::new(capacity);
            let mut reference: Vec<Duration> = Vec::new();
            for _ in 0..(capacity * 5 + 3) {
                let sample = Duration::from_micros(next());
                window.record(sample);
                reference.push(sample);
                if reference.len() > capacity {
                    reference.remove(0);
                }

                let mut sorted = reference.clone();
                sorted.sort_unstable();
                let stats = window.stats().unwrap();
                assert_eq!(stats.samples, reference.len());
                assert_eq!(
                    stats.p50_frame_time_ms,
                    percentile(&sorted, 0.50).as_secs_f64() * 1_000.0
                );
                assert_eq!(
                    stats.max_frame_time_ms,
                    sorted.last().unwrap().as_secs_f64() * 1_000.0
                );
            }
        }
    }

    #[test]
    fn reports_max_and_one_percent_low_for_a_spike() {
        let mut window = FrameWindow::new(100);
        for _ in 0..99 {
            window.record(ms(10));
        }
        window.record(ms(110));

        let stats = window.stats().unwrap();
        assert_eq!(stats.max_frame_time_ms, 110.0);
        // 100 samples: the slowest 1% is the single 110 ms frame.
        assert!((stats.one_percent_low_fps - 1.0 / 0.110).abs() < 0.001);
        // The average hides the spike, which is the point of the 1% low.
        assert!(stats.fps_from_average > 80.0);
    }

    #[test]
    fn one_percent_low_uses_at_least_one_frame_for_small_windows() {
        let mut window = FrameWindow::new(10);
        window.record(ms(10));
        window.record(ms(40));

        let stats = window.stats().unwrap();
        assert!((stats.one_percent_low_fps - 25.0).abs() < 0.001);
    }
}
