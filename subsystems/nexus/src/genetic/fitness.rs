//! # Fitness Functions
//!
//! Year 3 EVOLUTION - Multi-objective fitness evaluation for code evolution
//! Evaluates code quality across multiple dimensions.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use super::Fitness;
use super::genome::CodeGenome;

// ============================================================================
// FITNESS OBJECTIVES
// ============================================================================

/// Fitness objective type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveType {
    /// Correctness (test pass rate)
    Correctness,
    /// Performance (speed)
    Performance,
    /// Memory efficiency
    MemoryEfficiency,
    /// Code size (smaller is better)
    CodeSize,
    /// Energy efficiency
    EnergyEfficiency,
    /// Safety (no unsafe operations)
    Safety,
    /// Maintainability
    Maintainability,
    /// Robustness (error handling)
    Robustness,
    /// Parallelism potential
    Parallelism,
    /// Cache efficiency
    CacheEfficiency,
    /// Branch prediction
    BranchPrediction,
    /// Vectorization potential
    Vectorization,
}

/// Objective configuration
#[derive(Debug, Clone)]
pub struct Objective {
    /// Objective type
    pub obj_type: ObjectiveType,
    /// Weight (importance)
    pub weight: f64,
    /// Minimize (true) or maximize (false)
    pub minimize: bool,
    /// Target value (optional)
    pub target: Option<f64>,
    /// Threshold (minimum acceptable)
    pub threshold: Option<f64>,
}

impl Objective {
    pub fn new(obj_type: ObjectiveType, weight: f64, minimize: bool) -> Self {
        Self {
            obj_type,
            weight,
            minimize,
            target: None,
            threshold: None,
        }
    }

    pub fn with_target(mut self, target: f64) -> Self {
        self.target = Some(target);
        self
    }

    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = Some(threshold);
        self
    }
}

// ============================================================================
// FITNESS EVALUATOR
// ============================================================================

/// Multi-objective fitness evaluator
pub struct FitnessEvaluator {
    /// Objectives
    objectives: Vec<Objective>,
    /// Test cases
    test_cases: Vec<TestCase>,
    /// Performance benchmarks
    benchmarks: Vec<Benchmark>,
    /// Cache for evaluated fitness
    cache: BTreeMap<u64, Fitness>,
    /// Statistics
    stats: EvaluatorStats,
}

/// Test case for correctness evaluation
#[derive(Debug, Clone)]
pub struct TestCase {
    /// Test ID
    pub id: u64,
    /// Input values
    pub inputs: Vec<Value>,
    /// Expected output
    pub expected: Value,
    /// Weight
    pub weight: f64,
    /// Tags
    pub tags: Vec<String>,
}

/// Benchmark for performance evaluation
#[derive(Debug, Clone)]
pub struct Benchmark {
    /// Benchmark ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Input size
    pub input_size: usize,
    /// Target cycles
    pub target_cycles: u64,
    /// Memory limit
    pub memory_limit: usize,
}

/// Value type for test cases
#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
}

/// Evaluator statistics
#[derive(Debug, Clone, Default)]
pub struct EvaluatorStats {
    /// Total evaluations
    pub evaluations: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Average evaluation time
    pub avg_eval_time_us: u64,
}

impl FitnessEvaluator {
    /// Create new evaluator
    pub fn new() -> Self {
        Self {
            objectives: Vec::new(),
            test_cases: Vec::new(),
            benchmarks: Vec::new(),
            cache: BTreeMap::new(),
            stats: EvaluatorStats::default(),
        }
    }

    /// Add objective
    pub fn add_objective(&mut self, objective: Objective) {
        self.objectives.push(objective);
    }

    /// Add test case
    pub fn add_test_case(&mut self, test_case: TestCase) {
        self.test_cases.push(test_case);
    }

    /// Add benchmark
    pub fn add_benchmark(&mut self, benchmark: Benchmark) {
        self.benchmarks.push(benchmark);
    }

    /// Evaluate genome
    pub fn evaluate(&mut self, genome: &CodeGenome) -> Fitness {
        // Check cache
        if let Some(fitness) = self.cache.get(&genome.id) {
            self.stats.cache_hits += 1;
            return fitness.clone();
        }

        self.stats.evaluations += 1;

        let mut objective_values = Vec::new();

        for objective in &self.objectives {
            let raw_value = self.evaluate_objective(genome, objective);

            // Normalize and apply direction
            let normalized = if objective.minimize {
                1.0 / (1.0 + raw_value)
            } else {
                raw_value
            };

            // Apply weight
            let weighted = normalized * objective.weight;

            objective_values.push(weighted);
        }

        let fitness = Fitness::new(objective_values);

        // Cache result
        self.cache.insert(genome.id, fitness.clone());

        fitness
    }

    fn evaluate_objective(&self, genome: &CodeGenome, objective: &Objective) -> f64 {
        match objective.obj_type {
            ObjectiveType::Correctness => self.evaluate_correctness(genome),
            ObjectiveType::Performance => self.evaluate_performance(genome),
            ObjectiveType::MemoryEfficiency => self.evaluate_memory_efficiency(genome),
            ObjectiveType::CodeSize => self.evaluate_code_size(genome),
            ObjectiveType::EnergyEfficiency => self.evaluate_energy_efficiency(genome),
            ObjectiveType::Safety => self.evaluate_safety(genome),
            ObjectiveType::Maintainability => self.evaluate_maintainability(genome),
            ObjectiveType::Robustness => self.evaluate_robustness(genome),
            ObjectiveType::Parallelism => self.evaluate_parallelism(genome),
            ObjectiveType::CacheEfficiency => self.evaluate_cache_efficiency(genome),
            ObjectiveType::BranchPrediction => self.evaluate_branch_prediction(genome),
            ObjectiveType::Vectorization => self.evaluate_vectorization(genome),
        }
    }

    fn evaluate_correctness(&self, genome: &CodeGenome) -> f64 {
        if self.test_cases.is_empty() {
            return 1.0;
        }

        let mut passed_weight = 0.0;
        let mut total_weight = 0.0;

        for test in &self.test_cases {
            let result = self.run_test(genome, test);
            total_weight += test.weight;
            if result {
                passed_weight += test.weight;
            }
        }

        if total_weight > 0.0 {
            passed_weight / total_weight
        } else {
            0.0
        }
    }

    fn run_test(&self, genome: &CodeGenome, _test: &TestCase) -> bool {
        // Simplified: check if genome has enough genes to potentially compute result
        // In real implementation, would interpret/execute genome

        // For now, use a heuristic based on genome structure
        let has_arithmetic = genome
            .genes
            .iter()
            .any(|g| matches!(g.gene_type, super::genome::GeneType::Arithmetic));

        let has_output = genome
            .genes
            .iter()
            .any(|g| matches!(g.gene_type, super::genome::GeneType::Memory));

        has_arithmetic && has_output
    }

    fn evaluate_performance(&self, genome: &CodeGenome) -> f64 {
        // Estimate cycles based on genome structure
        let mut estimated_cycles = 0u64;

        for gene in genome.active_genes() {
            estimated_cycles += match gene.gene_type {
                super::genome::GeneType::Arithmetic => gene.codons.len() as u64,
                super::genome::GeneType::Memory => gene.codons.len() as u64 * 4, // Memory is slower
                super::genome::GeneType::Control => gene.codons.len() as u64 * 2,
                super::genome::GeneType::Call => gene.codons.len() as u64 * 10,
                _ => gene.codons.len() as u64,
            };
        }

        // Lower is better, convert to 0-1 scale
        if estimated_cycles == 0 {
            1.0
        } else {
            1000.0 / (1000.0 + estimated_cycles as f64)
        }
    }

    fn evaluate_memory_efficiency(&self, genome: &CodeGenome) -> f64 {
        // Count memory operations
        let memory_ops = genome
            .genes
            .iter()
            .filter(|g| matches!(g.gene_type, super::genome::GeneType::Memory))
            .count();

        let total_ops = genome.active_size();

        if total_ops == 0 {
            return 1.0;
        }

        // Fewer memory ops relative to total is better
        1.0 - (memory_ops as f64 / total_ops as f64)
    }

    fn evaluate_code_size(&self, genome: &CodeGenome) -> f64 {
        // Smaller is better
        let size = genome.complexity;
        100.0 / (100.0 + size)
    }

    fn evaluate_energy_efficiency(&self, genome: &CodeGenome) -> f64 {
        // Estimate based on expensive operations
        let expensive_ops = genome
            .genes
            .iter()
            .filter(|g| {
                matches!(
                    g.gene_type,
                    super::genome::GeneType::Memory | super::genome::GeneType::Call
                )
            })
            .count();

        let total = genome.active_size().max(1);
        1.0 - (expensive_ops as f64 * 2.0 / total as f64).min(1.0)
    }

    fn evaluate_safety(&self, genome: &CodeGenome) -> f64 {
        // Check for potentially unsafe patterns
        let mut unsafe_count = 0;

        for gene in genome.active_genes() {
            for codon in &gene.codons {
                if let super::genome::Codon::Op(op) = codon {
                    // Hypothetical unsafe operations
                    if *op >= 200 {
                        unsafe_count += 1;
                    }
                }
            }
        }

        let total = genome
            .genes
            .iter()
            .map(|g| g.codons.len())
            .sum::<usize>()
            .max(1);

        1.0 - (unsafe_count as f64 / total as f64)
    }

    fn evaluate_maintainability(&self, genome: &CodeGenome) -> f64 {
        // Based on structure and complexity
        let avg_gene_size = if genome.size() > 0 {
            genome.genes.iter().map(|g| g.codons.len()).sum::<usize>() as f64 / genome.size() as f64
        } else {
            0.0
        };

        // Prefer smaller, more modular genes
        10.0 / (10.0 + avg_gene_size)
    }

    fn evaluate_robustness(&self, genome: &CodeGenome) -> f64 {
        // Check for error handling patterns
        let control_genes = genome
            .genes
            .iter()
            .filter(|g| matches!(g.gene_type, super::genome::GeneType::Control))
            .count();

        let total = genome.size().max(1);

        // Some control flow is good for error handling
        let ratio = control_genes as f64 / total as f64;
        (ratio * 10.0).min(1.0)
    }

    fn evaluate_parallelism(&self, genome: &CodeGenome) -> f64 {
        // Check for independent operations that could parallelize
        // Simplified: genes without memory dependencies are parallelizable

        let independent = genome
            .genes
            .iter()
            .filter(|g| !matches!(g.gene_type, super::genome::GeneType::Memory))
            .count();

        let total = genome.size().max(1);
        independent as f64 / total as f64
    }

    fn evaluate_cache_efficiency(&self, genome: &CodeGenome) -> f64 {
        // Check for sequential memory access patterns
        let mut sequential_count = 0;
        let mut total_memory = 0;

        for gene in genome.active_genes() {
            if matches!(gene.gene_type, super::genome::GeneType::Memory) {
                total_memory += 1;
                // Check for sequential addressing
                let has_sequential = gene
                    .codons
                    .iter()
                    .filter_map(|c| match c {
                        super::genome::Codon::Addr(a) => Some(*a),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .windows(2)
                    .any(|w| (w[1] - w[0]).abs() <= 64);

                if has_sequential {
                    sequential_count += 1;
                }
            }
        }

        if total_memory == 0 {
            1.0
        } else {
            sequential_count as f64 / total_memory as f64
        }
    }

    fn evaluate_branch_prediction(&self, genome: &CodeGenome) -> f64 {
        // Fewer branches are easier to predict
        let branch_count = genome
            .genes
            .iter()
            .filter(|g| matches!(g.gene_type, super::genome::GeneType::Control))
            .count();

        let total = genome.size().max(1);
        1.0 - (branch_count as f64 / total as f64).min(0.5) * 2.0
    }

    fn evaluate_vectorization(&self, genome: &CodeGenome) -> f64 {
        // Check for vectorizable patterns (independent arithmetic)
        let arithmetic_genes = genome
            .genes
            .iter()
            .filter(|g| matches!(g.gene_type, super::genome::GeneType::Arithmetic))
            .count();

        let control_genes = genome
            .genes
            .iter()
            .filter(|g| matches!(g.gene_type, super::genome::GeneType::Control))
            .count();

        if arithmetic_genes == 0 {
            return 0.0;
        }

        // More arithmetic relative to control is more vectorizable
        arithmetic_genes as f64 / (arithmetic_genes + control_genes).max(1) as f64
    }

    /// Clear cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get statistics
    pub fn stats(&self) -> &EvaluatorStats {
        &self.stats
    }
}

impl Default for FitnessEvaluator {
    fn default() -> Self {
        let mut evaluator = Self::new();

        // Add default objectives
        evaluator.add_objective(Objective::new(ObjectiveType::Correctness, 1.0, false));
        evaluator.add_objective(Objective::new(ObjectiveType::Performance, 0.5, false));
        evaluator.add_objective(Objective::new(ObjectiveType::CodeSize, 0.3, true));

        evaluator
    }
}

// ============================================================================
// PARETO FUNCTIONS
// ============================================================================

/// Calculate Pareto fronts
pub fn pareto_fronts(fitnesses: &[Fitness]) -> Vec<Vec<usize>> {
    let n = fitnesses.len();
    let mut dominated_by: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut domination_count: Vec<usize> = vec![0; n];
    let mut fronts: Vec<Vec<usize>> = Vec::new();

    // Calculate domination
    for i in 0..n {
        for j in 0..n {
            if i != j {
                if fitnesses[i].dominates(&fitnesses[j]) {
                    dominated_by[i].push(j);
                } else if fitnesses[j].dominates(&fitnesses[i]) {
                    domination_count[i] += 1;
                }
            }
        }
    }

    // First front
    let mut current_front: Vec<usize> = (0..n).filter(|&i| domination_count[i] == 0).collect();

    while !current_front.is_empty() {
        let mut next_front = Vec::new();

        for &i in &current_front {
            for &j in &dominated_by[i] {
                domination_count[j] -= 1;
                if domination_count[j] == 0 {
                    next_front.push(j);
                }
            }
        }

        fronts.push(current_front);
        current_front = next_front;
    }

    fronts
}

/// Calculate crowding distance
pub fn crowding_distance(fitnesses: &[Fitness], front: &[usize]) -> Vec<f64> {
    if front.is_empty() {
        return Vec::new();
    }

    let n = front.len();
    let m = fitnesses[front[0]].objectives.len();
    let mut distances = vec![0.0; n];

    for obj in 0..m {
        // Sort by objective
        let mut sorted: Vec<(usize, f64)> = front
            .iter()
            .enumerate()
            .map(|(i, &idx)| (i, fitnesses[idx].objectives[obj]))
            .collect();
        sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));

        // Boundary points get infinite distance
        distances[sorted[0].0] = f64::INFINITY;
        distances[sorted[n - 1].0] = f64::INFINITY;

        // Calculate distance for interior points
        let range = sorted[n - 1].1 - sorted[0].1;
        if range > 0.0 {
            for i in 1..(n - 1) {
                distances[sorted[i].0] += (sorted[i + 1].1 - sorted[i - 1].1) / range;
            }
        }
    }

    distances
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use core::sync::atomic::AtomicU64;

    use super::*;

    #[test]
    fn test_evaluator_creation() {
        let evaluator = FitnessEvaluator::default();
        assert!(!evaluator.objectives.is_empty());
    }

    #[test]
    fn test_fitness_evaluation() {
        let mut evaluator = FitnessEvaluator::default();
        let counter = AtomicU64::new(1);
        let genome = CodeGenome::random(1, 10, &counter);

        let fitness = evaluator.evaluate(&genome);
        assert!(!fitness.objectives.is_empty());
    }

    #[test]
    fn test_pareto_dominance() {
        let f1 = Fitness::new(vec![1.0, 1.0]);
        let f2 = Fitness::new(vec![0.5, 0.5]);

        assert!(f1.dominates(&f2));
        assert!(!f2.dominates(&f1));
    }

    #[test]
    fn test_pareto_fronts() {
        let fitnesses = vec![
            Fitness::new(vec![1.0, 0.0]),
            Fitness::new(vec![0.0, 1.0]),
            Fitness::new(vec![0.5, 0.5]),
            Fitness::new(vec![0.3, 0.3]),
        ];

        let fronts = pareto_fronts(&fitnesses);
        assert!(!fronts.is_empty());
    }
}
