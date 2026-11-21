# Architecture map

Findings from reading the public Horizon and Starling sources on 2026-11-21. Everything here is
from source and docs I read; nothing was run against the game, and no Horizon Boost code is
integrated with either project yet. Items marked **not verified** are open questions.

Revisions read: `horizon-rust/horizon` `main` at `1bead00f0c49`, `starling-client/starling`
branch `starling` at `ff303502a4f7`.

## 1. What Horizon and Starling are

- **Horizon** is a from-scratch Rust implementation of Minecraft Java 26.3 (`README.md`). It is a
  large Cargo workspace (about 70 crates under `crates/`) with its own wgpu renderer, a mod API
  (`horizon-mod-*`, `horizon-hooks`, WebAssembly and native mods) and an existing performance
  platform. It is not a wrapper around the Java engine and not Bedrock.
- **Starling** is a fork of Horizon for Android. Its crate list is Horizon's plus `horizon-android`,
  `starling-account` and `starling-catalog`. `horizon-android` is a `cdylib` whose source is
  `horizon-client/src/android_lib.rs`; Android hooks live in `horizon-client` (`src/android.rs`).
  Its default branch is `starling`. `SYNC.md` shows it is periodically merged with upstream, so
  shared code drifts and has to be re-checked per revision.

## 2. Shared versus platform-specific

| Area | Where | Notes |
|---|---|---|
| Shared: gameplay, world, protocol | `horizon-client`, `horizon-world`, `horizon-protocol`, `horizon-core` | Same crates in both repos. |
| Shared: renderer | `horizon-client-render` (514 files in Horizon), `src/gfx/` | wgpu. GPU culling, occlusion, LOD, upload and in-flight tracking, frame policy. |
| Shared: perf tooling | `horizon-perf`, `horizon-perf-format`, `horizon-bench`, `horizon-system`, `horizon-threads` | `.hzperf` captures, benchmark reports, hardware probes, allocator and heap stats. |
| Shared: mod API | `horizon-mod-*`, `horizon-hooks`, `horizon-platform` | `horizon-platform` is documented as the public API for the Horizon layer, mods and ports. |
| PC-specific | `launcher/`, `packaging/{windows,macos,linux}`, Windows-only modules in `horizon-threads` | Windows, macOS and Linux per Horizon's README. |
| Android-specific | `crates/horizon-android`, `packaging/android`, `target_os = "android"` code | In Starling, `cfg(target_os = "android")` appears in `horizon-client` (55 files), `horizon-client-render` (19), `horizon-threads` (3), plus a few others. |

## 3. Rendering and frame pipeline (as found)

- Game loop, tick order and frame pacing: `horizon-client/src/game.rs`,
  `game/runtime/window_handler.rs`, `clock.rs`, `horizon-client-render/src/spans.rs`
  (`docs/architecture.md`, area 1).
- Renderer core: `horizon-client-render/src/gfx.rs` and `gfx/` (`cull.rs`, `gpu_culling.rs`,
  `occlusion.rs`, `frustum.rs`, `lod.rs`, `upload.rs`, `in_flight.rs`, `frame_policy.rs`,
  `entity_batch.rs`, `translucency.rs`, `timing.rs`).
- GPU time: `gfx/timing.rs` reads timestamp queries a few frames late so the CPU never waits, and
  skips frames whose readback slot is busy. This is a GPU-time source Horizon Boost does not need
  to rebuild.
- Chunk streaming, light and tickets: `streaming.rs`, `world.rs`, `chunk.rs`, `chunk_map.rs`,
  `light.rs` in `horizon-client-render`.
- Allocator and threads: `horizon-threads` (mimalloc dependency, `jobs`, `alloc_count`,
  `allocator_stats`, `heap_census`, `process_memory`).

## 4. Existing performance work (do not duplicate)

- Horizon already ships a perf platform (`docs/perf-platform.md`, `.hzperf` captures,
  `horizon-perf` reader, `benchmarks/`, `docs/benchmark.md`). Horizon Boost should consume or
  complement these, not rebuild them. Format details: **not verified**.
- `docs/starling-performance.md` (Starling, measurements dated 2026-10-28 on one POCO F6 Pro, so
  treat as a single data point) records:
  - GPU occlusion culling is off on Android on purpose (no measured GPU saving on Adreno 740).
  - A no-`discard` opaque terrain experiment saved about 8% GPU time and was not merged.
  - At render distance 16, roughly 6 ms of GPU time did not scale with resolution.
  - Phones are capped at 60 fps by `apply_mobile_defaults`; ADPF is told a 120 fps target even when
    uncapped, so the CPU clock stayed low.
  - 1% lows stayed between 10 and 47 fps with 13 to 29 ms worst frames, and the spikes were
    "not looked into yet". This is the gap the profiler change below starts to serve.

## 5. Candidate integration points

Prefer supported surfaces over patching engine internals.

1. **Offline analysis of existing captures** (`.hzperf`, benchmark JSON in `benchmarks/`): lowest
   risk, works for PC and Android captures. Needs the format read first.
2. **Frame-time feed**: Horizon Boost's `hb-profiler` takes durations from whatever the game
   already measures (frame spans, `timing.rs`). Needs a call site decided with the Horizon team.
3. **Mod API / `horizon-platform`**: a supported place for a mod-shaped consumer. Whether it
   exposes frame timing: **not verified**.

## 6. Bottlenecks that are documented (not independently measured here)

- Android terrain GPU cost at long render distance (Starling notes above).
- Frame spikes on Android (1% lows).
- Per-frame bursts of chunk packets and mesh uploads; Starling notes describe another fork bounding
  these per frame. Whether upstream already has an upload budget: **not verified**.

## 7. Compatibility concerns and risks

- Code is shared but forked: any hook written against Horizon may not match Starling's revision.
  Pin revisions before integrating (as the README already says).
- Horizon's crates generally forbid unsafe code (per the `horizon-threads` doc comment) and the
  games load mod code. Native hooks that bypass the mod API would be fragile.
- Android: thermal limits, ADPF, driver variance (Adreno vs Mali), and 60 fps defaults make
  FPS-only benchmarks misleading. Report frame-time percentiles and presented frames, not a mean.
- Optimizations in the renderer can change correctness (Starling notes a case where occlusion
  culling left terrain out). Any renderer-side change needs visual parity checks.

## 8. Horizon Boost crate plan

- **Keep** `hb-profiler`: dependency-free and cheap.
- **Add when needed**: `hb-core` (shared types and config) and `hb-benchmark` (repeatable reports)
  once there is a second consumer for them.
- **Hold off**: `hb-renderer`, `hb-memory`, `hb-scheduler`, `hb-hardware`. Horizon already has a
  GPU culling and upload stack, `horizon-threads` allocator/job tooling and `horizon-system`
  hardware probes. New crates here would duplicate them unless a gap is found first.
- `hb-platform` only if Android and PC integration genuinely diverge.
