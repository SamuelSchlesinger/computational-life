## MODIFIED Requirements

### Requirement: CLI Configuration

The system SHALL provide a command-line interface that accepts the following parameters:
- `--seed <u64>`: Random seed for reproducibility (required)
- `--population-size <usize>`: Number of programs (default: 131072) — only used in 0D mode
- `--program-size <usize>`: Bytes per program (default: 64)
- `--epochs <usize>`: Number of epochs to run (required)
- `--step-limit <usize>`: Max steps per program execution (default: 8192)
- `--mutation-rate <f64>`: Per-byte mutation probability per epoch (default: 0.00024)
- `--substrate <string>`: Which instruction set to use (default: "bff")
- `--metrics-interval <usize>`: Compute and output metrics every N epochs (default: 25)
- `--surface <spec>`: Run simulation on a surface (`flat:WxH`, `sphere:N`, `torus:MxN`, `obj:path`) **BREAKING: replaces `--grid`**
- `--neighbor-radius <f32>`: Geodesic neighbor radius in mesh units (default: auto-computed as 2x average edge length)
- `--blur <f32>`: Spatial blur strength for live viewer (default: 0.0)

When `--surface` is not provided, the simulation runs in 0D mode (flat soup, no spatial structure). When `--surface` is provided, the population size is determined by the mesh face count and `--population-size` is ignored.

#### Scenario: Minimal invocation
- **WHEN** the user runs `complife --seed 42 --epochs 1000`
- **THEN** the simulation SHALL run for 1000 epochs with default parameters using BFF
- **AND** CSV metrics SHALL be printed to stdout

#### Scenario: Custom parameters
- **WHEN** the user runs `complife --seed 42 --epochs 5000 --mutation-rate 0 --population-size 1024 --step-limit 4096`
- **THEN** the simulation SHALL use the specified parameters instead of defaults

#### Scenario: Surface simulation
- **WHEN** the user runs `complife --seed 42 --epochs 5000 --surface sphere:5`
- **THEN** the simulation SHALL run on an icosphere with subdivision level 5
- **AND** the population size SHALL be determined by the mesh (20480 faces)

#### Scenario: Flat grid via surface flag
- **WHEN** the user runs `complife --seed 42 --epochs 5000 --surface flat:240x135`
- **THEN** the simulation SHALL run on a flat grid equivalent to the legacy `--grid 240x135`

#### Scenario: OBJ mesh via surface flag
- **WHEN** the user runs `complife --seed 42 --epochs 5000 --surface obj:shapes/brain.obj`
- **THEN** the simulation SHALL load the mesh and run with population size equal to the face count

#### Scenario: Custom neighbor radius
- **WHEN** the user runs `complife --seed 42 --epochs 5000 --surface sphere:5 --neighbor-radius 0.3`
- **THEN** the geodesic neighbor radius SHALL be set to 0.3 mesh units instead of the auto-computed default
