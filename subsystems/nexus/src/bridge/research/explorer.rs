// SPDX-License-Identifier: GPL-2.0
//! # Bridge Explorer — Autonomous Algorithmic Exploration
//!
//! Genetic-algorithm-driven exploration of the bridge optimization space.
//! Each individual in the population encodes a set of strategy parameters
//! (routing weights, batch thresholds, cache sizes). The explorer mutates,
//! crosses over, and selects based on fitness measured against real bridge
//! telemetry. Elitism preserves the best strategies across generations,
//! while mutation rate adapts to prevent premature convergence.
//!
//! The bridge that evolves its own optimization strategies.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_POPULATION: usize = 64;
const MAX_PARAMS: usize = 16;
const MAX_GENERATIONS: usize = 4096;
const ELITISM_FRACTION: f32 = 0.15;
const BASE_MUTATION_RATE: f32 = 0.08;
const CROSSOVER_RATE: f32 = 0.70;
const EMA_ALPHA: f32 = 0.12;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const TOURNAMENT_SIZE: usize = 4;
const STAGNATION_LIMIT: u32 = 20;
const MUTATION_BOOST: f32 = 0.25;

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

fn xorshift_f32(state: &mut u64) -> f32 {
    (xorshift64(state) % 10000) as f32 / 10000.0
}

// ============================================================================
// STRATEGY PARAMETER TYPES
// ============================================================================

/// A single tunable parameter in a strategy
#[derive(Debug, Clone)]
pub struct StrategyParam {
    pub name: String,
    pub value: f32,
    pub min_value: f32,
    pub max_value: f32,
    pub mutation_sigma: f32,
}

/// An individual in the population — a complete strategy
#[derive(Debug, Clone)]
pub struct Individual {
    pub id: u64,
    pub params: Vec<StrategyParam>,
    pub fitness: f32,
    pub generation_born: u32,
    pub evaluated: bool,
}

/// Category of exploration being performed
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExplorationDomain {
    RoutingStrategy,
    BatchingHeuristic,
    CachingPolicy,
    PrefetchTiming,
    CoalescingThreshold,
}

/// Fitness evaluation result
#[derive(Debug, Clone)]
pub struct FitnessResult {
    pub individual_id: u64,
    pub fitness: f32,
    pub throughput_score: f32,
    pub latency_score: f32,
    pub efficiency_score: f32,
    pub domain: ExplorationDomain,
}

/// Exploration progress snapshot
#[derive(Debug, Clone)]
pub struct ExplorationProgress {
    pub generation: u32,
    pub best_fitness: f32,
    pub mean_fitness: f32,
    pub fitness_variance: f32,
    pub diversity_index: f32,
    pub stagnation_count: u32,
    pub mutation_rate: f32,
    pub population_size: usize,
}

// ============================================================================
// EXPLORER STATS
// ============================================================================

/// Aggregate exploration statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct ExplorerStats {
    pub total_generations: u64,
    pub total_evaluations: u64,
    pub best_fitness_ever: f32,
    pub current_best_fitness: f32,
    pub avg_fitness_ema: f32,
    pub fitness_improvement_rate: f32,
    pub crossovers_performed: u64,
    pub mutations_performed: u64,
    pub elites_preserved: u64,
    pub stagnation_resets: u64,
}

// ============================================================================
// FITNESS CURVE TRACKER
// ============================================================================

/// Tracks the fitness improvement curve across generations
#[derive(Debug, Clone)]
struct FitnessCurve {
    best_per_generation: BTreeMap<u32, f32>,
    mean_per_generation: BTreeMap<u32, f32>,
    improvement_ema: f32,
}

impl FitnessCurve {
    fn new() -> Self {
        Self {
            best_per_generation: BTreeMap::new(),
            mean_per_generation: BTreeMap::new(),
            improvement_ema: 0.0,
        }
    }

    fn record(&mut self, generation: u32, best: f32, mean: f32) {
        let prev_best = self
            .best_per_generation
            .values()
            .last()
            .copied()
            .unwrap_or(0.0);
        let improvement = if prev_best > 0.0 {
            (best - prev_best) / prev_best
        } else {
            0.0
        };
        self.improvement_ema = EMA_ALPHA * improvement + (1.0 - EMA_ALPHA) * self.improvement_ema;
        self.best_per_generation.insert(generation, best);
        self.mean_per_generation.insert(generation, mean);

        // Keep bounded
        while self.best_per_generation.len() > MAX_GENERATIONS {
            if let Some(&first) = self.best_per_generation.keys().next() {
                self.best_per_generation.remove(&first);
                self.mean_per_generation.remove(&first);
            }
        }
    }
}

// ============================================================================
// BRIDGE EXPLORER
// ============================================================================

/// Autonomous genetic-algorithm bridge optimizer
#[derive(Debug)]
pub struct BridgeExplorer {
    population: Vec<Individual>,
    fitness_curve: FitnessCurve,
    domain_best: BTreeMap<u64, Individual>,
    generation: u32,
    rng_state: u64,
    mutation_rate: f32,
    stagnation_count: u32,
    stats: ExplorerStats,
}

impl BridgeExplorer {
    /// Create a new explorer with a seed
    pub fn new(seed: u64) -> Self {
        Self {
            population: Vec::new(),
            fitness_curve: FitnessCurve::new(),
            domain_best: BTreeMap::new(),
            generation: 0,
            rng_state: seed | 1,
            mutation_rate: BASE_MUTATION_RATE,
            stagnation_count: 0,
            stats: ExplorerStats::default(),
        }
    }

    /// Explore a new strategy in the given domain — initializes a random individual
    pub fn explore_strategy(&mut self, domain: ExplorationDomain, param_count: usize) -> Individual {
        let count = param_count.min(MAX_PARAMS);
        let mut params = Vec::with_capacity(count);
        for i in 0..count {
            let v = xorshift_f32(&mut self.rng_state);
            let name_hash = fnv1a_hash(&(i as u64).to_le_bytes());
            let mut name = String::from("p_");
            // Encode param index into name
            let idx_byte = (i as u8) + b'0';
            name.push(idx_byte as char);
            params.push(StrategyParam {
                name,
                value: v,
                min_value: 0.0,
                max_value: 1.0,
                mutation_sigma: 0.1,
            });
            let _ = name_hash; // used for seeding
        }
        let id = fnv1a_hash(&self.generation.to_le_bytes()) ^ xorshift64(&mut self.rng_state);
        let ind = Individual {
            id,
            params,
            fitness: 0.0,
            generation_born: self.generation,
            evaluated: false,
        };
        if self.population.len() < MAX_POPULATION {
            self.population.push(ind.clone());
        }
        ind
    }

    /// Mutate an individual's parameters with adaptive mutation rate
    pub fn mutate_params(&mut self, individual: &mut Individual) {
        for param in individual.params.iter_mut() {
            if xorshift_f32(&mut self.rng_state) < self.mutation_rate {
                let delta =
                    (xorshift_f32(&mut self.rng_state) - 0.5) * 2.0 * param.mutation_sigma;
                param.value = (param.value + delta).clamp(param.min_value, param.max_value);
                self.stats.mutations_performed += 1;
            }
        }
    }

    /// Crossover two parents to produce an offspring
    pub fn crossover(&mut self, parent_a: &Individual, parent_b: &Individual) -> Individual {
        let len = parent_a.params.len().min(parent_b.params.len());
        let mut child_params = Vec::with_capacity(len);
        for i in 0..len {
            let pick_a = xorshift_f32(&mut self.rng_state) < 0.5;
            let base = if pick_a {
                &parent_a.params[i]
            } else {
                &parent_b.params[i]
            };
            // Blend crossover: average with some randomness
            let alpha = xorshift_f32(&mut self.rng_state);
            let blended_value = alpha * parent_a.params[i].value
                + (1.0 - alpha) * parent_b.params[i].value;
            child_params.push(StrategyParam {
                name: base.name.clone(),
                value: blended_value.clamp(base.min_value, base.max_value),
                min_value: base.min_value,
                max_value: base.max_value,
                mutation_sigma: base.mutation_sigma,
            });
        }
        let id = fnv1a_hash(&parent_a.id.to_le_bytes()) ^ fnv1a_hash(&parent_b.id.to_le_bytes());
        self.stats.crossovers_performed += 1;
        Individual {
            id,
            params: child_params,
            fitness: 0.0,
            generation_born: self.generation,
            evaluated: false,
        }
    }

    /// Evaluate an individual's fitness based on observed performance metrics
    pub fn fitness_evaluate(
        &mut self,
        individual: &mut Individual,
        throughput: f32,
        latency: f32,
        efficiency: f32,
        domain: ExplorationDomain,
    ) -> FitnessResult {
        let throughput_score = throughput.clamp(0.0, 1.0);
        let latency_score = (1.0 - latency.clamp(0.0, 1.0)).max(0.0);
        let efficiency_score = efficiency.clamp(0.0, 1.0);
        let fitness = throughput_score * 0.4 + latency_score * 0.35 + efficiency_score * 0.25;
        individual.fitness = fitness;
        individual.evaluated = true;
        self.stats.total_evaluations += 1;

        let domain_key = domain as u64;
        let is_new_best = self
            .domain_best
            .get(&domain_key)
            .map_or(true, |b| fitness > b.fitness);
        if is_new_best {
            self.domain_best.insert(domain_key, individual.clone());
        }

        if fitness > self.stats.best_fitness_ever {
            self.stats.best_fitness_ever = fitness;
        }
        self.stats.avg_fitness_ema =
            EMA_ALPHA * fitness + (1.0 - EMA_ALPHA) * self.stats.avg_fitness_ema;

        FitnessResult {
            individual_id: individual.id,
            fitness,
            throughput_score,
            latency_score,
            efficiency_score,
            domain,
        }
    }

    /// Elitism selection: keep top fraction, replace rest via tournament selection
    pub fn elitism_select(&mut self) {
        if self.population.is_empty() {
            return;
        }
        // Sort by fitness descending
        self.population
            .sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap_or(core::cmp::Ordering::Equal));

        let elite_count = ((self.population.len() as f32 * ELITISM_FRACTION) as usize).max(1);
        let prev_best = self.stats.current_best_fitness;
        self.stats.current_best_fitness = self.population.first().map_or(0.0, |i| i.fitness);

        // Check stagnation
        let improved = self.stats.current_best_fitness > prev_best + 0.001;
        if improved {
            self.stagnation_count = 0;
            self.mutation_rate = BASE_MUTATION_RATE;
        } else {
            self.stagnation_count += 1;
            if self.stagnation_count > STAGNATION_LIMIT {
                self.mutation_rate = (self.mutation_rate + MUTATION_BOOST).min(0.5);
                self.stats.stagnation_resets += 1;
                self.stagnation_count = 0;
            }
        }

        // Record fitness curve
        let mean = if self.population.is_empty() {
            0.0
        } else {
            self.population.iter().map(|i| i.fitness).sum::<f32>()
                / self.population.len() as f32
        };
        self.fitness_curve
            .record(self.generation, self.stats.current_best_fitness, mean);

        // Tournament selection for non-elite slots
        let elites: Vec<Individual> = self.population[..elite_count].to_vec();
        let mut new_pop = elites.clone();
        self.stats.elites_preserved += elite_count as u64;

        while new_pop.len() < MAX_POPULATION.min(self.population.len()) {
            // Tournament: pick best from random sample
            let mut best_idx = xorshift64(&mut self.rng_state) as usize % self.population.len();
            for _ in 1..TOURNAMENT_SIZE {
                let idx = xorshift64(&mut self.rng_state) as usize % self.population.len();
                if self.population[idx].fitness > self.population[best_idx].fitness {
                    best_idx = idx;
                }
            }
            let parent_a = self.population[best_idx].clone();
            let parent_b_idx = xorshift64(&mut self.rng_state) as usize % self.population.len();
            let parent_b = self.population[parent_b_idx].clone();

            if xorshift_f32(&mut self.rng_state) < CROSSOVER_RATE {
                let mut child = self.crossover(&parent_a, &parent_b);
                self.mutate_params(&mut child);
                new_pop.push(child);
            } else {
                let mut clone = parent_a;
                self.mutate_params(&mut clone);
                new_pop.push(clone);
            }
        }

        self.population = new_pop;
        self.generation += 1;
        self.stats.total_generations += 1;
        self.stats.fitness_improvement_rate = self.fitness_curve.improvement_ema;
    }

    /// Get current exploration progress
    pub fn exploration_progress(&self) -> ExplorationProgress {
        let (mean, variance) = if self.population.is_empty() {
            (0.0, 0.0)
        } else {
            let m = self.population.iter().map(|i| i.fitness).sum::<f32>()
                / self.population.len() as f32;
            let v = self
                .population
                .iter()
                .map(|i| (i.fitness - m) * (i.fitness - m))
                .sum::<f32>()
                / self.population.len() as f32;
            (m, v)
        };

        // Diversity: unique param value hashes / population size
        let mut param_hashes: Vec<u64> = Vec::new();
        for ind in &self.population {
            let mut hash_acc: u64 = 0;
            for p in &ind.params {
                hash_acc ^= fnv1a_hash(&p.value.to_le_bytes());
            }
            if !param_hashes.contains(&hash_acc) {
                param_hashes.push(hash_acc);
            }
        }
        let diversity = if self.population.is_empty() {
            0.0
        } else {
            param_hashes.len() as f32 / self.population.len() as f32
        };

        ExplorationProgress {
            generation: self.generation,
            best_fitness: self.stats.current_best_fitness,
            mean_fitness: mean,
            fitness_variance: variance,
            diversity_index: diversity,
            stagnation_count: self.stagnation_count,
            mutation_rate: self.mutation_rate,
            population_size: self.population.len(),
        }
    }

    /// Get aggregate stats
    pub fn stats(&self) -> ExplorerStats {
        self.stats
    }

    /// Get best individual for a domain
    pub fn domain_best(&self, domain: ExplorationDomain) -> Option<&Individual> {
        self.domain_best.get(&(domain as u64))
    }
}
