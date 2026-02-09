// SPDX-License-Identifier: GPL-2.0
//! # Apps Explorer — Autonomous Feature & Classification Exploration
//!
//! Genetic-algorithm-driven exploration of the application classification
//! space. Each individual in the population encodes a feature set, classifier
//! weights, and optimization strategy parameters. The explorer mutates,
//! crosses over, and selects based on fitness measured against real workload
//! telemetry. Diversity maintenance prevents premature convergence and
//! ensures the frontier of app understanding keeps expanding.
//!
//! The engine that discovers new ways to understand applications.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_POPULATION: usize = 64;
const MAX_FEATURES: usize = 32;
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
// FEATURE & CLASSIFIER TYPES
// ============================================================================

/// A single classification feature being explored
#[derive(Debug, Clone)]
pub struct ClassificationFeature {
    pub name: String,
    pub weight: f32,
    pub min_weight: f32,
    pub max_weight: f32,
    pub mutation_sigma: f32,
    pub domain_hash: u64,
}

/// Exploration dimension for app classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExplorationDimension {
    IoPattern,
    CpuProfile,
    MemoryBehavior,
    NetworkUsage,
    SyscallSignature,
    ResourcePrediction,
    OptimizationStrategy,
}

/// An individual in the population — a complete feature set + classifier
#[derive(Debug, Clone)]
pub struct FeatureIndividual {
    pub id: u64,
    pub features: Vec<ClassificationFeature>,
    pub fitness: f32,
    pub accuracy: f32,
    pub generation_born: u32,
    pub evaluated: bool,
    pub dimension: ExplorationDimension,
}

/// Fitness evaluation result for a feature set
#[derive(Debug, Clone)]
pub struct FeatureFitnessResult {
    pub individual_id: u64,
    pub fitness: f32,
    pub accuracy_score: f32,
    pub coverage_score: f32,
    pub efficiency_score: f32,
    pub dimension: ExplorationDimension,
}

/// Exploration frontier point
#[derive(Debug, Clone)]
pub struct FrontierPoint {
    pub id: u64,
    pub accuracy: f32,
    pub coverage: f32,
    pub efficiency: f32,
    pub generation: u32,
    pub feature_count: usize,
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

        while self.best_per_generation.len() > MAX_GENERATIONS {
            if let Some(&first) = self.best_per_generation.keys().next() {
                self.best_per_generation.remove(&first);
                self.mean_per_generation.remove(&first);
            }
        }
    }
}

// ============================================================================
// APPS EXPLORER
// ============================================================================

/// Autonomous genetic-algorithm app classification explorer
#[derive(Debug)]
pub struct AppsExplorer {
    population: Vec<FeatureIndividual>,
    fitness_curve: FitnessCurve,
    frontier: Vec<FrontierPoint>,
    dimension_best: BTreeMap<u64, FeatureIndividual>,
    generation: u32,
    rng_state: u64,
    mutation_rate: f32,
    stagnation_count: u32,
    stats: ExplorerStats,
}

impl AppsExplorer {
    /// Create a new explorer with a seed
    pub fn new(seed: u64) -> Self {
        Self {
            population: Vec::new(),
            fitness_curve: FitnessCurve::new(),
            frontier: Vec::new(),
            dimension_best: BTreeMap::new(),
            generation: 0,
            rng_state: seed | 1,
            mutation_rate: BASE_MUTATION_RATE,
            stagnation_count: 0,
            stats: ExplorerStats::default(),
        }
    }

    /// Explore new classification features in the given dimension
    pub fn explore_features(
        &mut self,
        dimension: ExplorationDimension,
        feature_count: usize,
    ) -> FeatureIndividual {
        let count = feature_count.min(MAX_FEATURES);
        let mut features = Vec::with_capacity(count);
        for i in 0..count {
            let w = xorshift_f32(&mut self.rng_state);
            let name_hash = fnv1a_hash(&(i as u64).to_le_bytes());
            let mut name = String::from("feat_");
            let idx_byte = (i as u8) + b'0';
            name.push(idx_byte as char);
            features.push(ClassificationFeature {
                name,
                weight: w,
                min_weight: 0.0,
                max_weight: 1.0,
                mutation_sigma: 0.1,
                domain_hash: name_hash,
            });
        }
        let id = fnv1a_hash(&self.generation.to_le_bytes()) ^ xorshift64(&mut self.rng_state);
        let ind = FeatureIndividual {
            id,
            features,
            fitness: 0.0,
            accuracy: 0.0,
            generation_born: self.generation,
            evaluated: false,
            dimension,
        };
        if self.population.len() < MAX_POPULATION {
            self.population.push(ind.clone());
        }
        ind
    }

    /// Mutate a classifier's feature weights with adaptive mutation rate
    pub fn mutate_classifier(&mut self, individual: &mut FeatureIndividual) {
        for feature in individual.features.iter_mut() {
            if xorshift_f32(&mut self.rng_state) < self.mutation_rate {
                let delta =
                    (xorshift_f32(&mut self.rng_state) - 0.5) * 2.0 * feature.mutation_sigma;
                feature.weight =
                    (feature.weight + delta).clamp(feature.min_weight, feature.max_weight);
                self.stats.mutations_performed += 1;
            }
        }
    }

    /// Crossover two feature sets to produce an offspring
    pub fn crossover_features(
        &mut self,
        parent_a: &FeatureIndividual,
        parent_b: &FeatureIndividual,
    ) -> FeatureIndividual {
        let len = parent_a.features.len().min(parent_b.features.len());
        let mut child_features = Vec::with_capacity(len);
        for i in 0..len {
            let alpha = xorshift_f32(&mut self.rng_state);
            let blended = alpha * parent_a.features[i].weight
                + (1.0 - alpha) * parent_b.features[i].weight;
            let base = if xorshift_f32(&mut self.rng_state) < 0.5 {
                &parent_a.features[i]
            } else {
                &parent_b.features[i]
            };
            child_features.push(ClassificationFeature {
                name: base.name.clone(),
                weight: blended.clamp(base.min_weight, base.max_weight),
                min_weight: base.min_weight,
                max_weight: base.max_weight,
                mutation_sigma: base.mutation_sigma,
                domain_hash: base.domain_hash,
            });
        }
        let id = fnv1a_hash(&parent_a.id.to_le_bytes()) ^ fnv1a_hash(&parent_b.id.to_le_bytes());
        self.stats.crossovers_performed += 1;
        FeatureIndividual {
            id,
            features: child_features,
            fitness: 0.0,
            accuracy: 0.0,
            generation_born: self.generation,
            evaluated: false,
            dimension: parent_a.dimension,
        }
    }

    /// Evaluate fitness of a feature set against workload telemetry
    pub fn fitness_test(
        &mut self,
        individual: &mut FeatureIndividual,
        accuracy: f32,
        coverage: f32,
        efficiency: f32,
    ) -> FeatureFitnessResult {
        let accuracy_score = accuracy.clamp(0.0, 1.0);
        let coverage_score = coverage.clamp(0.0, 1.0);
        let efficiency_score = efficiency.clamp(0.0, 1.0);
        let fitness = accuracy_score * 0.50 + coverage_score * 0.30 + efficiency_score * 0.20;
        individual.fitness = fitness;
        individual.accuracy = accuracy_score;
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
        }
        self.stats.avg_fitness_ema =
            EMA_ALPHA * fitness + (1.0 - EMA_ALPHA) * self.stats.avg_fitness_ema;

        FeatureFitnessResult {
            individual_id: individual.id,
            fitness,
            accuracy_score,
            coverage_score,
            efficiency_score,
            dimension: individual.dimension,
        }
    }

    /// Maintain population diversity — inject random individuals if diversity drops
    pub fn diversity_maintain(&mut self) {
        if self.population.is_empty() {
            return;
        }
        let mut param_hashes: Vec<u64> = Vec::new();
        for ind in &self.population {
            let mut hash_acc: u64 = 0;
            for f in &ind.features {
                hash_acc ^= fnv1a_hash(&f.weight.to_le_bytes());
            }
            if !param_hashes.contains(&hash_acc) {
                param_hashes.push(hash_acc);
            }
        }
        let diversity = param_hashes.len() as f32 / self.population.len() as f32;
        self.stats.diversity_index = diversity;

        if diversity < DIVERSITY_TARGET {
            let inject_count = (self.population.len() / 8).max(1);
            for _ in 0..inject_count {
                if self.population.len() >= MAX_POPULATION {
                    break;
                }
                let dim = match xorshift64(&mut self.rng_state) % 7 {
                    0 => ExplorationDimension::IoPattern,
                    1 => ExplorationDimension::CpuProfile,
                    2 => ExplorationDimension::MemoryBehavior,
                    3 => ExplorationDimension::NetworkUsage,
                    4 => ExplorationDimension::SyscallSignature,
                    5 => ExplorationDimension::ResourcePrediction,
                    _ => ExplorationDimension::OptimizationStrategy,
                };
                let feat_count = (xorshift64(&mut self.rng_state) % 8 + 4) as usize;
                self.explore_features(dim, feat_count);
            }
        }

        // Elitism + tournament selection
        self.population
            .sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap_or(core::cmp::Ordering::Equal));
        let elite_count = ((self.population.len() as f32 * ELITISM_FRACTION) as usize).max(1);
        let prev_best = self.stats.current_best_fitness;
        self.stats.current_best_fitness = self.population.first().map_or(0.0, |i| i.fitness);

        let improved = self.stats.current_best_fitness > prev_best + 0.001;
        if improved {
            self.stagnation_count = 0;
            self.mutation_rate = BASE_MUTATION_RATE;
        } else {
            self.stagnation_count += 1;
            if self.stagnation_count > STAGNATION_LIMIT {
                self.mutation_rate = (self.mutation_rate + MUTATION_BOOST).min(0.5);
                self.stagnation_count = 0;
            }
        }

        let mean = if self.population.is_empty() {
            0.0
        } else {
            self.population.iter().map(|i| i.fitness).sum::<f32>()
                / self.population.len() as f32
        };
        self.fitness_curve
            .record(self.generation, self.stats.current_best_fitness, mean);

        let elites: Vec<FeatureIndividual> = self.population[..elite_count].to_vec();
        let mut new_pop = elites;
        let target = MAX_POPULATION.min(self.population.len());
        while new_pop.len() < target {
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
                let mut child = self.crossover_features(&parent_a, &parent_b);
                self.mutate_classifier(&mut child);
                new_pop.push(child);
            } else {
                let mut clone = parent_a;
                self.mutate_classifier(&mut clone);
                new_pop.push(clone);
            }
        }
        self.population = new_pop;
        self.generation += 1;
        self.stats.total_generations += 1;
        self.stats.fitness_improvement_rate = self.fitness_curve.improvement_ema;
    }

    /// Expand the Pareto frontier with new non-dominated points
    pub fn frontier_expand(&mut self) {
        for ind in &self.population {
            if !ind.evaluated {
                continue;
            }
            let point = FrontierPoint {
                id: ind.id,
                accuracy: ind.accuracy,
                coverage: ind.fitness - ind.accuracy * 0.50,
                efficiency: ind.fitness,
                generation: ind.generation_born,
                feature_count: ind.features.len(),
            };
            let dominated = self.frontier.iter().any(|fp| {
                fp.accuracy >= point.accuracy
                    && fp.coverage >= point.coverage
                    && fp.efficiency >= point.efficiency
                    && (fp.accuracy > point.accuracy
                        || fp.coverage > point.coverage
                        || fp.efficiency > point.efficiency)
            });
            if !dominated {
                self.frontier
                    .retain(|fp| {
                        !(point.accuracy >= fp.accuracy
                            && point.coverage >= fp.coverage
                            && point.efficiency >= fp.efficiency)
                    });
                self.frontier.push(point);
                if self.frontier.len() > FRONTIER_BUDGET {
                    self.frontier.remove(0);
                }
            }
        }
        self.stats.frontier_size = self.frontier.len() as u64;
    }

    /// Get aggregate stats
    pub fn stats(&self) -> ExplorerStats {
        self.stats
    }

    /// Get best individual for a dimension
    pub fn dimension_best(&self, dim: ExplorationDimension) -> Option<&FeatureIndividual> {
        self.dimension_best.get(&(dim as u64))
    }

    /// Get frontier snapshot
    pub fn frontier(&self) -> &[FrontierPoint] {
        &self.frontier
    }
}
