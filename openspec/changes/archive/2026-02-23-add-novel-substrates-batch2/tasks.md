## 1. Pulse (Signal/Collision Machine)
- [ ] 1.1 Create `src/pulse.rs` implementing the Pulse substrate with movement, collision, and annihilation rules
- [ ] 1.2 Write unit tests for particle movement, collision products, annihilation, and empty tape
- [ ] 1.3 Write property-based tests (random tapes never panic, always terminate, step count <= step_limit)
- [ ] 1.4 Add `pub mod pulse;` to `src/lib.rs`

## 2. Worm (Motile Self-Modifying Program)
- [ ] 2.1 Create `src/worm.rs` implementing the Worm substrate with head/tail movement and erasure
- [ ] 2.2 Write unit tests for COPY_SELF replication, GROW/SHRINK, TURN, worm death on zero length
- [ ] 2.3 Write property-based tests (random tapes never panic, always terminate)
- [ ] 2.4 Add `pub mod worm;` to `src/lib.rs`

## 3. Echo (Delay-Line Memory Machine)
- [ ] 3.1 Create `src/echo.rs` implementing the Echo substrate with constrained write pointer
- [ ] 3.2 Write unit tests for ECHO copying at delay offset, SET_DELAY, 3-byte replicator, jumps
- [ ] 3.3 Write property-based tests (random tapes never panic, always terminate)
- [ ] 3.4 Add `pub mod echo;` to `src/lib.rs`

## 4. Meta (Heterogeneous Cellular Automaton)
- [ ] 4.1 Create `src/meta.rs` implementing the Meta substrate with synchronous double-buffered update
- [ ] 4.2 Write unit tests for rightward replicator, conflict resolution, fixed-point termination, bidirectional spreading
- [ ] 4.3 Write property-based tests (random tapes never panic, always terminate)
- [ ] 4.4 Add `pub mod meta;` to `src/lib.rs`

## 5. Tide (Bidirectional Oscillating PC)
- [ ] 5.1 Create `src/tide.rs` implementing the Tide substrate with oscillating PC and dual-nibble decoding
- [ ] 5.2 Write unit tests for direction reversal, forward/backward nibble interpretation, 3-byte replicator
- [ ] 5.3 Write property-based tests (random tapes never panic, always terminate)
- [ ] 5.4 Add `pub mod tide;` to `src/lib.rs`

## 6. Integration
- [ ] 6.1 Wire up all 5 substrates in CLI substrate selection (match arms in main.rs or equivalent)
- [ ] 6.2 Wire up all 5 substrates in the viewer/visualization substrate menu
- [ ] 6.3 Update DIVERGENCES.md with descriptions of the 5 new substrates
- [ ] 6.4 Update README.md substrate list
