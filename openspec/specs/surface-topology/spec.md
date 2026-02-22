# surface-topology Specification

## Purpose
TBD - created by archiving change reconcile-specs-with-codebase. Update Purpose after archive.
## Requirements
### Requirement: Surface Mesh Representation

The system SHALL represent simulation surfaces as triangle meshes where each face holds one program. A `SurfaceMesh` SHALL store:

- Vertex positions (`Vec<[f32; 3]>`)
- Triangular face indices (`Vec<[usize; 3]>`)
- Pre-computed face centroids
- Edge-based face adjacency (two faces are adjacent if they share an edge)
- Pre-computed geodesic neighbor indices in a flat buffer with per-face range lookups

The mesh SHALL validate that all face vertex indices are in bounds and SHALL detect non-manifold edges (edges shared by more than 2 faces).

#### Scenario: Face count determines population size
- **WHEN** a surface mesh with 5120 faces is used
- **THEN** the simulation population SHALL contain exactly 5120 programs

#### Scenario: Non-manifold rejection
- **WHEN** mesh geometry contains an edge shared by more than 2 faces
- **THEN** construction SHALL return an error

### Requirement: Geodesic Distance Neighborhoods

The system SHALL compute neighborhoods using geodesic distance on the face adjacency graph. For each face, Dijkstra's algorithm SHALL find all faces whose centroid-to-centroid distance along the adjacency graph is within a configurable radius. The default radius SHALL be 4.0 times the average adjacent-centroid distance. Neighbor computation SHALL be parallelized via rayon. Neighbors SHALL exclude the face itself.

#### Scenario: Default neighborhood radius
- **WHEN** neighbors are computed with no explicit radius
- **THEN** the radius SHALL default to 4.0 times the average edge-adjacent centroid distance
- **AND** each face SHALL have at least one neighbor (assuming a connected mesh)

#### Scenario: Custom neighborhood radius
- **WHEN** a specific radius is provided
- **THEN** only faces within that geodesic distance SHALL be included as neighbors

### Requirement: Procedural Shape Generators

The system SHALL provide built-in procedural mesh generators:

1. **Icosphere**: Golden icosahedron with recursive subdivision. Subdivision level N produces 20 x 4^N faces. Vertices SHALL be normalized to the unit sphere.
2. **Torus**: Parametric torus with configurable major and minor segment counts (minimum 3 each). Major radius 1.0, minor radius 0.4. Produces 2 x major x minor faces.
3. **Flat Grid**: XY-plane grid with configurable width and height. Each cell is split into 2 triangles, producing 2 x width x height faces. Scaled so the longest dimension spans [-1, 1].
4. **Hamster Tunnel**: A loop of sphere nodes connected by tubes. Configurable number of spheres (minimum 3), segments per connection (minimum 3), and random seed. Uses nearest-neighbor heuristic for sphere ordering and holonomy-corrected frame transport for twist-free tubes. Sphere radius 0.4, tube radius 0.12.

#### Scenario: Icosphere subdivision
- **WHEN** an icosphere with subdivision level 4 is generated
- **THEN** the mesh SHALL have 20 x 4^4 = 5120 faces
- **AND** all vertices SHALL lie on the unit sphere

#### Scenario: Flat grid dimensions
- **WHEN** a 64x64 flat grid is generated
- **THEN** the mesh SHALL have 2 x 64 x 64 = 8192 faces

#### Scenario: Hamster tunnel generation
- **WHEN** a hamster tunnel with 10 spheres and 16 segments is generated with a given seed
- **THEN** the mesh SHALL form a closed loop of connected nodes
- **AND** the same seed SHALL produce the same geometry

### Requirement: OBJ Mesh Import

The system SHALL import triangle meshes from Wavefront OBJ files. The importer SHALL support vertex positions (`v`) and face definitions (`f`) with formats: `v`, `v/vt`, `v/vt/vn`, `v//vn`. Faces with more than 3 vertices SHALL be fan-triangulated. OBJ 1-indexed vertices SHALL be converted to 0-indexed internally.

#### Scenario: Import a quad mesh
- **WHEN** an OBJ file contains quad faces
- **THEN** each quad SHALL be split into 2 triangles via fan triangulation

#### Scenario: Invalid vertex reference
- **WHEN** an OBJ face references a vertex index beyond the vertex count
- **THEN** the importer SHALL return an error

