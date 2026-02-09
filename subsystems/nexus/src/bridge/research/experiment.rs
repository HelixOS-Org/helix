// SPDX-License-Identifier: GPL-2.0
//! # Bridge Experiment — Controlled A/B Testing Framework
//!
//! Runs controlled experiments on bridge behavior, comparing a control group
//! against one or more treatment groups. Statistical significance is assessed
//! via chi-squared goodness-of-fit and Welch's t-test. Effect size is measured
//! with Cohen's d. Each experiment progresses through a lifecycle: design →
//! running → analysis → concluded.
//!
//! The bridge that doesn't guess — it tests.

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
const CHI_SQUARED_95_3DF: f32 = 7.815;
const T_CRITICAL_95: f32 = 1.96;

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

/// Approximate square root for no_std (Newton-Raphson)
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

/// Approximate absolute value
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

/// Type of statistical test to apply
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StatTestType {
    ChiSquared,
    WelchTTest,
    Both,
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
    pub test_type: StatTestType,
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

/// Chi-squared result
#[derive(Debug, Clone)]
pub struct ChiSquaredResult {
    pub chi2: f32,
    pub degrees_of_freedom: u32,
    pub p_value_approx: f32,
    pub significant: bool,
    pub observed: Vec<f32>,
    pub expected: Vec<f32>,
}

/// Experiment report
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
    pub chi_squared: Option<ChiSquaredResult>,
    pub conclusion: String,
}

// ============================================================================
// EXPERIMENT STATS
// ============================================================================

/// Aggregate experiment statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct ExperimentStats {
    pub total_designed: u64,
    pub total_running: u64,
    pub total_concluded: u64,
    pub total_significant: u64,
    pub total_samples: u64,
    pub avg_effect_size_ema: f32,
    pub avg_p_value_ema: f32,
    pub success_rate: f32,
}

// ============================================================================
// SAMPLE ACCUMULATOR
// ============================================================================

/// Accumulates samples and computes running statistics per group
#[derive(Debug, Clone)]
struct SampleAccumulator {
    control: Vec<f32>,
    treatment: Vec<f32>,
    control_categories: BTreeMap<u32, u32>,
    treatment_categories: BTreeMap<u32, u32>,
}

impl SampleAccumulator {
    fn new() -> Self {
        Self {
            control: Vec::new(),
            treatment: Vec::new(),
            control_categories: BTreeMap::new(),
            treatment_categories: BTreeMap::new(),
        }
    }

    fn add(&mut self, sample: ExperimentSample) {
        match sample.group {
            GroupAssignment::Control => {
                if self.control.len() < MAX_SAMPLES_PER_GROUP {
                    self.control.push(sample.value);
                }
                let c = self.control_categories.entry(sample.category).or_insert(0);
                *c += 1;
            },
            GroupAssignment::Treatment => {
                if self.treatment.len() < MAX_SAMPLES_PER_GROUP {
                    self.treatment.push(sample.value);
                }
                let c = self
                    .treatment_categories
                    .entry(sample.category)
                    .or_insert(0);
                *c += 1;
            },
        }
    }

    fn mean(data: &[f32]) -> f32 {
        if data.is_empty() {
            return 0.0;
        }
        data.iter().sum::<f32>() / data.len() as f32
    }

    fn variance(data: &[f32]) -> f32 {
        if data.len() < 2 {
            return 0.0;
        }
        let m = Self::mean(data);
        data.iter().map(|&x| (x - m) * (x - m)).sum::<f32>() / (data.len() - 1) as f32
    }
}

// ============================================================================
// BRIDGE EXPERIMENT
// ============================================================================

/// Controlled A/B experiment engine for bridge optimization
#[derive(Debug)]
pub struct BridgeExperiment {
    experiments: BTreeMap<u64, ExperimentInner>,
    rng_state: u64,
    stats: ExperimentStats,
}

#[derive(Debug)]
struct ExperimentInner {
    id: u64,
    design: ExperimentDesign,
    phase: ExperimentPhase,
    samples: SampleAccumulator,
    start_tick: u64,
    end_tick: Option<u64>,
    report: Option<ExperimentReport>,
}

impl BridgeExperiment {
    /// Create a new experiment engine
    pub fn new(seed: u64) -> Self {
        Self {
            experiments: BTreeMap::new(),
            rng_state: seed | 1,
            stats: ExperimentStats::default(),
        }
    }

    /// Design a new experiment
    pub fn design_experiment(&mut self, design: ExperimentDesign, tick: u64) -> u64 {
        let id = fnv1a_hash(design.name.as_bytes()) ^ xorshift64(&mut self.rng_state);
        let inner = ExperimentInner {
            id,
            design,
            phase: ExperimentPhase::Designed,
            samples: SampleAccumulator::new(),
            start_tick: tick,
            end_tick: None,
            report: None,
        };
        self.experiments.insert(id, inner);
        self.stats.total_designed += 1;

        // Evict oldest concluded if over capacity
        while self.experiments.len() > MAX_EXPERIMENTS {
            let concluded = self
                .experiments
                .iter()
                .filter(|(_, e)| e.phase == ExperimentPhase::Concluded)
                .min_by_key(|(_, e)| e.start_tick)
                .map(|(&k, _)| k);
            if let Some(k) = concluded {
                self.experiments.remove(&k);
            } else {
                break;
            }
        }
        id
    }

    /// Begin running an experiment
    pub fn run_experiment(&mut self, experiment_id: u64) -> bool {
        if let Some(exp) = self.experiments.get_mut(&experiment_id) {
            if exp.phase == ExperimentPhase::Designed {
                exp.phase = ExperimentPhase::Running;
                self.stats.total_running += 1;
                return true;
            }
        }
        false
    }

    /// Add a sample to a running experiment
    pub fn add_sample(&mut self, experiment_id: u64, sample: ExperimentSample) -> bool {
        if let Some(exp) = self.experiments.get_mut(&experiment_id) {
            if exp.phase == ExperimentPhase::Running {
                exp.samples.add(sample);
                self.stats.total_samples += 1;
                return true;
            }
        }
        false
    }

    /// Perform Welch's t-test on an experiment's data
    pub fn statistical_test(&self, experiment_id: u64) -> Option<StatTestResult> {
        let exp = self.experiments.get(&experiment_id)?;
        let ctrl = &exp.samples.control;
        let treat = &exp.samples.treatment;
        if ctrl.len() < MIN_SAMPLES_FOR_SIGNIFICANCE || treat.len() < MIN_SAMPLES_FOR_SIGNIFICANCE {
            return None;
        }
        let m1 = SampleAccumulator::mean(ctrl);
        let m2 = SampleAccumulator::mean(treat);
        let v1 = SampleAccumulator::variance(ctrl);
        let v2 = SampleAccumulator::variance(treat);
        let n1 = ctrl.len() as f32;
        let n2 = treat.len() as f32;

        let se = sqrt_approx(v1 / n1 + v2 / n2);
        if se < 1e-10 {
            return None;
        }
        let t_stat = (m2 - m1) / se;
        let p_val = self.p_value(abs_f32(t_stat));

        // Cohen's d
        let pooled_sd = sqrt_approx(((n1 - 1.0) * v1 + (n2 - 1.0) * v2) / (n1 + n2 - 2.0));
        let effect = if pooled_sd > 1e-10 {
            (m2 - m1) / pooled_sd
        } else {
            0.0
        };

        // Approximate power (simplified)
        let noncentrality = abs_f32(effect) * sqrt_approx(n1 * n2 / (n1 + n2));
        let power = (1.0 - (-0.5 * noncentrality).exp()).clamp(0.0, 1.0);

        Some(StatTestResult {
            test_statistic: t_stat,
            p_value: p_val,
            significant: p_val < exp.design.alpha,
            effect_size: effect,
            power,
        })
    }

    /// Approximate two-tailed p-value from |t| using logistic approximation
    pub fn p_value(&self, t_abs: f32) -> f32 {
        // Approximation: p ≈ 2 * (1 / (1 + exp(0.7 * t * sqrt(pi))))
        let z = 0.7 * t_abs * 1.7724539;
        let exp_neg = (-z).exp();
        let p = 2.0 * exp_neg / (1.0 + exp_neg);
        p.clamp(0.0, 1.0)
    }

    /// Chi-squared goodness of fit test comparing category distributions
    pub fn chi_squared_test(&self, experiment_id: u64) -> Option<ChiSquaredResult> {
        let exp = self.experiments.get(&experiment_id)?;
        let ctrl_cats = &exp.samples.control_categories;
        let treat_cats = &exp.samples.treatment_categories;

        // Collect all categories
        let mut all_cats: Vec<u32> = Vec::new();
        for &k in ctrl_cats.keys() {
            if !all_cats.contains(&k) {
                all_cats.push(k);
            }
        }
        for &k in treat_cats.keys() {
            if !all_cats.contains(&k) {
                all_cats.push(k);
            }
        }
        all_cats.sort();

        if all_cats.is_empty() {
            return None;
        }

        let ctrl_total: u32 = ctrl_cats.values().sum();
        let treat_total: u32 = treat_cats.values().sum();
        let grand_total = ctrl_total + treat_total;
        if grand_total == 0 {
            return None;
        }

        let mut chi2: f32 = 0.0;
        let mut observed: Vec<f32> = Vec::new();
        let mut expected: Vec<f32> = Vec::new();

        for &cat in &all_cats {
            let o_ctrl = *ctrl_cats.get(&cat).unwrap_or(&0) as f32;
            let o_treat = *treat_cats.get(&cat).unwrap_or(&0) as f32;
            let row_total = o_ctrl + o_treat;

            let e_ctrl = row_total * ctrl_total as f32 / grand_total as f32;
            let e_treat = row_total * treat_total as f32 / grand_total as f32;

            if e_ctrl > 0.0 {
                chi2 += (o_ctrl - e_ctrl) * (o_ctrl - e_ctrl) / e_ctrl;
            }
            if e_treat > 0.0 {
                chi2 += (o_treat - e_treat) * (o_treat - e_treat) / e_treat;
            }
            observed.push(o_treat);
            expected.push(e_treat);
        }

        let df = if all_cats.len() > 1 {
            (all_cats.len() - 1) as u32
        } else {
            1
        };

        let critical = match df {
            1 => CHI_SQUARED_95_1DF,
            2 => CHI_SQUARED_95_2DF,
            3 => CHI_SQUARED_95_3DF,
            _ => 3.841 + 2.0 * (df as f32 - 1.0),
        };

        // Approximate p-value from chi2: p ≈ exp(-chi2/2) for quick estimate
        let p_approx = (-chi2 / 2.0).exp().clamp(0.0, 1.0);

        Some(ChiSquaredResult {
            chi2,
            degrees_of_freedom: df,
            p_value_approx: p_approx,
            significant: chi2 > critical,
            observed,
            expected,
        })
    }

    /// Compute effect size (Cohen's d) for an experiment
    pub fn effect_size(&self, experiment_id: u64) -> Option<f32> {
        let exp = self.experiments.get(&experiment_id)?;
        let ctrl = &exp.samples.control;
        let treat = &exp.samples.treatment;
        if ctrl.len() < 2 || treat.len() < 2 {
            return None;
        }
        let m1 = SampleAccumulator::mean(ctrl);
        let m2 = SampleAccumulator::mean(treat);
        let v1 = SampleAccumulator::variance(ctrl);
        let v2 = SampleAccumulator::variance(treat);
        let n1 = ctrl.len() as f32;
        let n2 = treat.len() as f32;
        let pooled = sqrt_approx(((n1 - 1.0) * v1 + (n2 - 1.0) * v2) / (n1 + n2 - 2.0));
        if pooled < 1e-10 {
            return None;
        }
        Some((m2 - m1) / pooled)
    }

    /// Generate a full experiment report
    pub fn experiment_report(&mut self, experiment_id: u64) -> Option<ExperimentReport> {
        let t_test = self.statistical_test(experiment_id);
        let chi_sq = self.chi_squared_test(experiment_id);
        let effect = self.effect_size(experiment_id);

        let exp = self.experiments.get_mut(&experiment_id)?;
        let ctrl_mean = SampleAccumulator::mean(&exp.samples.control);
        let treat_mean = SampleAccumulator::mean(&exp.samples.treatment);

        let conclusion = if let Some(ref t) = t_test {
            if t.significant {
                let direction = if treat_mean > ctrl_mean {
                    "improvement"
                } else {
                    "regression"
                };
                let mut s = String::from("Significant ");
                s.push_str(direction);
                s.push_str(" detected");
                s
            } else {
                String::from("No significant difference detected")
            }
        } else {
            String::from("Insufficient data for analysis")
        };

        if exp.phase == ExperimentPhase::Running || exp.phase == ExperimentPhase::Analysis {
            exp.phase = ExperimentPhase::Analysis;
            if t_test.as_ref().map_or(false, |t| t.significant)
                || chi_sq.as_ref().map_or(false, |c| c.significant)
            {
                self.stats.total_significant += 1;
            }
        }

        // Update EMA stats
        if let Some(ref t) = t_test {
            self.stats.avg_effect_size_ema = EMA_ALPHA * abs_f32(t.effect_size)
                + (1.0 - EMA_ALPHA) * self.stats.avg_effect_size_ema;
            self.stats.avg_p_value_ema =
                EMA_ALPHA * t.p_value + (1.0 - EMA_ALPHA) * self.stats.avg_p_value_ema;
        }

        let report = ExperimentReport {
            experiment_id,
            name: exp.design.name.clone(),
            phase: exp.phase,
            control_mean: ctrl_mean,
            treatment_mean: treat_mean,
            control_n: exp.samples.control.len(),
            treatment_n: exp.samples.treatment.len(),
            t_test,
            chi_squared: chi_sq,
            conclusion,
        };
        exp.report = Some(report.clone());
        let _ = effect;
        Some(report)
    }

    /// Conclude an experiment
    pub fn conclude(&mut self, experiment_id: u64) -> bool {
        if let Some(exp) = self.experiments.get_mut(&experiment_id) {
            exp.phase = ExperimentPhase::Concluded;
            self.stats.total_concluded += 1;
            if self.stats.total_running > 0 {
                self.stats.total_running -= 1;
            }
            if self.stats.total_concluded > 0 {
                self.stats.success_rate =
                    self.stats.total_significant as f32 / self.stats.total_concluded as f32;
            }
            return true;
        }
        false
    }

    /// Get aggregate stats
    pub fn stats(&self) -> ExperimentStats {
        self.stats
    }
}
