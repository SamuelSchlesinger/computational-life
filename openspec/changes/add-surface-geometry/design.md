## Context

The simulation currently has two modes: a 0D soup (`Soup`, flat population with global random pairing) and a 2D spatial soup (`Soup2d`, rectangular grid with Chebyshev-distance-2 neighborhoods). We want to generalize the 2D case to arbitrary surface meshes so that the interaction topology is determined by mesh geometry.

The visualization currently renders the 2D grid as a flat RGBA texture in an egui panel. For 3D surfaces, we need proper mesh rendering with a camera the user can orbit around.

## Goals / Non-Goals

**Goals:**
- Unify all spatial modes under `--surface` (flat grids, spheres, toruses, imported meshes)
- Use geodesic distance to define neighborhoods, precomputed at startup
- Support `--blur` on surfaces (blurring in face-adjacency space)
- Render the mesh in 3D with per-face coloring and mouse-controlled camera
- Maintain current performance characteristics (parallel epoch execution via rayon)

**Non-Goals:**
- Dynamic mesh modification during simulation
- Multi-resolution or adaptive mesh refinement
- Physics simulation or mesh deformation
- Supporting non-manifold or degenerate meshes

## Decisions

### Mesh representation

Each mesh is stored as:
- `vertices: Vec<[f32; 3]>` — vertex positions for rendering
- `faces: Vec<[usize; 3]>` — triangle indices (all meshes are triangulated)
- `face_centroids: Vec<[f32; 3]>` — centroid of each face (for distance computation)
- `face_adjacency: Vec<Vec<usize>>` — for each face, which faces share an edge
- `neighbor_indices: Vec<usize>` + `neighbor_ranges: Vec<(usize, usize)>` — precomputed geodesic neighbor table (same flat layout as `Soup2d`)

One program per face. Face index = program index.

**Why triangles?** Triangles are the universal primitive — OBJ files can contain quads/ngons which we triangulate on load. Icospheres are naturally triangulated. Toruses are generated as quads then split into triangles. This keeps the adjacency logic uniform.

**Alternative considered:** One program per vertex. Rejected because face-based is more natural for visualization (each face gets a solid color) and because face adjacency is well-defined (two faces are adjacent if they share an edge), while vertex adjacency on a mesh is less standard.

### Neighborhood computation: geodesic distance

For each face, compute shortest-path distances to all other faces on the dual graph (faces as nodes, edges weighted by Euclidean distance between face centroids). Include all faces within a configurable radius as neighbors.

**Algorithm:** Run Dijkstra from each face on the dual graph, pruning when distance exceeds the radius. This produces a sparse neighbor set per face.

**Complexity:** O(N * K * log N) where K is the average number of faces within the radius. For typical meshes with localized neighborhoods, K << N, so this is much cheaper than O(N^2).

**Storage:** Same flat `neighbor_indices` / `neighbor_ranges` layout as `Soup2d`. For N = 80K faces with ~24 neighbors each, that's ~2M entries — trivial memory.

**CLI parameter:** `--neighbor-radius <f32>` in mesh-space units. Default is computed automatically as 2x the average edge length of the mesh, which gives neighborhoods roughly comparable to k=2 on a regular mesh. The user can override this to experiment with tighter or broader interaction ranges.

**Why geodesic over k-hop?** On meshes with irregular face sizes (common in imported OBJ files and in regions where triangles are dense), k-hop produces inconsistent neighborhoods — a face in a dense region has many more k-hop neighbors covering less physical area than a face in a sparse region. Geodesic distance produces neighborhoods proportional to physical surface area, which is what matters for the evolutionary specialization hypothesis: programs in a narrow passage have fewer neighbors because the passage is physically small, not because it happens to have fewer triangles.

### Built-in shapes

1. **Flat grid** (`--surface flat:WxH`): Generates a W*H quad grid, each quad split into 2 triangles. Each quad-pair maps to one logical cell to maintain population parity with `Soup2d` (so `--surface flat:240x135` produces 240*135 = 32,400 programs, same as `--grid 240x135` did). The two triangles per cell share a single program and are colored identically.

2. **Sphere** (`--surface sphere:<subdivisions>`): Icosphere with N subdivision levels. Subdivision 0 = icosahedron (20 faces), each level quadruples face count (sub 1 = 80, sub 2 = 320, sub 3 = 1280, sub 4 = 5120, sub 5 = 20480, sub 6 = 81920). Subdivision 6 gives ~82K cells, comparable to a 286x286 grid.

3. **Torus** (`--surface torus:MxN`): Parametric torus with M major segments and N minor segments, producing 2*M*N triangular faces. E.g., `torus:100x40` = 8000 faces.

4. **OBJ import** (`--surface obj:path/to/file.obj`): Load arbitrary mesh from an OBJ file. Triangulate all faces on load.

### Blur on surfaces

The `--blur` flag works on surfaces by blurring in face-adjacency space rather than pixel space. After computing per-face colors from program hashes, each face's color is blended with the average color of its direct edge-adjacent faces (3 neighbors for interior triangles):

```
out_color = (1 - alpha) * face_color + alpha * avg(adjacent_face_colors)
```

This uses the face adjacency table (not the geodesic neighbor table), keeping the blur local and fast. The adjacency is always available since it's computed as part of mesh construction.

### OBJ mesh import

Use the `tobj` crate to load `.obj` files. Triangulate all faces on load. Build the face adjacency table from shared edges using a half-edge lookup (hash map from sorted vertex-pair to face index).

### 3D rendering

Replace the current egui texture approach with bevy's built-in 3D mesh rendering:
- Create a `Mesh` with per-vertex colors (each triangle's 3 vertices get the face's program color)
- Use `Camera3d` with orbit controls
- On each frame where a new snapshot arrives, update the vertex colors in the mesh
- Keep the metrics side panel in egui (right panel, same as current 2D view)

For camera controls, use the `bevy_panorbit_camera` crate which provides orbit, pan, and zoom out of the box.

### Unified `--surface` CLI design

```
# Flat grid (replaces --grid):
complife --surface flat:240x135 --live ...

# Sphere:
complife --surface sphere:5 --live ...

# Torus:
complife --surface torus:100x40 --live ...

# OBJ import:
complife --surface obj:shapes/brain.obj --live ...

# 0D mode (no spatial structure, unchanged):
complife --seed 42 --epochs 5000 ...
```

When `--surface` is not provided, the simulation runs in 0D mode (flat soup, no spatial structure).

### Code organization

- **`src/surface.rs`**: `SurfaceMesh` struct (vertices, faces, adjacency, centroids), procedural generators (`icosphere`, `torus`, `flat_grid`), OBJ loader, geodesic neighbor table builder. Also `SoupSurface` simulation struct (analogous to `Soup2d`).
- **`src/viz.rs`**: Add `run_viz_surface` entry point. New bevy systems for 3D mesh rendering + camera. Surface blur function. Reuse existing metrics panel code.
- **`src/main.rs`**: Replace `--grid` with `--surface`, dispatch to `SoupSurface`.

### Reuse of `Soup2d` logic

`SoupSurface` will share the same epoch structure as `Soup2d`:
1. Shuffle cell order
2. Pair each cell with a random neighbor (from pre-computed table)
3. Execute all pairs in parallel via rayon
4. Copy results back

The only difference is how the neighbor table is built (geodesic distance on mesh vs. Chebyshev grid). The `run_epoch` and `mutate` methods can be nearly identical. Once `SoupSurface` is working, `Soup2d` can potentially be retired in favor of `--surface flat:WxH`.

## Risks / Trade-offs

- **Geodesic precomputation cost**: Dijkstra from each face is O(N * K * log N). For 80K faces, this may take a few seconds at startup. Mitigation: print progress during precomputation; the cost is one-time.
- **Large meshes**: An OBJ with 100K+ faces will have a large neighbor table and may be slow to initialize. Mitigation: warn the user about face count at startup.
- **Non-manifold meshes**: Imported OBJ files may have degenerate geometry. Mitigation: validate on load, reject non-manifold edges (edge shared by >2 faces).
- **Rendering performance**: Per-face vertex coloring on large meshes requires updating all vertex colors each frame. For 80K faces (240K vertices), this is ~720KB of color data per frame — manageable.
- **Breaking change**: `--grid` is removed in favor of `--surface flat:WxH`.

## Open Questions

None — `--grid` is removed outright with no deprecation period.
