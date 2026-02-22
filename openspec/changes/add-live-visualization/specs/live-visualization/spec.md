## ADDED Requirements

### Requirement: Live Visualization Mode

The system SHALL support a `--live` CLI flag (available when compiled with the `viz` feature) that launches a graphical window displaying live-updating plots of simulation metrics. When `--live` is passed, the system SHALL NOT print CSV to stdout; instead, metrics SHALL be displayed in the graphical UI.

#### Scenario: Launching live mode
- **WHEN** the user runs `complife --seed 42 --epochs 5000 --live`
- **THEN** a graphical window SHALL open displaying simulation plots
- **AND** the simulation SHALL begin advancing automatically

#### Scenario: Feature not enabled
- **WHEN** the binary is compiled without the `viz` feature and `--live` is passed
- **THEN** the system SHALL print an error message and exit

### Requirement: HOE Time Series Plot

The system SHALL display a line chart of high-order entropy (HOE) over epochs, updated live as the simulation progresses. This corresponds to the complexity line in Figure 1 of the paper.

#### Scenario: HOE plot updates
- **WHEN** the simulation advances by one or more epochs
- **THEN** the HOE line chart SHALL update to include the new data points

### Requirement: Unique Programs Time Series Plot

The system SHALL display a line chart of unique program count over epochs, updated live. This corresponds to the "unique tokens" line in Figure 1 of the paper.

#### Scenario: Unique programs plot shows state transition
- **WHEN** self-replicators emerge and the population becomes less diverse
- **THEN** the unique programs line SHALL show a visible drop

### Requirement: Zero Count Time Series Plot

The system SHALL display a line chart of zero byte count over epochs, updated live. This tracks the "zero-poisoning" phenomenon from Figure 2 of the paper.

#### Scenario: Zero count plot updates
- **WHEN** the simulation advances
- **THEN** the zero count line chart SHALL reflect the current number of zero bytes in the population

### Requirement: Byte Frequency Histogram Plot

The system SHALL display a bar chart showing the frequency of each byte value (0-255) across the entire population, updated live. This reveals which instruction bytes dominate as replicators take over.

#### Scenario: Histogram shows instruction bias
- **WHEN** BFF replicators have emerged
- **THEN** the histogram SHALL show peaks at BFF instruction byte values (e.g., 0x2E for '.', 0x5B for '[')

### Requirement: Simulation Runs on Dedicated Thread

The simulation SHALL run on a dedicated thread, separate from the bevy render loop. The simulation thread SHALL send metrics snapshots to the render thread via a channel. Control commands (play/pause, speed) SHALL be sent from the render thread to the simulation thread via a separate channel. There SHALL be no shared mutable state between the threads.

#### Scenario: UI stays responsive during heavy simulation
- **WHEN** the simulation is running with a large population (e.g., 2^17 programs)
- **THEN** the UI SHALL remain responsive and re-render smoothly regardless of simulation computation time

### Requirement: Playback Controls

The system SHALL provide playback controls in the UI:
- A play/pause toggle
- A speed control (target epochs per second or a relative speed slider)
- A display of the current epoch number and elapsed wall time

#### Scenario: Pause and resume
- **WHEN** the user clicks the pause button
- **THEN** the simulation thread SHALL stop advancing epochs
- **AND WHEN** the user clicks play
- **THEN** the simulation SHALL resume from where it stopped

#### Scenario: Adjust speed
- **WHEN** the user adjusts the speed slider
- **THEN** the simulation thread SHALL adjust its pacing to match the target rate

### Requirement: Benchmark Mode

The system SHALL support a `--benchmark` CLI flag (available in headless mode, no `viz` feature required) that runs the simulation for the specified number of epochs and reports throughput in epochs/sec and interactions/sec to stderr. This provides baseline performance data for future optimization work.

#### Scenario: Benchmark output
- **WHEN** the user runs `complife --seed 42 --epochs 100 --benchmark`
- **THEN** the system SHALL run the simulation and print throughput statistics to stderr upon completion
