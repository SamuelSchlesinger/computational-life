## ADDED Requirements

### Requirement: 2D Grid Live Mode

When `--live` and `--grid WxH` are both specified, the visualization SHALL display the 2D grid view as the primary content, with time-series metrics in a side panel. The grid rendering SHALL use the grid-visualization capability for color mapping and update behavior.

#### Scenario: Launching 2D live mode
- **WHEN** the user runs `complife --seed 42 --epochs 5000 --grid 240x135 --live --substrate forth`
- **THEN** a graphical window SHALL open showing the 2D grid and metrics plots
- **AND** the grid SHALL update as the simulation progresses
