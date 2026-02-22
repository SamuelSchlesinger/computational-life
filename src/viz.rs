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
    /// Optional grid snapshot for 2D visualization (RGBA color per cell).
    pub grid_snapshot: Option<GridSnapshot>,
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
    for prog in programs {
        let [r, g, b] = program_to_color(prog);
        pixels.push(r);
        pixels.push(g);
        pixels.push(b);
        pixels.push(255); // alpha
    }
    GridSnapshot { width, height, pixels }
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

    let _ = tx.send(compute_metrics(&soup, 0));

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
            let metrics = compute_metrics(&soup, epoch);
            if tx.send(metrics).is_err() {
                break;
            }
        }
    }
}

fn compute_metrics(soup: &Soup, epoch: usize) -> EpochMetrics {
    let pop_bytes = soup.population_bytes();
    EpochMetrics {
        epoch,
        hoe: high_order_entropy(&pop_bytes),
        unique_count: unique_program_count(&soup.programs),
        zero_count: zero_byte_count(&soup.programs),
        byte_histogram: byte_frequency_histogram(&soup.programs),
        grid_snapshot: None,
    }
}

// ─── 2D visualization ────────────────────────────────────────────────────────

/// Launch the live visualization for a 2D spatial soup.
pub fn run_viz_2d<S: Substrate + Send + 'static>(
    config: Soup2dConfig,
    seed: u64,
    max_epochs: usize,
    metrics_interval: usize,
) {
    let (metrics_tx, metrics_rx) = mpsc::channel::<EpochMetrics>();
    let (cmd_tx, cmd_rx) = mpsc::channel::<SimCommand>();

    thread::spawn(move || {
        sim_thread_loop_2d::<S>(config, seed, max_epochs, metrics_interval, metrics_tx, cmd_rx);
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
        .insert_resource(SimCommander(cmd_tx))
        .insert_resource(SimulationHistory::default())
        .insert_resource(PlaybackState {
            playing: true,
            max_epochs,
        })
        .insert_resource(GridTextureState::default())
        .add_systems(Update, drain_metrics)
        .add_systems(Update, render_ui_2d.after(drain_metrics))
        .run();
}

/// Simulation thread for 2D soup.
fn sim_thread_loop_2d<S: Substrate>(
    config: Soup2dConfig,
    seed: u64,
    max_epochs: usize,
    metrics_interval: usize,
    tx: mpsc::Sender<EpochMetrics>,
    cmd_rx: mpsc::Receiver<SimCommand>,
) {
    let w = config.width;
    let h = config.height;
    let mut soup = Soup2d::new(config, seed);
    let mut paused = false;
    let mut epoch = 0usize;

    let _ = tx.send(compute_metrics_2d(&soup, 0, w, h));

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
            let metrics = compute_metrics_2d(&soup, epoch, w, h);
            if tx.send(metrics).is_err() {
                break;
            }
        }
    }
}

fn compute_metrics_2d(soup: &Soup2d, epoch: usize, width: usize, height: usize) -> EpochMetrics {
    let pop_bytes = soup.population_bytes();
    EpochMetrics {
        epoch,
        hoe: high_order_entropy(&pop_bytes),
        unique_count: unique_program_count(&soup.programs),
        zero_count: zero_byte_count(&soup.programs),
        byte_histogram: byte_frequency_histogram(&soup.programs),
        grid_snapshot: Some(build_grid_snapshot(&soup.programs, width, height)),
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
) {
    // Update the grid texture if we have a new snapshot.
    if let Some(latest) = history.entries.last() {
        if let Some(ref snap) = latest.grid_snapshot {
            // Create or update the egui texture.
            let size = [snap.width, snap.height];
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &snap.pixels);
            let texture_opts = egui::TextureOptions {
                magnification: egui::TextureFilter::Nearest,
                minification: egui::TextureFilter::Nearest,
                ..default()
            };

            let ctx = contexts.ctx_mut();
            if let Some(existing_id) = grid_tex.texture_id {
                // Free old texture and allocate a new one.
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
