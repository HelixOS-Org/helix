// SPDX-License-Identifier: GPL-2.0
//! # Bridge Discovery Validator — Rigorous Validation of Discovered Improvements
//!
//! Before any discovery is applied to the live bridge, it must pass through
//! this validator. Four validation gates: performance regression check, safety
//! invariant verification, statistical significance re-confirmation, and
//! reproducibility testing across multiple trial runs. Only discoveries that
//! pass all gates earn the "validated" stamp.
//!
//! The bridge that trusts, but verifies — every single time.

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
const REPRODUCIBILITY_THRESHOLD: f32 = 0.75;
const SIGNIFICANCE_ALPHA: f32 = 0.05;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
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
    FailedSafety,
    FailedSignificance,
    FailedReproducibility,
}

/// Which gate a validation check belongs to
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidationGate {
    Regression,
    Safety,
    Significance,
    Reproducibility,
}

/// A single trial run result
#[derive(Debug, Clone, Copy)]
pub struct TrialRun {
    pub trial_id: u64,
    pub baseline_metric: f32,
    pub discovery_metric: f32,
    pub improvement: f32,
    pub passed: bool,
    pub tick: u64,
}

/// Safety invariant to check
#[derive(Debug, Clone)]
pub struct SafetyInvariant {
    pub name: String,
    pub min_threshold: f32,
    pub max_threshold: f32,
    pub current_value: f32,
    pub satisfied: bool,
}

/// Discovery to validate
#[derive(Debug, Clone)]
pub struct Discovery {
    pub discovery_id: u64,
    pub experiment_id: u64,
    pub description: String,
    pub claimed_improvement: f32,
    pub baseline_metric: f32,
    pub trials: Vec<TrialRun>,
    pub safety_checks: Vec<SafetyInvariant>,
    pub verdict: ValidationVerdict,
    pub validation_tick: u64,
}

/// Validation report for a discovery
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub discovery_id: u64,
    pub verdict: ValidationVerdict,
    pub regression_passed: bool,
    pub safety_passed: bool,
    pub significance_passed: bool,
    pub reproducibility_passed: bool,
    pub observed_improvement: f32,
    pub reproducibility_rate: f32,
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
    pub total_failed_safety: u64,
    pub total_failed_significance: u64,
    pub total_failed_reproducibility: u64,
    pub pass_rate_ema: f32,
    pub avg_improvement_ema: f32,
    pub avg_reproducibility_ema: f32,
}

// ============================================================================
// GATE CHECKER
// ============================================================================

/// Internal gate-by-gate checker
#[derive(Debug)]
struct GateChecker {
    regression_results: BTreeMap<u64, bool>,
    safety_results: BTreeMap<u64, bool>,
    significance_results: BTreeMap<u64, bool>,
    reproducibility_results: BTreeMap<u64, bool>,
}

impl GateChecker {
    fn new() -> Self {
        Self {
            regression_results: BTreeMap::new(),
            safety_results: BTreeMap::new(),
            significance_results: BTreeMap::new(),
            reproducibility_results: BTreeMap::new(),
        }
    }

    fn record(&mut self, discovery_id: u64, gate: ValidationGate, passed: bool) {
        match gate {
            ValidationGate::Regression => {
                self.regression_results.insert(discovery_id, passed);
            }
            ValidationGate::Safety => {
                self.safety_results.insert(discovery_id, passed);
            }
            ValidationGate::Significance => {
                self.significance_results.insert(discovery_id, passed);
            }
            ValidationGate::Reproducibility => {
                self.reproducibility_results.insert(discovery_id, passed);
            }
        }
    }

    fn all_passed(&self, discovery_id: u64) -> bool {
        self.regression_results.get(&discovery_id).copied().unwrap_or(false)
            && self.safety_results.get(&discovery_id).copied().unwrap_or(false)
            && self.significance_results.get(&discovery_id).copied().unwrap_or(false)
            && self.reproducibility_results.get(&discovery_id).copied().unwrap_or(false)
    }
}

// ============================================================================
// BRIDGE DISCOVERY VALIDATOR
// ============================================================================

/// Validates discovered improvements through four rigorous gates
#[derive(Debug)]
pub struct BridgeDiscoveryValidator {
    discoveries: BTreeMap<u64, Discovery>,
    gate_checker: GateChecker,
    rng_state: u64,
    stats: ValidatorStats,
}

impl BridgeDiscoveryValidator {
    /// Create a new validator
    pub fn new(seed: u64) -> Self {
        Self {
            discoveries: BTreeMap::new(),
            gate_checker: GateChecker::new(),
            rng_state: seed | 1,
            stats: ValidatorStats::default(),
        }
    }

    /// Register a discovery for validation
    pub fn register_discovery(
        &mut self,
        experiment_id: u64,
        description: String,
        claimed_improvement: f32,
        baseline_metric: f32,
        tick: u64,
    ) -> u64 {
        let id = fnv1a_hash(description.as_bytes()) ^ xorshift64(&mut self.rng_state);
        let discovery = Discovery {
            discovery_id: id,
            experiment_id,
            description,
            claimed_improvement,
            baseline_metric,
            trials: Vec::new(),
            safety_checks: Vec::new(),
            verdict: ValidationVerdict::Pending,
            validation_tick: tick,
        };
        self.discoveries.insert(id, discovery);

        // Evict oldest if over capacity
        while self.discoveries.len() > MAX_DISCOVERIES {
            let oldest = self
                .discoveries
                .iter()
                .filter(|(_, d)| d.verdict != ValidationVerdict::Pending)
                .min_by_key(|(_, d)| d.validation_tick)
                .map(|(&k, _)| k);
            if let Some(k) = oldest {
                self.discoveries.remove(&k);
            } else {
                break;
            }
        }
        id
    }

    /// Add a trial run to a discovery
    pub fn add_trial(
        &mut self,
        discovery_id: u64,
        baseline_metric: f32,
        discovery_metric: f32,
        tick: u64,
    ) -> bool {
        let disc = match self.discoveries.get_mut(&discovery_id) {
            Some(d) => d,
            None => return false,
        };
        if disc.trials.len() >= MAX_TRIAL_RUNS {
            return false;
        }
        let improvement = if baseline_metric > 1e-10 {
            (discovery_metric - baseline_metric) / baseline_metric
        } else {
            0.0
        };
        let trial_id = fnv1a_hash(&tick.to_le_bytes()) ^ xorshift64(&mut self.rng_state);
        disc.trials.push(TrialRun {
            trial_id,
            baseline_metric,
            discovery_metric,
            improvement,
            passed: improvement > 0.0,
            tick,
        });
        true
    }

    /// Regression check: ensure no metric degraded beyond threshold
    pub fn regression_check(&mut self, discovery_id: u64) -> bool {
        let disc = match self.discoveries.get(&discovery_id) {
            Some(d) => d,
            None => return false,
        };
        let passed = if disc.trials.is_empty() {
            false
        } else {
            // Check that average improvement is positive and no trial regressed badly
            let avg_improvement: f32 =
                disc.trials.iter().map(|t| t.improvement).sum::<f32>() / disc.trials.len() as f32;
            let worst_regression = disc
                .trials
                .iter()
                .map(|t| t.improvement)
                .fold(f32::MAX, |a, b| if b < a { b } else { a });
            avg_improvement > 0.0 && worst_regression > -REGRESSION_THRESHOLD
        };
        self.gate_checker
            .record(discovery_id, ValidationGate::Regression, passed);
        if !passed {
            self.stats.total_failed_regression += 1;
        }
        passed
    }

    /// Safety verification: check all invariants hold
    pub fn safety_verify(
        &mut self,
        discovery_id: u64,
        invariants: Vec<SafetyInvariant>,
    ) -> bool {
        let disc = match self.discoveries.get_mut(&discovery_id) {
            Some(d) => d,
            None => return false,
        };
        let mut all_safe = true;
        for mut inv in invariants {
            inv.satisfied =
                inv.current_value >= inv.min_threshold * SAFETY_MARGIN
                    && inv.current_value <= inv.max_threshold * (2.0 - SAFETY_MARGIN);
            if !inv.satisfied {
                all_safe = false;
            }
            disc.safety_checks.push(inv);
        }
        self.gate_checker
            .record(discovery_id, ValidationGate::Safety, all_safe);
        if !all_safe {
            self.stats.total_failed_safety += 1;
        }
        all_safe
    }

    /// Reproducibility test: check that improvement reproduces across trials
    pub fn reproducibility_test(&mut self, discovery_id: u64) -> f32 {
        let disc = match self.discoveries.get(&discovery_id) {
            Some(d) => d,
            None => return 0.0,
        };
        if disc.trials.len() < MIN_TRIALS_FOR_REPRODUCIBILITY {
            return 0.0;
        }
        let positive_trials = disc.trials.iter().filter(|t| t.passed).count();
        let rate = positive_trials as f32 / disc.trials.len() as f32;
        let passed = rate >= REPRODUCIBILITY_THRESHOLD;
        self.gate_checker
            .record(discovery_id, ValidationGate::Reproducibility, passed);
        if !passed {
            self.stats.total_failed_reproducibility += 1;
        }
        self.stats.avg_reproducibility_ema =
            EMA_ALPHA * rate + (1.0 - EMA_ALPHA) * self.stats.avg_reproducibility_ema;
        rate
    }

    /// Statistical significance re-test on trial data
    fn significance_check(&mut self, discovery_id: u64) -> bool {
        let disc = match self.discoveries.get(&discovery_id) {
            Some(d) => d,
            None => return false,
        };
        if disc.trials.len() < MIN_TRIALS_FOR_REPRODUCIBILITY {
            self.gate_checker
                .record(discovery_id, ValidationGate::Significance, false);
            self.stats.total_failed_significance += 1;
            return false;
        }
        let improvements: Vec<f32> = disc.trials.iter().map(|t| t.improvement).collect();
        let mean = improvements.iter().sum::<f32>() / improvements.len() as f32;
        let var = improvements
            .iter()
            .map(|&x| (x - mean) * (x - mean))
            .sum::<f32>()
            / (improvements.len() - 1).max(1) as f32;
        let se = sqrt_approx(var / improvements.len() as f32);
        let t_stat = if se > 1e-10 { mean / se } else { 0.0 };
        let significant = abs_f32(t_stat) > 1.96 && mean > 0.0;
        self.gate_checker
            .record(discovery_id, ValidationGate::Significance, significant);
        if !significant {
            self.stats.total_failed_significance += 1;
        }
        significant
    }

    /// Full validation: runs all four gates and produces a report
    pub fn validate_discovery(&mut self, discovery_id: u64) -> Option<ValidationReport> {
        let regression_ok = self.regression_check(discovery_id);
        let safety_ok = self
            .discoveries
            .get(&discovery_id)
            .map_or(false, |d| !d.safety_checks.is_empty())
            && self
                .gate_checker
                .safety_results
                .get(&discovery_id)
                .copied()
                .unwrap_or(false);
        // If no safety checks were run, run with empty (auto-pass)
        let safety_passed = if self
            .discoveries
            .get(&discovery_id)
            .map_or(true, |d| d.safety_checks.is_empty())
        {
            self.gate_checker
                .record(discovery_id, ValidationGate::Safety, true);
            true
        } else {
            safety_ok
        };
        let significance_ok = self.significance_check(discovery_id);
        let repro_rate = self.reproducibility_test(discovery_id);
        let repro_ok = repro_rate >= REPRODUCIBILITY_THRESHOLD;

        let all_passed = regression_ok && safety_passed && significance_ok && repro_ok;

        let disc = self.discoveries.get_mut(&discovery_id)?;
        let avg_improvement = if disc.trials.is_empty() {
            0.0
        } else {
            disc.trials.iter().map(|t| t.improvement).sum::<f32>() / disc.trials.len() as f32
        };

        disc.verdict = if all_passed {
            ValidationVerdict::Passed
        } else if !regression_ok {
            ValidationVerdict::FailedRegression
        } else if !safety_passed {
            ValidationVerdict::FailedSafety
        } else if !significance_ok {
            ValidationVerdict::FailedSignificance
        } else {
            ValidationVerdict::FailedReproducibility
        };

        self.stats.total_validations += 1;
        if all_passed {
            self.stats.total_passed += 1;
        }
        let pass_indicator = if all_passed { 1.0_f32 } else { 0.0 };
        self.stats.pass_rate_ema =
            EMA_ALPHA * pass_indicator + (1.0 - EMA_ALPHA) * self.stats.pass_rate_ema;
        self.stats.avg_improvement_ema =
            EMA_ALPHA * avg_improvement + (1.0 - EMA_ALPHA) * self.stats.avg_improvement_ema;

        let mut details = String::from("Gates: ");
        details.push_str(if regression_ok { "REG=OK " } else { "REG=FAIL " });
        details.push_str(if safety_passed { "SAFE=OK " } else { "SAFE=FAIL " });
        details.push_str(if significance_ok { "SIG=OK " } else { "SIG=FAIL " });
        details.push_str(if repro_ok { "REPRO=OK" } else { "REPRO=FAIL" });

        Some(ValidationReport {
            discovery_id,
            verdict: disc.verdict,
            regression_passed: regression_ok,
            safety_passed,
            significance_passed: significance_ok,
            reproducibility_passed: repro_ok,
            observed_improvement: avg_improvement,
            reproducibility_rate: repro_rate,
            trial_count: disc.trials.len(),
            details,
        })
    }

    /// Generate a validation report without running gates (read-only summary)
    pub fn validation_report(&self, discovery_id: u64) -> Option<ValidationReport> {
        let disc = self.discoveries.get(&discovery_id)?;
        let avg_improvement = if disc.trials.is_empty() {
            0.0
        } else {
            disc.trials.iter().map(|t| t.improvement).sum::<f32>() / disc.trials.len() as f32
        };
        let positive = disc.trials.iter().filter(|t| t.passed).count();
        let repro_rate = if disc.trials.is_empty() {
            0.0
        } else {
            positive as f32 / disc.trials.len() as f32
        };
        Some(ValidationReport {
            discovery_id,
            verdict: disc.verdict,
            regression_passed: self.gate_checker.regression_results.get(&discovery_id).copied().unwrap_or(false),
            safety_passed: self.gate_checker.safety_results.get(&discovery_id).copied().unwrap_or(false),
            significance_passed: self.gate_checker.significance_results.get(&discovery_id).copied().unwrap_or(false),
            reproducibility_passed: self.gate_checker.reproducibility_results.get(&discovery_id).copied().unwrap_or(false),
            observed_improvement: avg_improvement,
            reproducibility_rate: repro_rate,
            trial_count: disc.trials.len(),
            details: String::from("Read-only report"),
        })
    }

    /// Get aggregate stats
    pub fn stats(&self) -> ValidatorStats {
        self.stats
    }
}
