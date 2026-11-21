//! Measures the cost of `FrameWindow::record` once the window is full.
//!
//! Run with `cargo run --release -p hb-profiler --example record_cost`.
//! Numbers depend on the machine; compare runs on the same machine only.

use std::hint::black_box;
use std::time::{Duration, Instant};

use hb_profiler::FrameWindow;

fn main() {
    const RECORDS: u32 = 200_000;
    for capacity in [120, 1_000, 10_000] {
        let mut window = FrameWindow::new(capacity);
        for i in 0..capacity {
            window.record(Duration::from_micros(1_000 + (i as u64 % 7)));
        }

        let start = Instant::now();
        for i in 0..RECORDS {
            window.record(black_box(Duration::from_micros(1_000 + (i as u64 % 7))));
        }
        let elapsed = start.elapsed();
        black_box(window.len());

        println!(
            "capacity {capacity:>6}: {:>8.1} ns/record ({RECORDS} records)",
            elapsed.as_nanos() as f64 / f64::from(RECORDS)
        );
    }
}
