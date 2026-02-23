## Context

This change adds 5 novel computational substrates to the Computational Life project. Each substrate implements the `Substrate` trait and operates on the existing shared byte tape. The substrates explore paradigms not yet covered by the 8 existing substrates: parallel particle dynamics, motile programs, constrained memory access, cellular automata, and oscillating control flow.

## Goals / Non-Goals

- Goals:
  - Implement 5 substrates that each pass novelty, replicator-size, and aesthetic criteria
  - Each substrate produces self-replicators discoverable through the primordial soup simulation
  - All substrates fit the existing `Substrate` trait interface
  - Comprehensive unit tests and property-based tests for each substrate

- Non-Goals:
  - Modifying the simulation engine or `Substrate` trait
  - Optimizing existing substrates
  - Adding new visualization features beyond substrate selection

## Decisions

### Pulse: Collision Rule Design
- Decision: Use a compact, deterministic collision rule based on byte arithmetic (XOR of types, sum for velocities). Collision rules are fixed in the interpreter, not programmable per-configuration.
- Rationale: Fixed rules keep the interpreter simple and the `execute` interface unchanged. The rules should be engineered to be "replication-friendly" — including a COPY-type collision that preserves one particle's type.
- Encoding: Bit 7 = direction (0=right, 1=left), bits 6-0 = particle type. 0x00 = empty/vacuum.
- Step semantics: Each call to `execute` runs `step_limit` global particle-advance steps, each O(tape_length).

### Worm: Body Representation
- Decision: Head pointer and tail pointer delimit the worm body on the fixed tape. The worm's length can grow or shrink via dedicated instructions.
- Encoding: High nibble = opcode (16 instructions), low nibble = operand.
- Instructions: EMIT (write next byte's value ahead of head), COPY_SELF (write current instruction byte ahead), GROW (delay tail), SHRINK (advance tail extra), TURN (reverse direction), SKIP_Z (skip if ahead is zero), HALT, NOP, and arithmetic on a small accumulator.
- Key invariant: The worm body is always contiguous. If head overtakes tail (wrapping), the worm halts.

### Echo: Delay Encoding
- Decision: The delay register is a u8, giving offsets 0-255. SET_DELAY instruction uses the next byte as the delay value (2-byte instruction). The initial delay defaults to `tape.len() / 2`.
- Rationale: Defaulting to half-tape makes the simplest replicator (just ECHO in a loop) work immediately.
- The read pointer auto-advances on ECHO; the write pointer is always `rp + delay` and cannot be set independently.

### Meta: Update Semantics
- Decision: Synchronous update with double-buffering. All cells read from the current state and write to a buffer; after all cells update, the buffer replaces the tape.
- Conflict resolution: When two cells try to write to the same position, the cell with the higher payload (low 4 bits) wins. Ties broken by leftmost cell.
- Performance: Each step is O(tape_length). The `execute` function runs `step_limit` global CA steps.
- Encoding: Bits 7-6 = condition, bits 5-4 = action, bits 3-0 = payload.

### Tide: Bounce Mechanics
- Decision: The bounce distance is stored in a register, initialized to the program's length (determined by scanning for a HALT or boundary marker, or set explicitly by SET_BOUNCE instruction). When the PC has taken `bounce_distance` steps forward, it reverses. After `bounce_distance` steps backward, it reverses again.
- Encoding: Each byte has a high-nibble instruction (forward) and low-nibble instruction (backward). 4-bit opcode space per direction = 16 forward instructions and 16 backward instructions.
- The initial bounce distance defaults to `tape.len() / 2` so that the PC oscillates over the first half of the tape.

## Risks / Trade-offs

- **Pulse performance**: O(tape_length) per step is more expensive than O(1) per step for sequential substrates. For a 128-byte tape this is negligible; for large tapes it could slow simulation.
- **Meta double-buffering**: Requires allocating a temporary buffer of tape_length bytes inside `execute`. This is a small, bounded allocation that should not impact performance meaningfully.
- **Worm trivial replicators**: A single COPY_SELF byte may produce a trivially dominant replicator that crowds out interesting evolution. Consider whether to make COPY_SELF a 2-byte instruction to raise the bar slightly.

## Open Questions

- Exact collision rules for Pulse (need experimentation to find rules that enable discoverable replicators without being trivial)
- Whether Worm should support bidirectional movement or always move forward
- Optimal bounce-distance initialization strategy for Tide
