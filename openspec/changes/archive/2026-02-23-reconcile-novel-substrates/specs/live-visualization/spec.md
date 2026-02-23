## MODIFIED Requirements

### Requirement: Live Visualization Mode

The application SHALL launch directly into a graphical menu screen (no CLI flags required). The menu SHALL allow the user to configure all simulation parameters before starting. When the user clicks "Start Simulation", the application SHALL transition to the simulation view with 3D surface rendering and a metrics sidebar. A "Back to Menu" button SHALL allow returning to reconfigure and restart.

The menu SHALL provide controls for:
- Substrate selection (BFF, Forth, Subleq, Rsubleq4, Qop, Skim, Rig, Bits)
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

### Requirement: Color Mode Selection

The system SHALL support 7 color modes selectable at runtime via a dropdown in the sidebar:

1. **Hash**: FNV-1a hash of program bytes mapped to RGB (displayed as "Hash")
2. **Byte Entropy**: Shannon entropy of byte distribution per program, mapped through a heatmap (blue-cyan-green-yellow-red) (displayed as "Byte Entropy")
3. **Zero Fraction**: Fraction of zero bytes per program, mapped to grayscale where fewer zeros are brighter (displayed as "Zero Fraction")
4. **Neighbor Similarity**: Average Hamming distance to geodesic neighbors (1 - normalized), mapped through heatmap (displayed as "Neighbor Similarity")
5. **Instruction Density**: Fraction of bytes that are valid instructions (substrate-specific via `is_instruction`), mapped through heatmap (displayed as "Instruction Density")
6. **Unique Bytes**: Count of distinct byte values per program (normalized to 0-1), mapped through heatmap (displayed as "Unique Bytes")
7. **Territorial Dominance**: Fraction of geodesic neighbors with identical program content, mapped through heatmap (displayed as "Territorial Dominance")

The default color mode SHALL be Hash. Changing the color mode SHALL take effect immediately without restarting the simulation.

#### Scenario: Switch color mode at runtime
- **WHEN** the user selects "Byte Entropy" from the color mode dropdown
- **THEN** the surface colors SHALL update to reflect byte entropy per program
- **AND** the simulation SHALL continue uninterrupted

#### Scenario: Zero Fraction uses grayscale
- **WHEN** the user selects "Zero Fraction" color mode
- **THEN** cells with more zero bytes SHALL appear darker
- **AND** cells with fewer zero bytes SHALL appear brighter
