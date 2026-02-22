use rand::seq::SliceRandom;
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use rayon::prelude::*;

use crate::substrate::Substrate;

/// Configuration for a 2D spatial simulation.
pub struct Soup2dConfig {
    /// Grid width.
    pub width: usize,
    /// Grid height.
    pub height: usize,
    /// Number of bytes per program.
    pub program_size: usize,
    /// Maximum steps per program execution.
    pub step_limit: usize,
    /// Per-byte mutation probability per epoch (0.0 to disable).
    pub mutation_rate: f64,
}

/// A 2D spatial primordial soup: programs arranged on a grid,
/// interacting only with Chebyshev-distance-2 neighbors.
pub struct Soup2d {
    /// The population as a flat vector, indexed by y * width + x.
    pub programs: Vec<Vec<u8>>,
    pub config: Soup2dConfig,
    pub rng: SmallRng,
    /// Pre-computed flat buffer of all neighbor indices.
    neighbor_indices: Vec<usize>,
    /// Per-cell (start, end) range into `neighbor_indices`.
    neighbor_ranges: Vec<(usize, usize)>,
    /// Reusable scratch: shuffled iteration order.
    order: Vec<usize>,
    /// Reusable scratch: taken flags.
    taken: Vec<bool>,
    /// Reusable scratch: interaction pairs (first_idx, second_idx).
    pairs: Vec<(usize, usize)>,
    /// Reusable scratch: flat tape buffer for parallel execution.
    tape_pool: Vec<u8>,
}

impl Soup2d {
    /// Create a new 2D soup with randomly initialized programs.
    pub fn new(config: Soup2dConfig, seed: u64) -> Self {
        let mut rng = SmallRng::seed_from_u64(seed);
        let w = config.width;
        let h = config.height;
        let total = w * h;
        let programs = (0..total)
            .map(|_| {
                let mut prog = vec![0u8; config.program_size];
                rng.fill(&mut prog[..]);
                prog
            })
            .collect();

        // Pre-compute neighbor table.
        let mut neighbor_indices = Vec::with_capacity(total * 24);
        let mut neighbor_ranges = Vec::with_capacity(total);
        for cell in 0..total {
            let cx = cell % w;
            let cy = cell / w;
            let start = neighbor_indices.len();
            let x_lo = cx.saturating_sub(2);
            let x_hi = (cx + 2).min(w - 1);
            let y_lo = cy.saturating_sub(2);
            let y_hi = (cy + 2).min(h - 1);
            for ny in y_lo..=y_hi {
                for nx in x_lo..=x_hi {
                    if nx == cx && ny == cy {
                        continue;
                    }
                    neighbor_indices.push(ny * w + nx);
                }
            }
            neighbor_ranges.push((start, neighbor_indices.len()));
        }

        let order: Vec<usize> = (0..total).collect();
        let taken = vec![false; total];
        let pairs = Vec::with_capacity(total / 2);
        let tape_pool = Vec::new();

        Self {
            programs,
            config,
            rng,
            neighbor_indices,
            neighbor_ranges,
            order,
            taken,
            pairs,
            tape_pool,
        }
    }

    /// Get the pre-computed neighbor indices for cell at (cx, cy).
    #[cfg(test)]
    fn neighbors(&self, cx: usize, cy: usize) -> &[usize] {
        let cell = cy * self.config.width + cx;
        let (start, end) = self.neighbor_ranges[cell];
        &self.neighbor_indices[start..end]
    }

    /// Run one 2D epoch: iterate programs in shuffled order, pair with
    /// random Chebyshev-distance-2 neighbor, interact if neither is taken.
    ///
    /// Phase 1 (sequential): build interaction pairs using RNG.
    /// Phase 2 (parallel): copy programs into a flat tape buffer, execute
    /// all tapes in parallel via rayon, copy results back.
    pub fn run_epoch<S: Substrate + Sync>(&mut self) {
        let total = self.config.width * self.config.height;
        let ps = self.config.program_size;
        let step_limit = self.config.step_limit;

        // --- Phase 1: build pairs (sequential) ---

        // Reset and shuffle order.
        for i in 0..total {
            self.order[i] = i;
        }
        self.order.shuffle(&mut self.rng);

        // Reset taken flags and pairs.
        self.taken.fill(false);
        self.pairs.clear();

        for i in 0..total {
            let p_idx = self.order[i];
            if self.taken[p_idx] {
                continue;
            }

            let (start, end) = self.neighbor_ranges[p_idx];
            let neighbor_count = end - start;
            if neighbor_count == 0 {
                continue;
            }

            // Select a random neighbor.
            let n_idx = self.neighbor_indices[start + self.rng.gen_range(0..neighbor_count)];
            if self.taken[n_idx] {
                continue;
            }

            // Mark both as taken.
            self.taken[p_idx] = true;
            self.taken[n_idx] = true;

            // Random concatenation order.
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

        // Resize tape pool to hold all pair tapes.
        self.tape_pool.resize(num_pairs * tape_size, 0);

        // Copy programs into the flat tape buffer.
        for (i, &(first, second)) in self.pairs.iter().enumerate() {
            let base = i * tape_size;
            self.tape_pool[base..base + ps].copy_from_slice(&self.programs[first]);
            self.tape_pool[base + ps..base + tape_size].copy_from_slice(&self.programs[second]);
        }

        // Execute all tapes in parallel.
        self.tape_pool
            .par_chunks_mut(tape_size)
            .for_each(|tape| {
                S::execute(tape, step_limit);
            });

        // Copy results back.
        for (i, &(first, second)) in self.pairs.iter().enumerate() {
            let base = i * tape_size;
            self.programs[first].copy_from_slice(&self.tape_pool[base..base + ps]);
            self.programs[second].copy_from_slice(&self.tape_pool[base + ps..base + tape_size]);
        }
    }

    /// Apply background mutation to all programs.
    pub fn mutate(&mut self) {
        if self.config.mutation_rate <= 0.0 {
            return;
        }
        for prog in &mut self.programs {
            for byte in prog.iter_mut() {
                if self.rng.gen_bool(self.config.mutation_rate) {
                    let bit = 1u8 << self.rng.gen_range(0..8);
                    *byte ^= bit;
                }
            }
        }
    }

    /// Fill `buf` with the entire population as a flat byte slice.
    pub fn population_bytes_into(&self, buf: &mut Vec<u8>) {
        buf.clear();
        let total = self.config.width * self.config.height * self.config.program_size;
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
    fn test_neighbors_center() {
        let config = Soup2dConfig {
            width: 10,
            height: 10,
            program_size: 64,
            step_limit: 8192,
            mutation_rate: 0.0,
        };
        let soup = Soup2d::new(config, 42);
        let neighbors = soup.neighbors(5, 5);
        // 5x5 neighborhood minus center = 24
        assert_eq!(neighbors.len(), 24);
        // All should be within Chebyshev distance 2
        for &idx in neighbors {
            let nx: usize = idx % 10;
            let ny: usize = idx / 10;
            assert!(nx.abs_diff(5) <= 2);
            assert!(ny.abs_diff(5) <= 2);
            assert!(!(nx == 5 && ny == 5));
        }
    }

    #[test]
    fn test_neighbors_corner() {
        let config = Soup2dConfig {
            width: 10,
            height: 10,
            program_size: 64,
            step_limit: 8192,
            mutation_rate: 0.0,
        };
        let soup = Soup2d::new(config, 42);
        let neighbors = soup.neighbors(0, 0);
        // (0..2, 0..2) = 3x3 = 9 cells minus self = 8
        assert_eq!(neighbors.len(), 8);
        for &idx in neighbors {
            let nx: usize = idx % 10;
            let ny: usize = idx / 10;
            assert!(nx <= 2);
            assert!(ny <= 2);
        }
    }

    #[test]
    fn test_neighbors_edge() {
        let config = Soup2dConfig {
            width: 10,
            height: 10,
            program_size: 64,
            step_limit: 8192,
            mutation_rate: 0.0,
        };
        let soup = Soup2d::new(config, 42);
        let neighbors = soup.neighbors(0, 5);
        // x: 0..2 (3), y: 3..7 (5) = 15 - 1 = 14
        assert_eq!(neighbors.len(), 14);
    }

    #[test]
    fn test_neighbors_opposite_corner() {
        let config = Soup2dConfig {
            width: 10,
            height: 10,
            program_size: 64,
            step_limit: 8192,
            mutation_rate: 0.0,
        };
        let soup = Soup2d::new(config, 42);
        let neighbors = soup.neighbors(9, 9);
        // x: 7..9 (3), y: 7..9 (3) = 9 - 1 = 8
        assert_eq!(neighbors.len(), 8);
    }

    #[test]
    fn test_deterministic_2d_simulation() {
        let run = |seed: u64| -> Vec<Vec<u8>> {
            let config = Soup2dConfig {
                width: 5,
                height: 5,
                program_size: 16,
                step_limit: 256,
                mutation_rate: 0.001,
            };
            let mut soup = Soup2d::new(config, seed);
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
    fn test_population_size() {
        let config = Soup2dConfig {
            width: 8,
            height: 6,
            program_size: 32,
            step_limit: 8192,
            mutation_rate: 0.0,
        };
        let soup = Soup2d::new(config, 42);
        assert_eq!(soup.programs.len(), 48);
        for prog in &soup.programs {
            assert_eq!(prog.len(), 32);
        }
    }

    #[test]
    fn test_population_bytes_length() {
        let config = Soup2dConfig {
            width: 8,
            height: 6,
            program_size: 32,
            step_limit: 8192,
            mutation_rate: 0.0,
        };
        let soup = Soup2d::new(config, 42);
        assert_eq!(soup.population_bytes().len(), 48 * 32);
    }

    #[test]
    fn test_integration_small_2d_simulation() {
        use crate::metrics::high_order_entropy;

        let config = Soup2dConfig {
            width: 10,
            height: 10,
            program_size: 64,
            step_limit: 8192,
            mutation_rate: 0.00024,
        };
        let mut soup = Soup2d::new(config, 42);

        let initial_hoe = high_order_entropy(&soup.population_bytes());
        assert!(initial_hoe > 0.5, "Initial HOE should be reasonably high, got {initial_hoe}");

        for _ in 0..100 {
            soup.run_epoch::<Bff>();
            soup.mutate();
        }

        let final_hoe = high_order_entropy(&soup.population_bytes());
        assert!(final_hoe > 0.0, "Final HOE should be positive");
    }

    #[test]
    fn test_mutation_disabled_2d() {
        let config = Soup2dConfig {
            width: 4,
            height: 4,
            program_size: 16,
            mutation_rate: 0.0,
            step_limit: 8192,
        };
        let mut soup = Soup2d::new(config, 42);
        let before = soup.programs.clone();
        soup.mutate();
        assert_eq!(soup.programs, before);
    }
}
