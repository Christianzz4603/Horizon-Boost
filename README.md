# Horizon Boost

**Horizon Boost (HB)** is a performance research and optimization project for Horizon PC and Starling Mobile.

The goal is to measure real bottlenecks first, then implement safe, evidence-based optimizations. HB distinguishes rendered frames, presented frames, and display refresh rate so benchmark results remain meaningful.

## Project principles

- Measure before optimizing.
- Preserve correct world rendering and game behavior.
- Prefer supported Horizon integration points over fragile runtime patching.
- Keep platform-specific graphics code behind clear interfaces.
- Never claim an FPS improvement without reproducible before/after measurements.

## Current status

Early development. The first milestone is a small, dependency-free Rust profiling core for frame-time statistics. It does not hook into the game or change rendering yet.

`hb-profiler` keeps a bounded window of frame times with O(1) recording and reports average FPS, p50/p95/p99 frame time, the slowest frame and the 1% low FPS. See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for how Horizon and Starling are structured and where Horizon Boost could integrate.

## Planned components

| Component | Responsibility | Status |
|---|---|---|
| `hb-profiler` | Frame-time samples and benchmark statistics | Initial |
| `hb-core` | Shared configuration and platform abstractions | Planned |
| `hb-renderer` | Rendering-path research and supported integrations | Research |
| `hb-memory` | Resource and allocation measurements | Planned |
| `hb-benchmark` | Repeatable benchmark reports | Planned |

## Important metrics

- **Rendered FPS:** completed frames produced by the renderer.
- **Presented FPS:** frames submitted to the display system.
- **Display refresh rate:** the screen's physical refresh cadence.
- **Frame time:** time per frame, including p50/p95/p99 for consistency.

A very high render-loop counter is not automatically equivalent to useful, displayed frames.

## Development

This repository is a Rust Cargo workspace. The first crate has no third-party dependencies.

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo run --release -p hb-profiler --example record_cost
```

`record_cost` times `FrameWindow::record` on a full window. Results depend on the machine, so compare runs on the same hardware only.

## Compatibility

Horizon and Starling integration is not implemented yet. We must pin and verify the exact supported game/API revisions before adding native hooks. Starling Mobile and Horizon PC may require different integration layers.

## Stability

Native mods can execute with the permissions of the game process or user. Review experimental builds and dependencies before installing them. Back up worlds before testing unfamiliar builds.

## License

MIT. See [LICENSE](LICENSE).
