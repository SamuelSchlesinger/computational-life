## ADDED Requirements

### Requirement: 2D Simulation Allocation-Free Hot Loop

The `Soup2d` struct SHALL pre-allocate and reuse all scratch buffers needed by `run_epoch`, including the shuffled iteration order, taken-flags array, interaction tape, and neighbor lookup table. The `run_epoch` method SHALL NOT perform any heap allocations during execution. Program output SHALL be written back to the existing program buffers via `copy_from_slice` rather than allocating new vectors.

#### Scenario: No allocations during epoch execution
- **WHEN** `run_epoch` is called on a `Soup2d` with a 240x135 grid
- **THEN** the global allocator SHALL NOT be invoked during the method's execution
- **AND** the simulation results SHALL be identical to the previous allocating implementation for the same seed

### Requirement: Pre-Computed Neighbor Table

The `Soup2d` struct SHALL pre-compute the Chebyshev-distance-2 neighbor indices for every cell at construction time. The `run_epoch` method SHALL look up neighbors from this table rather than computing them per cell per epoch.

#### Scenario: Neighbor lookup from table
- **WHEN** `run_epoch` selects a cell for interaction
- **THEN** its neighbor set SHALL be retrieved from the pre-computed table in O(1) time
- **AND** the neighbor set SHALL be identical to the dynamically-computed result
