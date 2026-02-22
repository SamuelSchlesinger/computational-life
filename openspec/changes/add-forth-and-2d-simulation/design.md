## Context

The paper describes two orthogonal extensions beyond the basic BFF primordial soup:
1. **Forth substrate** (Section 3.1.1): A stack-based instruction set where bytes encode opcodes differently from BFF. The Forth variant produces self-replicators faster and more consistently than BFF.
2. **2D spatial simulation** (Section 2.2): Instead of uniform random pairing, programs are placed on a 2D grid and can only interact with neighbors within Chebyshev distance 2. This creates spatial dynamics where replicator waves visibly spread across the grid.

Both extensions reuse the existing `Substrate` trait and simulation infrastructure.

## Goals / Non-Goals

- Goals:
  - Faithfully implement the Forth primordial soup instruction set from Section 3.1.1
  - Implement 2D spatial simulation following Section 2.2's grid interaction rules
  - Provide a 2D grid viewer that renders programs as colored tiles (like Figures 8/10)
  - Keep the 2D simulation substrate-agnostic (works with both BFF and Forth)
- Non-Goals:
  - Long-tape Forth simulations (Section 3.1.2) — different execution model, separate proposal
  - 1D spatial simulation — mentioned briefly in the paper but not detailed
  - GPU acceleration

## Decisions

### Forth instruction encoding
- Decision: Implement the exact instruction set from Section 3.1.1 using byte-level decoding (top bits determine opcode class)
- The instruction byte is split: bits 7-6 select the class (00=fixed opcode, 01=push immediate, 1X=relative jump), bits 5-0 provide the operand
- For the 00 class, bits 3-0 select one of 14 specific opcodes (0x00-0x0D); byte values 0x0E-0x3F in the 00 class are no-ops
- Stack underflow terminates execution (matches paper behavior)
- All tape accesses wrap modulo tape length

### 2D simulation structure
- Decision: Create `Soup2d` as a separate struct (not a mode flag on `Soup`) since the interaction logic is fundamentally different (grid-based neighbor selection vs. uniform random pairing)
- The grid stores the same `Vec<Vec<u8>>` population, but indexes by `(x, y)` coordinates mapped to a flat index
- Each epoch iterates programs in a shuffled order; for each, a random neighbor within Chebyshev distance 2 is selected; if neither has been "taken" this epoch, they interact

### Grid viewer
- Decision: Render each program as a small colored square in the bevy window, using byte content to derive color. Each byte maps to an RGB value to create a visual fingerprint of the tape.
- Display the grid alongside the existing time-series plots
- Keep the time-series metrics (HOE, unique count, etc.) in a side panel

## Risks / Trade-offs

- 2D simulation is slower per epoch (O(grid_size) with neighbor lookups) but each interaction is the same cost — acceptable since the paper uses smaller populations for 2D (32400 vs 131072)
- The grid viewer adds rendering overhead for large grids — mitigate by only updating the texture when new epoch data arrives
- Forth programs with deep stacks could be slow — the step limit bounds this

## Open Questions

- None at this time
