//! # Genome Encoding
//!
//! Year 3 EVOLUTION - Various encoding schemes for genetic algorithms

#![allow(dead_code)]

extern crate alloc;
use alloc::format;
use alloc::vec;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::genome::{Chromosome, ChromosomeId, Gene, GeneId, GeneType};
use super::operators::Rng;

// ============================================================================
// ENCODING TRAIT
// ============================================================================

/// Encoding scheme trait
pub trait Encoding: Send + Sync {
    /// Decode chromosome to phenotype
    fn decode(&self, chromosome: &Chromosome) -> Phenotype;

    /// Encode phenotype to chromosome
    fn encode(&self, phenotype: &Phenotype) -> Chromosome;

    /// Get encoding name
    fn name(&self) -> &str;

    /// Get gene count for this encoding
    fn gene_count(&self) -> usize;
}

/// Phenotype (decoded representation)
#[derive(Debug, Clone)]
pub struct Phenotype {
    /// Decoded values
    pub values: BTreeMap<String, PhenotypeValue>,
    /// Validity
    pub valid: bool,
}

/// Phenotype value
#[derive(Debug, Clone)]
pub enum PhenotypeValue {
    Float(f64),
    Integer(i64),
    Boolean(bool),
    String(String),
    Vector(Vec<f64>),
    Matrix(Vec<Vec<f64>>),
    Tree(TreeNode),
    Graph(GraphNode),
    Custom(Vec<u8>),
}

/// Tree node for tree encoding
#[derive(Debug, Clone)]
pub struct TreeNode {
    /// Node type
    pub node_type: TreeNodeType,
    /// Children
    pub children: Vec<TreeNode>,
    /// Value (for terminals)
    pub value: Option<f64>,
}

/// Tree node type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeNodeType {
    // Functions
    Add,
    Sub,
    Mul,
    Div,
    Sin,
    Cos,
    Exp,
    Log,
    Sqrt,
    Pow,
    If,
    // Terminals
    Constant,
    Variable(usize),
}

/// Graph node for graph encoding
#[derive(Debug, Clone)]
pub struct GraphNode {
    /// Node ID
    pub id: u64,
    /// Node type
    pub node_type: u32,
    /// Edges (to node IDs)
    pub edges: Vec<u64>,
    /// Attributes
    pub attributes: BTreeMap<String, f64>,
}

// ============================================================================
// BINARY ENCODING
// ============================================================================

/// Binary encoding
pub struct BinaryEncoding {
    /// Number of bits
    bits: usize,
    /// Variable ranges
    ranges: Vec<(f64, f64)>,
}

impl BinaryEncoding {
    pub fn new(bits_per_var: usize, ranges: Vec<(f64, f64)>) -> Self {
        Self {
            bits: bits_per_var * ranges.len(),
            ranges,
        }
    }

    fn bits_per_var(&self) -> usize {
        if self.ranges.is_empty() {
            0
        } else {
            self.bits / self.ranges.len()
        }
    }
}

impl Encoding for BinaryEncoding {
    fn decode(&self, chromosome: &Chromosome) -> Phenotype {
        let bits_per = self.bits_per_var();
        let mut values = BTreeMap::new();

        for (var_idx, &(min, max)) in self.ranges.iter().enumerate() {
            let start = var_idx * bits_per;
            let end = start + bits_per;

            // Convert binary to integer
            let mut int_val: u64 = 0;
            for i in start..end.min(chromosome.genes.len()) {
                if let GeneType::Boolean(true) = chromosome.genes[i].gene_type {
                    int_val |= 1 << (i - start);
                }
            }

            // Convert to float in range
            let max_int = (1u64 << bits_per) - 1;
            let float_val = min + (int_val as f64 / max_int as f64) * (max - min);

            values.insert(
                alloc::format!("x{}", var_idx),
                PhenotypeValue::Float(float_val),
            );
        }

        Phenotype {
            values,
            valid: true,
        }
    }

    fn encode(&self, phenotype: &Phenotype) -> Chromosome {
        let bits_per = self.bits_per_var();
        let mut genes = Vec::with_capacity(self.bits);

        for (var_idx, &(min, max)) in self.ranges.iter().enumerate() {
            let key = alloc::format!("x{}", var_idx);
            let float_val = match phenotype.values.get(&key) {
                Some(PhenotypeValue::Float(v)) => *v,
                _ => min,
            };

            // Convert float to integer
            let max_int = (1u64 << bits_per) - 1;
            let normalized = (float_val - min) / (max - min);
            let int_val = (normalized * max_int as f64) as u64;

            // Convert to bits
            for bit in 0..bits_per {
                genes.push(Gene {
                    id: GeneId((var_idx * bits_per + bit) as u64),
                    gene_type: GeneType::Boolean((int_val >> bit) & 1 == 1),
                    name: alloc::format!("bit_{}_{}", var_idx, bit),
                    min: None,
                    max: None,
                    mutable: true,
                });
            }
        }

        Chromosome {
            id: ChromosomeId(0),
            genes,
            length: self.bits,
            strategy_params: BTreeMap::new(),
        }
    }

    fn name(&self) -> &str {
        "Binary"
    }

    fn gene_count(&self) -> usize {
        self.bits
    }
}

// ============================================================================
// GRAY CODE ENCODING
// ============================================================================

/// Gray code encoding (reduces Hamming cliffs)
pub struct GrayCodeEncoding {
    /// Underlying binary encoding
    binary: BinaryEncoding,
}

impl GrayCodeEncoding {
    pub fn new(bits_per_var: usize, ranges: Vec<(f64, f64)>) -> Self {
        Self {
            binary: BinaryEncoding::new(bits_per_var, ranges),
        }
    }

    fn binary_to_gray(binary: u64) -> u64 {
        binary ^ (binary >> 1)
    }

    fn gray_to_binary(gray: u64) -> u64 {
        let mut binary = gray;
        let mut mask = gray >> 1;
        while mask != 0 {
            binary ^= mask;
            mask >>= 1;
        }
        binary
    }
}

impl Encoding for GrayCodeEncoding {
    fn decode(&self, chromosome: &Chromosome) -> Phenotype {
        // Convert Gray to binary first
        let bits_per = self.binary.bits_per_var();
        let mut values = BTreeMap::new();

        for (var_idx, &(min, max)) in self.binary.ranges.iter().enumerate() {
            let start = var_idx * bits_per;
            let end = start + bits_per;

            // Get gray code value
            let mut gray_val: u64 = 0;
            for i in start..end.min(chromosome.genes.len()) {
                if let GeneType::Boolean(true) = chromosome.genes[i].gene_type {
                    gray_val |= 1 << (i - start);
                }
            }

            // Convert gray to binary
            let binary_val = Self::gray_to_binary(gray_val);

            // Convert to float
            let max_int = (1u64 << bits_per) - 1;
            let float_val = min + (binary_val as f64 / max_int as f64) * (max - min);

            values.insert(
                alloc::format!("x{}", var_idx),
                PhenotypeValue::Float(float_val),
            );
        }

        Phenotype {
            values,
            valid: true,
        }
    }

    fn encode(&self, phenotype: &Phenotype) -> Chromosome {
        let bits_per = self.binary.bits_per_var();
        let mut genes = Vec::with_capacity(self.binary.bits);

        for (var_idx, &(min, max)) in self.binary.ranges.iter().enumerate() {
            let key = alloc::format!("x{}", var_idx);
            let float_val = match phenotype.values.get(&key) {
                Some(PhenotypeValue::Float(v)) => *v,
                _ => min,
            };

            // Convert to binary then gray
            let max_int = (1u64 << bits_per) - 1;
            let normalized = (float_val - min) / (max - min);
            let binary_val = (normalized * max_int as f64) as u64;
            let gray_val = Self::binary_to_gray(binary_val);

            for bit in 0..bits_per {
                genes.push(Gene {
                    id: GeneId((var_idx * bits_per + bit) as u64),
                    gene_type: GeneType::Boolean((gray_val >> bit) & 1 == 1),
                    name: alloc::format!("gray_{}_{}", var_idx, bit),
                    min: None,
                    max: None,
                    mutable: true,
                });
            }
        }

        Chromosome {
            id: ChromosomeId(0),
            genes,
            length: self.binary.bits,
            strategy_params: BTreeMap::new(),
        }
    }

    fn name(&self) -> &str {
        "GrayCode"
    }

    fn gene_count(&self) -> usize {
        self.binary.bits
    }
}

// ============================================================================
// REAL-VALUED ENCODING
// ============================================================================

/// Direct real-valued encoding
pub struct RealEncoding {
    /// Variable count
    count: usize,
    /// Ranges
    ranges: Vec<(f64, f64)>,
}

impl RealEncoding {
    pub fn new(ranges: Vec<(f64, f64)>) -> Self {
        Self {
            count: ranges.len(),
            ranges,
        }
    }
}

impl Encoding for RealEncoding {
    fn decode(&self, chromosome: &Chromosome) -> Phenotype {
        let mut values = BTreeMap::new();

        for (i, gene) in chromosome.genes.iter().enumerate() {
            if let GeneType::Float(v) = gene.gene_type {
                values.insert(alloc::format!("x{}", i), PhenotypeValue::Float(v));
            }
        }

        Phenotype {
            values,
            valid: true,
        }
    }

    fn encode(&self, phenotype: &Phenotype) -> Chromosome {
        let mut genes = Vec::with_capacity(self.count);

        for (i, &(min, max)) in self.ranges.iter().enumerate() {
            let key = alloc::format!("x{}", i);
            let val = match phenotype.values.get(&key) {
                Some(PhenotypeValue::Float(v)) => *v,
                _ => (min + max) / 2.0,
            };

            genes.push(Gene {
                id: GeneId(i as u64),
                gene_type: GeneType::Float(val),
                name: key,
                min: Some(min),
                max: Some(max),
                mutable: true,
            });
        }

        Chromosome {
            id: ChromosomeId(0),
            genes,
            length: self.count,
            strategy_params: BTreeMap::new(),
        }
    }

    fn name(&self) -> &str {
        "Real"
    }

    fn gene_count(&self) -> usize {
        self.count
    }
}

// ============================================================================
// PERMUTATION ENCODING
// ============================================================================

/// Permutation encoding (for ordering problems)
pub struct PermutationEncoding {
    /// Size of permutation
    size: usize,
}

impl PermutationEncoding {
    pub fn new(size: usize) -> Self {
        Self { size }
    }

    /// Create random permutation
    pub fn random_chromosome(&self, rng: &Rng) -> Chromosome {
        let mut values: Vec<i64> = (0..self.size as i64).collect();

        // Fisher-Yates shuffle
        for i in (1..values.len()).rev() {
            let j = rng.next_usize(i + 1);
            values.swap(i, j);
        }

        let genes = values
            .iter()
            .enumerate()
            .map(|(i, &v)| Gene {
                id: GeneId(i as u64),
                gene_type: GeneType::Integer(v),
                name: alloc::format!("pos{}", i),
                min: Some(0.0),
                max: Some((self.size - 1) as f64),
                mutable: true,
            })
            .collect();

        Chromosome {
            id: ChromosomeId(0),
            genes,
            length: self.size,
            strategy_params: BTreeMap::new(),
        }
    }

    /// Check if valid permutation
    pub fn is_valid(&self, chromosome: &Chromosome) -> bool {
        let mut seen = vec![false; self.size];

        for gene in &chromosome.genes {
            if let GeneType::Integer(v) = gene.gene_type {
                let idx = v as usize;
                if idx >= self.size || seen[idx] {
                    return false;
                }
                seen[idx] = true;
            } else {
                return false;
            }
        }

        seen.iter().all(|&s| s)
    }

    /// Repair invalid permutation
    pub fn repair(&self, chromosome: &mut Chromosome) {
        let mut seen = vec![false; self.size];
        let mut missing: Vec<i64> = Vec::new();
        let mut duplicates: Vec<usize> = Vec::new();

        // Find duplicates and missing
        for (i, gene) in chromosome.genes.iter().enumerate() {
            if let GeneType::Integer(v) = gene.gene_type {
                let idx = v as usize;
                if idx >= self.size || seen[idx] {
                    duplicates.push(i);
                } else {
                    seen[idx] = true;
                }
            }
        }

        for i in 0..self.size {
            if !seen[i] {
                missing.push(i as i64);
            }
        }

        // Replace duplicates with missing values
        for (dup_idx, miss_val) in duplicates.iter().zip(missing.iter()) {
            if let GeneType::Integer(ref mut v) = chromosome.genes[*dup_idx].gene_type {
                *v = *miss_val;
            }
        }
    }
}

impl Encoding for PermutationEncoding {
    fn decode(&self, chromosome: &Chromosome) -> Phenotype {
        let order: Vec<f64> = chromosome
            .genes
            .iter()
            .filter_map(|g| {
                if let GeneType::Integer(v) = g.gene_type {
                    Some(v as f64)
                } else {
                    None
                }
            })
            .collect();

        let mut values = BTreeMap::new();
        values.insert(String::from("order"), PhenotypeValue::Vector(order));

        Phenotype {
            values,
            valid: self.is_valid(chromosome),
        }
    }

    fn encode(&self, phenotype: &Phenotype) -> Chromosome {
        let order = match phenotype.values.get("order") {
            Some(PhenotypeValue::Vector(v)) => v.clone(),
            _ => (0..self.size as f64).map(|x| x).collect(),
        };

        let genes = order
            .iter()
            .enumerate()
            .map(|(i, &v)| Gene {
                id: GeneId(i as u64),
                gene_type: GeneType::Integer(v as i64),
                name: alloc::format!("pos{}", i),
                min: Some(0.0),
                max: Some((self.size - 1) as f64),
                mutable: true,
            })
            .collect();

        Chromosome {
            id: ChromosomeId(0),
            genes,
            length: self.size,
            strategy_params: BTreeMap::new(),
        }
    }

    fn name(&self) -> &str {
        "Permutation"
    }

    fn gene_count(&self) -> usize {
        self.size
    }
}

// ============================================================================
// TREE ENCODING (GP)
// ============================================================================

/// Tree-based encoding for genetic programming
pub struct TreeEncoding {
    /// Maximum depth
    max_depth: usize,
    /// Function set
    functions: Vec<TreeNodeType>,
    /// Terminal set
    terminals: Vec<TreeNodeType>,
    /// Number of variables
    num_variables: usize,
}

impl TreeEncoding {
    pub fn new(max_depth: usize, num_variables: usize) -> Self {
        Self {
            max_depth,
            functions: vec![
                TreeNodeType::Add,
                TreeNodeType::Sub,
                TreeNodeType::Mul,
                TreeNodeType::Div,
            ],
            terminals: {
                let mut t = vec![TreeNodeType::Constant];
                for i in 0..num_variables {
                    t.push(TreeNodeType::Variable(i));
                }
                t
            },
            num_variables,
        }
    }

    /// Grow random tree
    pub fn grow_tree(&self, rng: &Rng, depth: usize, max_depth: usize) -> TreeNode {
        if depth >= max_depth {
            // Must be terminal
            self.random_terminal(rng)
        } else if depth == 0 || rng.next_bool(0.5) {
            // Function node
            let func = self.functions[rng.next_usize(self.functions.len())];
            let arity = self.arity(func);

            let children: Vec<TreeNode> = (0..arity)
                .map(|_| self.grow_tree(rng, depth + 1, max_depth))
                .collect();

            TreeNode {
                node_type: func,
                children,
                value: None,
            }
        } else {
            // Terminal node
            self.random_terminal(rng)
        }
    }

    fn random_terminal(&self, rng: &Rng) -> TreeNode {
        let terminal = self.terminals[rng.next_usize(self.terminals.len())];
        let value = if terminal == TreeNodeType::Constant {
            Some(rng.next_range(-10.0, 10.0))
        } else {
            None
        };

        TreeNode {
            node_type: terminal,
            children: Vec::new(),
            value,
        }
    }

    fn arity(&self, node_type: TreeNodeType) -> usize {
        match node_type {
            TreeNodeType::Add
            | TreeNodeType::Sub
            | TreeNodeType::Mul
            | TreeNodeType::Div
            | TreeNodeType::Pow => 2,
            TreeNodeType::Sin
            | TreeNodeType::Cos
            | TreeNodeType::Exp
            | TreeNodeType::Log
            | TreeNodeType::Sqrt => 1,
            TreeNodeType::If => 3,
            _ => 0,
        }
    }

    /// Evaluate tree
    pub fn evaluate(&self, tree: &TreeNode, inputs: &[f64]) -> f64 {
        match tree.node_type {
            TreeNodeType::Constant => tree.value.unwrap_or(0.0),
            TreeNodeType::Variable(i) => inputs.get(i).copied().unwrap_or(0.0),
            TreeNodeType::Add => {
                let a = self.evaluate(&tree.children[0], inputs);
                let b = self.evaluate(&tree.children[1], inputs);
                a + b
            },
            TreeNodeType::Sub => {
                let a = self.evaluate(&tree.children[0], inputs);
                let b = self.evaluate(&tree.children[1], inputs);
                a - b
            },
            TreeNodeType::Mul => {
                let a = self.evaluate(&tree.children[0], inputs);
                let b = self.evaluate(&tree.children[1], inputs);
                a * b
            },
            TreeNodeType::Div => {
                let a = self.evaluate(&tree.children[0], inputs);
                let b = self.evaluate(&tree.children[1], inputs);
                if b.abs() < 1e-10 { a } else { a / b }
            },
            TreeNodeType::Sin => self.evaluate(&tree.children[0], inputs).sin(),
            TreeNodeType::Cos => self.evaluate(&tree.children[0], inputs).cos(),
            TreeNodeType::Exp => self.evaluate(&tree.children[0], inputs).exp().min(1e10),
            TreeNodeType::Log => {
                let x = self.evaluate(&tree.children[0], inputs);
                if x > 0.0 { x.ln() } else { 0.0 }
            },
            TreeNodeType::Sqrt => {
                let x = self.evaluate(&tree.children[0], inputs);
                if x >= 0.0 { x.sqrt() } else { 0.0 }
            },
            TreeNodeType::Pow => {
                let a = self.evaluate(&tree.children[0], inputs);
                let b = self.evaluate(&tree.children[1], inputs);
                a.powf(b).min(1e10)
            },
            TreeNodeType::If => {
                let cond = self.evaluate(&tree.children[0], inputs);
                if cond > 0.0 {
                    self.evaluate(&tree.children[1], inputs)
                } else {
                    self.evaluate(&tree.children[2], inputs)
                }
            },
        }
    }

    /// Count nodes in tree
    #[inline]
    pub fn node_count(&self, tree: &TreeNode) -> usize {
        1 + tree
            .children
            .iter()
            .map(|c| self.node_count(c))
            .sum::<usize>()
    }

    /// Get tree depth
    pub fn tree_depth(&self, tree: &TreeNode) -> usize {
        if tree.children.is_empty() {
            1
        } else {
            1 + tree
                .children
                .iter()
                .map(|c| self.tree_depth(c))
                .max()
                .unwrap_or(0)
        }
    }

    /// Flatten tree to linear representation
    fn flatten(&self, tree: &TreeNode) -> Vec<Gene> {
        let mut genes = Vec::new();
        self.flatten_recursive(tree, &mut genes);
        genes
    }

    fn flatten_recursive(&self, tree: &TreeNode, genes: &mut Vec<Gene>) {
        let id = genes.len() as u64;

        // Encode node type as integer
        let node_val = match tree.node_type {
            TreeNodeType::Add => 0,
            TreeNodeType::Sub => 1,
            TreeNodeType::Mul => 2,
            TreeNodeType::Div => 3,
            TreeNodeType::Sin => 4,
            TreeNodeType::Cos => 5,
            TreeNodeType::Exp => 6,
            TreeNodeType::Log => 7,
            TreeNodeType::Sqrt => 8,
            TreeNodeType::Pow => 9,
            TreeNodeType::If => 10,
            TreeNodeType::Constant => 100,
            TreeNodeType::Variable(i) => 200 + i as i64,
        };

        genes.push(Gene {
            id: GeneId(id),
            gene_type: GeneType::Integer(node_val),
            name: alloc::format!("node_{}", id),
            min: None,
            max: None,
            mutable: true,
        });

        // Add constant value if applicable
        if tree.node_type == TreeNodeType::Constant {
            genes.push(Gene {
                id: GeneId(id + 1000000),
                gene_type: GeneType::Float(tree.value.unwrap_or(0.0)),
                name: alloc::format!("const_{}", id),
                min: Some(-10.0),
                max: Some(10.0),
                mutable: true,
            });
        }

        for child in &tree.children {
            self.flatten_recursive(child, genes);
        }
    }
}

impl Encoding for TreeEncoding {
    fn decode(&self, chromosome: &Chromosome) -> Phenotype {
        // Would reconstruct tree from linear encoding
        let mut values = BTreeMap::new();
        values.insert(String::from("tree"), PhenotypeValue::Custom(Vec::new()));

        Phenotype {
            values,
            valid: true,
        }
    }

    fn encode(&self, phenotype: &Phenotype) -> Chromosome {
        if let Some(PhenotypeValue::Tree(tree)) = phenotype.values.get("tree") {
            let genes = self.flatten(tree);
            Chromosome {
                id: ChromosomeId(0),
                genes,
                length: 0,
                strategy_params: BTreeMap::new(),
            }
        } else {
            Chromosome {
                id: ChromosomeId(0),
                genes: Vec::new(),
                length: 0,
                strategy_params: BTreeMap::new(),
            }
        }
    }

    fn name(&self) -> &str {
        "Tree"
    }

    fn gene_count(&self) -> usize {
        0 // Variable
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_encoding() {
        let encoding = BinaryEncoding::new(8, vec![(0.0, 1.0), (0.0, 10.0)]);

        let mut values = BTreeMap::new();
        values.insert(String::from("x0"), PhenotypeValue::Float(0.5));
        values.insert(String::from("x1"), PhenotypeValue::Float(5.0));

        let phenotype = Phenotype {
            values,
            valid: true,
        };
        let chromosome = encoding.encode(&phenotype);
        let decoded = encoding.decode(&chromosome);

        if let Some(PhenotypeValue::Float(v0)) = decoded.values.get("x0") {
            assert!((v0 - 0.5).abs() < 0.1);
        }
    }

    #[test]
    fn test_gray_code() {
        assert_eq!(GrayCodeEncoding::binary_to_gray(0), 0);
        assert_eq!(GrayCodeEncoding::binary_to_gray(1), 1);
        assert_eq!(GrayCodeEncoding::binary_to_gray(2), 3);
        assert_eq!(GrayCodeEncoding::binary_to_gray(3), 2);

        assert_eq!(GrayCodeEncoding::gray_to_binary(0), 0);
        assert_eq!(GrayCodeEncoding::gray_to_binary(1), 1);
        assert_eq!(GrayCodeEncoding::gray_to_binary(3), 2);
        assert_eq!(GrayCodeEncoding::gray_to_binary(2), 3);
    }

    #[test]
    fn test_permutation_encoding() {
        let encoding = PermutationEncoding::new(5);
        let rng = Rng::default();

        let chromosome = encoding.random_chromosome(&rng);
        assert!(encoding.is_valid(&chromosome));
    }

    #[test]
    fn test_tree_encoding() {
        let encoding = TreeEncoding::new(3, 2);
        let rng = Rng::default();

        let tree = encoding.grow_tree(&rng, 0, 3);
        let depth = encoding.tree_depth(&tree);

        assert!(depth <= 3);
    }

    #[test]
    fn test_tree_evaluation() {
        let encoding = TreeEncoding::new(3, 2);

        // x0 + x1
        let tree = TreeNode {
            node_type: TreeNodeType::Add,
            children: vec![
                TreeNode {
                    node_type: TreeNodeType::Variable(0),
                    children: Vec::new(),
                    value: None,
                },
                TreeNode {
                    node_type: TreeNodeType::Variable(1),
                    children: Vec::new(),
                    value: None,
                },
            ],
            value: None,
        };

        let result = encoding.evaluate(&tree, &[2.0, 3.0]);
        assert!((result - 5.0).abs() < 1e-10);
    }
}
