//! # Code Genome
//!
//! Year 3 EVOLUTION - Genome representation for code evolution
//! Encodes programs as evolvable genetic structures.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// GENE TYPES
// ============================================================================

/// Gene ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GeneId(pub u64);

/// Codon (basic unit of genetic information)
#[derive(Debug, Clone, Copy)]
pub enum Codon {
    /// Instruction opcode
    Op(u8),
    /// Register reference
    Reg(u8),
    /// Immediate value
    Imm(i32),
    /// Address offset
    Addr(i32),
    /// Label reference
    Label(u16),
    /// Type tag
    Type(u8),
    /// Control flow marker
    Control(ControlCodon),
    /// No operation
    Nop,
}

/// Control flow codons
#[derive(Debug, Clone, Copy)]
pub enum ControlCodon {
    /// Begin block
    BlockBegin,
    /// End block
    BlockEnd,
    /// Branch marker
    Branch,
    /// Loop begin
    LoopBegin,
    /// Loop end
    LoopEnd,
    /// Function boundary
    FuncBoundary,
}

/// Gene - a sequence of codons representing a code fragment
#[derive(Debug, Clone)]
pub struct Gene {
    /// Gene ID
    pub id: GeneId,
    /// Gene type
    pub gene_type: GeneType,
    /// Codon sequence
    pub codons: Vec<Codon>,
    /// Expression level (probability of activation)
    pub expression: f64,
    /// Innovation number (for NEAT)
    pub innovation: u64,
    /// Enabled
    pub enabled: bool,
    /// Metadata
    pub metadata: GeneMetadata,
}

/// Gene type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneType {
    /// Arithmetic operation
    Arithmetic,
    /// Memory operation
    Memory,
    /// Control flow
    Control,
    /// Function call
    Call,
    /// Constant/literal
    Constant,
    /// Variable declaration
    Variable,
    /// Type definition
    TypeDef,
    /// Regulatory (affects other genes)
    Regulatory,
}

/// Gene metadata
#[derive(Debug, Clone)]
pub struct GeneMetadata {
    /// Source location (if derived from existing code)
    pub source: Option<String>,
    /// Fitness contribution
    pub fitness_contribution: f64,
    /// Mutation count
    pub mutations: u32,
    /// Age (generations)
    pub age: u32,
}

impl Default for GeneMetadata {
    fn default() -> Self {
        Self {
            source: None,
            fitness_contribution: 0.0,
            mutations: 0,
            age: 0,
        }
    }
}

// ============================================================================
// CODE GENOME
// ============================================================================

/// Code genome - complete genetic representation of a program
#[derive(Debug, Clone)]
pub struct CodeGenome {
    /// Genome ID
    pub id: u64,
    /// Genes
    pub genes: Vec<Gene>,
    /// Gene connections (for graph-based genomes)
    pub connections: Vec<Connection>,
    /// Input nodes
    pub inputs: Vec<NodeId>,
    /// Output nodes
    pub outputs: Vec<NodeId>,
    /// Hidden nodes
    pub hidden: Vec<NodeId>,
    /// Global fitness
    pub fitness: f64,
    /// Complexity measure
    pub complexity: f64,
}

/// Node ID for graph-based genome
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u64);

/// Connection between nodes
#[derive(Debug, Clone)]
pub struct Connection {
    /// Source node
    pub from: NodeId,
    /// Target node
    pub to: NodeId,
    /// Weight
    pub weight: f64,
    /// Innovation number
    pub innovation: u64,
    /// Enabled
    pub enabled: bool,
}

impl CodeGenome {
    /// Create empty genome
    pub fn new(id: u64) -> Self {
        Self {
            id,
            genes: Vec::new(),
            connections: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            hidden: Vec::new(),
            fitness: 0.0,
            complexity: 0.0,
        }
    }

    /// Create random genome
    pub fn random(id: u64, gene_count: usize, innovation_counter: &AtomicU64) -> Self {
        let mut genome = Self::new(id);

        for _ in 0..gene_count {
            let gene = Gene::random(
                GeneId(innovation_counter.fetch_add(1, Ordering::Relaxed)),
                innovation_counter.fetch_add(1, Ordering::Relaxed),
            );
            genome.genes.push(gene);
        }

        genome.update_complexity();
        genome
    }

    /// Add gene
    pub fn add_gene(&mut self, gene: Gene) {
        self.genes.push(gene);
        self.update_complexity();
    }

    /// Remove gene
    pub fn remove_gene(&mut self, id: GeneId) -> Option<Gene> {
        if let Some(idx) = self.genes.iter().position(|g| g.id == id) {
            let gene = self.genes.remove(idx);
            self.update_complexity();
            Some(gene)
        } else {
            None
        }
    }

    /// Get gene by ID
    pub fn get_gene(&self, id: GeneId) -> Option<&Gene> {
        self.genes.iter().find(|g| g.id == id)
    }

    /// Get gene by ID (mutable)
    pub fn get_gene_mut(&mut self, id: GeneId) -> Option<&mut Gene> {
        self.genes.iter_mut().find(|g| g.id == id)
    }

    /// Enable gene
    pub fn enable_gene(&mut self, id: GeneId) {
        if let Some(gene) = self.get_gene_mut(id) {
            gene.enabled = true;
        }
    }

    /// Disable gene
    pub fn disable_gene(&mut self, id: GeneId) {
        if let Some(gene) = self.get_gene_mut(id) {
            gene.enabled = false;
        }
    }

    /// Get active genes
    pub fn active_genes(&self) -> impl Iterator<Item = &Gene> {
        self.genes.iter().filter(|g| g.enabled)
    }

    /// Add connection
    pub fn add_connection(&mut self, connection: Connection) {
        self.connections.push(connection);
    }

    /// Calculate distance to another genome (for speciation)
    pub fn distance(&self, other: &CodeGenome) -> f64 {
        const C1: f64 = 1.0; // Excess genes coefficient
        const C2: f64 = 1.0; // Disjoint genes coefficient
        const C3: f64 = 0.4; // Weight difference coefficient

        let n = self.genes.len().max(other.genes.len()).max(1) as f64;

        // Count excess and disjoint genes
        let max_self = self.genes.iter().map(|g| g.innovation).max().unwrap_or(0);
        let max_other = other.genes.iter().map(|g| g.innovation).max().unwrap_or(0);

        let threshold = max_self.min(max_other);

        let mut excess = 0;
        let mut disjoint = 0;
        let mut weight_diff = 0.0;
        let mut matching = 0;

        // Create innovation maps
        let self_innovations: BTreeMap<u64, &Gene> =
            self.genes.iter().map(|g| (g.innovation, g)).collect();
        let other_innovations: BTreeMap<u64, &Gene> =
            other.genes.iter().map(|g| (g.innovation, g)).collect();

        // Count matching and compute weight differences
        for (innov, gene) in &self_innovations {
            if let Some(other_gene) = other_innovations.get(innov) {
                matching += 1;
                weight_diff += (gene.expression - other_gene.expression).abs();
            } else if *innov > threshold {
                excess += 1;
            } else {
                disjoint += 1;
            }
        }

        for (innov, _) in &other_innovations {
            if !self_innovations.contains_key(innov) {
                if *innov > threshold {
                    excess += 1;
                } else {
                    disjoint += 1;
                }
            }
        }

        let avg_weight_diff = if matching > 0 {
            weight_diff / matching as f64
        } else {
            0.0
        };

        (C1 * excess as f64 / n) + (C2 * disjoint as f64 / n) + (C3 * avg_weight_diff)
    }

    /// Update complexity measure
    fn update_complexity(&mut self) {
        let gene_complexity: f64 = self
            .genes
            .iter()
            .filter(|g| g.enabled)
            .map(|g| g.codons.len() as f64)
            .sum();

        let connection_complexity = self.connections.iter().filter(|c| c.enabled).count() as f64;

        self.complexity = gene_complexity + connection_complexity * 0.5;
    }

    /// Get size (number of genes)
    pub fn size(&self) -> usize {
        self.genes.len()
    }

    /// Get active size
    pub fn active_size(&self) -> usize {
        self.genes.iter().filter(|g| g.enabled).count()
    }

    /// Clone with new ID
    pub fn clone_with_id(&self, id: u64) -> Self {
        let mut cloned = self.clone();
        cloned.id = id;
        cloned
    }

    /// Encode genome to bytes
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Header: gene count
        bytes.extend_from_slice(&(self.genes.len() as u32).to_le_bytes());

        // Encode each gene
        for gene in &self.genes {
            bytes.extend_from_slice(&gene.id.0.to_le_bytes());
            bytes.push(gene.gene_type as u8);
            bytes.push(if gene.enabled { 1 } else { 0 });

            // Encode codons
            bytes.extend_from_slice(&(gene.codons.len() as u16).to_le_bytes());
            for codon in &gene.codons {
                match codon {
                    Codon::Op(op) => {
                        bytes.push(0);
                        bytes.push(*op);
                    },
                    Codon::Reg(r) => {
                        bytes.push(1);
                        bytes.push(*r);
                    },
                    Codon::Imm(i) => {
                        bytes.push(2);
                        bytes.extend_from_slice(&i.to_le_bytes());
                    },
                    Codon::Addr(a) => {
                        bytes.push(3);
                        bytes.extend_from_slice(&a.to_le_bytes());
                    },
                    Codon::Label(l) => {
                        bytes.push(4);
                        bytes.extend_from_slice(&l.to_le_bytes());
                    },
                    Codon::Type(t) => {
                        bytes.push(5);
                        bytes.push(*t);
                    },
                    Codon::Control(c) => {
                        bytes.push(6);
                        bytes.push(*c as u8);
                    },
                    Codon::Nop => {
                        bytes.push(7);
                    },
                }
            }
        }

        bytes
    }

    /// Decode genome from bytes
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 4 {
            return None;
        }

        let gene_count = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        let mut offset = 4;
        let mut genes = Vec::with_capacity(gene_count);

        for _ in 0..gene_count {
            if offset + 10 > bytes.len() {
                break;
            }

            let id = GeneId(u64::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]));
            offset += 8;

            let gene_type = match bytes[offset] {
                0 => GeneType::Arithmetic,
                1 => GeneType::Memory,
                2 => GeneType::Control,
                3 => GeneType::Call,
                4 => GeneType::Constant,
                5 => GeneType::Variable,
                6 => GeneType::TypeDef,
                _ => GeneType::Regulatory,
            };
            offset += 1;

            let enabled = bytes[offset] != 0;
            offset += 1;

            let codon_count = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as usize;
            offset += 2;

            let mut codons = Vec::with_capacity(codon_count);
            for _ in 0..codon_count {
                if offset >= bytes.len() {
                    break;
                }

                let codon = match bytes[offset] {
                    0 => {
                        offset += 1;
                        Codon::Op(bytes[offset])
                    },
                    1 => {
                        offset += 1;
                        Codon::Reg(bytes[offset])
                    },
                    2 => {
                        offset += 1;
                        if offset + 4 > bytes.len() {
                            break;
                        }
                        let val = i32::from_le_bytes([
                            bytes[offset],
                            bytes[offset + 1],
                            bytes[offset + 2],
                            bytes[offset + 3],
                        ]);
                        offset += 3;
                        Codon::Imm(val)
                    },
                    7 => Codon::Nop,
                    _ => Codon::Nop,
                };
                offset += 1;
                codons.push(codon);
            }

            genes.push(Gene {
                id,
                gene_type,
                codons,
                expression: 1.0,
                innovation: id.0,
                enabled,
                metadata: GeneMetadata::default(),
            });
        }

        Some(Self {
            id: 0,
            genes,
            connections: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            hidden: Vec::new(),
            fitness: 0.0,
            complexity: 0.0,
        })
    }
}

impl Gene {
    /// Create random gene
    pub fn random(id: GeneId, innovation: u64) -> Self {
        let gene_types = [
            GeneType::Arithmetic,
            GeneType::Memory,
            GeneType::Control,
            GeneType::Call,
            GeneType::Constant,
        ];

        let gene_type = gene_types[rand_usize(gene_types.len())];

        let codons = match gene_type {
            GeneType::Arithmetic => vec![
                Codon::Op(rand_u8() % 8), // add, sub, mul, div, etc.
                Codon::Reg(rand_u8() % 16),
                Codon::Reg(rand_u8() % 16),
            ],
            GeneType::Memory => vec![
                Codon::Op(16 + rand_u8() % 4), // load, store, alloc, free
                Codon::Reg(rand_u8() % 16),
                Codon::Addr((rand_u8() as i32) * 4),
            ],
            GeneType::Control => vec![
                Codon::Control(ControlCodon::Branch),
                Codon::Label(rand_u16()),
            ],
            GeneType::Call => vec![
                Codon::Op(32), // call opcode
                Codon::Label(rand_u16()),
            ],
            GeneType::Constant => vec![Codon::Imm(rand_i32())],
            _ => vec![Codon::Nop],
        };

        Self {
            id,
            gene_type,
            codons,
            expression: rand_f64(),
            innovation,
            enabled: true,
            metadata: GeneMetadata::default(),
        }
    }

    /// Mutate gene
    pub fn mutate(&mut self) {
        let mutation_type = rand_usize(4);

        match mutation_type {
            0 => {
                // Mutate expression level
                self.expression = (self.expression + rand_f64() * 0.2 - 0.1).clamp(0.0, 1.0);
            },
            1 => {
                // Mutate codon
                if !self.codons.is_empty() {
                    let idx = rand_usize(self.codons.len());
                    self.codons[idx] = self.random_codon();
                }
            },
            2 => {
                // Insert codon
                let codon = self.random_codon();
                let idx = rand_usize(self.codons.len() + 1);
                self.codons.insert(idx, codon);
            },
            3 => {
                // Delete codon
                if self.codons.len() > 1 {
                    let idx = rand_usize(self.codons.len());
                    self.codons.remove(idx);
                }
            },
            _ => {},
        }

        self.metadata.mutations += 1;
    }

    fn random_codon(&self) -> Codon {
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

    /// Age the gene
    pub fn age(&mut self) {
        self.metadata.age += 1;
    }
}

// ============================================================================
// RANDOM HELPERS
// ============================================================================

static mut GENOME_SEED: u64 = 67890;

fn rand_u64() -> u64 {
    unsafe {
        GENOME_SEED = GENOME_SEED
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        GENOME_SEED
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
    use super::*;

    #[test]
    fn test_genome_creation() {
        let counter = AtomicU64::new(1);
        let genome = CodeGenome::random(1, 10, &counter);
        assert_eq!(genome.size(), 10);
    }

    #[test]
    fn test_genome_distance() {
        let counter = AtomicU64::new(1);
        let g1 = CodeGenome::random(1, 10, &counter);
        let g2 = CodeGenome::random(2, 10, &counter);
        let distance = g1.distance(&g2);
        assert!(distance >= 0.0);
    }

    #[test]
    fn test_gene_mutation() {
        let mut gene = Gene::random(GeneId(1), 1);
        let original_len = gene.codons.len();
        gene.mutate();
        // Mutation happened
        assert!(gene.metadata.mutations > 0);
    }

    #[test]
    fn test_genome_encode_decode() {
        let counter = AtomicU64::new(1);
        let genome = CodeGenome::random(1, 5, &counter);
        let bytes = genome.encode();
        let decoded = CodeGenome::decode(&bytes);
        assert!(decoded.is_some());
        assert_eq!(decoded.unwrap().size(), 5);
    }
}
