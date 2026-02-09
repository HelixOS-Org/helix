//! # Genetic Operators
//!
//! Year 3 EVOLUTION - Advanced crossover and mutation operators

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::genome::{Chromosome, Gene, GeneType};

// ============================================================================
// RANDOM UTILITY
// ============================================================================

/// Random number generator
pub struct Rng {
    state: AtomicU64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: AtomicU64::new(seed),
        }
    }

    #[inline]
    pub fn next_u64(&self) -> u64 {
        let mut x = self.state.load(Ordering::Relaxed);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state.store(x, Ordering::Relaxed);
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    #[inline(always)]
    pub fn next_f64(&self) -> f64 {
        (self.next_u64() as f64) / (u64::MAX as f64)
    }

    #[inline(always)]
    pub fn next_usize(&self, max: usize) -> usize {
        (self.next_u64() as usize) % max
    }

    #[inline(always)]
    pub fn next_range(&self, min: f64, max: f64) -> f64 {
        min + self.next_f64() * (max - min)
    }

    #[inline(always)]
    pub fn next_bool(&self, prob: f64) -> bool {
        self.next_f64() < prob
    }

    #[inline]
    pub fn next_gaussian(&self) -> f64 {
        // Box-Muller transform
        let u1 = self.next_f64();
        let u2 = self.next_f64();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * core::f64::consts::PI * u2).cos();
        z
    }
}

impl Default for Rng {
    fn default() -> Self {
        Self::new(0xDEADBEEFCAFEBABE)
    }
}

// ============================================================================
// CROSSOVER OPERATORS
// ============================================================================

/// Crossover operator trait
pub trait CrossoverOperator: Send + Sync {
    /// Perform crossover
    fn crossover(
        &self,
        parent1: &Chromosome,
        parent2: &Chromosome,
        rng: &Rng,
    ) -> (Chromosome, Chromosome);

    /// Operator name
    fn name(&self) -> &str;
}

/// Single-point crossover
pub struct SinglePointCrossover;

impl CrossoverOperator for SinglePointCrossover {
    fn crossover(
        &self,
        parent1: &Chromosome,
        parent2: &Chromosome,
        rng: &Rng,
    ) -> (Chromosome, Chromosome) {
        let len = parent1.genes.len().min(parent2.genes.len());
        if len <= 1 {
            return (parent1.clone(), parent2.clone());
        }

        let point = 1 + rng.next_usize(len - 1);

        let mut child1_genes = parent1.genes[..point].to_vec();
        child1_genes.extend_from_slice(&parent2.genes[point..]);

        let mut child2_genes = parent2.genes[..point].to_vec();
        child2_genes.extend_from_slice(&parent1.genes[point..]);

        (
            Chromosome {
                genes: child1_genes,
                ..parent1.clone()
            },
            Chromosome {
                genes: child2_genes,
                ..parent2.clone()
            },
        )
    }

    fn name(&self) -> &str {
        "SinglePoint"
    }
}

/// Two-point crossover
pub struct TwoPointCrossover;

impl CrossoverOperator for TwoPointCrossover {
    fn crossover(
        &self,
        parent1: &Chromosome,
        parent2: &Chromosome,
        rng: &Rng,
    ) -> (Chromosome, Chromosome) {
        let len = parent1.genes.len().min(parent2.genes.len());
        if len <= 2 {
            return (parent1.clone(), parent2.clone());
        }

        let mut p1 = rng.next_usize(len);
        let mut p2 = rng.next_usize(len);

        if p1 > p2 {
            core::mem::swap(&mut p1, &mut p2);
        }

        let mut child1_genes = Vec::with_capacity(len);
        let mut child2_genes = Vec::with_capacity(len);

        for i in 0..len {
            if i < p1 || i >= p2 {
                child1_genes.push(parent1.genes[i].clone());
                child2_genes.push(parent2.genes[i].clone());
            } else {
                child1_genes.push(parent2.genes[i].clone());
                child2_genes.push(parent1.genes[i].clone());
            }
        }

        (
            Chromosome {
                genes: child1_genes,
                ..parent1.clone()
            },
            Chromosome {
                genes: child2_genes,
                ..parent2.clone()
            },
        )
    }

    fn name(&self) -> &str {
        "TwoPoint"
    }
}

/// Uniform crossover
pub struct UniformCrossover {
    /// Swap probability
    swap_prob: f64,
}

impl UniformCrossover {
    pub fn new(swap_prob: f64) -> Self {
        Self {
            swap_prob: swap_prob.clamp(0.0, 1.0),
        }
    }
}

impl Default for UniformCrossover {
    fn default() -> Self {
        Self::new(0.5)
    }
}

impl CrossoverOperator for UniformCrossover {
    fn crossover(
        &self,
        parent1: &Chromosome,
        parent2: &Chromosome,
        rng: &Rng,
    ) -> (Chromosome, Chromosome) {
        let len = parent1.genes.len().min(parent2.genes.len());

        let mut child1_genes = Vec::with_capacity(len);
        let mut child2_genes = Vec::with_capacity(len);

        for i in 0..len {
            if rng.next_bool(self.swap_prob) {
                child1_genes.push(parent2.genes[i].clone());
                child2_genes.push(parent1.genes[i].clone());
            } else {
                child1_genes.push(parent1.genes[i].clone());
                child2_genes.push(parent2.genes[i].clone());
            }
        }

        (
            Chromosome {
                genes: child1_genes,
                ..parent1.clone()
            },
            Chromosome {
                genes: child2_genes,
                ..parent2.clone()
            },
        )
    }

    fn name(&self) -> &str {
        "Uniform"
    }
}

/// Simulated Binary Crossover (SBX) for real-valued
pub struct SBXCrossover {
    /// Distribution index (larger = offspring closer to parents)
    eta: f64,
}

impl SBXCrossover {
    pub fn new(eta: f64) -> Self {
        Self { eta: eta.max(1.0) }
    }
}

impl Default for SBXCrossover {
    fn default() -> Self {
        Self::new(20.0)
    }
}

impl CrossoverOperator for SBXCrossover {
    fn crossover(
        &self,
        parent1: &Chromosome,
        parent2: &Chromosome,
        rng: &Rng,
    ) -> (Chromosome, Chromosome) {
        let len = parent1.genes.len().min(parent2.genes.len());

        let mut child1_genes = Vec::with_capacity(len);
        let mut child2_genes = Vec::with_capacity(len);

        for i in 0..len {
            if let (GeneType::Float(v1), GeneType::Float(v2)) =
                (&parent1.genes[i].gene_type, &parent2.genes[i].gene_type)
            {
                let u = rng.next_f64();

                let beta = if u <= 0.5 {
                    (2.0 * u).powf(1.0 / (self.eta + 1.0))
                } else {
                    (1.0 / (2.0 * (1.0 - u))).powf(1.0 / (self.eta + 1.0))
                };

                let c1 = 0.5 * ((1.0 + beta) * v1 + (1.0 - beta) * v2);
                let c2 = 0.5 * ((1.0 - beta) * v1 + (1.0 + beta) * v2);

                child1_genes.push(Gene {
                    gene_type: GeneType::Float(c1),
                    ..parent1.genes[i].clone()
                });
                child2_genes.push(Gene {
                    gene_type: GeneType::Float(c2),
                    ..parent2.genes[i].clone()
                });
            } else {
                // Non-float genes: uniform crossover
                if rng.next_bool(0.5) {
                    child1_genes.push(parent2.genes[i].clone());
                    child2_genes.push(parent1.genes[i].clone());
                } else {
                    child1_genes.push(parent1.genes[i].clone());
                    child2_genes.push(parent2.genes[i].clone());
                }
            }
        }

        (
            Chromosome {
                genes: child1_genes,
                ..parent1.clone()
            },
            Chromosome {
                genes: child2_genes,
                ..parent2.clone()
            },
        )
    }

    fn name(&self) -> &str {
        "SBX"
    }
}

/// Order Crossover (OX) for permutations
pub struct OrderCrossover;

impl CrossoverOperator for OrderCrossover {
    fn crossover(
        &self,
        parent1: &Chromosome,
        parent2: &Chromosome,
        rng: &Rng,
    ) -> (Chromosome, Chromosome) {
        let len = parent1.genes.len();
        if len <= 2 {
            return (parent1.clone(), parent2.clone());
        }

        let mut p1 = rng.next_usize(len);
        let mut p2 = rng.next_usize(len);

        if p1 > p2 {
            core::mem::swap(&mut p1, &mut p2);
        }

        // Get values from parents (assuming integer genes)
        let get_values = |c: &Chromosome| -> Vec<i64> {
            c.genes
                .iter()
                .filter_map(|g| {
                    if let GeneType::Integer(v) = g.gene_type {
                        Some(v)
                    } else {
                        None
                    }
                })
                .collect()
        };

        let v1 = get_values(parent1);
        let v2 = get_values(parent2);

        if v1.len() != len || v2.len() != len {
            return (parent1.clone(), parent2.clone());
        }

        // Child 1: segment from parent1, rest from parent2 in order
        let mut child1_vals = vec![-1i64; len];
        for i in p1..=p2 {
            child1_vals[i] = v1[i];
        }

        let segment: Vec<i64> = child1_vals[p1..=p2].to_vec();
        let mut pos = (p2 + 1) % len;
        for &v in v2.iter().cycle().skip(p2 + 1).take(len) {
            if !segment.contains(&v) {
                child1_vals[pos] = v;
                pos = (pos + 1) % len;
                if pos == p1 {
                    break;
                }
            }
        }

        // Child 2: segment from parent2, rest from parent1 in order
        let mut child2_vals = vec![-1i64; len];
        for i in p1..=p2 {
            child2_vals[i] = v2[i];
        }

        let segment: Vec<i64> = child2_vals[p1..=p2].to_vec();
        let mut pos = (p2 + 1) % len;
        for &v in v1.iter().cycle().skip(p2 + 1).take(len) {
            if !segment.contains(&v) {
                child2_vals[pos] = v;
                pos = (pos + 1) % len;
                if pos == p1 {
                    break;
                }
            }
        }

        // Reconstruct chromosomes
        let make_chromosome = |vals: Vec<i64>, template: &Chromosome| -> Chromosome {
            let genes: Vec<Gene> = vals
                .iter()
                .zip(template.genes.iter())
                .map(|(&v, g)| Gene {
                    gene_type: GeneType::Integer(v),
                    ..g.clone()
                })
                .collect();
            Chromosome {
                genes,
                ..template.clone()
            }
        };

        (
            make_chromosome(child1_vals, parent1),
            make_chromosome(child2_vals, parent2),
        )
    }

    fn name(&self) -> &str {
        "Order"
    }
}

/// Blend crossover (BLX-α) for real-valued
pub struct BlendCrossover {
    /// Alpha parameter (exploration range)
    alpha: f64,
}

impl BlendCrossover {
    pub fn new(alpha: f64) -> Self {
        Self {
            alpha: alpha.max(0.0),
        }
    }
}

impl Default for BlendCrossover {
    fn default() -> Self {
        Self::new(0.5)
    }
}

impl CrossoverOperator for BlendCrossover {
    fn crossover(
        &self,
        parent1: &Chromosome,
        parent2: &Chromosome,
        rng: &Rng,
    ) -> (Chromosome, Chromosome) {
        let len = parent1.genes.len().min(parent2.genes.len());

        let mut child1_genes = Vec::with_capacity(len);
        let mut child2_genes = Vec::with_capacity(len);

        for i in 0..len {
            if let (GeneType::Float(v1), GeneType::Float(v2)) =
                (&parent1.genes[i].gene_type, &parent2.genes[i].gene_type)
            {
                let min = v1.min(*v2);
                let max = v1.max(*v2);
                let range = max - min;
                let extended_min = min - self.alpha * range;
                let extended_max = max + self.alpha * range;

                let c1 = rng.next_range(extended_min, extended_max);
                let c2 = rng.next_range(extended_min, extended_max);

                child1_genes.push(Gene {
                    gene_type: GeneType::Float(c1),
                    ..parent1.genes[i].clone()
                });
                child2_genes.push(Gene {
                    gene_type: GeneType::Float(c2),
                    ..parent2.genes[i].clone()
                });
            } else {
                child1_genes.push(parent1.genes[i].clone());
                child2_genes.push(parent2.genes[i].clone());
            }
        }

        (
            Chromosome {
                genes: child1_genes,
                ..parent1.clone()
            },
            Chromosome {
                genes: child2_genes,
                ..parent2.clone()
            },
        )
    }

    fn name(&self) -> &str {
        "Blend"
    }
}

// ============================================================================
// MUTATION OPERATORS
// ============================================================================

/// Mutation operator trait
pub trait MutationOperator: Send + Sync {
    /// Mutate chromosome
    fn mutate(&self, chromosome: &mut Chromosome, rng: &Rng);

    /// Operator name
    fn name(&self) -> &str;
}

/// Bit flip mutation (for binary)
pub struct BitFlipMutation {
    /// Probability per gene
    prob: f64,
}

impl BitFlipMutation {
    pub fn new(prob: f64) -> Self {
        Self {
            prob: prob.clamp(0.0, 1.0),
        }
    }
}

impl MutationOperator for BitFlipMutation {
    fn mutate(&self, chromosome: &mut Chromosome, rng: &Rng) {
        for gene in &mut chromosome.genes {
            if rng.next_bool(self.prob) {
                if let GeneType::Boolean(ref mut v) = gene.gene_type {
                    *v = !*v;
                }
            }
        }
    }

    fn name(&self) -> &str {
        "BitFlip"
    }
}

/// Gaussian mutation (for real-valued)
pub struct GaussianMutation {
    /// Probability per gene
    prob: f64,
    /// Standard deviation
    sigma: f64,
}

impl GaussianMutation {
    pub fn new(prob: f64, sigma: f64) -> Self {
        Self {
            prob: prob.clamp(0.0, 1.0),
            sigma: sigma.abs(),
        }
    }
}

impl MutationOperator for GaussianMutation {
    fn mutate(&self, chromosome: &mut Chromosome, rng: &Rng) {
        for gene in &mut chromosome.genes {
            if rng.next_bool(self.prob) {
                if let GeneType::Float(ref mut v) = gene.gene_type {
                    *v += rng.next_gaussian() * self.sigma;

                    // Clamp to bounds if available
                    if let (Some(min), Some(max)) = (gene.min, gene.max) {
                        *v = v.clamp(min, max);
                    }
                }
            }
        }
    }

    fn name(&self) -> &str {
        "Gaussian"
    }
}

/// Polynomial mutation (for real-valued)
pub struct PolynomialMutation {
    /// Probability per gene
    prob: f64,
    /// Distribution index
    eta: f64,
}

impl PolynomialMutation {
    pub fn new(prob: f64, eta: f64) -> Self {
        Self {
            prob: prob.clamp(0.0, 1.0),
            eta: eta.max(1.0),
        }
    }
}

impl MutationOperator for PolynomialMutation {
    fn mutate(&self, chromosome: &mut Chromosome, rng: &Rng) {
        for gene in &mut chromosome.genes {
            if rng.next_bool(self.prob) {
                if let GeneType::Float(ref mut v) = gene.gene_type {
                    let min = gene.min.unwrap_or(-1e10);
                    let max = gene.max.unwrap_or(1e10);

                    let u = rng.next_f64();
                    let delta = if u < 0.5 {
                        (2.0 * u).powf(1.0 / (self.eta + 1.0)) - 1.0
                    } else {
                        1.0 - (2.0 * (1.0 - u)).powf(1.0 / (self.eta + 1.0))
                    };

                    *v = *v + delta * (max - min);
                    *v = v.clamp(min, max);
                }
            }
        }
    }

    fn name(&self) -> &str {
        "Polynomial"
    }
}

/// Swap mutation (for permutations)
pub struct SwapMutation {
    /// Probability
    prob: f64,
}

impl SwapMutation {
    pub fn new(prob: f64) -> Self {
        Self {
            prob: prob.clamp(0.0, 1.0),
        }
    }
}

impl MutationOperator for SwapMutation {
    fn mutate(&self, chromosome: &mut Chromosome, rng: &Rng) {
        if rng.next_bool(self.prob) && chromosome.genes.len() >= 2 {
            let i = rng.next_usize(chromosome.genes.len());
            let j = rng.next_usize(chromosome.genes.len());
            chromosome.genes.swap(i, j);
        }
    }

    fn name(&self) -> &str {
        "Swap"
    }
}

/// Inversion mutation (for permutations)
pub struct InversionMutation {
    /// Probability
    prob: f64,
}

impl InversionMutation {
    pub fn new(prob: f64) -> Self {
        Self {
            prob: prob.clamp(0.0, 1.0),
        }
    }
}

impl MutationOperator for InversionMutation {
    fn mutate(&self, chromosome: &mut Chromosome, rng: &Rng) {
        if rng.next_bool(self.prob) && chromosome.genes.len() >= 2 {
            let mut i = rng.next_usize(chromosome.genes.len());
            let mut j = rng.next_usize(chromosome.genes.len());

            if i > j {
                core::mem::swap(&mut i, &mut j);
            }

            // Reverse segment
            chromosome.genes[i..=j].reverse();
        }
    }

    fn name(&self) -> &str {
        "Inversion"
    }
}

/// Scramble mutation (for permutations)
pub struct ScrambleMutation {
    /// Probability
    prob: f64,
}

impl ScrambleMutation {
    pub fn new(prob: f64) -> Self {
        Self {
            prob: prob.clamp(0.0, 1.0),
        }
    }
}

impl MutationOperator for ScrambleMutation {
    fn mutate(&self, chromosome: &mut Chromosome, rng: &Rng) {
        if rng.next_bool(self.prob) && chromosome.genes.len() >= 2 {
            let mut i = rng.next_usize(chromosome.genes.len());
            let mut j = rng.next_usize(chromosome.genes.len());

            if i > j {
                core::mem::swap(&mut i, &mut j);
            }

            // Scramble segment
            for k in i..j {
                let l = k + rng.next_usize(j - k + 1);
                chromosome.genes.swap(k, l);
            }
        }
    }

    fn name(&self) -> &str {
        "Scramble"
    }
}

/// Creep mutation (small changes for integers)
pub struct CreepMutation {
    /// Probability per gene
    prob: f64,
    /// Maximum creep
    max_creep: i64,
}

impl CreepMutation {
    pub fn new(prob: f64, max_creep: i64) -> Self {
        Self {
            prob: prob.clamp(0.0, 1.0),
            max_creep: max_creep.abs(),
        }
    }
}

impl MutationOperator for CreepMutation {
    fn mutate(&self, chromosome: &mut Chromosome, rng: &Rng) {
        for gene in &mut chromosome.genes {
            if rng.next_bool(self.prob) {
                if let GeneType::Integer(ref mut v) = gene.gene_type {
                    let delta =
                        (rng.next_usize(2 * self.max_creep as usize + 1) as i64) - self.max_creep;
                    *v += delta;

                    // Clamp to bounds
                    if let (Some(min), Some(max)) = (gene.min, gene.max) {
                        *v = (*v as f64).clamp(min, max) as i64;
                    }
                }
            }
        }
    }

    fn name(&self) -> &str {
        "Creep"
    }
}

// ============================================================================
// ADAPTIVE OPERATORS
// ============================================================================

/// Self-adaptive mutation rate
pub struct SelfAdaptiveMutation {
    /// Base operator
    base_operator: Box<dyn MutationOperator>,
    /// Learning rate
    tau: f64,
}

impl SelfAdaptiveMutation {
    pub fn new(base: Box<dyn MutationOperator>, tau: f64) -> Self {
        Self {
            base_operator: base,
            tau: tau.abs(),
        }
    }
}

impl MutationOperator for SelfAdaptiveMutation {
    fn mutate(&self, chromosome: &mut Chromosome, rng: &Rng) {
        // Mutate strategy parameters first
        if let Some(sigma) = chromosome.strategy_params.get_mut("sigma") {
            let new_sigma = *sigma * (self.tau * rng.next_gaussian()).exp();
            *sigma = new_sigma.max(1e-10); // Minimum sigma
        }

        // Apply base mutation with adapted rate
        self.base_operator.mutate(chromosome, rng);
    }

    fn name(&self) -> &str {
        "SelfAdaptive"
    }
}

// ============================================================================
// OPERATOR REGISTRY
// ============================================================================

/// Operator registry
pub struct OperatorRegistry {
    crossovers: BTreeMap<String, Box<dyn CrossoverOperator>>,
    mutations: BTreeMap<String, Box<dyn MutationOperator>>,
}

impl OperatorRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            crossovers: BTreeMap::new(),
            mutations: BTreeMap::new(),
        };

        // Register defaults
        registry.register_crossover(Box::new(SinglePointCrossover));
        registry.register_crossover(Box::new(TwoPointCrossover));
        registry.register_crossover(Box::new(UniformCrossover::default()));

        registry.register_mutation(Box::new(BitFlipMutation::new(0.01)));
        registry.register_mutation(Box::new(GaussianMutation::new(0.1, 0.1)));
        registry.register_mutation(Box::new(SwapMutation::new(0.1)));

        registry
    }

    #[inline(always)]
    pub fn register_crossover(&mut self, op: Box<dyn CrossoverOperator>) {
        self.crossovers.insert(op.name().to_string(), op);
    }

    #[inline(always)]
    pub fn register_mutation(&mut self, op: Box<dyn MutationOperator>) {
        self.mutations.insert(op.name().to_string(), op);
    }

    #[inline(always)]
    pub fn get_crossover(&self, name: &str) -> Option<&dyn CrossoverOperator> {
        self.crossovers.get(name).map(|b| b.as_ref())
    }

    #[inline(always)]
    pub fn get_mutation(&self, name: &str) -> Option<&dyn MutationOperator> {
        self.mutations.get(name).map(|b| b.as_ref())
    }
}

impl Default for OperatorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_chromosome() -> Chromosome {
        Chromosome {
            id: super::super::ChromosomeId(1),
            genes: vec![
                Gene {
                    id: super::super::GeneId(1),
                    gene_type: GeneType::Float(1.0),
                    name: String::from("g1"),
                    min: Some(0.0),
                    max: Some(10.0),
                    mutable: true,
                },
                Gene {
                    id: super::super::GeneId(2),
                    gene_type: GeneType::Float(2.0),
                    name: String::from("g2"),
                    min: Some(0.0),
                    max: Some(10.0),
                    mutable: true,
                },
                Gene {
                    id: super::super::GeneId(3),
                    gene_type: GeneType::Float(3.0),
                    name: String::from("g3"),
                    min: Some(0.0),
                    max: Some(10.0),
                    mutable: true,
                },
            ],
            length: 3,
            strategy_params: BTreeMap::new(),
        }
    }

    #[test]
    fn test_single_point_crossover() {
        let rng = Rng::default();
        let crossover = SinglePointCrossover;

        let p1 = create_chromosome();
        let mut p2 = create_chromosome();
        for gene in &mut p2.genes {
            if let GeneType::Float(ref mut v) = gene.gene_type {
                *v += 10.0;
            }
        }

        let (c1, c2) = crossover.crossover(&p1, &p2, &rng);
        assert_eq!(c1.genes.len(), 3);
        assert_eq!(c2.genes.len(), 3);
    }

    #[test]
    fn test_gaussian_mutation() {
        let rng = Rng::default();
        let mutation = GaussianMutation::new(1.0, 1.0);

        let mut c = create_chromosome();
        let original: Vec<f64> = c
            .genes
            .iter()
            .filter_map(|g| {
                if let GeneType::Float(v) = g.gene_type {
                    Some(v)
                } else {
                    None
                }
            })
            .collect();

        mutation.mutate(&mut c, &rng);

        let mutated: Vec<f64> = c
            .genes
            .iter()
            .filter_map(|g| {
                if let GeneType::Float(v) = g.gene_type {
                    Some(v)
                } else {
                    None
                }
            })
            .collect();

        // At least one should be different (very high probability)
        assert_ne!(original, mutated);
    }

    #[test]
    fn test_sbx_crossover() {
        let rng = Rng::default();
        let crossover = SBXCrossover::default();

        let p1 = create_chromosome();
        let mut p2 = create_chromosome();
        for gene in &mut p2.genes {
            if let GeneType::Float(ref mut v) = gene.gene_type {
                *v += 5.0;
            }
        }

        let (c1, c2) = crossover.crossover(&p1, &p2, &rng);
        assert_eq!(c1.genes.len(), 3);
        assert_eq!(c2.genes.len(), 3);
    }
}
