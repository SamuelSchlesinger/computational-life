# Change: Add live visualization of simulation metrics

## Why

Running simulations headlessly with CSV output makes it hard to observe the dynamics in real time. The paper uses several diagnostic plots (Figures 1, 2, 5) that reveal the state transition from pre-life to life. A live visualization lets us watch self-replicators emerge as they happen, see the byte distribution shift, and interactively control simulation speed.

## What Changes

- Add `bevy` and `bevy_egui` as dependencies (behind a `viz` feature flag to keep headless builds lightweight)
- Add new metrics functions: unique program count, zero byte count, byte frequency histogram
- Add a `--live` CLI flag that launches a bevy window with egui plots instead of printing CSV
- Run the simulation on a dedicated thread, communicating with the render thread via channels for maximum throughput
- Implement live-updating plots: HOE over time, unique programs over time, zero count over time, byte frequency histogram
- Add simulation playback controls: play/pause, speed slider, epoch counter display
- Add a `--benchmark` flag to the headless CLI to measure epoch throughput (epochs/sec)

## Impact

- Affected specs: `live-visualization` (new), `simulation-metrics` (new)
- Affected code: `Cargo.toml`, `src/main.rs`, new `src/viz.rs`, extended `src/metrics.rs`
- No changes to the core simulation engine (`soup.rs`, `bff.rs`, `substrate.rs`)
