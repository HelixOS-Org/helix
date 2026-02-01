//! # Evolution Loop
//!
//! Year 3 EVOLUTION - Main evolution loop and strategies
//! Orchestrates the complete evolutionary process.

#![allow(dead_code)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::crossover::CrossoverEngine;
use super::genome::CodeGenome;
use super::mutation::MutationEngine;
use super::population::Population;
use super::{
    EvolutionConfig, EvolutionStats, Fitness, Generation, GenerationStats, GeneticEngine, GenomeId,
    Individual, SelectionStrategy,
};

// ============================================================================
// EVOLUTION STRATEGIES
// ============================================================================

/// Evolution strategy
#[derive(Debug, Clone)]
pub enum EvolutionStrategy {
    /// Standard generational replacement
    Generational,
    /// Steady-state (replace worst)
    SteadyState { replacement_count: usize },
    /// μ + λ strategy
    MuPlusLambda { mu: usize, lambda: usize },
    /// μ, λ strategy
    MuCommaLambda { mu: usize, lambda: usize },
    /// Differential evolution
    DifferentialEvolution { f: f64, cr: f64 },
    /// Coevolution
    Coevolution { species_count: usize },
    /// Novelty search
    NoveltySearch { archive_threshold: f64 },
    /// MAP-Elites (quality-diversity)
    MapElites { grid_dims: Vec<usize> },
}

/// Evolution controller
pub struct EvolutionController {
    /// Strategy
    strategy: EvolutionStrategy,
    /// Configuration
    config: EvolutionConfig,
    /// Mutation engine
    mutation: MutationEngine,
    /// Crossover engine
    crossover: CrossoverEngine,
    /// Novelty archive (for novelty search)
    novelty_archive: Vec<CodeGenome>,
    /// MAP-Elites grid
    map_elites_grid: BTreeMap<Vec<usize>, Individual>,
    /// Current generation
    generation: Generation,
    /// Statistics
    stats: EvolutionStats,
}

impl EvolutionController {
    /// Create new controller
    pub fn new(strategy: EvolutionStrategy, config: EvolutionConfig) -> Self {
        Self {
            strategy,
            mutation: MutationEngine::default(),
            crossover: CrossoverEngine::default(),
            novelty_archive: Vec::new(),
            map_elites_grid: BTreeMap::new(),
            generation: Generation(0),
            stats: EvolutionStats::default(),
            config,
        }
    }

    /// Evolve population one generation
    pub fn evolve<F>(&mut self, population: &mut Population, fitness_fn: F) -> GenerationStats
    where
        F: Fn(&CodeGenome) -> Fitness,
    {
        match &self.strategy {
            EvolutionStrategy::Generational => self.evolve_generational(population, fitness_fn),
            EvolutionStrategy::SteadyState { replacement_count } => {
                self.evolve_steady_state(population, fitness_fn, *replacement_count)
            },
            EvolutionStrategy::MuPlusLambda { mu, lambda } => {
                self.evolve_mu_plus_lambda(population, fitness_fn, *mu, *lambda)
            },
            EvolutionStrategy::MuCommaLambda { mu, lambda } => {
                self.evolve_mu_comma_lambda(population, fitness_fn, *mu, *lambda)
            },
            EvolutionStrategy::DifferentialEvolution { f, cr } => {
                self.evolve_differential(population, fitness_fn, *f, *cr)
            },
            EvolutionStrategy::Coevolution { species_count } => {
                self.evolve_coevolution(population, fitness_fn, *species_count)
            },
            EvolutionStrategy::NoveltySearch { archive_threshold } => {
                self.evolve_novelty(population, fitness_fn, *archive_threshold)
            },
            EvolutionStrategy::MapElites { grid_dims } => {
                self.evolve_map_elites(population, fitness_fn, grid_dims.clone())
            },
        }
    }

    fn evolve_generational<F>(
        &mut self,
        population: &mut Population,
        fitness_fn: F,
    ) -> GenerationStats
    where
        F: Fn(&CodeGenome) -> Fitness,
    {
        // Evaluate all individuals
        for individual in population.iter_mut() {
            if individual.fitness.is_none() {
                individual.fitness = Some(fitness_fn(&individual.genome));
                self.stats.evaluations += 1;
            }
        }

        // Select parents
        let parents = population.tournament_selection(3, self.config.population_size);

        // Create offspring
        let mut offspring = Vec::new();

        // Elitism
        let elites = population.elites(self.config.elite_count);
        offspring.extend(elites);

        // Breeding
        while offspring.len() < self.config.population_size {
            let parent1 = &parents[rand_usize(parents.len())];
            let parent2 = &parents[rand_usize(parents.len())];

            let child_genome = if rand_f64() < self.config.crossover_rate {
                self.stats.crossovers += 1;
                self.crossover.crossover(
                    &parent1.genome,
                    &parent2.genome,
                    parent1.fitness.as_ref(),
                    parent2.fitness.as_ref(),
                )
            } else {
                parent1.genome.clone()
            };

            let mutated = if rand_f64() < self.config.mutation_rate {
                self.stats.mutations += 1;
                self.mutation.mutate(&child_genome)
            } else {
                child_genome
            };

            let child = Individual {
                id: GenomeId(rand_u64()),
                genome: mutated,
                fitness: None,
                species: None,
                generation: Generation(self.generation.0 + 1),
                parents: vec![parent1.id, parent2.id],
                lineage: super::Lineage::default(),
            };

            offspring.push(child);
        }

        population.replace(offspring);
        self.generation = Generation(self.generation.0 + 1);

        self.compute_stats(population)
    }

    fn evolve_steady_state<F>(
        &mut self,
        population: &mut Population,
        fitness_fn: F,
        replacement_count: usize,
    ) -> GenerationStats
    where
        F: Fn(&CodeGenome) -> Fitness,
    {
        // Evaluate all
        for individual in population.iter_mut() {
            if individual.fitness.is_none() {
                individual.fitness = Some(fitness_fn(&individual.genome));
                self.stats.evaluations += 1;
            }
        }

        // Create offspring
        let parents = population.tournament_selection(3, replacement_count * 2);

        let mut new_individuals = Vec::new();

        for i in 0..(replacement_count.min(parents.len() / 2)) {
            let parent1 = &parents[i * 2];
            let parent2 = &parents[i * 2 + 1];

            let child_genome = self.crossover.crossover(
                &parent1.genome,
                &parent2.genome,
                parent1.fitness.as_ref(),
                parent2.fitness.as_ref(),
            );

            let mutated = self.mutation.mutate(&child_genome);

            let fitness = fitness_fn(&mutated);
            self.stats.evaluations += 1;

            let child = Individual {
                id: GenomeId(rand_u64()),
                genome: mutated,
                fitness: Some(fitness),
                species: None,
                generation: self.generation,
                parents: vec![parent1.id, parent2.id],
                lineage: super::Lineage::default(),
            };

            new_individuals.push(child);
        }

        // Replace worst individuals
        let mut all: Vec<Individual> = population.iter().cloned().collect();
        all.sort_by(|a, b| {
            let fa = a
                .fitness
                .as_ref()
                .map(|f| f.scalar)
                .unwrap_or(f64::NEG_INFINITY);
            let fb = b
                .fitness
                .as_ref()
                .map(|f| f.scalar)
                .unwrap_or(f64::NEG_INFINITY);
            fb.partial_cmp(&fa).unwrap_or(core::cmp::Ordering::Equal)
        });

        // Remove worst
        for _ in 0..new_individuals.len().min(all.len()) {
            all.pop();
        }

        // Add new
        all.extend(new_individuals);

        population.replace(all);
        self.generation = Generation(self.generation.0 + 1);

        self.compute_stats(population)
    }

    fn evolve_mu_plus_lambda<F>(
        &mut self,
        population: &mut Population,
        fitness_fn: F,
        mu: usize,
        lambda: usize,
    ) -> GenerationStats
    where
        F: Fn(&CodeGenome) -> Fitness,
    {
        // Evaluate
        for individual in population.iter_mut() {
            if individual.fitness.is_none() {
                individual.fitness = Some(fitness_fn(&individual.genome));
                self.stats.evaluations += 1;
            }
        }

        // Select mu best parents
        let parents = population.elites(mu);

        // Generate lambda offspring
        let mut offspring = Vec::new();

        for _ in 0..lambda {
            let parent1 = &parents[rand_usize(parents.len())];
            let parent2 = &parents[rand_usize(parents.len())];

            let child_genome = self.crossover.crossover(
                &parent1.genome,
                &parent2.genome,
                parent1.fitness.as_ref(),
                parent2.fitness.as_ref(),
            );

            let mutated = self.mutation.mutate(&child_genome);
            let fitness = fitness_fn(&mutated);
            self.stats.evaluations += 1;

            offspring.push(Individual {
                id: GenomeId(rand_u64()),
                genome: mutated,
                fitness: Some(fitness),
                species: None,
                generation: Generation(self.generation.0 + 1),
                parents: vec![parent1.id, parent2.id],
                lineage: super::Lineage::default(),
            });
        }

        // Combine parents and offspring, select best mu
        let mut combined: Vec<Individual> = parents;
        combined.extend(offspring);

        combined.sort_by(|a, b| {
            let fa = a
                .fitness
                .as_ref()
                .map(|f| f.scalar)
                .unwrap_or(f64::NEG_INFINITY);
            let fb = b
                .fitness
                .as_ref()
                .map(|f| f.scalar)
                .unwrap_or(f64::NEG_INFINITY);
            fb.partial_cmp(&fa).unwrap_or(core::cmp::Ordering::Equal)
        });

        combined.truncate(self.config.population_size);
        population.replace(combined);
        self.generation = Generation(self.generation.0 + 1);

        self.compute_stats(population)
    }

    fn evolve_mu_comma_lambda<F>(
        &mut self,
        population: &mut Population,
        fitness_fn: F,
        mu: usize,
        lambda: usize,
    ) -> GenerationStats
    where
        F: Fn(&CodeGenome) -> Fitness,
    {
        // Evaluate
        for individual in population.iter_mut() {
            if individual.fitness.is_none() {
                individual.fitness = Some(fitness_fn(&individual.genome));
                self.stats.evaluations += 1;
            }
        }

        // Select mu best parents
        let parents = population.elites(mu);

        // Generate lambda offspring (must be >= mu)
        let mut offspring = Vec::new();

        for _ in 0..lambda.max(mu) {
            let parent1 = &parents[rand_usize(parents.len())];
            let parent2 = &parents[rand_usize(parents.len())];

            let child_genome = self.crossover.crossover(
                &parent1.genome,
                &parent2.genome,
                parent1.fitness.as_ref(),
                parent2.fitness.as_ref(),
            );

            let mutated = self.mutation.mutate(&child_genome);
            let fitness = fitness_fn(&mutated);
            self.stats.evaluations += 1;

            offspring.push(Individual {
                id: GenomeId(rand_u64()),
                genome: mutated,
                fitness: Some(fitness),
                species: None,
                generation: Generation(self.generation.0 + 1),
                parents: vec![parent1.id, parent2.id],
                lineage: super::Lineage::default(),
            });
        }

        // Select best from offspring only (comma strategy)
        offspring.sort_by(|a, b| {
            let fa = a
                .fitness
                .as_ref()
                .map(|f| f.scalar)
                .unwrap_or(f64::NEG_INFINITY);
            let fb = b
                .fitness
                .as_ref()
                .map(|f| f.scalar)
                .unwrap_or(f64::NEG_INFINITY);
            fb.partial_cmp(&fa).unwrap_or(core::cmp::Ordering::Equal)
        });

        offspring.truncate(self.config.population_size);
        population.replace(offspring);
        self.generation = Generation(self.generation.0 + 1);

        self.compute_stats(population)
    }

    fn evolve_differential<F>(
        &mut self,
        population: &mut Population,
        fitness_fn: F,
        f: f64,
        cr: f64,
    ) -> GenerationStats
    where
        F: Fn(&CodeGenome) -> Fitness,
    {
        // Evaluate
        for individual in population.iter_mut() {
            if individual.fitness.is_none() {
                individual.fitness = Some(fitness_fn(&individual.genome));
                self.stats.evaluations += 1;
            }
        }

        let individuals: Vec<Individual> = population.iter().cloned().collect();
        let mut next_gen = Vec::new();

        for i in 0..individuals.len() {
            let target = &individuals[i];

            // Select three distinct random individuals
            let mut indices: Vec<usize> = (0..individuals.len()).filter(|&j| j != i).collect();
            shuffle(&mut indices);

            if indices.len() < 3 {
                next_gen.push(target.clone());
                continue;
            }

            let r1 = &individuals[indices[0]];
            let r2 = &individuals[indices[1]];
            let r3 = &individuals[indices[2]];

            // Create mutant via differential
            let mutant = differential_mutation(&r1.genome, &r2.genome, &r3.genome, f);

            // Crossover with target
            let trial = if rand_f64() < cr {
                differential_crossover(&target.genome, &mutant, cr)
            } else {
                target.genome.clone()
            };

            let trial_fitness = fitness_fn(&trial);
            self.stats.evaluations += 1;

            // Selection
            let target_fitness = target.fitness.as_ref().map(|f| f.scalar).unwrap_or(0.0);

            if trial_fitness.scalar >= target_fitness {
                next_gen.push(Individual {
                    id: GenomeId(rand_u64()),
                    genome: trial,
                    fitness: Some(trial_fitness),
                    species: None,
                    generation: Generation(self.generation.0 + 1),
                    parents: vec![target.id],
                    lineage: super::Lineage::default(),
                });
            } else {
                next_gen.push(target.clone());
            }
        }

        population.replace(next_gen);
        self.generation = Generation(self.generation.0 + 1);

        self.compute_stats(population)
    }

    fn evolve_coevolution<F>(
        &mut self,
        population: &mut Population,
        fitness_fn: F,
        _species_count: usize,
    ) -> GenerationStats
    where
        F: Fn(&CodeGenome) -> Fitness,
    {
        // Simplified coevolution - evaluate against each other
        let individuals: Vec<Individual> = population.iter().cloned().collect();

        for individual in population.iter_mut() {
            let base_fitness = fitness_fn(&individual.genome);

            // Competitive fitness against others
            let mut competitive_score = 0.0;
            for other in &individuals {
                if other.id != individual.id {
                    let distance = individual.genome.distance(&other.genome);
                    competitive_score += distance * 0.1;
                }
            }

            let combined = Fitness::new(vec![
                base_fitness.scalar,
                competitive_score / individuals.len().max(1) as f64,
            ]);

            individual.fitness = Some(combined);
            self.stats.evaluations += 1;
        }

        // Standard generational after evaluation
        self.evolve_generational(population, fitness_fn)
    }

    fn evolve_novelty<F>(
        &mut self,
        population: &mut Population,
        fitness_fn: F,
        archive_threshold: f64,
    ) -> GenerationStats
    where
        F: Fn(&CodeGenome) -> Fitness,
    {
        // Calculate novelty as fitness
        for individual in population.iter_mut() {
            let novelty = self.calculate_novelty(&individual.genome);

            let base_fitness = fitness_fn(&individual.genome);

            // Combine novelty and fitness
            let combined = Fitness::new(vec![novelty * 0.5, base_fitness.scalar * 0.5]);
            individual.fitness = Some(combined);
            self.stats.evaluations += 1;

            // Add to archive if novel enough
            if novelty > archive_threshold {
                self.novelty_archive.push(individual.genome.clone());
            }
        }

        // Limit archive size
        while self.novelty_archive.len() > 500 {
            self.novelty_archive.remove(0);
        }

        self.evolve_generational(population, fitness_fn)
    }

    fn calculate_novelty(&self, genome: &CodeGenome) -> f64 {
        if self.novelty_archive.is_empty() {
            return 1.0;
        }

        let distances: Vec<f64> = self
            .novelty_archive
            .iter()
            .map(|g| genome.distance(g))
            .collect();

        // Average distance to k nearest neighbors
        let k = 15.min(distances.len());
        let mut sorted = distances;
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));

        sorted.iter().take(k).sum::<f64>() / k as f64
    }

    fn evolve_map_elites<F>(
        &mut self,
        population: &mut Population,
        fitness_fn: F,
        grid_dims: Vec<usize>,
    ) -> GenerationStats
    where
        F: Fn(&CodeGenome) -> Fitness,
    {
        // Evaluate and place in grid
        for individual in population.iter_mut() {
            let fitness = fitness_fn(&individual.genome);
            individual.fitness = Some(fitness.clone());
            self.stats.evaluations += 1;

            // Map to grid cell
            let cell = self.map_to_cell(&individual.genome, &grid_dims);

            // Check if cell is empty or new individual is better
            let should_insert = match self.map_elites_grid.get(&cell) {
                None => true,
                Some(existing) => existing
                    .fitness
                    .as_ref()
                    .map(|f| fitness.scalar > f.scalar)
                    .unwrap_or(true),
            };

            if should_insert {
                self.map_elites_grid.insert(cell, individual.clone());
            }
        }

        // Generate new individuals from grid
        let grid_individuals: Vec<Individual> = self.map_elites_grid.values().cloned().collect();

        let mut offspring = Vec::new();

        while offspring.len() < self.config.population_size {
            if grid_individuals.is_empty() {
                break;
            }

            let parent1 = &grid_individuals[rand_usize(grid_individuals.len())];
            let parent2 = &grid_individuals[rand_usize(grid_individuals.len())];

            let child_genome = self.crossover.crossover(
                &parent1.genome,
                &parent2.genome,
                parent1.fitness.as_ref(),
                parent2.fitness.as_ref(),
            );

            let mutated = self.mutation.mutate(&child_genome);

            offspring.push(Individual {
                id: GenomeId(rand_u64()),
                genome: mutated,
                fitness: None,
                species: None,
                generation: Generation(self.generation.0 + 1),
                parents: vec![parent1.id, parent2.id],
                lineage: super::Lineage::default(),
            });
        }

        population.replace(offspring);
        self.generation = Generation(self.generation.0 + 1);

        self.compute_stats(population)
    }

    fn map_to_cell(&self, genome: &CodeGenome, grid_dims: &[usize]) -> Vec<usize> {
        // Map genome features to grid coordinates
        let features = vec![
            genome.complexity,
            genome.size() as f64,
            genome.active_size() as f64,
        ];

        features
            .iter()
            .zip(grid_dims.iter())
            .map(|(f, &dim)| {
                let normalized = (f / 100.0).clamp(0.0, 0.999);
                (normalized * dim as f64) as usize
            })
            .collect()
    }

    fn compute_stats(&self, population: &Population) -> GenerationStats {
        let metrics = population.calculate_stats();

        GenerationStats {
            generation: self.generation,
            best_fitness: metrics.max_fitness,
            avg_fitness: metrics.mean_fitness,
            worst_fitness: metrics.min_fitness,
            diversity: metrics.diversity,
            species_count: 0,
        }
    }

    /// Get current generation
    pub fn generation(&self) -> Generation {
        self.generation
    }

    /// Get statistics
    pub fn stats(&self) -> &EvolutionStats {
        &self.stats
    }
}

/// Differential mutation
fn differential_mutation(r1: &CodeGenome, r2: &CodeGenome, r3: &CodeGenome, f: f64) -> CodeGenome {
    let mut result = r1.clone();

    // Modify genes based on difference between r2 and r3
    for (i, gene) in result.genes.iter_mut().enumerate() {
        if i < r2.genes.len() && i < r3.genes.len() {
            let diff = r2.genes[i].expression - r3.genes[i].expression;
            gene.expression = (gene.expression + f * diff).clamp(0.0, 1.0);
        }
    }

    result
}

/// Differential crossover
fn differential_crossover(target: &CodeGenome, mutant: &CodeGenome, cr: f64) -> CodeGenome {
    let mut result = target.clone();

    for (i, gene) in result.genes.iter_mut().enumerate() {
        if rand_f64() < cr && i < mutant.genes.len() {
            *gene = mutant.genes[i].clone();
        }
    }

    result
}

/// Shuffle slice
fn shuffle<T>(slice: &mut [T]) {
    for i in (1..slice.len()).rev() {
        let j = rand_usize(i + 1);
        slice.swap(i, j);
    }
}

// ============================================================================
// RANDOM HELPERS
// ============================================================================

static mut EVOLVE_SEED: u64 = 46802;

fn rand_u64() -> u64 {
    unsafe {
        EVOLVE_SEED = EVOLVE_SEED
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        EVOLVE_SEED
    }
}

fn rand_f64() -> f64 {
    (rand_u64() as f64) / (u64::MAX as f64)
}
fn rand_usize(max: usize) -> usize {
    if max == 0 {
        0
    } else {
        (rand_u64() as usize) % max
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_creation() {
        let config = EvolutionConfig::default();
        let controller = EvolutionController::new(EvolutionStrategy::Generational, config);
        assert_eq!(controller.generation().0, 0);
    }
}
