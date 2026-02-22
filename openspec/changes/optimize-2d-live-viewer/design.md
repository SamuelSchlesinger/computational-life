## Context
The 2D live viewer runs a spatial primordial soup simulation on a background thread and renders a color-mapped grid plus time-series plots via bevy/egui. At larger grid sizes (240x135+), performance is limited by heap allocations in the simulation loop and expensive per-interval metrics computation.

## Goals / Non-Goals
- Goals:
  - Achieve 2-5x throughput improvement for 2D simulations at 240x135 grid size
  - Keep UI responsive at 60fps even during heavy simulation
  - Bound memory usage regardless of simulation duration
- Non-Goals:
  - Parallelizing the simulation across threads (future work)
  - GPU-accelerated rendering of the grid (current egui texture approach is sufficient)
  - Changing the Forth interpreter performance (already tight)

## Decisions

### 1. Buffer reuse in Soup2d
- **Decision**: Add persistent scratch buffers to `Soup2d` struct for `order`, `taken`, `tape`, and a flat neighbor offset table.
- **Alternatives considered**: Using a pool allocator — rejected as over-engineering; struct-level buffers are simpler and sufficient.
- **Rationale**: The `run_epoch` hot loop currently allocates 4+ Vecs per epoch and 1+ per interaction. Moving these to struct fields eliminates all hot-path allocations.

### 2. Pre-computed neighbor table
- **Decision**: Pre-compute a flat array of neighbor offsets per cell at construction time, stored as `Vec<(usize, usize)>` pairs of (start, end) indices into a flat neighbor-index buffer.
- **Alternatives considered**: Computing neighbors inline without allocation — viable but more complex and less cache-friendly than a pre-computed table.
- **Rationale**: The neighbor set for each cell never changes. Computing it once at startup and looking it up via two indices per cell is simple and fast.

### 3. Decoupled snapshot vs. metrics cadence
- **Decision**: The sim thread sends two types of messages: lightweight `GridUpdate` (just pixels, sent frequently) and full `MetricsUpdate` (HOE + stats, sent at `metrics_interval`). The render thread processes both independently.
- **Alternatives considered**: Single message type with optional fields (current approach) — rejected because it forces either expensive metrics on every send or stale grid display.
- **Rationale**: Grid snapshots are cheap (hash each program, ~130KB). HOE computation via brotli is the expensive part. Decoupling lets the grid update smoothly while metrics update at a sustainable rate.

### 4. Latest-only grid snapshot storage
- **Decision**: Store only the latest grid snapshot in a dedicated resource, not in the history entries vector.
- **Alternatives considered**: Keeping snapshots in history for replay — not currently needed and wastes significant memory.
- **Rationale**: At 240x135, each snapshot is ~130KB. Over 10,000 epochs at interval=10, that's 130MB of snapshot data. Storing only the latest eliminates this.

### 5. In-place program output (avoid to_vec)
- **Decision**: Copy execution results back into `self.programs[first]` and `self.programs[second]` via `copy_from_slice` instead of allocating new Vecs with `to_vec()`.
- **Rationale**: Each interaction currently allocates two new `Vec<u8>`. With 16,200 interactions per epoch (half of 32,400), that's 32,400 allocations eliminated per epoch.

## Risks / Trade-offs
- Pre-computed neighbor table uses O(cells * 24) memory (~780KB for 240x135) — acceptable.
- Decoupled cadence means the grid may show a more recent state than the metrics plots — acceptable for a live viewer; both are approximate anyway.

## Open Questions
- None — the optimizations are straightforward and don't change external behavior.
