## ADDED Requirements

### Requirement: 3D Surface Rendering

When `--live` is used with `--surface`, the system SHALL render the simulation mesh as a 3D object in the bevy window. Each triangular face SHALL be colored according to its program's byte-hash (same color mapping as the 2D grid view). Face colors SHALL update live as the simulation progresses, using the same ~60fps throttling as the 2D grid view.

#### Scenario: Sphere rendering
- **WHEN** the user runs `complife --surface sphere:4 --live --seed 42 --epochs 50000`
- **THEN** a 3D colored sphere SHALL be rendered in the window
- **AND** face colors SHALL update as programs evolve

#### Scenario: Imported mesh rendering
- **WHEN** the user runs `complife --surface obj:shapes/complex.obj --live --seed 42 --epochs 50000`
- **THEN** the imported mesh SHALL be rendered in 3D with per-face program coloring

### Requirement: Orbit Camera Controls

When viewing a 3D surface simulation, the system SHALL provide mouse-based camera controls:
- **Left-click drag**: Orbit (rotate) the camera around the mesh center
- **Right-click drag**: Pan the camera
- **Scroll wheel**: Zoom in and out

The camera SHALL start at a default distance that frames the entire mesh in view.

#### Scenario: Orbit around sphere
- **WHEN** the user left-click-drags on the viewport
- **THEN** the camera SHALL orbit around the mesh center, allowing the user to view all sides

#### Scenario: Zoom into detail
- **WHEN** the user scrolls the mouse wheel forward
- **THEN** the camera SHALL zoom in toward the mesh surface
- **AND** individual face colors SHALL become visible at close range

### Requirement: Surface Blur Effect

When `--blur` is used with `--surface`, the system SHALL apply a spatial blur to face colors in face-adjacency space. Each face's rendered color SHALL be blended with the average color of its edge-adjacent faces: `out = (1 - alpha) * face_color + alpha * avg(adjacent_face_colors)`. This uses the face adjacency table (direct edge-sharing neighbors, typically 3 per interior triangle), not the geodesic neighbor table.

#### Scenario: Blur on sphere
- **WHEN** the user runs `complife --surface sphere:5 --live --blur 0.3 --seed 42 --epochs 50000`
- **THEN** face colors SHALL appear blended with their edge-adjacent neighbors
- **AND** the visual effect SHALL soften boundaries between differently-colored regions

#### Scenario: Blur disabled
- **WHEN** the user runs `--surface sphere:5 --live` without `--blur`
- **THEN** face colors SHALL be unblurred (identical to current behavior)

### Requirement: 3D View Metrics Panel

When viewing a 3D surface simulation with `--live`, the system SHALL display a metrics side panel alongside the 3D viewport, showing the same time-series plots as the 2D view (HOE, unique programs, zero byte count) and playback controls.

#### Scenario: Metrics alongside 3D view
- **WHEN** a surface simulation is running with `--live`
- **THEN** a right-side panel SHALL display HOE, unique program count, and zero byte count plots
- **AND** play/pause controls and epoch counter SHALL be visible
