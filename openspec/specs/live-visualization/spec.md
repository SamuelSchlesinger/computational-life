# live-visualization Specification

## Purpose
TBD - created by archiving change add-live-visualization. Update Purpose after archive.
## Requirements
### Requirement: Live Visualization Mode

The application SHALL launch directly into a graphical menu screen (no CLI flags required). The menu SHALL allow the user to configure all simulation parameters before starting. When the user clicks "Start Simulation", the application SHALL transition to the simulation view with 3D surface rendering and a metrics sidebar. A "Back to Menu" button SHALL allow returning to reconfigure and restart.

The menu SHALL provide controls for:
- Substrate selection (BFF, Forth, Subleq, Rsubleq4)
- Surface type and parameters (sphere subdivisions, torus segments, grid dimensions, hamster tunnel config)
- Surface seed (u64, default 42)
- Program size (default 64 bytes)
- Step limit (default 8192)
- Mutation rate (default 0.00024)
- Max epochs (default 100,000)
- Metrics interval (default 25 epochs)
- Color mode (default Hash)
- Blur strength (default 0.0)

A help overlay (toggled via ? button) SHALL describe the simulation and controls.

#### Scenario: Launch application
- **WHEN** the application starts
- **THEN** a GUI menu SHALL be displayed with all configurable parameters
- **AND** the user SHALL be able to adjust settings and start a simulation

#### Scenario: Return to menu
- **WHEN** the user clicks "Back to Menu" during simulation
- **THEN** the simulation SHALL stop and the menu SHALL reappear with previous settings preserved

### Requirement: HOE Time Series Plot

The system SHALL display a line chart of high-order entropy (HOE) over epochs in the sidebar, updated live as the simulation progresses. Data points SHALL be decimated to at most 1000 points for rendering performance.

#### Scenario: HOE plot updates
- **WHEN** the simulation advances by one or more metrics intervals
- **THEN** the HOE line chart SHALL update to include the new data points

### Requirement: Unique Programs Time Series Plot

The system SHALL display a line chart of unique program count over epochs in the sidebar, updated live. Data points SHALL be decimated to at most 1000 points.

#### Scenario: Unique programs plot shows state transition
- **WHEN** self-replicators emerge and the population becomes less diverse
- **THEN** the unique programs line SHALL show a visible drop

### Requirement: Zero Count Time Series Plot

The system SHALL display a line chart of zero byte count over epochs in the sidebar, updated live. Data points SHALL be decimated to at most 1000 points.

#### Scenario: Zero count plot updates
- **WHEN** the simulation advances
- **THEN** the zero count line chart SHALL reflect the current number of zero bytes in the population

### Requirement: Simulation Runs on Dedicated Thread

The simulation SHALL run on a dedicated thread, separate from the bevy render loop. The simulation thread SHALL send two types of snapshots to the render thread via channels:

1. **EpochMetrics**: HOE, unique count, zero count, and byte histogram — sent every `metrics_interval` epochs
2. **SurfaceSnapshot**: Per-cell RGBA colors — sent at approximately 60Hz (gated by 16ms timer)

Control commands (play, pause, set color mode, set blur, reset surface, request program) SHALL be sent from the render thread to the simulation thread via a command channel. There SHALL be no shared mutable state between the threads.

#### Scenario: UI stays responsive during heavy simulation
- **WHEN** the simulation is running on a large mesh (e.g., 5120 faces)
- **THEN** the UI SHALL remain responsive and re-render smoothly regardless of simulation computation time

#### Scenario: Decoupled snapshot cadence
- **WHEN** the metrics interval is set to 25
- **THEN** color snapshots SHALL still update at ~60Hz independent of metrics computation

### Requirement: Playback Controls

The system SHALL provide playback controls in the sidebar:
- A play/pause toggle button
- A display of the current epoch number

#### Scenario: Pause and resume
- **WHEN** the user clicks the pause button
- **THEN** the simulation thread SHALL stop advancing epochs
- **AND WHEN** the user clicks play
- **THEN** the simulation SHALL resume from where it stopped

### Requirement: 3D Surface Rendering

The system SHALL render the simulation surface as a 3D triangle mesh using bevy. Each face SHALL be colored according to the active color mode. Vertex colors SHALL be updated from the latest surface snapshot. The mesh SHALL be lit by a directional light and ambient light.

#### Scenario: Surface displays program colors
- **WHEN** the simulation is running
- **THEN** each mesh face SHALL be colored based on the program it contains and the active color mode

#### Scenario: Color updates are smooth
- **WHEN** new color snapshots arrive from the sim thread
- **THEN** vertex colors SHALL be updated in the mesh without rebuilding geometry

### Requirement: Orbit Camera Controls

The system SHALL provide orbit camera controls for the 3D view:
- Left-click drag: orbit around the focus point (yaw and pitch)
- Right-click drag: pan the focus point
- Scroll wheel: zoom in/out (distance multiplied by 1.0 - scroll * 0.03, minimum distance 0.05)

Pitch SHALL be clamped to avoid gimbal lock. Camera input SHALL be blocked when the cursor is over egui widgets or the sidebar panel.

#### Scenario: Orbit camera
- **WHEN** the user left-click drags on the 3D viewport
- **THEN** the camera SHALL orbit around the mesh center

#### Scenario: Zoom limits
- **WHEN** the user scrolls to zoom in
- **THEN** the camera distance SHALL not go below 0.05

### Requirement: Color Mode Selection

The system SHALL support 7 color modes selectable at runtime via a dropdown in the sidebar:

1. **Hash**: FNV-1a hash of program bytes mapped to RGB
2. **Entropy**: Shannon entropy of byte distribution per program, mapped through a heatmap (blue-cyan-green-yellow-red)
3. **Zeros**: Fraction of zero bytes per program, mapped through heatmap
4. **NeighborSimilarity**: Average Hamming distance to geodesic neighbors (1 - normalized), mapped through heatmap
5. **InstructionDensity**: Fraction of bytes that are valid instructions (substrate-specific via `is_instruction`), mapped through heatmap
6. **UniqueBytes**: Count of distinct byte values per program (normalized to 0-1), mapped through heatmap
7. **TerritorialDominance**: Fraction of geodesic neighbors with identical program content, mapped through heatmap

The default color mode SHALL be Hash. Changing the color mode SHALL take effect immediately without restarting the simulation.

#### Scenario: Switch color mode at runtime
- **WHEN** the user selects "Entropy" from the color mode dropdown
- **THEN** the surface colors SHALL update to reflect byte entropy per program
- **AND** the simulation SHALL continue uninterrupted

### Requirement: Spatial Blur Effect

The system SHALL support a spatial blur effect controlled by a slider in the sidebar (range 0.0 to 1.0, default 0.0). Blur SHALL operate in face-adjacency space: each face's color is blended with its edge-adjacent neighbors' colors using the blur parameter as the interpolation weight.

#### Scenario: Blur disabled
- **WHEN** blur is set to 0.0
- **THEN** face colors SHALL be unmodified

#### Scenario: Blur enabled
- **WHEN** blur is set to a positive value
- **THEN** face colors SHALL be smoothed with adjacent face colors

