//! Lightweight, dependency-free frame-time statistics for Horizon Boost.
//!
//! This crate measures durations supplied by an integration layer. It does not
//! hook a renderer, infer GPU time, or equate rendered frames with presented
//! frames. The caller is responsible for supplying valid frame durations.

use std::time::Duration;

/// A bounded collection of recent frame durations.
#[derive(Debug, Clone)]
pub struct FrameWindow {
    samples: Vec<Duration>,
    capacity: usize,
}

impl FrameWindow {
    /// Create a frame window with room for at least one sample.
    pub fn new(capacity: usize) -> Self {
        Self {
            samples: Vec::with_capacity(capacity.max(1)),
            capacity: capacity.max(1),
        }
    }

    /// Record one completed frame duration.
    pub fn record(&mut self, frame_time: Duration) {
        if frame_time.is_zero() {
            return;
        }

        if self.samples.len() == self.capacity {
            self.samples.remove(0);
        }
        self.samples.push(frame_time);
    }

    /// Number of samples currently retained.
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Whether the window contains no samples.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Calculate statistics for the current window.
    pub fn stats(&self) -> Option<FrameStats> {
        if self.samples.is_empty() {
            return None;
        }

        let mut sorted = self.samples.clone();
        sorted.sort_unstable();

        let average = self.samples.iter().map(Duration::as_secs_f64).sum::<f64>()
            / self.samples.len() as f64;

        Some(FrameStats {
            samples: self.samples.len(),
            average_frame_time_ms: average * 1_000.0,
            fps_from_average: if average > 0.0 { 1.0 / average } else { 0.0 },
            p50_frame_time_ms: percentile(&sorted, 0.50).as_secs_f64() * 1_000.0,
            p95_frame_time_ms: percentile(&sorted, 0.95).as_secs_f64() * 1_000.0,
            p99_frame_time_ms: percentile(&sorted, 0.99).as_secs_f64() * 1_000.0,
        })
    }
}

fn percentile(sorted: &[Duration], percentile: f64) -> Duration {
    let index = ((sorted.len() - 1) as f64 * percentile).ceil() as usize;
    sorted[index.min(sorted.len() - 1)]
}

/// Summary of measured frame durations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameStats {
    pub samples: usize,
    pub average_frame_time_ms: f64,
    pub fps_from_average: f64,
    pub p50_frame_time_ms: f64,
    pub p95_frame_time_ms: f64,
    pub p99_frame_time_ms: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_average_fps_and_percentiles() {
        let mut window = FrameWindow::new(10);
        for ms in [10, 20, 30, 40, 50] {
            window.record(Duration::from_millis(ms));
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
        window.record(Duration::from_millis(10));
        window.record(Duration::from_millis(20));
        window.record(Duration::from_millis(30));

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
}
