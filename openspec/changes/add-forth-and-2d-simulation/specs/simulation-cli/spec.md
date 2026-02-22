## MODIFIED Requirements

### Requirement: CLI Configuration

The system SHALL provide a command-line interface that accepts the following parameters:
- `--seed <u64>`: Random seed for reproducibility (required)
- `--population-size <usize>`: Number of programs (default: 131072). Ignored when `--grid` is specified.
- `--program-size <usize>`: Bytes per program (default: 64)
- `--epochs <usize>`: Number of epochs to run (required)
- `--step-limit <usize>`: Max steps per program execution (default: 8192)
- `--mutation-rate <f64>`: Per-byte mutation probability per epoch (default: 0.00024)
- `--substrate <string>`: Which instruction set to use (default: "bff"). Supported values: "bff", "forth".
- `--metrics-interval <usize>`: Compute and output metrics every N epochs (default: 1)
- `--grid <WxH>`: Enable 2D spatial simulation on a W-by-H grid (optional). When provided, population size is W*H.

#### Scenario: Minimal invocation
- **WHEN** the user runs `complife --seed 42 --epochs 1000`
- **THEN** the simulation SHALL run for 1000 epochs with default parameters using BFF
- **AND** CSV metrics SHALL be printed to stdout

#### Scenario: Custom parameters
- **WHEN** the user runs `complife --seed 42 --epochs 5000 --mutation-rate 0 --population-size 1024 --step-limit 4096`
- **THEN** the simulation SHALL use the specified parameters instead of defaults

#### Scenario: Forth substrate selection
- **WHEN** the user runs `complife --seed 42 --epochs 1000 --substrate forth`
- **THEN** the simulation SHALL use the Forth interpreter

#### Scenario: 2D grid mode
- **WHEN** the user runs `complife --seed 42 --epochs 5000 --grid 240x135 --substrate forth`
- **THEN** a 2D spatial simulation SHALL run with Forth on a 240x135 grid
