use std::sync::{mpsc, Mutex};
use std::thread;

use bevy::prelude::*;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::render::mesh::PrimitiveTopology;
use bevy::render::render_asset::RenderAssetUsages;
use bevy_egui::{EguiContexts, EguiPlugin, egui};
use egui_plot::{Bar, BarChart, Line, Plot, PlotPoints};

use crate::metrics::{byte_frequency_histogram, high_order_entropy, unique_program_count, zero_byte_count};
use crate::soup::{Soup, SoupConfig};
use crate::substrate::Substrate;
use crate::surface::{SoupSurface, SoupSurfaceConfig, SurfaceMesh, face_normal};

/// Metrics snapshot sent from sim thread to render thread.
#[derive(Clone)]
pub struct EpochMetrics {
    pub epoch: usize,
    pub hoe: f64,
    pub unique_count: usize,
    pub zero_count: usize,
    pub byte_histogram: [usize; 256],
}

/// Per-cell color snapshot for surface visualization.
#[derive(Clone)]
pub struct SurfaceSnapshot {
    /// RGBA bytes, length = num_cells * 4.
    pub colors: Vec<u8>,
}

/// Available color modes for surface visualization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorMode {
    /// FNV hash of program bytes → RGB.
    Hash,
    /// Shannon entropy of byte distribution → heatmap.
    Entropy,
    /// Fraction of zero-valued bytes → gradient.
    Zeros,
    /// Average Hamming distance to geodesic neighbors → heatmap.
    NeighborSimilarity,
    /// Fraction of bytes that are valid instructions → heatmap.
    InstructionDensity,
    /// Number of distinct byte values → heatmap.
    UniqueBytes,
    /// Fraction of neighbors holding an identical program → heatmap.
    TerritorialDominance,
}

impl ColorMode {
    const ALL: [ColorMode; 7] = [
        ColorMode::Hash,
        ColorMode::Entropy,
        ColorMode::Zeros,
        ColorMode::NeighborSimilarity,
        ColorMode::InstructionDensity,
        ColorMode::UniqueBytes,
        ColorMode::TerritorialDominance,
    ];

    fn label(self) -> &'static str {
        match self {
            ColorMode::Hash => "Hash",
            ColorMode::Entropy => "Byte Entropy",
            ColorMode::Zeros => "Zero Fraction",
            ColorMode::NeighborSimilarity => "Neighbor Similarity",
            ColorMode::InstructionDensity => "Instruction Density",
            ColorMode::UniqueBytes => "Unique Bytes",
            ColorMode::TerritorialDominance => "Territorial Dominance",
        }
    }
}

/// Commands sent from render thread to sim thread.
pub enum SimCommand {
    Play,
    Pause,
    SetColorMode(ColorMode),
    SetBlur(f32),
}

/// Bevy resource wrapping the metrics channel receiver (Mutex for Sync).
#[derive(Resource)]
struct SimReceiver(Mutex<mpsc::Receiver<EpochMetrics>>);

/// Bevy resource wrapping the command channel sender.
#[derive(Resource)]
struct SimCommander(mpsc::Sender<SimCommand>);

/// Bevy resource storing metrics history for plotting.
#[derive(Resource, Default)]
struct SimulationHistory {
    entries: Vec<EpochMetrics>,
}

/// Bevy resource for playback state.
#[derive(Resource)]
struct PlaybackState {
    playing: bool,
    max_epochs: usize,
}

/// Bevy resource for visualization settings (color mode + blur), controlled by GUI.
#[derive(Resource)]
struct VizSettings {
    color_mode: ColorMode,
    blur: f32,
}

// ─── Surface-specific resources ──────────────────────────────────────────────

/// Bevy resource wrapping the surface snapshot channel receiver.
#[derive(Resource)]
struct SurfaceSnapshotReceiver(Mutex<mpsc::Receiver<SurfaceSnapshot>>);

/// Bevy resource storing the latest surface snapshot.
#[derive(Resource, Default)]
struct LatestSurfaceSnapshot {
    snapshot: Option<SurfaceSnapshot>,
    dirty: bool,
}

/// Bevy resource holding the mesh handle for color updates.
#[derive(Resource)]
struct SurfaceMeshHandle(Handle<Mesh>);

/// Bevy resource holding the number of cells.
#[derive(Resource)]
struct NumCells(usize);

/// Orbit camera component.
#[derive(Component)]
struct OrbitCamera {
    focus: Vec3,
    distance: f32,
    yaw: f32,
    pitch: f32,
}

/// Hash a program's bytes to an RGB color.
fn program_to_color(program: &[u8]) -> [u8; 3] {
    let mut hash: u32 = 2166136261;
    for &b in program {
        hash ^= b as u32;
        hash = hash.wrapping_mul(16777619);
    }
    [
        (hash & 0xFF) as u8,
        ((hash >> 8) & 0xFF) as u8,
        ((hash >> 16) & 0xFF) as u8,
    ]
}

/// Fill a color buffer from program data (RGBA, one entry per cell).
fn fill_surface_colors(programs: &[Vec<u8>], colors: &mut Vec<u8>) {
    colors.clear();
    for prog in programs {
        let [r, g, b] = program_to_color(prog);
        colors.push(r);
        colors.push(g);
        colors.push(b);
        colors.push(255);
    }
}

/// Fill color buffer by Shannon entropy of program byte distribution → heatmap.
fn fill_colors_entropy(programs: &[Vec<u8>], colors: &mut Vec<u8>) {
    colors.clear();
    for prog in programs {
        let mut counts = [0u32; 256];
        for &b in prog {
            counts[b as usize] += 1;
        }
        let n = prog.len() as f64;
        let mut entropy = 0.0f64;
        for &c in &counts {
            if c > 0 {
                let p = c as f64 / n;
                entropy -= p * p.log2();
            }
        }
        // Max entropy for 256 symbols is 8.0 bits, but for short programs
        // the practical max is log2(program_size).
        let max_entropy = (prog.len() as f64).log2().max(1.0);
        let t = (entropy / max_entropy).min(1.0) as f32;
        let [r, g, b] = heatmap(t);
        colors.push(r);
        colors.push(g);
        colors.push(b);
        colors.push(255);
    }
}

/// Fill color buffer by fraction of zero bytes → dark-to-bright gradient.
fn fill_colors_zeros(programs: &[Vec<u8>], colors: &mut Vec<u8>) {
    colors.clear();
    for prog in programs {
        let zero_count = prog.iter().filter(|&&b| b == 0).count();
        let t = zero_count as f32 / prog.len() as f32;
        // Invert: more zeros = darker (poisoned), fewer = brighter.
        let brightness = ((1.0 - t) * 255.0) as u8;
        colors.push(brightness);
        colors.push(brightness);
        colors.push(brightness);
        colors.push(255);
    }
}

/// Fill color buffer by average Hamming distance to geodesic neighbors → heatmap.
/// Low distance (similar to neighbors) = cool, high distance (boundary) = hot.
fn fill_colors_neighbor_similarity(
    programs: &[Vec<u8>],
    neighbor_indices: &[usize],
    neighbor_ranges: &[(usize, usize)],
    colors: &mut Vec<u8>,
) {
    colors.clear();
    let ps = programs.first().map_or(0, |p| p.len());
    let max_bits = (ps * 8) as f32;

    for (i, prog) in programs.iter().enumerate() {
        let (start, end) = neighbor_ranges[i];
        let neighbor_count = end - start;
        if neighbor_count == 0 || max_bits == 0.0 {
            colors.extend_from_slice(&[128, 128, 128, 255]);
            continue;
        }

        let mut total_dist = 0u32;
        for &ni in &neighbor_indices[start..end] {
            let dist: u32 = prog.iter().zip(programs[ni].iter())
                .map(|(a, b)| (a ^ b).count_ones())
                .sum();
            total_dist += dist;
        }

        let avg_dist = total_dist as f32 / neighbor_count as f32;
        let t = (avg_dist / max_bits).min(1.0);
        // Invert: similar neighbors = bright (colony), dissimilar = dark (boundary).
        let [r, g, b] = heatmap(1.0 - t);
        colors.push(r);
        colors.push(g);
        colors.push(b);
        colors.push(255);
    }
}

/// Fill color buffer by fraction of bytes that are valid instructions → heatmap.
fn fill_colors_instruction_density(
    programs: &[Vec<u8>],
    is_instruction: fn(u8) -> bool,
    colors: &mut Vec<u8>,
) {
    colors.clear();
    for prog in programs {
        let count = prog.iter().filter(|&&b| is_instruction(b)).count();
        let t = count as f32 / prog.len().max(1) as f32;
        let [r, g, b] = heatmap(t);
        colors.push(r);
        colors.push(g);
        colors.push(b);
        colors.push(255);
    }
}

/// Fill color buffer by number of distinct byte values → heatmap.
fn fill_colors_unique_bytes(programs: &[Vec<u8>], colors: &mut Vec<u8>) {
    colors.clear();
    for prog in programs {
        let mut seen = [false; 256];
        for &b in prog {
            seen[b as usize] = true;
        }
        let unique = seen.iter().filter(|&&s| s).count();
        // Normalize: max possible is min(program_size, 256).
        let max_unique = prog.len().min(256) as f32;
        let t = unique as f32 / max_unique.max(1.0);
        // Invert: few unique bytes (replicator) = hot, many (random) = cool.
        let [r, g, b] = heatmap(1.0 - t);
        colors.push(r);
        colors.push(g);
        colors.push(b);
        colors.push(255);
    }
}

/// Fill color buffer by fraction of neighbors holding an identical program → heatmap.
/// High dominance (deep inside territory) = hot, low (soup/border) = cool.
fn fill_colors_territorial_dominance(
    programs: &[Vec<u8>],
    neighbor_indices: &[usize],
    neighbor_ranges: &[(usize, usize)],
    colors: &mut Vec<u8>,
) {
    colors.clear();
    for (i, prog) in programs.iter().enumerate() {
        let (start, end) = neighbor_ranges[i];
        let neighbor_count = end - start;
        if neighbor_count == 0 {
            colors.extend_from_slice(&[128, 128, 128, 255]);
            continue;
        }

        let identical = neighbor_indices[start..end]
            .iter()
            .filter(|&&ni| programs[ni] == *prog)
            .count();

        let t = identical as f32 / neighbor_count as f32;
        let [r, g, b] = heatmap(t);
        colors.push(r);
        colors.push(g);
        colors.push(b);
        colors.push(255);
    }
}

/// Map a 0..1 value to a blue→cyan→green→yellow→red heatmap.
fn heatmap(t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    let (r, g, b) = if t < 0.25 {
        let s = t / 0.25;
        (0.0, s, 1.0)
    } else if t < 0.5 {
        let s = (t - 0.25) / 0.25;
        (0.0, 1.0, 1.0 - s)
    } else if t < 0.75 {
        let s = (t - 0.5) / 0.25;
        (s, 1.0, 0.0)
    } else {
        let s = (t - 0.75) / 0.25;
        (1.0, 1.0 - s, 0.0)
    };
    [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
}

/// Fill color buffer using the selected color mode.
fn fill_colors_for_mode<S: Substrate>(
    mode: ColorMode,
    programs: &[Vec<u8>],
    neighbor_indices: &[usize],
    neighbor_ranges: &[(usize, usize)],
    colors: &mut Vec<u8>,
) {
    match mode {
        ColorMode::Hash => fill_surface_colors(programs, colors),
        ColorMode::Entropy => fill_colors_entropy(programs, colors),
        ColorMode::Zeros => fill_colors_zeros(programs, colors),
        ColorMode::NeighborSimilarity => {
            fill_colors_neighbor_similarity(programs, neighbor_indices, neighbor_ranges, colors)
        }
        ColorMode::InstructionDensity => {
            fill_colors_instruction_density(programs, S::is_instruction, colors)
        }
        ColorMode::UniqueBytes => fill_colors_unique_bytes(programs, colors),
        ColorMode::TerritorialDominance => {
            fill_colors_territorial_dominance(programs, neighbor_indices, neighbor_ranges, colors)
        }
    }
}

/// Apply spatial blur in face-adjacency space.
fn blur_surface_colors(
    colors: &mut Vec<u8>,
    scratch: &mut Vec<u8>,
    face_adjacency: &[Vec<usize>],
    alpha: f32,
) {
    if alpha <= 0.0 {
        return;
    }
    let alpha = alpha.min(1.0);
    let one_minus_alpha = 1.0 - alpha;
    let num_faces = face_adjacency.len();
    scratch.resize(num_faces * 4, 0);

    for i in 0..num_faces {
        let idx = i * 4;
        let adj = &face_adjacency[i];
        let count = adj.len() as f32;

        if count == 0.0 {
            scratch[idx..idx + 4].copy_from_slice(&colors[idx..idx + 4]);
            continue;
        }

        let mut sum_r = 0u32;
        let mut sum_g = 0u32;
        let mut sum_b = 0u32;
        for &j in adj {
            let jdx = j * 4;
            sum_r += colors[jdx] as u32;
            sum_g += colors[jdx + 1] as u32;
            sum_b += colors[jdx + 2] as u32;
        }

        let center_r = colors[idx] as f32;
        let center_g = colors[idx + 1] as f32;
        let center_b = colors[idx + 2] as f32;

        let avg_r = sum_r as f32 / count;
        let avg_g = sum_g as f32 / count;
        let avg_b = sum_b as f32 / count;

        scratch[idx] = (one_minus_alpha * center_r + alpha * avg_r) as u8;
        scratch[idx + 1] = (one_minus_alpha * center_g + alpha * avg_g) as u8;
        scratch[idx + 2] = (one_minus_alpha * center_b + alpha * avg_b) as u8;
        scratch[idx + 3] = 255;
    }

    std::mem::swap(colors, scratch);
}

// ─── Standard (0D) visualization ─────────────────────────────────────────────

/// Launch the live visualization for a standard (0D) soup.
pub fn run_viz<S: Substrate + Send + 'static>(
    config: SoupConfig,
    seed: u64,
    max_epochs: usize,
    metrics_interval: usize,
) {
    let (metrics_tx, metrics_rx) = mpsc::channel::<EpochMetrics>();
    let (cmd_tx, cmd_rx) = mpsc::channel::<SimCommand>();

    thread::spawn(move || {
        sim_thread_loop::<S>(config, seed, max_epochs, metrics_interval, metrics_tx, cmd_rx);
    });

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Computational Life".into(),
                resolution: (1280., 800.).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin)
        .insert_resource(SimReceiver(Mutex::new(metrics_rx)))
        .insert_resource(SimCommander(cmd_tx))
        .insert_resource(SimulationHistory::default())
        .insert_resource(PlaybackState {
            playing: true,
            max_epochs,
        })
        .add_systems(Update, drain_metrics)
        .add_systems(Update, render_ui.after(drain_metrics))
        .run();
}

/// Simulation thread for standard soup.
fn sim_thread_loop<S: Substrate>(
    config: SoupConfig,
    seed: u64,
    max_epochs: usize,
    metrics_interval: usize,
    tx: mpsc::Sender<EpochMetrics>,
    cmd_rx: mpsc::Receiver<SimCommand>,
) {
    let mut soup = Soup::new(config, seed);
    let mut paused = false;
    let mut epoch = 0usize;

    let mut pop_buf = Vec::new();
    let _ = tx.send(compute_metrics(&soup, 0, &mut pop_buf));

    loop {
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                SimCommand::Play => paused = false,
                SimCommand::Pause => paused = true,
                SimCommand::SetColorMode(_) | SimCommand::SetBlur(_) => {}
            }
        }

        if paused || epoch >= max_epochs {
            thread::sleep(std::time::Duration::from_millis(10));
            continue;
        }

        soup.run_epoch::<S>();
        soup.mutate();
        epoch += 1;

        if epoch % metrics_interval == 0 || epoch == max_epochs {
            let metrics = compute_metrics(&soup, epoch, &mut pop_buf);
            if tx.send(metrics).is_err() {
                break;
            }
        }
    }
}

fn compute_metrics(soup: &Soup, epoch: usize, pop_buf: &mut Vec<u8>) -> EpochMetrics {
    soup.population_bytes_into(pop_buf);
    EpochMetrics {
        epoch,
        hoe: high_order_entropy(pop_buf),
        unique_count: unique_program_count(&soup.programs),
        zero_count: zero_byte_count(&soup.programs),
        byte_histogram: byte_frequency_histogram(&soup.programs),
    }
}

// ─── Surface visualization ───────────────────────────────────────────────────

/// Launch the live visualization for a surface soup.
pub fn run_viz_surface<S: Substrate + Send + Sync + 'static>(
    mesh: SurfaceMesh,
    config: SoupSurfaceConfig,
    seed: u64,
    max_epochs: usize,
    metrics_interval: usize,
    blur: f32,
) {
    let (metrics_tx, metrics_rx) = mpsc::channel::<EpochMetrics>();
    let (snap_tx, snap_rx) = mpsc::channel::<SurfaceSnapshot>();
    let (cmd_tx, cmd_rx) = mpsc::channel::<SimCommand>();

    let num_cells = mesh.num_cells();
    let face_adjacency = mesh.face_adjacency.clone();
    let face_adjacency_for_thread = face_adjacency.clone();

    // Build render mesh data before moving mesh to sim thread.
    let render_positions = build_render_positions(&mesh);
    let render_normals = build_render_normals(&mesh);
    let num_render_vertices = render_positions.len();

    let (center, radius) = mesh.bounding_sphere();

    thread::spawn(move || {
        sim_thread_loop_surface::<S>(
            mesh, config, seed, max_epochs, metrics_interval,
            metrics_tx, snap_tx, cmd_rx, &face_adjacency_for_thread, blur,
        );
    });

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Computational Life (Surface)".into(),
                resolution: (1280., 800.).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin)
        .insert_resource(SimReceiver(Mutex::new(metrics_rx)))
        .insert_resource(SurfaceSnapshotReceiver(Mutex::new(snap_rx)))
        .insert_resource(SimCommander(cmd_tx))
        .insert_resource(SimulationHistory::default())
        .insert_resource(LatestSurfaceSnapshot::default())
        .insert_resource(PlaybackState {
            playing: true,
            max_epochs,
        })
        .insert_resource(VizSettings {
            color_mode: ColorMode::Hash,
            blur,
        })
        .insert_resource(NumCells(num_cells))
        .insert_resource(SurfaceRenderData {
            positions: render_positions,
            normals: render_normals,
            num_render_vertices,
            center,
            radius,
        })
        .add_systems(Startup, setup_surface_scene)
        .add_systems(Update, drain_metrics)
        .add_systems(Update, drain_surface_snapshot)
        .add_systems(Update, update_surface_mesh.after(drain_surface_snapshot))
        .add_systems(Update, orbit_camera_system)
        .add_systems(Update, render_ui_surface.after(drain_metrics).after(drain_surface_snapshot))
        .run();
}

/// Render data passed to the bevy app.
#[derive(Resource)]
struct SurfaceRenderData {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    num_render_vertices: usize,
    center: [f32; 3],
    radius: f32,
}

/// Build per-face-vertex positions (3 vertices per face, unindexed).
fn build_render_positions(mesh: &SurfaceMesh) -> Vec<[f32; 3]> {
    let mut positions = Vec::with_capacity(mesh.faces.len() * 3);
    for face in &mesh.faces {
        positions.push(mesh.vertices[face[0]]);
        positions.push(mesh.vertices[face[1]]);
        positions.push(mesh.vertices[face[2]]);
    }
    positions
}

/// Build per-face-vertex normals (flat shading).
fn build_render_normals(mesh: &SurfaceMesh) -> Vec<[f32; 3]> {
    let mut normals = Vec::with_capacity(mesh.faces.len() * 3);
    for face in &mesh.faces {
        let n = face_normal(
            &mesh.vertices[face[0]],
            &mesh.vertices[face[1]],
            &mesh.vertices[face[2]],
        );
        normals.push(n);
        normals.push(n);
        normals.push(n);
    }
    normals
}

/// Simulation thread for surface soup.
fn sim_thread_loop_surface<S: Substrate + Sync>(
    mesh: SurfaceMesh,
    config: SoupSurfaceConfig,
    seed: u64,
    max_epochs: usize,
    metrics_interval: usize,
    metrics_tx: mpsc::Sender<EpochMetrics>,
    snap_tx: mpsc::Sender<SurfaceSnapshot>,
    cmd_rx: mpsc::Receiver<SimCommand>,
    face_adjacency: &[Vec<usize>],
    blur: f32,
) {
    let mut soup = SoupSurface::new(mesh, config, seed);
    let mut paused = false;
    let mut epoch = 0usize;
    let mut color_mode = ColorMode::Hash;
    let mut blur = blur;

    let num_cells = soup.mesh.num_cells();
    let mut color_buf: Vec<u8> = Vec::with_capacity(num_cells * 4);
    let mut blur_scratch: Vec<u8> = Vec::new();
    let mut pop_buf: Vec<u8> = Vec::new();

    // Send initial state.
    let _ = metrics_tx.send(compute_metrics_surface(&soup, 0, &mut pop_buf));
    fill_colors_for_mode::<S>(color_mode, &soup.programs, &soup.mesh.neighbor_indices, &soup.mesh.neighbor_ranges, &mut color_buf);
    blur_surface_colors(&mut color_buf, &mut blur_scratch, face_adjacency, blur);
    let _ = snap_tx.send(SurfaceSnapshot { colors: color_buf.clone() });

    // Throttle snapshots to ~60fps.
    let snap_interval = std::time::Duration::from_millis(16);
    let mut last_snap_send = std::time::Instant::now();

    loop {
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                SimCommand::Play => paused = false,
                SimCommand::Pause => paused = true,
                SimCommand::SetColorMode(mode) => color_mode = mode,
                SimCommand::SetBlur(b) => blur = b,
            }
        }

        if paused || epoch >= max_epochs {
            thread::sleep(std::time::Duration::from_millis(10));
            continue;
        }

        soup.run_epoch::<S>();
        soup.mutate();
        epoch += 1;

        // Send snapshot at most ~60fps.
        let now = std::time::Instant::now();
        if now.duration_since(last_snap_send) >= snap_interval || epoch == max_epochs {
            fill_colors_for_mode::<S>(color_mode, &soup.programs, &soup.mesh.neighbor_indices, &soup.mesh.neighbor_ranges, &mut color_buf);
            blur_surface_colors(&mut color_buf, &mut blur_scratch, face_adjacency, blur);
            if snap_tx.send(SurfaceSnapshot { colors: color_buf.clone() }).is_err() {
                break;
            }
            last_snap_send = now;
        }

        if epoch % metrics_interval == 0 || epoch == max_epochs {
            if metrics_tx.send(compute_metrics_surface(&soup, epoch, &mut pop_buf)).is_err() {
                break;
            }
        }
    }
}

fn compute_metrics_surface(soup: &SoupSurface, epoch: usize, pop_buf: &mut Vec<u8>) -> EpochMetrics {
    soup.population_bytes_into(pop_buf);
    EpochMetrics {
        epoch,
        hoe: high_order_entropy(pop_buf),
        unique_count: unique_program_count(&soup.programs),
        zero_count: zero_byte_count(&soup.programs),
        byte_histogram: byte_frequency_histogram(&soup.programs),
    }
}

// ─── Surface bevy systems ────────────────────────────────────────────────────

/// Startup system: create the 3D mesh, camera, and lights.
fn setup_surface_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    render_data: Res<SurfaceRenderData>,
) {
    // Build bevy mesh with per-face vertices (unindexed for flat shading + per-face colors).
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, render_data.positions.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, render_data.normals.clone());

    // Initial colors: all grey.
    let initial_colors: Vec<[f32; 4]> = vec![[0.5, 0.5, 0.5, 1.0]; render_data.num_render_vertices];
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, initial_colors);

    let mesh_handle = meshes.add(mesh);

    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.8,
        metallic: 0.0,
        reflectance: 0.1,
        ..default()
    });

    commands.spawn((
        Mesh3d(mesh_handle.clone()),
        MeshMaterial3d(material),
        Transform::default(),
    ));

    commands.insert_resource(SurfaceMeshHandle(mesh_handle));

    // Camera with orbit controls.
    let center = Vec3::from_array(render_data.center);
    let distance = render_data.radius * 2.5;
    let yaw = 0.4_f32;
    let pitch = -0.5_f32;

    let rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
    let camera_pos = center + rotation * Vec3::new(0.0, 0.0, distance);

    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(camera_pos).looking_at(center, Vec3::Y),
        OrbitCamera {
            focus: center,
            distance,
            yaw,
            pitch,
        },
    ));

    // Directional light for depth cues.
    commands.spawn((
        DirectionalLight {
            illuminance: 12000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.6, 0.4, 0.0)),
    ));

    // Ambient light so no face is fully dark.
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 200.0,
    });
}

/// System: drain surface snapshots, keeping only the latest.
fn drain_surface_snapshot(
    receiver: Res<SurfaceSnapshotReceiver>,
    mut latest: ResMut<LatestSurfaceSnapshot>,
) {
    let rx = receiver.0.lock().unwrap();
    while let Ok(snapshot) = rx.try_recv() {
        latest.snapshot = Some(snapshot);
        latest.dirty = true;
    }
}

/// System: update the mesh vertex colors from the latest surface snapshot.
fn update_surface_mesh(
    mut meshes: ResMut<Assets<Mesh>>,
    handle: Option<Res<SurfaceMeshHandle>>,
    mut latest: ResMut<LatestSurfaceSnapshot>,
    num_cells: Res<NumCells>,
) {
    if !latest.dirty {
        return;
    }
    let Some(handle) = handle else { return };
    let Some(ref snap) = latest.snapshot else { return };
    let Some(mesh) = meshes.get_mut(&handle.0) else { return };

    // Map per-cell RGBA to per-vertex colors (3 vertices per face).
    let mut vertex_colors: Vec<[f32; 4]> = Vec::with_capacity(num_cells.0 * 3);
    for i in 0..num_cells.0 {
        let idx = i * 4;
        let r = snap.colors[idx] as f32 / 255.0;
        let g = snap.colors[idx + 1] as f32 / 255.0;
        let b = snap.colors[idx + 2] as f32 / 255.0;
        let color = [r, g, b, 1.0];
        vertex_colors.push(color);
        vertex_colors.push(color);
        vertex_colors.push(color);
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vertex_colors);
    latest.dirty = false;
}

/// System: orbit camera with mouse controls.
fn orbit_camera_system(
    mut contexts: EguiContexts,
    mut query: Query<(&mut OrbitCamera, &mut Transform)>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut scroll: EventReader<MouseWheel>,
) {
    let egui_wants_pointer = contexts.ctx_mut().is_pointer_over_area();

    // Collect events (must drain even if egui has focus).
    let motion: Vec<_> = mouse_motion.read().cloned().collect();
    let scrolls: Vec<_> = scroll.read().cloned().collect();

    if egui_wants_pointer {
        return;
    }

    let Ok((mut orbit, mut transform)) = query.get_single_mut() else {
        return;
    };

    // Left-drag: orbit.
    if mouse_buttons.pressed(MouseButton::Left) {
        for ev in &motion {
            orbit.yaw -= ev.delta.x * 0.005;
            orbit.pitch -= ev.delta.y * 0.005;
            orbit.pitch = orbit.pitch.clamp(
                -std::f32::consts::FRAC_PI_2 + 0.05,
                std::f32::consts::FRAC_PI_2 - 0.05,
            );
        }
    }

    // Right-drag: pan.
    if mouse_buttons.pressed(MouseButton::Right) {
        for ev in &motion {
            let right = transform.right().as_vec3();
            let up = transform.up().as_vec3();
            let pan_speed = orbit.distance * 0.001;
            orbit.focus += (-right * ev.delta.x + up * ev.delta.y) * pan_speed;
        }
    }

    // Scroll: zoom.
    for ev in &scrolls {
        orbit.distance *= 1.0 - ev.y * 0.03;
        orbit.distance = orbit.distance.max(0.05);
    }

    // Update transform from orbit parameters.
    let rotation = Quat::from_euler(EulerRot::YXZ, orbit.yaw, orbit.pitch, 0.0);
    transform.translation = orbit.focus + rotation * Vec3::new(0.0, 0.0, orbit.distance);
    transform.look_at(orbit.focus, Vec3::Y);
}

/// System: render egui UI with metrics (surface view — side panel only).
fn render_ui_surface(
    mut contexts: EguiContexts,
    history: Res<SimulationHistory>,
    mut playback: ResMut<PlaybackState>,
    mut viz: ResMut<VizSettings>,
    commander: Res<SimCommander>,
) {
    let ctx = contexts.ctx_mut();

    egui::SidePanel::right("metrics_panel").min_width(350.0).show(ctx, |ui| {
        render_controls_section(ui, &history, &mut playback, &commander);
        ui.separator();
        render_viz_settings(ui, &mut viz, &commander);
        ui.separator();

        let entries = &history.entries;
        if entries.is_empty() {
            return;
        }
        let available_height = ui.available_height();
        let plot_height = (available_height - 40.0) / 3.0;
        render_time_series_plots_compact(ui, entries, plot_height);
    });
}

/// Render the Visualization settings section (color mode + blur slider).
fn render_viz_settings(
    ui: &mut egui::Ui,
    viz: &mut VizSettings,
    commander: &SimCommander,
) {
    ui.heading("Visualization");
    ui.add_space(4.0);

    // Color mode selector.
    let prev_mode = viz.color_mode;
    egui::ComboBox::from_label("Color mode")
        .selected_text(viz.color_mode.label())
        .show_ui(ui, |ui| {
            for mode in ColorMode::ALL {
                ui.selectable_value(&mut viz.color_mode, mode, mode.label());
            }
        });
    if viz.color_mode != prev_mode {
        let _ = commander.0.send(SimCommand::SetColorMode(viz.color_mode));
    }

    ui.add_space(4.0);

    // Blur slider.
    let prev_blur = viz.blur;
    ui.add(egui::Slider::new(&mut viz.blur, 0.0..=1.0).text("Blur"));
    if (viz.blur - prev_blur).abs() > f32::EPSILON {
        let _ = commander.0.send(SimCommand::SetBlur(viz.blur));
    }

    ui.add_space(8.0);
}

// ─── Shared systems ──────────────────────────────────────────────────────────

/// System: drain metrics from the channel each frame.
fn drain_metrics(receiver: Res<SimReceiver>, mut history: ResMut<SimulationHistory>) {
    let rx = receiver.0.lock().unwrap();
    while let Ok(metrics) = rx.try_recv() {
        history.entries.push(metrics);
    }
}

/// System: render egui UI with plots and controls (standard 0D view).
fn render_ui(
    mut contexts: EguiContexts,
    history: Res<SimulationHistory>,
    mut playback: ResMut<PlaybackState>,
    commander: Res<SimCommander>,
) {
    let ctx = contexts.ctx_mut();

    render_controls_panel(ctx, &history, &mut playback, &commander);

    egui::CentralPanel::default().show(ctx, |ui| {
        let entries = &history.entries;
        if entries.is_empty() {
            ui.label("Waiting for simulation data...");
            return;
        }

        let available_height = ui.available_height();
        let plot_height = (available_height - 60.0) / 4.0;
        render_time_series_plots(ui, entries, plot_height);
    });
}

// ─── Shared UI helpers ───────────────────────────────────────────────────────

fn render_controls_panel(
    ctx: &egui::Context,
    history: &SimulationHistory,
    playback: &mut PlaybackState,
    commander: &SimCommander,
) {
    egui::SidePanel::left("controls").min_width(220.0).show(ctx, |ui| {
        render_controls_section(ui, history, playback, commander);
    });
}

fn render_controls_section(
    ui: &mut egui::Ui,
    history: &SimulationHistory,
    playback: &mut PlaybackState,
    commander: &SimCommander,
) {
    ui.heading("Playback");
    ui.add_space(8.0);

    let current_epoch = history.entries.last().map(|e| e.epoch).unwrap_or(0);
    ui.label(format!("Epoch: {} / {}", current_epoch, playback.max_epochs));
    ui.add_space(8.0);

    let label = if playback.playing { "Pause" } else { "Play" };
    if ui.button(label).clicked() {
        playback.playing = !playback.playing;
        let cmd = if playback.playing {
            SimCommand::Play
        } else {
            SimCommand::Pause
        };
        let _ = commander.0.send(cmd);
    }

    ui.add_space(16.0);

    if let Some(latest) = history.entries.last() {
        ui.heading("Current Metrics");
        ui.add_space(4.0);
        ui.label(format!("HOE: {:.4}", latest.hoe));
        ui.label(format!("Unique programs: {}", latest.unique_count));
        ui.label(format!("Zero bytes: {}", latest.zero_count));
    }
}

fn render_time_series_plots(ui: &mut egui::Ui, entries: &[EpochMetrics], plot_height: f32) {
    ui.label("High-Order Entropy");
    let hoe_points: PlotPoints = entries.iter().map(|e| [e.epoch as f64, e.hoe]).collect();
    Plot::new("hoe_plot")
        .height(plot_height)
        .show(ui, |plot_ui: &mut egui_plot::PlotUi| {
            plot_ui.line(Line::new(hoe_points).name("HOE"));
        });

    ui.label("Unique Programs");
    let unique_points: PlotPoints = entries.iter().map(|e| [e.epoch as f64, e.unique_count as f64]).collect();
    Plot::new("unique_plot")
        .height(plot_height)
        .show(ui, |plot_ui: &mut egui_plot::PlotUi| {
            plot_ui.line(Line::new(unique_points).name("Unique"));
        });

    ui.label("Zero Byte Count");
    let zero_points: PlotPoints = entries.iter().map(|e| [e.epoch as f64, e.zero_count as f64]).collect();
    Plot::new("zero_plot")
        .height(plot_height)
        .show(ui, |plot_ui: &mut egui_plot::PlotUi| {
            plot_ui.line(Line::new(zero_points).name("Zeros"));
        });

    ui.label("Byte Frequency Distribution");
    if let Some(latest) = entries.last() {
        let bars: Vec<Bar> = latest.byte_histogram.iter().enumerate()
            .map(|(i, &count)| Bar::new(i as f64, count as f64))
            .collect();
        Plot::new("byte_hist")
            .height(plot_height)
            .show(ui, |plot_ui: &mut egui_plot::PlotUi| {
                plot_ui.bar_chart(BarChart::new(bars).name("Frequency"));
            });
    }
}

fn render_time_series_plots_compact(ui: &mut egui::Ui, entries: &[EpochMetrics], plot_height: f32) {
    ui.label("High-Order Entropy");
    let hoe_points: PlotPoints = entries.iter().map(|e| [e.epoch as f64, e.hoe]).collect();
    Plot::new("hoe_plot")
        .height(plot_height)
        .show(ui, |plot_ui: &mut egui_plot::PlotUi| {
            plot_ui.line(Line::new(hoe_points).name("HOE"));
        });

    ui.label("Unique Programs");
    let unique_points: PlotPoints = entries.iter().map(|e| [e.epoch as f64, e.unique_count as f64]).collect();
    Plot::new("unique_plot")
        .height(plot_height)
        .show(ui, |plot_ui: &mut egui_plot::PlotUi| {
            plot_ui.line(Line::new(unique_points).name("Unique"));
        });

    ui.label("Zero Byte Count");
    let zero_points: PlotPoints = entries.iter().map(|e| [e.epoch as f64, e.zero_count as f64]).collect();
    Plot::new("zero_plot")
        .height(plot_height)
        .show(ui, |plot_ui: &mut egui_plot::PlotUi| {
            plot_ui.line(Line::new(zero_points).name("Zeros"));
        });
}
