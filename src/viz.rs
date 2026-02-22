use std::sync::{mpsc, Mutex};
use std::thread;

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, egui};
use egui_plot::{Bar, BarChart, Line, Plot, PlotPoints};

use crate::metrics::{byte_frequency_histogram, high_order_entropy, unique_program_count, zero_byte_count};
use crate::soup::{Soup, SoupConfig};
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

/// Launch the live visualization.
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

/// Simulation thread: runs epochs, computes metrics, sends them over channel.
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

    // Send epoch 0 metrics.
    let _ = tx.send(compute_metrics(&soup, 0));

    loop {
        // Check for commands (non-blocking).
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
                break; // Window closed
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
    }
}

/// System: drain metrics from the channel each frame.
fn drain_metrics(receiver: Res<SimReceiver>, mut history: ResMut<SimulationHistory>) {
    let rx = receiver.0.lock().unwrap();
    while let Ok(metrics) = rx.try_recv() {
        history.entries.push(metrics);
    }
}

/// System: render egui UI with plots and controls.
fn render_ui(
    mut contexts: EguiContexts,
    history: Res<SimulationHistory>,
    mut playback: ResMut<PlaybackState>,
    commander: Res<SimCommander>,
) {
    let ctx = contexts.ctx_mut();

    egui::SidePanel::left("controls").min_width(220.0).show(ctx, |ui| {
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
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        let entries = &history.entries;
        if entries.is_empty() {
            ui.label("Waiting for simulation data...");
            return;
        }

        let available_height = ui.available_height();
        let plot_height = (available_height - 60.0) / 4.0;

        // 1. HOE over epochs
        ui.label("High-Order Entropy");
        let hoe_points: PlotPoints = entries
            .iter()
            .map(|e| [e.epoch as f64, e.hoe])
            .collect();
        Plot::new("hoe_plot")
            .height(plot_height)
            .show(ui, |plot_ui: &mut egui_plot::PlotUi| {
                plot_ui.line(Line::new(hoe_points).name("HOE"));
            });

        // 2. Unique programs over epochs
        ui.label("Unique Programs");
        let unique_points: PlotPoints = entries
            .iter()
            .map(|e| [e.epoch as f64, e.unique_count as f64])
            .collect();
        Plot::new("unique_plot")
            .height(plot_height)
            .show(ui, |plot_ui: &mut egui_plot::PlotUi| {
                plot_ui.line(Line::new(unique_points).name("Unique"));
            });

        // 3. Zero count over epochs
        ui.label("Zero Byte Count");
        let zero_points: PlotPoints = entries
            .iter()
            .map(|e| [e.epoch as f64, e.zero_count as f64])
            .collect();
        Plot::new("zero_plot")
            .height(plot_height)
            .show(ui, |plot_ui: &mut egui_plot::PlotUi| {
                plot_ui.line(Line::new(zero_points).name("Zeros"));
            });

        // 4. Byte frequency histogram (latest snapshot)
        ui.label("Byte Frequency Distribution");
        if let Some(latest) = entries.last() {
            let bars: Vec<Bar> = latest
                .byte_histogram
                .iter()
                .enumerate()
                .map(|(i, &count)| Bar::new(i as f64, count as f64))
                .collect();
            Plot::new("byte_hist")
                .height(plot_height)
                .show(ui, |plot_ui: &mut egui_plot::PlotUi| {
                    plot_ui.bar_chart(BarChart::new(bars).name("Frequency"));
                });
        }
    });
}
