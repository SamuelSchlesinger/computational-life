## ADDED Requirements

### Requirement: Decoupled Grid Snapshot Cadence

The 2D visualization sim thread SHALL send grid snapshot updates independently of expensive metrics (HOE, unique count, histogram). Grid snapshots SHALL be sent at a higher frequency than full metrics to keep the visual display responsive. The grid update SHALL contain only the pixel data needed for rendering; full metrics SHALL be sent at the configured `metrics_interval`.

#### Scenario: Grid updates faster than metrics
- **WHEN** a 2D simulation is running with `--live` and `metrics_interval=10`
- **THEN** the grid display SHALL update more frequently than every 10 epochs
- **AND** the time-series plots SHALL update at the `metrics_interval` rate

### Requirement: Bounded Grid Snapshot Memory

The visualization system SHALL store only the latest grid snapshot, not accumulate snapshots in the history. Time-series metric entries in the history SHALL NOT carry grid pixel data.

#### Scenario: Memory stays bounded over long runs
- **WHEN** a 2D simulation runs for 100,000 epochs with a 240x135 grid
- **THEN** only one grid snapshot (~130KB) SHALL be held in memory at any time
- **AND** the history entries SHALL consume memory proportional only to the number of scalar metrics stored

## MODIFIED Requirements

### Requirement: Simulation Runs on Dedicated Thread

The simulation SHALL run on a dedicated thread, separate from the bevy render loop. The simulation thread SHALL send metrics snapshots to the render thread via a channel. For 2D simulations, the simulation thread SHALL additionally send lightweight grid updates via the same or a separate channel. Control commands (play/pause, speed) SHALL be sent from the render thread to the simulation thread via a separate channel. There SHALL be no shared mutable state between the threads.

#### Scenario: UI stays responsive during heavy simulation
- **WHEN** the simulation is running with a large population (e.g., 2^17 programs)
- **THEN** the UI SHALL remain responsive and re-render smoothly regardless of simulation computation time

#### Scenario: 2D grid updates independently of metrics
- **WHEN** a 2D simulation thread completes an epoch
- **THEN** it MAY send a grid snapshot without computing expensive metrics
- **AND** the render thread SHALL update the grid texture upon receiving the snapshot
