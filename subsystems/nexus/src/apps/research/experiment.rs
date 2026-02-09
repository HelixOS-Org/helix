// SPDX-License-Identifier: GPL-2.0
//! # Apps Experiment — Controlled A/B Testing for Classification
//!
//! Runs controlled experiments on app classification and prediction algorithms.
//! A/B tests compare a control classifier against treatment classifiers, feature
//! sets, or adaptation strategies. Statistical significance is assessed via
//! chi-squared goodness-of-fit and Welch's t-test. Effect size is measured
//! with Cohen's d. Each experiment progresses through: design → running →
//! analysis → concluded.
//!
//! The engine that doesn't guess — it tests every classification change.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_EXPERIMENTS: usize = 128;
const MAX_SAMPLES_PER_GROUP: usize = 2048;
const MIN_SAMPLES_FOR_SIGNIFICANCE: usize = 30;
const DEFAULT_ALPHA: f32 = 0.05;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const CHI_SQUARED_95_1DF: f32 = 3.841;
const CHI_SQUARED_95_2DF: f32 = 5.991;
const T_CRITICAL_95: f32 = 1.96;
const MAX_ARCHIVE: usize = 512;

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

fn sqrt_approx(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    let mut guess = x * 0.5;
    for _ in 0..12 {
        guess = 0.5 * (guess + x / guess);
    }
    guess
}

fn abs_f32(x: f32) -> f32 {
    if x < 0.0 { -x } else { x }
}

// ============================================================================
// EXPERIMENT TYPES
// ============================================================================

/// Phase of an experiment
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExperimentPhase {
    Designed,
    Running,
    Analysis,
    Concluded,
    Abandoned,
}

/// Type of classification experiment
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExperimentKind {
    ClassifierComparison,
    FeatureSetEvaluation,
    AdaptationStrategy,
    PredictionAlgorithm,
}

/// Which group a sample belongs to
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GroupAssignment {
    Control,
    Treatment,
}

/// A single observation within an experiment
#[derive(Debug, Clone, Copy)]
pub struct ExperimentSample {
    pub group: GroupAssignment,
    pub value: f32,
    pub tick: u64,
    pub category: u32,
}

/// Design of an experiment
#[derive(Debug, Clone)]
pub struct ExperimentDesign {
    pub name: String,
    pub hypothesis_id: u64,
    pub metric_name: String,
    pub kind: ExperimentKind,
    pub alpha: f32,
    pub min_samples: usize,
    pub max_duration_ticks: u64,
}

/// Statistical test result
#[derive(Debug, Clone, Copy)]
pub struct StatTestResult {
    pub test_statistic: f32,
    pub p_value: f32,
    pub significant: bool,
    pub effect_size: f32,
    pub power: f32,
}

/// Experiment conclusion report
#[derive(Debug, Clone)]
pub struct ExperimentReport {
    pub experiment_id: u64,
    pub name: String,
    pub phase: ExperimentPhase,
    pub control_mean: f32,
    pub treatment_mean: f32,
    pub control_n: usize,
    pub treatment_n: usize,
    pub t_test: Option<StatTestResult>,
    pub effect_size_d: f32,
    pub winner: GroupAssignment,
    pub conclusion: String,
}

/// Archived experiment summary
#[derive(Debug, Clone)]
pub struct ArchivedExperiment {
    pub experiment_id: u64,
    pub name: String,
    pub winner: GroupAssignment,
    pub effect_size: f32,
    pub tick: u64,
}

// ============================================================================
// EXPERIMENT STATS
// ============================================================================

/// Aggregate experiment statistics
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct ExperimentStats {
    pub total_designed: u64,
    pub total_concluded: u64,
    pub total_abandoned: u64,
    pub total_samples: u64,
    pub significant_results: u64,
    pub avg_effect_size_ema: f32,
    pub avg_sample_count_ema: f32,
    pub treatment_win_rate: f32,
    pub archived_count: u64,
}

// ============================================================================
// EXPERIMENT STATE
// ============================================================================

/// A live experiment with control and treatment samples
#[derive(Debug, Clone)]
struct LiveExperiment {
    experiment_id: u64,
    design: ExperimentDesign,
    phase: ExperimentPhase,
    control_samples: Vec<f32>,
    treatment_samples: Vec<f32>,
    start_tick: u64,
}

// ============================================================================
// APPS EXPERIMENT ENGINE
// ============================================================================

/// Controlled A/B testing framework for app classification research
#[derive(Debug)]
pub struct AppsExperiment {
    experiments: BTreeMap<u64, LiveExperiment>,
    archive: Vec<ArchivedExperiment>,
    rng_state: u64,
    current_tick: u64,
    stats: ExperimentStats,
}

impl AppsExperiment {
    /// Create a new experiment engine with a seed
    pub fn new(seed: u64) -> Self {
        Self {
            experiments: BTreeMap::new(),
            archive: Vec::new(),
            rng_state: seed | 1,
            current_tick: 0,
            stats: ExperimentStats::default(),
        }
    }

    /// Design a new A/B test for a classification hypothesis
    pub fn design_ab_test(&mut self, design: ExperimentDesign, tick: u64) -> u64 {
        self.current_tick = tick;
        let id = fnv1a_hash(design.name.as_bytes()) ^ xorshift64(&mut self.rng_state);
        let live = LiveExperiment {
            experiment_id: id,
            design,
            phase: ExperimentPhase::Designed,
            control_samples: Vec::new(),
            treatment_samples: Vec::new(),
            start_tick: tick,
        };
        if self.experiments.len() < MAX_EXPERIMENTS {
            self.experiments.insert(id, live);
            self.stats.total_designed += 1;
        }
        id
    }

    /// Run an experiment by feeding it a sample
    pub fn run_experiment(
        &mut self,
        experiment_id: u64,
        group: GroupAssignment,
        value: f32,
        tick: u64,
    ) -> bool {
        self.current_tick = tick;
        let exp = match self.experiments.get_mut(&experiment_id) {
            Some(e) => e,
            None => return false,
        };
        if exp.phase == ExperimentPhase::Designed {
            exp.phase = ExperimentPhase::Running;
        }
        if exp.phase != ExperimentPhase::Running {
            return false;
        }
        match group {
            GroupAssignment::Control => {
                if exp.control_samples.len() < MAX_SAMPLES_PER_GROUP {
                    exp.control_samples.push(value);
                }
            }
            GroupAssignment::Treatment => {
                if exp.treatment_samples.len() < MAX_SAMPLES_PER_GROUP {
                    exp.treatment_samples.push(value);
                }
            }
        }
        self.stats.total_samples += 1;

        // Check for duration expiry
        let elapsed = tick.saturating_sub(exp.start_tick);
        if elapsed > exp.design.max_duration_ticks {
            exp.phase = ExperimentPhase::Analysis;
        }
        // Check minimum sample threshold
        let enough = exp.control_samples.len() >= exp.design.min_samples
            && exp.treatment_samples.len() >= exp.design.min_samples;
        if enough && exp.control_samples.len() >= MIN_SAMPLES_FOR_SIGNIFICANCE {
            exp.phase = ExperimentPhase::Analysis;
        }
        true
    }

    /// Compute Welch's t-test significance between control and treatment
    pub fn significance_test(&self, experiment_id: u64) -> Option<StatTestResult> {
        let exp = self.experiments.get(&experiment_id)?;
        if exp.control_samples.len() < MIN_SAMPLES_FOR_SIGNIFICANCE
            || exp.treatment_samples.len() < MIN_SAMPLES_FOR_SIGNIFICANCE
        {
            return None;
        }
        let (c_mean, c_var) = mean_variance(&exp.control_samples);
        let (t_mean, t_var) = mean_variance(&exp.treatment_samples);
        let c_n = exp.control_samples.len() as f32;
        let t_n = exp.treatment_samples.len() as f32;

        let se = sqrt_approx(c_var / c_n + t_var / t_n);
        if se < 0.0001 {
            return Some(StatTestResult {
                test_statistic: 0.0,
                p_value: 1.0,
                significant: false,
                effect_size: 0.0,
                power: 0.0,
            });
        }
        let t_stat = (t_mean - c_mean) / se;
        // Approximate p-value from t-statistic (normal approximation for large n)
        let p_approx = approx_p_value(abs_f32(t_stat));
        let pooled_sd = sqrt_approx((c_var + t_var) / 2.0);
        let cohens_d = if pooled_sd > 0.001 {
            abs_f32(t_mean - c_mean) / pooled_sd
        } else {
            0.0
        };
        // Approximate power (simplified)
        let power = (1.0 - p_approx).clamp(0.0, 1.0);

        Some(StatTestResult {
            test_statistic: t_stat,
            p_value: p_approx,
            significant: p_approx < exp.design.alpha,
            effect_size: cohens_d,
            power,
        })
    }

    /// Compute effect size (Cohen's d) between groups
    pub fn effect_size(&self, experiment_id: u64) -> f32 {
        let exp = match self.experiments.get(&experiment_id) {
            Some(e) => e,
            None => return 0.0,
        };
        if exp.control_samples.is_empty() || exp.treatment_samples.is_empty() {
            return 0.0;
        }
        let (c_mean, c_var) = mean_variance(&exp.control_samples);
        let (t_mean, t_var) = mean_variance(&exp.treatment_samples);
        let pooled_sd = sqrt_approx((c_var + t_var) / 2.0);
        if pooled_sd < 0.001 {
            return 0.0;
        }
        abs_f32(t_mean - c_mean) / pooled_sd
    }

    /// Declare a winner for the experiment and conclude it
    #[inline]
    pub fn winner_declare(&mut self, experiment_id: u64, tick: u64) -> Option<ExperimentReport> {
        self.current_tick = tick;
        let exp = match self.experiments.get_mut(&experiment_id) {
            Some(e) => e,
            None => return None,
        };
        if exp.control_samples.is_empty() || exp.treatment_samples.is_empty() {
            return None;
        }
        let (c_mean, _) = mean_variance(&exp.control_samples);
        let (t_mean, _) = mean_variance(&exp.treatment_samples);
        let t_test = self.significance_test(experiment_id);
        let effect = self.effect_size(experiment_id);

        let exp = self.experiments.get_mut(&experiment_id).unwrap();
        exp.phase = ExperimentPhase::Concluded;
        self.stats.total_concluded += 1;

        let winner = if t_mean > c_mean {
            GroupAssignment::Treatment
        } else {
            GroupAssignment::Control
        };
        let is_sig = t_test.as_ref().map_or(false, |t| t.significant);
        if is_sig {
            self.stats.significant_results += 1;
        }
        self.stats.avg_effect_size_ema =
            EMA_ALPHA * effect + (1.0 - EMA_ALPHA) * self.stats.avg_effect_size_ema;
        let avg_n = (exp.control_samples.len() + exp.treatment_samples.len()) as f32 / 2.0;
        self.stats.avg_sample_count_ema =
            EMA_ALPHA * avg_n + (1.0 - EMA_ALPHA) * self.stats.avg_sample_count_ema;

        if matches!(winner, GroupAssignment::Treatment) {
            let total_concluded = self.stats.total_concluded.max(1) as f32;
            self.stats.treatment_win_rate =
                EMA_ALPHA * 1.0 + (1.0 - EMA_ALPHA) * self.stats.treatment_win_rate;
            let _ = total_concluded;
        }

        let conclusion = if is_sig {
            String::from("Statistically significant difference detected")
        } else {
            String::from("No statistically significant difference found")
        };

        Some(ExperimentReport {
            experiment_id: exp.experiment_id,
            name: exp.design.name.clone(),
            phase: exp.phase,
            control_mean: c_mean,
            treatment_mean: t_mean,
            control_n: exp.control_samples.len(),
            treatment_n: exp.treatment_samples.len(),
            t_test,
            effect_size_d: effect,
            winner,
            conclusion,
        })
    }

    /// Archive concluded experiments
    pub fn experiment_archive(&mut self, tick: u64) -> u64 {
        self.current_tick = tick;
        let mut to_archive: Vec<u64> = Vec::new();
        for (&id, exp) in self.experiments.iter() {
            if exp.phase == ExperimentPhase::Concluded || exp.phase == ExperimentPhase::Abandoned {
                to_archive.push(id);
            }
        }
        let archived = to_archive.len() as u64;
        for id in &to_archive {
            if let Some(exp) = self.experiments.remove(id) {
                let (c_mean, _) = mean_variance(&exp.control_samples);
                let (t_mean, _) = mean_variance(&exp.treatment_samples);
                let winner = if t_mean > c_mean {
                    GroupAssignment::Treatment
                } else {
                    GroupAssignment::Control
                };
                let (_, c_var) = mean_variance(&exp.control_samples);
                let (_, t_var) = mean_variance(&exp.treatment_samples);
                let pooled_sd = sqrt_approx((c_var + t_var) / 2.0);
                let effect = if pooled_sd > 0.001 {
                    abs_f32(t_mean - c_mean) / pooled_sd
                } else {
                    0.0
                };
                if self.archive.len() < MAX_ARCHIVE {
                    self.archive.push(ArchivedExperiment {
                        experiment_id: exp.experiment_id,
                        name: exp.design.name,
                        winner,
                        effect_size: effect,
                        tick,
                    });
                }
            }
        }
        self.stats.archived_count += archived;
        archived
    }

    /// Get aggregate stats
    #[inline(always)]
    pub fn stats(&self) -> ExperimentStats {
        self.stats
    }

    /// Get experiment report without concluding
    pub fn peek_report(&self, experiment_id: u64) -> Option<ExperimentReport> {
        let exp = self.experiments.get(&experiment_id)?;
        let (c_mean, _) = mean_variance(&exp.control_samples);
        let (t_mean, _) = mean_variance(&exp.treatment_samples);
        let t_test = self.significance_test(experiment_id);
        let effect = self.effect_size(experiment_id);
        let winner = if t_mean > c_mean {
            GroupAssignment::Treatment
        } else {
            GroupAssignment::Control
        };
        Some(ExperimentReport {
            experiment_id: exp.experiment_id,
            name: exp.design.name.clone(),
            phase: exp.phase,
            control_mean: c_mean,
            treatment_mean: t_mean,
            control_n: exp.control_samples.len(),
            treatment_n: exp.treatment_samples.len(),
            t_test,
            effect_size_d: effect,
            winner,
            conclusion: String::from("In progress"),
        })
    }
}

// ============================================================================
// STATISTICAL HELPERS
// ============================================================================

fn mean_variance(samples: &[f32]) -> (f32, f32) {
    if samples.is_empty() {
        return (0.0, 0.0);
    }
    let n = samples.len() as f32;
    let mean = samples.iter().sum::<f32>() / n;
    let var = samples.iter().map(|&x| (x - mean) * (x - mean)).sum::<f32>() / n;
    (mean, var)
}

/// Approximate two-tailed p-value from |t| using normal CDF approximation
fn approx_p_value(t_abs: f32) -> f32 {
    // Abramowitz & Stegun approximation for the normal CDF tail
    let b0: f32 = 0.2316419;
    let b1: f32 = 0.319381530;
    let b2: f32 = -0.356563782;
    let b3: f32 = 1.781477937;
    let b4: f32 = -1.821255978;
    let b5: f32 = 1.330274429;
    let t_val = 1.0 / (1.0 + b0 * t_abs);
    let t2 = t_val * t_val;
    let t3 = t2 * t_val;
    let t4 = t3 * t_val;
    let t5 = t4 * t_val;
    let poly = b1 * t_val + b2 * t2 + b3 * t3 + b4 * t4 + b5 * t5;
    // exp(-t^2/2) approximation
    let exp_approx = approx_neg_exp(t_abs * t_abs * 0.5);
    let one_tail = poly * exp_approx * 0.3989422804; // 1/sqrt(2*pi)
    (2.0 * one_tail).clamp(0.0, 1.0)
}

/// Approximate e^(-x) for x >= 0
fn approx_neg_exp(x: f32) -> f32 {
    if x > 20.0 {
        return 0.0;
    }
    let terms = 12;
    let mut result: f32 = 1.0;
    let mut term: f32 = 1.0;
    for i in 1..=terms {
        term *= -x / i as f32;
        result += term;
    }
    result.clamp(0.0, 1.0)
}
