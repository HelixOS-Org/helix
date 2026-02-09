//! # Island Model
//!
//! Year 3 EVOLUTION - Island model for parallel evolution
//! Multiple isolated populations with periodic migration.

#![allow(dead_code)]
#![allow(clippy::excessive_nesting)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::population::Population;
use super::{GenomeId, Individual, IslandId};

// ============================================================================
// ISLAND MODEL
// ============================================================================

/// Island in the archipelago
#[derive(Debug, Clone)]
pub struct Island {
    /// Island ID
    pub id: IslandId,
    /// Population
    pub population: Population,
    /// Best individual ever on this island
    pub best_ever: Option<Individual>,
    /// Generation count
    pub generation: u64,
    /// Stagnation counter
    pub stagnant_generations: u64,
    /// Topology connections (other island IDs)
    pub neighbors: Vec<IslandId>,
}

impl Island {
    /// Create new island
    pub fn new(id: IslandId, capacity: usize) -> Self {
        Self {
            id,
            population: Population::new(capacity),
            best_ever: None,
            generation: 0,
            stagnant_generations: 0,
            neighbors: Vec::new(),
        }
    }

    /// Add neighbor
    #[inline]
    pub fn add_neighbor(&mut self, neighbor: IslandId) {
        if !self.neighbors.contains(&neighbor) {
            self.neighbors.push(neighbor);
        }
    }

    /// Get best individual
    #[inline(always)]
    pub fn best(&self) -> Option<&Individual> {
        self.population.best()
    }

    /// Update best ever
    pub fn update_best(&mut self) {
        if let Some(current_best) = self.population.best() {
            let should_update = match &self.best_ever {
                None => true,
                Some(best) => current_best
                    .fitness
                    .as_ref()
                    .zip(best.fitness.as_ref())
                    .map(|(c, b)| c.scalar > b.scalar)
                    .unwrap_or(false),
            };

            if should_update {
                self.best_ever = Some(current_best.clone());
                self.stagnant_generations = 0;
            } else {
                self.stagnant_generations += 1;
            }
        }
    }
}

/// Island topology
#[derive(Debug, Clone)]
pub enum Topology {
    /// Ring topology (each island connected to neighbors)
    Ring,
    /// Fully connected
    FullyConnected,
    /// Star (one central island)
    Star,
    /// Grid (2D layout)
    Grid { width: usize },
    /// Random (each island has random neighbors)
    Random { connections: usize },
    /// Hypercube
    Hypercube,
}

/// Migration policy
#[derive(Debug, Clone)]
pub struct MigrationPolicy {
    /// Migration rate
    pub rate: f64,
    /// Interval (generations between migrations)
    pub interval: u64,
    /// Number of migrants
    pub migrant_count: usize,
    /// Selection method for emigrants
    pub emigrant_selection: EmigrantSelection,
    /// Replacement method for immigrants
    pub immigrant_replacement: ImmigrantReplacement,
}

impl Default for MigrationPolicy {
    fn default() -> Self {
        Self {
            rate: 0.1,
            interval: 10,
            migrant_count: 3,
            emigrant_selection: EmigrantSelection::Best,
            immigrant_replacement: ImmigrantReplacement::Worst,
        }
    }
}

/// Emigrant selection method
#[derive(Debug, Clone)]
pub enum EmigrantSelection {
    /// Select best individuals
    Best,
    /// Random selection
    Random,
    /// Tournament selection
    Tournament { size: usize },
    /// Select most novel
    MostNovel,
}

/// Immigrant replacement method
#[derive(Debug, Clone)]
pub enum ImmigrantReplacement {
    /// Replace worst individuals
    Worst,
    /// Replace random individuals
    Random,
    /// Replace most similar
    MostSimilar,
    /// Replace oldest
    Oldest,
}

/// Island configuration
#[derive(Debug, Clone)]
pub struct IslandConfig {
    /// Population size per island
    pub population_size: usize,
    /// Topology
    pub topology: Topology,
    /// Migration policy
    pub migration: MigrationPolicy,
}

impl Default for IslandConfig {
    fn default() -> Self {
        Self {
            population_size: 50,
            topology: Topology::Ring,
            migration: MigrationPolicy::default(),
        }
    }
}

// ============================================================================
// ISLAND MANAGER
// ============================================================================

/// Island manager
pub struct IslandManager {
    /// Islands
    islands: BTreeMap<IslandId, Island>,
    /// Configuration
    config: IslandConfig,
    /// Current generation
    generation: u64,
    /// Statistics
    stats: IslandStats,
}

/// Island statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct IslandStats {
    /// Total migrations
    pub migrations: u64,
    /// Successful adaptations (immigrant improved population)
    pub successful_adaptations: u64,
}

impl IslandManager {
    /// Create new manager
    pub fn new(island_count: usize, config: IslandConfig) -> Self {
        let mut manager = Self {
            islands: BTreeMap::new(),
            generation: 0,
            stats: IslandStats::default(),
            config,
        };

        // Create islands
        for i in 0..island_count {
            let id = IslandId(i as u64);
            manager
                .islands
                .insert(id, Island::new(id, manager.config.population_size));
        }

        // Set up topology
        manager.setup_topology();

        manager
    }

    fn setup_topology(&mut self) {
        let island_ids: Vec<IslandId> = self.islands.keys().copied().collect();
        let n = island_ids.len();

        match &self.config.topology {
            Topology::Ring => {
                for (i, &id) in island_ids.iter().enumerate() {
                    let left = island_ids[(i + n - 1) % n];
                    let right = island_ids[(i + 1) % n];
                    if let Some(island) = self.islands.get_mut(&id) {
                        island.add_neighbor(left);
                        island.add_neighbor(right);
                    }
                }
            },
            Topology::FullyConnected => {
                for &id1 in &island_ids {
                    for &id2 in &island_ids {
                        if id1 != id2 {
                            if let Some(island) = self.islands.get_mut(&id1) {
                                island.add_neighbor(id2);
                            }
                        }
                    }
                }
            },
            Topology::Star => {
                if let Some(&center) = island_ids.first() {
                    for &id in &island_ids[1..] {
                        if let Some(island) = self.islands.get_mut(&id) {
                            island.add_neighbor(center);
                        }
                        if let Some(center_island) = self.islands.get_mut(&center) {
                            center_island.add_neighbor(id);
                        }
                    }
                }
            },
            Topology::Grid { width } => {
                for (i, &id) in island_ids.iter().enumerate() {
                    let x = i % width;
                    let _y = i / width;

                    // Right neighbor
                    if x + 1 < *width && i + 1 < n {
                        if let Some(island) = self.islands.get_mut(&id) {
                            island.add_neighbor(island_ids[i + 1]);
                        }
                    }

                    // Bottom neighbor
                    if i + width < n {
                        if let Some(island) = self.islands.get_mut(&id) {
                            island.add_neighbor(island_ids[i + width]);
                        }
                    }
                }
            },
            Topology::Random { connections } => {
                for &id in &island_ids {
                    let mut added = 0;
                    while added < *connections {
                        let other = island_ids[rand_usize(n)];
                        if other != id {
                            if let Some(island) = self.islands.get_mut(&id) {
                                if !island.neighbors.contains(&other) {
                                    island.add_neighbor(other);
                                    added += 1;
                                }
                            }
                        }
                    }
                }
            },
            Topology::Hypercube => {
                // Connect islands differing by one bit in their ID
                for &id1 in &island_ids {
                    for &id2 in &island_ids {
                        if (id1.0 ^ id2.0).count_ones() == 1 {
                            if let Some(island) = self.islands.get_mut(&id1) {
                                island.add_neighbor(id2);
                            }
                        }
                    }
                }
            },
        }
    }

    /// Maybe perform migration
    pub fn maybe_migrate(&mut self, _main_population: &mut Population, migration_rate: f64) {
        self.generation += 1;

        // Check if it's time to migrate
        if self.generation % self.config.migration.interval != 0 {
            return;
        }

        if rand_f64() > migration_rate {
            return;
        }

        self.perform_migration();
    }

    fn perform_migration(&mut self) {
        let island_ids: Vec<IslandId> = self.islands.keys().copied().collect();

        // Collect emigrants from each island
        let mut emigrants: BTreeMap<IslandId, Vec<Individual>> = BTreeMap::new();

        for &id in &island_ids {
            if let Some(island) = self.islands.get(&id) {
                let selected = self.select_emigrants(island);
                emigrants.insert(id, selected);
            }
        }

        // Send emigrants to neighbors
        for &source_id in &island_ids {
            if let Some(source_emigrants) = emigrants.get(&source_id) {
                if let Some(source) = self.islands.get(&source_id) {
                    let neighbors = source.neighbors.clone();

                    for neighbor_id in neighbors {
                        if let Some(target) = self.islands.get_mut(&neighbor_id) {
                            for emigrant in source_emigrants {
                                Self::replace_immigrant_static(
                                    &self.config.migration.immigrant_replacement,
                                    target,
                                    emigrant.clone(),
                                );
                                self.stats.migrations += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    fn select_emigrants(&self, island: &Island) -> Vec<Individual> {
        let count = self.config.migration.migrant_count;

        match &self.config.migration.emigrant_selection {
            EmigrantSelection::Best => island.population.elites(count),
            EmigrantSelection::Random => {
                let all: Vec<_> = island.population.iter().collect();
                let mut selected = Vec::new();
                for _ in 0..count.min(all.len()) {
                    let idx = rand_usize(all.len());
                    selected.push(all[idx].clone());
                }
                selected
            },
            EmigrantSelection::Tournament { size } => {
                island.population.tournament_selection(*size, count)
            },
            EmigrantSelection::MostNovel => {
                // Calculate novelty and select most novel
                let mut with_novelty: Vec<(&Individual, f64)> = island
                    .population
                    .iter()
                    .map(|ind| {
                        let novelty = island
                            .population
                            .iter()
                            .filter(|other| other.id != ind.id)
                            .map(|other| ind.genome.distance(&other.genome))
                            .sum::<f64>();
                        (ind, novelty)
                    })
                    .collect();

                with_novelty
                    .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));

                with_novelty
                    .into_iter()
                    .take(count)
                    .map(|(ind, _)| ind.clone())
                    .collect()
            },
        }
    }

    fn replace_immigrant(&mut self, island: &mut Island, immigrant: Individual) {
        Self::replace_immigrant_static(
            &self.config.migration.immigrant_replacement,
            island,
            immigrant,
        );
    }

    fn replace_immigrant_static(
        replacement_strategy: &ImmigrantReplacement,
        island: &mut Island,
        immigrant: Individual,
    ) {
        match replacement_strategy {
            ImmigrantReplacement::Worst => {
                if let Some(worst) = island.population.worst() {
                    let worst_id = worst.id;
                    island.population.remove(worst_id);
                }
                island.population.add(immigrant);
            },
            ImmigrantReplacement::Random => {
                let all: Vec<GenomeId> = island.population.iter().map(|i| i.id).collect();
                if !all.is_empty() {
                    let idx = rand_usize(all.len());
                    island.population.remove(all[idx]);
                }
                island.population.add(immigrant);
            },
            ImmigrantReplacement::MostSimilar => {
                let all: Vec<(GenomeId, f64)> = island
                    .population
                    .iter()
                    .map(|ind| (ind.id, ind.genome.distance(&immigrant.genome)))
                    .collect();

                if let Some((id, _)) = all
                    .iter()
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal))
                {
                    island.population.remove(*id);
                }
                island.population.add(immigrant);
            },
            ImmigrantReplacement::Oldest => {
                let oldest = island
                    .population
                    .iter()
                    .min_by_key(|i| i.generation.0)
                    .map(|i| i.id);

                if let Some(id) = oldest {
                    island.population.remove(id);
                }
                island.population.add(immigrant);
            },
        }
    }

    /// Get island
    #[inline(always)]
    pub fn get(&self, id: IslandId) -> Option<&Island> {
        self.islands.get(&id)
    }

    /// Get island (mutable)
    #[inline(always)]
    pub fn get_mut(&mut self, id: IslandId) -> Option<&mut Island> {
        self.islands.get_mut(&id)
    }

    /// Get all islands
    #[inline(always)]
    pub fn islands(&self) -> impl Iterator<Item = &Island> {
        self.islands.values()
    }

    /// Get global best
    pub fn global_best(&self) -> Option<&Individual> {
        self.islands
            .values()
            .filter_map(|island| island.best_ever.as_ref())
            .max_by(|a, b| {
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
                fa.partial_cmp(&fb).unwrap_or(core::cmp::Ordering::Equal)
            })
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &IslandStats {
        &self.stats
    }
}

// ============================================================================
// RANDOM HELPERS
// ============================================================================

use core::sync::atomic::{AtomicU64, Ordering};

static ISLAND_SEED: AtomicU64 = AtomicU64::new(57913);

fn rand_u64() -> u64 {
    let mut current = ISLAND_SEED.load(Ordering::Relaxed);
    loop {
        let next = current.wrapping_mul(6364136223846793005).wrapping_add(1);
        match ISLAND_SEED.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed)
        {
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
    fn test_island_creation() {
        let island = Island::new(IslandId(1), 50);
        assert_eq!(island.id.0, 1);
        assert!(island.neighbors.is_empty());
    }

    #[test]
    fn test_manager_creation() {
        let config = IslandConfig::default();
        let manager = IslandManager::new(4, config);
        assert_eq!(manager.islands.len(), 4);
    }

    #[test]
    fn test_ring_topology() {
        let config = IslandConfig {
            topology: Topology::Ring,
            ..Default::default()
        };
        let manager = IslandManager::new(4, config);

        // Each island should have 2 neighbors in ring
        for island in manager.islands() {
            assert_eq!(island.neighbors.len(), 2);
        }
    }

    #[test]
    fn test_fully_connected() {
        let config = IslandConfig {
            topology: Topology::FullyConnected,
            ..Default::default()
        };
        let manager = IslandManager::new(4, config);

        // Each island should have 3 neighbors (all others)
        for island in manager.islands() {
            assert_eq!(island.neighbors.len(), 3);
        }
    }
}
