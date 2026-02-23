# Change: Add five novel computational substrates (Batch 2)

## Why

The project currently has 8 substrates covering pointer machines (BFF), stack machines (Forth), single-instruction machines (SUBLEQ, RSUBLEQ4), queue/FIFO (Qop), skip-chains (Skim), register-indirect (Rig), and bit-serial (Bits). While diverse, all 8 share two architectural assumptions: (1) a single sequential program counter, and (2) imperative fetch-decode-execute semantics. This batch introduces 5 substrates that break one or both assumptions, exploring fundamentally different computational paradigms.

These 5 designs were selected from 11 candidates evaluated on three criteria: novelty (different enough from all existing substrates), replicator size (plausibly discoverable through simulation), and aesthetic intrigue (compelling intellectual story). Six candidates were rejected for insufficient novelty or excessively large minimum replicators.

## What Changes

- **Pulse** (Signal/Collision Machine): No PC. Multiple particles move simultaneously and compute through collisions. Inspired by collision-based computing (Fredkin, Margolus). First substrate with inherent parallelism. Estimated replicator: ~15-25 bytes.
- **Worm** (Motile Self-Modifying Program): The program physically crawls through the tape — head reads, tail erases. Self-replication = emitting a copy ahead before the tail catches up. Estimated replicator: 1-4 bytes.
- **Echo** (Delay-Line Memory Machine): Write address is always `read_address + delay`. No random writes. Inspired by mercury delay-line memory (EDSAC, UNIVAC). Estimated replicator: 3-4 bytes.
- **Meta** (Heterogeneous Cellular Automaton): No PC. Every byte is a local CA rule applied simultaneously. The only substrate where the tape is a population of autonomous agents, not a program. Estimated replicator: 1 byte.
- **Tide** (Bidirectional Oscillating PC): PC sweeps forward (high nibble) then backward (low nibble). Each byte encodes two instructions. Natural read/write rhythm without explicit loops. Estimated replicator: 3 bytes.

## Impact

- Affected specs: New specs created for each substrate (`pulse-interpreter`, `worm-interpreter`, `echo-interpreter`, `meta-interpreter`, `tide-interpreter`)
- Affected code: New source files `src/pulse.rs`, `src/worm.rs`, `src/echo.rs`, `src/meta.rs`, `src/tide.rs`; updates to `src/lib.rs` for module declarations; updates to CLI/viewer for substrate selection
