## 1. Surface mesh data structure and procedural generators

- [ ] 1.1 Create `src/surface.rs` with `SurfaceMesh` struct (vertices, faces, face centroids, face adjacency)
- [ ] 1.2 Implement face adjacency builder (half-edge lookup from shared vertex pairs)
- [ ] 1.3 Implement geodesic neighbor table builder (Dijkstra from each face on dual graph, weighted by centroid distance, pruned at radius)
- [ ] 1.4 Implement automatic default radius (2x average edge length)
- [ ] 1.5 Implement icosphere generator (subdivision levels 0-7)
- [ ] 1.6 Implement parametric torus generator (MxN segments, triangulated)
- [ ] 1.7 Implement flat grid generator (WxH quads, 2 triangles per cell, one program per cell)
- [ ] 1.8 Write unit tests: adjacency correctness, geodesic neighbor counts, icosphere face counts, torus face counts, flat grid parity with Soup2d population size

## 2. OBJ mesh import

- [ ] 2.1 Add `tobj` dependency to `Cargo.toml`
- [ ] 2.2 Implement OBJ loader: load file, triangulate, build `SurfaceMesh`
- [ ] 2.3 Add validation: reject non-manifold edges, warn on disconnected components
- [ ] 2.4 Write tests with small inline OBJ data (cube, tetrahedron)

## 3. Surface simulation engine

- [ ] 3.1 Implement `SoupSurface` struct with geodesic neighbor table
- [ ] 3.2 Implement `run_epoch` (same structure as `Soup2d`: shuffle, pair, parallel execute, copy back)
- [ ] 3.3 Implement `mutate` and `population_bytes_into`
- [ ] 3.4 Write determinism test (same seed → same result on icosphere)
- [ ] 3.5 Write integration test: small icosphere simulation runs, HOE is computable

## 4. CLI integration

- [ ] 4.1 Replace `--grid` with `--surface <spec>` CLI flag
- [ ] 4.2 Add `--neighbor-radius <f32>` CLI flag (default: auto-computed from mesh)
- [ ] 4.3 Parse surface specs: `flat:WxH`, `sphere:N`, `torus:MxN`, `obj:path`
- [ ] 4.4 Dispatch to `SoupSurface` in `dispatch()`
- [ ] 4.5 Test: headless surface simulation runs and prints CSV metrics

## 5. 3D live visualization

- [ ] 5.1 Add `bevy_panorbit_camera` dependency
- [ ] 5.2 Implement `run_viz_surface` entry point in `src/viz.rs`
- [ ] 5.3 Create bevy 3D mesh from `SurfaceMesh` with per-vertex colors
- [ ] 5.4 Implement sim thread loop for surface (reuse pattern from `sim_thread_loop_2d`)
- [ ] 5.5 Implement system to update vertex colors from snapshot each frame
- [ ] 5.6 Set up orbit camera with mouse rotate, pan, and scroll zoom
- [ ] 5.7 Implement surface blur (blend face colors with edge-adjacent face colors)
- [ ] 5.8 Reuse metrics side panel (egui right panel with HOE, unique programs, zero count plots)
- [ ] 5.9 Manual test: `--surface sphere:4 --live` renders colored sphere with working camera

## 6. End-to-end verification

- [ ] 6.1 `cargo build --release --features viz` compiles cleanly
- [ ] 6.2 `cargo test` — all tests pass (existing + new)
- [ ] 6.3 `--surface flat:240x135 --live` works (replaces `--grid`)
- [ ] 6.4 `--surface sphere:5 --live --seed 42 --epochs 50000` shows colored sphere with orbit camera
- [ ] 6.5 `--surface torus:80x30 --live --seed 42 --epochs 50000` shows colored torus
- [ ] 6.6 `--surface obj:<test.obj> --live --seed 42 --epochs 50000` loads and renders custom mesh
- [ ] 6.7 `--blur 0.3` works on all surface types
