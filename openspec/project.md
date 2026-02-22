# Project Context

## Purpose

Reproduce the experiments from "Computational Life: How Well-formed, Self-replicating Programs Emerge from Simple Interaction" (Agüera y Arcas et al., arXiv:2406.19108v2, 2024). The paper demonstrates that self-replicating programs spontaneously emerge from random interactions in a "primordial soup" simulation (a variant of Fontana's Turing Gas), without any explicit fitness landscape.

The simulation places a large population of short programs (typically 2^17 programs of 64 bytes each) in a pool. At each step, two programs are randomly selected, concatenated, and executed. The output is split back into fixed-size strings that replace the parents. Self-replicators emerge naturally from this process, primarily through self-modification rather than random mutation.

## Tech Stack

- **Language:** Rust
- **Build:** Cargo
- **Compression:** `brotli` crate (for approximating Kolmogorov complexity via the paper's "high-order entropy" metric, using brotli at quality 2)
- **Testing:** Built-in `#[test]` for unit tests, `proptest` for property-based testing
- **Analysis/visualization:** TBD (likely Python scripts or a separate plotting tool for analyzing simulation output)

## Project Conventions

### Code Style

- Follow standard Rust idioms (`rustfmt`, `clippy`)
- Prefer `snake_case` for functions/variables, `CamelCase` for types
- Use `///` doc comments on public items
- Keep functions focused and short; extract helpers when a function exceeds ~50 lines

### Architecture Patterns

- **Trait-based substrate abstraction:** Each instruction set (BFF, Forth, Z80, 8080, SUBLEQ) implements a common `Substrate` trait that defines program execution semantics
- **Simulation engine is substrate-agnostic:** The primordial soup loop, population management, and metrics collection are generic over the substrate
- **Separation of concerns:** Interpreter logic, simulation orchestration, and analysis/output are distinct modules
- **Data-oriented design:** The population is a flat array of byte arrays; avoid unnecessary heap allocations in the hot loop

### Testing Strategy

- **Unit tests:** Verify each instruction's behavior in every substrate interpreter. Validate that known self-replicators (from the paper) actually self-replicate when executed
- **Property-based tests (`proptest`):** Fuzz the interpreters with random programs to ensure they never panic, always terminate (via step limits), and produce outputs of the expected size
- **Integration tests:** Run short simulations (small populations, few epochs) and verify statistical properties (e.g., entropy decreases over time once replicators emerge)

### Git Workflow

- Feature branches off `main`
- Conventional commit messages (e.g., `feat:`, `fix:`, `test:`, `docs:`)
- PRs for non-trivial changes

## Domain Context

Key concepts from the paper that AI assistants must understand:

- **Primordial soup / Turing Gas:** A population of programs that randomly interact. Two programs are selected, concatenated onto a shared tape, and executed. The resulting tape is split into fixed-size chunks that replace the parents.
- **BFF (Brainfuck Family):** The primary substrate. An extended Brainfuck with additional instructions for arithmetic, comparison, and block operations. Programs operate on a byte tape with a movable head.
- **Forth:** A stack-based language extended with tape operations. Programs manipulate both a stack and a byte tape.
- **Z80 / 8080:** Real 8-bit CPU instruction sets, emulated. Programs are machine code operating on a memory region.
- **SUBLEQ:** A one-instruction language (subtract and branch if less than or equal to zero). Serves as a counterexample — self-replication is possible but was not observed to emerge spontaneously.
- **Self-replicator:** A program that, when executed with itself as input, produces an exact copy of itself in the output.
- **High-order entropy (HOE):** The paper's proxy for Kolmogorov complexity, computed as `brotli_compressed_size(population) / raw_size(population)`. A drop in HOE signals the emergence of replicators.
- **Epoch:** One epoch = `N` interaction steps, where `N` is the population size (so on average each program participates in one interaction per epoch).

## Important Constraints

- **Faithful reproduction:** The goal is to match the paper's methodology as closely as possible. Deviations from the paper's described algorithms should be explicitly noted and justified.
- **Performance:** The simulation must handle 2^17 (131,072) programs of 64 bytes each, running for thousands of epochs. The interpreter hot loop must be fast. Avoid allocations in the inner loop.
- **Determinism:** Simulations should be reproducible given the same random seed.
- **Termination:** All interpreters must enforce a step limit to prevent infinite loops (the paper uses a per-execution step budget).
- **Reference:** The paper PDF is at `complife.pdf` in the project root. Consult it for exact instruction set definitions and simulation parameters.

## External Dependencies

- **`brotli` crate:** For computing compressed sizes as a complexity proxy
- **`rand` crate:** For random program selection and initial population generation
- **`proptest` crate:** For property-based testing
- **`complife.pdf`:** The reference paper, stored in the project root
