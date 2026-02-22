## ADDED Requirements

### Requirement: Color Mode Selection

The live visualization SHALL support multiple color modes for surface cells, selectable at runtime via the GUI without restarting the simulation. The available modes SHALL be:

- **Hash**: FNV hash of program bytes mapped to RGB (current default behavior).
- **Hamming weight**: total popcount of all program bytes mapped to a heatmap gradient.
- **Byte entropy**: Shannon entropy of the program's byte frequency distribution mapped to a heatmap gradient.
- **Zero fraction**: proportion of zero-valued bytes mapped to a dark-to-bright gradient.
- **Change delta**: Hamming distance between current and previous-epoch program bytes mapped to bright (high change) vs dark (stable).

The default mode SHALL be Hash.

#### Scenario: User switches color mode at runtime
- **WHEN** the user selects a different color mode from the GUI control
- **THEN** the surface colors SHALL update to reflect the new mode within the next snapshot interval (~16ms)
- **AND** no simulation restart SHALL occur

#### Scenario: Change delta mode tracks per-epoch differences
- **WHEN** the color mode is set to Change delta
- **THEN** cells that changed significantly since the previous epoch SHALL appear bright
- **AND** cells that are stable SHALL appear dark

#### Scenario: Default mode on startup
- **WHEN** the live visualization starts
- **THEN** the color mode SHALL default to Hash

### Requirement: Runtime Blur Control

The live visualization SHALL provide a blur strength slider in the GUI side panel, allowing the user to adjust spatial blur at runtime. The `--blur` CLI flag SHALL set the initial slider value. Changes to the slider SHALL take effect on the next snapshot.

#### Scenario: Adjusting blur at runtime
- **WHEN** the user moves the blur slider
- **THEN** the spatial blur strength SHALL update on the next rendered snapshot
- **AND** the CLI `--blur` value SHALL serve only as the initial position

### Requirement: Visualization Settings Panel

The live visualization side panel SHALL include a "Visualization" section containing the color mode selector and the blur strength slider, grouped above the metrics plots.

#### Scenario: Settings visible in surface mode
- **WHEN** the live visualization is running in surface mode
- **THEN** the side panel SHALL display a Visualization section with a color mode selector and a blur slider
