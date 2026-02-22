# Change: Optimize 2D live viewer for larger, faster runs

## Why
The 2D live viewer becomes a bottleneck at larger grid sizes (e.g. 240x135 = 32,400 programs). Per-epoch heap allocations in the simulation hot loop, expensive brotli compression on every metrics send, unbounded history growth storing full grid snapshots, and per-frame texture re-creation all limit throughput and responsiveness.

## What Changes
- **Simulation hot loop** (`soup2d.rs`): Eliminate per-epoch and per-interaction heap allocations by reusing buffers for the shuffled order, taken flags, tape, and neighbor lists.
- **Decoupled snapshot cadence** (`viz.rs`): Send lightweight grid snapshots every epoch (or every N epochs independently configurable) while computing expensive metrics (HOE, unique count) less frequently.
- **Bounded history** (`viz.rs`): Only store the latest grid snapshot rather than accumulating one per history entry. Cap or downsample time-series history to prevent unbounded memory growth.
- **Texture reuse** (`viz.rs`): Only update the egui texture when new snapshot data actually arrives, instead of freeing and re-allocating every frame.

## Impact
- Affected specs: `soup-simulation`, `live-visualization`
- Affected code: `src/soup2d.rs`, `src/viz.rs`
- No behavioral changes to simulation correctness or determinism
- No breaking changes to CLI interface
