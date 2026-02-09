// SPDX-License-Identifier: GPL-2.0
//! # Bridge Synthesis Engine — Self-Improvement Through Code Synthesis
//!
//! The bridge evolves its own algorithms at runtime. New optimisation
//! strategies are generated through evolutionary search: candidate
//! algorithms are represented as parameter vectors, evaluated against
//! a fitness function, mutated, crossed-over, and selected across
//! generations. The strongest survive and are deployed live.
//!
//! FNV-1a hashing identifies algorithms; xorshift64 drives mutation
//! and crossover; EMA tracks improvement velocity.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_POPULATION: usize = 128;
const MAX_GENOME_LEN: usize = 32;
const MAX_STRATEGIES: usize = 256;
const MAX_GENERATIONS: usize = 500;
const TOURNAMENT_SIZE: usize = 4;
const MUTATION_RATE: f32 = 0.15;
const CROSSOVER_RATE: f32 = 0.70;
const ELITE_FRACTION: f32 = 0.10;
const EMA_ALPHA: f32 = 0.10;
const NOVELTY_THRESHOLD: f32 = 0.30;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

fn abs_f32(x: f32) -> f32 {
    if x < 0.0 { -x } else { x }
}

// ============================================================================
// SYNTHESIS TYPES
// ============================================================================

/// Status of a synthesised algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlgorithmStatus {
    Evolving,
    Candidate,
    Validated,
    Deployed,
    Retired,
}

/// An individual in the evolutionary population.
#[derive(Debug, Clone)]
pub struct Individual {
    pub individual_id: u64,
    pub genome: Vec<f32>,
    pub fitness: f32,
    pub generation: u32,
    pub parent_ids: (u64, u64),
    pub mutation_count: u32,
}

/// A synthesised optimisation strategy derived from evolution.
#[derive(Debug, Clone)]
pub struct SynthesisedStrategy {
    pub strategy_id: u64,
    pub name: String,
    pub status: AlgorithmStatus,
    pub genome: Vec<f32>,
    pub fitness: f32,
    pub generation: u32,
    pub novelty_score: f32,
    pub deployed_tick: Option<u64>,
    pub improvement_over_baseline: f32,
}

/// Validation result for an evolved algorithm.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub strategy_id: u64,
    pub baseline_fitness: f32,
    pub evolved_fitness: f32,
    pub improvement: f32,
    pub is_valid: bool,
    pub trials: u32,
    pub consistency: f32,
}

/// Novelty assessment of a synthesised strategy.
#[derive(Debug, Clone)]
pub struct NoveltyAssessment {
    pub strategy_id: u64,
    pub novelty_score: f32,
    pub nearest_existing_distance: f32,
    pub genome_entropy: f32,
    pub is_novel: bool,
}

// ============================================================================
// SYNTHESIS STATS
// ============================================================================

/// Aggregate statistics for the synthesis engine.
#[derive(Debug, Clone, Copy, Default)]
pub struct SynthesisEngineStats {
    pub total_generations: u64,
    pub total_individuals: u64,
    pub total_strategies: u64,
    pub total_deployed: u64,
    pub best_fitness_ema: f32,
    pub avg_fitness_ema: f32,
    pub improvement_rate_ema: f32,
    pub avg_novelty_ema: f32,
    pub mutation_effectiveness_ema: f32,
}

// ============================================================================
// POPULATION MANAGER
// ============================================================================

#[derive(Debug)]
struct Population {
    individuals: Vec<Individual>,
    generation: u32,
    best_fitness: f32,
    avg_fitness_ema: f32,
}

impl Population {
    fn new() -> Self {
        Self {
            individuals: Vec::new(),
            generation: 0,
            best_fitness: 0.0,
            avg_fitness_ema: 0.0,
        }
    }

    fn initialise(&mut self, count: usize, genome_len: usize, rng: &mut u64) {
        self.individuals.clear();
        for _ in 0..count.min(MAX_POPULATION) {
            let id = xorshift64(rng);
            let genome: Vec<f32> = (0..genome_len.min(MAX_GENOME_LEN))
                .map(|_| {
                    let r = xorshift64(rng);
                    (r % 1000) as f32 / 1000.0
                })
                .collect();
            self.individuals.push(Individual {
                individual_id: id,
                genome,
                fitness: 0.0,
                generation: 0,
                parent_ids: (0, 0),
                mutation_count: 0,
            });
        }
    }

    fn tournament_select(&self, rng: &mut u64) -> usize {
        let n = self.individuals.len();
        if n == 0 { return 0; }
        let mut best_idx = (xorshift64(rng) % n as u64) as usize;
        for _ in 1..TOURNAMENT_SIZE {
            let idx = (xorshift64(rng) % n as u64) as usize;
            if self.individuals[idx].fitness > self.individuals[best_idx].fitness {
                best_idx = idx;
            }
        }
        best_idx
    }

    fn crossover(parent_a: &Individual, parent_b: &Individual, rng: &mut u64) -> Individual {
        let len = parent_a.genome.len().min(parent_b.genome.len());
        let cut = if len > 1 { (xorshift64(rng) % (len as u64 - 1) + 1) as usize } else { 0 };
        let mut genome = Vec::with_capacity(len);
        for i in 0..len {
            if i < cut {
                genome.push(parent_a.genome[i]);
            } else {
                genome.push(parent_b.genome[i]);
            }
        }
        let id = xorshift64(rng);
        Individual {
            individual_id: id,
            genome,
            fitness: 0.0,
            generation: parent_a.generation + 1,
            parent_ids: (parent_a.individual_id, parent_b.individual_id),
            mutation_count: 0,
        }
    }

    fn mutate(ind: &mut Individual, rate: f32, rng: &mut u64) {
        for gene in ind.genome.iter_mut() {
            let r = (xorshift64(rng) % 1000) as f32 / 1000.0;
            if r < rate {
                let delta = ((xorshift64(rng) % 200) as f32 / 1000.0) - 0.1;
                *gene = (*gene + delta).max(0.0).min(1.0);
                ind.mutation_count += 1;
            }
        }
    }

    fn update_stats(&mut self) {
        let n = self.individuals.len();
        if n == 0 { return; }
        let mut best = f32::MIN;
        let mut sum = 0.0f32;
        for ind in &self.individuals {
            if ind.fitness > best { best = ind.fitness; }
            sum += ind.fitness;
        }
        self.best_fitness = best;
        let avg = sum / n as f32;
        self.avg_fitness_ema = EMA_ALPHA * avg + (1.0 - EMA_ALPHA) * self.avg_fitness_ema;
    }
}

// ============================================================================
// BRIDGE SYNTHESIS ENGINE
// ============================================================================

/// Self-improvement engine that evolves new optimisation strategies
/// through genetic programming and validates them before deployment.
#[derive(Debug)]
pub struct BridgeSynthesisEngine {
    population: Population,
    strategies: BTreeMap<u64, SynthesisedStrategy>,
    tick: u64,
    rng_state: u64,
    baseline_fitness: f32,
    prev_best: f32,
    stats: SynthesisEngineStats,
}

impl BridgeSynthesisEngine {
    pub fn new(seed: u64, baseline_fitness: f32) -> Self {
        Self {
            population: Population::new(),
            strategies: BTreeMap::new(),
            tick: 0,
            rng_state: seed | 1,
            baseline_fitness,
            prev_best: baseline_fitness,
            stats: SynthesisEngineStats::default(),
        }
    }

    /// Run one evolutionary generation: evaluate fitness, select, cross,
    /// mutate, and return the best individual's genome + fitness.
    pub fn evolve_algorithm(
        &mut self,
        population_size: usize,
        genome_len: usize,
    ) -> (Vec<f32>, f32) {
        self.tick += 1;
        self.stats.total_generations += 1;

        // Initialise on first call.
        if self.population.individuals.is_empty() {
            self.population.initialise(population_size, genome_len, &mut self.rng_state);
        }

        // Evaluate fitness for each individual.
        for ind in self.population.individuals.iter_mut() {
            ind.fitness = self.evaluate_fitness(&ind.genome);
        }
        self.population.update_stats();
        self.population.generation += 1;

        let n = self.population.individuals.len();
        let elite_count = ((n as f32 * ELITE_FRACTION) as usize).max(1);

        // Sort by fitness descending.
        self.population.individuals.sort_by(|a, b| {
            b.fitness.partial_cmp(&a.fitness).unwrap_or(core::cmp::Ordering::Equal)
        });

        // Keep elites, breed the rest.
        let mut next_gen: Vec<Individual> = self.population.individuals[..elite_count].to_vec();

        while next_gen.len() < n {
            let r = (xorshift64(&mut self.rng_state) % 1000) as f32 / 1000.0;
            if r < CROSSOVER_RATE && self.population.individuals.len() >= 2 {
                let a = self.population.tournament_select(&mut self.rng_state);
                let b = self.population.tournament_select(&mut self.rng_state);
                let mut child = Population::crossover(
                    &self.population.individuals[a],
                    &self.population.individuals[b],
                    &mut self.rng_state,
                );
                Population::mutate(&mut child, MUTATION_RATE, &mut self.rng_state);
                next_gen.push(child);
            } else {
                // Clone + mutate
                let idx = self.population.tournament_select(&mut self.rng_state);
                let mut clone = self.population.individuals[idx].clone();
                clone.individual_id = xorshift64(&mut self.rng_state);
                Population::mutate(&mut clone, MUTATION_RATE * 2.0, &mut self.rng_state);
                next_gen.push(clone);
            }
        }
        next_gen.truncate(MAX_POPULATION);

        self.stats.total_individuals += next_gen.len() as u64;
        let improvement = self.population.best_fitness - self.prev_best;
        self.stats.improvement_rate_ema =
            EMA_ALPHA * improvement + (1.0 - EMA_ALPHA) * self.stats.improvement_rate_ema;
        self.stats.best_fitness_ema = EMA_ALPHA * self.population.best_fitness
            + (1.0 - EMA_ALPHA) * self.stats.best_fitness_ema;
        self.stats.avg_fitness_ema = self.population.avg_fitness_ema;
        self.prev_best = self.population.best_fitness;

        let best_genome = next_gen.first().map(|i| i.genome.clone()).unwrap_or_default();
        let best_fitness = next_gen.first().map(|i| i.fitness).unwrap_or(0.0);
        self.population.individuals = next_gen;

        (best_genome, best_fitness)
    }

    /// Promote the best evolved genome into a named strategy.
    pub fn generate_strategy(&mut self, name: String) -> SynthesisedStrategy {
        self.tick += 1;
        self.stats.total_strategies += 1;
        let sid = fnv1a_hash(name.as_bytes()) ^ xorshift64(&mut self.rng_state);

        let (genome, fitness) = if let Some(best) = self.population.individuals.first() {
            (best.genome.clone(), best.fitness)
        } else {
            (Vec::new(), 0.0)
        };

        let novelty = self.compute_novelty(&genome);
        let improvement = fitness - self.baseline_fitness;

        let strategy = SynthesisedStrategy {
            strategy_id: sid,
            name,
            status: AlgorithmStatus::Candidate,
            genome,
            fitness,
            generation: self.population.generation,
            novelty_score: novelty,
            deployed_tick: None,
            improvement_over_baseline: improvement,
        };

        if self.strategies.len() >= MAX_STRATEGIES {
            // Evict least-fit strategy.
            if let Some((&evict_id, _)) = self.strategies.iter()
                .min_by(|a, b| a.1.fitness.partial_cmp(&b.1.fitness).unwrap_or(core::cmp::Ordering::Equal))
            {
                self.strategies.remove(&evict_id);
            }
        }
        self.strategies.insert(sid, strategy.clone());
        strategy
    }

    /// Validate an evolved strategy through repeated trials.
    pub fn validate_evolution(&mut self, strategy_id: u64, trials: u32) -> ValidationResult {
        let (genome, fitness) = if let Some(s) = self.strategies.get(&strategy_id) {
            (s.genome.clone(), s.fitness)
        } else {
            return ValidationResult {
                strategy_id,
                baseline_fitness: self.baseline_fitness,
                evolved_fitness: 0.0,
                improvement: 0.0,
                is_valid: false,
                trials: 0,
                consistency: 0.0,
            };
        };

        let n = trials.min(100);
        let mut results = Vec::new();
        for _ in 0..n {
            let mut perturbed = genome.clone();
            for g in perturbed.iter_mut() {
                let r = xorshift64(&mut self.rng_state);
                let noise = ((r % 100) as f32 / 1000.0) - 0.05;
                *g = (*g + noise).max(0.0).min(1.0);
            }
            results.push(self.evaluate_fitness(&perturbed));
        }

        let mean: f32 = results.iter().sum::<f32>() / n.max(1) as f32;
        let variance: f32 = results.iter().map(|r| (r - mean) * (r - mean)).sum::<f32>() / n.max(1) as f32;
        let consistency = (1.0 - variance.min(1.0)).max(0.0);
        let improvement = mean - self.baseline_fitness;
        let is_valid = improvement > 0.0 && consistency > 0.5;

        if is_valid {
            if let Some(s) = self.strategies.get_mut(&strategy_id) {
                s.status = AlgorithmStatus::Validated;
            }
        }

        ValidationResult {
            strategy_id,
            baseline_fitness: self.baseline_fitness,
            evolved_fitness: mean,
            improvement,
            is_valid,
            trials: n,
            consistency,
        }
    }

    /// Compute the improvement rate (EMA-smoothed delta of best fitness
    /// per generation).
    pub fn improvement_rate(&self) -> f32 {
        self.stats.improvement_rate_ema
    }

    /// Assess how novel a strategy is compared to existing ones.
    pub fn synthesis_novelty(&self, strategy_id: u64) -> NoveltyAssessment {
        let genome = self.strategies.get(&strategy_id)
            .map(|s| &s.genome)
            .cloned()
            .unwrap_or_default();

        let novelty = self.compute_novelty(&genome);
        let nearest = self.nearest_distance(&genome);
        let entropy = self.genome_entropy(&genome);

        NoveltyAssessment {
            strategy_id,
            novelty_score: novelty,
            nearest_existing_distance: nearest,
            genome_entropy: entropy,
            is_novel: novelty > NOVELTY_THRESHOLD,
        }
    }

    /// Deploy a validated strategy.
    pub fn deploy_strategy(&mut self, strategy_id: u64) -> bool {
        if let Some(s) = self.strategies.get_mut(&strategy_id) {
            if s.status == AlgorithmStatus::Validated {
                s.status = AlgorithmStatus::Deployed;
                s.deployed_tick = Some(self.tick);
                self.stats.total_deployed += 1;
                return true;
            }
        }
        false
    }

    /// Aggregate statistics.
    pub fn stats(&self) -> SynthesisEngineStats {
        self.stats
    }

    // ---- internal helpers ----

    /// Proxy fitness: reward genomes whose elements are diverse and sum
    /// to a moderate total (simulating a balanced optimisation strategy).
    fn evaluate_fitness(&self, genome: &[f32]) -> f32 {
        if genome.is_empty() { return 0.0; }
        let sum: f32 = genome.iter().sum();
        let mean = sum / genome.len() as f32;
        let variance: f32 = genome.iter().map(|g| (g - mean) * (g - mean)).sum::<f32>() / genome.len() as f32;
        let balance = 1.0 - abs_f32(mean - 0.5) * 2.0; // reward mean ≈ 0.5
        let diversity = variance.min(0.25) / 0.25; // reward some variance
        balance * 0.6 + diversity * 0.4
    }

    fn compute_novelty(&self, genome: &[f32]) -> f32 {
        if self.strategies.is_empty() { return 1.0; }
        let nearest = self.nearest_distance(genome);
        (nearest * 4.0).min(1.0)
    }

    fn nearest_distance(&self, genome: &[f32]) -> f32 {
        let mut min_dist = f32::MAX;
        for s in self.strategies.values() {
            let dist = genome.iter()
                .zip(s.genome.iter())
                .map(|(a, b)| (a - b) * (a - b))
                .sum::<f32>();
            let d = if dist > 0.0 {
                // sqrt approximation: Newton's method, 3 iterations
                let mut x = dist;
                x = 0.5 * (x + dist / x);
                x = 0.5 * (x + dist / x);
                x = 0.5 * (x + dist / x);
                x
            } else {
                0.0
            };
            if d < min_dist { min_dist = d; }
        }
        min_dist
    }

    fn genome_entropy(&self, genome: &[f32]) -> f32 {
        if genome.is_empty() { return 0.0; }
        // Bucket into 10 bins, compute histogram entropy.
        let mut bins = [0u32; 10];
        for &g in genome {
            let idx = ((g * 10.0) as usize).min(9);
            bins[idx] += 1;
        }
        let n = genome.len() as f32;
        let mut entropy = 0.0f32;
        for &count in &bins {
            if count > 0 {
                let p = count as f32 / n;
                entropy -= p * approx_ln(p);
            }
        }
        entropy
    }
}

/// Approximate natural logarithm for no_std (Taylor series around 1).
fn approx_ln(x: f32) -> f32 {
    let t = (x - 1.0) / (x + 1.0);
    let t2 = t * t;
    let mut term = t;
    let mut sum = t;
    for k in 1..8u32 {
        term *= t2;
        sum += term / (2 * k + 1) as f32;
    }
    2.0 * sum
}
