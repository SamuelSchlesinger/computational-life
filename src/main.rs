use clap::Parser;
use complife::bff::Bff;
use complife::metrics::high_order_entropy;
use complife::soup::{Soup, SoupConfig};

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

    /// Which instruction set to use.
    #[arg(long, default_value = "bff")]
    substrate: String,

    /// Compute and output metrics every N epochs.
    #[arg(long, default_value_t = 1)]
    metrics_interval: usize,

    /// Run in benchmark mode: suppress CSV, print throughput stats.
    #[arg(long)]
    benchmark: bool,

    /// Launch live visualization window (requires --features viz).
    #[cfg(feature = "viz")]
    #[arg(long)]
    live: bool,
}

fn main() {
    let cli = Cli::parse();

    let config = SoupConfig {
        population_size: cli.population_size,
        program_size: cli.program_size,
        step_limit: cli.step_limit,
        mutation_rate: cli.mutation_rate,
    };

    #[cfg(feature = "viz")]
    if cli.live {
        match cli.substrate.as_str() {
            "bff" => complife::viz::run_viz::<Bff>(config, cli.seed, cli.epochs, cli.metrics_interval),
            other => {
                eprintln!("Unknown substrate: {other}. Available: bff");
                std::process::exit(1);
            }
        }
        return;
    }

    match cli.substrate.as_str() {
        "bff" => {
            if cli.benchmark {
                run_benchmark::<Bff>(config, cli.seed, cli.epochs);
            } else {
                run_simulation::<Bff>(config, cli.seed, cli.epochs, cli.metrics_interval);
            }
        }
        other => {
            eprintln!("Unknown substrate: {other}. Available: bff");
            std::process::exit(1);
        }
    }
}

fn run_simulation<S: complife::substrate::Substrate>(
    config: SoupConfig,
    seed: u64,
    epochs: usize,
    metrics_interval: usize,
) {
    let mut soup = Soup::new(config, seed);

    // CSV header
    println!("epoch,hoe");

    // Epoch 0 metrics
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

fn run_benchmark<S: complife::substrate::Substrate>(
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
