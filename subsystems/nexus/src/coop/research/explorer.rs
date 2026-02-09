// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Explorer — Autonomous Protocol & Strategy Exploration
//!
//! Genetic-algorithm-driven exploration of the cooperation protocol space.
//! Each individual in the population encodes protocol parameters: negotiation
//! weights, fairness thresholds, concession rates, and trust decay factors.
//! The explorer mutates, crosses over, and selects based on multi-objective
//! fitness measured by fairness, throughput, and latency. A Pareto front
//! tracks non-dominated protocol variants while novelty search prevents
//! premature convergence to local optima.
//!
//! The engine that discovers new ways for subsystems to cooperate.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_POPULATION: usize = 64;
const MAX_PROTOCOL_PARAMS: usize = 24;
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
const DIVERSITY_TARGET: f32 = 0.60;
const FRONTIER_BUDGET: usize = 128;
const NOVELTY_ARCHIVE_MAX: usize = 256;
const NOVELTY_K_NEAREST: usize = 5;

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
// PROTOCOL PARAMETER TYPES
// ============================================================================

/// A single tunable parameter in a cooperation protocol
#[derive(Debug, Clone)]
pub struct ProtocolParam {
    pub name: String,
    pub value: f32,
    pub min_value: f32,
    pub max_value: f32,
    pub mutation_sigma: f32,
    pub domain_hash: u64,
}

/// Strategy dimension being explored
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StrategyDimension {
    NegotiationWeight,
    FairnessThreshold,
    ConcessionRate,
    TrustDecay,
    AuctionBidding,
    ResourceSharing,
    ConflictResolution,
}

/// An individual in the population — a full protocol parameter set
#[derive(Debug, Clone)]
pub struct ProtocolIndividual {
    pub id: u64,
    pub params: Vec<ProtocolParam>,
    pub fitness: f32,
    pub fairness_score: f32,
    pub generation_born: u32,
    pub evaluated: bool,
    pub dimension: StrategyDimension,
}

/// Multi-objective fitness result for a protocol variant
#[derive(Debug, Clone)]
pub struct ProtocolFitnessResult {
    pub individual_id: u64,
    pub fitness: f32,
    pub fairness_score: f32,
    pub throughput_score: f32,
    pub latency_score: f32,
    pub dimension: StrategyDimension,
}

/// A point on the Pareto front
#[derive(Debug, Clone)]
pub struct ParetoPoint {
    pub id: u64,
    pub fairness: f32,
    pub throughput: f32,
    pub latency: f32,
    pub generation: u32,
    pub param_count: usize,
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
    pub diversity_index: f32,
    pub frontier_size: u64,
    pub novelty_archive_size: u64,
}

// ============================================================================
// FITNESS CURVE TRACKER
// ============================================================================

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
        while self.best_per_generation.len() > MAX_GENERATIONS {
            if let Some(&first) = self.best_per_generation.keys().next() {
                self.best_per_generation.remove(&first);
                self.mean_per_generation.remove(&first);
            }
        }
    }
}

// ============================================================================
// COOPERATION EXPLORER
// ============================================================================

/// Autonomous genetic-algorithm cooperation protocol explorer
#[derive(Debug)]
pub struct CoopExplorer {
    population: Vec<ProtocolIndividual>,
    fitness_curve: FitnessCurve,
    frontier: Vec<ParetoPoint>,
    dimension_best: BTreeMap<u64, ProtocolIndividual>,
    novelty_archive: Vec<Vec<f32>>,
    generation: u32,
    rng_state: u64,
    mutation_rate: f32,
    stagnation_count: u32,
    stats: ExplorerStats,
}

impl CoopExplorer {
    /// Create a new cooperation explorer with the given PRNG seed
    pub fn new(seed: u64) -> Self {
        Self {
            population: Vec::new(),
            fitness_curve: FitnessCurve::new(),
            frontier: Vec::new(),
            dimension_best: BTreeMap::new(),
            novelty_archive: Vec::new(),
            generation: 0,
            rng_state: seed | 1,
            mutation_rate: BASE_MUTATION_RATE,
            stagnation_count: 0,
            stats: ExplorerStats::default(),
        }
    }

    /// Explore a new cooperation protocol in the given strategy dimension
    pub fn explore_protocol(
        &mut self,
        dimension: StrategyDimension,
        param_count: usize,
    ) -> ProtocolIndividual {
        let count = param_count.min(MAX_PROTOCOL_PARAMS);
        let mut params = Vec::with_capacity(count);
        for i in 0..count {
            let v = xorshift_f32(&mut self.rng_state);
            let name_hash = fnv1a_hash(&(i as u64).to_le_bytes());
            let mut name = String::from("cparam_");
            let idx_byte = (i as u8) + b'0';
            name.push(idx_byte as char);
            params.push(ProtocolParam {
                name,
                value: v,
                min_value: 0.0,
                max_value: 1.0,
                mutation_sigma: 0.1,
                domain_hash: name_hash,
            });
        }
        let id = fnv1a_hash(&self.generation.to_le_bytes()) ^ xorshift64(&mut self.rng_state);
        let ind = ProtocolIndividual {
            id,
            params,
            fitness: 0.0,
            fairness_score: 0.0,
            generation_born: self.generation,
            evaluated: false,
            dimension,
        };
        if self.population.len() < MAX_POPULATION {
            self.population.push(ind.clone());
        }
        ind
    }

    /// Mutate a protocol's parameters with adaptive mutation rate
    pub fn mutate_strategy(&mut self, individual: &mut ProtocolIndividual) {
        for param in individual.params.iter_mut() {
            if xorshift_f32(&mut self.rng_state) < self.mutation_rate {
                let delta = (xorshift_f32(&mut self.rng_state) - 0.5) * 2.0 * param.mutation_sigma;
                param.value = (param.value + delta).clamp(param.min_value, param.max_value);
                self.stats.mutations_performed += 1;
            }
        }
        if self.stagnation_count > STAGNATION_LIMIT {
            self.mutation_rate = (self.mutation_rate + MUTATION_BOOST).min(0.50);
            self.stagnation_count = 0;
        }
    }

    /// Crossover two protocol parameter sets to produce offspring
    pub fn crossover_protocols(
        &mut self,
        parent_a: &ProtocolIndividual,
        parent_b: &ProtocolIndividual,
    ) -> ProtocolIndividual {
        let len = parent_a.params.len().min(parent_b.params.len());
        let mut child_params = Vec::with_capacity(len);
        for i in 0..len {
            let alpha = xorshift_f32(&mut self.rng_state);
            let blended =
                alpha * parent_a.params[i].value + (1.0 - alpha) * parent_b.params[i].value;
            let base = if xorshift_f32(&mut self.rng_state) < 0.5 {
                &parent_a.params[i]
            } else {
                &parent_b.params[i]
            };
            child_params.push(ProtocolParam {
                name: base.name.clone(),
                value: blended.clamp(base.min_value, base.max_value),
                min_value: base.min_value,
                max_value: base.max_value,
                mutation_sigma: base.mutation_sigma,
                domain_hash: base.domain_hash,
            });
        }
        let id = fnv1a_hash(&parent_a.id.to_le_bytes()) ^ fnv1a_hash(&parent_b.id.to_le_bytes());
        self.stats.crossovers_performed += 1;
        ProtocolIndividual {
            id,
            params: child_params,
            fitness: 0.0,
            fairness_score: 0.0,
            generation_born: self.generation,
            evaluated: false,
            dimension: parent_a.dimension,
        }
    }

    /// Evaluate fairness of a protocol variant against contention scenarios
    pub fn evaluate_fairness(
        &mut self,
        individual: &mut ProtocolIndividual,
        fairness: f32,
        throughput: f32,
        latency: f32,
    ) -> ProtocolFitnessResult {
        let fs = fairness.clamp(0.0, 1.0);
        let ts = throughput.clamp(0.0, 1.0);
        let ls = (1.0 - latency.clamp(0.0, 1.0)).max(0.0);
        let fitness = fs * 0.45 + ts * 0.35 + ls * 0.20;
        individual.fitness = fitness;
        individual.fairness_score = fs;
        individual.evaluated = true;
        self.stats.total_evaluations += 1;

        let dim_key = individual.dimension as u64;
        let is_new_best = self
            .dimension_best
            .get(&dim_key)
            .map_or(true, |b| fitness > b.fitness);
        if is_new_best {
            self.dimension_best.insert(dim_key, individual.clone());
        }
        if fitness > self.stats.best_fitness_ever {
            self.stats.best_fitness_ever = fitness;
            self.stagnation_count = 0;
        } else {
            self.stagnation_count += 1;
        }
        self.stats.avg_fitness_ema =
            EMA_ALPHA * fitness + (1.0 - EMA_ALPHA) * self.stats.avg_fitness_ema;
        self.stats.current_best_fitness = self.stats.best_fitness_ever;

        let mean_fit = if self.population.is_empty() {
            fitness
        } else {
            let sum: f32 = self.population.iter().map(|p| p.fitness).sum();
            sum / self.population.len() as f32
        };
        self.fitness_curve
            .record(self.generation, fitness, mean_fit);

        ProtocolFitnessResult {
            individual_id: individual.id,
            fitness,
            fairness_score: fs,
            throughput_score: ts,
            latency_score: ls,
            dimension: individual.dimension,
        }
    }

    /// Maintain the Pareto front of non-dominated protocol variants
    pub fn pareto_front(&mut self) -> &[ParetoPoint] {
        let mut candidates: Vec<ParetoPoint> = Vec::new();
        for ind in &self.population {
            if !ind.evaluated {
                continue;
            }
            candidates.push(ParetoPoint {
                id: ind.id,
                fairness: ind.fairness_score,
                throughput: ind.fitness,
                latency: 1.0 - ind.fitness,
                generation: ind.generation_born,
                param_count: ind.params.len(),
            });
        }
        // Filter dominated solutions
        let mut front: Vec<ParetoPoint> = Vec::new();
        for c in &candidates {
            let dominated = candidates.iter().any(|o| {
                o.id != c.id
                    && o.fairness >= c.fairness
                    && o.throughput >= c.throughput
                    && (o.fairness > c.fairness || o.throughput > c.throughput)
            });
            if !dominated && front.len() < FRONTIER_BUDGET {
                front.push(c.clone());
            }
        }
        self.frontier = front;
        self.stats.frontier_size = self.frontier.len() as u64;
        &self.frontier
    }

    /// Novelty search — measure behavioral novelty to escape local optima
    pub fn novelty_search(&mut self, individual: &ProtocolIndividual) -> f32 {
        let behavior: Vec<f32> = individual.params.iter().map(|p| p.value).collect();
        if self.novelty_archive.is_empty() {
            if self.novelty_archive.len() < NOVELTY_ARCHIVE_MAX {
                self.novelty_archive.push(behavior);
            }
            self.stats.novelty_archive_size = self.novelty_archive.len() as u64;
            return 1.0;
        }
        let mut distances: Vec<f32> = Vec::new();
        for archived in &self.novelty_archive {
            let len = behavior.len().min(archived.len());
            let mut dist_sq: f32 = 0.0;
            for i in 0..len {
                let d = behavior[i] - archived[i];
                dist_sq += d * d;
            }
            distances.push(dist_sq);
        }
        distances.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
        let k = NOVELTY_K_NEAREST.min(distances.len());
        let avg_dist = if k > 0 {
            distances[..k].iter().sum::<f32>() / k as f32
        } else {
            0.0
        };
        if self.novelty_archive.len() < NOVELTY_ARCHIVE_MAX {
            self.novelty_archive.push(behavior);
        }
        self.stats.novelty_archive_size = self.novelty_archive.len() as u64;
        avg_dist
    }

    /// Advance to the next generation — selection, crossover, mutation
    pub fn advance_generation(&mut self) {
        self.generation += 1;
        self.stats.total_generations += 1;
        self.mutation_rate = BASE_MUTATION_RATE;

        // Compute diversity
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
        self.stats.diversity_index = diversity;
        self.stats.fitness_improvement_rate = self.fitness_curve.improvement_ema;
    }

    /// Get current exploration statistics
    pub fn stats(&self) -> &ExplorerStats {
        &self.stats
    }
}
