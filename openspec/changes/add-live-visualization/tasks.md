## 1. Dependencies and Feature Flag
- [x] 1.1 Add `bevy` and `bevy_egui` dependencies behind a `viz` feature flag in `Cargo.toml`
- [x] 1.2 Add `viz` module to `lib.rs` gated with `#[cfg(feature = "viz")]`

## 2. Extended Metrics
- [x] 2.1 Add `unique_program_count(programs: &[Vec<u8>]) -> usize` to `metrics.rs`
- [x] 2.2 Add `zero_byte_count(programs: &[Vec<u8>]) -> usize` to `metrics.rs`
- [x] 2.3 Add `byte_frequency_histogram(programs: &[Vec<u8>]) -> [usize; 256]` to `metrics.rs`
- [x] 2.4 Unit tests for all three new metrics functions

## 3. Benchmark Mode (headless, no viz feature needed)
- [x] 3.1 Add `--benchmark` flag to CLI
- [x] 3.2 When `--benchmark` is set, suppress CSV output and print throughput stats (epochs/sec, interactions/sec) to stderr after completion
- [x] 3.3 Smoke test: `cargo run --release -- --seed 42 --epochs 100 --benchmark --population-size 1024`

## 4. Visualization Module — Threading and Data Flow
- [x] 4.1 Create `src/viz.rs` with bevy app setup (window, camera, bevy_egui plugin)
- [x] 4.2 Define `EpochMetrics` struct (epoch, hoe, unique_count, zero_count, byte_histogram)
- [x] 4.3 Define `SimCommand` enum (Play, Pause)
- [x] 4.4 Spawn simulation thread that owns `Soup`, runs epochs in a loop, sends `EpochMetrics` via channel, receives `SimCommand`s
- [x] 4.5 Define bevy resources: `SimReceiver` (wraps channel receiver in Mutex), `SimCommander` (wraps channel sender), `SimulationHistory` (stores metrics history), `PlaybackState`

## 5. Visualization Module — UI and Plots
- [x] 5.1 Implement `drain_metrics` system: each frame, drain the channel and append to `SimulationHistory`
- [x] 5.2 Implement `render_ui` egui system with panels:
  - HOE line chart (egui_plot)
  - Unique programs line chart
  - Zero count line chart
  - Byte frequency histogram (bar chart)
  - Playback controls (play/pause button, epoch display)
- [x] 5.3 Verify compilation with `cargo build --features viz`

## 6. CLI Integration
- [x] 6.1 Add `--live` flag to CLI (only available with `viz` feature via conditional compilation)
- [x] 6.2 Wire `--live` to launch the bevy app, passing config and seed to the sim thread
- [x] 6.3 Ensure headless mode is unaffected when `viz` feature is not enabled

## 7. Smoke Tests
- [x] 7.1 Run `cargo run --features viz -- --seed 42 --epochs 5000 --live --population-size 1024` and verify window opens with updating plots
- [x] 7.2 Verify headless mode still works: `cargo run -- --seed 42 --epochs 10`
- [x] 7.3 Verify benchmark mode: `cargo run --release -- --seed 42 --epochs 100 --benchmark --population-size 1024`
