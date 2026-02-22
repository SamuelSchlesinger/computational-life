# simulation-cli Specification

## Purpose
TBD - created by archiving change add-core-engine-and-bff. Update Purpose after archive.
## Requirements
### Requirement: CLI Configuration

The system SHALL provide a command-line interface that accepts the following parameters:
- `--seed <u64>`: Random seed for reproducibility (required)
- `--population-size <usize>`: Number of programs (default: 131072)
- `--program-size <usize>`: Bytes per program (default: 64)
- `--epochs <usize>`: Number of epochs to run (required)
- `--step-limit <usize>`: Max steps per program execution (default: 8192)
- `--mutation-rate <f64>`: Per-byte mutation probability per epoch (default: 0.00024)
- `--substrate <string>`: Which instruction set to use (default: "bff")
- `--metrics-interval <usize>`: Compute and output metrics every N epochs (default: 1)

#### Scenario: Minimal invocation
- **WHEN** the user runs `complife --seed 42 --epochs 1000`
- **THEN** the simulation SHALL run for 1000 epochs with default parameters using BFF
- **AND** CSV metrics SHALL be printed to stdout

#### Scenario: Custom parameters
- **WHEN** the user runs `complife --seed 42 --epochs 5000 --mutation-rate 0 --population-size 1024 --step-limit 4096`
- **THEN** the simulation SHALL use the specified parameters instead of defaults

### Requirement: CLI Progress Reporting

The system SHALL print a brief progress indicator to stderr so the user knows the simulation is running. At minimum, the current epoch number SHALL be reported periodically.

#### Scenario: Progress output
- **WHEN** a long simulation is running
- **THEN** progress updates SHALL be written to stderr (not stdout, to avoid mixing with CSV data)

