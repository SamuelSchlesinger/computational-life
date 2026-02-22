# Change: Add Forth substrate and 2D spatial simulation with grid viewer

## Why

The paper (Section 3.1) demonstrates that self-replicators emerge even more quickly and consistently in a Forth-based substrate than in BFF. The 2D spatial simulation (Section 2.2) adds locality of interaction, producing visually striking dynamics where replicator waves spread across a grid and multiple replicator species compete for territory (Figures 8 and 10). Combining the Forth substrate with 2D spatial simulation and a grid viewer enables us to reproduce the paper's most visually compelling experiments.

## What Changes

- Implement the Forth (primordial soup variant) interpreter in `src/forth.rs`, following the instruction set from Section 3.1.1 (16 fixed opcodes + push-immediate + relative jump)
- Add a 2D spatial simulation mode in `src/soup2d.rs` where programs are arranged on a grid and interactions are restricted to Chebyshev-distance-2 neighbors
- Add a 2D grid viewer (behind the `viz` feature) that renders each program as a colored tile, showing replicator spread in real time
- Wire the `--substrate forth` option and `--grid WxH` CLI flags into main
- Add unit tests, property tests, and integration tests

## Impact

- Affected specs: `forth-interpreter` (new), `spatial-simulation` (new), `grid-visualization` (new), `simulation-cli` (modified), `live-visualization` (modified)
- Affected code: new `src/forth.rs`, new `src/soup2d.rs`, extended `src/viz.rs`, extended `src/main.rs`, `src/lib.rs`, `Cargo.toml`
