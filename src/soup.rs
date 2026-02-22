use rand::Rng;
use rand::SeedableRng;
use rand::rngs::SmallRng;

use crate::substrate::Substrate;

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

/// Configuration for a primordial soup simulation.
pub struct SoupConfig {
    /// Number of programs in the population.
    pub population_size: usize,
    /// Number of bytes per program.
    pub program_size: usize,
    /// Maximum steps per program execution.
    pub step_limit: usize,
    /// Per-byte mutation probability per epoch (0.0 to disable).
    pub mutation_rate: f64,
}

impl Default for SoupConfig {
    fn default() -> Self {
        Self {
            population_size: 1 << 17, // 131072
            program_size: 64,
            step_limit: 1 << 13, // 8192
            mutation_rate: 0.00024,
        }
    }
}

/// The primordial soup: a population of programs that interact.
pub struct Soup {
    /// The population: a flat vector of programs, each `program_size` bytes.
    pub programs: Vec<Vec<u8>>,
    pub config: SoupConfig,
    pub rng: SmallRng,
}

impl Soup {
    /// Create a new soup with randomly initialized programs.
    pub fn new(config: SoupConfig, seed: u64) -> Self {
        let mut rng = SmallRng::seed_from_u64(seed);
        let programs = (0..config.population_size)
            .map(|_| {
                let mut prog = vec![0u8; config.program_size];
                rng.fill(&mut prog[..]);
                prog
            })
            .collect();
        Self {
            programs,
            config,
            rng,
        }
    }

    /// Run a single interaction step: pick two programs, concatenate in random
    /// order, execute, split result back.
    pub fn interaction_step<S: Substrate>(&mut self) {
        let n = self.programs.len();
        if n < 2 {
            return;
        }

        // Pick two distinct programs uniformly at random.
        let i = self.rng.gen_range(0..n);
        let mut j = self.rng.gen_range(0..n - 1);
        if j >= i {
            j += 1;
        }

        // Random concatenation order (AB or BA).
        let (first, second) = if self.rng.r#gen::<bool>() {
            (i, j)
        } else {
            (j, i)
        };

        // Concatenate into a temporary buffer.
        let ps = self.config.program_size;
        let mut tape = vec![0u8; ps * 2];
        tape[..ps].copy_from_slice(&self.programs[first]);
        tape[ps..].copy_from_slice(&self.programs[second]);

        // Execute.
        S::execute(&mut tape, self.config.step_limit);

        // Split result back.
        self.programs[first] = tape[..ps].to_vec();
        self.programs[second] = tape[ps..].to_vec();
    }

    /// Run one epoch: N interaction steps where N = population_size.
    pub fn run_epoch<S: Substrate>(&mut self) {
        let n = self.config.population_size;
        for _ in 0..n {
            self.interaction_step::<S>();
        }
    }

    /// Apply background mutation: flip random bits with the configured probability.
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
            pos = pos.saturating_add(1).saturating_add(geometric_skip(&mut self.rng, inv_log));
        }
    }

    /// Fill `buf` with the entire population as a flat byte slice.
    pub fn population_bytes_into(&self, buf: &mut Vec<u8>) {
        buf.clear();
        let total = self.config.population_size * self.config.program_size;
        buf.reserve(total);
        for prog in &self.programs {
            buf.extend_from_slice(prog);
        }
    }

    /// Get the entire population as a flat byte slice (convenience wrapper).
    pub fn population_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.population_bytes_into(&mut buf);
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bff::Bff;

    #[test]
    fn test_deterministic_initialization() {
        let config1 = SoupConfig {
            population_size: 64,
            program_size: 16,
            ..Default::default()
        };
        let config2 = SoupConfig {
            population_size: 64,
            program_size: 16,
            ..Default::default()
        };
        let soup1 = Soup::new(config1, 42);
        let soup2 = Soup::new(config2, 42);
        assert_eq!(soup1.programs, soup2.programs);
    }

    #[test]
    fn test_deterministic_simulation() {
        let run = |seed: u64| -> Vec<Vec<u8>> {
            let config = SoupConfig {
                population_size: 32,
                program_size: 16,
                step_limit: 256,
                mutation_rate: 0.001,
            };
            let mut soup = Soup::new(config, seed);
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
    fn test_different_seeds_different_results() {
        let config1 = SoupConfig {
            population_size: 64,
            program_size: 16,
            ..Default::default()
        };
        let config2 = SoupConfig {
            population_size: 64,
            program_size: 16,
            ..Default::default()
        };
        let soup1 = Soup::new(config1, 1);
        let soup2 = Soup::new(config2, 2);
        assert_ne!(soup1.programs, soup2.programs);
    }

    #[test]
    fn test_population_size() {
        let config = SoupConfig {
            population_size: 128,
            program_size: 32,
            ..Default::default()
        };
        let soup = Soup::new(config, 0);
        assert_eq!(soup.programs.len(), 128);
        for prog in &soup.programs {
            assert_eq!(prog.len(), 32);
        }
    }

    #[test]
    fn test_mutation_disabled() {
        let config = SoupConfig {
            population_size: 64,
            program_size: 16,
            mutation_rate: 0.0,
            ..Default::default()
        };
        let mut soup = Soup::new(config, 42);
        let before = soup.programs.clone();
        soup.mutate();
        assert_eq!(soup.programs, before);
    }

    #[test]
    fn test_population_bytes_length() {
        let config = SoupConfig {
            population_size: 64,
            program_size: 16,
            ..Default::default()
        };
        let soup = Soup::new(config, 42);
        assert_eq!(soup.population_bytes().len(), 64 * 16);
    }

    #[test]
    fn test_integration_small_simulation() {
        use crate::metrics::high_order_entropy;

        let config = SoupConfig {
            population_size: 256,
            program_size: 64,
            step_limit: 8192,
            mutation_rate: 0.00024,
        };
        let mut soup = Soup::new(config, 42);

        // Initial HOE should be near 1.0 (random data).
        let initial_hoe = high_order_entropy(&soup.population_bytes());
        assert!(
            initial_hoe > 0.8,
            "Initial HOE should be near 1.0, got {initial_hoe}"
        );

        // Run 100 epochs.
        for _ in 0..100 {
            soup.run_epoch::<Bff>();
            soup.mutate();
        }

        // HOE should still be computable (no crashes).
        let final_hoe = high_order_entropy(&soup.population_bytes());
        assert!(final_hoe > 0.0, "Final HOE should be positive");
        // We don't assert that replicators have emerged — 100 epochs with 256
        // programs is too short. Just verify the simulation runs without error.
    }
}
