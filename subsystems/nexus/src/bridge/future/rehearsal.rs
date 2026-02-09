// SPDX-License-Identifier: GPL-2.0
//! # Bridge Rehearsal Engine
//!
//! Dry-run rehearsal for syscall handling paths. The rehearsal engine walks
//! through a predicted syscall sequence without committing any side effects,
//! measuring latency at each stage, identifying bottlenecks, and computing
//! counterfactual outcomes for alternative strategies. This lets the bridge
//! choose the optimal path before the real request arrives.
//!
//! Dress rehearsal before opening night — every cycle saved is a gift.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_REHEARSALS: usize = 512;
const MAX_STAGES_PER_PATH: usize = 32;
const MAX_BOTTLENECKS: usize = 64;
const EMA_ALPHA: f32 = 0.10;
const BOTTLENECK_THRESHOLD_NS: u64 = 5_000;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const COUNTERFACTUAL_BRANCHES: usize = 4;

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

// ============================================================================
// REHEARSAL TYPES
// ============================================================================

/// A stage in a syscall handling path
#[derive(Debug, Clone)]
pub struct PathStage {
    pub name: String,
    pub stage_id: u64,
    pub estimated_latency_ns: u64,
    pub resource_cost: f32,
    pub can_fail: bool,
    pub failure_probability: f32,
}

/// A complete syscall handling path for rehearsal
#[derive(Debug, Clone)]
pub struct RehearsalPath {
    pub path_id: u64,
    pub syscall_nr: u32,
    pub stages: Vec<PathStage>,
    pub total_latency_ns: u64,
    pub total_resource_cost: f32,
}

/// Identified bottleneck in a rehearsal path
#[derive(Debug, Clone)]
pub struct Bottleneck {
    pub bottleneck_id: u64,
    pub path_id: u64,
    pub stage_name: String,
    pub stage_index: usize,
    pub latency_ns: u64,
    pub fraction_of_total: f32,
    pub severity: f32,
    pub optimization_potential: f32,
}

/// A counterfactual: "what if we did it differently?"
#[derive(Debug, Clone)]
pub struct Counterfactual {
    pub scenario_id: u64,
    pub original_path_id: u64,
    pub description: String,
    pub modified_latency_ns: u64,
    pub latency_delta_ns: i64,
    pub modified_cost: f32,
    pub improvement_fraction: f32,
}

/// Result of a single rehearsal run
#[derive(Debug, Clone)]
pub struct RehearsalResult {
    pub rehearsal_id: u64,
    pub path: RehearsalPath,
    pub bottlenecks: Vec<Bottleneck>,
    pub counterfactuals: Vec<Counterfactual>,
    pub estimated_benefit_ns: u64,
}

// ============================================================================
// REHEARSAL STATS
// ============================================================================

/// Aggregate rehearsal statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct RehearsalStats {
    pub total_rehearsals: u64,
    pub total_bottlenecks_found: u64,
    pub avg_path_latency_ns: f32,
    pub avg_bottleneck_severity: f32,
    pub total_benefit_ns: u64,
    pub avg_counterfactual_improvement: f32,
    pub rehearsal_cost_ratio: f32,
}

// ============================================================================
// STAGE PERFORMANCE TRACKER
// ============================================================================

/// Tracks historical stage performance for bottleneck analysis
#[derive(Debug, Clone)]
struct StageHistory {
    latency_ema: f32,
    failure_rate_ema: f32,
    observations: u64,
}

impl StageHistory {
    fn new() -> Self {
        Self {
            latency_ema: 0.0,
            failure_rate_ema: 0.0,
            observations: 0,
        }
    }

    fn record(&mut self, latency_ns: u64, failed: bool) {
        self.observations += 1;
        self.latency_ema = EMA_ALPHA * latency_ns as f32 + (1.0 - EMA_ALPHA) * self.latency_ema;
        let f = if failed { 1.0 } else { 0.0 };
        self.failure_rate_ema = EMA_ALPHA * f + (1.0 - EMA_ALPHA) * self.failure_rate_ema;
    }
}

// ============================================================================
// BRIDGE REHEARSAL
// ============================================================================

/// Dry-run rehearsal engine for syscall handling paths. Rehearses paths
/// without committing, identifies bottlenecks, and explores counterfactuals.
#[derive(Debug)]
pub struct BridgeRehearsal {
    rehearsal_history: Vec<RehearsalResult>,
    write_idx: usize,
    stage_history: BTreeMap<u64, StageHistory>,
    known_bottlenecks: BTreeMap<u64, Bottleneck>,
    tick: u64,
    total_rehearsals: u64,
    total_bottlenecks: u64,
    total_benefit: u64,
    avg_latency_ema: f32,
    avg_severity_ema: f32,
    avg_improvement_ema: f32,
    rehearsal_overhead_ema: f32,
    rng_state: u64,
}

impl BridgeRehearsal {
    pub fn new() -> Self {
        Self {
            rehearsal_history: Vec::new(),
            write_idx: 0,
            stage_history: BTreeMap::new(),
            known_bottlenecks: BTreeMap::new(),
            tick: 0,
            total_rehearsals: 0,
            total_bottlenecks: 0,
            total_benefit: 0,
            avg_latency_ema: 0.0,
            avg_severity_ema: 0.0,
            avg_improvement_ema: 0.0,
            rehearsal_overhead_ema: 0.0,
            rng_state: 0xFACE_BEAD_CAFE_D00D,
        }
    }

    /// Rehearse a syscall handling path, returning analysis
    pub fn rehearse_path(&mut self, syscall_nr: u32, stages: Vec<PathStage>) -> RehearsalResult {
        self.tick += 1;
        self.total_rehearsals += 1;

        let path_id = fnv1a_hash(&self.total_rehearsals.to_le_bytes())
            ^ fnv1a_hash(&syscall_nr.to_le_bytes());

        let mut total_latency: u64 = 0;
        let mut total_cost: f32 = 0.0;

        // Walk through stages, accumulating latency and cost
        for stage in &stages {
            total_latency += stage.estimated_latency_ns;
            total_cost += stage.resource_cost;

            let hist = self
                .stage_history
                .entry(stage.stage_id)
                .or_insert_with(StageHistory::new);
            let simulated_fail = stage.failure_probability > 0.5;
            hist.record(stage.estimated_latency_ns, simulated_fail);
        }

        let path = RehearsalPath {
            path_id,
            syscall_nr,
            stages: stages.clone(),
            total_latency_ns: total_latency,
            total_resource_cost: total_cost,
        };

        // Identify bottlenecks
        let bottlenecks = self.find_bottlenecks(&path);
        self.total_bottlenecks += bottlenecks.len() as u64;

        for b in &bottlenecks {
            self.known_bottlenecks.insert(b.bottleneck_id, b.clone());
            self.avg_severity_ema =
                EMA_ALPHA * b.severity + (1.0 - EMA_ALPHA) * self.avg_severity_ema;
        }

        // Trim known bottlenecks
        while self.known_bottlenecks.len() > MAX_BOTTLENECKS {
            let weakest = self
                .known_bottlenecks
                .iter()
                .min_by(|a, b| {
                    a.1.severity
                        .partial_cmp(&b.1.severity)
                        .unwrap_or(core::cmp::Ordering::Equal)
                })
                .map(|(&k, _)| k);
            if let Some(k) = weakest {
                self.known_bottlenecks.remove(&k);
            } else {
                break;
            }
        }

        // Generate counterfactuals
        let counterfactuals = self.generate_counterfactuals(&path, &bottlenecks);
        let benefit: u64 = counterfactuals
            .iter()
            .filter(|c| c.latency_delta_ns < 0)
            .map(|c| (-c.latency_delta_ns) as u64)
            .max()
            .unwrap_or(0);
        self.total_benefit += benefit;

        if !counterfactuals.is_empty() {
            let avg_imp = counterfactuals
                .iter()
                .map(|c| c.improvement_fraction)
                .sum::<f32>()
                / counterfactuals.len() as f32;
            self.avg_improvement_ema =
                EMA_ALPHA * avg_imp + (1.0 - EMA_ALPHA) * self.avg_improvement_ema;
        }

        self.avg_latency_ema =
            EMA_ALPHA * total_latency as f32 + (1.0 - EMA_ALPHA) * self.avg_latency_ema;

        // Estimate rehearsal overhead as fraction of path latency
        let overhead_fraction = 0.02; // Rehearsal costs ~2% of actual execution
        self.rehearsal_overhead_ema =
            EMA_ALPHA * overhead_fraction + (1.0 - EMA_ALPHA) * self.rehearsal_overhead_ema;

        let result = RehearsalResult {
            rehearsal_id: path_id,
            path,
            bottlenecks,
            counterfactuals,
            estimated_benefit_ns: benefit,
        };

        if self.rehearsal_history.len() < MAX_REHEARSALS {
            self.rehearsal_history.push(result.clone());
        } else {
            self.rehearsal_history[self.write_idx] = result.clone();
        }
        self.write_idx = (self.write_idx + 1) % MAX_REHEARSALS;

        result
    }

    /// Identify bottlenecks in a rehearsed path
    pub fn identify_bottleneck(&self, path: &RehearsalPath) -> Vec<Bottleneck> {
        self.find_bottlenecks(path)
    }

    fn find_bottlenecks(&self, path: &RehearsalPath) -> Vec<Bottleneck> {
        let mut bottlenecks = Vec::new();
        let total = path.total_latency_ns.max(1);

        for (i, stage) in path.stages.iter().enumerate() {
            if stage.estimated_latency_ns >= BOTTLENECK_THRESHOLD_NS {
                let fraction = stage.estimated_latency_ns as f32 / total as f32;
                let historical_lat = self
                    .stage_history
                    .get(&stage.stage_id)
                    .map(|h| h.latency_ema)
                    .unwrap_or(stage.estimated_latency_ns as f32);
                let severity = fraction * (1.0 + stage.failure_probability);
                let opt_potential = if historical_lat > stage.estimated_latency_ns as f32 {
                    0.1
                } else {
                    (1.0 - stage.estimated_latency_ns as f32 / historical_lat.max(1.0))
                        .abs()
                        .min(0.9)
                };

                let bid = fnv1a_hash(&stage.stage_id.to_le_bytes())
                    ^ fnv1a_hash(&path.path_id.to_le_bytes());

                bottlenecks.push(Bottleneck {
                    bottleneck_id: bid,
                    path_id: path.path_id,
                    stage_name: stage.name.clone(),
                    stage_index: i,
                    latency_ns: stage.estimated_latency_ns,
                    fraction_of_total: fraction,
                    severity: severity.min(1.0),
                    optimization_potential: opt_potential,
                });
            }
        }
        bottlenecks.sort_by(|a, b| {
            b.severity
                .partial_cmp(&a.severity)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        bottlenecks
    }

    /// Compute the cost of rehearsal relative to actual execution
    pub fn rehearsal_cost(&self) -> f32 {
        self.rehearsal_overhead_ema
    }

    /// Generate counterfactual alternatives for a rehearsed path
    pub fn counterfactual(&mut self, path: &RehearsalPath) -> Vec<Counterfactual> {
        let bottlenecks = self.find_bottlenecks(path);
        self.generate_counterfactuals(path, &bottlenecks)
    }

    fn generate_counterfactuals(
        &mut self,
        path: &RehearsalPath,
        bottlenecks: &[Bottleneck],
    ) -> Vec<Counterfactual> {
        let mut results = Vec::new();
        let original_latency = path.total_latency_ns;

        for (i, bn) in bottlenecks.iter().enumerate().take(COUNTERFACTUAL_BRANCHES) {
            // Scenario: eliminate this bottleneck entirely
            let noise = (xorshift64(&mut self.rng_state) % 20) as f32 / 100.0;
            let reduction = (bn.latency_ns as f32 * (0.5 + noise * 0.5)) as u64;
            let modified = if original_latency > reduction {
                original_latency - reduction
            } else {
                original_latency / 2
            };
            let delta = modified as i64 - original_latency as i64;
            let improvement = if original_latency > 0 {
                (-delta).max(0) as f32 / original_latency as f32
            } else {
                0.0
            };

            let scenario_id = fnv1a_hash(&bn.bottleneck_id.to_le_bytes()) ^ (i as u64);
            results.push(Counterfactual {
                scenario_id,
                original_path_id: path.path_id,
                description: String::from("Remove bottleneck stage"),
                modified_latency_ns: modified,
                latency_delta_ns: delta,
                modified_cost: path.total_resource_cost * (1.0 - bn.fraction_of_total * 0.5),
                improvement_fraction: improvement,
            });
        }
        results
    }

    /// Net benefit of rehearsal: total savings from acted-upon insights
    pub fn rehearsal_benefit(&self) -> (u64, f32) {
        let overhead_cost = (self.avg_latency_ema
            * self.rehearsal_overhead_ema
            * self.total_rehearsals as f32) as u64;
        let net = if self.total_benefit > overhead_cost {
            self.total_benefit - overhead_cost
        } else {
            0
        };
        let ratio = if overhead_cost > 0 {
            self.total_benefit as f32 / overhead_cost as f32
        } else {
            0.0
        };
        (net, ratio)
    }

    /// Aggregate rehearsal statistics
    pub fn stats(&self) -> RehearsalStats {
        let cost_ratio = if self.avg_latency_ema > 0.0 {
            self.rehearsal_overhead_ema
        } else {
            0.0
        };

        RehearsalStats {
            total_rehearsals: self.total_rehearsals,
            total_bottlenecks_found: self.total_bottlenecks,
            avg_path_latency_ns: self.avg_latency_ema,
            avg_bottleneck_severity: self.avg_severity_ema,
            total_benefit_ns: self.total_benefit,
            avg_counterfactual_improvement: self.avg_improvement_ema,
            rehearsal_cost_ratio: cost_ratio,
        }
    }
}
