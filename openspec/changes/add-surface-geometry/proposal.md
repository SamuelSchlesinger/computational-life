# Change: Add surface geometry simulation

## Why

The current 2D simulation runs on a flat rectangular grid where every interior cell has the same neighborhood structure. This produces a uniform interaction topology — there are no bottlenecks, peninsulas, or isolated regions that could foster evolutionary specialization. By generalizing the simulation to run on arbitrary geometric surfaces (spheres, toruses, imported meshes with complex features), we create varied local connectivity. Narrow passages and cul-de-sacs in the mesh can act as barriers to gene flow, potentially enabling distinct evolutionary strategies to develop in different regions — analogous to geographic speciation in biology.

## What Changes

- **Unified `--surface` CLI flag**: All spatial modes go through `--surface`. Flat grids (`flat:WxH`), spheres (`sphere:N`), toruses (`torus:MxN`), and OBJ imports (`obj:path/to/file.obj`) are all surface specs. The old `--grid` flag is replaced by `--surface flat:WxH`.
- **New surface topology abstraction**: A `SoupSurface` simulation engine that runs on an arbitrary triangle mesh. Each face hosts one program. Neighbors are determined by geodesic distance on the mesh — precomputed at startup into a flat lookup table.
- **Geodesic distance neighborhoods**: Instead of k-hop adjacency, compute shortest-path distances on the dual graph (weighted by face centroid distances) and include all faces within a configurable radius. This captures the true geometry of the surface — narrow passages produce small neighborhoods, open regions produce large ones.
- **Built-in procedural shapes**: Sphere (icosphere), torus (parametric), and flat grid.
- **OBJ mesh import**: Load arbitrary triangle meshes from `.obj` files.
- **3D live visualization**: Render the mesh in 3D with per-face program coloring, orbit camera (rotate, pan, zoom), and the metrics side panel.
- **Surface blur**: The `--blur` effect works on surfaces by blurring in face-adjacency space (each face blends with its geodesic neighbors' colors).

## Impact

- Affected specs: `simulation-cli`, `live-visualization`, new `surface-topology`
- Affected code: `src/main.rs` (CLI), new `src/surface.rs` (mesh + soup), `src/viz.rs` (3D rendering)
- **BREAKING**: `--grid WxH` is replaced by `--surface flat:WxH`
- New dependencies: `tobj` (OBJ loading), `bevy_panorbit_camera` (orbit camera)
