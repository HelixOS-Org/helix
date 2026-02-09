// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Synthesis — Protocol Generation from Research
//!
//! Synthesizes new cooperation protocols from validated research discoveries.
//! Takes confirmed hypotheses, significant experiment results, and validated
//! safety proofs, then generates concrete protocol parameters ready for
//! deployment. Includes parameter optimization via grid search and gradient-
//! free descent, safe deployment staging with rollback capability, and
//! synthesis reports documenting every decision and its provenance.
//!
//! The engine that turns cooperation research into deployable protocols.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_SYNTHESIZED: usize = 128;
const MAX_PARAMETERS: usize = 32;
const OPTIMIZATION_STEPS: usize = 64;
const GRID_RESOLUTION: usize = 8;
const EMA_ALPHA: f32 = 0.12;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const SAFE_DEPLOYMENT_THRESHOLD: f32 = 0.85;
const ROLLBACK_DEGRADATION: f32 = 0.10;
const OPTIMIZATION_STEP_SIZE: f32 = 0.05;
const MIN_IMPROVEMENT: f32 = 0.001;

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
// SYNTHESIS TYPES
// ============================================================================

/// Phase of a synthesized protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SynthesisPhase {
    Drafting,
    Optimizing,
    ReadyForDeploy,
    Deployed,
    RolledBack,
    Archived,
}

/// A synthesized protocol parameter
#[derive(Debug, Clone)]
pub struct SynthParam {
    pub name: String,
    pub value: f32,
    pub min_bound: f32,
    pub max_bound: f32,
    pub optimized: bool,
    pub source_finding_id: u64,
}

/// A fully synthesized cooperation protocol
#[derive(Debug, Clone)]
pub struct SynthesizedProtocol {
    pub id: u64,
    pub name: String,
    pub phase: SynthesisPhase,
    pub params: Vec<SynthParam>,
    pub created_tick: u64,
    pub deployed_tick: u64,
    pub fairness_estimate: f32,
    pub throughput_estimate: f32,
    pub latency_estimate: f32,
    pub composite_score: f32,
    pub hypothesis_ids: Vec<u64>,
    pub experiment_ids: Vec<u64>,
    pub validation_passed: bool,
}

/// Optimization result for parameter tuning
#[derive(Debug, Clone)]
pub struct OptimizationResult {
    pub protocol_id: u64,
    pub iterations: u32,
    pub initial_score: f32,
    pub final_score: f32,
    pub improvement: f32,
    pub best_params: Vec<(String, f32)>,
}

/// Deployment readiness report
#[derive(Debug, Clone)]
pub struct DeploymentReport {
    pub protocol_id: u64,
    pub safe: bool,
    pub fairness_check: bool,
    pub throughput_check: bool,
    pub latency_check: bool,
    pub composite_score: f32,
    pub recommendation: String,
}

/// Synthesis report documenting decisions and provenance
#[derive(Debug, Clone)]
pub struct SynthesisReport {
    pub protocol_id: u64,
    pub protocol_name: String,
    pub phase: SynthesisPhase,
    pub param_count: usize,
    pub optimized_params: usize,
    pub composite_score: f32,
    pub hypothesis_count: usize,
    pub experiment_count: usize,
    pub summary: String,
}

/// Rollback record
#[derive(Debug, Clone)]
pub struct RollbackRecord {
    pub protocol_id: u64,
    pub reason: String,
    pub tick: u64,
    pub pre_rollback_score: f32,
    pub degradation: f32,
}

// ============================================================================
// SYNTHESIS STATS
// ============================================================================

/// Aggregate synthesis statistics
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct SynthesisStats {
    pub total_synthesized: u64,
    pub total_deployed: u64,
    pub total_rolled_back: u64,
    pub total_optimizations: u64,
    pub avg_composite_ema: f32,
    pub avg_improvement_ema: f32,
    pub deployment_success_rate_ema: f32,
    pub best_composite_ever: f32,
}

// ============================================================================
// COOPERATION SYNTHESIS ENGINE
// ============================================================================

/// Protocol synthesis engine for cooperation research
#[derive(Debug)]
pub struct CoopSynthesis {
    protocols: BTreeMap<u64, SynthesizedProtocol>,
    rollback_log: Vec<RollbackRecord>,
    tick: u64,
    rng_state: u64,
    stats: SynthesisStats,
}

impl CoopSynthesis {
    /// Create a new cooperation synthesis engine
    pub fn new(seed: u64) -> Self {
        Self {
            protocols: BTreeMap::new(),
            rollback_log: Vec::new(),
            tick: 0,
            rng_state: seed | 1,
            stats: SynthesisStats::default(),
        }
    }

    /// Synthesize a new protocol from validated research findings
    pub fn synthesize_protocol(
        &mut self,
        name: String,
        hypothesis_ids: Vec<u64>,
        experiment_ids: Vec<u64>,
        param_names: Vec<String>,
        initial_values: Vec<f32>,
        finding_ids: Vec<u64>,
    ) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes()) ^ xorshift64(&mut self.rng_state);

        let count = param_names.len().min(MAX_PARAMETERS);
        let mut params = Vec::with_capacity(count);
        for i in 0..count {
            let val = if i < initial_values.len() {
                initial_values[i]
            } else {
                xorshift_f32(&mut self.rng_state)
            };
            let finding_id = if i < finding_ids.len() {
                finding_ids[i]
            } else {
                0
            };
            params.push(SynthParam {
                name: if i < param_names.len() {
                    param_names[i].clone()
                } else {
                    let mut s = String::from("synth_p");
                    s.push((b'0' + i as u8) as char);
                    s
                },
                value: val.clamp(0.0, 1.0),
                min_bound: 0.0,
                max_bound: 1.0,
                optimized: false,
                source_finding_id: finding_id,
            });
        }

        // Compute initial composite score estimate
        let fairness_est = params.iter().map(|p| p.value).sum::<f32>() / params.len().max(1) as f32;
        let throughput_est = xorshift_f32(&mut self.rng_state) * 0.3 + 0.5;
        let latency_est = xorshift_f32(&mut self.rng_state) * 0.2 + 0.3;
        let composite = fairness_est * 0.45 + throughput_est * 0.35 + (1.0 - latency_est) * 0.20;

        let proto = SynthesizedProtocol {
            id,
            name,
            phase: SynthesisPhase::Drafting,
            params,
            created_tick: self.tick,
            deployed_tick: 0,
            fairness_estimate: fairness_est,
            throughput_estimate: throughput_est,
            latency_estimate: latency_est,
            composite_score: composite,
            hypothesis_ids,
            experiment_ids,
            validation_passed: false,
        };

        if self.protocols.len() < MAX_SYNTHESIZED {
            self.protocols.insert(id, proto);
            self.stats.total_synthesized += 1;
        }
        self.stats.avg_composite_ema =
            EMA_ALPHA * composite + (1.0 - EMA_ALPHA) * self.stats.avg_composite_ema;

        id
    }

    /// Optimize protocol parameters via grid search + gradient-free descent
    pub fn parameter_optimization(
        &mut self,
        protocol_id: u64,
        objective_weights: (f32, f32, f32),
    ) -> Option<OptimizationResult> {
        let proto = self.protocols.get_mut(&protocol_id)?;
        proto.phase = SynthesisPhase::Optimizing;
        let initial_score = proto.composite_score;
        let (w_fair, w_thru, w_lat) = objective_weights;
        let w_sum = w_fair + w_thru + w_lat;
        let (wf, wt, wl) = if w_sum > 0.0 {
            (w_fair / w_sum, w_thru / w_sum, w_lat / w_sum)
        } else {
            (0.45, 0.35, 0.20)
        };

        let mut best_score = initial_score;
        let mut best_values: Vec<f32> = proto.params.iter().map(|p| p.value).collect();
        let mut iterations: u32 = 0;

        // Grid search phase — coarse exploration
        for param_idx in 0..proto.params.len() {
            let min_b = proto.params[param_idx].min_bound;
            let max_b = proto.params[param_idx].max_bound;
            let step = (max_b - min_b) / GRID_RESOLUTION as f32;
            for g in 0..=GRID_RESOLUTION {
                let candidate = min_b + step * g as f32;
                let old_val = proto.params[param_idx].value;
                proto.params[param_idx].value = candidate;

                let fair_est = proto.params.iter().map(|p| p.value).sum::<f32>()
                    / proto.params.len().max(1) as f32;
                let score = wf * fair_est
                    + wt * proto.throughput_estimate
                    + wl * (1.0 - proto.latency_estimate);

                if score > best_score {
                    best_score = score;
                    best_values = proto.params.iter().map(|p| p.value).collect();
                } else {
                    proto.params[param_idx].value = old_val;
                }
                iterations += 1;
            }
        }

        // Gradient-free refinement — random perturbation descent
        for _ in 0..OPTIMIZATION_STEPS {
            let idx = (xorshift64(&mut self.rng_state) as usize) % proto.params.len().max(1);
            let delta = (xorshift_f32(&mut self.rng_state) - 0.5) * 2.0 * OPTIMIZATION_STEP_SIZE;
            let old_val = proto.params[idx].value;
            let new_val =
                (old_val + delta).clamp(proto.params[idx].min_bound, proto.params[idx].max_bound);
            proto.params[idx].value = new_val;

            let fair_est = proto.params.iter().map(|p| p.value).sum::<f32>()
                / proto.params.len().max(1) as f32;
            let score = wf * fair_est
                + wt * proto.throughput_estimate
                + wl * (1.0 - proto.latency_estimate);

            if score > best_score + MIN_IMPROVEMENT {
                best_score = score;
                best_values = proto.params.iter().map(|p| p.value).collect();
            } else {
                proto.params[idx].value = old_val;
            }
            iterations += 1;
        }

        // Apply best values
        for (i, val) in best_values.iter().enumerate() {
            if i < proto.params.len() {
                proto.params[i].value = *val;
                proto.params[i].optimized = true;
            }
        }
        proto.composite_score = best_score;
        let improvement = best_score - initial_score;
        self.stats.total_optimizations += 1;
        self.stats.avg_improvement_ema =
            EMA_ALPHA * improvement + (1.0 - EMA_ALPHA) * self.stats.avg_improvement_ema;
        if best_score > self.stats.best_composite_ever {
            self.stats.best_composite_ever = best_score;
        }

        let best_params: Vec<(String, f32)> = proto
            .params
            .iter()
            .map(|p| (p.name.clone(), p.value))
            .collect();

        Some(OptimizationResult {
            protocol_id,
            iterations,
            initial_score,
            final_score: best_score,
            improvement,
            best_params,
        })
    }

    /// Check deployment safety and stage for deployment
    pub fn safe_deployment(&mut self, protocol_id: u64) -> Option<DeploymentReport> {
        let proto = self.protocols.get_mut(&protocol_id)?;
        let fairness_ok = proto.fairness_estimate >= SAFE_DEPLOYMENT_THRESHOLD;
        let throughput_ok = proto.throughput_estimate >= 0.5;
        let latency_ok = proto.latency_estimate <= 0.5;
        let safe = fairness_ok
            && throughput_ok
            && latency_ok
            && proto.composite_score >= SAFE_DEPLOYMENT_THRESHOLD;

        let recommendation = if safe {
            proto.phase = SynthesisPhase::ReadyForDeploy;
            String::from("Protocol meets all safety thresholds. Deployment approved.")
        } else if proto.composite_score >= SAFE_DEPLOYMENT_THRESHOLD * 0.9 {
            String::from("Protocol is borderline. Consider additional validation.")
        } else {
            String::from("Protocol does not meet safety thresholds. Deployment denied.")
        };

        Some(DeploymentReport {
            protocol_id,
            safe,
            fairness_check: fairness_ok,
            throughput_check: throughput_ok,
            latency_check: latency_ok,
            composite_score: proto.composite_score,
            recommendation,
        })
    }

    /// Deploy a protocol (mark as deployed)
    #[inline]
    pub fn deploy(&mut self, protocol_id: u64) -> bool {
        self.tick += 1;
        if let Some(proto) = self.protocols.get_mut(&protocol_id) {
            if proto.phase == SynthesisPhase::ReadyForDeploy {
                proto.phase = SynthesisPhase::Deployed;
                proto.deployed_tick = self.tick;
                self.stats.total_deployed += 1;
                let rate =
                    self.stats.total_deployed as f32 / self.stats.total_synthesized.max(1) as f32;
                self.stats.deployment_success_rate_ema =
                    EMA_ALPHA * rate + (1.0 - EMA_ALPHA) * self.stats.deployment_success_rate_ema;
                return true;
            }
        }
        false
    }

    /// Generate a synthesis report documenting all decisions
    pub fn synthesis_report(&self, protocol_id: u64) -> Option<SynthesisReport> {
        let proto = self.protocols.get(&protocol_id)?;
        let optimized_count = proto.params.iter().filter(|p| p.optimized).count();
        let summary = match proto.phase {
            SynthesisPhase::Deployed => {
                String::from("Protocol synthesized, optimized, and deployed successfully.")
            },
            SynthesisPhase::ReadyForDeploy => {
                String::from("Protocol optimized and ready for deployment.")
            },
            SynthesisPhase::Optimizing => {
                String::from("Protocol undergoing parameter optimization.")
            },
            SynthesisPhase::RolledBack => {
                String::from("Protocol was deployed but rolled back due to degradation.")
            },
            SynthesisPhase::Archived => {
                String::from("Protocol archived — superseded by newer synthesis.")
            },
            SynthesisPhase::Drafting => {
                String::from("Protocol in draft stage, pending optimization.")
            },
        };
        Some(SynthesisReport {
            protocol_id,
            protocol_name: proto.name.clone(),
            phase: proto.phase,
            param_count: proto.params.len(),
            optimized_params: optimized_count,
            composite_score: proto.composite_score,
            hypothesis_count: proto.hypothesis_ids.len(),
            experiment_count: proto.experiment_ids.len(),
            summary,
        })
    }

    /// Rollback a deployed protocol due to observed degradation
    pub fn rollback_protocol(
        &mut self,
        protocol_id: u64,
        reason: String,
        observed_degradation: f32,
    ) -> bool {
        self.tick += 1;
        let proto = match self.protocols.get_mut(&protocol_id) {
            Some(p) => p,
            None => return false,
        };
        if proto.phase != SynthesisPhase::Deployed {
            return false;
        }
        let record = RollbackRecord {
            protocol_id,
            reason,
            tick: self.tick,
            pre_rollback_score: proto.composite_score,
            degradation: observed_degradation,
        };
        self.rollback_log.push(record);
        proto.phase = SynthesisPhase::RolledBack;
        self.stats.total_rolled_back += 1;
        true
    }

    /// Get current synthesis engine statistics
    #[inline(always)]
    pub fn stats(&self) -> &SynthesisStats {
        &self.stats
    }
}
