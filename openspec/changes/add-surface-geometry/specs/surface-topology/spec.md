## ADDED Requirements

### Requirement: Surface Mesh Representation

The system SHALL represent simulation surfaces as triangle meshes. Each triangular face SHALL host exactly one program. The mesh SHALL store vertex positions (for 3D rendering), face centroids (for distance computation), and a face adjacency table (for determining direct neighbors). Face adjacency SHALL be defined as two faces sharing an edge (two common vertices).

#### Scenario: Icosphere mesh
- **WHEN** an icosphere with subdivision level 4 is generated
- **THEN** the mesh SHALL contain 5120 triangular faces
- **AND** each face SHALL have exactly 3 edge-adjacent faces

#### Scenario: Program-to-face mapping
- **WHEN** a simulation is initialized on a mesh with N faces
- **THEN** the population SHALL contain exactly N programs, one per face

### Requirement: Geodesic Distance Neighborhoods

The system SHALL compute program neighborhoods using geodesic distance on the mesh's dual graph. Edge weights SHALL be the Euclidean distance between face centroids. For each face, the system SHALL precompute all faces within a configurable radius using Dijkstra's algorithm and store them in a flat lookup table for O(1) access during simulation. The default radius SHALL be 2x the average edge length of the mesh. The user MAY override this with the `--neighbor-radius` CLI flag.

#### Scenario: Geodesic neighbors on icosphere
- **WHEN** a simulation runs on an icosphere with the default neighbor radius
- **THEN** each face's neighborhood SHALL include all faces whose geodesic distance (shortest path through face centroids) is within the radius
- **AND** the neighborhood SHALL NOT include the face itself

#### Scenario: Narrow passage limits neighbors
- **WHEN** a mesh has a narrow passage connecting two larger regions
- **THEN** faces on opposite sides of the passage SHALL have fewer shared neighbors than faces in the open regions
- **AND** the passage SHALL act as a bottleneck for gene flow

#### Scenario: Neighbor table precomputation
- **WHEN** a mesh is loaded or generated
- **THEN** the neighbor table SHALL be computed once at startup
- **AND** no graph traversal SHALL occur during epoch execution

### Requirement: Procedural Sphere Generation

The system SHALL generate icosphere meshes from a subdivision level parameter. Subdivision 0 produces a regular icosahedron (20 faces). Each subsequent level subdivides every triangle into 4 triangles, producing 20 * 4^level faces.

#### Scenario: Subdivision levels
- **WHEN** the user specifies `--surface sphere:3`
- **THEN** the mesh SHALL contain 1280 triangular faces (20 * 4^3)
- **AND** all vertices SHALL lie on the unit sphere (normalized to radius 1)

### Requirement: Procedural Torus Generation

The system SHALL generate torus meshes from major and minor segment counts. A torus with M major segments and N minor segments SHALL produce 2*M*N triangular faces (M*N quads, each split into 2 triangles).

#### Scenario: Torus generation
- **WHEN** the user specifies `--surface torus:80x30`
- **THEN** the mesh SHALL contain 4800 triangular faces (2 * 80 * 30)
- **AND** the torus topology SHALL wrap in both directions (no boundary edges)

### Requirement: Flat Grid Surface

The system SHALL generate flat grid meshes from width and height parameters. A WxH grid SHALL produce W*H logical cells, each represented as a pair of triangles. Each cell pair SHALL host one program, maintaining population parity with the legacy `Soup2d` grid.

#### Scenario: Flat grid generation
- **WHEN** the user specifies `--surface flat:240x135`
- **THEN** the mesh SHALL produce 240 * 135 = 32400 programs
- **AND** the behavior SHALL be equivalent to the legacy `--grid 240x135` mode

### Requirement: OBJ Mesh Import

The system SHALL load triangle meshes from Wavefront OBJ files via the `--surface obj:<path>` syntax. Non-triangular faces (quads, ngons) SHALL be triangulated on load. The system SHALL reject meshes with non-manifold edges (edges shared by more than 2 faces) and report an error.

#### Scenario: Loading a valid OBJ
- **WHEN** the user specifies `--surface obj:shapes/brain.obj`
- **THEN** the system SHALL load the mesh, triangulate all faces, and build the face adjacency and geodesic neighbor tables
- **AND** the population size SHALL equal the number of triangular faces

#### Scenario: Non-manifold mesh rejected
- **WHEN** the user loads an OBJ file containing non-manifold edges
- **THEN** the system SHALL print an error message and exit

### Requirement: Surface Simulation Engine

The system SHALL implement a simulation engine for surface meshes that follows the same interaction protocol as the 2D grid simulation: shuffle cell order, pair each cell with a random neighbor from its precomputed geodesic neighborhood, execute all pairs in parallel, and copy results back. The simulation SHALL be deterministic given the same seed.

#### Scenario: Deterministic surface simulation
- **WHEN** two simulations are run on the same mesh with the same seed and parameters
- **THEN** they SHALL produce identical populations at every epoch

#### Scenario: Parallel execution
- **WHEN** an epoch is executed on a surface mesh
- **THEN** program pairs SHALL be executed in parallel via rayon
