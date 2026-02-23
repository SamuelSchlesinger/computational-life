use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use rand::Rng;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rayon::prelude::*;

use crate::substrate::Substrate;

// ─── Dijkstra helper ─────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
struct DijkNode {
    dist: f32,
    face: usize,
}

impl Eq for DijkNode {}

impl Ord for DijkNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .dist
            .partial_cmp(&self.dist)
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for DijkNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// ─── SurfaceMesh ─────────────────────────────────────────────────────────────

/// A triangle mesh representing a simulation surface.
///
/// Each triangular face hosts one program. The mesh stores geometry for
/// rendering and a pre-computed geodesic neighbor table for the simulation.
pub struct SurfaceMesh {
    /// Vertex positions (for rendering).
    pub vertices: Vec<[f32; 3]>,
    /// Triangular faces (indices into `vertices`).
    pub faces: Vec<[usize; 3]>,
    /// Centroid of each face.
    pub face_centroids: Vec<[f32; 3]>,
    /// Direct face adjacency (faces sharing an edge).
    pub face_adjacency: Vec<Vec<usize>>,
    /// Flat buffer of geodesic neighbor indices.
    pub neighbor_indices: Vec<usize>,
    /// Per-face (start, end) ranges into `neighbor_indices`.
    pub neighbor_ranges: Vec<(usize, usize)>,
}

impl SurfaceMesh {
    /// Number of simulation cells (one per face).
    pub fn num_cells(&self) -> usize {
        self.faces.len()
    }

    /// Construct from raw geometry. Builds adjacency and centroids.
    /// Does NOT compute geodesic neighbors — call `compute_neighbors` after.
    fn from_geometry(vertices: Vec<[f32; 3]>, faces: Vec<[usize; 3]>) -> Result<Self, String> {
        // Validate vertex indices.
        for (fi, face) in faces.iter().enumerate() {
            for &vi in face {
                if vi >= vertices.len() {
                    return Err(format!(
                        "Face {fi} references vertex {vi}, but only {} vertices exist",
                        vertices.len()
                    ));
                }
            }
        }

        let face_adjacency = build_face_adjacency(&faces)?;
        let face_centroids = compute_face_centroids(&vertices, &faces);
        let n = faces.len();

        Ok(Self {
            vertices,
            faces,
            face_centroids,
            face_adjacency,
            neighbor_indices: Vec::new(),
            neighbor_ranges: vec![(0, 0); n],
        })
    }

    /// Compute geodesic neighbor table. `radius` of `None` uses 2x average
    /// centroid-to-centroid distance across adjacent faces.
    pub fn compute_neighbors(&mut self, radius: Option<f32>) {
        let radius = radius.unwrap_or_else(|| 4.0 * self.avg_adjacent_centroid_distance());
        let n = self.faces.len();
        eprintln!("Computing geodesic neighbors for {n} faces (radius: {radius:.4})...");

        // Borrow shared data immutably so rayon can access it in parallel.
        let face_adjacency = &self.face_adjacency;
        let face_centroids = &self.face_centroids;

        // Run Dijkstra from each face in parallel.
        let per_face_neighbors: Vec<Vec<usize>> = (0..n)
            .into_par_iter()
            .map(|source| {
                let mut dist = vec![f32::INFINITY; n];
                dist[source] = 0.0;
                let mut heap = BinaryHeap::new();
                heap.push(DijkNode {
                    dist: 0.0,
                    face: source,
                });

                while let Some(node) = heap.pop() {
                    if node.dist > dist[node.face] {
                        continue;
                    }
                    for &adj in &face_adjacency[node.face] {
                        let edge_dist =
                            centroid_distance(&face_centroids[node.face], &face_centroids[adj]);
                        let new_dist = node.dist + edge_dist;
                        if new_dist <= radius && new_dist < dist[adj] {
                            dist[adj] = new_dist;
                            heap.push(DijkNode {
                                dist: new_dist,
                                face: adj,
                            });
                        }
                    }
                }

                let mut neighbors = Vec::new();
                for (i, &d) in dist.iter().enumerate() {
                    if i != source && d <= radius {
                        neighbors.push(i);
                    }
                }
                neighbors
            })
            .collect();

        // Flatten into the compact buffer format.
        let mut neighbor_indices = Vec::new();
        let mut neighbor_ranges = Vec::with_capacity(n);
        for neighbors in &per_face_neighbors {
            let start = neighbor_indices.len();
            neighbor_indices.extend_from_slice(neighbors);
            neighbor_ranges.push((start, neighbor_indices.len()));
        }

        let total_neighbors: usize = neighbor_ranges.iter().map(|(s, e)| e - s).sum();
        let avg = if n > 0 {
            total_neighbors as f64 / n as f64
        } else {
            0.0
        };
        eprintln!("  Average neighbors per face: {avg:.1}");
        eprintln!("  done.");

        self.neighbor_indices = neighbor_indices;
        self.neighbor_ranges = neighbor_ranges;
    }

    /// Average centroid-to-centroid distance between adjacent faces.
    fn avg_adjacent_centroid_distance(&self) -> f32 {
        let mut total = 0.0f32;
        let mut count = 0usize;
        for (i, adj_list) in self.face_adjacency.iter().enumerate() {
            for &j in adj_list {
                if j > i {
                    total += centroid_distance(&self.face_centroids[i], &self.face_centroids[j]);
                    count += 1;
                }
            }
        }
        if count == 0 {
            1.0
        } else {
            total / count as f32
        }
    }

    /// Compute the bounding box center and radius (for camera framing).
    pub fn bounding_sphere(&self) -> ([f32; 3], f32) {
        if self.vertices.is_empty() {
            return ([0.0; 3], 1.0);
        }
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for v in &self.vertices {
            for i in 0..3 {
                min[i] = min[i].min(v[i]);
                max[i] = max[i].max(v[i]);
            }
        }
        let center = [
            (min[0] + max[0]) / 2.0,
            (min[1] + max[1]) / 2.0,
            (min[2] + max[2]) / 2.0,
        ];
        let mut max_dist_sq = 0.0f32;
        for v in &self.vertices {
            let dx = v[0] - center[0];
            let dy = v[1] - center[1];
            let dz = v[2] - center[2];
            max_dist_sq = max_dist_sq.max(dx * dx + dy * dy + dz * dz);
        }
        (center, max_dist_sq.sqrt())
    }

    // ─── Procedural generators ───────────────────────────────────────────────

    /// Generate an icosphere with the given number of subdivision levels.
    /// Subdivision 0 = icosahedron (20 faces). Each level quadruples face count.
    pub fn icosphere(subdivisions: usize) -> Result<Self, String> {
        let phi = (1.0 + 5.0_f32.sqrt()) / 2.0;

        let mut vertices: Vec<[f32; 3]> = vec![
            [-1.0, phi, 0.0],
            [1.0, phi, 0.0],
            [-1.0, -phi, 0.0],
            [1.0, -phi, 0.0],
            [0.0, -1.0, phi],
            [0.0, 1.0, phi],
            [0.0, -1.0, -phi],
            [0.0, 1.0, -phi],
            [phi, 0.0, -1.0],
            [phi, 0.0, 1.0],
            [-phi, 0.0, -1.0],
            [-phi, 0.0, 1.0],
        ];

        // Normalize to unit sphere.
        for v in &mut vertices {
            let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            v[0] /= len;
            v[1] /= len;
            v[2] /= len;
        }

        let mut faces: Vec<[usize; 3]> = vec![
            [0, 11, 5],
            [0, 5, 1],
            [0, 1, 7],
            [0, 7, 10],
            [0, 10, 11],
            [1, 5, 9],
            [5, 11, 4],
            [11, 10, 2],
            [10, 7, 6],
            [7, 1, 8],
            [3, 9, 4],
            [3, 4, 2],
            [3, 2, 6],
            [3, 6, 8],
            [3, 8, 9],
            [4, 9, 5],
            [2, 4, 11],
            [6, 2, 10],
            [8, 6, 7],
            [9, 8, 1],
        ];

        for _ in 0..subdivisions {
            let mut midpoint_cache: HashMap<(usize, usize), usize> = HashMap::new();
            let mut new_faces = Vec::with_capacity(faces.len() * 4);

            for face in &faces {
                let m01 = get_midpoint(face[0], face[1], &mut vertices, &mut midpoint_cache);
                let m12 = get_midpoint(face[1], face[2], &mut vertices, &mut midpoint_cache);
                let m20 = get_midpoint(face[2], face[0], &mut vertices, &mut midpoint_cache);

                new_faces.push([face[0], m01, m20]);
                new_faces.push([face[1], m12, m01]);
                new_faces.push([face[2], m20, m12]);
                new_faces.push([m01, m12, m20]);
            }

            faces = new_faces;
        }

        Self::from_geometry(vertices, faces)
    }

    /// Generate a torus with `major` segments around the ring and `minor`
    /// segments around the tube cross-section. Major radius = 1.0, minor = 0.4.
    pub fn torus(major: usize, minor: usize) -> Result<Self, String> {
        if major < 3 || minor < 3 {
            return Err("Torus requires at least 3 segments in each dimension".into());
        }

        let r_major = 1.0_f32;
        let r_minor = 0.4_f32;

        let mut vertices = Vec::with_capacity(major * minor);
        for i in 0..major {
            let u = 2.0 * std::f32::consts::PI * i as f32 / major as f32;
            for j in 0..minor {
                let v = 2.0 * std::f32::consts::PI * j as f32 / minor as f32;
                let x = (r_major + r_minor * v.cos()) * u.cos();
                let y = r_minor * v.sin();
                let z = (r_major + r_minor * v.cos()) * u.sin();
                vertices.push([x, y, z]);
            }
        }

        let mut faces = Vec::with_capacity(2 * major * minor);
        for i in 0..major {
            let i_next = (i + 1) % major;
            for j in 0..minor {
                let j_next = (j + 1) % minor;
                let v00 = i * minor + j;
                let v10 = i_next * minor + j;
                let v11 = i_next * minor + j_next;
                let v01 = i * minor + j_next;
                faces.push([v00, v10, v11]);
                faces.push([v00, v11, v01]);
            }
        }

        Self::from_geometry(vertices, faces)
    }

    /// Generate a flat grid in the XY plane. `width` x `height` quads,
    /// each split into 2 triangles = 2*width*height faces (programs).
    /// Centered at origin, scaled so longest dimension spans [-1, 1].
    pub fn flat_grid(width: usize, height: usize) -> Result<Self, String> {
        if width == 0 || height == 0 {
            return Err("Grid dimensions must be positive".into());
        }

        let scale = 2.0 / (width.max(height) as f32);
        let x_offset = width as f32 * scale / 2.0;
        let y_offset = height as f32 * scale / 2.0;

        let mut vertices = Vec::with_capacity((width + 1) * (height + 1));
        for j in 0..=height {
            for i in 0..=width {
                let x = i as f32 * scale - x_offset;
                let y = j as f32 * scale - y_offset;
                vertices.push([x, y, 0.0]);
            }
        }

        let cols = width + 1;
        let mut faces = Vec::with_capacity(2 * width * height);
        for j in 0..height {
            for i in 0..width {
                let v00 = j * cols + i;
                let v10 = j * cols + i + 1;
                let v01 = (j + 1) * cols + i;
                let v11 = (j + 1) * cols + i + 1;
                faces.push([v00, v10, v11]);
                faces.push([v00, v11, v01]);
            }
        }

        Self::from_geometry(vertices, faces)
    }

    /// Load a mesh from a Wavefront OBJ file.
    pub fn from_obj(path: &str) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read OBJ file '{path}': {e}"))?;

        let mut vertices = Vec::new();
        let mut faces = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let mut parts = line.split_whitespace();
            match parts.next() {
                Some("v") => {
                    let coords: Vec<f32> = parts
                        .take(3)
                        .map(|s| s.parse::<f32>())
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| {
                            format!("Line {}: invalid vertex coordinate: {e}", line_num + 1)
                        })?;
                    if coords.len() < 3 {
                        return Err(format!("Line {}: vertex needs 3 coordinates", line_num + 1));
                    }
                    vertices.push([coords[0], coords[1], coords[2]]);
                }
                Some("f") => {
                    let indices: Vec<usize> = parts
                        .map(|s| {
                            // Handle v, v/vt, v/vt/vn, v//vn formats.
                            let idx_str = s.split('/').next().unwrap();
                            idx_str.parse::<usize>().map(|i| i - 1) // OBJ is 1-indexed
                        })
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| format!("Line {}: invalid face index: {e}", line_num + 1))?;

                    if indices.len() < 3 {
                        return Err(format!(
                            "Line {}: face needs at least 3 vertices",
                            line_num + 1
                        ));
                    }
                    // Fan triangulation for quads and n-gons.
                    for i in 1..indices.len() - 1 {
                        faces.push([indices[0], indices[i], indices[i + 1]]);
                    }
                }
                _ => {} // Ignore vn, vt, mtllib, usemtl, etc.
            }
        }

        if faces.is_empty() {
            return Err(format!("OBJ file '{path}' contains no faces"));
        }

        eprintln!(
            "Loaded OBJ: {} vertices, {} faces",
            vertices.len(),
            faces.len()
        );
        Self::from_geometry(vertices, faces)
    }

    /// Generate a "hamster tunnel": a loop of spheres connected by tubes.
    ///
    /// Sphere positions are scattered randomly in 3D, sorted into a short
    /// path via nearest-neighbor heuristic, and connected as a closed loop.
    /// This creates torus-like topology where information can flow in cycles.
    ///
    /// - `num_spheres`: number of sphere nodes (>= 3 for a loop).
    /// - `segments`: vertices per ring cross-section (>= 3).
    /// - `seed`: RNG seed for sphere placement.
    pub fn hamster_tunnel(num_spheres: usize, segments: usize, seed: u64) -> Result<Self, String> {
        if num_spheres < 3 {
            return Err("Hamster tunnel requires at least 3 spheres".into());
        }
        if segments < 3 {
            return Err("Hamster tunnel requires at least 3 circumferential segments".into());
        }

        const SPHERE_RADIUS: f32 = 0.4;
        const TUBE_RADIUS: f32 = 0.12;
        const RINGS_PER_SEGMENT: usize = 16;

        let mut rng = SmallRng::seed_from_u64(seed);

        // ── Phase A: scatter sphere centers in a bounded volume ──
        // Radius scales so average nearest-neighbor distance ≈ 2.0.
        let spread = 2.0 * (3.0 * num_spheres as f32 / (4.0 * std::f32::consts::PI)).cbrt();
        let mut raw_centers: Vec<[f32; 3]> = Vec::with_capacity(num_spheres);
        for _ in 0..num_spheres {
            loop {
                let x = rng.r#gen::<f32>() * 2.0 - 1.0;
                let y = rng.r#gen::<f32>() * 2.0 - 1.0;
                let z = rng.r#gen::<f32>() * 2.0 - 1.0;
                if x * x + y * y + z * z <= 1.0 {
                    raw_centers.push([x * spread, y * spread, z * spread]);
                    break;
                }
            }
        }

        // ── Phase A2: nearest-neighbor sort for a short path ──
        let mut centers: Vec<[f32; 3]> = Vec::with_capacity(num_spheres);
        let mut used = vec![false; num_spheres];
        let mut current = 0;
        used[0] = true;
        centers.push(raw_centers[0]);
        for _ in 1..num_spheres {
            let mut best = usize::MAX;
            let mut best_dist = f32::INFINITY;
            for (j, &u) in used.iter().enumerate() {
                if u {
                    continue;
                }
                let d = centroid_distance(&raw_centers[current], &raw_centers[j]);
                if d < best_dist {
                    best_dist = d;
                    best = j;
                }
            }
            used[best] = true;
            centers.push(raw_centers[best]);
            current = best;
        }

        // ── Phase B: build ring sample points along the closed loop ──
        // The loop goes: center[0]→center[1]→…→center[N-1]→center[0].
        let num_segments = num_spheres; // N segments for a loop of N spheres
        let total_rings = num_segments * RINGS_PER_SEGMENT; // no +1: loop wraps

        let mut ring_positions: Vec<[f32; 3]> = Vec::with_capacity(total_rings);
        let mut ring_radii: Vec<f32> = Vec::with_capacity(total_rings);
        let mut ring_tangents: Vec<[f32; 3]> = Vec::with_capacity(total_rings);

        for seg in 0..num_segments {
            let c0 = centers[seg];
            let c1 = centers[(seg + 1) % num_spheres];
            let tangent = normalize3([c1[0] - c0[0], c1[1] - c0[1], c1[2] - c0[2]]);

            for r in 0..RINGS_PER_SEGMENT {
                let u = r as f32 / RINGS_PER_SEGMENT as f32;
                let pos = [
                    c0[0] + (c1[0] - c0[0]) * u,
                    c0[1] + (c1[1] - c0[1]) * u,
                    c0[2] + (c1[2] - c0[2]) * u,
                ];
                let cos_val = (std::f32::consts::PI * u).cos();
                let radius = TUBE_RADIUS + (SPHERE_RADIUS - TUBE_RADIUS) * cos_val * cos_val;

                ring_positions.push(pos);
                ring_radii.push(radius);
                ring_tangents.push(tangent);
            }
        }

        // ── Phase C: measure holonomy twist via parallel transport ──
        let t0 = ring_tangents[0];
        let up_candidate = if t0[1].abs() < 0.9 {
            [0.0, 1.0, 0.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        let initial_normal = normalize3(cross3(t0, up_candidate));
        let initial_binormal = cross3(t0, initial_normal);

        // Pass 1: transport frame around full loop to measure accumulated twist.
        let mut normal = initial_normal;
        for ring_idx in 1..total_rings {
            let prev_t = ring_tangents[ring_idx - 1];
            let tangent = ring_tangents[ring_idx];
            let d = dot3(prev_t, tangent);
            if d < 0.9999 {
                let axis = normalize3(cross3(prev_t, tangent));
                let angle = d.clamp(-1.0, 1.0).acos();
                normal = normalize3(rotate_around_axis(normal, axis, angle));
            }
        }
        // Transport across the closure edge (last ring -> ring 0).
        {
            let prev_t = ring_tangents[total_rings - 1];
            let tangent = ring_tangents[0];
            let d = dot3(prev_t, tangent);
            if d < 0.9999 {
                let axis = normalize3(cross3(prev_t, tangent));
                let angle = d.clamp(-1.0, 1.0).acos();
                normal = normalize3(rotate_around_axis(normal, axis, angle));
            }
        }
        let cos_twist = dot3(normal, initial_normal);
        let sin_twist = dot3(normal, initial_binormal);
        let total_twist = sin_twist.atan2(cos_twist);

        // ── Phase D: generate ring vertices with holonomy correction ──
        let mut normal = initial_normal;
        let mut vertices: Vec<[f32; 3]> = Vec::with_capacity(total_rings * segments);

        for ring_idx in 0..total_rings {
            let pos = ring_positions[ring_idx];
            let r = ring_radii[ring_idx];
            let tangent = ring_tangents[ring_idx];

            if ring_idx > 0 {
                let prev_t = ring_tangents[ring_idx - 1];
                let d = dot3(prev_t, tangent);
                if d < 0.9999 {
                    let axis = normalize3(cross3(prev_t, tangent));
                    let angle = d.clamp(-1.0, 1.0).acos();
                    normal = normalize3(rotate_around_axis(normal, axis, angle));
                }
            }

            // Counter-twist: rotate frame around tangent to distribute correction.
            let correction = -total_twist * (ring_idx as f32 / total_rings as f32);
            let cn = normalize3(rotate_around_axis(normal, tangent, correction));
            let cb = cross3(tangent, cn);

            for j in 0..segments {
                let theta = 2.0 * std::f32::consts::PI * j as f32 / segments as f32;
                let c = theta.cos();
                let s = theta.sin();
                vertices.push([
                    pos[0] + r * (c * cn[0] + s * cb[0]),
                    pos[1] + r * (c * cn[1] + s * cb[1]),
                    pos[2] + r * (c * cn[2] + s * cb[2]),
                ]);
            }
        }

        // ── Phase E: connect adjacent rings with triangle strips (loop) ──
        let body_faces = 2 * segments * total_rings;
        let mut faces: Vec<[usize; 3]> = Vec::with_capacity(body_faces);

        for k in 0..total_rings {
            let base0 = k * segments;
            let base1 = ((k + 1) % total_rings) * segments; // wraps around
            for j in 0..segments {
                let j_next = (j + 1) % segments;
                faces.push([base0 + j, base1 + j, base1 + j_next]);
                faces.push([base0 + j, base1 + j_next, base0 + j_next]);
            }
        }

        // No caps needed — the loop closes on itself.

        let face_count = faces.len();
        eprintln!(
            "Surface: hamster tunnel ({num_spheres} spheres, {segments} segments, {face_count} faces)"
        );
        Self::from_geometry(vertices, faces)
    }
}

// ─── Surface spec ───────────────────────────────────────────────────────────

/// Specification for generating a surface mesh.
#[derive(Clone, Debug, PartialEq)]
pub enum SurfaceSpec {
    Sphere {
        subdivisions: usize,
    },
    Torus {
        major: usize,
        minor: usize,
    },
    FlatGrid {
        width: usize,
        height: usize,
    },
    HamsterTunnel {
        num_spheres: usize,
        segments: usize,
        seed: u64,
    },
    ObjFile {
        path: String,
    },
}

impl SurfaceSpec {
    /// Build a SurfaceMesh from this spec (does NOT compute neighbors).
    pub fn build(&self) -> Result<SurfaceMesh, String> {
        match self {
            SurfaceSpec::Sphere { subdivisions } => SurfaceMesh::icosphere(*subdivisions),
            SurfaceSpec::Torus { major, minor } => SurfaceMesh::torus(*major, *minor),
            SurfaceSpec::FlatGrid { width, height } => SurfaceMesh::flat_grid(*width, *height),
            SurfaceSpec::HamsterTunnel {
                num_spheres,
                segments,
                seed,
            } => SurfaceMesh::hamster_tunnel(*num_spheres, *segments, *seed),
            SurfaceSpec::ObjFile { path } => SurfaceMesh::from_obj(path),
        }
    }

    /// Human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            SurfaceSpec::Sphere { .. } => "Sphere",
            SurfaceSpec::Torus { .. } => "Torus",
            SurfaceSpec::FlatGrid { .. } => "Flat Grid",
            SurfaceSpec::HamsterTunnel { .. } => "Hamster Tunnel",
            SurfaceSpec::ObjFile { .. } => "OBJ File",
        }
    }
}

// ─── Geometry helpers ────────────────────────────────────────────────────────

/// Build face adjacency table. Returns error if non-manifold edges found.
fn build_face_adjacency(faces: &[[usize; 3]]) -> Result<Vec<Vec<usize>>, String> {
    let mut edge_to_faces: HashMap<(usize, usize), Vec<usize>> = HashMap::new();

    for (fi, face) in faces.iter().enumerate() {
        for e in 0..3 {
            let v0 = face[e];
            let v1 = face[(e + 1) % 3];
            let edge = if v0 < v1 { (v0, v1) } else { (v1, v0) };
            edge_to_faces.entry(edge).or_default().push(fi);
        }
    }

    for (&(v0, v1), face_list) in &edge_to_faces {
        if face_list.len() > 2 {
            return Err(format!(
                "Non-manifold edge ({v0}, {v1}): shared by {} faces",
                face_list.len()
            ));
        }
    }

    let mut adjacency = vec![Vec::new(); faces.len()];
    for face_list in edge_to_faces.values() {
        if face_list.len() == 2 {
            adjacency[face_list[0]].push(face_list[1]);
            adjacency[face_list[1]].push(face_list[0]);
        }
    }

    Ok(adjacency)
}

/// Compute face centroids.
fn compute_face_centroids(vertices: &[[f32; 3]], faces: &[[usize; 3]]) -> Vec<[f32; 3]> {
    faces
        .iter()
        .map(|f| {
            let (v0, v1, v2) = (vertices[f[0]], vertices[f[1]], vertices[f[2]]);
            [
                (v0[0] + v1[0] + v2[0]) / 3.0,
                (v0[1] + v1[1] + v2[1]) / 3.0,
                (v0[2] + v1[2] + v2[2]) / 3.0,
            ]
        })
        .collect()
}

/// Euclidean distance between two centroids.
fn centroid_distance(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Get or create a midpoint vertex on the unit sphere (for icosphere subdivision).
fn get_midpoint(
    v0: usize,
    v1: usize,
    vertices: &mut Vec<[f32; 3]>,
    cache: &mut HashMap<(usize, usize), usize>,
) -> usize {
    let key = if v0 < v1 { (v0, v1) } else { (v1, v0) };
    if let Some(&mid) = cache.get(&key) {
        return mid;
    }
    let p0 = vertices[v0];
    let p1 = vertices[v1];
    let mid = [
        (p0[0] + p1[0]) / 2.0,
        (p0[1] + p1[1]) / 2.0,
        (p0[2] + p1[2]) / 2.0,
    ];
    // Normalize to unit sphere.
    let len = (mid[0] * mid[0] + mid[1] * mid[1] + mid[2] * mid[2]).sqrt();
    let mid = [mid[0] / len, mid[1] / len, mid[2] / len];
    let idx = vertices.len();
    vertices.push(mid);
    cache.insert(key, idx);
    idx
}

/// Compute face normal (unnormalized cross product).
pub fn face_normal(v0: &[f32; 3], v1: &[f32; 3], v2: &[f32; 3]) -> [f32; 3] {
    let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
    let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
    let nx = e1[1] * e2[2] - e1[2] * e2[1];
    let ny = e1[2] * e2[0] - e1[0] * e2[2];
    let nz = e1[0] * e2[1] - e1[1] * e2[0];
    let len = (nx * nx + ny * ny + nz * nz).sqrt();
    if len < 1e-10 {
        [0.0, 1.0, 0.0]
    } else {
        [nx / len, ny / len, nz / len]
    }
}

fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len < 1e-10 {
        [0.0, 1.0, 0.0]
    } else {
        [v[0] / len, v[1] / len, v[2] / len]
    }
}

fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Rotate vector `v` around unit `axis` by `angle` radians (Rodrigues' formula).
fn rotate_around_axis(v: [f32; 3], axis: [f32; 3], angle: f32) -> [f32; 3] {
    let c = angle.cos();
    let s = angle.sin();
    let d = dot3(axis, v);
    let cr = cross3(axis, v);
    [
        v[0] * c + cr[0] * s + axis[0] * d * (1.0 - c),
        v[1] * c + cr[1] * s + axis[1] * d * (1.0 - c),
        v[2] * c + cr[2] * s + axis[2] * d * (1.0 - c),
    ]
}

/// Sample from geometric distribution via CDF inversion.
/// Returns the number of bytes to skip before the next mutation.
/// `inv_log` should be `1.0 / ln(1 - mutation_rate)` (precomputed).
fn geometric_skip(rng: &mut SmallRng, inv_log: f64) -> usize {
    let u: f64 = rng.r#gen::<f64>();
    if u < 1e-300 {
        return usize::MAX;
    }
    (u.ln() * inv_log) as usize
}

// ─── SoupSurface ─────────────────────────────────────────────────────────────

/// Configuration for a surface simulation.
#[derive(Clone, Copy)]
pub struct SoupSurfaceConfig {
    /// Bytes per program.
    pub program_size: usize,
    /// Maximum steps per program execution.
    pub step_limit: usize,
    /// Per-byte mutation probability per epoch.
    pub mutation_rate: f64,
}

/// A primordial soup simulation running on a triangle mesh surface.
pub struct SoupSurface {
    pub programs: Vec<Vec<u8>>,
    pub config: SoupSurfaceConfig,
    pub mesh: SurfaceMesh,
    pub rng: SmallRng,
    /// Reusable scratch: shuffled iteration order.
    order: Vec<usize>,
    /// Reusable scratch: taken flags.
    taken: Vec<bool>,
    /// Reusable scratch: interaction pairs.
    pairs: Vec<(usize, usize)>,
    /// Reusable scratch: flat tape buffer for parallel execution.
    tape_pool: Vec<u8>,
}

impl SoupSurface {
    /// Create a new surface soup with randomly initialized programs.
    pub fn new(mesh: SurfaceMesh, config: SoupSurfaceConfig, seed: u64) -> Self {
        let mut rng = SmallRng::seed_from_u64(seed);
        let total = mesh.num_cells();
        let programs = (0..total)
            .map(|_| {
                let mut prog = vec![0u8; config.program_size];
                rng.fill(&mut prog[..]);
                prog
            })
            .collect();

        let order: Vec<usize> = (0..total).collect();
        let taken = vec![false; total];
        let pairs = Vec::with_capacity(total / 2);
        let tape_pool = Vec::new();

        Self {
            programs,
            config,
            mesh,
            rng,
            order,
            taken,
            pairs,
            tape_pool,
        }
    }

    /// Run one epoch: pair each cell with a random geodesic neighbor, execute
    /// in parallel.
    pub fn run_epoch<S: Substrate + Sync>(&mut self) {
        let total = self.mesh.num_cells();
        let ps = self.config.program_size;
        let step_limit = self.config.step_limit;

        // --- Phase 1: build pairs (sequential) ---

        for i in 0..total {
            self.order[i] = i;
        }
        self.order.shuffle(&mut self.rng);

        self.taken.fill(false);
        self.pairs.clear();

        for i in 0..total {
            let p_idx = self.order[i];
            if self.taken[p_idx] {
                continue;
            }

            let (start, end) = self.mesh.neighbor_ranges[p_idx];
            let neighbor_count = end - start;
            if neighbor_count == 0 {
                continue;
            }

            let n_idx = self.mesh.neighbor_indices[start + self.rng.gen_range(0..neighbor_count)];
            if self.taken[n_idx] {
                continue;
            }

            self.taken[p_idx] = true;
            self.taken[n_idx] = true;

            let (first, second) = if self.rng.r#gen::<bool>() {
                (p_idx, n_idx)
            } else {
                (n_idx, p_idx)
            };

            self.pairs.push((first, second));
        }

        // --- Phase 2: execute all pairs in parallel ---

        let num_pairs = self.pairs.len();
        let tape_size = ps * 2;

        self.tape_pool.resize(num_pairs * tape_size, 0);

        for (i, &(first, second)) in self.pairs.iter().enumerate() {
            let base = i * tape_size;
            self.tape_pool[base..base + ps].copy_from_slice(&self.programs[first]);
            self.tape_pool[base + ps..base + tape_size].copy_from_slice(&self.programs[second]);
        }

        self.tape_pool.par_chunks_mut(tape_size).for_each(|tape| {
            S::execute(tape, step_limit);
        });

        for (i, &(first, second)) in self.pairs.iter().enumerate() {
            let base = i * tape_size;
            self.programs[first].copy_from_slice(&self.tape_pool[base..base + ps]);
            self.programs[second].copy_from_slice(&self.tape_pool[base + ps..base + tape_size]);
        }
    }

    /// Apply background mutation to all programs.
    ///
    /// Uses geometric distribution to skip directly to the next mutation site,
    /// reducing RNG calls from O(total_bytes) to O(total_bytes * mutation_rate).
    pub fn mutate(&mut self) {
        if self.config.mutation_rate <= 0.0 {
            return;
        }
        let total_bytes = self.programs.len() * self.config.program_size;
        let ps = self.config.program_size;
        let inv_log = 1.0 / (1.0 - self.config.mutation_rate).ln();

        let mut pos = geometric_skip(&mut self.rng, inv_log);
        while pos < total_bytes {
            let prog_idx = pos / ps;
            let byte_idx = pos % ps;
            let bit = 1u8 << self.rng.gen_range(0..8);
            self.programs[prog_idx][byte_idx] ^= bit;
            pos = pos
                .saturating_add(1)
                .saturating_add(geometric_skip(&mut self.rng, inv_log));
        }
    }

    /// Fill `buf` with the entire population as a flat byte slice.
    pub fn population_bytes_into(&self, buf: &mut Vec<u8>) {
        buf.clear();
        let total = self.mesh.num_cells() * self.config.program_size;
        buf.reserve(total);
        for prog in &self.programs {
            buf.extend_from_slice(prog);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bff::Bff;

    #[test]
    fn test_icosphere_face_counts() {
        // Sub 0 = 20, sub 1 = 80, sub 2 = 320, sub 3 = 1280
        for (sub, expected) in [(0, 20), (1, 80), (2, 320), (3, 1280)] {
            let mesh = SurfaceMesh::icosphere(sub).unwrap();
            assert_eq!(
                mesh.faces.len(),
                expected,
                "Icosphere sub {sub}: expected {expected} faces, got {}",
                mesh.faces.len()
            );
        }
    }

    #[test]
    fn test_icosphere_vertices_on_unit_sphere() {
        let mesh = SurfaceMesh::icosphere(2).unwrap();
        for (i, v) in mesh.vertices.iter().enumerate() {
            let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            assert!(
                (len - 1.0).abs() < 1e-5,
                "Vertex {i} has length {len}, expected 1.0"
            );
        }
    }

    #[test]
    fn test_torus_face_count() {
        let mesh = SurfaceMesh::torus(10, 5).unwrap();
        assert_eq!(mesh.faces.len(), 2 * 10 * 5);
    }

    #[test]
    fn test_flat_grid_face_count() {
        let mesh = SurfaceMesh::flat_grid(10, 8).unwrap();
        assert_eq!(mesh.faces.len(), 2 * 10 * 8);
    }

    #[test]
    fn test_adjacency_symmetric() {
        let mesh = SurfaceMesh::icosphere(1).unwrap();
        for (i, adj) in mesh.face_adjacency.iter().enumerate() {
            for &j in adj {
                assert!(
                    mesh.face_adjacency[j].contains(&i),
                    "Face {i} adjacent to {j}, but not vice versa"
                );
            }
        }
    }

    #[test]
    fn test_icosphere_faces_have_3_adjacent() {
        // Every face on a closed icosphere should have exactly 3 adjacent faces.
        let mesh = SurfaceMesh::icosphere(2).unwrap();
        for (i, adj) in mesh.face_adjacency.iter().enumerate() {
            assert_eq!(
                adj.len(),
                3,
                "Face {i} has {} adjacent faces, expected 3",
                adj.len()
            );
        }
    }

    #[test]
    fn test_torus_faces_have_3_adjacent() {
        let mesh = SurfaceMesh::torus(8, 5).unwrap();
        for (i, adj) in mesh.face_adjacency.iter().enumerate() {
            assert_eq!(
                adj.len(),
                3,
                "Face {i} has {} adjacent faces, expected 3",
                adj.len()
            );
        }
    }

    #[test]
    fn test_geodesic_neighbors_exclude_self() {
        let mut mesh = SurfaceMesh::icosphere(1).unwrap();
        mesh.compute_neighbors(None);
        for i in 0..mesh.num_cells() {
            let (start, end) = mesh.neighbor_ranges[i];
            let neighbors = &mesh.neighbor_indices[start..end];
            assert!(
                !neighbors.contains(&i),
                "Face {i} has itself as a geodesic neighbor"
            );
        }
    }

    #[test]
    fn test_geodesic_neighbors_nonempty() {
        let mut mesh = SurfaceMesh::icosphere(2).unwrap();
        mesh.compute_neighbors(None);
        for i in 0..mesh.num_cells() {
            let (start, end) = mesh.neighbor_ranges[i];
            assert!(end > start, "Face {i} has no geodesic neighbors");
        }
    }

    #[test]
    fn test_deterministic_surface_simulation() {
        let run = |seed: u64| -> Vec<Vec<u8>> {
            let mut mesh = SurfaceMesh::icosphere(1).unwrap();
            mesh.compute_neighbors(None);
            let config = SoupSurfaceConfig {
                program_size: 16,
                step_limit: 256,
                mutation_rate: 0.001,
            };
            let mut soup = SoupSurface::new(mesh, config, seed);
            for _ in 0..10 {
                soup.run_epoch::<Bff>();
                soup.mutate();
            }
            soup.programs
        };
        assert_eq!(run(42), run(42));
        assert_ne!(run(42), run(99));
    }

    #[test]
    fn test_obj_loader_cube() {
        let obj = "\
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 1.0 1.0 0.0
v 0.0 1.0 0.0
v 0.0 0.0 1.0
v 1.0 0.0 1.0
v 1.0 1.0 1.0
v 0.0 1.0 1.0
f 1 2 3 4
f 5 6 7 8
f 1 2 6 5
f 2 3 7 6
f 3 4 8 7
f 4 1 5 8
";
        let dir = std::env::temp_dir();
        let path = dir.join("test_cube.obj");
        std::fs::write(&path, obj).unwrap();
        let mesh = SurfaceMesh::from_obj(path.to_str().unwrap()).unwrap();
        // 6 quads → 12 triangles
        assert_eq!(mesh.faces.len(), 12);
        assert_eq!(mesh.vertices.len(), 8);
    }

    #[test]
    fn test_mutation_disabled_surface() {
        let mut mesh = SurfaceMesh::icosphere(0).unwrap();
        mesh.compute_neighbors(None);
        let config = SoupSurfaceConfig {
            program_size: 16,
            step_limit: 256,
            mutation_rate: 0.0,
        };
        let mut soup = SoupSurface::new(mesh, config, 42);
        let before = soup.programs.clone();
        soup.mutate();
        assert_eq!(soup.programs, before);
    }

    #[test]
    fn test_integration_small_surface_simulation() {
        use crate::metrics::high_order_entropy;

        let mut mesh = SurfaceMesh::icosphere(1).unwrap();
        mesh.compute_neighbors(None);
        let config = SoupSurfaceConfig {
            program_size: 64,
            step_limit: 8192,
            mutation_rate: 0.00024,
        };
        let mut soup = SoupSurface::new(mesh, config, 42);

        let mut buf = Vec::new();
        soup.population_bytes_into(&mut buf);
        let initial_hoe = high_order_entropy(&buf);
        assert!(
            initial_hoe > 0.5,
            "Initial HOE should be high, got {initial_hoe}"
        );

        for _ in 0..20 {
            soup.run_epoch::<Bff>();
            soup.mutate();
        }

        soup.population_bytes_into(&mut buf);
        let final_hoe = high_order_entropy(&buf);
        assert!(final_hoe > 0.0, "Final HOE should be positive");
    }

    #[test]
    fn test_hamster_tunnel_basic() {
        let mesh = SurfaceMesh::hamster_tunnel(5, 16, 42).unwrap();
        // 5 spheres => 5 segments (loop) => 5 * 16 = 80 rings
        // Body faces: 2 * 16 * 80 = 2560 (loop wraps, no caps)
        assert_eq!(mesh.faces.len(), 2560);
    }

    #[test]
    fn test_hamster_tunnel_min_params() {
        let mesh = SurfaceMesh::hamster_tunnel(3, 3, 0).unwrap();
        assert!(mesh.faces.len() > 0);
    }

    #[test]
    fn test_hamster_tunnel_invalid_params() {
        assert!(SurfaceMesh::hamster_tunnel(2, 16, 0).is_err()); // need >= 3 for loop
        assert!(SurfaceMesh::hamster_tunnel(5, 2, 0).is_err());
    }

    #[test]
    fn test_hamster_tunnel_adjacency_symmetric() {
        let mesh = SurfaceMesh::hamster_tunnel(4, 8, 42).unwrap();
        for (i, adj) in mesh.face_adjacency.iter().enumerate() {
            for &j in adj {
                assert!(
                    mesh.face_adjacency[j].contains(&i),
                    "Face {i} adjacent to {j}, but not vice versa"
                );
            }
        }
    }

    #[test]
    fn test_hamster_tunnel_deterministic() {
        let m1 = SurfaceMesh::hamster_tunnel(6, 12, 42).unwrap();
        let m2 = SurfaceMesh::hamster_tunnel(6, 12, 42).unwrap();
        assert_eq!(m1.vertices, m2.vertices);
        assert_eq!(m1.faces, m2.faces);
    }

    #[test]
    fn test_example_obj_files() {
        let examples_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/obj");
        let cases = [
            ("tetrahedron.obj", 4, 4),
            ("cube.obj", 8, 12),
            ("octahedron.obj", 6, 8),
            ("icosahedron.obj", 12, 20),
            ("torus.obj", 96, 192),
        ];
        for (file, expected_verts, expected_faces) in cases {
            let path = format!("{examples_dir}/{file}");
            let mesh = SurfaceMesh::from_obj(&path).unwrap_or_else(|e| panic!("{file}: {e}"));
            assert_eq!(
                mesh.vertices.len(),
                expected_verts,
                "{file}: expected {expected_verts} vertices, got {}",
                mesh.vertices.len()
            );
            assert_eq!(
                mesh.faces.len(),
                expected_faces,
                "{file}: expected {expected_faces} faces, got {}",
                mesh.faces.len()
            );
        }
    }

    #[test]
    fn test_obj_spec_builds() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/obj/icosahedron.obj");
        let spec = SurfaceSpec::ObjFile {
            path: path.to_string(),
        };
        assert_eq!(spec.label(), "OBJ File");
        let mesh = spec.build().unwrap();
        assert_eq!(mesh.faces.len(), 20);
    }
}
