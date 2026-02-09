//! Species management for NEAT speciation.

use alloc::vec::Vec;

use crate::neuroevo::genome::NeatGenome;

/// A species is a group of similar genomes
#[derive(Debug)]
pub struct Species {
    /// Species ID
    pub id: usize,
    /// Representative genome
    pub representative: NeatGenome,
    /// Member genome indices
    pub members: Vec<usize>,
    /// Average fitness
    pub avg_fitness: f64,
    /// Best fitness ever
    pub best_fitness: f64,
    /// Generations since improvement
    pub stagnation: u32,
    /// Offspring allocation
    pub offspring: usize,
}

impl Species {
    /// Create a new species with a representative
    pub fn new(id: usize, representative: NeatGenome) -> Self {
        Self {
            id,
            representative,
            members: Vec::new(),
            avg_fitness: 0.0,
            best_fitness: 0.0,
            stagnation: 0,
            offspring: 0,
        }
    }

    /// Check if a genome belongs to this species
    #[inline(always)]
    pub fn is_compatible(&self, genome: &NeatGenome, threshold: f64) -> bool {
        genome.compatibility_distance(&self.representative, 1.0, 1.0, 0.4) < threshold
    }

    /// Clear members for new generation
    #[inline(always)]
    pub fn clear_members(&mut self) {
        self.members.clear();
    }

    /// Add a member
    #[inline(always)]
    pub fn add_member(&mut self, idx: usize) {
        self.members.push(idx);
    }

    /// Update species statistics
    pub fn update_stats(&mut self, genomes: &[NeatGenome]) {
        if self.members.is_empty() {
            return;
        }

        let total_fitness: f64 = self.members.iter().map(|&i| genomes[i].fitness).sum();

        self.avg_fitness = total_fitness / self.members.len() as f64;

        let max_fitness = self
            .members
            .iter()
            .map(|&i| genomes[i].fitness)
            .fold(f64::NEG_INFINITY, f64::max);

        if max_fitness > self.best_fitness {
            self.best_fitness = max_fitness;
            self.stagnation = 0;
        } else {
            self.stagnation += 1;
        }

        // Update representative (use best member)
        if let Some(&best_idx) = self
            .members
            .iter()
            .max_by(|&&a, &&b| genomes[a].fitness.partial_cmp(&genomes[b].fitness).unwrap())
        {
            self.representative = genomes[best_idx].clone();
        }
    }
}
