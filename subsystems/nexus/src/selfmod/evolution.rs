//! # Code Evolution Engine
//!
//! Year 3 EVOLUTION - Evolutionary optimization of self-modifying code

#![allow(dead_code)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

// ============================================================================
// EVOLUTION TYPES
// ============================================================================

/// Evolution ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvolutionId(pub u64);

static EVOLUTION_COUNTER: AtomicU64 = AtomicU64::new(1);

impl EvolutionId {
    pub fn generate() -> Self {
        Self(EVOLUTION_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Variant ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VariantId(pub u64);

static VARIANT_COUNTER: AtomicU64 = AtomicU64::new(1);

impl VariantId {
    pub fn generate() -> Self {
        Self(VARIANT_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Evolution configuration
#[derive(Debug, Clone)]
pub struct EvolutionConfig {
    /// Population size
    pub population_size: usize,
    /// Number of generations
    pub max_generations: usize,
    /// Mutation rate
    pub mutation_rate: f64,
    /// Crossover rate
    pub crossover_rate: f64,
    /// Elite count
    pub elite_count: usize,
    /// Tournament size
    pub tournament_size: usize,
    /// Fitness threshold for termination
    pub fitness_threshold: Option<f64>,
    /// Stagnation limit (generations without improvement)
    pub stagnation_limit: usize,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            population_size: 50,
            max_generations: 100,
            mutation_rate: 0.1,
            crossover_rate: 0.8,
            elite_count: 2,
            tournament_size: 3,
            fitness_threshold: None,
            stagnation_limit: 20,
        }
    }
}

// ============================================================================
// CODE VARIANT
// ============================================================================

/// Code variant
#[derive(Debug, Clone)]
pub struct CodeVariant {
    /// ID
    pub id: VariantId,
    /// Parent ID
    pub parent: Option<VariantId>,
    /// Generation
    pub generation: u32,
    /// Genome (code representation)
    pub genome: Genome,
    /// Fitness
    pub fitness: Option<Fitness>,
    /// Metadata
    pub metadata: BTreeMap<String, String>,
}

impl CodeVariant {
    pub fn new(genome: Genome) -> Self {
        Self {
            id: VariantId::generate(),
            parent: None,
            generation: 0,
            genome,
            fitness: None,
            metadata: BTreeMap::new(),
        }
    }

    pub fn with_parent(genome: Genome, parent: VariantId, generation: u32) -> Self {
        Self {
            id: VariantId::generate(),
            parent: Some(parent),
            generation,
            genome,
            fitness: None,
            metadata: BTreeMap::new(),
        }
    }
}

/// Code genome
#[derive(Debug, Clone)]
pub struct Genome {
    /// Code fragments
    pub fragments: Vec<CodeFragment>,
    /// Parameters
    pub parameters: BTreeMap<String, f64>,
    /// Flags
    pub flags: BTreeMap<String, bool>,
    /// Strategy selections
    pub strategies: BTreeMap<String, u32>,
}

impl Genome {
    pub fn new() -> Self {
        Self {
            fragments: Vec::new(),
            parameters: BTreeMap::new(),
            flags: BTreeMap::new(),
            strategies: BTreeMap::new(),
        }
    }

    /// Distance to another genome
    pub fn distance(&self, other: &Genome) -> f64 {
        let mut dist = 0.0;
        let mut count = 0;

        // Parameter distance
        for (key, &val) in &self.parameters {
            if let Some(&other_val) = other.parameters.get(key) {
                dist += (val - other_val).abs();
                count += 1;
            }
        }

        // Flag distance
        for (key, &val) in &self.flags {
            if let Some(&other_val) = other.flags.get(key) {
                if val != other_val {
                    dist += 1.0;
                }
                count += 1;
            }
        }

        // Strategy distance
        for (key, &val) in &self.strategies {
            if let Some(&other_val) = other.strategies.get(key) {
                if val != other_val {
                    dist += 1.0;
                }
                count += 1;
            }
        }

        if count > 0 { dist / count as f64 } else { 0.0 }
    }
}

impl Default for Genome {
    fn default() -> Self {
        Self::new()
    }
}

/// Code fragment
#[derive(Debug, Clone)]
pub struct CodeFragment {
    /// Fragment ID
    pub id: u64,
    /// Fragment type
    pub fragment_type: FragmentType,
    /// Code content
    pub code: String,
    /// Dependencies
    pub dependencies: Vec<u64>,
    /// Mutable regions
    pub mutable_regions: Vec<MutableRegion>,
}

/// Fragment type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FragmentType {
    Function,
    Block,
    Expression,
    Statement,
    Pattern,
    Type,
}

/// Mutable region
#[derive(Debug, Clone)]
pub struct MutableRegion {
    /// Start offset
    pub start: usize,
    /// End offset
    pub end: usize,
    /// Region type
    pub region_type: RegionType,
    /// Possible values
    pub options: Vec<String>,
}

/// Region type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionType {
    Constant,
    Operator,
    Method,
    Strategy,
    Expression,
}

/// Fitness
#[derive(Debug, Clone)]
pub struct Fitness {
    /// Overall score
    pub score: f64,
    /// Performance metrics
    pub performance: PerformanceMetrics,
    /// Quality metrics
    pub quality: QualityMetrics,
    /// Constraint satisfaction
    pub constraints_satisfied: bool,
    /// Evaluation time
    pub eval_time: u64,
}

/// Performance metrics
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    /// Execution time (ns)
    pub execution_time: u64,
    /// Memory usage (bytes)
    pub memory_usage: usize,
    /// Throughput
    pub throughput: f64,
    /// Latency (ns)
    pub latency: u64,
    /// Cache hits ratio
    pub cache_hit_ratio: f64,
}

/// Quality metrics
#[derive(Debug, Clone, Default)]
pub struct QualityMetrics {
    /// Code size
    pub code_size: usize,
    /// Complexity
    pub complexity: usize,
    /// Test coverage
    pub coverage: f64,
    /// Error rate
    pub error_rate: f64,
}

// ============================================================================
// RANDOM
// ============================================================================

struct Rng {
    state: AtomicU64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self {
            state: AtomicU64::new(seed),
        }
    }

    fn next(&self) -> u64 {
        let mut x = self.state.load(Ordering::Relaxed);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state.store(x, Ordering::Relaxed);
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    fn next_f64(&self) -> f64 {
        (self.next() as f64) / (u64::MAX as f64)
    }

    fn next_usize(&self, max: usize) -> usize {
        (self.next() as usize) % max
    }

    fn next_bool(&self, prob: f64) -> bool {
        self.next_f64() < prob
    }

    fn next_gaussian(&self) -> f64 {
        let u1 = self.next_f64();
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (2.0 * core::f64::consts::PI * u2).cos()
    }
}

// ============================================================================
// EVOLUTION ENGINE
// ============================================================================

/// Evolution engine
pub struct EvolutionEngine {
    /// Configuration
    config: EvolutionConfig,
    /// Population
    population: Vec<CodeVariant>,
    /// Best variant
    best: Option<CodeVariant>,
    /// Generation counter
    generation: u32,
    /// Stagnation counter
    stagnation: usize,
    /// Running flag
    running: AtomicBool,
    /// History
    history: Vec<GenerationStats>,
    /// Random
    rng: Rng,
    /// Mutation operators
    mutations: Vec<MutationOperator>,
    /// Crossover operators
    crossovers: Vec<CrossoverOperator>,
}

/// Generation statistics
#[derive(Debug, Clone)]
pub struct GenerationStats {
    /// Generation number
    pub generation: u32,
    /// Best fitness
    pub best_fitness: f64,
    /// Average fitness
    pub avg_fitness: f64,
    /// Worst fitness
    pub worst_fitness: f64,
    /// Diversity
    pub diversity: f64,
    /// Number of evaluations
    pub evaluations: usize,
}

impl EvolutionEngine {
    pub fn new(config: EvolutionConfig) -> Self {
        Self {
            config,
            population: Vec::new(),
            best: None,
            generation: 0,
            stagnation: 0,
            running: AtomicBool::new(false),
            history: Vec::new(),
            rng: Rng::new(0xDEADBEEF),
            mutations: Vec::new(),
            crossovers: Vec::new(),
        }
    }

    /// Initialize population
    pub fn initialize<F>(&mut self, generator: F)
    where
        F: Fn(usize, &Rng) -> Genome,
    {
        self.population.clear();
        self.generation = 0;
        self.stagnation = 0;
        self.best = None;

        for i in 0..self.config.population_size {
            let genome = generator(i, &self.rng);
            self.population.push(CodeVariant::new(genome));
        }
    }

    /// Run evolution
    pub fn evolve<F>(&mut self, evaluator: F) -> Option<&CodeVariant>
    where
        F: Fn(&CodeVariant) -> Fitness,
    {
        self.running.store(true, Ordering::Relaxed);

        while self.running.load(Ordering::Relaxed)
            && self.generation < self.config.max_generations as u32
            && self.stagnation < self.config.stagnation_limit
        {
            // Evaluate population
            for variant in &mut self.population {
                if variant.fitness.is_none() {
                    variant.fitness = Some(evaluator(variant));
                }
            }

            // Update best
            let current_best = self
                .population
                .iter()
                .filter_map(|v| v.fitness.as_ref().map(|f| (v, f.score)))
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            if let Some((best, score)) = current_best {
                let improved = self
                    .best
                    .as_ref()
                    .and_then(|b| b.fitness.as_ref())
                    .map(|f| score > f.score)
                    .unwrap_or(true);

                if improved {
                    self.best = Some(best.clone());
                    self.stagnation = 0;
                } else {
                    self.stagnation += 1;
                }

                // Check fitness threshold
                if let Some(threshold) = self.config.fitness_threshold {
                    if score >= threshold {
                        break;
                    }
                }
            }

            // Record stats
            self.record_stats();

            // Generate next generation
            self.next_generation();

            self.generation += 1;
        }

        self.running.store(false, Ordering::Relaxed);
        self.best.as_ref()
    }

    fn record_stats(&mut self) {
        let fitnesses: Vec<f64> = self
            .population
            .iter()
            .filter_map(|v| v.fitness.as_ref().map(|f| f.score))
            .collect();

        if fitnesses.is_empty() {
            return;
        }

        let best = fitnesses.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let worst = fitnesses.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let avg = fitnesses.iter().sum::<f64>() / fitnesses.len() as f64;

        // Calculate diversity
        let diversity = self.calculate_diversity();

        self.history.push(GenerationStats {
            generation: self.generation,
            best_fitness: best,
            avg_fitness: avg,
            worst_fitness: worst,
            diversity,
            evaluations: fitnesses.len(),
        });
    }

    fn calculate_diversity(&self) -> f64 {
        if self.population.len() < 2 {
            return 0.0;
        }

        let mut total_dist = 0.0;
        let mut count = 0;

        for i in 0..self.population.len() {
            for j in (i + 1)..self.population.len() {
                total_dist += self.population[i]
                    .genome
                    .distance(&self.population[j].genome);
                count += 1;
            }
        }

        if count > 0 {
            total_dist / count as f64
        } else {
            0.0
        }
    }

    fn next_generation(&mut self) {
        let mut new_population = Vec::with_capacity(self.config.population_size);

        // Elitism
        let mut sorted: Vec<_> = self.population.iter().collect();
        sorted.sort_by(|a, b| {
            let fa = a
                .fitness
                .as_ref()
                .map(|f| f.score)
                .unwrap_or(f64::NEG_INFINITY);
            let fb = b
                .fitness
                .as_ref()
                .map(|f| f.score)
                .unwrap_or(f64::NEG_INFINITY);
            fb.partial_cmp(&fa).unwrap()
        });

        for elite in sorted.iter().take(self.config.elite_count) {
            let mut clone = (*elite).clone();
            clone.generation = self.generation + 1;
            new_population.push(clone);
        }

        // Generate rest through selection, crossover, mutation
        while new_population.len() < self.config.population_size {
            let parent1 = self.tournament_select();
            let parent2 = self.tournament_select();

            let mut offspring = if self.rng.next_bool(self.config.crossover_rate) {
                self.crossover(parent1, parent2)
            } else {
                vec![parent1.clone(), parent2.clone()]
            };

            for child in &mut offspring {
                if self.rng.next_bool(self.config.mutation_rate) {
                    self.mutate(child);
                }

                child.id = VariantId::generate();
                child.generation = self.generation + 1;
                child.fitness = None;

                if new_population.len() < self.config.population_size {
                    new_population.push(child.clone());
                }
            }
        }

        self.population = new_population;
    }

    fn tournament_select(&self) -> &CodeVariant {
        let mut best: Option<&CodeVariant> = None;
        let mut best_fitness = f64::NEG_INFINITY;

        for _ in 0..self.config.tournament_size {
            let idx = self.rng.next_usize(self.population.len());
            let candidate = &self.population[idx];

            let fitness = candidate.fitness.as_ref().map(|f| f.score).unwrap_or(0.0);
            if fitness > best_fitness {
                best_fitness = fitness;
                best = Some(candidate);
            }
        }

        best.unwrap_or(&self.population[0])
    }

    fn crossover(&self, p1: &CodeVariant, p2: &CodeVariant) -> Vec<CodeVariant> {
        let mut c1_genome = Genome::new();
        let mut c2_genome = Genome::new();

        // Uniform crossover for parameters
        for (key, &val1) in &p1.genome.parameters {
            if let Some(&val2) = p2.genome.parameters.get(key) {
                if self.rng.next_bool(0.5) {
                    c1_genome.parameters.insert(key.clone(), val1);
                    c2_genome.parameters.insert(key.clone(), val2);
                } else {
                    c1_genome.parameters.insert(key.clone(), val2);
                    c2_genome.parameters.insert(key.clone(), val1);
                }
            } else {
                c1_genome.parameters.insert(key.clone(), val1);
            }
        }

        // Uniform crossover for flags
        for (key, &val1) in &p1.genome.flags {
            if let Some(&val2) = p2.genome.flags.get(key) {
                if self.rng.next_bool(0.5) {
                    c1_genome.flags.insert(key.clone(), val1);
                    c2_genome.flags.insert(key.clone(), val2);
                } else {
                    c1_genome.flags.insert(key.clone(), val2);
                    c2_genome.flags.insert(key.clone(), val1);
                }
            } else {
                c1_genome.flags.insert(key.clone(), val1);
            }
        }

        // Crossover strategies
        for (key, &val1) in &p1.genome.strategies {
            if let Some(&val2) = p2.genome.strategies.get(key) {
                if self.rng.next_bool(0.5) {
                    c1_genome.strategies.insert(key.clone(), val1);
                    c2_genome.strategies.insert(key.clone(), val2);
                } else {
                    c1_genome.strategies.insert(key.clone(), val2);
                    c2_genome.strategies.insert(key.clone(), val1);
                }
            } else {
                c1_genome.strategies.insert(key.clone(), val1);
            }
        }

        // Crossover fragments (single point)
        let len = p1.genome.fragments.len().min(p2.genome.fragments.len());
        if len > 0 {
            let point = self.rng.next_usize(len);
            c1_genome.fragments = p1.genome.fragments[..point].to_vec();
            c1_genome
                .fragments
                .extend_from_slice(&p2.genome.fragments[point..]);

            c2_genome.fragments = p2.genome.fragments[..point].to_vec();
            c2_genome
                .fragments
                .extend_from_slice(&p1.genome.fragments[point..]);
        }

        vec![
            CodeVariant::with_parent(c1_genome, p1.id, self.generation + 1),
            CodeVariant::with_parent(c2_genome, p2.id, self.generation + 1),
        ]
    }

    fn mutate(&self, variant: &mut CodeVariant) {
        // Mutate parameters (Gaussian perturbation)
        for (_, val) in variant.genome.parameters.iter_mut() {
            if self.rng.next_bool(0.3) {
                *val += self.rng.next_gaussian() * 0.1;
            }
        }

        // Mutate flags (flip)
        for (_, val) in variant.genome.flags.iter_mut() {
            if self.rng.next_bool(0.1) {
                *val = !*val;
            }
        }

        // Mutate strategies
        for (_, val) in variant.genome.strategies.iter_mut() {
            if self.rng.next_bool(0.1) {
                *val = self.rng.next() as u32 % 10;
            }
        }

        // Mutate fragments
        for fragment in &mut variant.genome.fragments {
            if self.rng.next_bool(0.05) {
                for region in &mut fragment.mutable_regions {
                    if !region.options.is_empty() && self.rng.next_bool(0.2) {
                        let idx = self.rng.next_usize(region.options.len());
                        // Would replace the region with options[idx]
                        let _ = &region.options[idx];
                    }
                }
            }
        }
    }

    /// Get best variant
    pub fn best(&self) -> Option<&CodeVariant> {
        self.best.as_ref()
    }

    /// Get history
    pub fn history(&self) -> &[GenerationStats] {
        &self.history
    }

    /// Get current generation
    pub fn generation(&self) -> u32 {
        self.generation
    }

    /// Stop evolution
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    /// Is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
}

/// Mutation operator
pub struct MutationOperator {
    /// Name
    pub name: String,
    /// Probability
    pub probability: f64,
    /// Mutator function
    pub mutate: Box<dyn Fn(&mut Genome, &Rng) + Send + Sync>,
}

/// Crossover operator
pub struct CrossoverOperator {
    /// Name
    pub name: String,
    /// Probability
    pub probability: f64,
    /// Crossover function
    pub crossover: Box<dyn Fn(&Genome, &Genome, &Rng) -> (Genome, Genome) + Send + Sync>,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genome_distance() {
        let mut g1 = Genome::new();
        g1.parameters.insert(String::from("x"), 1.0);
        g1.parameters.insert(String::from("y"), 2.0);

        let mut g2 = Genome::new();
        g2.parameters.insert(String::from("x"), 1.5);
        g2.parameters.insert(String::from("y"), 2.5);

        let dist = g1.distance(&g2);
        assert!(dist > 0.0);
    }

    #[test]
    fn test_evolution_engine() {
        let config = EvolutionConfig {
            population_size: 10,
            max_generations: 5,
            ..Default::default()
        };

        let mut engine = EvolutionEngine::new(config);

        engine.initialize(|_, rng| {
            let mut genome = Genome::new();
            genome
                .parameters
                .insert(String::from("x"), rng.next_f64() * 10.0);
            genome
        });

        let best = engine.evolve(|variant| {
            let x = variant.genome.parameters.get("x").copied().unwrap_or(0.0);
            // Maximize: -(x-5)^2 => optimum at x=5
            let score = -(x - 5.0).powi(2);

            Fitness {
                score,
                performance: PerformanceMetrics::default(),
                quality: QualityMetrics::default(),
                constraints_satisfied: true,
                eval_time: 0,
            }
        });

        assert!(best.is_some());
        assert!(engine.generation() > 0);
    }

    #[test]
    fn test_code_variant() {
        let genome = Genome::new();
        let variant = CodeVariant::new(genome);

        assert!(variant.parent.is_none());
        assert!(variant.fitness.is_none());
    }
}
