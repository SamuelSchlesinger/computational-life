## ADDED Requirements

### Requirement: 2D Grid Rendering

When a 2D simulation is running in `--live` mode, the system SHALL render the grid as a 2D image where each program is represented by a colored tile. The tile color SHALL be derived from the program's byte contents to create a visual fingerprint that distinguishes different programs. Programs with identical contents SHALL have identical colors.

#### Scenario: Grid display
- **WHEN** a 2D simulation runs with `--live --grid 240x135`
- **THEN** the visualization SHALL show a 240x135 grid of colored tiles
- **AND** the grid SHALL update as epochs progress

#### Scenario: Replicator wave visible
- **WHEN** a self-replicator emerges and spreads across the grid
- **THEN** neighboring cells taken over by the replicator SHALL display similar colors, forming a visible wavefront

### Requirement: Grid Color Mapping

Each program SHALL be mapped to a color by hashing or sampling its byte contents. The mapping SHALL ensure that:
- Identical programs produce identical colors
- Different programs produce visually distinguishable colors (with high probability)
- The color space is perceptually varied (not all similar hues)

#### Scenario: Visual differentiation
- **WHEN** the grid contains a mix of random and replicated programs
- **THEN** random programs SHALL appear as varied colors
- **AND** clusters of identical replicators SHALL appear as uniform colored regions

### Requirement: Combined Grid and Metrics View

In 2D `--live` mode, the system SHALL display both the grid visualization and the time-series metrics plots (HOE, unique programs, zero count). The grid SHALL be the primary visual element, with metrics plots in a side or bottom panel.

#### Scenario: Full 2D live view
- **WHEN** a 2D simulation runs with `--live`
- **THEN** the window SHALL show the grid as the main content area
- **AND** HOE, unique programs, and zero count plots SHALL be visible in a panel

### Requirement: Grid Update Frequency

The grid visualization SHALL update at a rate that keeps the UI responsive. The simulation thread SHALL send grid snapshots at the configured metrics interval. For large grids, the system SHALL avoid sending full program data every frame — instead, a compact color representation SHALL be sent.

#### Scenario: Performance with large grid
- **WHEN** a 240x135 grid simulation runs with `--live`
- **THEN** the UI SHALL maintain at least 30 FPS
- **AND** grid updates SHALL occur at the metrics interval rate
