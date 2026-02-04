//! # Crossover Operators
//!
//! Year 3 EVOLUTION - Crossover operators for genetic recombination
//! Combines genetic material from two parent genomes.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::Fitness;
use super::genome::{CodeGenome, Connection, Gene};

// ============================================================================
// CROSSOVER TYPES
// ============================================================================

/// Crossover type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossoverType {
    /// Uniform crossover (random gene selection)
    Uniform,
    /// Single-point crossover
    SinglePoint,
    /// Two-point crossover
    TwoPoint,
    /// NEAT-style crossover (matching genes)
    Neat,
    /// Fitness-weighted crossover
    FitnessWeighted,
    /// Semantic crossover (preserve structure)
    Semantic,
    /// Homologous crossover (aligned genes)
    Homologous,
}

/// Crossover configuration
#[derive(Debug, Clone)]
pub struct CrossoverConfig {
    /// Default crossover type
    pub crossover_type: CrossoverType,
    /// Probability of taking gene from fitter parent
    pub fitter_bias: f64,
    /// Probability of including disjoint genes
    pub disjoint_rate: f64,
    /// Probability of including excess genes
    pub excess_rate: f64,
    /// Enable gene averaging (for matching genes)
    pub average_matching: bool,
}

impl Default for CrossoverConfig {
    fn default() -> Self {
        Self {
            crossover_type: CrossoverType::Neat,
            fitter_bias: 0.6,
            disjoint_rate: 0.5,
            excess_rate: 0.5,
            average_matching: false,
        }
    }
}

/// Crossover result
#[derive(Debug, Clone)]
pub struct CrossoverResult {
    /// Offspring genome
    pub offspring: CodeGenome,
    /// Genes from parent 1
    pub from_parent1: usize,
    /// Genes from parent 2
    pub from_parent2: usize,
    /// Averaged genes
    pub averaged: usize,
}

// ============================================================================
// CROSSOVER OPERATORS
// ============================================================================

/// Uniform crossover - randomly select genes from each parent
pub fn uniform_crossover(parent1: &CodeGenome, parent2: &CodeGenome) -> CodeGenome {
    let mut offspring = CodeGenome::new(rand_u64());

    // Build innovation maps
    let p1_map: BTreeMap<u64, &Gene> = parent1.genes.iter().map(|g| (g.innovation, g)).collect();
    let p2_map: BTreeMap<u64, &Gene> = parent2.genes.iter().map(|g| (g.innovation, g)).collect();

    // Collect all unique innovations
    let all_innovations: Vec<u64> = p1_map
        .keys()
        .chain(p2_map.keys())
        .copied()
        .collect::<alloc::collections::BTreeSet<_>>()
        .into_iter()
        .collect();

    for innovation in all_innovations {
        let gene = match (p1_map.get(&innovation), p2_map.get(&innovation)) {
            (Some(g1), Some(g2)) => {
                // Matching gene - randomly choose
                if rand_f64() < 0.5 {
                    (*g1).clone()
                } else {
                    (*g2).clone()
                }
            },
            (Some(g), None) | (None, Some(g)) => {
                // Disjoint/excess - include with probability
                if rand_f64() < 0.5 {
                    (*g).clone()
                } else {
                    continue;
                }
            },
            (None, None) => continue,
        };

        offspring.genes.push(gene);
    }

    // Crossover connections
    crossover_connections(&mut offspring, parent1, parent2);

    offspring
}

/// Single-point crossover
pub fn single_point_crossover(parent1: &CodeGenome, parent2: &CodeGenome) -> CodeGenome {
    let mut offspring = CodeGenome::new(rand_u64());

    let len1 = parent1.genes.len();
    let len2 = parent2.genes.len();

    if len1 == 0 && len2 == 0 {
        return offspring;
    }

    let point1 = rand_usize(len1.max(1));
    let point2 = rand_usize(len2.max(1));

    // Take first part from parent1
    for gene in parent1.genes.iter().take(point1) {
        offspring.genes.push(gene.clone());
    }

    // Take second part from parent2
    for gene in parent2.genes.iter().skip(point2) {
        offspring.genes.push(gene.clone());
    }

    crossover_connections(&mut offspring, parent1, parent2);

    offspring
}

/// Two-point crossover
pub fn two_point_crossover(parent1: &CodeGenome, parent2: &CodeGenome) -> CodeGenome {
    let mut offspring = CodeGenome::new(rand_u64());

    let len1 = parent1.genes.len();
    let len2 = parent2.genes.len();

    if len1 < 2 || len2 < 2 {
        return uniform_crossover(parent1, parent2);
    }

    let mut point1_a = rand_usize(len1);
    let mut point1_b = rand_usize(len1);
    if point1_a > point1_b {
        core::mem::swap(&mut point1_a, &mut point1_b);
    }

    let mut point2_a = rand_usize(len2);
    let mut point2_b = rand_usize(len2);
    if point2_a > point2_b {
        core::mem::swap(&mut point2_a, &mut point2_b);
    }

    // Take [0..point1_a) from parent1
    for gene in parent1.genes.iter().take(point1_a) {
        offspring.genes.push(gene.clone());
    }

    // Take [point2_a..point2_b) from parent2
    for gene in parent2
        .genes
        .iter()
        .skip(point2_a)
        .take(point2_b - point2_a)
    {
        offspring.genes.push(gene.clone());
    }

    // Take [point1_b..) from parent1
    for gene in parent1.genes.iter().skip(point1_b) {
        offspring.genes.push(gene.clone());
    }

    crossover_connections(&mut offspring, parent1, parent2);

    offspring
}

/// NEAT-style crossover
pub fn neat_crossover(
    parent1: &CodeGenome,
    parent2: &CodeGenome,
    fitness1: &Fitness,
    fitness2: &Fitness,
    config: &CrossoverConfig,
) -> CodeGenome {
    let mut offspring = CodeGenome::new(rand_u64());

    // Determine fitter parent
    let (fitter, weaker) = if fitness1.scalar >= fitness2.scalar {
        (parent1, parent2)
    } else {
        (parent2, parent1)
    };

    // Build innovation maps
    let fitter_map: BTreeMap<u64, &Gene> = fitter.genes.iter().map(|g| (g.innovation, g)).collect();
    let weaker_map: BTreeMap<u64, &Gene> = weaker.genes.iter().map(|g| (g.innovation, g)).collect();

    // Find max innovation in weaker parent
    let max_weaker = weaker.genes.iter().map(|g| g.innovation).max().unwrap_or(0);

    for (innovation, gene) in &fitter_map {
        if let Some(weaker_gene) = weaker_map.get(innovation) {
            // Matching gene
            let chosen = if config.average_matching {
                average_genes(gene, weaker_gene)
            } else if rand_f64() < config.fitter_bias {
                (*gene).clone()
            } else {
                (*weaker_gene).clone()
            };
            offspring.genes.push(chosen);
        } else if *innovation > max_weaker {
            // Excess gene (from fitter parent)
            if rand_f64() < config.excess_rate {
                offspring.genes.push((*gene).clone());
            }
        } else {
            // Disjoint gene (from fitter parent)
            if rand_f64() < config.disjoint_rate {
                offspring.genes.push((*gene).clone());
            }
        }
    }

    // NEAT crossover for connections
    neat_crossover_connections(&mut offspring, fitter, weaker, config);

    // Copy nodes from fitter parent
    offspring.inputs = fitter.inputs.clone();
    offspring.outputs = fitter.outputs.clone();
    offspring.hidden = fitter.hidden.clone();

    offspring
}

/// Average two genes
fn average_genes(g1: &Gene, g2: &Gene) -> Gene {
    let mut averaged = g1.clone();

    // Average expression
    averaged.expression = (g1.expression + g2.expression) / 2.0;

    // For codons, randomly mix
    let len = g1.codons.len().min(g2.codons.len());
    for i in 0..len {
        if rand_f64() < 0.5 {
            averaged.codons[i] = g2.codons[i].clone();
        }
    }

    averaged
}

/// Crossover connections (uniform)
fn crossover_connections(offspring: &mut CodeGenome, parent1: &CodeGenome, parent2: &CodeGenome) {
    let c1_map: BTreeMap<(u64, u64), &Connection> = parent1
        .connections
        .iter()
        .map(|c| ((c.from.0, c.to.0), c))
        .collect();
    let c2_map: BTreeMap<(u64, u64), &Connection> = parent2
        .connections
        .iter()
        .map(|c| ((c.from.0, c.to.0), c))
        .collect();

    let all_keys: alloc::collections::BTreeSet<(u64, u64)> =
        c1_map.keys().chain(c2_map.keys()).copied().collect();

    for key in all_keys {
        let conn = match (c1_map.get(&key), c2_map.get(&key)) {
            (Some(c1), Some(c2)) => {
                // Matching - choose randomly
                if rand_f64() < 0.5 {
                    (*c1).clone()
                } else {
                    (*c2).clone()
                }
            },
            (Some(c), None) | (None, Some(c)) => {
                if rand_f64() < 0.5 {
                    (*c).clone()
                } else {
                    continue;
                }
            },
            (None, None) => continue,
        };

        offspring.connections.push(conn);
    }
}

/// NEAT-style connection crossover
fn neat_crossover_connections(
    offspring: &mut CodeGenome,
    fitter: &CodeGenome,
    weaker: &CodeGenome,
    config: &CrossoverConfig,
) {
    let fitter_map: BTreeMap<u64, &Connection> = fitter
        .connections
        .iter()
        .map(|c| (c.innovation, c))
        .collect();
    let weaker_map: BTreeMap<u64, &Connection> = weaker
        .connections
        .iter()
        .map(|c| (c.innovation, c))
        .collect();

    let max_weaker = weaker
        .connections
        .iter()
        .map(|c| c.innovation)
        .max()
        .unwrap_or(0);

    for (innovation, conn) in &fitter_map {
        if let Some(weaker_conn) = weaker_map.get(innovation) {
            // Matching connection
            let mut chosen = if rand_f64() < config.fitter_bias {
                (*conn).clone()
            } else {
                (*weaker_conn).clone()
            };

            // Disable if either parent has it disabled
            if !conn.enabled || !weaker_conn.enabled {
                chosen.enabled = rand_f64() > 0.25; // 75% chance disabled
            }

            offspring.connections.push(chosen);
        } else if *innovation > max_weaker {
            // Excess
            if rand_f64() < config.excess_rate {
                offspring.connections.push((*conn).clone());
            }
        } else {
            // Disjoint
            if rand_f64() < config.disjoint_rate {
                offspring.connections.push((*conn).clone());
            }
        }
    }
}

/// Fitness-weighted crossover
pub fn fitness_weighted_crossover(
    parent1: &CodeGenome,
    parent2: &CodeGenome,
    fitness1: &Fitness,
    fitness2: &Fitness,
) -> CodeGenome {
    let mut offspring = CodeGenome::new(rand_u64());

    let total_fitness = fitness1.scalar + fitness2.scalar;
    let p1_prob = if total_fitness > 0.0 {
        fitness1.scalar / total_fitness
    } else {
        0.5
    };

    let p1_map: BTreeMap<u64, &Gene> = parent1.genes.iter().map(|g| (g.innovation, g)).collect();
    let p2_map: BTreeMap<u64, &Gene> = parent2.genes.iter().map(|g| (g.innovation, g)).collect();

    let all_innovations: alloc::collections::BTreeSet<u64> =
        p1_map.keys().chain(p2_map.keys()).copied().collect();

    for innovation in all_innovations {
        let gene = match (p1_map.get(&innovation), p2_map.get(&innovation)) {
            (Some(g1), Some(g2)) => {
                if rand_f64() < p1_prob {
                    (*g1).clone()
                } else {
                    (*g2).clone()
                }
            },
            (Some(g), None) => {
                if rand_f64() < p1_prob {
                    (*g).clone()
                } else {
                    continue;
                }
            },
            (None, Some(g)) => {
                if rand_f64() >= p1_prob {
                    (*g).clone()
                } else {
                    continue;
                }
            },
            (None, None) => continue,
        };

        offspring.genes.push(gene);
    }

    offspring
}

/// Semantic crossover (preserve functional units)
pub fn semantic_crossover(parent1: &CodeGenome, parent2: &CodeGenome) -> CodeGenome {
    let mut offspring = CodeGenome::new(rand_u64());

    // Group genes by type
    let mut p1_by_type: BTreeMap<u8, Vec<&Gene>> = BTreeMap::new();
    let mut p2_by_type: BTreeMap<u8, Vec<&Gene>> = BTreeMap::new();

    for gene in &parent1.genes {
        p1_by_type
            .entry(gene.gene_type as u8)
            .or_insert_with(Vec::new)
            .push(gene);
    }

    for gene in &parent2.genes {
        p2_by_type
            .entry(gene.gene_type as u8)
            .or_insert_with(Vec::new)
            .push(gene);
    }

    // For each gene type, do uniform crossover within type
    let all_types: alloc::collections::BTreeSet<u8> = p1_by_type
        .keys()
        .chain(p2_by_type.keys())
        .copied()
        .collect();

    for gene_type in all_types {
        let genes1 = p1_by_type
            .get(&gene_type)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let genes2 = p2_by_type
            .get(&gene_type)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        // Take some from each parent
        let count = (genes1.len() + genes2.len()) / 2;

        for _ in 0..count {
            let source = if rand_f64() < 0.5 && !genes1.is_empty() {
                genes1
            } else if !genes2.is_empty() {
                genes2
            } else if !genes1.is_empty() {
                genes1
            } else {
                continue;
            };

            let idx = rand_usize(source.len());
            offspring.genes.push(source[idx].clone());
        }
    }

    offspring
}

// ============================================================================
// CROSSOVER ENGINE
// ============================================================================

/// Crossover engine
pub struct CrossoverEngine {
    /// Configuration
    config: CrossoverConfig,
    /// Statistics
    stats: CrossoverStats,
}

/// Crossover statistics
#[derive(Debug, Clone, Default)]
pub struct CrossoverStats {
    pub uniform: u64,
    pub single_point: u64,
    pub two_point: u64,
    pub neat: u64,
    pub fitness_weighted: u64,
    pub semantic: u64,
}

impl CrossoverEngine {
    /// Create new engine
    pub fn new(config: CrossoverConfig) -> Self {
        Self {
            config,
            stats: CrossoverStats::default(),
        }
    }

    /// Perform crossover
    pub fn crossover(
        &mut self,
        parent1: &CodeGenome,
        parent2: &CodeGenome,
        fitness1: Option<&Fitness>,
        fitness2: Option<&Fitness>,
    ) -> CodeGenome {
        match self.config.crossover_type {
            CrossoverType::Uniform => {
                self.stats.uniform += 1;
                uniform_crossover(parent1, parent2)
            },
            CrossoverType::SinglePoint => {
                self.stats.single_point += 1;
                single_point_crossover(parent1, parent2)
            },
            CrossoverType::TwoPoint => {
                self.stats.two_point += 1;
                two_point_crossover(parent1, parent2)
            },
            CrossoverType::Neat => {
                self.stats.neat += 1;
                if let (Some(f1), Some(f2)) = (fitness1, fitness2) {
                    neat_crossover(parent1, parent2, f1, f2, &self.config)
                } else {
                    uniform_crossover(parent1, parent2)
                }
            },
            CrossoverType::FitnessWeighted => {
                self.stats.fitness_weighted += 1;
                if let (Some(f1), Some(f2)) = (fitness1, fitness2) {
                    fitness_weighted_crossover(parent1, parent2, f1, f2)
                } else {
                    uniform_crossover(parent1, parent2)
                }
            },
            CrossoverType::Semantic => {
                self.stats.semantic += 1;
                semantic_crossover(parent1, parent2)
            },
            CrossoverType::Homologous => {
                // Use NEAT-style as default homologous
                self.stats.neat += 1;
                if let (Some(f1), Some(f2)) = (fitness1, fitness2) {
                    neat_crossover(parent1, parent2, f1, f2, &self.config)
                } else {
                    uniform_crossover(parent1, parent2)
                }
            },
        }
    }

    /// Get statistics
    pub fn stats(&self) -> &CrossoverStats {
        &self.stats
    }
}

impl Default for CrossoverEngine {
    fn default() -> Self {
        Self::new(CrossoverConfig::default())
    }
}

// ============================================================================
// RANDOM HELPERS
// ============================================================================

use core::sync::atomic::{AtomicU64, Ordering};

static CROSSOVER_SEED: AtomicU64 = AtomicU64::new(24680);

fn rand_u64() -> u64 {
    let mut current = CROSSOVER_SEED.load(Ordering::Relaxed);
    loop {
        let next = current.wrapping_mul(6364136223846793005).wrapping_add(1);
        match CROSSOVER_SEED.compare_exchange_weak(
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

    #[test]
    fn test_uniform_crossover() {
        let counter = AtomicU64::new(1);
        let p1 = CodeGenome::random(1, 10, &counter);
        let p2 = CodeGenome::random(2, 10, &counter);

        let offspring = uniform_crossover(&p1, &p2);
        assert!(!offspring.genes.is_empty());
    }

    #[test]
    fn test_single_point_crossover() {
        let counter = AtomicU64::new(1);
        let p1 = CodeGenome::random(1, 10, &counter);
        let p2 = CodeGenome::random(2, 10, &counter);

        let offspring = single_point_crossover(&p1, &p2);
        assert!(!offspring.genes.is_empty());
    }

    #[test]
    fn test_neat_crossover() {
        let counter = AtomicU64::new(1);
        let p1 = CodeGenome::random(1, 10, &counter);
        let p2 = CodeGenome::random(2, 10, &counter);
        let f1 = Fitness::new(vec![1.0, 0.8]);
        let f2 = Fitness::new(vec![0.9, 0.7]);
        let config = CrossoverConfig::default();

        let offspring = neat_crossover(&p1, &p2, &f1, &f2, &config);
        assert!(!offspring.genes.is_empty());
    }

    #[test]
    fn test_crossover_engine() {
        let mut engine = CrossoverEngine::default();
        let counter = AtomicU64::new(1);
        let p1 = CodeGenome::random(1, 10, &counter);
        let p2 = CodeGenome::random(2, 10, &counter);

        let offspring = engine.crossover(&p1, &p2, None, None);
        assert!(!offspring.genes.is_empty());
    }
}
