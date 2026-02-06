//! # Elitism
//!
//! Year 3 EVOLUTION - Elite preservation strategies
//! Preserves best solutions across generations.

#![allow(dead_code)]

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::population::Population;
use super::{Fitness, Generation, GenomeId, Individual, SpeciesId};
use crate::math::F64Ext;

// ============================================================================
// ELITISM TYPES
// ============================================================================

/// Elitism strategy
#[derive(Debug, Clone)]
pub enum ElitismStrategy {
    /// Preserve top N individuals
    TopN { count: usize },
    /// Preserve percentage of population
    Percentage { ratio: f64 },
    /// Preserve best per species
    PerSpecies { count_per_species: usize },
    /// Preserve Pareto front
    ParetoFront,
    /// Preserve based on novelty
    Novelty { count: usize },
    /// Hall of Fame
    HallOfFame { capacity: usize },
    /// Adaptive elitism
    Adaptive { min: usize, max: usize },
}

impl Default for ElitismStrategy {
    fn default() -> Self {
        Self::TopN { count: 5 }
    }
}

/// Elite individual
#[derive(Debug, Clone)]
pub struct Elite {
    /// Individual
    pub individual: Individual,
    /// Generation added
    pub generation_added: Generation,
    /// Contribution count (times selected as parent)
    pub contribution_count: u64,
    /// Rank in hall of fame
    pub rank: usize,
}

/// Elitism configuration
#[derive(Debug, Clone)]
pub struct ElitismConfig {
    /// Strategy
    pub strategy: ElitismStrategy,
    /// Preserve elite fitness exactly (no mutation)
    pub preserve_fitness: bool,
    /// Maximum elite age
    pub max_age: Option<u64>,
    /// Decay rate for old elites
    pub age_decay: f64,
}

impl Default for ElitismConfig {
    fn default() -> Self {
        Self {
            strategy: ElitismStrategy::default(),
            preserve_fitness: true,
            max_age: None,
            age_decay: 0.0,
        }
    }
}

// ============================================================================
// HALL OF FAME
// ============================================================================

/// Hall of Fame - preserves best individuals ever seen
pub struct HallOfFame {
    /// Elite individuals
    elites: Vec<Elite>,
    /// Capacity
    capacity: usize,
    /// Current generation
    generation: Generation,
    /// Statistics
    stats: HallOfFameStats,
}

/// Hall of Fame statistics
#[derive(Debug, Clone, Default)]
pub struct HallOfFameStats {
    /// Total additions
    pub additions: u64,
    /// Total removals
    pub removals: u64,
    /// Current size
    pub size: usize,
    /// Average fitness
    pub avg_fitness: f64,
}

impl HallOfFame {
    /// Create new hall of fame
    pub fn new(capacity: usize) -> Self {
        Self {
            elites: Vec::with_capacity(capacity),
            capacity,
            generation: Generation(0),
            stats: HallOfFameStats::default(),
        }
    }

    /// Try to add individual
    pub fn try_add(&mut self, individual: &Individual, generation: Generation) -> bool {
        let fitness = match &individual.fitness {
            Some(f) => f.scalar,
            None => return false,
        };

        // Check if already in hall of fame
        if self.contains(&individual.id) {
            return false;
        }

        // Check if qualifies
        let qualifies = if self.elites.len() < self.capacity {
            true
        } else {
            let min_fitness = self
                .elites
                .iter()
                .filter_map(|e| e.individual.fitness.as_ref())
                .map(|f| f.scalar)
                .fold(f64::INFINITY, f64::min);
            fitness > min_fitness
        };

        if qualifies {
            let elite = Elite {
                individual: individual.clone(),
                generation_added: generation,
                contribution_count: 0,
                rank: 0,
            };

            self.elites.push(elite);
            self.stats.additions += 1;

            // Sort and trim
            self.elites.sort_by(|a, b| {
                let fa = a
                    .individual
                    .fitness
                    .as_ref()
                    .map(|f| f.scalar)
                    .unwrap_or(0.0);
                let fb = b
                    .individual
                    .fitness
                    .as_ref()
                    .map(|f| f.scalar)
                    .unwrap_or(0.0);
                fb.partial_cmp(&fa).unwrap_or(core::cmp::Ordering::Equal)
            });

            while self.elites.len() > self.capacity {
                self.elites.pop();
                self.stats.removals += 1;
            }

            // Update ranks
            for (i, elite) in self.elites.iter_mut().enumerate() {
                elite.rank = i + 1;
            }

            self.update_stats();
            true
        } else {
            false
        }
    }

    /// Check if individual is in hall of fame
    pub fn contains(&self, id: &GenomeId) -> bool {
        self.elites.iter().any(|e| e.individual.id == *id)
    }

    /// Get elite by rank
    pub fn get_by_rank(&self, rank: usize) -> Option<&Elite> {
        self.elites.get(rank.saturating_sub(1))
    }

    /// Get best
    pub fn best(&self) -> Option<&Elite> {
        self.elites.first()
    }

    /// Get all elites
    pub fn all(&self) -> &[Elite] {
        &self.elites
    }

    /// Record contribution
    pub fn record_contribution(&mut self, id: GenomeId) {
        if let Some(elite) = self.elites.iter_mut().find(|e| e.individual.id == id) {
            elite.contribution_count += 1;
        }
    }

    /// Age out old elites
    pub fn age_out(&mut self, current_gen: Generation, max_age: u64) {
        let before = self.elites.len();

        self.elites
            .retain(|elite| current_gen.0 - elite.generation_added.0 <= max_age);

        let removed = before - self.elites.len();
        self.stats.removals += removed as u64;
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.size = self.elites.len();

        if self.elites.is_empty() {
            self.stats.avg_fitness = 0.0;
        } else {
            let sum: f64 = self
                .elites
                .iter()
                .filter_map(|e| e.individual.fitness.as_ref())
                .map(|f| f.scalar)
                .sum();
            self.stats.avg_fitness = sum / self.elites.len() as f64;
        }
    }

    /// Get statistics
    pub fn stats(&self) -> &HallOfFameStats {
        &self.stats
    }

    /// Get size
    pub fn len(&self) -> usize {
        self.elites.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.elites.is_empty()
    }
}

// ============================================================================
// ELITISM MANAGER
// ============================================================================

/// Elitism manager
pub struct ElitismManager {
    /// Configuration
    config: ElitismConfig,
    /// Hall of fame
    hall_of_fame: HallOfFame,
    /// Current generation
    generation: Generation,
    /// Statistics
    stats: ElitismStats,
}

/// Elitism statistics
#[derive(Debug, Clone, Default)]
pub struct ElitismStats {
    /// Elites preserved
    pub elites_preserved: u64,
    /// Elites per generation
    pub elites_per_gen: Vec<usize>,
}

impl ElitismManager {
    /// Create new manager
    pub fn new(config: ElitismConfig) -> Self {
        let hof_capacity = match &config.strategy {
            ElitismStrategy::HallOfFame { capacity } => *capacity,
            _ => 100,
        };

        Self {
            config,
            hall_of_fame: HallOfFame::new(hof_capacity),
            generation: Generation(0),
            stats: ElitismStats::default(),
        }
    }

    /// Get elites from population
    pub fn get_elites(&mut self, population: &Population) -> Vec<Individual> {
        let elites = match &self.config.strategy {
            ElitismStrategy::TopN { count } => population.elites(*count),
            ElitismStrategy::Percentage { ratio } => {
                let count = (population.len() as f64 * ratio).ceil() as usize;
                population.elites(count.max(1))
            },
            ElitismStrategy::PerSpecies { count_per_species } => {
                self.get_elites_per_species(population, *count_per_species)
            },
            ElitismStrategy::ParetoFront => self.get_pareto_elites(population),
            ElitismStrategy::Novelty { count } => self.get_novelty_elites(population, *count),
            ElitismStrategy::HallOfFame { .. } => {
                // Update hall of fame and return its members
                self.update_hall_of_fame(population);
                self.hall_of_fame
                    .all()
                    .iter()
                    .map(|e| e.individual.clone())
                    .collect()
            },
            ElitismStrategy::Adaptive { min, max } => {
                self.get_adaptive_elites(population, *min, *max)
            },
        };

        self.stats.elites_preserved += elites.len() as u64;
        self.stats.elites_per_gen.push(elites.len());

        elites
    }

    fn get_elites_per_species(&self, population: &Population, count: usize) -> Vec<Individual> {
        let mut elites = Vec::new();
        let mut by_species: BTreeMap<SpeciesId, Vec<&Individual>> = BTreeMap::new();

        for individual in population.iter() {
            if let Some(species) = individual.species {
                by_species.entry(species).or_default().push(individual);
            }
        }

        for (_, mut members) in by_species {
            members.sort_by(|a, b| {
                let fa = a.fitness.as_ref().map(|f| f.scalar).unwrap_or(0.0);
                let fb = b.fitness.as_ref().map(|f| f.scalar).unwrap_or(0.0);
                fb.partial_cmp(&fa).unwrap_or(core::cmp::Ordering::Equal)
            });

            for member in members.into_iter().take(count) {
                elites.push(member.clone());
            }
        }

        elites
    }

    fn get_pareto_elites(&self, population: &Population) -> Vec<Individual> {
        let fitnesses: Vec<Fitness> = population
            .iter()
            .filter_map(|i| i.fitness.clone())
            .collect();

        if fitnesses.is_empty() {
            return Vec::new();
        }

        let fronts = super::fitness::pareto_fronts(&fitnesses);

        if fronts.is_empty() {
            return Vec::new();
        }

        // Return individuals in first front
        let first_front = &fronts[0];
        let individuals: Vec<&Individual> =
            population.iter().filter(|i| i.fitness.is_some()).collect();

        first_front
            .iter()
            .filter_map(|&idx| individuals.get(idx))
            .map(|i| (*i).clone())
            .collect()
    }

    fn get_novelty_elites(&self, population: &Population, count: usize) -> Vec<Individual> {
        let mut with_novelty: Vec<(&Individual, f64)> = population
            .iter()
            .map(|ind| {
                let novelty = population
                    .iter()
                    .filter(|other| other.id != ind.id)
                    .map(|other| ind.genome.distance(&other.genome))
                    .sum::<f64>();
                (ind, novelty)
            })
            .collect();

        with_novelty.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));

        with_novelty
            .into_iter()
            .take(count)
            .map(|(ind, _)| ind.clone())
            .collect()
    }

    fn get_adaptive_elites(
        &self,
        population: &Population,
        min: usize,
        max: usize,
    ) -> Vec<Individual> {
        // Adapt elite count based on population diversity
        let diversity = self.calculate_diversity(population);

        // More diversity = fewer elites needed
        // Less diversity = more elites to preserve
        let diversity_factor = (1.0 - diversity).clamp(0.0, 1.0);
        let count = min + ((max - min) as f64 * diversity_factor) as usize;

        population.elites(count)
    }

    fn calculate_diversity(&self, population: &Population) -> f64 {
        let individuals: Vec<&Individual> = population.iter().collect();
        let n = individuals.len();

        if n < 2 {
            return 0.0;
        }

        let mut total_distance = 0.0;
        let mut count = 0;

        for i in 0..n.min(20) {
            for j in (i + 1)..n.min(20) {
                total_distance += individuals[i].genome.distance(&individuals[j].genome);
                count += 1;
            }
        }

        if count > 0 {
            // Normalize to 0-1
            (total_distance / count as f64).min(10.0) / 10.0
        } else {
            0.0
        }
    }

    fn update_hall_of_fame(&mut self, population: &Population) {
        for individual in population.iter() {
            self.hall_of_fame.try_add(individual, self.generation);
        }

        // Age out if configured
        if let Some(max_age) = self.config.max_age {
            self.hall_of_fame.age_out(self.generation, max_age);
        }
    }

    /// Set generation
    pub fn set_generation(&mut self, generation: Generation) {
        self.generation = generation;
    }

    /// Get hall of fame
    pub fn hall_of_fame(&self) -> &HallOfFame {
        &self.hall_of_fame
    }

    /// Get statistics
    pub fn stats(&self) -> &ElitismStats {
        &self.stats
    }
}

impl Default for ElitismManager {
    fn default() -> Self {
        Self::new(ElitismConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use core::sync::atomic::AtomicU64;

    use super::super::Lineage;
    use super::super::genome::CodeGenome;
    use super::*;

    fn create_individual(id: u64, fitness: f64) -> Individual {
        let counter = AtomicU64::new(1);
        Individual {
            id: GenomeId(id),
            genome: CodeGenome::random(id, 5, &counter),
            fitness: Some(Fitness::new(vec![fitness])),
            species: None,
            generation: Generation(0),
            parents: Vec::new(),
            lineage: Lineage::default(),
        }
    }

    #[test]
    fn test_hall_of_fame_creation() {
        let hof = HallOfFame::new(10);
        assert!(hof.is_empty());
        assert_eq!(hof.len(), 0);
    }

    #[test]
    fn test_hall_of_fame_add() {
        let mut hof = HallOfFame::new(5);

        for i in 0..10 {
            let ind = create_individual(i, i as f64 / 10.0);
            hof.try_add(&ind, Generation(0));
        }

        assert_eq!(hof.len(), 5);

        // Best should have highest fitness
        let best = hof.best().unwrap();
        assert_eq!(best.rank, 1);
    }

    #[test]
    fn test_elitism_manager() {
        let config = ElitismConfig {
            strategy: ElitismStrategy::TopN { count: 3 },
            ..Default::default()
        };
        let mut manager = ElitismManager::new(config);

        let mut population = Population::new(10);
        for i in 0..10 {
            population.add(create_individual(i, i as f64 / 10.0));
        }

        let elites = manager.get_elites(&population);
        assert_eq!(elites.len(), 3);
    }
}
