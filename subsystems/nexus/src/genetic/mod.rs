//! # NEXUS Genetic Algorithm Engine
//!
//! Year 3 EVOLUTION - Genetic algorithms for code evolution
//! Evolves code genomes through mutation, crossover, and selection.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    GENETIC ALGORITHM ENGINE                     │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────┐  │
//! │  │   Genome    │───▶│ Population  │───▶│     Evolution       │  │
//! │  │  Encoding   │    │  Manager    │    │       Loop          │  │
//! │  └─────────────┘    └─────────────┘    └─────────────────────┘  │
//! │         │                  │                     │              │
//! │         ▼                  ▼                     ▼              │
//! │  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────┐  │
//! │  │  Mutation   │───▶│  Crossover  │───▶│     Selection       │  │
//! │  │  Operators  │    │  Operators  │    │     Strategies      │  │
//! │  └─────────────┘    └─────────────┘    └─────────────────────┘  │
//! │         │                  │                     │              │
//! │         ▼                  ▼                     ▼              │
//! │  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────┐  │
//! │  │  Fitness    │───▶│   Island    │───▶│     Speciation      │  │
//! │  │  Functions  │    │   Model     │    │      (NEAT)         │  │
//! │  └─────────────┘    └─────────────┘    └─────────────────────┘  │
//! │                                                                 │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - `genome`: Code genome representation and encoding
//! - `fitness`: Multi-objective fitness functions
//! - `mutation`: Mutation operators for code modification
//! - `crossover`: Crossover operators for code recombination
//! - `population`: Population management and diversity
//! - `evolve`: Main evolution loop and strategies
//! - `island`: Island model for parallel evolution
//! - `speciation`: NEAT-style speciation for complexity growth
//! - `niching`: Niching and crowding for diversity
//! - `elitism`: Elite preservation strategies

#![allow(dead_code)]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

pub mod crossover;
pub mod elitism;
pub mod evolve;
pub mod fitness;
pub mod genome;
pub mod island;
pub mod mutation;
pub mod niching;
pub mod population;
pub mod speciation;

// ============================================================================
// CORE TYPES
// ============================================================================

/// Genome ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GenomeId(pub u64);

/// Species ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpeciesId(pub u64);

/// Island ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IslandId(pub u64);

/// Generation number
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Generation(pub u64);

/// Fitness value (multiple objectives)
#[derive(Debug, Clone)]
pub struct Fitness {
    /// Objective values
    pub objectives: Vec<f64>,
    /// Aggregated scalar fitness
    pub scalar: f64,
    /// Pareto rank (for multi-objective)
    pub pareto_rank: u32,
    /// Crowding distance
    pub crowding_distance: f64,
}

impl Fitness {
    pub fn new(objectives: Vec<f64>) -> Self {
        let scalar = objectives.iter().sum::<f64>() / objectives.len().max(1) as f64;
        Self {
            objectives,
            scalar,
            pareto_rank: 0,
            crowding_distance: 0.0,
        }
    }

    pub fn dominates(&self, other: &Fitness) -> bool {
        let dominated = self
            .objectives
            .iter()
            .zip(other.objectives.iter())
            .all(|(a, b)| a >= b);
        let strictly = self
            .objectives
            .iter()
            .zip(other.objectives.iter())
            .any(|(a, b)| a > b);
        dominated && strictly
    }
}

/// Individual in the population
#[derive(Debug, Clone)]
pub struct Individual {
    /// Unique ID
    pub id: GenomeId,
    /// Genome data
    pub genome: genome::CodeGenome,
    /// Fitness
    pub fitness: Option<Fitness>,
    /// Species
    pub species: Option<SpeciesId>,
    /// Generation born
    pub generation: Generation,
    /// Parent IDs
    pub parents: Vec<GenomeId>,
    /// Lineage info
    pub lineage: Lineage,
}

/// Lineage tracking
#[derive(Debug, Clone, Default)]
pub struct Lineage {
    /// Ancestor chain
    pub ancestors: Vec<GenomeId>,
    /// Number of mutations from original
    pub mutations: u64,
    /// Number of crossovers from original
    pub crossovers: u64,
}

/// Evolution configuration
#[derive(Debug, Clone)]
pub struct EvolutionConfig {
    /// Population size
    pub population_size: usize,
    /// Elite count
    pub elite_count: usize,
    /// Mutation rate
    pub mutation_rate: f64,
    /// Crossover rate
    pub crossover_rate: f64,
    /// Maximum generations
    pub max_generations: u64,
    /// Fitness threshold (stop if reached)
    pub fitness_threshold: Option<f64>,
    /// Selection strategy
    pub selection: SelectionStrategy,
    /// Enable speciation
    pub enable_speciation: bool,
    /// Enable islands
    pub enable_islands: bool,
    /// Island count
    pub island_count: usize,
    /// Migration rate
    pub migration_rate: f64,
    /// Stagnation threshold (generations without improvement)
    pub stagnation_threshold: u64,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            population_size: 100,
            elite_count: 5,
            mutation_rate: 0.3,
            crossover_rate: 0.7,
            max_generations: 1000,
            fitness_threshold: Some(0.99),
            selection: SelectionStrategy::Tournament { size: 3 },
            enable_speciation: true,
            enable_islands: false,
            island_count: 4,
            migration_rate: 0.01,
            stagnation_threshold: 50,
        }
    }
}

/// Selection strategy
#[derive(Debug, Clone)]
pub enum SelectionStrategy {
    /// Roulette wheel selection
    Roulette,
    /// Tournament selection
    Tournament { size: usize },
    /// Rank selection
    Rank,
    /// Truncation selection
    Truncation { ratio: f64 },
    /// NSGA-II (multi-objective)
    NSGAII,
    /// Lexicase selection
    Lexicase,
}

// ============================================================================
// GENETIC ALGORITHM ENGINE
// ============================================================================

/// Main genetic algorithm engine
pub struct GeneticEngine {
    /// Configuration
    config: EvolutionConfig,
    /// Current population
    population: population::Population,
    /// Current generation
    generation: Generation,
    /// Best individual ever seen
    best_ever: Option<Individual>,
    /// Species manager
    species_manager: speciation::SpeciesManager,
    /// Island manager
    island_manager: island::IslandManager,
    /// Statistics
    stats: EvolutionStats,
    /// Next ID
    next_id: AtomicU64,
    /// History
    history: Vec<GenerationStats>,
}

/// Evolution statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct EvolutionStats {
    /// Total evaluations
    pub evaluations: u64,
    /// Total mutations
    pub mutations: u64,
    /// Total crossovers
    pub crossovers: u64,
    /// Successful improvements
    pub improvements: u64,
    /// Generations stagnant
    pub stagnant_generations: u64,
}

/// Per-generation statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct GenerationStats {
    /// Generation number
    pub generation: Generation,
    /// Best fitness
    pub best_fitness: f64,
    /// Average fitness
    pub avg_fitness: f64,
    /// Worst fitness
    pub worst_fitness: f64,
    /// Population diversity
    pub diversity: f64,
    /// Species count
    pub species_count: usize,
}

impl GeneticEngine {
    /// Create new engine
    pub fn new(config: EvolutionConfig) -> Self {
        Self {
            population: population::Population::new(config.population_size),
            generation: Generation(0),
            best_ever: None,
            species_manager: speciation::SpeciesManager::new(
                speciation::SpeciationConfig::default(),
            ),
            island_manager: island::IslandManager::new(
                config.island_count,
                island::IslandConfig::default(),
            ),
            stats: EvolutionStats::default(),
            next_id: AtomicU64::new(1),
            history: Vec::new(),
            config,
        }
    }

    /// Initialize population
    pub fn initialize<F>(&mut self, generator: F)
    where
        F: Fn() -> genome::CodeGenome,
    {
        for _ in 0..self.config.population_size {
            let genome = generator();
            let individual = Individual {
                id: GenomeId(self.next_id.fetch_add(1, Ordering::Relaxed)),
                genome,
                fitness: None,
                species: None,
                generation: Generation(0),
                parents: Vec::new(),
                lineage: Lineage::default(),
            };
            self.population.add(individual);
        }
    }

    /// Evaluate population
    pub fn evaluate<F>(&mut self, fitness_fn: F)
    where
        F: Fn(&genome::CodeGenome) -> Fitness,
    {
        for individual in self.population.iter_mut() {
            if individual.fitness.is_none() {
                let fitness = fitness_fn(&individual.genome);
                individual.fitness = Some(fitness);
                self.stats.evaluations += 1;
            }
        }

        // Update best ever
        if let Some(best) = self.population.best() {
            if let Some(best_fitness) = &best.fitness {
                let dominated = self
                    .best_ever
                    .as_ref()
                    .and_then(|b| b.fitness.as_ref())
                    .map(|f| best_fitness.scalar > f.scalar)
                    .unwrap_or(true);

                if dominated {
                    self.best_ever = Some(best.clone());
                    self.stats.stagnant_generations = 0;
                } else {
                    self.stats.stagnant_generations += 1;
                }
            }
        }
    }

    /// Evolve one generation
    pub fn evolve_generation<F>(&mut self, fitness_fn: F)
    where
        F: Fn(&genome::CodeGenome) -> Fitness + Clone,
    {
        // Evaluate current population
        self.evaluate(fitness_fn.clone());

        // Speciate if enabled
        if self.config.enable_speciation {
            self.species_manager.speciate(&mut self.population);
        }

        // Record stats
        self.record_stats();

        // Select parents
        let parents = self.select_parents();

        // Create offspring
        let mut offspring = Vec::new();

        // Elite preservation
        let elites = self.population.elites(self.config.elite_count);
        offspring.extend(elites);

        // Generate rest of population
        while offspring.len() < self.config.population_size {
            let (p1, p2) = self.choose_parents(&parents);

            // Extract data from borrowed parents before modifying self.stats
            let p1_genome = p1.genome.clone();
            let p2_genome = p2.genome.clone();
            let p1_id = p1.id;
            let p2_id = p2.id;
            let p1_lineage = p1.lineage.clone();

            let do_crossover = rand_f64() < self.config.crossover_rate;
            let child_genome = if do_crossover {
                // Crossover
                self.stats.crossovers += 1;
                crossover::uniform_crossover(&p1_genome, &p2_genome)
            } else {
                // Clone one parent
                p1_genome
            };

            // Mutation
            let do_mutation = rand_f64() < self.config.mutation_rate;
            let mutated = if do_mutation {
                self.stats.mutations += 1;
                mutation::point_mutation(&child_genome)
            } else {
                child_genome
            };

            let mut lineage = p1_lineage;
            lineage.ancestors.push(p1_id);
            if p1_id != p2_id {
                lineage.ancestors.push(p2_id);
                lineage.crossovers += 1;
            }
            if rand_f64() < self.config.mutation_rate {
                lineage.mutations += 1;
            }

            let child = Individual {
                id: GenomeId(self.next_id.fetch_add(1, Ordering::Relaxed)),
                genome: mutated,
                fitness: None,
                species: None,
                generation: Generation(self.generation.0 + 1),
                parents: vec![p1_id, p2_id],
                lineage,
            };

            offspring.push(child);
        }

        // Replace population
        self.population.replace(offspring);
        self.generation = Generation(self.generation.0 + 1);

        // Handle islands if enabled
        if self.config.enable_islands {
            self.island_manager
                .maybe_migrate(&mut self.population, self.config.migration_rate);
        }
    }

    fn select_parents(&self) -> Vec<Individual> {
        match &self.config.selection {
            SelectionStrategy::Tournament { size } => self
                .population
                .tournament_selection(*size, self.config.population_size),
            SelectionStrategy::Roulette => self
                .population
                .roulette_selection(self.config.population_size),
            SelectionStrategy::Rank => self.population.rank_selection(self.config.population_size),
            SelectionStrategy::Truncation { ratio } => self.population.truncation_selection(*ratio),
            SelectionStrategy::NSGAII => {
                self.population.nsga2_selection(self.config.population_size)
            },
            SelectionStrategy::Lexicase => self
                .population
                .lexicase_selection(self.config.population_size),
        }
    }

    fn choose_parents<'a>(&self, parents: &'a [Individual]) -> (&'a Individual, &'a Individual) {
        let idx1 = rand_usize(parents.len());
        let mut idx2 = rand_usize(parents.len());
        while idx2 == idx1 && parents.len() > 1 {
            idx2 = rand_usize(parents.len());
        }
        (&parents[idx1], &parents[idx2])
    }

    fn record_stats(&mut self) {
        let fitnesses: Vec<f64> = self
            .population
            .iter()
            .filter_map(|i| i.fitness.as_ref())
            .map(|f| f.scalar)
            .collect();

        if fitnesses.is_empty() {
            return;
        }

        let best = fitnesses.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let worst = fitnesses.iter().cloned().fold(f64::INFINITY, f64::min);
        let avg = fitnesses.iter().sum::<f64>() / fitnesses.len() as f64;

        let stats = GenerationStats {
            generation: self.generation,
            best_fitness: best,
            avg_fitness: avg,
            worst_fitness: worst,
            diversity: self.calculate_diversity(),
            species_count: self.species_manager.species_count(),
        };

        self.history.push(stats);
    }

    fn calculate_diversity(&self) -> f64 {
        // Simplified diversity measure
        let n = self.population.len();
        if n < 2 {
            return 0.0;
        }

        let mut total_distance = 0.0;
        let mut count = 0;

        let individuals: Vec<_> = self.population.iter().collect();
        for i in 0..individuals.len() {
            for j in (i + 1)..individuals.len() {
                total_distance += individuals[i].genome.distance(&individuals[j].genome);
                count += 1;
            }
        }

        if count > 0 {
            total_distance / count as f64
        } else {
            0.0
        }
    }

    /// Run evolution until termination
    #[inline]
    pub fn run<F>(&mut self, fitness_fn: F) -> Option<Individual>
    where
        F: Fn(&genome::CodeGenome) -> Fitness + Clone,
    {
        while !self.should_terminate() {
            self.evolve_generation(fitness_fn.clone());
        }

        self.best_ever.clone()
    }

    fn should_terminate(&self) -> bool {
        // Max generations reached
        if self.generation.0 >= self.config.max_generations {
            return true;
        }

        // Fitness threshold reached
        if let Some(threshold) = self.config.fitness_threshold {
            if let Some(best) = &self.best_ever {
                if let Some(fitness) = &best.fitness {
                    if fitness.scalar >= threshold {
                        return true;
                    }
                }
            }
        }

        // Stagnation
        if self.stats.stagnant_generations >= self.config.stagnation_threshold {
            return true;
        }

        false
    }

    /// Get best individual
    #[inline(always)]
    pub fn best(&self) -> Option<&Individual> {
        self.best_ever.as_ref()
    }

    /// Get current generation
    #[inline(always)]
    pub fn generation(&self) -> Generation {
        self.generation
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &EvolutionStats {
        &self.stats
    }

    /// Get history
    #[inline(always)]
    pub fn history(&self) -> &[GenerationStats] {
        &self.history
    }

    /// Get population
    #[inline(always)]
    pub fn population(&self) -> &population::Population {
        &self.population
    }
}

// ============================================================================
// RANDOM HELPERS (simplified for no_std)
// ============================================================================

static SEED: AtomicU64 = AtomicU64::new(12345);

fn rand_u64() -> u64 {
    let mut current = SEED.load(Ordering::Relaxed);
    loop {
        let next = current.wrapping_mul(6364136223846793005).wrapping_add(1);
        match SEED.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return next,
            Err(x) => current = x,
        }
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
    fn test_fitness_dominance() {
        let f1 = Fitness::new(vec![1.0, 2.0, 3.0]);
        let f2 = Fitness::new(vec![0.5, 1.0, 2.0]);
        assert!(f1.dominates(&f2));
        assert!(!f2.dominates(&f1));
    }

    #[test]
    fn test_engine_creation() {
        let config = EvolutionConfig::default();
        let engine = GeneticEngine::new(config);
        assert_eq!(engine.generation().0, 0);
    }
}
