use std::sync::{mpsc, Mutex};
use std::thread;

use bevy::prelude::*;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::render::mesh::PrimitiveTopology;
use bevy::render::render_asset::RenderAssetUsages;
use bevy_egui::{EguiContexts, EguiPlugin, egui};
use egui_plot::{Line, Plot, PlotPoints};

use crate::bff::Bff;
use crate::forth::Forth;
use crate::metrics::{byte_frequency_histogram, high_order_entropy, unique_program_count, zero_byte_count};
use crate::substrate::Substrate;
use crate::subleq::{Subleq, Rsubleq4};
use crate::surface::{SoupSurface, SoupSurfaceConfig, SurfaceMesh, SurfaceSpec, face_normal};

const MAX_PLOT_POINTS: usize = 1000;

// ─── App state machine ───────────────────────────────────────────────────────

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum AppState {
    #[default]
    Menu,
    Simulating,
}

// ─── Public types ────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubstrateKind {
    Bff,
    Forth,
    Subleq,
    Rsubleq4,
}

impl SubstrateKind {
    const ALL: [SubstrateKind; 4] = [
        SubstrateKind::Bff,
        SubstrateKind::Forth,
        SubstrateKind::Subleq,
        SubstrateKind::Rsubleq4,
    ];

    fn label(self) -> &'static str {
        match self {
            SubstrateKind::Bff => "BFF",
            SubstrateKind::Forth => "Forth",
            SubstrateKind::Subleq => "SUBLEQ",
            SubstrateKind::Rsubleq4 => "RSUBLEQ4",
        }
    }
}

/// Available color modes for surface visualization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorMode {
    Hash,
    Entropy,
    Zeros,
    NeighborSimilarity,
    InstructionDensity,
    UniqueBytes,
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

/// Persistent configuration resource — survives state transitions.
#[derive(Resource)]
pub struct MenuConfig {
    pub substrate: SubstrateKind,
    // Surface
    pub selected_type: usize,
    pub sphere_subdivisions: usize,
    pub torus_major: usize,
    pub torus_minor: usize,
    pub grid_width: usize,
    pub grid_height: usize,
    pub tunnel_spheres: usize,
    pub tunnel_segments: usize,
    pub seed: u64,
    pub neighbor_radius: Option<f32>,
    // Simulation
    pub program_size: usize,
    pub step_limit: usize,
    pub mutation_rate: f64,
    pub max_epochs: usize,
    pub metrics_interval: usize,
    // Visualization
    pub color_mode: ColorMode,
    pub blur: f32,
    // Error display
    pub last_error: Option<String>,
}

impl MenuConfig {
    pub fn new(
        substrate: SubstrateKind,
        spec: &SurfaceSpec,
        seed: u64,
        neighbor_radius: Option<f32>,
        program_size: usize,
        step_limit: usize,
        mutation_rate: f64,
        max_epochs: usize,
        metrics_interval: usize,
        blur: f32,
    ) -> Self {
        let selected_type = match spec {
            SurfaceSpec::Sphere { .. } => 0,
            SurfaceSpec::Torus { .. } => 1,
            SurfaceSpec::FlatGrid { .. } => 2,
            SurfaceSpec::HamsterTunnel { .. } => 3,
        };
        Self {
            substrate,
            selected_type,
            sphere_subdivisions: match spec {
                SurfaceSpec::Sphere { subdivisions } => *subdivisions,
                _ => 4,
            },
            torus_major: match spec {
                SurfaceSpec::Torus { major, .. } => *major,
                _ => 32,
            },
            torus_minor: match spec {
                SurfaceSpec::Torus { minor, .. } => *minor,
                _ => 16,
            },
            grid_width: match spec {
                SurfaceSpec::FlatGrid { width, .. } => *width,
                _ => 64,
            },
            grid_height: match spec {
                SurfaceSpec::FlatGrid { height, .. } => *height,
                _ => 64,
            },
            tunnel_spheres: match spec {
                SurfaceSpec::HamsterTunnel { num_spheres, .. } => *num_spheres,
                _ => 10,
            },
            tunnel_segments: match spec {
                SurfaceSpec::HamsterTunnel { segments, .. } => *segments,
                _ => 24,
            },
            seed,
            neighbor_radius,
            program_size,
            step_limit,
            mutation_rate,
            max_epochs,
            metrics_interval,
            color_mode: ColorMode::Hash,
            blur,
            last_error: None,
        }
    }

    fn current_spec(&self) -> SurfaceSpec {
        match self.selected_type {
            0 => SurfaceSpec::Sphere { subdivisions: self.sphere_subdivisions },
            1 => SurfaceSpec::Torus { major: self.torus_major, minor: self.torus_minor },
            2 => SurfaceSpec::FlatGrid { width: self.grid_width, height: self.grid_height },
            3 => SurfaceSpec::HamsterTunnel {
                num_spheres: self.tunnel_spheres,
                segments: self.tunnel_segments,
                seed: self.seed,
            },
            _ => SurfaceSpec::Sphere { subdivisions: 4 },
        }
    }

    fn to_surface_config_gui(&self) -> SurfaceConfigGui {
        SurfaceConfigGui {
            selected_type: self.selected_type,
            sphere_subdivisions: self.sphere_subdivisions,
            torus_major: self.torus_major,
            torus_minor: self.torus_minor,
            grid_width: self.grid_width,
            grid_height: self.grid_height,
            tunnel_spheres: self.tunnel_spheres,
            tunnel_segments: self.tunnel_segments,
            seed: self.seed,
            neighbor_radius: self.neighbor_radius,
            last_error: None,
        }
    }
}

// ─── Shared data types ───────────────────────────────────────────────────────

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
    pub colors: Vec<u8>,
}

/// Commands sent from render thread to sim thread.
pub enum SimCommand {
    Play,
    Pause,
    SetColorMode(ColorMode),
    SetBlur(f32),
    ResetSurface {
        mesh: SurfaceMesh,
        config: SoupSurfaceConfig,
        seed: u64,
    },
}

// ─── Marker components ──────────────────────────────────────────────────────

#[derive(Component)]
struct SimEntity;

#[derive(Component)]
struct MenuEntity;

// ─── Sim-only resources ──────────────────────────────────────────────────────

#[derive(Resource)]
struct SimReceiver(Mutex<mpsc::Receiver<EpochMetrics>>);

#[derive(Resource)]
struct SimCommander(mpsc::Sender<SimCommand>);

#[derive(Resource, Default)]
struct SimulationHistory {
    entries: Vec<EpochMetrics>,
    awaiting_reset: bool,
}

#[derive(Resource)]
struct PlaybackState {
    playing: bool,
    max_epochs: usize,
}

#[derive(Resource)]
struct VizSettings {
    color_mode: ColorMode,
    blur: f32,
}

#[derive(Resource)]
struct SurfaceSnapshotReceiver(Mutex<mpsc::Receiver<SurfaceSnapshot>>);

#[derive(Resource, Default)]
struct LatestSurfaceSnapshot {
    snapshot: Option<SurfaceSnapshot>,
    dirty: bool,
}

#[derive(Resource)]
struct SurfaceMeshHandle(Handle<Mesh>);

#[derive(Resource)]
struct NumCells(usize);

#[derive(Resource)]
struct SimConfig(SoupSurfaceConfig);

/// GUI state for surface configuration panel (in-sim sidebar).
#[derive(Resource)]
struct SurfaceConfigGui {
    selected_type: usize,
    sphere_subdivisions: usize,
    torus_major: usize,
    torus_minor: usize,
    grid_width: usize,
    grid_height: usize,
    tunnel_spheres: usize,
    tunnel_segments: usize,
    seed: u64,
    neighbor_radius: Option<f32>,
    last_error: Option<String>,
}

impl SurfaceConfigGui {
    fn current_spec(&self) -> SurfaceSpec {
        match self.selected_type {
            0 => SurfaceSpec::Sphere { subdivisions: self.sphere_subdivisions },
            1 => SurfaceSpec::Torus { major: self.torus_major, minor: self.torus_minor },
            2 => SurfaceSpec::FlatGrid { width: self.grid_width, height: self.grid_height },
            3 => SurfaceSpec::HamsterTunnel {
                num_spheres: self.tunnel_spheres,
                segments: self.tunnel_segments,
                seed: self.seed,
            },
            _ => SurfaceSpec::Sphere { subdivisions: 4 },
        }
    }
}

#[derive(Resource, Default)]
struct PendingMeshRebuild {
    active: bool,
}

#[derive(Resource)]
struct SurfaceRenderData {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    num_render_vertices: usize,
    center: [f32; 3],
    radius: f32,
}

#[derive(Component)]
struct OrbitCamera {
    focus: Vec3,
    distance: f32,
    yaw: f32,
    pitch: f32,
}

// ─── Color / heatmap / blur helpers ──────────────────────────────────────────

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
        let max_entropy = (prog.len() as f64).log2().max(1.0);
        let t = (entropy / max_entropy).min(1.0) as f32;
        let [r, g, b] = heatmap(t);
        colors.push(r);
        colors.push(g);
        colors.push(b);
        colors.push(255);
    }
}

fn fill_colors_zeros(programs: &[Vec<u8>], colors: &mut Vec<u8>) {
    colors.clear();
    for prog in programs {
        let zero_count = prog.iter().filter(|&&b| b == 0).count();
        let t = zero_count as f32 / prog.len() as f32;
        let brightness = ((1.0 - t) * 255.0) as u8;
        colors.push(brightness);
        colors.push(brightness);
        colors.push(brightness);
        colors.push(255);
    }
}

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
        let [r, g, b] = heatmap(1.0 - t);
        colors.push(r);
        colors.push(g);
        colors.push(b);
        colors.push(255);
    }
}

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

fn fill_colors_unique_bytes(programs: &[Vec<u8>], colors: &mut Vec<u8>) {
    colors.clear();
    for prog in programs {
        let mut seen = [false; 256];
        for &b in prog {
            seen[b as usize] = true;
        }
        let unique = seen.iter().filter(|&&s| s).count();
        let max_unique = prog.len().min(256) as f32;
        let t = unique as f32 / max_unique.max(1.0);
        let [r, g, b] = heatmap(1.0 - t);
        colors.push(r);
        colors.push(g);
        colors.push(b);
        colors.push(255);
    }
}

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

// ─── Spawn sim thread (runtime substrate dispatch) ───────────────────────────

fn spawn_sim_thread(
    kind: SubstrateKind,
    mesh: SurfaceMesh,
    config: SoupSurfaceConfig,
    seed: u64,
    max_epochs: usize,
    metrics_interval: usize,
    blur: f32,
) -> (mpsc::Receiver<EpochMetrics>, mpsc::Receiver<SurfaceSnapshot>, mpsc::Sender<SimCommand>) {
    let (metrics_tx, metrics_rx) = mpsc::channel();
    let (snap_tx, snap_rx) = mpsc::channel();
    let (cmd_tx, cmd_rx) = mpsc::channel();

    let face_adjacency = mesh.face_adjacency.clone();

    match kind {
        SubstrateKind::Bff => {
            thread::spawn(move || {
                sim_thread_loop_surface::<Bff>(
                    mesh, config, seed, max_epochs, metrics_interval,
                    metrics_tx, snap_tx, cmd_rx, face_adjacency, blur,
                );
            });
        }
        SubstrateKind::Forth => {
            thread::spawn(move || {
                sim_thread_loop_surface::<Forth>(
                    mesh, config, seed, max_epochs, metrics_interval,
                    metrics_tx, snap_tx, cmd_rx, face_adjacency, blur,
                );
            });
        }
        SubstrateKind::Subleq => {
            thread::spawn(move || {
                sim_thread_loop_surface::<Subleq>(
                    mesh, config, seed, max_epochs, metrics_interval,
                    metrics_tx, snap_tx, cmd_rx, face_adjacency, blur,
                );
            });
        }
        SubstrateKind::Rsubleq4 => {
            thread::spawn(move || {
                sim_thread_loop_surface::<Rsubleq4>(
                    mesh, config, seed, max_epochs, metrics_interval,
                    metrics_tx, snap_tx, cmd_rx, face_adjacency, blur,
                );
            });
        }
    }

    (metrics_rx, snap_rx, cmd_tx)
}

// ─── Surface sim thread ─────────────────────────────────────────────────────

fn sim_thread_loop_surface<S: Substrate + Sync>(
    mesh: SurfaceMesh,
    config: SoupSurfaceConfig,
    seed: u64,
    max_epochs: usize,
    metrics_interval: usize,
    metrics_tx: mpsc::Sender<EpochMetrics>,
    snap_tx: mpsc::Sender<SurfaceSnapshot>,
    cmd_rx: mpsc::Receiver<SimCommand>,
    mut face_adjacency: Vec<Vec<usize>>,
    blur: f32,
) {
    let mut soup = SoupSurface::new(mesh, config, seed);
    let mut paused = false;
    let mut epoch = 0usize;
    let mut color_mode = ColorMode::Hash;
    let mut blur = blur;

    let mut num_cells = soup.mesh.num_cells();
    let mut color_buf: Vec<u8> = Vec::with_capacity(num_cells * 4);
    let mut blur_scratch: Vec<u8> = Vec::new();
    let mut pop_buf: Vec<u8> = Vec::new();

    // Send initial state.
    let _ = metrics_tx.send(compute_metrics_surface(&soup, 0, &mut pop_buf));
    fill_colors_for_mode::<S>(color_mode, &soup.programs, &soup.mesh.neighbor_indices, &soup.mesh.neighbor_ranges, &mut color_buf);
    blur_surface_colors(&mut color_buf, &mut blur_scratch, &face_adjacency, blur);
    let _ = snap_tx.send(SurfaceSnapshot { colors: color_buf.clone() });

    let snap_interval = std::time::Duration::from_millis(16);
    let mut last_snap_send = std::time::Instant::now();

    loop {
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                SimCommand::Play => paused = false,
                SimCommand::Pause => paused = true,
                SimCommand::SetColorMode(mode) => color_mode = mode,
                SimCommand::SetBlur(b) => blur = b,
                SimCommand::ResetSurface { mesh: new_mesh, config: new_config, seed: new_seed } => {
                    face_adjacency = new_mesh.face_adjacency.clone();
                    soup = SoupSurface::new(new_mesh, new_config, new_seed);
                    epoch = 0;
                    num_cells = soup.mesh.num_cells();
                    color_buf = Vec::with_capacity(num_cells * 4);
                    blur_scratch = Vec::new();
                    pop_buf = Vec::new();
                    let _ = metrics_tx.send(compute_metrics_surface(&soup, 0, &mut pop_buf));
                    fill_colors_for_mode::<S>(color_mode, &soup.programs, &soup.mesh.neighbor_indices, &soup.mesh.neighbor_ranges, &mut color_buf);
                    blur_surface_colors(&mut color_buf, &mut blur_scratch, &face_adjacency, blur);
                    let _ = snap_tx.send(SurfaceSnapshot { colors: color_buf.clone() });
                }
            }
        }

        if paused || epoch >= max_epochs {
            thread::sleep(std::time::Duration::from_millis(10));
            continue;
        }

        soup.run_epoch::<S>();
        soup.mutate();
        epoch += 1;

        let now = std::time::Instant::now();
        if now.duration_since(last_snap_send) >= snap_interval || epoch == max_epochs {
            fill_colors_for_mode::<S>(color_mode, &soup.programs, &soup.mesh.neighbor_indices, &soup.mesh.neighbor_ranges, &mut color_buf);
            blur_surface_colors(&mut color_buf, &mut blur_scratch, &face_adjacency, blur);
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

// ─── Render mesh helpers ─────────────────────────────────────────────────────

fn build_render_positions(mesh: &SurfaceMesh) -> Vec<[f32; 3]> {
    let mut positions = Vec::with_capacity(mesh.faces.len() * 3);
    for face in &mesh.faces {
        positions.push(mesh.vertices[face[0]]);
        positions.push(mesh.vertices[face[1]]);
        positions.push(mesh.vertices[face[2]]);
    }
    positions
}

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

// ─── App entry point ─────────────────────────────────────────────────────────

pub fn run_app(menu_config: MenuConfig) {
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
        .init_state::<AppState>()
        .insert_resource(menu_config)
        // Menu lifecycle
        .add_systems(OnEnter(AppState::Menu), enter_menu)
        .add_systems(OnExit(AppState::Menu), exit_menu)
        .add_systems(Update, render_menu_ui.run_if(in_state(AppState::Menu)))
        // Simulation lifecycle
        .add_systems(OnEnter(AppState::Simulating), enter_simulation)
        .add_systems(OnExit(AppState::Simulating), exit_simulation)
        // Simulation update systems (all gated)
        .add_systems(Update, (
            drain_metrics,
            drain_surface_snapshot,
            update_surface_mesh.after(drain_surface_snapshot),
            orbit_camera_system,
            render_ui_surface.after(drain_metrics).after(drain_surface_snapshot),
            apply_mesh_rebuild.after(render_ui_surface),
        ).run_if(in_state(AppState::Simulating)))
        .run();
}

// ─── Menu systems ────────────────────────────────────────────────────────────

fn enter_menu(mut commands: Commands) {
    commands.spawn((Camera2d, MenuEntity));
}

fn exit_menu(mut commands: Commands, query: Query<Entity, With<MenuEntity>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn render_menu_ui(
    mut contexts: EguiContexts,
    mut menu: ResMut<MenuConfig>,
    mut next_state: ResMut<NextState<AppState>>,
    windows: Query<&Window>,
) {
    if windows.is_empty() {
        return;
    }
    let ctx = contexts.ctx_mut();

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.heading("Computational Life");
            ui.add_space(20.0);
        });

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Substrate selector.
            ui.heading("Substrate");
            ui.add_space(4.0);
            egui::ComboBox::from_label("Substrate")
                .selected_text(menu.substrate.label())
                .show_ui(ui, |ui| {
                    for kind in SubstrateKind::ALL {
                        ui.selectable_value(&mut menu.substrate, kind, kind.label());
                    }
                });
            ui.add_space(12.0);

            // Reborrow to allow disjoint field borrows.
            let m = &mut *menu;

            // Surface parameters (shared helper).
            ui.heading("Surface");
            ui.add_space(4.0);
            render_surface_params(
                ui,
                &mut m.selected_type,
                &mut m.sphere_subdivisions,
                &mut m.torus_major,
                &mut m.torus_minor,
                &mut m.grid_width,
                &mut m.grid_height,
                &mut m.tunnel_spheres,
                &mut m.tunnel_segments,
                &mut m.seed,
            );
            ui.add_space(12.0);

            // Simulation parameters.
            ui.heading("Simulation");
            ui.add_space(4.0);

            let mut ps = m.program_size as u32;
            ui.add(egui::Slider::new(&mut ps, 8..=256).text("Program size"));
            m.program_size = ps as usize;

            let mut sl = m.step_limit as f64;
            ui.add(egui::Slider::new(&mut sl, 64.0..=1_000_000.0).logarithmic(true).text("Step limit"));
            m.step_limit = sl as usize;

            let mut mr = m.mutation_rate;
            ui.add(egui::Slider::new(&mut mr, 0.0..=0.01).logarithmic(true).text("Mutation rate"));
            m.mutation_rate = mr;

            let mut me = m.max_epochs as f64;
            ui.add(egui::Slider::new(&mut me, 100.0..=10_000_000.0).logarithmic(true).text("Max epochs"));
            m.max_epochs = me as usize;

            let mut mi = m.metrics_interval as f64;
            ui.add(egui::Slider::new(&mut mi, 1.0..=10_000.0).logarithmic(true).text("Metrics interval"));
            m.metrics_interval = mi as usize;

            ui.add_space(12.0);

            // Visualization settings.
            ui.heading("Visualization");
            ui.add_space(4.0);

            egui::ComboBox::from_label("Color mode")
                .selected_text(m.color_mode.label())
                .show_ui(ui, |ui| {
                    for mode in ColorMode::ALL {
                        ui.selectable_value(&mut m.color_mode, mode, mode.label());
                    }
                });

            ui.add(egui::Slider::new(&mut m.blur, 0.0..=1.0).text("Blur"));

            ui.add_space(20.0);

            // Start button.
            ui.vertical_centered(|ui| {
                if ui.button("Start Simulation").clicked() {
                    m.last_error = None;
                    let spec = m.current_spec();
                    match spec.build() {
                        Ok(_) => {
                            next_state.set(AppState::Simulating);
                        }
                        Err(e) => {
                            m.last_error = Some(e);
                        }
                    }
                }
            });

            if let Some(ref err) = m.last_error {
                ui.add_space(8.0);
                ui.colored_label(egui::Color32::RED, err);
            }
        });
    });
}

// ─── Simulation lifecycle ────────────────────────────────────────────────────

fn enter_simulation(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    menu: Res<MenuConfig>,
) {
    // Build mesh from spec.
    let spec = menu.current_spec();
    let mut surface_mesh = spec.build().expect("spec was validated in menu");
    surface_mesh.compute_neighbors(menu.neighbor_radius);

    let num_cells = surface_mesh.num_cells();

    // Pre-compute render data.
    let render_positions = build_render_positions(&surface_mesh);
    let render_normals = build_render_normals(&surface_mesh);
    let num_render_vertices = render_positions.len();
    let (center, radius) = surface_mesh.bounding_sphere();

    // Spawn sim thread.
    let config = SoupSurfaceConfig {
        program_size: menu.program_size,
        step_limit: menu.step_limit,
        mutation_rate: menu.mutation_rate,
    };

    let (metrics_rx, snap_rx, cmd_tx) = spawn_sim_thread(
        menu.substrate,
        surface_mesh,
        config,
        menu.seed,
        menu.max_epochs,
        menu.metrics_interval,
        menu.blur,
    );

    // Set initial color mode + blur on the sim thread.
    if menu.color_mode != ColorMode::Hash {
        let _ = cmd_tx.send(SimCommand::SetColorMode(menu.color_mode));
    }

    // Insert sim-only resources.
    commands.insert_resource(SimReceiver(Mutex::new(metrics_rx)));
    commands.insert_resource(SurfaceSnapshotReceiver(Mutex::new(snap_rx)));
    commands.insert_resource(SimCommander(cmd_tx));
    commands.insert_resource(SimulationHistory::default());
    commands.insert_resource(LatestSurfaceSnapshot::default());
    commands.insert_resource(PlaybackState {
        playing: true,
        max_epochs: menu.max_epochs,
    });
    commands.insert_resource(VizSettings {
        color_mode: menu.color_mode,
        blur: menu.blur,
    });
    commands.insert_resource(NumCells(num_cells));
    commands.insert_resource(SimConfig(config));
    commands.insert_resource(menu.to_surface_config_gui());
    commands.insert_resource(PendingMeshRebuild::default());
    commands.insert_resource(SurfaceRenderData {
        positions: render_positions.clone(),
        normals: render_normals.clone(),
        num_render_vertices,
        center,
        radius,
    });

    // Build bevy mesh.
    let mut bevy_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, render_positions);
    bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, render_normals);
    let initial_colors: Vec<[f32; 4]> = vec![[0.5, 0.5, 0.5, 1.0]; num_render_vertices];
    bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, initial_colors);

    let mesh_handle = meshes.add(bevy_mesh);

    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.8,
        metallic: 0.0,
        reflectance: 0.1,
        ..default()
    });

    // Spawn 3D entities with SimEntity marker.
    commands.spawn((
        Mesh3d(mesh_handle.clone()),
        MeshMaterial3d(material),
        Transform::default(),
        SimEntity,
    ));

    commands.insert_resource(SurfaceMeshHandle(mesh_handle));

    // Camera with orbit controls.
    let center_v = Vec3::from_array(center);
    let distance = radius * 2.5;
    let yaw = 0.4_f32;
    let pitch = -0.5_f32;
    let rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
    let camera_pos = center_v + rotation * Vec3::new(0.0, 0.0, distance);

    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(camera_pos).looking_at(center_v, Vec3::Y),
        OrbitCamera { focus: center_v, distance, yaw, pitch },
        SimEntity,
    ));

    // Directional light.
    commands.spawn((
        DirectionalLight {
            illuminance: 12000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.6, 0.4, 0.0)),
        SimEntity,
    ));

    // Ambient light.
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 200.0,
    });
}

fn exit_simulation(
    mut commands: Commands,
    sim_entities: Query<Entity, With<SimEntity>>,
) {
    // Despawn all sim entities.
    for entity in &sim_entities {
        commands.entity(entity).despawn();
    }

    // Remove all sim-only resources.
    // Dropping receivers closes channels → sim thread exits.
    commands.remove_resource::<SimReceiver>();
    commands.remove_resource::<SurfaceSnapshotReceiver>();
    commands.remove_resource::<SimCommander>();
    commands.remove_resource::<SimulationHistory>();
    commands.remove_resource::<LatestSurfaceSnapshot>();
    commands.remove_resource::<PlaybackState>();
    commands.remove_resource::<VizSettings>();
    commands.remove_resource::<NumCells>();
    commands.remove_resource::<SimConfig>();
    commands.remove_resource::<SurfaceConfigGui>();
    commands.remove_resource::<SurfaceMeshHandle>();
    commands.remove_resource::<PendingMeshRebuild>();
    commands.remove_resource::<SurfaceRenderData>();
    commands.remove_resource::<AmbientLight>();
}

// ─── Simulation update systems ───────────────────────────────────────────────

fn drain_metrics(receiver: Res<SimReceiver>, mut history: ResMut<SimulationHistory>) {
    let rx = receiver.0.lock().unwrap();
    while let Ok(metrics) = rx.try_recv() {
        if history.awaiting_reset {
            if metrics.epoch == 0 {
                history.awaiting_reset = false;
            } else {
                continue;
            }
        }
        history.entries.push(metrics);
    }
}

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

    let expected_len = num_cells.0 * 4;
    if snap.colors.len() != expected_len {
        latest.dirty = false;
        return;
    }

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

fn orbit_camera_system(
    mut contexts: EguiContexts,
    mut query: Query<(&mut OrbitCamera, &mut Transform)>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut scroll: EventReader<MouseWheel>,
    windows: Query<&Window>,
) {
    if windows.is_empty() {
        return;
    }
    let egui_wants_pointer = contexts.ctx_mut().is_pointer_over_area();

    let motion: Vec<_> = mouse_motion.read().cloned().collect();
    let scrolls: Vec<_> = scroll.read().cloned().collect();

    if egui_wants_pointer {
        return;
    }

    let Ok((mut orbit, mut transform)) = query.get_single_mut() else {
        return;
    };

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

    if mouse_buttons.pressed(MouseButton::Right) {
        for ev in &motion {
            let right = transform.right().as_vec3();
            let up = transform.up().as_vec3();
            let pan_speed = orbit.distance * 0.001;
            orbit.focus += (-right * ev.delta.x + up * ev.delta.y) * pan_speed;
        }
    }

    for ev in &scrolls {
        orbit.distance *= 1.0 - ev.y * 0.03;
        orbit.distance = orbit.distance.max(0.05);
    }

    let rotation = Quat::from_euler(EulerRot::YXZ, orbit.yaw, orbit.pitch, 0.0);
    transform.translation = orbit.focus + rotation * Vec3::new(0.0, 0.0, orbit.distance);
    transform.look_at(orbit.focus, Vec3::Y);
}

fn render_ui_surface(
    mut contexts: EguiContexts,
    mut history: ResMut<SimulationHistory>,
    mut playback: ResMut<PlaybackState>,
    mut viz: ResMut<VizSettings>,
    commander: Res<SimCommander>,
    mut gui: ResMut<SurfaceConfigGui>,
    mut num_cells: ResMut<NumCells>,
    mut render_data: ResMut<SurfaceRenderData>,
    mut latest_snap: ResMut<LatestSurfaceSnapshot>,
    mut pending_rebuild: ResMut<PendingMeshRebuild>,
    sim_config: Res<SimConfig>,
    mut next_state: ResMut<NextState<AppState>>,
    mut menu: ResMut<MenuConfig>,
    windows: Query<&Window>,
) {
    if windows.is_empty() {
        return;
    }
    let ctx = contexts.ctx_mut();

    egui::SidePanel::right("metrics_panel").min_width(350.0).show(ctx, |ui| {
        // Back to Menu button at the top.
        if ui.button("Back to Menu").clicked() {
            // Copy current settings back into MenuConfig.
            menu.color_mode = viz.color_mode;
            menu.blur = viz.blur;
            menu.selected_type = gui.selected_type;
            menu.sphere_subdivisions = gui.sphere_subdivisions;
            menu.torus_major = gui.torus_major;
            menu.torus_minor = gui.torus_minor;
            menu.grid_width = gui.grid_width;
            menu.grid_height = gui.grid_height;
            menu.tunnel_spheres = gui.tunnel_spheres;
            menu.tunnel_segments = gui.tunnel_segments;
            menu.seed = gui.seed;
            menu.neighbor_radius = gui.neighbor_radius;
            next_state.set(AppState::Menu);
        }
        ui.separator();

        render_controls_section(ui, &history, &mut playback, &commander);
        ui.separator();
        render_surface_config(
            ui, &mut gui, &mut num_cells, &mut render_data,
            &mut history, &mut latest_snap, &commander,
            &mut pending_rebuild, &sim_config.0, &mut playback,
        );
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

fn apply_mesh_rebuild(
    mut meshes: ResMut<Assets<Mesh>>,
    handle: Option<Res<SurfaceMeshHandle>>,
    render_data: Res<SurfaceRenderData>,
    mut pending: ResMut<PendingMeshRebuild>,
    mut query: Query<(&mut OrbitCamera, &mut Transform)>,
) {
    if !pending.active {
        return;
    }
    pending.active = false;

    let Some(handle) = handle else { return };
    let Some(mesh) = meshes.get_mut(&handle.0) else { return };

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, render_data.positions.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, render_data.normals.clone());

    let initial_colors: Vec<[f32; 4]> = vec![[0.5, 0.5, 0.5, 1.0]; render_data.num_render_vertices];
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, initial_colors);

    if let Ok((mut orbit, mut transform)) = query.get_single_mut() {
        let center = Vec3::from_array(render_data.center);
        let distance = render_data.radius * 2.5;
        orbit.focus = center;
        orbit.distance = distance;
        let rotation = Quat::from_euler(EulerRot::YXZ, orbit.yaw, orbit.pitch, 0.0);
        transform.translation = center + rotation * Vec3::new(0.0, 0.0, distance);
        transform.look_at(center, Vec3::Y);
    }
}

// ─── Shared UI helpers ───────────────────────────────────────────────────────

/// Render surface type combo, per-type parameter sliders, seed field, and face count.
/// Shared between the menu UI and the in-sim sidebar.
fn render_surface_params(
    ui: &mut egui::Ui,
    selected_type: &mut usize,
    sphere_subdivisions: &mut usize,
    torus_major: &mut usize,
    torus_minor: &mut usize,
    grid_width: &mut usize,
    grid_height: &mut usize,
    tunnel_spheres: &mut usize,
    tunnel_segments: &mut usize,
    seed: &mut u64,
) {
    let type_labels = ["Sphere", "Torus", "Flat Grid", "Hamster Tunnel"];
    egui::ComboBox::from_label("Type")
        .selected_text(type_labels[*selected_type])
        .show_ui(ui, |ui| {
            for (i, label) in type_labels.iter().enumerate() {
                ui.selectable_value(selected_type, i, *label);
            }
        });

    ui.add_space(4.0);

    match *selected_type {
        0 => {
            let mut sub = *sphere_subdivisions as u32;
            ui.add(egui::Slider::new(&mut sub, 0..=6).text("Subdivisions"));
            *sphere_subdivisions = sub as usize;
            let face_count = 20 * 4usize.pow(sub);
            ui.label(format!("Faces: {face_count}"));
        }
        1 => {
            let mut major = *torus_major as u32;
            let mut minor = *torus_minor as u32;
            ui.add(egui::Slider::new(&mut major, 3..=128).text("Major segments"));
            ui.add(egui::Slider::new(&mut minor, 3..=64).text("Minor segments"));
            *torus_major = major as usize;
            *torus_minor = minor as usize;
            ui.label(format!("Faces: {}", 2 * *torus_major * *torus_minor));
        }
        2 => {
            let mut w = *grid_width as u32;
            let mut h = *grid_height as u32;
            ui.add(egui::Slider::new(&mut w, 1..=256).text("Width"));
            ui.add(egui::Slider::new(&mut h, 1..=256).text("Height"));
            *grid_width = w as usize;
            *grid_height = h as usize;
            ui.label(format!("Faces: {}", 2 * *grid_width * *grid_height));
        }
        3 => {
            let mut spheres = *tunnel_spheres as u32;
            let mut segs = *tunnel_segments as u32;
            ui.add(egui::Slider::new(&mut spheres, 3..=50).text("Spheres"));
            ui.add(egui::Slider::new(&mut segs, 3..=48).text("Segments"));
            *tunnel_spheres = spheres as usize;
            *tunnel_segments = segs as usize;
            let rings_per_seg = 16usize;
            let total_rings = *tunnel_spheres * rings_per_seg;
            ui.label(format!("Faces: {}", 2 * *tunnel_segments * total_rings));
        }
        _ => {}
    }

    ui.add_space(4.0);

    let mut seed_str = seed.to_string();
    ui.horizontal(|ui| {
        ui.label("Seed:");
        if ui.text_edit_singleline(&mut seed_str).changed() {
            if let Ok(s) = seed_str.parse::<u64>() {
                *seed = s;
            }
        }
    });
}

fn render_surface_config(
    ui: &mut egui::Ui,
    gui: &mut SurfaceConfigGui,
    num_cells: &mut NumCells,
    render_data: &mut SurfaceRenderData,
    history: &mut SimulationHistory,
    latest_snap: &mut LatestSurfaceSnapshot,
    commander: &SimCommander,
    pending_rebuild: &mut PendingMeshRebuild,
    config: &SoupSurfaceConfig,
    playback: &mut PlaybackState,
) {
    ui.heading("Surface");
    ui.add_space(4.0);

    render_surface_params(
        ui,
        &mut gui.selected_type,
        &mut gui.sphere_subdivisions,
        &mut gui.torus_major,
        &mut gui.torus_minor,
        &mut gui.grid_width,
        &mut gui.grid_height,
        &mut gui.tunnel_spheres,
        &mut gui.tunnel_segments,
        &mut gui.seed,
    );

    ui.add_space(4.0);

    if ui.button("Generate").clicked() {
        gui.last_error = None;
        let spec = gui.current_spec();
        match spec.build() {
            Ok(mut mesh) => {
                mesh.compute_neighbors(gui.neighbor_radius);
                let num = mesh.num_cells();

                let positions = build_render_positions(&mesh);
                let normals = build_render_normals(&mesh);
                let (center, radius) = mesh.bounding_sphere();

                render_data.positions = positions;
                render_data.normals = normals;
                render_data.num_render_vertices = num * 3;
                render_data.center = center;
                render_data.radius = radius;
                num_cells.0 = num;

                history.entries.clear();
                history.awaiting_reset = true;
                latest_snap.snapshot = None;
                latest_snap.dirty = false;

                let _ = commander.0.send(SimCommand::ResetSurface {
                    mesh,
                    config: *config,
                    seed: gui.seed,
                });
                let _ = commander.0.send(SimCommand::Play);
                playback.playing = true;

                pending_rebuild.active = true;
            }
            Err(e) => {
                gui.last_error = Some(e);
            }
        }
    }

    if let Some(ref err) = gui.last_error {
        ui.colored_label(egui::Color32::RED, err);
    }

    ui.add_space(8.0);
}

fn render_viz_settings(
    ui: &mut egui::Ui,
    viz: &mut VizSettings,
    commander: &SimCommander,
) {
    ui.heading("Visualization");
    ui.add_space(4.0);

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

    let prev_blur = viz.blur;
    ui.add(egui::Slider::new(&mut viz.blur, 0.0..=1.0).text("Blur"));
    if (viz.blur - prev_blur).abs() > f32::EPSILON {
        let _ = commander.0.send(SimCommand::SetBlur(viz.blur));
    }

    ui.add_space(8.0);
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

// ─── Plot helpers ────────────────────────────────────────────────────────────

fn decimated_plot_points(
    entries: &[EpochMetrics],
    extract: impl Fn(&EpochMetrics) -> [f64; 2],
) -> PlotPoints<'_> {
    let n = entries.len();
    if n <= MAX_PLOT_POINTS {
        return entries.iter().map(extract).collect();
    }
    let mut points = Vec::with_capacity(MAX_PLOT_POINTS);
    points.push(extract(&entries[0]));
    let interior_count = MAX_PLOT_POINTS - 2;
    let stride = (n - 2) as f64 / interior_count as f64;
    for i in 0..interior_count {
        let idx = 1 + (i as f64 * stride) as usize;
        points.push(extract(&entries[idx]));
    }
    points.push(extract(&entries[n - 1]));
    PlotPoints::new(points)
}

fn render_time_series_plots_compact(ui: &mut egui::Ui, entries: &[EpochMetrics], plot_height: f32) {
    ui.label("High-Order Entropy");
    let hoe_points = decimated_plot_points(entries, |e| [e.epoch as f64, e.hoe]);
    Plot::new("hoe_plot")
        .height(plot_height)
        .show(ui, |plot_ui: &mut egui_plot::PlotUi| {
            plot_ui.line(Line::new(hoe_points).name("HOE"));
        });

    ui.label("Unique Programs");
    let unique_points = decimated_plot_points(entries, |e| [e.epoch as f64, e.unique_count as f64]);
    Plot::new("unique_plot")
        .height(plot_height)
        .show(ui, |plot_ui: &mut egui_plot::PlotUi| {
            plot_ui.line(Line::new(unique_points).name("Unique"));
        });

    ui.label("Zero Byte Count");
    let zero_points = decimated_plot_points(entries, |e| [e.epoch as f64, e.zero_count as f64]);
    Plot::new("zero_plot")
        .height(plot_height)
        .show(ui, |plot_ui: &mut egui_plot::PlotUi| {
            plot_ui.line(Line::new(zero_points).name("Zeros"));
        });
}
