// SPDX-License-Identifier: GPL-2.0
//! # Holistic Explorer — System-Wide Algorithmic Exploration
//!
//! Coordinates autonomous research exploration across every NEXUS subsystem:
//! bridge, application, and cooperation layers. Rather than optimising a
//! single protocol dimension, the Holistic Explorer searches for cross-
//! subsystem synergies, emergent system properties, and novel kernel
//! configurations that no isolated research engine would discover.
//!
//! Multi-objective genetic algorithms drive the search. Each individual in
//! the population encodes a *system-wide configuration vector* — scheduler
//! weights, memory policies, IPC strategies, trust parameters — and is
//! evaluated against throughput, latency, fairness, and energy objectives
//! simultaneously. A Pareto front tracks the best non-dominated system
//! states while novelty search prevents convergence to local optima.
//!
//! The engine that explores the entire kernel possibility space.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_POPULATION: usize = 128;
const MAX_OBJECTIVES: usize = 8;
const MAX_DIMENSIONS: usize = 48;
const MAX_GENERATIONS: u64 = 16_384;
const ELITISM_FRACTION: f32 = 0.12;
const CROSSOVER_RATE: f32 = 0.72;
const BASE_MUTATION_RATE: f32 = 0.06;
const MUTATION_BOOST: f32 = 0.22;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const STAGNATION_LIMIT: u32 = 30;
const PARETO_BUDGET: usize = 256;
const NOVELTY_K: usize = 6;
const SYNERGY_THRESHOLD: f32 = 0.15;
const EMERGENCE_SCAN_WINDOW: usize = 64;
const FRONTIER_CAPACITY: usize = 512;

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
// TYPES
// ============================================================================

/// Subsystem being explored
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SubsystemAxis {
    Bridge,
    Application,
    Cooperation,
    Memory,
    Scheduler,
    Ipc,
    Trust,
    Energy,
}

/// A single dimension in the global configuration vector
#[derive(Debug, Clone)]
pub struct ConfigDimension {
    pub name: String,
    pub axis: SubsystemAxis,
    pub value: f32,
    pub min_val: f32,
    pub max_val: f32,
    pub sensitivity: f32,
    pub hash: u64,
}

/// An objective being optimised (throughput, latency, fairness …)
#[derive(Debug, Clone)]
pub struct Objective {
    pub name: String,
    pub weight: f32,
    pub measured: f32,
    pub target: f32,
    pub is_minimize: bool,
}

/// One individual in the evolutionary population
#[derive(Debug, Clone)]
pub struct SystemIndividual {
    pub id: u64,
    pub dimensions: Vec<ConfigDimension>,
    pub objectives: Vec<Objective>,
    pub fitness: f32,
    pub crowding_distance: f32,
    pub rank: u32,
    pub generation_born: u64,
    pub evaluated: bool,
}

/// A point on the Pareto front
#[derive(Debug, Clone)]
pub struct ParetoPoint {
    pub id: u64,
    pub scores: Vec<f32>,
    pub generation: u64,
    pub dimension_count: usize,
}

/// Cross-subsystem synergy detection result
#[derive(Debug, Clone)]
pub struct SynergyResult {
    pub axis_a: SubsystemAxis,
    pub axis_b: SubsystemAxis,
    pub correlation: f32,
    pub synergy_score: f32,
    pub tick: u64,
}

/// Emergent property detected at the system level
#[derive(Debug, Clone)]
pub struct EmergentProperty {
    pub id: u64,
    pub description: String,
    pub involved_axes: Vec<SubsystemAxis>,
    pub strength: f32,
    pub first_seen: u64,
    pub occurrences: u64,
}

/// Exploration statistics
#[derive(Debug, Clone)]
pub struct ExplorerStats {
    pub generation: u64,
    pub population_size: u64,
    pub best_fitness: f32,
    pub avg_fitness_ema: f32,
    pub pareto_size: u64,
    pub stagnation_count: u32,
    pub mutation_rate: f32,
    pub synergies_found: u64,
    pub emergent_properties: u64,
    pub frontier_size: u64,
    pub novelty_archive_size: u64,
    pub total_evaluations: u64,
}

// ============================================================================
// HOLISTIC EXPLORER
// ============================================================================

/// System-wide algorithmic exploration engine
pub struct HolisticExplorer {
    population: Vec<SystemIndividual>,
    pareto_front: Vec<ParetoPoint>,
    synergies: BTreeMap<u64, SynergyResult>,
    emergent: BTreeMap<u64, EmergentProperty>,
    novelty_archive: Vec<Vec<f32>>,
    frontier_log: Vec<(u64, f32)>,
    rng_state: u64,
    stats: ExplorerStats,
}

impl HolisticExplorer {
    /// Create a new holistic explorer
    pub fn new(seed: u64) -> Self {
        Self {
            population: Vec::new(),
            pareto_front: Vec::new(),
            synergies: BTreeMap::new(),
            emergent: BTreeMap::new(),
            novelty_archive: Vec::new(),
            frontier_log: Vec::new(),
            rng_state: seed | 1,
            stats: ExplorerStats {
                generation: 0, population_size: 0, best_fitness: 0.0,
                avg_fitness_ema: 0.0, pareto_size: 0, stagnation_count: 0,
                mutation_rate: BASE_MUTATION_RATE, synergies_found: 0,
                emergent_properties: 0, frontier_size: 0,
                novelty_archive_size: 0, total_evaluations: 0,
            },
        }
    }

    /// Full system-wide exploration tick
    pub fn global_exploration(&mut self, tick: u64) -> &ExplorerStats {
        self.stats.generation = tick;
        if self.population.is_empty() {
            self.seed_population();
        }
        self.evaluate_all(tick);
        self.non_dominated_sort();
        self.crowding_distance_assignment();
        self.select_and_breed();
        self.mutate_population(tick);
        if self.stats.stagnation_count > STAGNATION_LIMIT {
            self.stats.mutation_rate =
                (self.stats.mutation_rate + MUTATION_BOOST).min(0.50);
            self.stats.stagnation_count = 0;
        }
        self.stats.population_size = self.population.len() as u64;
        &self.stats
    }

    /// Multi-objective evolutionary step (NSGA-II style)
    pub fn multi_objective_evolve(&mut self, tick: u64) -> usize {
        let prev_best = self.stats.best_fitness;
        self.evaluate_all(tick);
        self.non_dominated_sort();
        self.crowding_distance_assignment();
        let children = self.crossover_pass();
        self.select_survivors();
        let new_best = self.stats.best_fitness;
        if (new_best - prev_best).abs() < 1e-6 {
            self.stats.stagnation_count += 1;
        } else {
            self.stats.stagnation_count = 0;
        }
        children
    }

    /// Detect cross-subsystem synergies
    pub fn cross_subsystem_synergy(&mut self, tick: u64) -> Vec<SynergyResult> {
        let mut found = Vec::new();
        let axes = [
            SubsystemAxis::Bridge, SubsystemAxis::Application,
            SubsystemAxis::Cooperation, SubsystemAxis::Memory,
            SubsystemAxis::Scheduler, SubsystemAxis::Ipc,
        ];
        for i in 0..axes.len() {
            for j in (i + 1)..axes.len() {
                let corr = self.axis_correlation(axes[i], axes[j]);
                if corr.abs() > SYNERGY_THRESHOLD {
                    let key = fnv1a_hash(&[i as u8, j as u8]);
                    let syn = SynergyResult {
                        axis_a: axes[i], axis_b: axes[j],
                        correlation: corr,
                        synergy_score: corr.abs() * 1.5,
                        tick,
                    };
                    found.push(syn.clone());
                    self.synergies.insert(key, syn);
                    self.stats.synergies_found = self.synergies.len() as u64;
                }
            }
        }
        found
    }

    /// Search for emergent system-level properties
    pub fn emergent_pattern_search(&mut self, tick: u64) -> Vec<EmergentProperty> {
        let mut detected = Vec::new();
        let window = self.frontier_log.len().min(EMERGENCE_SCAN_WINDOW);
        if window < 4 { return detected; }
        let recent = &self.frontier_log[self.frontier_log.len() - window..];
        let mean: f32 = recent.iter().map(|(_, v)| *v).sum::<f32>() / window as f32;
        let variance: f32 = recent.iter()
            .map(|(_, v)| (*v - mean) * (*v - mean)).sum::<f32>() / window as f32;
        if variance > 0.05 {
            let id = fnv1a_hash(&tick.to_le_bytes());
            let prop = EmergentProperty {
                id,
                description: String::from("fitness_oscillation_detected"),
                involved_axes: Vec::new(),
                strength: variance,
                first_seen: tick,
                occurrences: 1,
            };
            if let Some(existing) = self.emergent.get_mut(&id) {
                existing.occurrences += 1;
            } else {
                self.emergent.insert(id, prop.clone());
            }
            detected.push(prop);
            self.stats.emergent_properties = self.emergent.len() as u64;
        }
        detected
    }

    /// Compute the Pareto-optimal set across all objectives
    pub fn pareto_optimal_set(&mut self) -> &[ParetoPoint] {
        let mut candidates: Vec<ParetoPoint> = Vec::new();
        for ind in &self.population {
            if !ind.evaluated { continue; }
            let scores: Vec<f32> = ind.objectives.iter().map(|o| o.measured).collect();
            candidates.push(ParetoPoint {
                id: ind.id, scores, generation: ind.generation_born,
                dimension_count: ind.dimensions.len(),
            });
        }
        let mut front: Vec<ParetoPoint> = Vec::new();
        for c in &candidates {
            let dominated = candidates.iter().any(|o| {
                o.id != c.id
                    && o.scores.iter().zip(c.scores.iter()).all(|(a, b)| a >= b)
                    && o.scores.iter().zip(c.scores.iter()).any(|(a, b)| a > b)
            });
            if !dominated && front.len() < PARETO_BUDGET {
                front.push(c.clone());
            }
        }
        self.pareto_front = front;
        self.stats.pareto_size = self.pareto_front.len() as u64;
        &self.pareto_front
    }

    /// Return the current exploration frontier log
    pub fn exploration_frontier(&self) -> &[(u64, f32)] {
        let limit = self.frontier_log.len().min(FRONTIER_CAPACITY);
        &self.frontier_log[self.frontier_log.len() - limit..]
    }

    /// Current statistics snapshot
    pub fn stats(&self) -> &ExplorerStats { &self.stats }

    // ── private helpers ─────────────────────────────────────────────────

    fn seed_population(&mut self) {
        for i in 0..MAX_POPULATION {
            let id = fnv1a_hash(&(i as u64).to_le_bytes());
            let dim_count = (xorshift64(&mut self.rng_state) % MAX_DIMENSIONS as u64).max(4) as usize;
            let dims: Vec<ConfigDimension> = (0..dim_count).map(|d| {
                let val = xorshift_f32(&mut self.rng_state);
                ConfigDimension {
                    name: String::from("dim"), axis: SubsystemAxis::Bridge,
                    value: val, min_val: 0.0, max_val: 1.0, sensitivity: 0.5,
                    hash: fnv1a_hash(&(d as u64).to_le_bytes()),
                }
            }).collect();
            self.population.push(SystemIndividual {
                id, dimensions: dims, objectives: Vec::new(),
                fitness: 0.0, crowding_distance: 0.0, rank: 0,
                generation_born: 0, evaluated: false,
            });
        }
    }

    fn evaluate_all(&mut self, tick: u64) {
        let mut best = self.stats.best_fitness;
        for ind in self.population.iter_mut() {
            if ind.evaluated { continue; }
            let fit: f32 = ind.dimensions.iter().map(|d| d.value * d.sensitivity)
                .sum::<f32>() / ind.dimensions.len().max(1) as f32;
            ind.fitness = fit;
            ind.evaluated = true;
            if fit > best { best = fit; }
            self.stats.total_evaluations += 1;
        }
        self.stats.avg_fitness_ema =
            EMA_ALPHA * best + (1.0 - EMA_ALPHA) * self.stats.avg_fitness_ema;
        self.stats.best_fitness = best;
        self.frontier_log.push((tick, best));
    }

    fn non_dominated_sort(&mut self) {
        let len = self.population.len();
        for i in 0..len {
            let fi = self.population[i].fitness;
            let rank = (0..len).filter(|&j| j != i && self.population[j].fitness > fi)
                .count() as u32;
            self.population[i].rank = rank;
        }
    }

    fn crowding_distance_assignment(&mut self) {
        let len = self.population.len();
        if len < 3 { return; }
        for ind in self.population.iter_mut() { ind.crowding_distance = 0.0; }
        let mut idx: Vec<usize> = (0..len).collect();
        idx.sort_by(|&a, &b| self.population[a].fitness
            .partial_cmp(&self.population[b].fitness).unwrap_or(core::cmp::Ordering::Equal));
        self.population[idx[0]].crowding_distance = f32::MAX;
        self.population[idx[len - 1]].crowding_distance = f32::MAX;
        let range = self.population[idx[len - 1]].fitness - self.population[idx[0]].fitness;
        if range < 1e-9 { return; }
        for k in 1..(len - 1) {
            let d = (self.population[idx[k + 1]].fitness
                - self.population[idx[k - 1]].fitness) / range;
            self.population[idx[k]].crowding_distance += d;
        }
    }

    fn select_and_breed(&mut self) {
        self.population.sort_by(|a, b| a.rank.cmp(&b.rank).then(
            b.crowding_distance.partial_cmp(&a.crowding_distance)
                .unwrap_or(core::cmp::Ordering::Equal)));
        self.population.truncate(((self.population.len() as f32 * ELITISM_FRACTION) as usize).max(2));
    }

    fn crossover_pass(&mut self) -> usize {
        let mut children = 0;
        let parents = self.population.clone();
        let len = parents.len();
        if len < 2 { return 0; }
        while self.population.len() < MAX_POPULATION {
            let a = (xorshift64(&mut self.rng_state) % len as u64) as usize;
            let b = (xorshift64(&mut self.rng_state) % len as u64) as usize;
            if a == b || xorshift_f32(&mut self.rng_state) > CROSSOVER_RATE { continue; }
            let id = fnv1a_hash(&self.stats.total_evaluations.to_le_bytes());
            let dl = parents[a].dimensions.len().min(parents[b].dimensions.len());
            let dims: Vec<ConfigDimension> = (0..dl).map(|d| {
                let mut dim = parents[a].dimensions[d].clone();
                if xorshift_f32(&mut self.rng_state) >= 0.5 {
                    dim.value = parents[b].dimensions[d].value;
                }
                dim
            }).collect();
            self.population.push(SystemIndividual {
                id, dimensions: dims, objectives: Vec::new(),
                fitness: 0.0, crowding_distance: 0.0, rank: 0,
                generation_born: self.stats.generation, evaluated: false,
            });
            children += 1;
        }
        children
    }

    fn select_survivors(&mut self) {
        if self.population.len() > MAX_POPULATION {
            self.population.sort_by(|a, b|
                b.fitness.partial_cmp(&a.fitness).unwrap_or(core::cmp::Ordering::Equal));
            self.population.truncate(MAX_POPULATION);
        }
    }

    fn mutate_population(&mut self, _tick: u64) {
        let rate = self.stats.mutation_rate;
        for ind in self.population.iter_mut() {
            for dim in ind.dimensions.iter_mut() {
                if xorshift_f32(&mut self.rng_state) < rate {
                    let delta = (xorshift_f32(&mut self.rng_state) - 0.5) * 0.2;
                    dim.value = (dim.value + delta).clamp(dim.min_val, dim.max_val);
                    ind.evaluated = false;
                }
            }
        }
    }

    fn axis_correlation(&self, a: SubsystemAxis, b: SubsystemAxis) -> f32 {
        let (mut sa, mut sb, mut sab, mut n) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        for ind in &self.population {
            let va: f32 = ind.dimensions.iter().filter(|d| d.axis == a).map(|d| d.value).sum();
            let vb: f32 = ind.dimensions.iter().filter(|d| d.axis == b).map(|d| d.value).sum();
            sa += va; sb += vb; sab += va * vb; n += 1.0;
        }
        if n < 2.0 { return 0.0; }
        (sab / n) - (sa / n) * (sb / n)
    }
}
