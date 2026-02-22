## ADDED Requirements

### Requirement: 2D Grid Population Layout

The system SHALL support arranging programs on a 2D grid of configurable width and height. The total population size SHALL equal `width * height`. Each grid cell SHALL contain exactly one program. The default grid size SHALL be 240x135 (32400 programs), matching Section 2.2 of the paper.

#### Scenario: Grid initialization
- **WHEN** a 2D simulation is initialized with grid size 240x135 and a seed
- **THEN** 32400 programs SHALL be created, each of the configured program size
- **AND** each program SHALL be filled with uniformly random bytes from the seeded RNG

#### Scenario: Custom grid size
- **WHEN** the user specifies `--grid 100x100`
- **THEN** the grid SHALL be 100x100 with 10000 programs

### Requirement: Chebyshev-Distance-2 Neighbor Interaction

The system SHALL restrict interactions to programs within Chebyshev distance 2 on the grid. Two programs at coordinates `(x0, y0)` and `(x1, y1)` SHALL be eligible to interact if and only if `|x0 - x1| <= 2` AND `|y0 - y1| <= 2`. The grid SHALL NOT wrap (no toroidal topology).

#### Scenario: Neighbor selection
- **WHEN** a program at position (5, 5) is selected for interaction
- **THEN** its partner SHALL be chosen uniformly at random from grid cells within Chebyshev distance 2
- **AND** the partner SHALL satisfy `|x - 5| <= 2` and `|y - 5| <= 2`

#### Scenario: Edge programs have fewer neighbors
- **WHEN** a program at position (0, 0) on a 240x135 grid is selected
- **THEN** its neighbor pool SHALL only include cells in the range (0..2, 0..2), excluding itself

### Requirement: 2D Epoch Execution

Each epoch in a 2D simulation SHALL iterate through all programs in a random order. For each program `P`, one of its Chebyshev-distance-2 neighbors `N` SHALL be selected uniformly at random. If neither `P` nor `N` has been "taken" (already interacted) in this epoch, they SHALL interact using the standard concatenation-execution-split rule. Programs that are not selected for interaction in an epoch still participate in background mutation.

#### Scenario: Epoch with pairing exclusion
- **WHEN** an epoch runs on a 2D grid
- **THEN** programs SHALL be iterated in a shuffled order
- **AND** each program SHALL only interact once per epoch (marked "taken" after interaction)
- **AND** programs whose selected neighbor is already taken SHALL be skipped

#### Scenario: Mutation still applies to all
- **WHEN** an epoch completes on a 2D grid
- **THEN** background mutation SHALL apply to ALL programs, including those that did not interact

### Requirement: 2D Simulation CLI Integration

The system SHALL accept a `--grid WxH` CLI argument to enable 2D spatial simulation mode. When `--grid` is provided, the `--population-size` argument SHALL be ignored (population size is `W * H`). The 2D mode SHALL work with any substrate.

#### Scenario: Launching 2D simulation
- **WHEN** the user runs `complife --seed 42 --epochs 5000 --grid 240x135 --substrate forth`
- **THEN** a 2D spatial simulation SHALL run with 32400 Forth programs on a 240x135 grid

#### Scenario: 2D mode with BFF
- **WHEN** the user runs `complife --seed 42 --epochs 5000 --grid 100x100 --substrate bff`
- **THEN** a 2D BFF simulation SHALL run on a 100x100 grid
