## 1. Forth Interpreter
- [x] 1.1 Create `src/forth.rs` with instruction byte decoding (3 classes: fixed opcode, push immediate, relative jump)
- [x] 1.2 Implement all 14 fixed opcodes (READ, READ64, WRITE, WRITE64, DUP, POP, SWAP, SKIPNZ, INC, DEC, ADD, SUB, COPY, RCOPY)
- [x] 1.3 Implement push-immediate (0x40-0x7F) and relative jump (0x80-0xFF)
- [x] 1.4 Implement bounded stack with underflow/overflow termination
- [x] 1.5 Implement `Substrate` trait for `Forth`
- [x] 1.6 Unit tests: each fixed opcode in isolation
- [x] 1.7 Unit tests: push immediate and relative jump
- [x] 1.8 Unit tests: stack underflow/overflow termination
- [x] 1.9 Unit tests: step limit enforcement
- [x] 1.10 Unit test: trivial self-replicator (0x0C on empty stack copies byte to position 64)
- [x] 1.11 Property tests: random programs never panic and always terminate within step limit

## 2. CLI Integration for Forth
- [x] 2.1 Add `Forth` to substrate match in `main.rs` (both headless and viz paths)
- [x] 2.2 Register `forth` module in `lib.rs`
- [x] 2.3 Smoke test: `cargo run -- --seed 42 --epochs 100 --substrate forth`

## 3. 2D Spatial Simulation
- [x] 3.1 Create `src/soup2d.rs` with `Soup2d` struct (grid dimensions, programs, RNG)
- [x] 3.2 Implement grid initialization (random programs from seed)
- [x] 3.3 Implement Chebyshev-distance-2 neighbor lookup
- [x] 3.4 Implement 2D epoch: shuffled iteration, "taken" marking, neighbor pairing, concatenation-execution-split
- [x] 3.5 Implement background mutation (same as Soup, applied to all programs)
- [x] 3.6 Implement `population_bytes()` for HOE computation
- [x] 3.7 Unit tests: neighbor enumeration at corners, edges, and center
- [x] 3.8 Unit test: deterministic 2D simulation (same seed = same result)
- [x] 3.9 Integration test: run small 2D simulation (e.g., 10x10 grid, 100 epochs)

## 4. CLI Integration for 2D Mode
- [x] 4.1 Add `--grid WxH` CLI argument with parsing
- [x] 4.2 Wire 2D mode into headless simulation path
- [x] 4.3 Wire 2D mode into benchmark path
- [x] 4.4 Smoke test: `cargo run -- --seed 42 --epochs 100 --grid 20x20 --substrate forth`

## 5. 2D Grid Visualization
- [x] 5.1 Define `GridSnapshot` struct (width, height, color data) for sending grid state to render thread
- [x] 5.2 Extend `EpochMetrics` to optionally include `GridSnapshot`
- [x] 5.3 Implement program-to-color hashing function
- [x] 5.4 Create bevy `Image` texture from grid color data
- [x] 5.5 Implement `render_grid_ui` system: display grid texture in central panel, metrics in side panel
- [x] 5.6 Wire 2D viz mode into `run_viz` and simulation thread
- [x] 5.7 Smoke test: `cargo run --features viz -- --seed 42 --epochs 5000 --grid 60x40 --live --substrate forth`
