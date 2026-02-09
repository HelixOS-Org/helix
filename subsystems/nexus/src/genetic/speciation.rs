//! # Speciation
//!
//! Year 3 EVOLUTION - NEAT-style speciation for complexity growth
//! Groups similar individuals into species for protected innovation.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::cmp::Ordering;

use super::genome::CodeGenome;
use super::population::Population;
use super::{GenomeId, SpeciesId};
use crate::math::F64Ext;

// ============================================================================
// SPECIES
// ============================================================================

/// Species - a group of similar individuals
#[derive(Debug, Clone)]
pub struct Species {
    /// Species ID
    pub id: SpeciesId,
    /// Representative genome
    pub representative: CodeGenome,
    /// Member IDs
    pub members: Vec<GenomeId>,
    /// Age (generations)
    pub age: u64,
    /// Best fitness ever
    pub best_fitness: f64,
    /// Generations since fitness improved
    pub stagnant_generations: u64,
    /// Adjusted fitness sum
    pub adjusted_fitness_sum: f64,
    /// Offspring allocation
    pub offspring_allocation: usize,
}

impl Species {
    /// Create new species
    pub fn new(id: SpeciesId, representative: CodeGenome) -> Self {
        Self {
            id,
            representative,
            members: Vec::new(),
            age: 0,
            best_fitness: f64::NEG_INFINITY,
            stagnant_generations: 0,
            adjusted_fitness_sum: 0.0,
            offspring_allocation: 0,
        }
    }

    /// Add member
    #[inline]
    pub fn add_member(&mut self, id: GenomeId) {
        if !self.members.contains(&id) {
            self.members.push(id);
        }
    }

    /// Remove member
    #[inline(always)]
    pub fn remove_member(&mut self, id: GenomeId) {
        self.members.retain(|&m| m != id);
    }

    /// Check if genome belongs to this species
    #[inline(always)]
    pub fn matches(&self, genome: &CodeGenome, threshold: f64) -> bool {
        self.representative.distance(genome) < threshold
    }

    /// Update statistics
    #[inline]
    pub fn update_stats(&mut self, best_fitness: f64) {
        if best_fitness > self.best_fitness {
            self.best_fitness = best_fitness;
            self.stagnant_generations = 0;
        } else {
            self.stagnant_generations += 1;
        }
        self.age += 1;
    }

    /// Calculate adjusted fitness
    pub fn calculate_adjusted_fitness(&mut self, population: &Population) {
        let member_count = self.members.len() as f64;
        if member_count == 0.0 {
            self.adjusted_fitness_sum = 0.0;
            return;
        }

        self.adjusted_fitness_sum = self
            .members
            .iter()
            .filter_map(|&id| population.get(id))
            .filter_map(|ind| ind.fitness.as_ref())
            .map(|f| f.scalar / member_count)
            .sum();
    }

    /// Select random representative
    #[inline]
    pub fn update_representative(&mut self, population: &Population) {
        if let Some(&member_id) = self.members.first() {
            if let Some(individual) = population.get(member_id) {
                self.representative = individual.genome.clone();
            }
        }
    }

    /// Get size
    #[inline(always)]
    pub fn size(&self) -> usize {
        self.members.len()
    }

    /// Is empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }
}

// ============================================================================
// SPECIATION CONFIG
// ============================================================================

/// Speciation configuration
#[derive(Debug, Clone)]
pub struct SpeciationConfig {
    /// Distance threshold for species membership
    pub threshold: f64,
    /// Target number of species
    pub target_species: usize,
    /// Threshold adjustment rate
    pub threshold_adjustment: f64,
    /// Maximum stagnation before species extinction
    pub max_stagnation: u64,
    /// Elitism per species
    pub elitism_per_species: usize,
    /// Minimum species size
    pub min_species_size: usize,
    /// Coefficients for distance calculation
    pub c1_excess: f64,
    pub c2_disjoint: f64,
    pub c3_weight: f64,
}

impl Default for SpeciationConfig {
    fn default() -> Self {
        Self {
            threshold: 3.0,
            target_species: 10,
            threshold_adjustment: 0.1,
            max_stagnation: 15,
            elitism_per_species: 2,
            min_species_size: 5,
            c1_excess: 1.0,
            c2_disjoint: 1.0,
            c3_weight: 0.4,
        }
    }
}

// ============================================================================
// SPECIES MANAGER
// ============================================================================

/// Species manager
pub struct SpeciesManager {
    /// All species
    species: BTreeMap<SpeciesId, Species>,
    /// Configuration
    config: SpeciationConfig,
    /// Next species ID
    next_id: u64,
    /// Statistics
    stats: SpeciationStats,
}

/// Speciation statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct SpeciationStats {
    /// Total species created
    pub species_created: u64,
    /// Total species extinct
    pub species_extinct: u64,
    /// Current species count
    pub current_species: usize,
    /// Average species size
    pub avg_species_size: f64,
}

impl SpeciesManager {
    /// Create new manager
    pub fn new(config: SpeciationConfig) -> Self {
        Self {
            species: BTreeMap::new(),
            next_id: 1,
            stats: SpeciationStats::default(),
            config,
        }
    }

    /// Speciate population
    pub fn speciate(&mut self, population: &mut Population) {
        // Clear current memberships
        for species in self.species.values_mut() {
            species.members.clear();
        }

        // Assign each individual to a species
        for individual in population.iter_mut() {
            let genome = &individual.genome;
            let assigned = self.assign_to_species(genome);

            if let Some(species) = self.species.get_mut(&assigned) {
                species.add_member(individual.id);
                individual.species = Some(assigned);
            }
        }

        // Remove empty species
        self.remove_empty_species();

        // Update representatives
        for species in self.species.values_mut() {
            species.update_representative(population);
        }

        // Adjust threshold toward target species count
        self.adjust_threshold();

        // Calculate adjusted fitness
        for species in self.species.values_mut() {
            species.calculate_adjusted_fitness(population);
        }

        // Handle stagnation
        self.handle_stagnation(population);

        // Allocate offspring
        self.allocate_offspring(population.len());

        // Update stats
        self.update_stats();
    }

    fn assign_to_species(&mut self, genome: &CodeGenome) -> SpeciesId {
        // Try to find matching species
        for (id, species) in &self.species {
            if species.matches(genome, self.config.threshold) {
                return *id;
            }
        }

        // Create new species
        let id = SpeciesId(self.next_id);
        self.next_id += 1;
        self.stats.species_created += 1;

        let new_species = Species::new(id, genome.clone());
        self.species.insert(id, new_species);

        id
    }

    fn remove_empty_species(&mut self) {
        let empty: Vec<SpeciesId> = self
            .species
            .iter()
            .filter(|(_, s)| s.is_empty())
            .map(|(id, _)| *id)
            .collect();

        for id in empty {
            self.species.remove(&id);
            self.stats.species_extinct += 1;
        }
    }

    fn adjust_threshold(&mut self) {
        let current = self.species.len();
        let target = self.config.target_species;

        match current.cmp(&target) {
            Ordering::Less => {
                // Too few species, increase threshold (more separation)
                self.config.threshold += self.config.threshold_adjustment;
            },
            Ordering::Greater => {
                // Too many species, decrease threshold (less separation)
                self.config.threshold =
                    (self.config.threshold - self.config.threshold_adjustment).max(0.1);
            },
            Ordering::Equal => {},
        }
    }

    fn handle_stagnation(&mut self, _population: &Population) {
        let stagnant: Vec<SpeciesId> = self
            .species
            .iter()
            .filter(|(_, s)| s.stagnant_generations > self.config.max_stagnation)
            .map(|(id, _)| *id)
            .collect();

        // Keep at least 2 species
        let max_extinctions = self.species.len().saturating_sub(2);
        let extinctions = stagnant.len().min(max_extinctions);

        for id in stagnant.into_iter().take(extinctions) {
            self.species.remove(&id);
            self.stats.species_extinct += 1;
        }
    }

    fn allocate_offspring(&mut self, total_offspring: usize) {
        let total_adjusted: f64 = self.species.values().map(|s| s.adjusted_fitness_sum).sum();

        if total_adjusted <= 0.0 {
            // Equal allocation
            let per_species = total_offspring / self.species.len().max(1);
            for species in self.species.values_mut() {
                species.offspring_allocation = per_species;
            }
            return;
        }

        let mut allocated = 0;

        for species in self.species.values_mut() {
            let share = species.adjusted_fitness_sum / total_adjusted;
            let offspring = (share * total_offspring as f64).round() as usize;
            species.offspring_allocation = offspring.max(self.config.min_species_size);
            allocated += species.offspring_allocation;
        }

        // Adjust for rounding
        if allocated < total_offspring {
            if let Some(best) = self.best_species_mut() {
                best.offspring_allocation += total_offspring - allocated;
            }
        }
    }

    fn best_species_mut(&mut self) -> Option<&mut Species> {
        let best_id = self
            .species
            .iter()
            .max_by(|a, b| {
                a.1.best_fitness
                    .partial_cmp(&b.1.best_fitness)
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
            .map(|(id, _)| *id)?;

        self.species.get_mut(&best_id)
    }

    fn update_stats(&mut self) {
        self.stats.current_species = self.species.len();

        let total_members: usize = self.species.values().map(|s| s.size()).sum();

        self.stats.avg_species_size = if self.species.is_empty() {
            0.0
        } else {
            total_members as f64 / self.species.len() as f64
        };
    }

    /// Get species count
    #[inline(always)]
    pub fn species_count(&self) -> usize {
        self.species.len()
    }

    /// Get species
    #[inline(always)]
    pub fn get(&self, id: SpeciesId) -> Option<&Species> {
        self.species.get(&id)
    }

    /// Get all species
    #[inline(always)]
    pub fn species(&self) -> impl Iterator<Item = &Species> {
        self.species.values()
    }

    /// Get best species
    #[inline]
    pub fn best_species(&self) -> Option<&Species> {
        self.species.values().max_by(|a, b| {
            a.best_fitness
                .partial_cmp(&b.best_fitness)
                .unwrap_or(core::cmp::Ordering::Equal)
        })
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &SpeciationStats {
        &self.stats
    }

    /// Get configuration
    #[inline(always)]
    pub fn config(&self) -> &SpeciationConfig {
        &self.config
    }

    /// Update configuration
    #[inline(always)]
    pub fn set_config(&mut self, config: SpeciationConfig) {
        self.config = config;
    }
}

impl Default for SpeciesManager {
    fn default() -> Self {
        Self::new(SpeciationConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use core::sync::atomic::AtomicU64;

    use super::*;

    #[test]
    fn test_species_creation() {
        let counter = AtomicU64::new(1);
        let genome = CodeGenome::random(1, 5, &counter);
        let species = Species::new(SpeciesId(1), genome);

        assert_eq!(species.id.0, 1);
        assert!(species.members.is_empty());
    }

    #[test]
    fn test_species_manager() {
        let manager = SpeciesManager::default();
        assert_eq!(manager.species_count(), 0);
    }

    #[test]
    fn test_add_member() {
        let counter = AtomicU64::new(1);
        let genome = CodeGenome::random(1, 5, &counter);
        let mut species = Species::new(SpeciesId(1), genome);

        species.add_member(GenomeId(1));
        species.add_member(GenomeId(2));

        assert_eq!(species.size(), 2);
    }

    #[test]
    fn test_species_matching() {
        let counter = AtomicU64::new(1);
        let genome = CodeGenome::random(1, 5, &counter);
        let species = Species::new(SpeciesId(1), genome.clone());

        // Same genome should match
        assert!(species.matches(&genome, 10.0));
    }
}
