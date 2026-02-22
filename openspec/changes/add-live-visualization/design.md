## Context

The simulation engine runs a primordial soup that can take thousands of epochs before self-replicators emerge. The paper uses several diagnostic plots to detect and analyze state transitions (Figures 1, 2, 5). We want to watch these plots update live during a running simulation, using bevy for windowing and bevy_egui for plotting.

## Goals / Non-Goals

- Goals:
  - Live-updating plots of key metrics from the paper (HOE, unique programs, zero count, byte distribution)
  - Playback controls (play/pause, speed adjustment)
  - Feature-gated so headless builds remain lightweight
  - Smooth UX: simulation advances in the background, UI stays responsive

- Non-Goals:
  - 2D spatial grid visualization (deferred to when spatial simulations are implemented)
  - Per-program or per-tape visualization (deferred)
  - Tracer token analysis (deferred)
  - Multi-run statistical views like Figure 6's heatmap

## Decisions

### Feature flag `viz`

bevy and bevy_egui are heavy dependencies. Gate them behind `cargo build --features viz`. The `--live` CLI flag is only available when the feature is enabled. Default `cargo build` produces the lightweight headless binary.

### Bevy architecture — simulation on a dedicated thread

The simulation MUST run on a separate thread from the bevy render loop to maximize throughput. The architecture:

- **Simulation thread:** Owns the `Soup`. Runs epochs in a tight loop, computes metrics periodically, and sends `EpochMetrics` snapshots to the render thread via a `std::sync::mpsc` channel. Respects play/pause and speed commands received via a second channel (render → sim).
- **Render thread (bevy main loop):** Owns the `SimulationHistory` resource. Each frame, drains the metrics channel and appends new data points. The `render_ui` system draws egui plots from the history. Sends control commands (play/pause, target speed) to the sim thread.
- **Communication:** Two channels:
  1. `mpsc::Receiver<EpochMetrics>` — sim → render (metrics data)
  2. `mpsc::Sender<SimCommand>` — render → sim (play/pause/speed)
- **No shared mutable state** between threads — all communication is via channels. This avoids locking and keeps both threads running at full speed.
- The byte frequency histogram is included in `EpochMetrics` (256 `usize` values) since it's cheap and the sim thread already has the data.

### Simulation parallelism (future, measure first)

The inner epoch loop (N interaction steps) could potentially be parallelized via batching: split the population into non-overlapping chunks and run interactions within each chunk in parallel. However, this changes the interaction dynamics (programs can only interact within their chunk per batch). This is a semantics-altering optimization and must be validated carefully against single-threaded results. **For this proposal, we keep the simulation itself single-threaded on its dedicated thread** and revisit parallelism as a separate change if throughput is insufficient. We will add a `--benchmark` flag to measure epoch throughput (epochs/sec) so we have data to guide future optimization.

### Metrics history

Store time-series metrics in `Vec<EpochMetrics>` where each entry contains:
- epoch number
- HOE value
- unique program count
- zero byte count

The byte frequency histogram is computed on-demand (current frame only, not stored historically) since it's 256 values and changes every epoch.

### Plots (matching the paper)

1. **HOE over epochs** (line chart) — the primary diagnostic from Figures 1, 5
2. **Unique programs over epochs** (line chart) — from Figure 1 (blue line, "unique tokens")
3. **Zero count over epochs** (line chart) — from Figure 2 right panel ("zero_n")
4. **Byte frequency histogram** (bar chart, 256 bars) — shows convergence toward instruction bytes

All plots use egui's built-in `egui_plot` module. Each plot is in its own egui panel/collapsible section.

### Unique program counting

Use a `HashSet<[u8; 64]>` or hash-based counting. For 2^17 programs of 64 bytes each, this is feasible (~8MB of data to hash). Compute once per metrics snapshot.

## Risks / Trade-offs

- **Frame rate with large populations:** Computing HOE (brotli compression of 8MB) and unique program count per epoch may be slow. Mitigation: metrics are computed on the sim thread, not the render thread — the UI never blocks on computation. The metrics interval is configurable.
- **bevy version churn:** bevy has fast release cycles. We'll use bevy 0.15 with bevy_egui 0.33 (well-tested combination). The viz module is isolated and feature-gated.
- **Large dependency tree:** bevy pulls in many crates. Mitigation: feature flag keeps it out of default builds.
- **Thread synchronization overhead:** Using channels has minimal overhead compared to shared-memory approaches. The sim thread sends one `EpochMetrics` struct per metrics interval (small payload). No contention.

## Open Questions

None.
