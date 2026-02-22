## Context

This is a greenfield Rust project reproducing the "Computational Life" paper (arXiv:2406.19108v2). The simulation must handle 2^17 programs of 64 bytes each, running for thousands of epochs, so performance in the inner loop matters. The architecture must also support five different instruction sets (BFF, Forth, Z80, 8080, SUBLEQ), so the substrate abstraction must be clean.

## Goals / Non-Goals

- Goals:
  - Faithful reproduction of the paper's BFF primordial soup experiments
  - Clean `Substrate` trait that future substrates (Forth, Z80, 8080, SUBLEQ) can implement without modifying the simulation engine
  - Deterministic, seed-based reproducibility
  - CSV output for analysis (epoch, HOE, unique token count, etc.)

- Non-Goals:
  - 2D spatial simulations (deferred to a later proposal)
  - Long-tape simulations (deferred)
  - Visualization / plotting (out of scope; use external tools on CSV output)
  - GPU/CUDA acceleration (out of scope)
  - Multi-threading (can be added later; start single-threaded for simplicity and determinism)

## Decisions

### Single binary crate (not a library + binary workspace)

Start with a single `complife` crate that contains both the library modules and a `main.rs` binary. Split into a workspace only if/when we need to share code across multiple binaries. This avoids premature structure.

### Substrate trait design

```rust
pub trait Substrate {
    /// Execute the concatenated program on the tape, up to the step limit.
    /// The tape is modified in-place. Returns the number of steps executed.
    fn execute(tape: &mut [u8], step_limit: usize) -> usize;
}
```

The trait is intentionally minimal. The simulation engine handles concatenation, splitting, population management, and metrics — the substrate only defines how to execute bytes on a tape. The trait uses a static method (no `&self`) because substrates carry no state.

### Population representation

A flat `Vec<[u8; 64]>` (or `Vec<Vec<u8>>` if program size needs to be configurable). For the default 2^17 programs of 64 bytes, this is 8 MB — fits comfortably in cache. Interaction uses a temporary 128-byte buffer for the concatenated tape.

### Bracket matching in BFF

Pre-compute a bracket-match table for each concatenated program before execution, stored in a stack-allocated array indexed by instruction position. If brackets are unmatched (no matching `]` for a `[`), the program terminates immediately on encountering the unmatched bracket. This matches the paper's description: "If no matching parenthesis is found, the program terminates."

### Random number generation

Use `rand` crate with `StdRng::seed_from_u64(seed)` for full determinism. The seed is a CLI argument.

### HOE metric

Concatenate the entire population into a single byte buffer, compress with brotli at quality 2 (matching the paper's `brotli -q2`), and compute `compressed_size / raw_size`. This is computed once per epoch.

## Risks / Trade-offs

- **Performance of brotli compression on 8 MB per epoch:** This could be a bottleneck. Mitigation: only compute HOE every N epochs (configurable), or compute it in a background thread. Start with every-epoch and measure.
- **Fixed 64-byte program size:** Hardcoding `[u8; 64]` is simpler and faster but less flexible. Mitigation: use a const generic or configurable parameter if needed later. Start with a constant.
- **Single-threaded:** Simpler and deterministic but slower. The paper mentions their code supports both CPU and CUDA. Mitigation: the hot loop is embarrassingly parallel (independent interactions), so threading can be added later without architectural changes.

## Open Questions

- The paper mentions "tracer tokens" for analysis (epoch, position, char packed into 64-bit integers). Should we implement these in this first pass, or defer? **Decision: defer.** They're an analysis tool, not core to the simulation.
