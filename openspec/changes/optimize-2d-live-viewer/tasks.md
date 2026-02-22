## 1. Simulation hot-loop optimization (soup2d.rs)
- [x] 1.1 Add persistent `order`, `taken`, and `tape` buffers to `Soup2d` struct
- [x] 1.2 Pre-compute flat neighbor table at construction time; replace `neighbors()` method with table lookup
- [x] 1.3 Replace `tape[..ps].to_vec()` with `copy_from_slice` back into existing program buffers
- [x] 1.4 Verify determinism: existing tests pass with identical results for same seeds

## 2. Decoupled snapshot and metrics (viz.rs)
- [x] 2.1 Split into separate channels: `GridSnapshot` channel (pixels only) and `EpochMetrics` channel (HOE, unique, zero, histogram)
- [x] 2.2 Send `GridSnapshot` every epoch; send `EpochMetrics` at `metrics_interval`
- [x] 2.3 Remove `grid_snapshot` field from `EpochMetrics`; store latest snapshot in a dedicated `LatestGridSnapshot` resource

## 3. Render-side optimization (viz.rs)
- [x] 3.1 Only update egui texture when a new grid snapshot has been received (track a dirty flag)
- [x] 3.2 Remove grid snapshot data from `SimulationHistory::entries` to bound memory

## 4. Validation
- [x] 4.1 Run existing tests (`cargo test`) — 83/83 pass, no regressions
- [ ] 4.2 Run `cargo run --features viz -- --substrate forth --grid 240x135 --live --seed 42 --epochs 50000` and verify smooth grid rendering with responsive UI
- [x] 4.3 Run `cargo run -- --substrate forth --grid 240x135 --benchmark --seed 42 --epochs 100` — 30.8 epochs/sec
- [x] 4.4 Fix benchmark grid display bug (was showing `32400x32400` instead of `240x135`)
