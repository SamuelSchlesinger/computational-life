## MODIFIED Requirements

### Requirement: CLI Configuration

The system SHALL provide a graphical menu interface that allows the user to configure simulation parameters before starting. The menu SHALL accept the following settings:

- Substrate selection: BFF, Forth, Subleq, or Rsubleq4 (default: BFF)
- Surface type: Sphere, Torus, Flat Grid, or Hamster Tunnel with type-specific dimension controls
- Surface seed (u64, default: 42)
- Program size in bytes (default: 64)
- Step limit per execution (default: 8192)
- Mutation rate per byte per epoch (default: 0.00024)
- Max epochs (default: 100,000)
- Metrics interval in epochs (default: 25)
- Color mode (default: Hash)
- Blur strength (default: 0.0)

A "Start Simulation" button SHALL launch the simulation with the configured parameters. Returning to the menu SHALL preserve previously selected settings.

#### Scenario: Default configuration
- **WHEN** the application starts and the user clicks "Start Simulation" without changing defaults
- **THEN** the simulation SHALL run with BFF substrate on a sphere surface with default parameters

#### Scenario: Custom parameters
- **WHEN** the user selects Forth substrate, sets program size to 128, and starts the simulation
- **THEN** the simulation SHALL use the specified parameters

### Requirement: CLI Progress Reporting

The system SHALL display simulation progress in the sidebar during active simulation. The current epoch number SHALL be shown alongside the play/pause controls. Metrics plots (HOE, unique program count, zero byte count) SHALL update live at each metrics interval to indicate ongoing progress.

#### Scenario: Progress display
- **WHEN** a simulation is running
- **THEN** the sidebar SHALL show the current epoch number
- **AND** metrics plots SHALL update at the configured metrics interval
