# Change: Add core simulation engine and BFF substrate

## Why

This is the foundational change for the project. We need the core simulation engine (primordial soup loop), a trait abstraction for substrates, the BFF interpreter (the paper's primary substrate), complexity metrics (high-order entropy), and a CLI to run experiments. Without these, nothing else can be built.

## What Changes

- Initialize Cargo project with workspace structure
- Define the `Substrate` trait that all instruction sets will implement
- Implement the BFF (Brainfuck Family) interpreter with all 10 instructions from Section 2 of the paper
- Implement the primordial soup simulation loop from Section 2.1
- Implement high-order entropy (HOE) metric using brotli compression (Section 2.1, footnote 1)
- Implement background mutation (random bit-flipping at configurable rate)
- Add a CLI binary to configure and run simulations with CSV epoch-level output
- Add unit tests, property-based tests, and a short integration test

## Impact

- Affected specs: `program-execution`, `soup-simulation`, `bff-interpreter`, `complexity-measurement`, `simulation-cli` (all new)
- Affected code: entire `src/` directory (new Cargo project)
