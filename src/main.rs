use clap::Parser;
use complife::bff::Bff;
use complife::forth::Forth;
use complife::subleq::{Subleq, Rsubleq4};
use complife::metrics::high_order_entropy;
use complife::soup::{Soup, SoupConfig};
use complife::substrate::Substrate;
use complife::surface::{SoupSurface, SoupSurfaceConfig, SurfaceMesh, SurfaceSpec};

#[derive(Parser)]
#[command(name = "complife", about = "Computational Life: primordial soup simulation")]
struct Cli {
    /// Random seed for reproducibility.
    #[arg(long)]
    seed: u64,

    /// Number of epochs to run.
    #[arg(long)]
    epochs: usize,

    /// Number of programs in the population (0D mode only).
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

    /// Which instruction set to use (bff, forth, subleq, rsubleq4).
    #[arg(long, default_value = "bff")]
    substrate: String,

    /// Compute and output metrics every N epochs.
    #[arg(long, default_value_t = 25)]
    metrics_interval: usize,

    /// Run in benchmark mode: suppress CSV, print throughput stats.
    #[arg(long)]
    benchmark: bool,

    /// Run simulation on a surface (flat:WxH, sphere:N, torus:MxN, tunnel:N, obj:PATH).
    #[arg(long)]
    surface: Option<String>,

    /// Geodesic neighbor radius in mesh units (default: auto).
    #[arg(long)]
    neighbor_radius: Option<f32>,

    /// Launch live visualization window (requires --features viz).
    #[cfg(feature = "viz")]
    #[arg(long)]
    live: bool,

    /// Spatial blur strength for live viewer (0.0 = off, 1.0 = max).
    #[cfg(feature = "viz")]
    #[arg(long, default_value_t = 0.0)]
    blur: f32,
}

/// Parse a "WxH" dimension string.
fn parse_dimensions(s: &str) -> Result<(usize, usize), String> {
    let parts: Vec<&str> = s.split('x').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid dimensions '{s}', expected WxH (e.g. 240x135)"));
    }
    let w = parts[0].parse::<usize>().map_err(|e| format!("Invalid width: {e}"))?;
    let h = parts[1].parse::<usize>().map_err(|e| format!("Invalid height: {e}"))?;
    if w == 0 || h == 0 {
        return Err("Dimensions must be positive".into());
    }
    Ok((w, h))
}

/// Parse a --surface spec into a SurfaceSpec (for surface types with GUI support).
/// OBJ files bypass SurfaceSpec and go directly to SurfaceMesh.
fn parse_surface_spec(spec: &str, seed: u64) -> Result<SurfaceSpec, String> {
    if let Some(rest) = spec.strip_prefix("flat:") {
        let (w, h) = parse_dimensions(rest)?;
        Ok(SurfaceSpec::FlatGrid { width: w, height: h })
    } else if let Some(rest) = spec.strip_prefix("sphere:") {
        let sub = rest.parse::<usize>().map_err(|e| format!("Invalid subdivision level: {e}"))?;
        Ok(SurfaceSpec::Sphere { subdivisions: sub })
    } else if let Some(rest) = spec.strip_prefix("torus:") {
        let (m, n) = parse_dimensions(rest)?;
        Ok(SurfaceSpec::Torus { major: m, minor: n })
    } else if let Some(rest) = spec.strip_prefix("tunnel:") {
        let n = rest.parse::<usize>().map_err(|e| format!("Invalid tunnel sphere count: {e}"))?;
        Ok(SurfaceSpec::HamsterTunnel { num_spheres: n, segments: 24, seed })
    } else {
        Err(format!(
            "Unknown surface type: '{spec}'. Expected flat:WxH, sphere:N, torus:MxN, tunnel:N, or obj:PATH"
        ))
    }
}

/// Parse a --surface spec into a SurfaceMesh.
fn parse_surface(spec: &str, neighbor_radius: Option<f32>, seed: u64) -> Result<SurfaceMesh, String> {
    // OBJ files bypass SurfaceSpec.
    if let Some(rest) = spec.strip_prefix("obj:") {
        let mut mesh = SurfaceMesh::from_obj(rest)?;
        mesh.compute_neighbors(neighbor_radius);
        return Ok(mesh);
    }
    let surface_spec = parse_surface_spec(spec, seed)?;
    let mut mesh = surface_spec.build()?;
    mesh.compute_neighbors(neighbor_radius);
    Ok(mesh)
}

fn main() {
    let cli = Cli::parse();

    #[cfg(feature = "viz")]
    if cli.live {
        let substrate = match cli.substrate.as_str() {
            "bff" => complife::viz::SubstrateKind::Bff,
            "forth" => complife::viz::SubstrateKind::Forth,
            "subleq" => complife::viz::SubstrateKind::Subleq,
            "rsubleq4" => complife::viz::SubstrateKind::Rsubleq4,
            other => {
                eprintln!("Unknown substrate: {other}. Available: bff, forth, subleq, rsubleq4");
                std::process::exit(1);
            }
        };
        let initial_spec = cli.surface.as_ref()
            .and_then(|s| parse_surface_spec(s, cli.seed).ok())
            .unwrap_or(SurfaceSpec::Sphere { subdivisions: 4 });

        let menu_config = complife::viz::MenuConfig::new(
            substrate,
            &initial_spec,
            cli.seed,
            cli.neighbor_radius,
            cli.program_size,
            cli.step_limit,
            cli.mutation_rate,
            cli.epochs,
            cli.metrics_interval,
            cli.blur,
        );
        complife::viz::run_app(menu_config);
        return;
    }

    match cli.substrate.as_str() {
        "bff" => dispatch::<Bff>(&cli),
        "forth" => dispatch::<Forth>(&cli),
        "subleq" => dispatch::<Subleq>(&cli),
        "rsubleq4" => dispatch::<Rsubleq4>(&cli),
        other => {
            eprintln!("Unknown substrate: {other}. Available: bff, forth, subleq, rsubleq4");
            std::process::exit(1);
        }
    }
}

fn dispatch<S: Substrate + Send + Sync + 'static>(cli: &Cli) {
    if let Some(ref surface_spec_str) = cli.surface {
        let mesh = match parse_surface(surface_spec_str, cli.neighbor_radius, cli.seed) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        };
        let config = SoupSurfaceConfig {
            program_size: cli.program_size,
            step_limit: cli.step_limit,
            mutation_rate: cli.mutation_rate,
        };

        if cli.benchmark {
            run_benchmark_surface::<S>(mesh, config, cli.seed, cli.epochs);
        } else {
            run_simulation_surface::<S>(mesh, config, cli.seed, cli.epochs, cli.metrics_interval);
        }
    } else {
        let config = SoupConfig {
            population_size: cli.population_size,
            program_size: cli.program_size,
            step_limit: cli.step_limit,
            mutation_rate: cli.mutation_rate,
        };

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
    let mut pop_buf = Vec::new();

    println!("epoch,hoe");
    soup.population_bytes_into(&mut pop_buf);
    let hoe = high_order_entropy(&pop_buf);
    println!("0,{hoe:.6}");

    for epoch in 1..=epochs {
        soup.run_epoch::<S>();
        soup.mutate();

        if epoch % metrics_interval == 0 {
            soup.population_bytes_into(&mut pop_buf);
            let hoe = high_order_entropy(&pop_buf);
            println!("{epoch},{hoe:.6}");
        }

        if epoch % 100 == 0 || epoch == epochs {
            eprint!("\repoch {epoch}/{epochs}");
        }
    }
    eprintln!();
}

fn run_benchmark<S: Substrate>(config: SoupConfig, seed: u64, epochs: usize) {
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

fn run_simulation_surface<S: Substrate + Sync>(
    mesh: SurfaceMesh,
    config: SoupSurfaceConfig,
    seed: u64,
    epochs: usize,
    metrics_interval: usize,
) {
    let num_cells = mesh.num_cells();
    let mut soup = SoupSurface::new(mesh, config, seed);
    let mut pop_buf = Vec::new();

    eprintln!("Surface simulation: {num_cells} programs");
    println!("epoch,hoe");
    soup.population_bytes_into(&mut pop_buf);
    let hoe = high_order_entropy(&pop_buf);
    println!("0,{hoe:.6}");

    for epoch in 1..=epochs {
        soup.run_epoch::<S>();
        soup.mutate();

        if epoch % metrics_interval == 0 {
            soup.population_bytes_into(&mut pop_buf);
            let hoe = high_order_entropy(&pop_buf);
            println!("{epoch},{hoe:.6}");
        }

        if epoch % 100 == 0 || epoch == epochs {
            eprint!("\repoch {epoch}/{epochs}");
        }
    }
    eprintln!();
}

fn run_benchmark_surface<S: Substrate + Sync>(
    mesh: SurfaceMesh,
    config: SoupSurfaceConfig,
    seed: u64,
    epochs: usize,
) {
    let num_cells = mesh.num_cells();
    let mut soup = SoupSurface::new(mesh, config, seed);

    let start = std::time::Instant::now();
    for _ in 0..epochs {
        soup.run_epoch::<S>();
        soup.mutate();
    }
    let elapsed = start.elapsed();

    let epochs_per_sec = epochs as f64 / elapsed.as_secs_f64();

    eprintln!("Benchmark results (surface):");
    eprintln!("  Epochs:            {epochs}");
    eprintln!("  Population size:   {num_cells}");
    eprintln!("  Elapsed:           {elapsed:.2?}");
    eprintln!("  Epochs/sec:        {epochs_per_sec:.1}");
}
