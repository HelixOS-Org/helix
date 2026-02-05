//! # Population Management
//!
//! Year 3 EVOLUTION - Population management and selection
//! Manages individuals and implements selection strategies.

#![allow(dead_code)]

extern crate alloc;
use alloc::vec::Vec;

use super::{Fitness, GenomeId, Individual};
use crate::math::F64Ext;

// ============================================================================
// POPULATION
// ============================================================================

/// Population of individuals
#[derive(Debug, Clone)]
pub struct Population {
    /// Individuals
    individuals: Vec<Individual>,
    /// Capacity
    capacity: usize,
    /// Statistics
    stats: PopulationStats,
}

/// Population statistics
#[derive(Debug, Clone, Default)]
pub struct PopulationStats {
    /// Total individuals added
    pub total_added: u64,
    /// Total individuals removed
    pub total_removed: u64,
    /// Peak size
    pub peak_size: usize,
}

impl Population {
    /// Create new population
    pub fn new(capacity: usize) -> Self {
        Self {
            individuals: Vec::with_capacity(capacity),
            capacity,
            stats: PopulationStats::default(),
        }
    }

    /// Add individual
    pub fn add(&mut self, individual: Individual) {
        self.individuals.push(individual);
        self.stats.total_added += 1;
        self.stats.peak_size = self.stats.peak_size.max(self.individuals.len());
    }

    /// Remove individual by ID
    pub fn remove(&mut self, id: GenomeId) -> Option<Individual> {
        if let Some(idx) = self.individuals.iter().position(|i| i.id == id) {
            self.stats.total_removed += 1;
            Some(self.individuals.remove(idx))
        } else {
            None
        }
    }

    /// Get individual by ID
    pub fn get(&self, id: GenomeId) -> Option<&Individual> {
        self.individuals.iter().find(|i| i.id == id)
    }

    /// Get individual by ID (mutable)
    pub fn get_mut(&mut self, id: GenomeId) -> Option<&mut Individual> {
        self.individuals.iter_mut().find(|i| i.id == id)
    }

    /// Get best individual
    pub fn best(&self) -> Option<&Individual> {
        self.individuals
            .iter()
            .filter(|i| i.fitness.is_some())
            .max_by(|a, b| {
                let fa = a.fitness.as_ref().unwrap().scalar;
                let fb = b.fitness.as_ref().unwrap().scalar;
                fa.partial_cmp(&fb).unwrap_or(core::cmp::Ordering::Equal)
            })
    }

    /// Get worst individual
    pub fn worst(&self) -> Option<&Individual> {
        self.individuals
            .iter()
            .filter(|i| i.fitness.is_some())
            .min_by(|a, b| {
                let fa = a.fitness.as_ref().unwrap().scalar;
                let fb = b.fitness.as_ref().unwrap().scalar;
                fa.partial_cmp(&fb).unwrap_or(core::cmp::Ordering::Equal)
            })
    }

    /// Get top N individuals
    pub fn elites(&self, n: usize) -> Vec<Individual> {
        let mut sorted: Vec<_> = self
            .individuals
            .iter()
            .filter(|i| i.fitness.is_some())
            .collect();

        sorted.sort_by(|a, b| {
            let fa = a.fitness.as_ref().unwrap().scalar;
            let fb = b.fitness.as_ref().unwrap().scalar;
            fb.partial_cmp(&fa).unwrap_or(core::cmp::Ordering::Equal)
        });

        sorted.into_iter().take(n).cloned().collect()
    }

    /// Replace population with new individuals
    pub fn replace(&mut self, new_individuals: Vec<Individual>) {
        self.stats.total_removed += self.individuals.len() as u64;
        self.individuals = new_individuals;
        self.stats.total_added += self.individuals.len() as u64;
        self.stats.peak_size = self.stats.peak_size.max(self.individuals.len());
    }

    /// Get population size
    pub fn len(&self) -> usize {
        self.individuals.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.individuals.is_empty()
    }

    /// Iterator over individuals
    pub fn iter(&self) -> impl Iterator<Item = &Individual> {
        self.individuals.iter()
    }

    /// Mutable iterator
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Individual> {
        self.individuals.iter_mut()
    }

    // ========================================================================
    // SELECTION STRATEGIES
    // ========================================================================

    /// Tournament selection
    pub fn tournament_selection(&self, tournament_size: usize, count: usize) -> Vec<Individual> {
        let mut selected = Vec::with_capacity(count);

        for _ in 0..count {
            let mut best: Option<&Individual> = None;

            for _ in 0..tournament_size {
                let idx = rand_usize(self.individuals.len());
                let candidate = &self.individuals[idx];

                let is_better = match (&best, &candidate.fitness) {
                    (None, Some(_)) => true,
                    (Some(b), Some(cf)) => b
                        .fitness
                        .as_ref()
                        .map(|bf| cf.scalar > bf.scalar)
                        .unwrap_or(true),
                    _ => false,
                };

                if is_better {
                    best = Some(candidate);
                }
            }

            if let Some(winner) = best {
                selected.push(winner.clone());
            }
        }

        selected
    }

    /// Roulette wheel selection
    pub fn roulette_selection(&self, count: usize) -> Vec<Individual> {
        let fitnesses: Vec<f64> = self
            .individuals
            .iter()
            .map(|i| i.fitness.as_ref().map(|f| f.scalar).unwrap_or(0.0))
            .collect();

        let min_fitness = fitnesses.iter().cloned().fold(f64::INFINITY, f64::min);
        let adjusted: Vec<f64> = fitnesses.iter().map(|f| f - min_fitness + 1.0).collect();

        let total: f64 = adjusted.iter().sum();

        let mut selected = Vec::with_capacity(count);

        for _ in 0..count {
            let mut target = rand_f64() * total;

            for (i, &fitness) in adjusted.iter().enumerate() {
                target -= fitness;
                if target <= 0.0 {
                    selected.push(self.individuals[i].clone());
                    break;
                }
            }

            if selected.len() < count {
                // Fallback if we didn't select
                selected.push(self.individuals[rand_usize(self.individuals.len())].clone());
            }
        }

        selected
    }

    /// Rank selection
    pub fn rank_selection(&self, count: usize) -> Vec<Individual> {
        let mut indexed: Vec<(usize, &Individual)> = self.individuals.iter().enumerate().collect();

        indexed.sort_by(|(_, a), (_, b)| {
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

        let n = indexed.len() as f64;
        let total_rank: f64 = (n * (n + 1.0)) / 2.0;

        let mut selected = Vec::with_capacity(count);

        for _ in 0..count {
            let mut target = rand_f64() * total_rank;

            for (rank, (_, individual)) in indexed.iter().enumerate() {
                let rank_value = n - rank as f64;
                target -= rank_value;
                if target <= 0.0 {
                    selected.push((*individual).clone());
                    break;
                }
            }
        }

        selected
    }

    /// Truncation selection
    pub fn truncation_selection(&self, ratio: f64) -> Vec<Individual> {
        let cutoff = (self.individuals.len() as f64 * ratio).ceil() as usize;
        self.elites(cutoff)
    }

    /// NSGA-II selection
    pub fn nsga2_selection(&self, count: usize) -> Vec<Individual> {
        // Get fitness values
        let fitnesses: Vec<Fitness> = self
            .individuals
            .iter()
            .filter_map(|i| i.fitness.clone())
            .collect();

        if fitnesses.is_empty() {
            return self.tournament_selection(3, count);
        }

        // Calculate Pareto fronts
        let fronts = super::fitness::pareto_fronts(&fitnesses);

        let mut selected = Vec::with_capacity(count);
        let mut fitness_idx = 0;

        for front in &fronts {
            if selected.len() >= count {
                break;
            }

            // Calculate crowding distances for this front
            let distances = super::fitness::crowding_distance(&fitnesses, front);

            // Sort by crowding distance (descending)
            let mut front_with_distance: Vec<(usize, f64)> = front
                .iter()
                .enumerate()
                .map(|(i, &idx)| (idx, distances[i]))
                .collect();

            front_with_distance
                .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));

            for (idx, _) in front_with_distance {
                if selected.len() >= count {
                    break;
                }
                // Find individual with this fitness
                if let Some(individual) = self
                    .individuals
                    .iter()
                    .filter(|i| i.fitness.is_some())
                    .nth(idx)
                {
                    selected.push(individual.clone());
                }
                fitness_idx += 1;
            }
        }

        // Suppress unused warning
        let _ = fitness_idx;

        selected
    }

    /// Lexicase selection
    pub fn lexicase_selection(&self, count: usize) -> Vec<Individual> {
        let mut selected = Vec::with_capacity(count);

        // Get objective count
        let obj_count = self
            .individuals
            .iter()
            .filter_map(|i| i.fitness.as_ref())
            .map(|f| f.objectives.len())
            .max()
            .unwrap_or(0);

        if obj_count == 0 {
            return self.tournament_selection(3, count);
        }

        for _ in 0..count {
            // Shuffle objectives
            let mut objectives: Vec<usize> = (0..obj_count).collect();
            shuffle(&mut objectives);

            let mut candidates: Vec<&Individual> = self
                .individuals
                .iter()
                .filter(|i| i.fitness.is_some())
                .collect();

            for obj in objectives {
                if candidates.len() <= 1 {
                    break;
                }

                // Find best value for this objective
                let best_val = candidates
                    .iter()
                    .filter_map(|i| i.fitness.as_ref())
                    .map(|f| f.objectives.get(obj).copied().unwrap_or(0.0))
                    .fold(f64::NEG_INFINITY, f64::max);

                // Keep only individuals with best value (within tolerance)
                let tolerance = 0.001;
                candidates.retain(|i| {
                    i.fitness
                        .as_ref()
                        .and_then(|f| f.objectives.get(obj))
                        .map(|&v| (v - best_val).abs() < tolerance)
                        .unwrap_or(false)
                });
            }

            if let Some(winner) = candidates.first() {
                selected.push((*winner).clone());
            } else if !self.individuals.is_empty() {
                selected.push(self.individuals[rand_usize(self.individuals.len())].clone());
            }
        }

        selected
    }

    /// Get statistics
    pub fn stats(&self) -> &PopulationStats {
        &self.stats
    }

    /// Calculate population statistics
    pub fn calculate_stats(&self) -> PopulationMetrics {
        let fitnesses: Vec<f64> = self
            .individuals
            .iter()
            .filter_map(|i| i.fitness.as_ref())
            .map(|f| f.scalar)
            .collect();

        if fitnesses.is_empty() {
            return PopulationMetrics::default();
        }

        let sum: f64 = fitnesses.iter().sum();
        let mean = sum / fitnesses.len() as f64;

        let variance =
            fitnesses.iter().map(|f| (f - mean).powi(2)).sum::<f64>() / fitnesses.len() as f64;

        let std_dev = variance.sqrt();

        let min = fitnesses.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = fitnesses.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        PopulationMetrics {
            size: self.individuals.len(),
            mean_fitness: mean,
            std_dev_fitness: std_dev,
            min_fitness: min,
            max_fitness: max,
            diversity: self.calculate_diversity(),
        }
    }

    fn calculate_diversity(&self) -> f64 {
        if self.individuals.len() < 2 {
            return 0.0;
        }

        let mut total_distance = 0.0;
        let mut count = 0;

        for i in 0..self.individuals.len() {
            for j in (i + 1)..self.individuals.len() {
                total_distance += self.individuals[i]
                    .genome
                    .distance(&self.individuals[j].genome);
                count += 1;
            }
        }

        if count > 0 {
            total_distance / count as f64
        } else {
            0.0
        }
    }
}

/// Population metrics
#[derive(Debug, Clone, Default)]
pub struct PopulationMetrics {
    pub size: usize,
    pub mean_fitness: f64,
    pub std_dev_fitness: f64,
    pub min_fitness: f64,
    pub max_fitness: f64,
    pub diversity: f64,
}

/// Shuffle a slice
fn shuffle<T>(slice: &mut [T]) {
    for i in (1..slice.len()).rev() {
        let j = rand_usize(i + 1);
        slice.swap(i, j);
    }
}

// ============================================================================
// RANDOM HELPERS
// ============================================================================

use core::sync::atomic::{AtomicU64, Ordering};

static POPULATION_SEED: AtomicU64 = AtomicU64::new(35791);

fn rand_u64() -> u64 {
    let mut current = POPULATION_SEED.load(Ordering::Relaxed);
    loop {
        let next = current.wrapping_mul(6364136223846793005).wrapping_add(1);
        match POPULATION_SEED.compare_exchange_weak(
            current,
            next,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
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
    use core::sync::atomic::AtomicU64;

    use super::*;

    fn create_test_individual(id: u64, fitness: f64) -> Individual {
        let counter = AtomicU64::new(1);
        Individual {
            id: GenomeId(id),
            genome: CodeGenome::random(id, 5, &counter),
            fitness: Some(Fitness::new(vec![fitness])),
            species: None,
            generation: Generation(0),
            parents: Vec::new(),
            lineage: super::super::Lineage::default(),
        }
    }

    #[test]
    fn test_population_creation() {
        let pop = Population::new(100);
        assert_eq!(pop.len(), 0);
        assert_eq!(pop.capacity, 100);
    }

    #[test]
    fn test_add_and_get() {
        let mut pop = Population::new(100);
        let ind = create_test_individual(1, 0.5);
        pop.add(ind);

        assert_eq!(pop.len(), 1);
        assert!(pop.get(GenomeId(1)).is_some());
    }

    #[test]
    fn test_best_worst() {
        let mut pop = Population::new(100);
        pop.add(create_test_individual(1, 0.3));
        pop.add(create_test_individual(2, 0.7));
        pop.add(create_test_individual(3, 0.5));

        let best = pop.best().unwrap();
        assert_eq!(best.id.0, 2);

        let worst = pop.worst().unwrap();
        assert_eq!(worst.id.0, 1);
    }

    #[test]
    fn test_tournament_selection() {
        let mut pop = Population::new(100);
        for i in 0..10 {
            pop.add(create_test_individual(i, (i as f64) / 10.0));
        }

        let selected = pop.tournament_selection(3, 5);
        assert_eq!(selected.len(), 5);
    }

    #[test]
    fn test_roulette_selection() {
        let mut pop = Population::new(100);
        for i in 0..10 {
            pop.add(create_test_individual(i, (i as f64) / 10.0 + 0.1));
        }

        let selected = pop.roulette_selection(5);
        assert_eq!(selected.len(), 5);
    }
}
