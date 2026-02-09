// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Experiment — Controlled Protocol Testing
//!
//! Designs and executes controlled experiments on cooperation protocols.
//! Tests new negotiation algorithms, fairness metrics, and trust models in
//! sandboxed environments where we can measure outcomes precisely. Uses
//! Welch's t-test for significance, Cohen's d for effect size, and
//! statistical power analysis to determine when results are meaningful.
//! Every experiment is recorded with full provenance for reproducibility.
//!
//! The engine that rigorously tests every cooperation idea.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_EXPERIMENTS: usize = 256;
const MAX_SAMPLES_PER_ARM: usize = 512;
const MIN_SAMPLES_FOR_ANALYSIS: usize = 8;
const SIGNIFICANCE_LEVEL: f32 = 0.05;
const MIN_EFFECT_SIZE: f32 = 0.20;
const EMA_ALPHA: f32 = 0.12;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const POWER_TARGET: f32 = 0.80;
const MAX_EXPERIMENT_TICKS: u64 = 100_000;

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
// EXPERIMENT TYPES
// ============================================================================

/// Phase of a controlled experiment
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExperimentPhase {
    Designed,
    Running,
    Collecting,
    Analyzing,
    Concluded,
    Invalidated,
}

/// What aspect of cooperation is being tested
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoopTestDomain {
    NegotiationAlgorithm,
    FairnessMetric,
    TrustModel,
    AuctionMechanism,
    ConflictResolution,
    ResourceSharing,
    BackoffStrategy,
}

/// A single observation in an experiment arm
#[derive(Debug, Clone)]
pub struct Observation {
    pub tick: u64,
    pub value: f32,
    pub arm_id: u64,
    pub metadata_hash: u64,
}

/// One arm of an A/B experiment (control or treatment)
#[derive(Debug, Clone)]
pub struct ExperimentArm {
    pub id: u64,
    pub name: String,
    pub is_control: bool,
    pub observations: Vec<Observation>,
    pub mean: f32,
    pub variance: f32,
    pub count: usize,
}

/// Full experiment definition and results
#[derive(Debug, Clone)]
pub struct CoopExperimentRecord {
    pub id: u64,
    pub name: String,
    pub domain: CoopTestDomain,
    pub phase: ExperimentPhase,
    pub control: ExperimentArm,
    pub treatment: ExperimentArm,
    pub hypothesis_id: u64,
    pub created_tick: u64,
    pub concluded_tick: u64,
    pub t_statistic: f32,
    pub p_value: f32,
    pub effect_size: f32,
    pub significant: bool,
}

/// Statistical analysis result
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub experiment_id: u64,
    pub t_statistic: f32,
    pub p_value_approx: f32,
    pub cohens_d: f32,
    pub power_estimate: f32,
    pub significant: bool,
    pub practical: bool,
}

/// Experiment conclusion summary
#[derive(Debug, Clone)]
pub struct Conclusion {
    pub experiment_id: u64,
    pub domain: CoopTestDomain,
    pub treatment_better: bool,
    pub effect_size: f32,
    pub confidence: f32,
    pub recommendation: String,
}

// ============================================================================
// EXPERIMENT STATS
// ============================================================================

/// Aggregate experiment engine statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct ExperimentStats {
    pub total_designed: u64,
    pub total_concluded: u64,
    pub total_significant: u64,
    pub total_practical: u64,
    pub total_invalidated: u64,
    pub observations_collected: u64,
    pub avg_effect_size_ema: f32,
    pub avg_power_ema: f32,
    pub success_rate_ema: f32,
}

// ============================================================================
// COOPERATION EXPERIMENT ENGINE
// ============================================================================

/// Controlled experimentation engine for cooperation protocols
#[derive(Debug)]
pub struct CoopExperiment {
    experiments: BTreeMap<u64, CoopExperimentRecord>,
    tick: u64,
    rng_state: u64,
    stats: ExperimentStats,
}

impl CoopExperiment {
    /// Create a new experiment engine with the given PRNG seed
    pub fn new(seed: u64) -> Self {
        Self {
            experiments: BTreeMap::new(),
            tick: 0,
            rng_state: seed | 1,
            stats: ExperimentStats::default(),
        }
    }

    /// Design a new controlled A/B experiment
    pub fn design_experiment(
        &mut self,
        name: String,
        domain: CoopTestDomain,
        hypothesis_id: u64,
        control_name: String,
        treatment_name: String,
    ) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes()) ^ xorshift64(&mut self.rng_state);
        let control_id = fnv1a_hash(control_name.as_bytes());
        let treatment_id = fnv1a_hash(treatment_name.as_bytes());

        let record = CoopExperimentRecord {
            id,
            name,
            domain,
            phase: ExperimentPhase::Designed,
            control: ExperimentArm {
                id: control_id,
                name: control_name,
                is_control: true,
                observations: Vec::new(),
                mean: 0.0,
                variance: 0.0,
                count: 0,
            },
            treatment: ExperimentArm {
                id: treatment_id,
                name: treatment_name,
                is_control: false,
                observations: Vec::new(),
                mean: 0.0,
                variance: 0.0,
                count: 0,
            },
            hypothesis_id,
            created_tick: self.tick,
            concluded_tick: 0,
            t_statistic: 0.0,
            p_value: 1.0,
            effect_size: 0.0,
            significant: false,
        };

        if self.experiments.len() < MAX_EXPERIMENTS {
            self.experiments.insert(id, record);
            self.stats.total_designed += 1;
        }
        id
    }

    /// Run a controlled test — add an observation to the appropriate arm
    pub fn controlled_test(
        &mut self,
        experiment_id: u64,
        is_control: bool,
        value: f32,
    ) -> bool {
        self.tick += 1;
        let exp = match self.experiments.get_mut(&experiment_id) {
            Some(e) => e,
            None => return false,
        };
        if exp.phase == ExperimentPhase::Concluded
            || exp.phase == ExperimentPhase::Invalidated
        {
            return false;
        }
        if exp.phase == ExperimentPhase::Designed {
            exp.phase = ExperimentPhase::Running;
        }

        let arm = if is_control {
            &mut exp.control
        } else {
            &mut exp.treatment
        };
        if arm.observations.len() >= MAX_SAMPLES_PER_ARM {
            return false;
        }
        let meta_hash = fnv1a_hash(&value.to_le_bytes()) ^ fnv1a_hash(&self.tick.to_le_bytes());
        arm.observations.push(Observation {
            tick: self.tick,
            value,
            arm_id: arm.id,
            metadata_hash: meta_hash,
        });
        arm.count = arm.observations.len();

        // Incremental mean and variance update (Welford's)
        let n = arm.count as f32;
        let old_mean = arm.mean;
        arm.mean = old_mean + (value - old_mean) / n;
        if arm.count > 1 {
            arm.variance += (value - old_mean) * (value - arm.mean);
        }

        self.stats.observations_collected += 1;
        true
    }

    /// Measure the current outcome difference between arms
    pub fn measure_outcome(&self, experiment_id: u64) -> Option<(f32, f32, f32)> {
        let exp = self.experiments.get(&experiment_id)?;
        if exp.control.count == 0 || exp.treatment.count == 0 {
            return None;
        }
        let diff = exp.treatment.mean - exp.control.mean;
        let ctrl_var = if exp.control.count > 1 {
            exp.control.variance / (exp.control.count as f32 - 1.0)
        } else {
            0.0
        };
        let treat_var = if exp.treatment.count > 1 {
            exp.treatment.variance / (exp.treatment.count as f32 - 1.0)
        } else {
            0.0
        };
        let pooled_std = ((ctrl_var + treat_var) / 2.0).max(0.0001);
        Some((diff, exp.control.mean, exp.treatment.mean))
    }

    /// Perform full statistical analysis: Welch's t-test + Cohen's d
    pub fn statistical_analysis(&mut self, experiment_id: u64) -> Option<AnalysisResult> {
        let exp = match self.experiments.get_mut(&experiment_id) {
            Some(e) => e,
            None => return None,
        };
        if exp.control.count < MIN_SAMPLES_FOR_ANALYSIS
            || exp.treatment.count < MIN_SAMPLES_FOR_ANALYSIS
        {
            return None;
        }
        exp.phase = ExperimentPhase::Analyzing;

        let n1 = exp.control.count as f32;
        let n2 = exp.treatment.count as f32;
        let s1_sq = if n1 > 1.0 {
            exp.control.variance / (n1 - 1.0)
        } else {
            0.0
        };
        let s2_sq = if n2 > 1.0 {
            exp.treatment.variance / (n2 - 1.0)
        } else {
            0.0
        };

        // Welch's t-statistic
        let se = (s1_sq / n1 + s2_sq / n2).max(0.0001);
        let t_stat = (exp.treatment.mean - exp.control.mean) / se;

        // Welch-Satterthwaite degrees of freedom (approximation)
        let num = (s1_sq / n1 + s2_sq / n2) * (s1_sq / n1 + s2_sq / n2);
        let denom = (s1_sq / n1) * (s1_sq / n1) / (n1 - 1.0)
            + (s2_sq / n2) * (s2_sq / n2) / (n2 - 1.0);
        let df = if denom > 0.0 { num / denom } else { n1 + n2 - 2.0 };

        // Approximate p-value using a sigmoid approximation for |t|
        let abs_t = if t_stat < 0.0 { -t_stat } else { t_stat };
        let p_approx = 2.0 / (1.0 + (0.7 * abs_t * (1.0 + 1.0 / (4.0 * df.max(1.0)))));
        let p_approx = p_approx.clamp(0.0, 1.0);

        // Cohen's d — standardized effect size
        let pooled_sd = ((s1_sq + s2_sq) / 2.0).max(0.0001);
        let cohens_d = (exp.treatment.mean - exp.control.mean) / pooled_sd;
        let abs_d = if cohens_d < 0.0 { -cohens_d } else { cohens_d };

        // Power estimate (simplified — proportional to sample size and effect)
        let power_est = (1.0 - p_approx) * (n1 + n2) / (n1 + n2 + 20.0);
        let power_est = power_est.clamp(0.0, 1.0);

        let significant = p_approx < SIGNIFICANCE_LEVEL;
        let practical = abs_d >= MIN_EFFECT_SIZE;

        exp.t_statistic = t_stat;
        exp.p_value = p_approx;
        exp.effect_size = cohens_d;
        exp.significant = significant;

        self.stats.avg_effect_size_ema =
            EMA_ALPHA * abs_d + (1.0 - EMA_ALPHA) * self.stats.avg_effect_size_ema;
        self.stats.avg_power_ema =
            EMA_ALPHA * power_est + (1.0 - EMA_ALPHA) * self.stats.avg_power_ema;

        Some(AnalysisResult {
            experiment_id,
            t_statistic: t_stat,
            p_value_approx: p_approx,
            cohens_d,
            power_estimate: power_est,
            significant,
            practical,
        })
    }

    /// Conclude an experiment and generate a recommendation
    pub fn experiment_conclusion(&mut self, experiment_id: u64) -> Option<Conclusion> {
        let exp = match self.experiments.get_mut(&experiment_id) {
            Some(e) => e,
            None => return None,
        };
        if exp.phase == ExperimentPhase::Concluded {
            let treatment_better = exp.effect_size > 0.0 && exp.significant;
            let confidence = 1.0 - exp.p_value;
            let recommendation = if treatment_better {
                String::from("Treatment protocol outperforms control. Deploy recommended.")
            } else if exp.significant {
                String::from("Control protocol is superior. Retain current protocol.")
            } else {
                String::from("No significant difference. More data needed or discard.")
            };
            return Some(Conclusion {
                experiment_id,
                domain: exp.domain,
                treatment_better,
                effect_size: exp.effect_size,
                confidence,
                recommendation,
            });
        }

        // Auto-invalidate expired experiments
        if self.tick > exp.created_tick + MAX_EXPERIMENT_TICKS
            && exp.phase != ExperimentPhase::Analyzing
        {
            exp.phase = ExperimentPhase::Invalidated;
            self.stats.total_invalidated += 1;
            return None;
        }

        exp.phase = ExperimentPhase::Concluded;
        exp.concluded_tick = self.tick;
        self.stats.total_concluded += 1;
        if exp.significant {
            self.stats.total_significant += 1;
        }
        let abs_d = if exp.effect_size < 0.0 {
            -exp.effect_size
        } else {
            exp.effect_size
        };
        if abs_d >= MIN_EFFECT_SIZE {
            self.stats.total_practical += 1;
        }

        let rate = self.stats.total_significant as f32
            / self.stats.total_concluded.max(1) as f32;
        self.stats.success_rate_ema =
            EMA_ALPHA * rate + (1.0 - EMA_ALPHA) * self.stats.success_rate_ema;

        let treatment_better = exp.effect_size > 0.0 && exp.significant;
        let confidence = 1.0 - exp.p_value;
        let recommendation = if treatment_better {
            String::from("Treatment protocol outperforms control. Deploy recommended.")
        } else if exp.significant {
            String::from("Control protocol is superior. Retain current protocol.")
        } else {
            String::from("No significant difference. More data needed or discard.")
        };

        Some(Conclusion {
            experiment_id,
            domain: exp.domain,
            treatment_better,
            effect_size: exp.effect_size,
            confidence,
            recommendation,
        })
    }

    /// Get current experiment engine statistics
    pub fn stats(&self) -> &ExperimentStats {
        &self.stats
    }
}
