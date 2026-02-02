//! # Niching
//!
//! Year 3 EVOLUTION - Niching and crowding for diversity maintenance
//! Prevents premature convergence by maintaining population diversity.

#![allow(dead_code)]

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::genome::CodeGenome;
use super::population::Population;
use super::{Fitness, GenomeId, Individual};
use crate::math::F64Ext;

// ============================================================================
// NICHING TYPES
// ============================================================================

/// Niche
#[derive(Debug, Clone)]
pub struct Niche {
    /// Niche ID
    pub id: u64,
    /// Center (representative genome)
    pub center: CodeGenome,
    /// Radius
    pub radius: f64,
    /// Members
    pub members: Vec<GenomeId>,
    /// Peak fitness in niche
    pub peak_fitness: f64,
}

impl Niche {
    /// Create new niche
    pub fn new(id: u64, center: CodeGenome, radius: f64) -> Self {
        Self {
            id,
            center,
            radius,
            members: Vec::new(),
            peak_fitness: 0.0,
        }
    }

    /// Check if genome belongs to this niche
    pub fn contains(&self, genome: &CodeGenome) -> bool {
        self.center.distance(genome) <= self.radius
    }

    /// Add member
    pub fn add_member(&mut self, id: GenomeId) {
        if !self.members.contains(&id) {
            self.members.push(id);
        }
    }

    /// Clear members
    pub fn clear_members(&mut self) {
        self.members.clear();
    }

    /// Get size
    pub fn size(&self) -> usize {
        self.members.len()
    }
}

/// Sharing function type
#[derive(Debug, Clone, Copy)]
pub enum SharingFunction {
    /// Triangular sharing
    Triangular,
    /// Power sharing
    Power { alpha: f64 },
    /// Gaussian sharing
    Gaussian,
    /// Binary sharing
    Binary,
}

/// Niching configuration
#[derive(Debug, Clone)]
pub struct NichingConfig {
    /// Sharing radius (sigma)
    pub sigma: f64,
    /// Sharing function
    pub sharing_function: SharingFunction,
    /// Enable fitness sharing
    pub fitness_sharing: bool,
    /// Enable crowding
    pub crowding: bool,
    /// Crowding factor
    pub crowding_factor: usize,
    /// Enable clearing
    pub clearing: bool,
    /// Clearing radius
    pub clearing_radius: f64,
    /// Winners per niche (for clearing)
    pub winners_per_niche: usize,
}

impl Default for NichingConfig {
    fn default() -> Self {
        Self {
            sigma: 3.0,
            sharing_function: SharingFunction::Triangular,
            fitness_sharing: true,
            crowding: false,
            crowding_factor: 3,
            clearing: false,
            clearing_radius: 2.0,
            winners_per_niche: 1,
        }
    }
}

// ============================================================================
// NICHING MANAGER
// ============================================================================

/// Niching manager
pub struct NichingManager {
    /// Configuration
    config: NichingConfig,
    /// Niches
    niches: BTreeMap<u64, Niche>,
    /// Next niche ID
    next_id: u64,
    /// Statistics
    stats: NichingStats,
}

/// Niching statistics
#[derive(Debug, Clone, Default)]
pub struct NichingStats {
    /// Total niches created
    pub niches_created: u64,
    /// Current niche count
    pub current_niches: usize,
    /// Average niche size
    pub avg_niche_size: f64,
    /// Fitness adjustments made
    pub adjustments_made: u64,
}

impl NichingManager {
    /// Create new manager
    pub fn new(config: NichingConfig) -> Self {
        Self {
            config,
            niches: BTreeMap::new(),
            next_id: 1,
            stats: NichingStats::default(),
        }
    }

    /// Apply niching to population
    pub fn apply(&mut self, population: &mut Population) {
        if self.config.fitness_sharing {
            self.apply_fitness_sharing(population);
        }

        if self.config.clearing {
            self.apply_clearing(population);
        }

        self.update_stats();
    }

    /// Apply fitness sharing
    fn apply_fitness_sharing(&mut self, population: &mut Population) {
        let individuals: Vec<(GenomeId, CodeGenome, Fitness)> = population
            .iter()
            .filter_map(|ind| {
                ind.fitness
                    .as_ref()
                    .map(|f| (ind.id, ind.genome.clone(), f.clone()))
            })
            .collect();

        for individual in population.iter_mut() {
            if let Some(fitness) = &mut individual.fitness {
                let niche_count = self.calculate_niche_count(&individual.genome, &individuals);

                // Shared fitness = raw fitness / niche count
                if niche_count > 0.0 {
                    fitness.scalar /= niche_count;
                    self.stats.adjustments_made += 1;
                }
            }
        }
    }

    /// Calculate niche count for a genome
    fn calculate_niche_count(
        &self,
        genome: &CodeGenome,
        others: &[(GenomeId, CodeGenome, Fitness)],
    ) -> f64 {
        let mut niche_count = 0.0;

        for (_, other_genome, _) in others {
            let distance = genome.distance(other_genome);
            let sharing = self.sharing_value(distance);
            niche_count += sharing;
        }

        niche_count
    }

    /// Calculate sharing value for a distance
    fn sharing_value(&self, distance: f64) -> f64 {
        if distance >= self.config.sigma {
            return 0.0;
        }

        match self.config.sharing_function {
            SharingFunction::Triangular => 1.0 - (distance / self.config.sigma),
            SharingFunction::Power { alpha } => 1.0 - (distance / self.config.sigma).powf(alpha),
            SharingFunction::Gaussian => {
                (-distance.powi(2) / (2.0 * self.config.sigma.powi(2))).exp()
            },
            SharingFunction::Binary => 1.0,
        }
    }

    /// Apply clearing
    fn apply_clearing(&mut self, population: &mut Population) {
        // Clear previous niches
        for niche in self.niches.values_mut() {
            niche.clear_members();
        }

        // Sort by fitness
        let mut sorted: Vec<(GenomeId, &CodeGenome, f64)> = population
            .iter()
            .filter_map(|ind| {
                ind.fitness
                    .as_ref()
                    .map(|f| (ind.id, &ind.genome, f.scalar))
            })
            .collect();

        sorted.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(core::cmp::Ordering::Equal));

        // Assign to niches and clear losers
        let mut cleared = Vec::new();

        for (id, genome, _) in &sorted {
            // Find or create niche
            let niche_id = self.find_or_create_niche(genome);

            if let Some(niche) = self.niches.get_mut(&niche_id) {
                if niche.members.len() < self.config.winners_per_niche {
                    // This is a winner
                    niche.add_member(*id);
                } else {
                    // This is a loser, clear its fitness
                    cleared.push(*id);
                }
            }
        }

        // Clear fitness of losers
        for id in cleared {
            if let Some(individual) = population.get_mut(id) {
                if let Some(fitness) = &mut individual.fitness {
                    fitness.scalar = 0.0;
                }
            }
        }
    }

    /// Find or create niche for genome
    fn find_or_create_niche(&mut self, genome: &CodeGenome) -> u64 {
        // Try to find existing niche
        for (id, niche) in &self.niches {
            if niche.center.distance(genome) <= self.config.clearing_radius {
                return *id;
            }
        }

        // Create new niche
        let id = self.next_id;
        self.next_id += 1;
        self.stats.niches_created += 1;

        let niche = Niche::new(id, genome.clone(), self.config.clearing_radius);
        self.niches.insert(id, niche);

        id
    }

    /// Apply crowding (for steady-state)
    pub fn crowding_replacement(
        &self,
        population: &Population,
        offspring: &Individual,
    ) -> Option<GenomeId> {
        // Select random individuals
        let candidates: Vec<&Individual> = population
            .iter()
            .take(self.config.crowding_factor)
            .collect();

        if candidates.is_empty() {
            return None;
        }

        // Find most similar
        let most_similar = candidates.iter().min_by(|a, b| {
            let da = a.genome.distance(&offspring.genome);
            let db = b.genome.distance(&offspring.genome);
            da.partial_cmp(&db).unwrap_or(core::cmp::Ordering::Equal)
        })?;

        // Replace if offspring is fitter
        let offspring_fitness = offspring.fitness.as_ref()?.scalar;
        let similar_fitness = most_similar.fitness.as_ref()?.scalar;

        if offspring_fitness > similar_fitness {
            Some(most_similar.id)
        } else {
            None
        }
    }

    /// Calculate population diversity
    pub fn calculate_diversity(&self, population: &Population) -> f64 {
        let individuals: Vec<&Individual> = population.iter().collect();
        let n = individuals.len();

        if n < 2 {
            return 0.0;
        }

        let mut total_distance = 0.0;
        let mut count = 0;

        for i in 0..n {
            for j in (i + 1)..n {
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

    /// Get niche count
    pub fn niche_count(&self) -> usize {
        self.niches.len()
    }

    /// Get niches
    pub fn niches(&self) -> impl Iterator<Item = &Niche> {
        self.niches.values()
    }

    /// Update statistics
    fn update_stats(&mut self) {
        self.stats.current_niches = self.niches.len();

        if self.niches.is_empty() {
            self.stats.avg_niche_size = 0.0;
        } else {
            let total_size: usize = self.niches.values().map(|n| n.size()).sum();
            self.stats.avg_niche_size = total_size as f64 / self.niches.len() as f64;
        }
    }

    /// Get statistics
    pub fn stats(&self) -> &NichingStats {
        &self.stats
    }
}

impl Default for NichingManager {
    fn default() -> Self {
        Self::new(NichingConfig::default())
    }
}

// ============================================================================
// CROWDING SELECTION
// ============================================================================

/// Deterministic crowding
pub fn deterministic_crowding(parents: &[Individual], offspring: &[Individual]) -> Vec<Individual> {
    let mut survivors = Vec::new();

    // Pair parents with offspring based on similarity
    let n = parents.len().min(offspring.len());

    for i in 0..n {
        let parent = &parents[i];
        let child = &offspring[i];

        let parent_fitness = parent.fitness.as_ref().map(|f| f.scalar).unwrap_or(0.0);
        let child_fitness = child.fitness.as_ref().map(|f| f.scalar).unwrap_or(0.0);

        // Winner survives
        if child_fitness > parent_fitness {
            survivors.push(child.clone());
        } else {
            survivors.push(parent.clone());
        }
    }

    // Add remaining
    for parent in parents.iter().skip(n) {
        survivors.push(parent.clone());
    }

    survivors
}

/// Restricted tournament selection
pub fn restricted_tournament_selection(
    population: &Population,
    offspring: &Individual,
    window_size: usize,
) -> Option<GenomeId> {
    // Select random window
    let all: Vec<&Individual> = population.iter().collect();
    if all.is_empty() {
        return None;
    }

    let window_size = window_size.min(all.len());

    // Find most similar in window
    let window: Vec<&Individual> = all.iter().take(window_size).copied().collect();

    let most_similar = window.iter().min_by(|a, b| {
        let da = a.genome.distance(&offspring.genome);
        let db = b.genome.distance(&offspring.genome);
        da.partial_cmp(&db).unwrap_or(core::cmp::Ordering::Equal)
    })?;

    Some(most_similar.id)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use core::sync::atomic::AtomicU64;

    use super::*;

    #[test]
    fn test_niche_creation() {
        let counter = AtomicU64::new(1);
        let genome = CodeGenome::random(1, 5, &counter);
        let niche = Niche::new(1, genome.clone(), 2.0);

        assert!(niche.contains(&genome));
    }

    #[test]
    fn test_sharing_triangular() {
        let manager = NichingManager::default();

        assert_eq!(manager.sharing_value(0.0), 1.0);
        assert!(manager.sharing_value(manager.config.sigma / 2.0) > 0.0);
        assert_eq!(manager.sharing_value(manager.config.sigma), 0.0);
    }

    #[test]
    fn test_niching_manager() {
        let manager = NichingManager::default();
        assert_eq!(manager.niche_count(), 0);
    }
}
