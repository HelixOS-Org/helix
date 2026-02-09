// SPDX-License-Identifier: GPL-2.0
//! # Apps Methodology — Research Methodology Validation for App Experiments
//!
//! Ensures that all experiments in the app research pipeline follow proper
//! methodology: adequate sample sizes, proper control groups, randomization,
//! bias detection, and statistically valid designs. Grades each experiment
//! and provides improvement advice when methodology falls short.
//!
//! The engine that keeps the research pipeline scientifically rigorous.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_EVALUATIONS: usize = 512;
const MAX_ADVICE_ENTRIES: usize = 256;
const MIN_SAMPLE_SIZE: usize = 30;
const GOOD_SAMPLE_SIZE: usize = 100;
const EXCELLENT_SAMPLE_SIZE: usize = 500;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const BIAS_THRESHOLD: f32 = 0.15;
const GRADE_A_THRESHOLD: f32 = 0.85;
const GRADE_B_THRESHOLD: f32 = 0.70;
const GRADE_C_THRESHOLD: f32 = 0.55;
const MAX_BIAS_TYPES: usize = 8;
const RANDOMIZATION_WEIGHT: f32 = 0.20;
const SAMPLE_WEIGHT: f32 = 0.25;
const CONTROL_WEIGHT: f32 = 0.25;
const BIAS_WEIGHT: f32 = 0.15;
const DOCUMENTATION_WEIGHT: f32 = 0.15;

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

fn abs_f32(x: f32) -> f32 {
    if x < 0.0 { -x } else { x }
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

// ============================================================================
// TYPES
// ============================================================================

/// Letter grade for experiment methodology.
#[derive(Clone, Copy, PartialEq)]
pub enum MethodologyGrade {
    A,
    B,
    C,
    D,
    F,
}

/// Type of detected bias.
#[derive(Clone, Copy, PartialEq)]
pub enum BiasType {
    SelectionBias,
    ConfirmationBias,
    SurvivorshipBias,
    MeasurementBias,
    SamplingBias,
    TemporalBias,
    OrderEffect,
    VolunteerBias,
}

/// Experiment description submitted for methodology validation.
#[derive(Clone)]
pub struct ExperimentDesign {
    pub experiment_id: u64,
    pub title: String,
    pub sample_size_control: usize,
    pub sample_size_treatment: usize,
    pub has_control_group: bool,
    pub randomized: bool,
    pub blinded: bool,
    pub documented: bool,
    pub duration_ticks: u64,
    pub submitted_tick: u64,
}

/// Validation result for an experiment.
#[derive(Clone)]
pub struct ValidationResult {
    pub experiment_id: u64,
    pub grade: MethodologyGrade,
    pub overall_score: f32,
    pub sample_score: f32,
    pub control_score: f32,
    pub randomization_score: f32,
    pub bias_score: f32,
    pub documentation_score: f32,
    pub detected_biases: Vec<BiasType>,
    pub passes_minimum: bool,
}

/// Sample adequacy assessment.
#[derive(Clone)]
pub struct SampleAdequacy {
    pub experiment_id: u64,
    pub control_n: usize,
    pub treatment_n: usize,
    pub adequate: bool,
    pub power_estimate: f32,
    pub recommended_n: usize,
    pub imbalance_ratio: f32,
}

/// Control group design assessment.
#[derive(Clone)]
pub struct ControlDesign {
    pub experiment_id: u64,
    pub has_control: bool,
    pub proper_isolation: bool,
    pub comparable_baseline: bool,
    pub control_quality: f32,
}

/// Bias detection report.
#[derive(Clone)]
pub struct BiasReport {
    pub experiment_id: u64,
    pub biases: Vec<(BiasType, f32)>,
    pub overall_bias_risk: f32,
    pub correctable: bool,
    pub correction_hints: Vec<String>,
}

/// Improvement advice for an experiment.
#[derive(Clone)]
pub struct ImprovementAdvice {
    pub experiment_id: u64,
    pub current_grade: MethodologyGrade,
    pub target_grade: MethodologyGrade,
    pub suggestions: Vec<String>,
    pub priority_action: String,
    pub estimated_improvement: f32,
}

/// Engine-level stats.
#[derive(Clone)]
#[repr(align(64))]
pub struct MethodologyStats {
    pub experiments_evaluated: u64,
    pub grade_a_count: u64,
    pub grade_b_count: u64,
    pub grade_c_count: u64,
    pub grade_d_count: u64,
    pub grade_f_count: u64,
    pub ema_grade_score: f32,
    pub ema_bias_risk: f32,
    pub ema_sample_adequacy: f32,
}

// ============================================================================
// ENGINE
// ============================================================================

/// Research methodology validator for app experiments.
pub struct AppsMethodology {
    evaluations: BTreeMap<u64, ValidationResult>,
    designs: BTreeMap<u64, ExperimentDesign>,
    advice_log: VecDeque<ImprovementAdvice>,
    stats: MethodologyStats,
    rng_state: u64,
    tick: u64,
}

impl AppsMethodology {
    /// Create a new methodology engine.
    pub fn new(seed: u64) -> Self {
        Self {
            evaluations: BTreeMap::new(),
            designs: BTreeMap::new(),
            advice_log: VecDeque::new(),
            stats: MethodologyStats {
                experiments_evaluated: 0,
                grade_a_count: 0,
                grade_b_count: 0,
                grade_c_count: 0,
                grade_d_count: 0,
                grade_f_count: 0,
                ema_grade_score: 0.0,
                ema_bias_risk: 0.0,
                ema_sample_adequacy: 0.0,
            },
            rng_state: seed ^ 0x91d6a3c5ef280b47,
            tick: 0,
        }
    }

    // ── Primary API ────────────────────────────────────────────────────

    /// Validate an experiment's methodology.
    pub fn validate_experiment(
        &mut self,
        title: &str,
        control_n: usize,
        treatment_n: usize,
        has_control: bool,
        randomized: bool,
        blinded: bool,
        documented: bool,
        duration: u64,
    ) -> ValidationResult {
        self.tick += 1;
        self.stats.experiments_evaluated += 1;

        let id = fnv1a_hash(title.as_bytes()) ^ self.tick;

        let design = ExperimentDesign {
            experiment_id: id,
            title: String::from(title),
            sample_size_control: control_n,
            sample_size_treatment: treatment_n,
            has_control_group: has_control,
            randomized,
            blinded,
            documented,
            duration_ticks: duration,
            submitted_tick: self.tick,
        };

        // Compute component scores
        let sample_sc = self.score_sample(control_n, treatment_n);
        let control_sc = self.score_control(has_control, control_n, treatment_n);
        let rand_sc = if randomized { 1.0 } else { 0.3 };
        let doc_sc = if documented { 1.0 } else { 0.2 };

        // Bias detection
        let biases = self.detect_biases(&design);
        let bias_risk: f32 = if biases.is_empty() {
            0.0
        } else {
            biases.iter().map(|&(_, s)| s).sum::<f32>() / biases.len() as f32
        };
        let bias_sc = 1.0 - bias_risk;

        let overall = sample_sc * SAMPLE_WEIGHT
            + control_sc * CONTROL_WEIGHT
            + rand_sc * RANDOMIZATION_WEIGHT
            + bias_sc * BIAS_WEIGHT
            + doc_sc * DOCUMENTATION_WEIGHT;

        let grade = if overall >= GRADE_A_THRESHOLD {
            MethodologyGrade::A
        } else if overall >= GRADE_B_THRESHOLD {
            MethodologyGrade::B
        } else if overall >= GRADE_C_THRESHOLD {
            MethodologyGrade::C
        } else if overall >= 0.35 {
            MethodologyGrade::D
        } else {
            MethodologyGrade::F
        };

        self.update_grade_counts(grade);
        self.stats.ema_grade_score =
            EMA_ALPHA * overall + (1.0 - EMA_ALPHA) * self.stats.ema_grade_score;
        self.stats.ema_bias_risk =
            EMA_ALPHA * bias_risk + (1.0 - EMA_ALPHA) * self.stats.ema_bias_risk;

        let bias_types: Vec<BiasType> = biases.iter().map(|&(bt, _)| bt).collect();

        let result = ValidationResult {
            experiment_id: id,
            grade,
            overall_score: overall,
            sample_score: sample_sc,
            control_score: control_sc,
            randomization_score: rand_sc,
            bias_score: bias_sc,
            documentation_score: doc_sc,
            detected_biases: bias_types,
            passes_minimum: overall >= GRADE_C_THRESHOLD,
        };

        if self.evaluations.len() >= MAX_EVALUATIONS {
            if let Some(oldest) = self.evaluations.keys().next().cloned() {
                self.evaluations.remove(&oldest);
            }
        }
        self.evaluations.insert(id, result.clone());
        self.designs.insert(id, design);
        result
    }

    /// Assess sample adequacy for an experiment.
    #[inline]
    pub fn sample_adequacy(&mut self, experiment_id: u64) -> Option<SampleAdequacy> {
        let design = self.designs.get(&experiment_id)?;
        let c_n = design.sample_size_control;
        let t_n = design.sample_size_treatment;
        let total = c_n + t_n;
        let adequate = c_n >= MIN_SAMPLE_SIZE && t_n >= MIN_SAMPLE_SIZE;

        let imbalance = if c_n > 0 && t_n > 0 {
            let ratio = c_n as f32 / t_n as f32;
            abs_f32(ratio - 1.0)
        } else {
            1.0
        };

        let power_est = if total >= EXCELLENT_SAMPLE_SIZE {
            0.95
        } else if total >= GOOD_SAMPLE_SIZE {
            0.80
        } else if total >= MIN_SAMPLE_SIZE * 2 {
            0.60
        } else {
            0.30
        };

        let recommended = GOOD_SAMPLE_SIZE.max(c_n).max(t_n);

        self.stats.ema_sample_adequacy =
            EMA_ALPHA * power_est + (1.0 - EMA_ALPHA) * self.stats.ema_sample_adequacy;

        Some(SampleAdequacy {
            experiment_id,
            control_n: c_n,
            treatment_n: t_n,
            adequate,
            power_estimate: power_est,
            recommended_n: recommended,
            imbalance_ratio: imbalance,
        })
    }

    /// Assess control group design quality.
    pub fn control_design(&self, experiment_id: u64) -> Option<ControlDesign> {
        let design = self.designs.get(&experiment_id)?;
        let has = design.has_control_group;
        let isolation = has && design.randomized;
        let comparable = has && {
            let ratio = design.sample_size_control as f32
                / design.sample_size_treatment.max(1) as f32;
            ratio >= 0.5 && ratio <= 2.0
        };
        let quality = if has { 0.4 } else { 0.0 }
            + if isolation { 0.3 } else { 0.0 }
            + if comparable { 0.3 } else { 0.0 };

        Some(ControlDesign {
            experiment_id,
            has_control: has,
            proper_isolation: isolation,
            comparable_baseline: comparable,
            control_quality: quality,
        })
    }

    /// Detect potential biases in an experiment.
    pub fn bias_detection(&self, experiment_id: u64) -> Option<BiasReport> {
        let design = self.designs.get(&experiment_id)?;
        let biases = self.detect_biases(design);
        let overall_risk: f32 = if biases.is_empty() {
            0.0
        } else {
            biases.iter().map(|&(_, s)| s).sum::<f32>() / biases.len() as f32
        };

        let mut hints = Vec::new();
        for &(bt, severity) in &biases {
            if severity > BIAS_THRESHOLD {
                let hint = match bt {
                    BiasType::SelectionBias => String::from("Randomize participant selection"),
                    BiasType::SamplingBias => String::from("Increase sample size and diversity"),
                    BiasType::TemporalBias => String::from("Extend experiment duration"),
                    BiasType::MeasurementBias => String::from("Use blinded measurement"),
                    BiasType::OrderEffect => String::from("Counterbalance treatment ordering"),
                    BiasType::SurvivorshipBias => String::from("Track all participants including dropouts"),
                    BiasType::ConfirmationBias => String::from("Pre-register hypotheses and analysis plan"),
                    BiasType::VolunteerBias => String::from("Use mandatory sampling where possible"),
                };
                hints.push(hint);
            }
        }

        let correctable = overall_risk < 0.6;

        Some(BiasReport {
            experiment_id,
            biases,
            overall_bias_risk: overall_risk,
            correctable,
            correction_hints: hints,
        })
    }

    /// Get the methodology grade for a previously evaluated experiment.
    #[inline(always)]
    pub fn methodology_grade(&self, experiment_id: u64) -> Option<MethodologyGrade> {
        self.evaluations.get(&experiment_id).map(|v| v.grade)
    }

    /// Generate improvement advice for an experiment.
    pub fn improvement_advice(&mut self, experiment_id: u64) -> Option<ImprovementAdvice> {
        let eval = self.evaluations.get(&experiment_id)?.clone();
        let current = eval.grade;
        let target = match current {
            MethodologyGrade::A => MethodologyGrade::A,
            MethodologyGrade::B => MethodologyGrade::A,
            MethodologyGrade::C => MethodologyGrade::B,
            MethodologyGrade::D => MethodologyGrade::C,
            MethodologyGrade::F => MethodologyGrade::D,
        };

        let mut suggestions = Vec::new();
        let mut priority = String::from("No changes needed");
        let mut est_improvement = 0.0f32;

        if eval.sample_score < 0.7 {
            suggestions.push(String::from("Increase sample size to at least 100 per group"));
            priority = String::from("Increase sample size");
            est_improvement += 0.15;
        }
        if eval.control_score < 0.7 {
            suggestions.push(String::from("Add proper control group with baseline measurement"));
            if est_improvement < 0.05 {
                priority = String::from("Add control group");
            }
            est_improvement += 0.12;
        }
        if eval.randomization_score < 0.7 {
            suggestions.push(String::from("Randomize treatment assignment"));
            est_improvement += 0.10;
        }
        if eval.bias_score < 0.7 {
            suggestions.push(String::from("Address detected biases before proceeding"));
            est_improvement += 0.10;
        }
        if eval.documentation_score < 0.7 {
            suggestions.push(String::from("Document hypothesis, methods, and analysis plan"));
            est_improvement += 0.08;
        }

        let advice = ImprovementAdvice {
            experiment_id,
            current_grade: current,
            target_grade: target,
            suggestions,
            priority_action: priority,
            estimated_improvement: est_improvement.min(0.5),
        };

        if self.advice_log.len() >= MAX_ADVICE_ENTRIES {
            self.advice_log.pop_front();
        }
        self.advice_log.push_back(advice.clone());
        Some(advice)
    }

    /// Return engine stats.
    #[inline(always)]
    pub fn stats(&self) -> &MethodologyStats {
        &self.stats
    }

    // ── Internal Helpers ───────────────────────────────────────────────

    fn score_sample(&self, control_n: usize, treatment_n: usize) -> f32 {
        let min_n = control_n.min(treatment_n);
        if min_n >= EXCELLENT_SAMPLE_SIZE {
            1.0
        } else if min_n >= GOOD_SAMPLE_SIZE {
            0.85
        } else if min_n >= MIN_SAMPLE_SIZE {
            0.60
        } else if min_n >= 10 {
            0.30
        } else {
            0.10
        }
    }

    fn score_control(&self, has_control: bool, control_n: usize, treatment_n: usize) -> f32 {
        if !has_control {
            return 0.1;
        }
        let balance = if treatment_n > 0 {
            let ratio = control_n as f32 / treatment_n as f32;
            1.0 - abs_f32(ratio - 1.0).min(1.0)
        } else {
            0.0
        };
        0.5 + balance * 0.5
    }

    fn detect_biases(&self, design: &ExperimentDesign) -> Vec<(BiasType, f32)> {
        let mut biases = Vec::new();

        if !design.randomized {
            biases.push((BiasType::SelectionBias, 0.5));
        }
        if design.sample_size_control < MIN_SAMPLE_SIZE || design.sample_size_treatment < MIN_SAMPLE_SIZE {
            biases.push((BiasType::SamplingBias, 0.4));
        }
        if design.duration_ticks < 100 {
            biases.push((BiasType::TemporalBias, 0.3));
        }
        if !design.blinded {
            biases.push((BiasType::MeasurementBias, 0.25));
            biases.push((BiasType::ConfirmationBias, 0.20));
        }
        let imbalance = if design.sample_size_treatment > 0 {
            abs_f32(design.sample_size_control as f32 / design.sample_size_treatment as f32 - 1.0)
        } else {
            1.0
        };
        if imbalance > 0.5 {
            biases.push((BiasType::VolunteerBias, imbalance.min(1.0) * 0.3));
        }

        biases
    }

    fn update_grade_counts(&mut self, grade: MethodologyGrade) {
        match grade {
            MethodologyGrade::A => self.stats.grade_a_count += 1,
            MethodologyGrade::B => self.stats.grade_b_count += 1,
            MethodologyGrade::C => self.stats.grade_c_count += 1,
            MethodologyGrade::D => self.stats.grade_d_count += 1,
            MethodologyGrade::F => self.stats.grade_f_count += 1,
        }
    }
}
