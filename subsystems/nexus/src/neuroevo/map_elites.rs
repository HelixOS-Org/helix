//! MAP-Elites: Quality-Diversity algorithm for illuminating solution spaces.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::neuroevo::activation::ActivationFunction;
use crate::neuroevo::genome::NeatGenome;
use crate::neuroevo::innovation::InnovationTracker;
use crate::neuroevo::utils::lcg_next;

/// A cell in the MAP-Elites archive
#[derive(Debug, Clone)]
pub struct EliteCell {
    /// The genome in this cell
    pub genome: NeatGenome,
    /// Behavior descriptor (determines cell position)
    pub behavior: Vec<f64>,
    /// Fitness (quality)
    pub fitness: f64,
}

/// MAP-Elites configuration
#[derive(Debug, Clone)]
pub struct MapElitesConfig {
    /// Number of dimensions in behavior space
    pub behavior_dims: usize,
    /// Number of bins per dimension
    pub bins_per_dim: usize,
    /// Behavior bounds (min, max) for each dimension
    pub behavior_bounds: Vec<(f64, f64)>,
    /// Batch size for parallel evaluation
    pub batch_size: usize,
}

/// MAP-Elites: Illuminating Search Spaces
pub struct MapElites {
    /// Configuration
    pub config: MapElitesConfig,
    /// Archive grid (flattened)
    pub archive: BTreeMap<usize, EliteCell>,
    /// Innovation tracker
    pub tracker: InnovationTracker,
    /// Best fitness found
    pub best_fitness: f64,
    /// Best genome
    pub best_genome: Option<NeatGenome>,
    /// Number of cells filled
    pub coverage: usize,
    /// Random seed
    seed: u64,
}

impl MapElites {
    /// Create a new MAP-Elites instance
    pub fn new(config: MapElitesConfig, seed: u64) -> Self {
        Self {
            config,
            archive: BTreeMap::new(),
            tracker: InnovationTracker::new(),
            best_fitness: f64::NEG_INFINITY,
            best_genome: None,
            coverage: 0,
            seed,
        }
    }

    /// Convert behavior descriptor to cell index
    pub fn behavior_to_cell(&self, behavior: &[f64]) -> usize {
        let mut index = 0;
        let mut multiplier = 1;

        for (i, &val) in behavior.iter().enumerate().take(self.config.behavior_dims) {
            let (min, max) = self
                .config
                .behavior_bounds
                .get(i)
                .copied()
                .unwrap_or((0.0, 1.0));
            let normalized = (val - min) / (max - min);
            let bin = ((normalized * self.config.bins_per_dim as f64) as usize)
                .min(self.config.bins_per_dim - 1);
            index += bin * multiplier;
            multiplier *= self.config.bins_per_dim;
        }

        index
    }

    /// Try to add a genome to the archive
    pub fn try_add(&mut self, genome: NeatGenome, behavior: Vec<f64>, fitness: f64) -> bool {
        let cell_idx = self.behavior_to_cell(&behavior);

        if let Some(existing) = self.archive.get(&cell_idx) {
            if fitness <= existing.fitness {
                return false;
            }
        } else {
            self.coverage += 1;
        }

        let cell = EliteCell {
            genome: genome.clone(),
            behavior,
            fitness,
        };

        self.archive.insert(cell_idx, cell);

        if fitness > self.best_fitness {
            self.best_fitness = fitness;
            self.best_genome = Some(genome);
        }

        true
    }

    /// Generate a new genome by mutation
    pub fn generate_offspring(&mut self, num_inputs: usize, num_outputs: usize) -> NeatGenome {
        self.seed = lcg_next(self.seed);

        if self.archive.is_empty() {
            // Create random initial genome
            return NeatGenome::minimal(
                num_inputs,
                num_outputs,
                ActivationFunction::Tanh,
                &mut self.tracker,
            );
        }

        // Select random parent from archive
        let parent_idx = self.seed as usize % self.archive.len();
        let parent = self.archive.values().nth(parent_idx).unwrap();

        let mut offspring = parent.genome.clone();

        // Apply mutations
        self.seed = lcg_next(self.seed);
        offspring.mutate_weights(0.8, 0.5, self.seed);

        self.seed = lcg_next(self.seed);
        if (self.seed as f64 / u64::MAX as f64) < 0.05 {
            offspring.mutate_add_connection(&mut self.tracker, self.seed);
        }

        self.seed = lcg_next(self.seed);
        if (self.seed as f64 / u64::MAX as f64) < 0.03 {
            offspring.mutate_add_node(&mut self.tracker, self.seed);
        }

        offspring
    }

    /// Get coverage statistics
    #[inline]
    pub fn get_coverage_stats(&self) -> (usize, usize, f64) {
        let total_cells = self
            .config
            .bins_per_dim
            .pow(self.config.behavior_dims as u32);
        let coverage_pct = self.coverage as f64 / total_cells as f64 * 100.0;
        (self.coverage, total_cells, coverage_pct)
    }
}
