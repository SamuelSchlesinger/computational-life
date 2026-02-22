## 1. Project Setup
- [x] 1.1 Initialize Cargo project (`cargo init --name complife`)
- [x] 1.2 Add dependencies to `Cargo.toml`: `rand`, `brotli`, `clap`, `proptest` (dev)
- [x] 1.3 Set up module structure: `main.rs`, `lib.rs`, `substrate.rs`, `bff.rs`, `soup.rs`, `metrics.rs`

## 2. Substrate Trait
- [x] 2.1 Define `Substrate` trait in `substrate.rs` with `execute(tape: &mut [u8], step_limit: usize) -> usize`
- [x] 2.2 Add doc comments explaining the trait contract

## 3. BFF Interpreter
- [x] 3.1 Implement BFF instruction set in `bff.rs` (all 10 instructions from Section 2)
- [x] 3.2 Implement bracket matching (pre-scan with termination on unmatched)
- [x] 3.3 Implement `Substrate` trait for `Bff`
- [x] 3.4 Unit tests: each instruction in isolation (head movement, arithmetic, copy, brackets)
- [x] 3.5 Unit tests: bracket matching edge cases (unmatched, nested, empty loops)
- [x] 3.6 Unit tests: step limit enforcement
- [x] 3.7 Property tests: random programs never panic and always terminate within step limit

## 4. Complexity Metrics
- [x] 4.1 Implement HOE computation in `metrics.rs` using brotli crate at quality 2
- [x] 4.2 Unit test: HOE of uniform random data is close to 1.0
- [x] 4.3 Unit test: HOE of repeated identical data is significantly below 1.0

## 5. Simulation Engine
- [x] 5.1 Implement population initialization (random bytes from seed) in `soup.rs`
- [x] 5.2 Implement single interaction step (select pair, concatenate, execute, split)
- [x] 5.3 Implement epoch loop (N interactions per epoch)
- [x] 5.4 Implement background mutation (configurable per-byte flip rate)
- [x] 5.5 Unit test: deterministic reproduction (same seed = same result)
- [x] 5.6 Integration test: run small simulation (e.g. 256 programs, 100 epochs) and verify HOE output is reasonable

## 6. CLI
- [x] 6.1 Implement CLI argument parsing with `clap` in `main.rs`
- [x] 6.2 Wire CLI to simulation engine and metrics output
- [x] 6.3 Implement CSV output to stdout (epoch, hoe)
- [x] 6.4 Implement progress reporting to stderr
- [x] 6.5 Smoke test: run `cargo run -- --seed 42 --epochs 10` and verify output
