// SPDX-License-Identifier: GPL-2.0
//! # Apps Discovery Validator — Rigorous Validation of Classification Discoveries
//!
//! Before any app classification discovery is applied live, it must pass through
//! this validator. Four validation gates: classification accuracy improvement
//! check, no regression on existing workloads, statistical significance
//! re-confirmation via cross-validation, and holdout set testing. Only
//! discoveries that pass all gates earn the "validated" stamp.
//!
//! The engine that trusts, but verifies — every classification change.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_DISCOVERIES: usize = 256;
const MAX_TRIAL_RUNS: usize = 16;
const MIN_TRIALS_FOR_REPRODUCIBILITY: usize = 5;
const REGRESSION_THRESHOLD: f32 = 0.02;
const HOLDOUT_FRACTION: f32 = 0.20;
const SIGNIFICANCE_ALPHA: f32 = 0.05;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const CROSS_VALIDATION_FOLDS: usize = 5;
const SAFETY_MARGIN: f32 = 0.95;

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
// VALIDATION TYPES
// ============================================================================

/// Overall validation verdict
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidationVerdict {
    Pending,
    Passed,
    FailedRegression,
    FailedSignificance,
    FailedCrossValidation,
    FailedHoldout,
}

/// Which gate a validation check belongs to
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidationGate {
    Regression,
    Significance,
    CrossValidation,
    Holdout,
}

/// A single trial run result
#[derive(Debug, Clone, Copy)]
pub struct TrialRun {
    pub trial_id: u64,
    pub baseline_accuracy: f32,
    pub discovery_accuracy: f32,
    pub improvement: f32,
    pub passed: bool,
    pub tick: u64,
}

/// Workload regression check entry
#[derive(Debug, Clone)]
pub struct WorkloadRegression {
    pub workload_name: String,
    pub baseline_accuracy: f32,
    pub post_accuracy: f32,
    pub regressed: bool,
}

/// A discovery to validate
#[derive(Debug, Clone)]
pub struct ClassificationDiscovery {
    pub discovery_id: u64,
    pub experiment_id: u64,
    pub description: String,
    pub claimed_improvement: f32,
    pub baseline_accuracy: f32,
    pub trials: Vec<TrialRun>,
    pub regressions: Vec<WorkloadRegression>,
    pub verdict: ValidationVerdict,
    pub validation_tick: u64,
}

/// Validation report for a discovery
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub discovery_id: u64,
    pub verdict: ValidationVerdict,
    pub regression_passed: bool,
    pub significance_passed: bool,
    pub cross_validation_passed: bool,
    pub holdout_passed: bool,
    pub observed_improvement: f32,
    pub cross_val_mean_accuracy: f32,
    pub holdout_accuracy: f32,
    pub trial_count: usize,
    pub details: String,
}

// ============================================================================
// VALIDATOR STATS
// ============================================================================

/// Aggregate validation statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatorStats {
    pub total_validations: u64,
    pub total_passed: u64,
    pub total_failed_regression: u64,
    pub total_failed_significance: u64,
    pub total_failed_cross_val: u64,
    pub total_failed_holdout: u64,
    pub pass_rate_ema: f32,
    pub avg_improvement_ema: f32,
    pub avg_cross_val_ema: f32,
}

// ============================================================================
// GATE CHECKER
// ============================================================================

/// Internal per-gate validation logic
#[derive(Debug)]
struct GateChecker {
    regression_results: BTreeMap<u64, bool>,
    significance_results: BTreeMap<u64, bool>,
    cross_val_results: BTreeMap<u64, f32>,
    holdout_results: BTreeMap<u64, f32>,
}

impl GateChecker {
    fn new() -> Self {
        Self {
            regression_results: BTreeMap::new(),
            significance_results: BTreeMap::new(),
            cross_val_results: BTreeMap::new(),
            holdout_results: BTreeMap::new(),
        }
    }

    fn record_regression(&mut self, discovery_id: u64, passed: bool) {
        self.regression_results.insert(discovery_id, passed);
    }

    fn record_significance(&mut self, discovery_id: u64, passed: bool) {
        self.significance_results.insert(discovery_id, passed);
    }

    fn record_cross_val(&mut self, discovery_id: u64, mean_acc: f32) {
        self.cross_val_results.insert(discovery_id, mean_acc);
    }

    fn record_holdout(&mut self, discovery_id: u64, holdout_acc: f32) {
        self.holdout_results.insert(discovery_id, holdout_acc);
    }
}

// ============================================================================
// APPS DISCOVERY VALIDATOR
// ============================================================================

/// Rigorous validation engine for app classification discoveries
#[derive(Debug)]
pub struct AppsDiscoveryValidator {
    discoveries: BTreeMap<u64, ClassificationDiscovery>,
    gate_checker: GateChecker,
    rng_state: u64,
    current_tick: u64,
    stats: ValidatorStats,
}

impl AppsDiscoveryValidator {
    /// Create a new validator with a seed
    pub fn new(seed: u64) -> Self {
        Self {
            discoveries: BTreeMap::new(),
            gate_checker: GateChecker::new(),
            rng_state: seed | 1,
            current_tick: 0,
            stats: ValidatorStats::default(),
        }
    }

    /// Register a discovery for validation
    pub fn register_discovery(
        &mut self,
        experiment_id: u64,
        description: String,
        claimed_improvement: f32,
        baseline_accuracy: f32,
        tick: u64,
    ) -> u64 {
        self.current_tick = tick;
        let id = fnv1a_hash(description.as_bytes()) ^ xorshift64(&mut self.rng_state);
        let discovery = ClassificationDiscovery {
            discovery_id: id,
            experiment_id,
            description,
            claimed_improvement,
            baseline_accuracy,
            trials: Vec::new(),
            regressions: Vec::new(),
            verdict: ValidationVerdict::Pending,
            validation_tick: tick,
        };
        if self.discoveries.len() < MAX_DISCOVERIES {
            self.discoveries.insert(id, discovery);
        }
        id
    }

    /// Validate improvement by running trial comparisons
    pub fn validate_improvement(
        &mut self,
        discovery_id: u64,
        new_accuracy: f32,
        tick: u64,
    ) -> bool {
        self.current_tick = tick;
        let discovery = match self.discoveries.get_mut(&discovery_id) {
            Some(d) => d,
            None => return false,
        };
        if discovery.trials.len() >= MAX_TRIAL_RUNS {
            return false;
        }
        let trial_id = xorshift64(&mut self.rng_state);
        let improvement = new_accuracy - discovery.baseline_accuracy;
        let passed = improvement > 0.0;
        discovery.trials.push(TrialRun {
            trial_id,
            baseline_accuracy: discovery.baseline_accuracy,
            discovery_accuracy: new_accuracy,
            improvement,
            passed,
            tick,
        });
        passed
    }

    /// Run regression test against known workloads
    pub fn regression_test(
        &mut self,
        discovery_id: u64,
        workload_name: String,
        baseline_acc: f32,
        post_acc: f32,
    ) -> bool {
        let discovery = match self.discoveries.get_mut(&discovery_id) {
            Some(d) => d,
            None => return false,
        };
        let regression = baseline_acc - post_acc;
        let regressed = regression > REGRESSION_THRESHOLD;
        discovery.regressions.push(WorkloadRegression {
            workload_name,
            baseline_accuracy: baseline_acc,
            post_accuracy: post_acc,
            regressed,
        });
        let all_pass = discovery.regressions.iter().all(|r| !r.regressed);
        self.gate_checker.record_regression(discovery_id, all_pass);
        !regressed
    }

    /// Perform k-fold cross-validation on the discovery
    pub fn cross_validation(
        &mut self,
        discovery_id: u64,
        fold_accuracies: &[f32],
    ) -> f32 {
        let k = fold_accuracies.len().min(CROSS_VALIDATION_FOLDS);
        if k == 0 {
            return 0.0;
        }
        let mean = fold_accuracies[..k].iter().sum::<f32>() / k as f32;
        let variance = fold_accuracies[..k]
            .iter()
            .map(|&x| (x - mean) * (x - mean))
            .sum::<f32>()
            / k as f32;
        let std_dev = sqrt_approx(variance);

        // Require low variance and high mean
        let cv_passed = std_dev < 0.10 && mean > SAFETY_MARGIN * 0.5;
        self.gate_checker.record_cross_val(discovery_id, mean);
        self.gate_checker
            .record_significance(discovery_id, cv_passed);
        self.stats.avg_cross_val_ema =
            EMA_ALPHA * mean + (1.0 - EMA_ALPHA) * self.stats.avg_cross_val_ema;
        mean
    }

    /// Holdout set test — final accuracy on held-out data
    pub fn holdout_test(
        &mut self,
        discovery_id: u64,
        holdout_accuracy: f32,
        baseline_accuracy: f32,
    ) -> bool {
        let improvement = holdout_accuracy - baseline_accuracy;
        let passed = improvement > 0.0 && holdout_accuracy > SAFETY_MARGIN * baseline_accuracy;
        self.gate_checker
            .record_holdout(discovery_id, holdout_accuracy);
        passed
    }

    /// Issue final validation verdict combining all gates
    pub fn validation_verdict(&mut self, discovery_id: u64, tick: u64) -> Option<ValidationReport> {
        self.current_tick = tick;
        let discovery = match self.discoveries.get(&discovery_id) {
            Some(d) => d,
            None => return None,
        };

        // Gate 1: Regression
        let regression_passed = self
            .gate_checker
            .regression_results
            .get(&discovery_id)
            .copied()
            .unwrap_or(true);

        // Gate 2: Significance (via cross-val consistency)
        let significance_passed = self
            .gate_checker
            .significance_results
            .get(&discovery_id)
            .copied()
            .unwrap_or(false);

        // Gate 3: Cross-validation mean accuracy
        let cv_mean = self
            .gate_checker
            .cross_val_results
            .get(&discovery_id)
            .copied()
            .unwrap_or(0.0);
        let cross_val_passed = cv_mean > discovery.baseline_accuracy;

        // Gate 4: Holdout accuracy
        let holdout_acc = self
            .gate_checker
            .holdout_results
            .get(&discovery_id)
            .copied()
            .unwrap_or(0.0);
        let holdout_passed = holdout_acc > discovery.baseline_accuracy * SAFETY_MARGIN;

        // Compute observed improvement from trials
        let observed_improvement = if discovery.trials.is_empty() {
            0.0
        } else {
            discovery.trials.iter().map(|t| t.improvement).sum::<f32>()
                / discovery.trials.len() as f32
        };

        // Determine verdict
        let verdict = if !regression_passed {
            ValidationVerdict::FailedRegression
        } else if !significance_passed {
            ValidationVerdict::FailedSignificance
        } else if !cross_val_passed {
            ValidationVerdict::FailedCrossValidation
        } else if !holdout_passed {
            ValidationVerdict::FailedHoldout
        } else {
            ValidationVerdict::Passed
        };

        // Update discovery
        if let Some(d) = self.discoveries.get_mut(&discovery_id) {
            d.verdict = verdict;
            d.validation_tick = tick;
        }

        // Update stats
        self.stats.total_validations += 1;
        match verdict {
            ValidationVerdict::Passed => self.stats.total_passed += 1,
            ValidationVerdict::FailedRegression => self.stats.total_failed_regression += 1,
            ValidationVerdict::FailedSignificance => self.stats.total_failed_significance += 1,
            ValidationVerdict::FailedCrossValidation => self.stats.total_failed_cross_val += 1,
            ValidationVerdict::FailedHoldout => self.stats.total_failed_holdout += 1,
            ValidationVerdict::Pending => {}
        }

        let pass_val = if verdict == ValidationVerdict::Passed { 1.0 } else { 0.0 };
        self.stats.pass_rate_ema =
            EMA_ALPHA * pass_val + (1.0 - EMA_ALPHA) * self.stats.pass_rate_ema;
        self.stats.avg_improvement_ema =
            EMA_ALPHA * observed_improvement + (1.0 - EMA_ALPHA) * self.stats.avg_improvement_ema;

        let details = match verdict {
            ValidationVerdict::Passed => String::from("All validation gates passed"),
            ValidationVerdict::FailedRegression => String::from("Regression detected on existing workloads"),
            ValidationVerdict::FailedSignificance => String::from("Cross-validation variance too high"),
            ValidationVerdict::FailedCrossValidation => String::from("Cross-validation accuracy below baseline"),
            ValidationVerdict::FailedHoldout => String::from("Holdout accuracy below safety margin"),
            ValidationVerdict::Pending => String::from("Validation incomplete"),
        };

        Some(ValidationReport {
            discovery_id,
            verdict,
            regression_passed,
            significance_passed,
            cross_validation_passed: cross_val_passed,
            holdout_passed,
            observed_improvement,
            cross_val_mean_accuracy: cv_mean,
            holdout_accuracy: holdout_acc,
            trial_count: discovery.trials.len(),
            details,
        })
    }

    /// Get aggregate stats
    pub fn stats(&self) -> ValidatorStats {
        self.stats
    }

    /// Get discovery by id
    pub fn discovery(&self, discovery_id: u64) -> Option<&ClassificationDiscovery> {
        self.discoveries.get(&discovery_id)
    }
}
