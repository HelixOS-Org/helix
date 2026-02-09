// SPDX-License-Identifier: GPL-2.0
//! # Bridge Methodology — Research Methodology Framework
//!
//! Ensures every bridge experiment follows proper scientific methodology:
//! adequate control groups, randomization of treatment assignments,
//! sufficient sample sizes for statistical power, and absence of common
//! experimental design flaws. Before an experiment runs, the methodology
//! framework validates its design and scores its rigour. Post-experiment,
//! it suggests improvements for the next iteration.
//!
//! Good science requires good methodology. The bridge enforces it.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_EXPERIMENTS: usize = 1024;
const MAX_CHECKS_PER_EXPERIMENT: usize = 16;
const MAX_ISSUES: usize = 32;
const MIN_SAMPLE_SIZE: usize = 10;
const RECOMMENDED_SAMPLE_SIZE: usize = 30;
const IDEAL_SAMPLE_SIZE: usize = 100;
const MIN_CONTROL_RATIO: f32 = 0.20;
const MAX_CONTROL_RATIO: f32 = 0.50;
const RANDOMIZATION_ENTROPY_THRESHOLD: f32 = 0.60;
const METHODOLOGY_PASS_THRESHOLD: f32 = 0.70;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const MAX_HISTORY: usize = 2048;
const BIAS_DETECTION_WINDOW: usize = 20;

// ============================================================================
// HELPERS
// ============================================================================

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

fn abs_f32(v: f32) -> f32 {
    if v < 0.0 { -v } else { v }
}

fn sqrt_approx(v: f32) -> f32 {
    if v <= 0.0 {
        return 0.0;
    }
    let mut g = v * 0.5;
    for _ in 0..6 {
        g = 0.5 * (g + v / g);
    }
    g
}

// ============================================================================
// TYPES
// ============================================================================

/// Description of an experiment design.
#[derive(Clone)]
struct ExperimentDesign {
    id: u64,
    name: String,
    treatment_size: usize,
    control_size: usize,
    total_samples: usize,
    randomization_seed: u64,
    has_control_group: bool,
    treatment_assignments: Vec<bool>, // true = treatment, false = control
    submit_tick: u64,
}

/// A single methodology check result.
#[derive(Clone)]
pub struct MethodologyCheck {
    pub experiment_id: u64,
    pub checks_passed: Vec<String>,
    pub issues: Vec<String>,
    pub overall_score: f32,
    pub passed: bool,
    pub sample_adequate: bool,
    pub control_adequate: bool,
    pub randomization_adequate: bool,
}

/// Detailed issue description.
#[derive(Clone)]
struct Issue {
    severity: IssueSeverity,
    category: String,
    description: String,
    suggestion: String,
}

/// Issue severity level.
#[derive(Clone, Copy, PartialEq)]
enum IssueSeverity {
    Critical,
    Major,
    Minor,
    Info,
}

/// History entry for methodology checks.
#[derive(Clone)]
struct CheckRecord {
    experiment_id: u64,
    score: f32,
    passed: bool,
    issue_count: usize,
    tick: u64,
}

/// Methodology statistics.
#[derive(Clone)]
#[repr(align(64))]
pub struct MethodologyStats {
    pub total_validations: u64,
    pub passed_validations: u64,
    pub failed_validations: u64,
    pub avg_score_ema: f32,
    pub avg_sample_size_ema: f32,
    pub avg_control_ratio_ema: f32,
    pub avg_randomization_ema: f32,
    pub critical_issues_found: u64,
    pub improvement_suggestions: u64,
    pub best_score: f32,
}

/// Improvement suggestion.
#[derive(Clone)]
pub struct ImprovementSuggestion {
    pub experiment_id: u64,
    pub category: String,
    pub suggestion: String,
    pub expected_score_gain: f32,
    pub priority: u32,
}

// ============================================================================
// BRIDGE METHODOLOGY
// ============================================================================

/// Research methodology validation framework.
#[repr(align(64))]
pub struct BridgeMethodology {
    experiments: BTreeMap<u64, ExperimentDesign>,
    check_history: Vec<CheckRecord>,
    stats: MethodologyStats,
    rng_state: u64,
    tick: u64,
}

impl BridgeMethodology {
    /// Create a new methodology framework.
    pub fn new(seed: u64) -> Self {
        Self {
            experiments: BTreeMap::new(),
            check_history: Vec::new(),
            stats: MethodologyStats {
                total_validations: 0,
                passed_validations: 0,
                failed_validations: 0,
                avg_score_ema: 0.5,
                avg_sample_size_ema: 0.0,
                avg_control_ratio_ema: 0.0,
                avg_randomization_ema: 0.5,
                critical_issues_found: 0,
                improvement_suggestions: 0,
                best_score: 0.0,
            },
            rng_state: seed ^ 0xAE100D0106A001,
            tick: 0,
        }
    }

    /// Register an experiment design for validation.
    pub fn register_experiment(
        &mut self,
        name: &str,
        treatment_size: usize,
        control_size: usize,
        randomization_seed: u64,
        assignments: &[bool],
    ) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes()) ^ self.tick;

        if self.experiments.len() >= MAX_EXPERIMENTS {
            self.evict_oldest();
        }

        let total = treatment_size + control_size;
        let mut stored_assignments = Vec::new();
        for &a in assignments.iter().take(total) {
            stored_assignments.push(a);
        }

        self.experiments.insert(
            id,
            ExperimentDesign {
                id,
                name: String::from(name),
                treatment_size,
                control_size,
                total_samples: total,
                randomization_seed,
                has_control_group: control_size > 0,
                treatment_assignments: stored_assignments,
                submit_tick: self.tick,
            },
        );
        id
    }

    /// Full methodology validation of an experiment.
    pub fn validate_methodology(&mut self, experiment_id: u64) -> MethodologyCheck {
        self.tick += 1;
        self.stats.total_validations += 1;

        let design = match self.experiments.get(&experiment_id) {
            Some(d) => d.clone(),
            None => {
                return MethodologyCheck {
                    experiment_id,
                    checks_passed: Vec::new(),
                    issues: vec![String::from("Experiment not found")],
                    overall_score: 0.0,
                    passed: false,
                    sample_adequate: false,
                    control_adequate: false,
                    randomization_adequate: false,
                };
            }
        };

        let mut passed_checks = Vec::new();
        let mut issues = Vec::new();
        let mut score: f32 = 0.0;
        let mut weights_total: f32 = 0.0;

        // Check 1: Sample size (weight 0.30)
        let (sample_ok, sample_score) = self.evaluate_sample_size(design.total_samples);
        weights_total += 0.30;
        score += sample_score * 0.30;
        if sample_ok {
            passed_checks.push(String::from("sample_size: adequate"));
        } else {
            issues.push(String::from("sample_size: insufficient"));
        }

        // Check 2: Control group (weight 0.25)
        let (control_ok, control_score) =
            self.evaluate_control_group(design.control_size, design.total_samples);
        weights_total += 0.25;
        score += control_score * 0.25;
        if control_ok {
            passed_checks.push(String::from("control_group: present and adequate"));
        } else {
            issues.push(String::from("control_group: missing or undersized"));
        }

        // Check 3: Randomization (weight 0.25)
        let (rand_ok, rand_score) =
            self.evaluate_randomization(&design.treatment_assignments, design.total_samples);
        weights_total += 0.25;
        score += rand_score * 0.25;
        if rand_ok {
            passed_checks.push(String::from("randomization: adequate entropy"));
        } else {
            issues.push(String::from("randomization: poor entropy or bias"));
        }

        // Check 4: Balance (weight 0.10)
        let (balance_ok, balance_score) = self.evaluate_balance(&design);
        weights_total += 0.10;
        score += balance_score * 0.10;
        if balance_ok {
            passed_checks.push(String::from("balance: groups reasonably balanced"));
        } else {
            issues.push(String::from("balance: groups severely imbalanced"));
        }

        // Check 5: Contamination risk (weight 0.10)
        let (contam_ok, contam_score) = self.evaluate_contamination_risk(&design);
        weights_total += 0.10;
        score += contam_score * 0.10;
        if contam_ok {
            passed_checks.push(String::from("contamination: low risk"));
        } else {
            issues.push(String::from("contamination: high risk of cross-talk"));
        }

        let overall = if weights_total > 0.0 {
            score / weights_total * weights_total // already weighted
        } else {
            0.0
        };
        let overall_clamped = overall.max(0.0).min(1.0);
        let passed = overall_clamped >= METHODOLOGY_PASS_THRESHOLD;

        if passed {
            self.stats.passed_validations += 1;
        } else {
            self.stats.failed_validations += 1;
        }
        if !issues.is_empty() {
            self.stats.critical_issues_found += issues.len() as u64;
        }
        if overall_clamped > self.stats.best_score {
            self.stats.best_score = overall_clamped;
        }

        self.stats.avg_score_ema =
            self.stats.avg_score_ema * (1.0 - EMA_ALPHA) + overall_clamped * EMA_ALPHA;
        self.stats.avg_sample_size_ema = self.stats.avg_sample_size_ema * (1.0 - EMA_ALPHA)
            + design.total_samples as f32 * EMA_ALPHA;
        self.stats.avg_randomization_ema =
            self.stats.avg_randomization_ema * (1.0 - EMA_ALPHA) + rand_score * EMA_ALPHA;

        if self.check_history.len() < MAX_HISTORY {
            self.check_history.push(CheckRecord {
                experiment_id,
                score: overall_clamped,
                passed,
                issue_count: issues.len(),
                tick: self.tick,
            });
        }

        MethodologyCheck {
            experiment_id,
            checks_passed: passed_checks,
            issues,
            overall_score: overall_clamped,
            passed,
            sample_adequate: sample_ok,
            control_adequate: control_ok,
            randomization_adequate: rand_ok,
        }
    }

    /// Check if sample size is adequate.
    #[inline(always)]
    pub fn sample_size_check(&self, n: usize) -> (bool, f32) {
        self.evaluate_sample_size(n)
    }

    /// Check if control group is adequate.
    #[inline(always)]
    pub fn control_group_check(&self, control_n: usize, total_n: usize) -> (bool, f32) {
        self.evaluate_control_group(control_n, total_n)
    }

    /// Check randomization quality.
    #[inline(always)]
    pub fn randomization_check(&self, assignments: &[bool]) -> (bool, f32) {
        self.evaluate_randomization(assignments, assignments.len())
    }

    /// Overall methodology score across all validated experiments.
    #[inline(always)]
    pub fn methodology_score(&self) -> f32 {
        self.stats.avg_score_ema
    }

    /// Generate improvement suggestions for an experiment.
    pub fn improvement_suggestion(&mut self, experiment_id: u64) -> Vec<ImprovementSuggestion> {
        self.tick += 1;
        let mut suggestions = Vec::new();

        let design = match self.experiments.get(&experiment_id) {
            Some(d) => d.clone(),
            None => return suggestions,
        };

        // Sample size suggestion
        if design.total_samples < RECOMMENDED_SAMPLE_SIZE {
            let gain = (RECOMMENDED_SAMPLE_SIZE - design.total_samples) as f32
                / IDEAL_SAMPLE_SIZE as f32
                * 0.3;
            suggestions.push(ImprovementSuggestion {
                experiment_id,
                category: String::from("sample_size"),
                suggestion: String::from("increase total samples to at least 30"),
                expected_score_gain: gain.min(0.3),
                priority: 1,
            });
            self.stats.improvement_suggestions += 1;
        }

        // Control group suggestion
        if !design.has_control_group {
            suggestions.push(ImprovementSuggestion {
                experiment_id,
                category: String::from("control_group"),
                suggestion: String::from("add a control group with 30-50% of samples"),
                expected_score_gain: 0.25,
                priority: 1,
            });
            self.stats.improvement_suggestions += 1;
        } else {
            let ratio = design.control_size as f32 / design.total_samples.max(1) as f32;
            if ratio < MIN_CONTROL_RATIO {
                suggestions.push(ImprovementSuggestion {
                    experiment_id,
                    category: String::from("control_group"),
                    suggestion: String::from("increase control group to at least 20% of total"),
                    expected_score_gain: 0.15,
                    priority: 2,
                });
                self.stats.improvement_suggestions += 1;
            }
        }

        // Randomization suggestion
        let (rand_ok, _) = self.evaluate_randomization(
            &design.treatment_assignments,
            design.total_samples,
        );
        if !rand_ok {
            suggestions.push(ImprovementSuggestion {
                experiment_id,
                category: String::from("randomization"),
                suggestion: String::from("use better PRNG seeding for treatment assignment"),
                expected_score_gain: 0.20,
                priority: 2,
            });
            self.stats.improvement_suggestions += 1;
        }

        suggestions.sort_by_key(|s| s.priority);
        suggestions
    }

    /// Get statistics.
    #[inline(always)]
    pub fn stats(&self) -> &MethodologyStats {
        &self.stats
    }

    /// Number of registered experiments.
    #[inline(always)]
    pub fn experiment_count(&self) -> usize {
        self.experiments.len()
    }

    // ========================================================================
    // INTERNAL HELPERS
    // ========================================================================

    fn evaluate_sample_size(&self, n: usize) -> (bool, f32) {
        if n >= IDEAL_SAMPLE_SIZE {
            (true, 1.0)
        } else if n >= RECOMMENDED_SAMPLE_SIZE {
            let frac = n as f32 / IDEAL_SAMPLE_SIZE as f32;
            (true, 0.7 + 0.3 * frac)
        } else if n >= MIN_SAMPLE_SIZE {
            let frac = n as f32 / RECOMMENDED_SAMPLE_SIZE as f32;
            (false, 0.3 + 0.4 * frac)
        } else {
            (false, n as f32 / MIN_SAMPLE_SIZE as f32 * 0.3)
        }
    }

    fn evaluate_control_group(&self, control_n: usize, total_n: usize) -> (bool, f32) {
        if total_n == 0 || control_n == 0 {
            return (false, 0.0);
        }
        let ratio = control_n as f32 / total_n as f32;
        if ratio >= MIN_CONTROL_RATIO && ratio <= MAX_CONTROL_RATIO {
            let center_dist = abs_f32(ratio - 0.35) / 0.15;
            (true, 1.0 - center_dist * 0.2)
        } else if ratio > 0.0 {
            (false, (ratio / MIN_CONTROL_RATIO).min(0.6))
        } else {
            (false, 0.0)
        }
    }

    fn evaluate_randomization(&self, assignments: &[bool], _total: usize) -> (bool, f32) {
        if assignments.is_empty() {
            return (false, 0.0);
        }
        let n = assignments.len() as f32;
        let treatment_count = assignments.iter().filter(|&&a| a).count() as f32;
        let expected_ratio = 0.5;
        let actual_ratio = treatment_count / n;
        let deviation = abs_f32(actual_ratio - expected_ratio);

        // Runs test for randomness
        let mut runs = 1u64;
        for i in 1..assignments.len() {
            if assignments[i] != assignments[i - 1] {
                runs += 1;
            }
        }
        let expected_runs = (2.0 * treatment_count * (n - treatment_count)) / n + 1.0;
        let run_deviation = abs_f32(runs as f32 - expected_runs) / expected_runs.max(1.0);

        let balance_score = (1.0 - deviation * 4.0).max(0.0);
        let runs_score = (1.0 - run_deviation).max(0.0);
        let entropy_score = (balance_score * 0.5 + runs_score * 0.5).max(0.0).min(1.0);

        (
            entropy_score >= RANDOMIZATION_ENTROPY_THRESHOLD,
            entropy_score,
        )
    }

    fn evaluate_balance(&self, design: &ExperimentDesign) -> (bool, f32) {
        if design.total_samples == 0 {
            return (false, 0.0);
        }
        let ratio = design.treatment_size as f32 / design.total_samples as f32;
        let deviation = abs_f32(ratio - 0.5);
        let score = (1.0 - deviation * 3.0).max(0.0).min(1.0);
        (deviation < 0.3, score)
    }

    fn evaluate_contamination_risk(&self, design: &ExperimentDesign) -> (bool, f32) {
        // Contamination risk is higher with small sample sizes and poor randomization
        let size_factor = (design.total_samples as f32 / 50.0).min(1.0);
        let separation_score = if design.has_control_group { 0.7 } else { 0.3 };
        let score = (size_factor * 0.5 + separation_score * 0.5).max(0.0).min(1.0);
        (score >= 0.5, score)
    }

    fn evict_oldest(&mut self) {
        let oldest = self
            .experiments
            .values()
            .min_by_key(|e| e.submit_tick)
            .map(|e| e.id);
        if let Some(oid) = oldest {
            self.experiments.remove(&oid);
        }
    }
}
