use std::sync::{mpsc, Mutex};
use std::thread;

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, egui};
use egui_plot::{Bar, BarChart, Line, Plot, PlotPoints};

use crate::metrics::{byte_frequency_histogram, high_order_entropy, unique_program_count, zero_byte_count};
use crate::soup::{Soup, SoupConfig};
use crate::soup2d::{Soup2d, Soup2dConfig};
use crate::substrate::Substrate;

/// Metrics snapshot sent from sim thread to render thread.
#[derive(Clone)]
pub struct EpochMetrics {
    pub epoch: usize,
    pub hoe: f64,
    pub unique_count: usize,
    pub zero_count: usize,
    pub byte_histogram: [usize; 256],
}

/// Compact color representation of the 2D grid.
#[derive(Clone)]
pub struct GridSnapshot {
    pub width: usize,
    pub height: usize,
    /// RGBA bytes, length = width * height * 4.
    pub pixels: Vec<u8>,
}

/// Commands sent from render thread to sim thread.
pub enum SimCommand {
    Play,
    Pause,
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

/// Bevy resource wrapping the grid snapshot channel receiver (2D only).
#[derive(Resource)]
struct GridReceiver(Mutex<mpsc::Receiver<GridSnapshot>>);

/// Bevy resource storing the latest grid snapshot (2D only).
#[derive(Resource, Default)]
struct LatestGridSnapshot {
    snapshot: Option<GridSnapshot>,
    dirty: bool,
}

/// Bevy resource storing the latest grid texture handle.
#[derive(Resource, Default)]
struct GridTextureState {
    texture_id: Option<egui::TextureId>,
    width: usize,
    height: usize,
}

/// Hash a program's bytes to an RGB color.
fn program_to_color(program: &[u8]) -> [u8; 3] {
    // Use a simple hash: FNV-1a on the program bytes, then extract RGB.
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

/// Build a GridSnapshot from the 2D soup's programs.
fn build_grid_snapshot(programs: &[Vec<u8>], width: usize, height: usize) -> GridSnapshot {
    let mut pixels = Vec::with_capacity(width * height * 4);
    fill_grid_pixels(programs, &mut pixels);
    GridSnapshot { width, height, pixels }
}

/// Fill a reusable pixel buffer from program data (avoids re-allocation).
fn fill_grid_pixels(programs: &[Vec<u8>], pixels: &mut Vec<u8>) {
    pixels.clear();
    for prog in programs {
        let [r, g, b] = program_to_color(prog);
        pixels.push(r);
        pixels.push(g);
        pixels.push(b);
        pixels.push(255);
    }
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

// ─── 2D visualization ────────────────────────────────────────────────────────

/// Apply a spatial box blur to an RGBA pixel buffer.
///
/// Each pixel is blended with the average of its 4 direct neighbors (von Neumann):
/// `out = (1 - alpha) * center + alpha * avg(neighbors)`
///
/// Alpha channel is left at 255. Edge pixels use fewer neighbors.
fn blur_grid_pixels(pixels: &mut Vec<u8>, scratch: &mut Vec<u8>, width: usize, height: usize, alpha: f32) {
    if alpha <= 0.0 {
        return;
    }
    let alpha = alpha.min(1.0);
    let one_minus_alpha = 1.0 - alpha;
    let len = width * height * 4;
    scratch.resize(len, 0);

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 4;

            // Accumulate neighbor values for R, G, B.
            let mut sum_r = 0u32;
            let mut sum_g = 0u32;
            let mut sum_b = 0u32;
            let mut count = 0u32;

            if x > 0 {
                let ni = idx - 4;
                sum_r += pixels[ni] as u32;
                sum_g += pixels[ni + 1] as u32;
                sum_b += pixels[ni + 2] as u32;
                count += 1;
            }
            if x + 1 < width {
                let ni = idx + 4;
                sum_r += pixels[ni] as u32;
                sum_g += pixels[ni + 1] as u32;
                sum_b += pixels[ni + 2] as u32;
                count += 1;
            }
            if y > 0 {
                let ni = idx - width * 4;
                sum_r += pixels[ni] as u32;
                sum_g += pixels[ni + 1] as u32;
                sum_b += pixels[ni + 2] as u32;
                count += 1;
            }
            if y + 1 < height {
                let ni = idx + width * 4;
                sum_r += pixels[ni] as u32;
                sum_g += pixels[ni + 1] as u32;
                sum_b += pixels[ni + 2] as u32;
                count += 1;
            }

            let center_r = pixels[idx] as f32;
            let center_g = pixels[idx + 1] as f32;
            let center_b = pixels[idx + 2] as f32;

            let avg_r = sum_r as f32 / count as f32;
            let avg_g = sum_g as f32 / count as f32;
            let avg_b = sum_b as f32 / count as f32;

            scratch[idx] = (one_minus_alpha * center_r + alpha * avg_r) as u8;
            scratch[idx + 1] = (one_minus_alpha * center_g + alpha * avg_g) as u8;
            scratch[idx + 2] = (one_minus_alpha * center_b + alpha * avg_b) as u8;
            scratch[idx + 3] = 255;
        }
    }

    std::mem::swap(pixels, scratch);
}

/// Launch the live visualization for a 2D spatial soup.
pub fn run_viz_2d<S: Substrate + Send + Sync + 'static>(
    config: Soup2dConfig,
    seed: u64,
    max_epochs: usize,
    metrics_interval: usize,
    blur: f32,
) {
    let (metrics_tx, metrics_rx) = mpsc::channel::<EpochMetrics>();
    let (grid_tx, grid_rx) = mpsc::channel::<GridSnapshot>();
    let (cmd_tx, cmd_rx) = mpsc::channel::<SimCommand>();

    thread::spawn(move || {
        sim_thread_loop_2d::<S>(config, seed, max_epochs, metrics_interval, metrics_tx, grid_tx, cmd_rx, blur);
    });

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Computational Life (2D)".into(),
                resolution: (1280., 800.).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin)
        .insert_resource(SimReceiver(Mutex::new(metrics_rx)))
        .insert_resource(GridReceiver(Mutex::new(grid_rx)))
        .insert_resource(SimCommander(cmd_tx))
        .insert_resource(SimulationHistory::default())
        .insert_resource(LatestGridSnapshot::default())
        .insert_resource(PlaybackState {
            playing: true,
            max_epochs,
        })
        .insert_resource(GridTextureState::default())
        .add_systems(Update, drain_metrics)
        .add_systems(Update, drain_grid)
        .add_systems(Update, render_ui_2d.after(drain_metrics).after(drain_grid))
        .run();
}

/// Simulation thread for 2D soup.
///
/// Sends grid snapshots throttled to ~60fps to keep visuals responsive
/// without wasting time on frames the renderer can't display.
/// Expensive metrics (HOE etc.) are sent only at `metrics_interval`.
fn sim_thread_loop_2d<S: Substrate + Sync>(
    config: Soup2dConfig,
    seed: u64,
    max_epochs: usize,
    metrics_interval: usize,
    metrics_tx: mpsc::Sender<EpochMetrics>,
    grid_tx: mpsc::Sender<GridSnapshot>,
    cmd_rx: mpsc::Receiver<SimCommand>,
    blur: f32,
) {
    let w = config.width;
    let h = config.height;
    let mut soup = Soup2d::new(config, seed);
    let mut paused = false;
    let mut epoch = 0usize;

    // Reusable pixel buffer to avoid per-frame allocation.
    let mut pixel_buf: Vec<u8> = Vec::with_capacity(w * h * 4);

    // Reusable scratch buffer for blur (avoids per-frame allocation).
    let mut blur_scratch: Vec<u8> = Vec::new();

    // Reusable population buffer to avoid per-metrics allocation.
    let mut pop_buf: Vec<u8> = Vec::new();

    // Send initial state.
    let _ = metrics_tx.send(compute_metrics_2d(&soup, 0, &mut pop_buf));
    let _ = grid_tx.send(build_grid_snapshot(&soup.programs, w, h));

    // Throttle grid snapshots to ~60fps.
    let grid_interval = std::time::Duration::from_millis(16);
    let mut last_grid_send = std::time::Instant::now();

    loop {
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                SimCommand::Play => paused = false,
                SimCommand::Pause => paused = true,
            }
        }

        if paused || epoch >= max_epochs {
            thread::sleep(std::time::Duration::from_millis(10));
            continue;
        }

        soup.run_epoch::<S>();
        soup.mutate();
        epoch += 1;

        // Send grid snapshot at most ~60fps.
        let now = std::time::Instant::now();
        if now.duration_since(last_grid_send) >= grid_interval || epoch == max_epochs {
            fill_grid_pixels(&soup.programs, &mut pixel_buf);
            blur_grid_pixels(&mut pixel_buf, &mut blur_scratch, w, h, blur);
            let snap = GridSnapshot {
                width: w,
                height: h,
                pixels: pixel_buf.clone(),
            };
            if grid_tx.send(snap).is_err() {
                break;
            }
            last_grid_send = now;
        }

        // Send expensive metrics less frequently.
        if epoch % metrics_interval == 0 || epoch == max_epochs {
            if metrics_tx.send(compute_metrics_2d(&soup, epoch, &mut pop_buf)).is_err() {
                break;
            }
        }
    }
}

fn compute_metrics_2d(soup: &Soup2d, epoch: usize, pop_buf: &mut Vec<u8>) -> EpochMetrics {
    soup.population_bytes_into(pop_buf);
    EpochMetrics {
        epoch,
        hoe: high_order_entropy(pop_buf),
        unique_count: unique_program_count(&soup.programs),
        zero_count: zero_byte_count(&soup.programs),
        byte_histogram: byte_frequency_histogram(&soup.programs),
    }
}

// ─── Shared systems ──────────────────────────────────────────────────────────

/// System: drain metrics from the channel each frame.
fn drain_metrics(receiver: Res<SimReceiver>, mut history: ResMut<SimulationHistory>) {
    let rx = receiver.0.lock().unwrap();
    while let Ok(metrics) = rx.try_recv() {
        history.entries.push(metrics);
    }
}

/// System: drain grid snapshots from the grid channel, keeping only the latest.
fn drain_grid(receiver: Res<GridReceiver>, mut latest: ResMut<LatestGridSnapshot>) {
    let rx = receiver.0.lock().unwrap();
    while let Ok(snapshot) = rx.try_recv() {
        latest.snapshot = Some(snapshot);
        latest.dirty = true;
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

/// System: render egui UI with grid and plots (2D view).
fn render_ui_2d(
    mut contexts: EguiContexts,
    history: Res<SimulationHistory>,
    mut playback: ResMut<PlaybackState>,
    commander: Res<SimCommander>,
    mut grid_tex: ResMut<GridTextureState>,
    mut latest_grid: ResMut<LatestGridSnapshot>,
) {
    // Only update the texture when new grid data has arrived.
    if latest_grid.dirty {
        if let Some(ref snap) = latest_grid.snapshot {
            let size = [snap.width, snap.height];
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &snap.pixels);
            let texture_opts = egui::TextureOptions {
                magnification: egui::TextureFilter::Nearest,
                minification: egui::TextureFilter::Nearest,
                ..default()
            };

            let ctx = contexts.ctx_mut();
            if let Some(existing_id) = grid_tex.texture_id {
                ctx.tex_manager().write().free(existing_id);
            }
            let id = ctx.tex_manager().write().alloc(
                "grid".into(),
                egui::ImageData::Color(color_image.into()),
                texture_opts,
            );
            grid_tex.texture_id = Some(id);
            grid_tex.width = snap.width;
            grid_tex.height = snap.height;
        }
        latest_grid.dirty = false;
    }

    let ctx = contexts.ctx_mut();

    // Right panel with metrics plots.
    egui::SidePanel::right("metrics_panel").min_width(350.0).show(ctx, |ui| {
        render_controls_section(ui, &history, &mut playback, &commander);
        ui.separator();

        let entries = &history.entries;
        if entries.is_empty() {
            return;
        }
        let available_height = ui.available_height();
        let plot_height = (available_height - 40.0) / 3.0;
        render_time_series_plots_compact(ui, entries, plot_height);
    });

    // Central panel with grid image.
    egui::CentralPanel::default().show(ctx, |ui| {
        if let Some(tex_id) = grid_tex.texture_id {
            let available = ui.available_size();
            let grid_w = grid_tex.width as f32;
            let grid_h = grid_tex.height as f32;
            // Scale to fit while maintaining aspect ratio.
            let scale = (available.x / grid_w).min(available.y / grid_h);
            let display_size = egui::Vec2::new(grid_w * scale, grid_h * scale);
            ui.centered_and_justified(|ui| {
                ui.image(egui::load::SizedTexture::new(tex_id, display_size));
            });
        } else {
            ui.label("Waiting for simulation data...");
        }
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
    // 1. HOE
    ui.label("High-Order Entropy");
    let hoe_points: PlotPoints = entries.iter().map(|e| [e.epoch as f64, e.hoe]).collect();
    Plot::new("hoe_plot")
        .height(plot_height)
        .show(ui, |plot_ui: &mut egui_plot::PlotUi| {
            plot_ui.line(Line::new(hoe_points).name("HOE"));
        });

    // 2. Unique programs
    ui.label("Unique Programs");
    let unique_points: PlotPoints = entries.iter().map(|e| [e.epoch as f64, e.unique_count as f64]).collect();
    Plot::new("unique_plot")
        .height(plot_height)
        .show(ui, |plot_ui: &mut egui_plot::PlotUi| {
            plot_ui.line(Line::new(unique_points).name("Unique"));
        });

    // 3. Zero count
    ui.label("Zero Byte Count");
    let zero_points: PlotPoints = entries.iter().map(|e| [e.epoch as f64, e.zero_count as f64]).collect();
    Plot::new("zero_plot")
        .height(plot_height)
        .show(ui, |plot_ui: &mut egui_plot::PlotUi| {
            plot_ui.line(Line::new(zero_points).name("Zeros"));
        });

    // 4. Byte histogram
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
    // Compact version for 2D view side panel (3 plots, no histogram).
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
