## MODIFIED Requirements

### Requirement: Population Initialization

The system SHALL initialize a population of programs on a surface mesh, with one program per mesh face. Each program SHALL consist of `P` bytes filled with uniformly random bytes from a seeded random number generator. The default program size SHALL be P=64. Population size is determined by the mesh face count. Initializing with the same seed and mesh SHALL produce the identical population.

#### Scenario: Surface population initialization
- **WHEN** a simulation is initialized on an icosphere with 5120 faces, program size 64, and seed 42
- **THEN** the population SHALL contain 5120 programs of 64 bytes each
- **AND** each byte SHALL be drawn from a uniform distribution over 0..255
- **AND** initializing with the same seed and mesh SHALL produce the identical population

### Requirement: Primordial Soup Interaction Loop

The system SHALL implement a spatial interaction loop where programs interact with geodesic neighbors on the surface mesh. Each epoch:

1. Shuffle cell iteration order
2. For each unpaired cell, select a random geodesic neighbor
3. Create ordered pairs (first, second) where each cell participates in at most one pair per epoch
4. For each pair: concatenate programs into a 2P-byte tape, execute via the substrate in parallel (using rayon), and copy the resulting halves back to the respective cells

Parallel execution SHALL process pairs in chunks for efficiency.

#### Scenario: One epoch of spatial interactions
- **WHEN** an epoch is run on a surface with N faces
- **THEN** pairs SHALL be formed from shuffled cells and their geodesic neighbors
- **AND** each cell SHALL participate in at most one pair per epoch
- **AND** pair execution SHALL be parallelized

#### Scenario: Deterministic reproduction
- **WHEN** two simulations are run with identical parameters, mesh, and seed
- **THEN** they SHALL produce identical populations at every epoch

### Requirement: Background Mutation

The system SHALL support background mutation after each epoch. Each byte in the population SHALL be independently subject to a single-bit flip (XOR with 1 << random_bit) with a configurable per-byte probability. The default mutation rate SHALL be 0.024% (0.00024). Mutation rate of 0 SHALL disable mutations entirely.

Mutation SHALL use geometric distribution sampling to skip directly to the next mutation site, reducing RNG calls from O(total_bytes) to O(total_bytes x mutation_rate).

#### Scenario: Mutation at default rate
- **WHEN** an epoch completes with mutation rate 0.00024
- **THEN** each byte in the population SHALL be independently subject to a single-bit flip with that probability

#### Scenario: Mutation disabled
- **WHEN** mutation rate is set to 0
- **THEN** no bytes SHALL be mutated between epochs

## REMOVED Requirements

### Requirement: Epoch Metrics Output
**Reason**: The application no longer outputs CSV to stdout. Metrics are computed and sent to the visualization thread via channels. Metrics display is now covered by the live-visualization spec.
**Migration**: Metrics are accessible through the GUI sidebar plots.
