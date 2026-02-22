use clap::Parser;
use complife::bff::Bff;
use complife::forth::Forth;
use complife::metrics::high_order_entropy;
use complife::soup::{Soup, SoupConfig};
use complife::soup2d::{Soup2d, Soup2dConfig};
use complife::substrate::Substrate;

#[derive(Parser)]
#[command(name = "complife", about = "Computational Life: primordial soup simulation")]
struct Cli {
    /// Random seed for reproducibility.
    #[arg(long)]
    seed: u64,

    /// Number of epochs to run.
    #[arg(long)]
    epochs: usize,

    /// Number of programs in the population.
    #[arg(long, default_value_t = 1 << 17)]
    population_size: usize,

    /// Bytes per program.
    #[arg(long, default_value_t = 64)]
    program_size: usize,

    /// Max steps per program execution.
    #[arg(long, default_value_t = 1 << 13)]
    step_limit: usize,

    /// Per-byte mutation probability per epoch (0 to disable).
    #[arg(long, default_value_t = 0.00024)]
    mutation_rate: f64,

    /// Which instruction set to use (bff, forth).
    #[arg(long, default_value = "bff")]
    substrate: String,

    /// Compute and output metrics every N epochs.
    #[arg(long, default_value_t = 1)]
    metrics_interval: usize,

    /// Run in benchmark mode: suppress CSV, print throughput stats.
    #[arg(long)]
    benchmark: bool,

    /// Enable 2D spatial simulation on a WxH grid (e.g. 240x135).
    #[arg(long)]
    grid: Option<String>,

    /// Launch live visualization window (requires --features viz).
    #[cfg(feature = "viz")]
    #[arg(long)]
    live: bool,
}

/// Parse a "WxH" grid specification string.
fn parse_grid(s: &str) -> Result<(usize, usize), String> {
    let parts: Vec<&str> = s.split('x').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid grid format '{s}', expected WxH (e.g. 240x135)"));
    }
    let w = parts[0].parse::<usize>().map_err(|e| format!("Invalid grid width: {e}"))?;
    let h = parts[1].parse::<usize>().map_err(|e| format!("Invalid grid height: {e}"))?;
    if w == 0 || h == 0 {
        return Err("Grid dimensions must be positive".to_string());
    }
    Ok((w, h))
}

fn main() {
    let cli = Cli::parse();

    // Dispatch based on substrate and mode.
    match cli.substrate.as_str() {
        "bff" => dispatch::<Bff>(&cli),
        "forth" => dispatch::<Forth>(&cli),
        other => {
            eprintln!("Unknown substrate: {other}. Available: bff, forth");
            std::process::exit(1);
        }
    }
}

fn dispatch<S: Substrate + Send + 'static>(cli: &Cli) {
    if let Some(ref grid_str) = cli.grid {
        let (w, h) = match parse_grid(grid_str) {
            Ok(dims) => dims,
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        };
        let config = Soup2dConfig {
            width: w,
            height: h,
            program_size: cli.program_size,
            step_limit: cli.step_limit,
            mutation_rate: cli.mutation_rate,
        };

        #[cfg(feature = "viz")]
        if cli.live {
            complife::viz::run_viz_2d::<S>(config, cli.seed, cli.epochs, cli.metrics_interval);
            return;
        }

        if cli.benchmark {
            run_benchmark_2d::<S>(config, cli.seed, cli.epochs);
        } else {
            run_simulation_2d::<S>(config, cli.seed, cli.epochs, cli.metrics_interval);
        }
    } else {
        let config = SoupConfig {
            population_size: cli.population_size,
            program_size: cli.program_size,
            step_limit: cli.step_limit,
            mutation_rate: cli.mutation_rate,
        };

        #[cfg(feature = "viz")]
        if cli.live {
            complife::viz::run_viz::<S>(config, cli.seed, cli.epochs, cli.metrics_interval);
            return;
        }

        if cli.benchmark {
            run_benchmark::<S>(config, cli.seed, cli.epochs);
        } else {
            run_simulation::<S>(config, cli.seed, cli.epochs, cli.metrics_interval);
        }
    }
}

fn run_simulation<S: Substrate>(
    config: SoupConfig,
    seed: u64,
    epochs: usize,
    metrics_interval: usize,
) {
    let mut soup = Soup::new(config, seed);

    println!("epoch,hoe");
    let hoe = high_order_entropy(&soup.population_bytes());
    println!("0,{hoe:.6}");

    for epoch in 1..=epochs {
        soup.run_epoch::<S>();
        soup.mutate();

        if epoch % metrics_interval == 0 {
            let hoe = high_order_entropy(&soup.population_bytes());
            println!("{epoch},{hoe:.6}");
        }

        if epoch % 100 == 0 || epoch == epochs {
            eprint!("\repoch {epoch}/{epochs}");
        }
    }
    eprintln!();
}

fn run_benchmark<S: Substrate>(
    config: SoupConfig,
    seed: u64,
    epochs: usize,
) {
    let pop_size = config.population_size;
    let mut soup = Soup::new(config, seed);

    let start = std::time::Instant::now();
    for _ in 0..epochs {
        soup.run_epoch::<S>();
        soup.mutate();
    }
    let elapsed = start.elapsed();

    let total_interactions = epochs as u64 * pop_size as u64;
    let epochs_per_sec = epochs as f64 / elapsed.as_secs_f64();
    let interactions_per_sec = total_interactions as f64 / elapsed.as_secs_f64();

    eprintln!("Benchmark results:");
    eprintln!("  Epochs:            {epochs}");
    eprintln!("  Population size:   {pop_size}");
    eprintln!("  Total interactions: {total_interactions}");
    eprintln!("  Elapsed:           {elapsed:.2?}");
    eprintln!("  Epochs/sec:        {epochs_per_sec:.1}");
    eprintln!("  Interactions/sec:  {interactions_per_sec:.0}");
}

fn run_simulation_2d<S: Substrate>(
    config: Soup2dConfig,
    seed: u64,
    epochs: usize,
    metrics_interval: usize,
) {
    let w = config.width;
    let h = config.height;
    let mut soup = Soup2d::new(config, seed);

    eprintln!("2D simulation: {w}x{h} grid ({} programs)", w * h);
    println!("epoch,hoe");
    let hoe = high_order_entropy(&soup.population_bytes());
    println!("0,{hoe:.6}");

    for epoch in 1..=epochs {
        soup.run_epoch::<S>();
        soup.mutate();

        if epoch % metrics_interval == 0 {
            let hoe = high_order_entropy(&soup.population_bytes());
            println!("{epoch},{hoe:.6}");
        }

        if epoch % 100 == 0 || epoch == epochs {
            eprint!("\repoch {epoch}/{epochs}");
        }
    }
    eprintln!();
}

fn run_benchmark_2d<S: Substrate>(
    config: Soup2dConfig,
    seed: u64,
    epochs: usize,
) {
    let pop_size = config.width * config.height;
    let mut soup = Soup2d::new(config, seed);

    let start = std::time::Instant::now();
    for _ in 0..epochs {
        soup.run_epoch::<S>();
        soup.mutate();
    }
    let elapsed = start.elapsed();

    let epochs_per_sec = epochs as f64 / elapsed.as_secs_f64();

    eprintln!("Benchmark results (2D):");
    eprintln!("  Epochs:            {epochs}");
    eprintln!("  Grid:              {}x{}", pop_size, pop_size); // will fix below
    eprintln!("  Population size:   {pop_size}");
    eprintln!("  Elapsed:           {elapsed:.2?}");
    eprintln!("  Epochs/sec:        {epochs_per_sec:.1}");
}
