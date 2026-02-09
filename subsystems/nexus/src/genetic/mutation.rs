//! # Mutation Operators
//!
//! Year 3 EVOLUTION - Mutation operators for code genome modification
//! Introduces genetic variation through controlled code changes.

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use super::genome::{CodeGenome, Codon, ControlCodon, Gene, GeneId};

// ============================================================================
// MUTATION TYPES
// ============================================================================

/// Mutation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationType {
    /// Point mutation (single codon change)
    Point,
    /// Insertion (add codon/gene)
    Insertion,
    /// Deletion (remove codon/gene)
    Deletion,
    /// Duplication (copy gene)
    Duplication,
    /// Inversion (reverse sequence)
    Inversion,
    /// Translocation (move gene)
    Translocation,
    /// Swap (exchange two elements)
    Swap,
    /// Expression change (modify gene expression)
    Expression,
    /// Enable/disable gene
    Toggle,
    /// Structural (add/remove node)
    Structural,
    /// Regulatory (change gene interactions)
    Regulatory,
}

/// Mutation configuration
#[derive(Debug, Clone)]
pub struct MutationConfig {
    /// Point mutation rate
    pub point_rate: f64,
    /// Insertion rate
    pub insertion_rate: f64,
    /// Deletion rate
    pub deletion_rate: f64,
    /// Duplication rate
    pub duplication_rate: f64,
    /// Inversion rate
    pub inversion_rate: f64,
    /// Swap rate
    pub swap_rate: f64,
    /// Expression mutation rate
    pub expression_rate: f64,
    /// Toggle rate
    pub toggle_rate: f64,
    /// Structural mutation rate
    pub structural_rate: f64,
    /// Maximum mutations per genome
    pub max_mutations: usize,
    /// Preserve minimum genes
    pub min_genes: usize,
}

impl Default for MutationConfig {
    fn default() -> Self {
        Self {
            point_rate: 0.3,
            insertion_rate: 0.1,
            deletion_rate: 0.1,
            duplication_rate: 0.05,
            inversion_rate: 0.02,
            swap_rate: 0.05,
            expression_rate: 0.2,
            toggle_rate: 0.05,
            structural_rate: 0.02,
            max_mutations: 5,
            min_genes: 1,
        }
    }
}

/// Mutation result
#[derive(Debug, Clone)]
pub struct MutationResult {
    /// Mutation types applied
    pub mutations: Vec<MutationType>,
    /// Genes affected
    pub affected_genes: Vec<GeneId>,
    /// Success
    pub success: bool,
}

// ============================================================================
// MUTATION OPERATORS
// ============================================================================

/// Apply point mutation
pub fn point_mutation(genome: &CodeGenome) -> CodeGenome {
    let mut mutated = genome.clone();

    if mutated.genes.is_empty() {
        return mutated;
    }

    // Select random gene
    let gene_idx = rand_usize(mutated.genes.len());
    let gene = &mut mutated.genes[gene_idx];

    if gene.codons.is_empty() {
        return mutated;
    }

    // Select random codon
    let codon_idx = rand_usize(gene.codons.len());

    // Mutate codon
    gene.codons[codon_idx] = mutate_codon(&gene.codons[codon_idx]);
    gene.metadata.mutations += 1;

    mutated
}

/// Mutate a single codon
fn mutate_codon(codon: &Codon) -> Codon {
    match codon {
        Codon::Op(op) => {
            // Small change to opcode
            let delta = (rand_i32() % 5) - 2;
            Codon::Op(((*op as i32 + delta).clamp(0, 255)) as u8)
        },
        Codon::Reg(reg) => {
            // Change register
            Codon::Reg(((*reg as i32 + rand_i32() % 3 - 1).clamp(0, 15)) as u8)
        },
        Codon::Imm(val) => {
            // Small change to immediate
            let mutation_type = rand_usize(4);
            match mutation_type {
                0 => Codon::Imm(val + rand_i32() % 100 - 50), // Small delta
                1 => Codon::Imm(val * 2),                     // Double
                2 => Codon::Imm(val / 2),                     // Half
                _ => Codon::Imm(!val),                        // Flip bits
            }
        },
        Codon::Addr(addr) => {
            let delta = rand_i32() % 64 - 32;
            Codon::Addr(addr + delta)
        },
        Codon::Label(label) => {
            let delta = (rand_i32() % 10 - 5) as i16;
            Codon::Label(((*label as i16 + delta).max(0)) as u16)
        },
        Codon::Type(t) => Codon::Type(((*t as i32 + rand_i32() % 3 - 1).clamp(0, 15)) as u8),
        Codon::Control(_) => {
            // Randomly change control type
            let controls = [
                ControlCodon::BlockBegin,
                ControlCodon::BlockEnd,
                ControlCodon::Branch,
                ControlCodon::LoopBegin,
                ControlCodon::LoopEnd,
            ];
            Codon::Control(controls[rand_usize(controls.len())])
        },
        Codon::Nop => {
            // Sometimes replace NOP with actual instruction
            if rand_f64() < 0.5 {
                Codon::Op(rand_u8() % 64)
            } else {
                Codon::Nop
            }
        },
    }
}

/// Apply insertion mutation
pub fn insertion_mutation(genome: &CodeGenome) -> CodeGenome {
    let mut mutated = genome.clone();

    let mutation_type = rand_usize(2);

    match mutation_type {
        0 => {
            // Insert codon in existing gene
            if !mutated.genes.is_empty() {
                let gene_idx = rand_usize(mutated.genes.len());
                let gene = &mut mutated.genes[gene_idx];
                let insert_idx = rand_usize(gene.codons.len() + 1);
                let new_codon = random_codon();
                gene.codons.insert(insert_idx, new_codon);
                gene.metadata.mutations += 1;
            }
        },
        1 => {
            // Insert new gene
            let new_gene = Gene::random(GeneId(rand_u64()), rand_u64());
            let insert_idx = rand_usize(mutated.genes.len() + 1);
            mutated.genes.insert(insert_idx, new_gene);
        },
        _ => {},
    }

    mutated
}

/// Apply deletion mutation
pub fn deletion_mutation(genome: &CodeGenome, min_genes: usize) -> CodeGenome {
    let mut mutated = genome.clone();

    if mutated.genes.len() <= min_genes {
        return mutated;
    }

    let mutation_type = rand_usize(2);

    match mutation_type {
        0 => {
            // Delete codon
            if !mutated.genes.is_empty() {
                let gene_idx = rand_usize(mutated.genes.len());
                let gene = &mut mutated.genes[gene_idx];
                if gene.codons.len() > 1 {
                    let del_idx = rand_usize(gene.codons.len());
                    gene.codons.remove(del_idx);
                    gene.metadata.mutations += 1;
                }
            }
        },
        1 => {
            // Delete gene
            let del_idx = rand_usize(mutated.genes.len());
            mutated.genes.remove(del_idx);
        },
        _ => {},
    }

    mutated
}

/// Apply duplication mutation
pub fn duplication_mutation(genome: &CodeGenome) -> CodeGenome {
    let mut mutated = genome.clone();

    if mutated.genes.is_empty() {
        return mutated;
    }

    let gene_idx = rand_usize(mutated.genes.len());
    let mut duplicated = mutated.genes[gene_idx].clone();

    // Give new ID and innovation number
    duplicated.id = GeneId(rand_u64());
    duplicated.innovation = rand_u64();
    duplicated.metadata.mutations = 0;

    // Maybe slight mutation
    if rand_f64() < 0.5 && !duplicated.codons.is_empty() {
        let codon_idx = rand_usize(duplicated.codons.len());
        duplicated.codons[codon_idx] = mutate_codon(&duplicated.codons[codon_idx]);
    }

    let insert_idx = rand_usize(mutated.genes.len() + 1);
    mutated.genes.insert(insert_idx, duplicated);

    mutated
}

/// Apply inversion mutation
pub fn inversion_mutation(genome: &CodeGenome) -> CodeGenome {
    let mut mutated = genome.clone();

    if mutated.genes.is_empty() {
        return mutated;
    }

    let gene_idx = rand_usize(mutated.genes.len());
    let gene = &mut mutated.genes[gene_idx];

    if gene.codons.len() < 2 {
        return mutated;
    }

    // Select range to invert
    let start = rand_usize(gene.codons.len() - 1);
    let end = start + 1 + rand_usize(gene.codons.len() - start - 1);

    // Reverse the range
    gene.codons[start..=end].reverse();
    gene.metadata.mutations += 1;

    mutated
}

/// Apply swap mutation
pub fn swap_mutation(genome: &CodeGenome) -> CodeGenome {
    let mut mutated = genome.clone();

    if mutated.genes.len() < 2 {
        return mutated;
    }

    let mutation_type = rand_usize(2);

    match mutation_type {
        0 => {
            // Swap genes
            let idx1 = rand_usize(mutated.genes.len());
            let mut idx2 = rand_usize(mutated.genes.len());
            while idx2 == idx1 {
                idx2 = rand_usize(mutated.genes.len());
            }
            mutated.genes.swap(idx1, idx2);
        },
        1 => {
            // Swap codons within gene
            let gene_idx = rand_usize(mutated.genes.len());
            let gene = &mut mutated.genes[gene_idx];
            if gene.codons.len() >= 2 {
                let idx1 = rand_usize(gene.codons.len());
                let mut idx2 = rand_usize(gene.codons.len());
                while idx2 == idx1 {
                    idx2 = rand_usize(gene.codons.len());
                }
                gene.codons.swap(idx1, idx2);
                gene.metadata.mutations += 1;
            }
        },
        _ => {},
    }

    mutated
}

/// Apply expression mutation
pub fn expression_mutation(genome: &CodeGenome) -> CodeGenome {
    let mut mutated = genome.clone();

    if mutated.genes.is_empty() {
        return mutated;
    }

    let gene_idx = rand_usize(mutated.genes.len());
    let gene = &mut mutated.genes[gene_idx];

    // Modify expression level
    let delta = (rand_f64() - 0.5) * 0.4; // ±0.2
    gene.expression = (gene.expression + delta).clamp(0.0, 1.0);

    mutated
}

/// Apply toggle mutation (enable/disable gene)
pub fn toggle_mutation(genome: &CodeGenome) -> CodeGenome {
    let mut mutated = genome.clone();

    if mutated.genes.is_empty() {
        return mutated;
    }

    let gene_idx = rand_usize(mutated.genes.len());
    mutated.genes[gene_idx].enabled = !mutated.genes[gene_idx].enabled;

    mutated
}

/// Apply structural mutation (add/remove nodes in graph genome)
pub fn structural_mutation(genome: &CodeGenome) -> CodeGenome {
    let mut mutated = genome.clone();

    let mutation_type = rand_usize(3);

    match mutation_type {
        0 => {
            // Add hidden node
            let new_node = super::genome::NodeId(rand_u64());
            mutated.hidden.push(new_node);

            // Connect to random existing nodes
            if !mutated.inputs.is_empty() {
                let from = mutated.inputs[rand_usize(mutated.inputs.len())];
                mutated.connections.push(super::genome::Connection {
                    from,
                    to: new_node,
                    weight: rand_f64() * 2.0 - 1.0,
                    innovation: rand_u64(),
                    enabled: true,
                });
            }

            if !mutated.outputs.is_empty() {
                let to = mutated.outputs[rand_usize(mutated.outputs.len())];
                mutated.connections.push(super::genome::Connection {
                    from: new_node,
                    to,
                    weight: rand_f64() * 2.0 - 1.0,
                    innovation: rand_u64(),
                    enabled: true,
                });
            }
        },
        1 => {
            // Add connection
            let all_nodes: Vec<super::genome::NodeId> = mutated
                .inputs
                .iter()
                .chain(mutated.hidden.iter())
                .chain(mutated.outputs.iter())
                .copied()
                .collect();

            if all_nodes.len() >= 2 {
                let from = all_nodes[rand_usize(all_nodes.len())];
                let to = all_nodes[rand_usize(all_nodes.len())];

                // Check if connection already exists
                let exists = mutated
                    .connections
                    .iter()
                    .any(|c| c.from == from && c.to == to);

                if !exists && from != to {
                    mutated.connections.push(super::genome::Connection {
                        from,
                        to,
                        weight: rand_f64() * 2.0 - 1.0,
                        innovation: rand_u64(),
                        enabled: true,
                    });
                }
            }
        },
        2 => {
            // Mutate connection weight
            if !mutated.connections.is_empty() {
                let conn_idx = rand_usize(mutated.connections.len());
                let conn = &mut mutated.connections[conn_idx];

                if rand_f64() < 0.1 {
                    // Complete reset
                    conn.weight = rand_f64() * 2.0 - 1.0;
                } else {
                    // Perturb
                    conn.weight += (rand_f64() - 0.5) * 0.5;
                    conn.weight = conn.weight.clamp(-5.0, 5.0);
                }
            }
        },
        _ => {},
    }

    mutated
}

/// Generate random codon
fn random_codon() -> Codon {
    match rand_usize(8) {
        0 => Codon::Op(rand_u8() % 64),
        1 => Codon::Reg(rand_u8() % 16),
        2 => Codon::Imm(rand_i32()),
        3 => Codon::Addr(rand_i32() % 1024),
        4 => Codon::Label(rand_u16()),
        5 => Codon::Type(rand_u8() % 16),
        6 => Codon::Control(ControlCodon::Branch),
        _ => Codon::Nop,
    }
}

// ============================================================================
// MUTATION ENGINE
// ============================================================================

/// Mutation engine
pub struct MutationEngine {
    /// Configuration
    config: MutationConfig,
    /// Statistics
    stats: MutationStats,
}

/// Mutation statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct MutationStats {
    pub point_mutations: u64,
    pub insertions: u64,
    pub deletions: u64,
    pub duplications: u64,
    pub inversions: u64,
    pub swaps: u64,
    pub expression_changes: u64,
    pub toggles: u64,
    pub structural: u64,
}

impl MutationEngine {
    /// Create new engine
    pub fn new(config: MutationConfig) -> Self {
        Self {
            config,
            stats: MutationStats::default(),
        }
    }

    /// Apply mutations to genome
    pub fn mutate(&mut self, genome: &CodeGenome) -> CodeGenome {
        let mut mutated = genome.clone();
        let mut mutations_applied = 0;

        while mutations_applied < self.config.max_mutations {
            let mutation = self.select_mutation();

            mutated = match mutation {
                MutationType::Point if rand_f64() < self.config.point_rate => {
                    self.stats.point_mutations += 1;
                    mutations_applied += 1;
                    point_mutation(&mutated)
                },
                MutationType::Insertion if rand_f64() < self.config.insertion_rate => {
                    self.stats.insertions += 1;
                    mutations_applied += 1;
                    insertion_mutation(&mutated)
                },
                MutationType::Deletion if rand_f64() < self.config.deletion_rate => {
                    self.stats.deletions += 1;
                    mutations_applied += 1;
                    deletion_mutation(&mutated, self.config.min_genes)
                },
                MutationType::Duplication if rand_f64() < self.config.duplication_rate => {
                    self.stats.duplications += 1;
                    mutations_applied += 1;
                    duplication_mutation(&mutated)
                },
                MutationType::Inversion if rand_f64() < self.config.inversion_rate => {
                    self.stats.inversions += 1;
                    mutations_applied += 1;
                    inversion_mutation(&mutated)
                },
                MutationType::Swap if rand_f64() < self.config.swap_rate => {
                    self.stats.swaps += 1;
                    mutations_applied += 1;
                    swap_mutation(&mutated)
                },
                MutationType::Expression if rand_f64() < self.config.expression_rate => {
                    self.stats.expression_changes += 1;
                    mutations_applied += 1;
                    expression_mutation(&mutated)
                },
                MutationType::Toggle if rand_f64() < self.config.toggle_rate => {
                    self.stats.toggles += 1;
                    mutations_applied += 1;
                    toggle_mutation(&mutated)
                },
                MutationType::Structural if rand_f64() < self.config.structural_rate => {
                    self.stats.structural += 1;
                    mutations_applied += 1;
                    structural_mutation(&mutated)
                },
                _ => break,
            };
        }

        mutated
    }

    fn select_mutation(&self) -> MutationType {
        let mutations = [
            MutationType::Point,
            MutationType::Insertion,
            MutationType::Deletion,
            MutationType::Duplication,
            MutationType::Inversion,
            MutationType::Swap,
            MutationType::Expression,
            MutationType::Toggle,
            MutationType::Structural,
        ];
        mutations[rand_usize(mutations.len())]
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &MutationStats {
        &self.stats
    }
}

impl Default for MutationEngine {
    fn default() -> Self {
        Self::new(MutationConfig::default())
    }
}

// ============================================================================
// RANDOM HELPERS
// ============================================================================

use core::sync::atomic::{AtomicU64, Ordering};

static MUTATION_SEED: AtomicU64 = AtomicU64::new(13579);

fn rand_u64() -> u64 {
    let mut current = MUTATION_SEED.load(Ordering::Relaxed);
    loop {
        let next = current.wrapping_mul(6364136223846793005).wrapping_add(1);
        match MUTATION_SEED.compare_exchange_weak(
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

fn rand_u8() -> u8 {
    rand_u64() as u8
}
fn rand_u16() -> u16 {
    rand_u64() as u16
}
fn rand_i32() -> i32 {
    rand_u64() as i32
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
    fn test_point_mutation() {
        let counter = AtomicU64::new(1);
        let genome = CodeGenome::random(1, 5, &counter);
        let mutated = point_mutation(&genome);
        assert_eq!(mutated.size(), genome.size());
    }

    #[test]
    fn test_insertion_mutation() {
        let counter = AtomicU64::new(1);
        let genome = CodeGenome::random(1, 5, &counter);
        let mutated = insertion_mutation(&genome);
        // Size may increase
        assert!(mutated.size() >= genome.size());
    }

    #[test]
    fn test_deletion_mutation() {
        let counter = AtomicU64::new(1);
        let genome = CodeGenome::random(1, 5, &counter);
        let mutated = deletion_mutation(&genome, 1);
        // Size may decrease
        assert!(mutated.size() <= genome.size());
    }

    #[test]
    fn test_mutation_engine() {
        let mut engine = MutationEngine::default();
        let counter = AtomicU64::new(1);
        let genome = CodeGenome::random(1, 10, &counter);
        let mutated = engine.mutate(&genome);
        // Mutation happened
        assert!(
            engine.stats.point_mutations > 0
                || engine.stats.insertions > 0
                || engine.stats.deletions > 0
                || engine.stats.expression_changes > 0
        );
    }
}
