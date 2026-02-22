## ADDED Requirements

### Requirement: Population Initialization

The system SHALL initialize a population of `N` programs, each consisting of `P` bytes, filled with uniformly random bytes from a seeded random number generator. The default values SHALL be N=2^17 (131072) and P=64.

#### Scenario: Default population initialization
- **WHEN** a simulation is initialized with default parameters and a given seed
- **THEN** the population SHALL contain 131072 programs of 64 bytes each
- **AND** each byte SHALL be drawn from a uniform distribution over 0..255
- **AND** initializing with the same seed SHALL produce the identical population

### Requirement: Primordial Soup Interaction Loop

The system SHALL implement the primordial soup interaction loop as described in Section 2.1 of the paper. Each epoch consists of `N` interaction steps (where `N` is the population size). In each step, two distinct programs SHALL be selected uniformly at random. They SHALL be concatenated in a randomly chosen order (AB or BA with equal probability), executed via the substrate, and the resulting tape SHALL be split back into two programs that replace the originals.

#### Scenario: One epoch of interactions
- **WHEN** an epoch is run on a population of size N
- **THEN** exactly N interaction steps SHALL be performed
- **AND** each step SHALL select two distinct programs uniformly at random
- **AND** concatenation order SHALL be chosen uniformly at random (AB or BA)

#### Scenario: Deterministic reproduction
- **WHEN** two simulations are run with identical parameters and the same seed
- **THEN** they SHALL produce identical populations at every epoch

### Requirement: Background Mutation

The system SHALL support optional background mutation. When enabled, after each epoch, each byte in the population SHALL be flipped (XOR with a random mask) with a configurable per-byte probability. The default mutation rate SHALL be 0.024% (approximately 3/8192 per byte per epoch, matching the paper). Mutation rate of 0 SHALL disable mutations entirely.

#### Scenario: Mutation at default rate
- **WHEN** an epoch completes with mutation rate 0.024%
- **THEN** each byte in the population SHALL be independently flipped with probability 0.00024

#### Scenario: Mutation disabled
- **WHEN** mutation rate is set to 0
- **THEN** no bytes SHALL be randomly mutated between epochs

### Requirement: Epoch Metrics Output

The system SHALL compute and output metrics after each epoch (or every K epochs, where K is configurable). Metrics SHALL include at minimum: epoch number and high-order entropy (HOE). Output SHALL be in CSV format to stdout or a file.

#### Scenario: CSV metrics output
- **WHEN** an epoch completes
- **THEN** a CSV row SHALL be emitted containing at least: epoch number, high-order entropy
