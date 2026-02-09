//! Novelty Search - exploration-based search without explicit objectives.

use alloc::vec::Vec;

/// Behavior characterization for novelty search
#[derive(Debug, Clone)]
pub struct BehaviorVector {
    /// Behavior features
    pub features: Vec<f64>,
}

impl BehaviorVector {
    /// Create a new behavior vector
    pub fn new(features: Vec<f64>) -> Self {
        Self { features }
    }

    /// Compute distance to another behavior
    pub fn distance(&self, other: &BehaviorVector) -> f64 {
        if self.features.len() != other.features.len() {
            return f64::INFINITY;
        }

        let sum: f64 = self
            .features
            .iter()
            .zip(other.features.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum();

        libm::sqrt(sum)
    }
}

/// Novelty Search - exploration without explicit objectives
pub struct NoveltySearch {
    /// Archive of novel behaviors
    pub archive: Vec<BehaviorVector>,
    /// Archive threshold for adding new behaviors
    pub archive_threshold: f64,
    /// K-nearest neighbors for novelty calculation
    pub k_neighbors: usize,
    /// Maximum archive size
    pub max_archive_size: usize,
    /// Current population behaviors
    population_behaviors: Vec<BehaviorVector>,
}

impl NoveltySearch {
    /// Create a new novelty search instance
    pub fn new(archive_threshold: f64, k_neighbors: usize, max_archive_size: usize) -> Self {
        Self {
            archive: Vec::new(),
            archive_threshold,
            k_neighbors,
            max_archive_size,
            population_behaviors: Vec::new(),
        }
    }

    /// Calculate novelty score for a behavior
    pub fn novelty_score(&self, behavior: &BehaviorVector) -> f64 {
        // Combine archive and population for neighbor search
        let mut all_behaviors: Vec<&BehaviorVector> = self.archive.iter().collect();
        all_behaviors.extend(self.population_behaviors.iter());

        if all_behaviors.is_empty() {
            return f64::INFINITY; // First behavior is maximally novel
        }

        // Calculate distances to all behaviors
        let mut distances: Vec<f64> = all_behaviors.iter().map(|b| behavior.distance(b)).collect();

        // Sort to find k-nearest
        distances.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Average distance to k-nearest
        let k = self.k_neighbors.min(distances.len());
        distances[..k].iter().sum::<f64>() / k as f64
    }

    /// Add a behavior to the archive if novel enough
    pub fn add_to_archive(&mut self, behavior: BehaviorVector) {
        let novelty = self.novelty_score(&behavior);

        if novelty > self.archive_threshold {
            self.archive.push(behavior);

            // Prune archive if too large
            if self.archive.len() > self.max_archive_size {
                // Remove least novel
                self.prune_archive();
            }
        }
    }

    /// Set population behaviors for current generation
    #[inline(always)]
    pub fn set_population_behaviors(&mut self, behaviors: Vec<BehaviorVector>) {
        self.population_behaviors = behaviors;
    }

    /// Prune archive to max size by removing least novel
    fn prune_archive(&mut self) {
        if self.archive.len() <= self.max_archive_size {
            return;
        }

        // Calculate novelty for each archive member
        let mut scored: Vec<(usize, f64)> = Vec::new();
        for (i, behavior) in self.archive.iter().enumerate() {
            // Calculate novelty within archive only
            let mut distances: Vec<f64> = self
                .archive
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, b)| behavior.distance(b))
                .collect();
            distances.sort_by(|a, b| a.partial_cmp(b).unwrap());

            let k = self.k_neighbors.min(distances.len());
            let avg_dist = if k > 0 {
                distances[..k].iter().sum::<f64>() / k as f64
            } else {
                0.0
            };
            scored.push((i, avg_dist));
        }

        // Sort by novelty (ascending, so least novel first)
        scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        // Keep only the most novel
        let to_remove = self.archive.len() - self.max_archive_size;
        let remove_indices: alloc::collections::BTreeSet<usize> =
            scored[..to_remove].iter().map(|(i, _)| *i).collect();

        self.archive = self
            .archive
            .iter()
            .enumerate()
            .filter(|(i, _)| !remove_indices.contains(i))
            .map(|(_, b)| b.clone())
            .collect();
    }
}
