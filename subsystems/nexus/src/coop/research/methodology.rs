// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Methodology — Research Methodology for Cooperation Experiments
//!
//! Enforces rigorous experimental methodology for all cooperation research.
//! Designs controlled experiments with proper control groups for fairness
//! testing, implements randomization for resource sharing experiments, and
//! validates sample adequacy before drawing conclusions. Audits existing
//! methodology for flaws and suggests design improvements. This ensures that
//! cooperation "discoveries" are genuinely caused by the intervention, not
//! confounded by environmental changes or selection bias.
//!
//! The engine that keeps cooperation experiments scientifically valid.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_EXPERIMENTS: usize = 256;
const MAX_CONTROL_GROUPS: usize = 32;
const MIN_SAMPLE_SIZE: usize = 10;
const RECOMMENDED_SAMPLE_SIZE: usize = 30;
const MAX_RANDOMIZATION_SEED_POOL: usize = 64;
const ADEQUACY_THRESHOLD: f32 = 0.80;
const RANDOMIZATION_CHECK_THRESHOLD: f32 = 0.05;
const EMA_ALPHA: f32 = 0.12;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const AUDIT_PASS_THRESHOLD: f32 = 0.70;
const BALANCE_TOLERANCE: f32 = 0.15;
const MAX_AUDIT_FINDINGS: usize = 64;
const DESIGN_SCORE_WEIGHTS: [f32; 4] = [0.30, 0.25, 0.25, 0.20];

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
// METHODOLOGY TYPES
// ============================================================================

/// Type of cooperation experiment design
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExperimentDesign {
    RandomizedControlled,
    QuasiExperimental,
    ABTesting,
    CrossoverDesign,
    FactorialDesign,
    WithinSubjects,
}

/// Aspect of cooperation being tested
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoopTestAspect {
    FairnessPolicy,
    SharingAlgorithm,
    TrustMechanism,
    NegotiationStrategy,
    ContentionHandling,
    AuctionRules,
    CoalitionPolicy,
}

/// Validity threat to an experiment
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidityThreat {
    SelectionBias,
    ConfoundingVariable,
    InsufficientSample,
    NoControlGroup,
    PoorRandomization,
    MeasurementBias,
    HistoryEffect,
}

/// A control group specification
#[derive(Debug, Clone)]
pub struct ControlGroup {
    pub id: u64,
    pub name: String,
    pub is_control: bool,
    pub assigned_count: usize,
    pub mean_baseline: f32,
    pub variance_baseline: f32,
    pub balanced: bool,
}

/// Experiment validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub experiment_id: u64,
    pub valid: bool,
    pub design_score: f32,
    pub randomization_quality: f32,
    pub sample_adequacy: f32,
    pub control_group_quality: f32,
    pub threats: Vec<ValidityThreat>,
    pub recommendations: Vec<String>,
}

/// An audit finding about methodology
#[derive(Debug, Clone)]
pub struct AuditFinding {
    pub id: u64,
    pub experiment_id: u64,
    pub threat: ValidityThreat,
    pub severity: f32,
    pub description: String,
    pub remediation: String,
}

/// Experiment definition being validated
#[derive(Debug, Clone)]
pub struct CoopExperimentDef {
    pub id: u64,
    pub design: ExperimentDesign,
    pub aspect: CoopTestAspect,
    pub control_groups: Vec<ControlGroup>,
    pub sample_size: usize,
    pub randomization_seed: u64,
    pub created_tick: u64,
    pub validation: Option<ValidationResult>,
}

/// Design improvement suggestion
#[derive(Debug, Clone)]
pub struct DesignImprovement {
    pub experiment_id: u64,
    pub current_score: f32,
    pub projected_score: f32,
    pub suggestion: String,
    pub priority: f32,
}

// ============================================================================
// METHODOLOGY STATS
// ============================================================================

/// Aggregate statistics for the methodology engine
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct MethodologyStats {
    pub total_validations: u64,
    pub passed_validations: u64,
    pub failed_validations: u64,
    pub total_audits: u64,
    pub threats_detected: u64,
    pub avg_design_score_ema: f32,
    pub avg_sample_adequacy_ema: f32,
    pub avg_randomization_quality_ema: f32,
    pub improvements_suggested: u64,
    pub experiments_redesigned: u64,
}

// ============================================================================
// COOPERATION METHODOLOGY
// ============================================================================

/// Methodology engine for cooperation experiments
#[derive(Debug)]
pub struct CoopMethodology {
    experiments: VecDeque<CoopExperimentDef>,
    audit_findings: Vec<AuditFinding>,
    randomization_pool: Vec<u64>,
    design_history: LinearMap<f32, 64>,
    rng_state: u64,
    tick: u64,
    stats: MethodologyStats,
}

impl CoopMethodology {
    /// Create a new methodology engine with the given PRNG seed
    pub fn new(seed: u64) -> Self {
        let mut rng = seed | 1;
        let mut pool = Vec::with_capacity(MAX_RANDOMIZATION_SEED_POOL);
        for _ in 0..MAX_RANDOMIZATION_SEED_POOL {
            pool.push(xorshift64(&mut rng));
        }
        Self {
            experiments: VecDeque::new(),
            audit_findings: Vec::new(),
            randomization_pool: pool,
            design_history: LinearMap::new(),
            rng_state: rng,
            tick: 0,
            stats: MethodologyStats::default(),
        }
    }

    /// Validate a cooperation experiment for methodological soundness
    pub fn validate_coop_experiment(
        &mut self,
        design: ExperimentDesign,
        aspect: CoopTestAspect,
        sample_size: usize,
        control_groups: Vec<ControlGroup>,
    ) -> ValidationResult {
        self.tick += 1;
        let exp_id = fnv1a_hash(&self.tick.to_le_bytes()) ^ xorshift64(&mut self.rng_state);

        let randomization_quality = self.assess_randomization(&control_groups);
        let sample_adequacy = self.assess_sample_adequacy(sample_size, design);
        let control_quality = self.assess_control_groups(&control_groups);
        let mut threats: Vec<ValidityThreat> = Vec::new();
        let mut recommendations: Vec<String> = Vec::new();

        // Check for validity threats
        if sample_size < MIN_SAMPLE_SIZE {
            threats.push(ValidityThreat::InsufficientSample);
            let mut rec = String::from("Increase sample to at least ");
            let needed = RECOMMENDED_SAMPLE_SIZE;
            let digit_tens = (needed / 10) as u8 + b'0';
            let digit_ones = (needed % 10) as u8 + b'0';
            rec.push(digit_tens as char);
            rec.push(digit_ones as char);
            recommendations.push(rec);
        }
        if control_groups.is_empty() {
            threats.push(ValidityThreat::NoControlGroup);
            recommendations.push(String::from("Add a control group with baseline behavior"));
        }
        let has_control = control_groups.iter().any(|g| g.is_control);
        if !has_control && !control_groups.is_empty() {
            threats.push(ValidityThreat::NoControlGroup);
            recommendations.push(String::from("Designate at least one group as control"));
        }
        if randomization_quality < RANDOMIZATION_CHECK_THRESHOLD * 10.0 {
            threats.push(ValidityThreat::PoorRandomization);
            recommendations.push(String::from("Improve randomization of group assignment"));
        }
        if !self.check_group_balance(&control_groups) {
            threats.push(ValidityThreat::SelectionBias);
            recommendations.push(String::from("Rebalance group sizes to within tolerance"));
        }

        let design_score = DESIGN_SCORE_WEIGHTS[0] * randomization_quality
            + DESIGN_SCORE_WEIGHTS[1] * sample_adequacy
            + DESIGN_SCORE_WEIGHTS[2] * control_quality
            + DESIGN_SCORE_WEIGHTS[3] * (1.0 - threats.len() as f32 / 7.0).max(0.0);
        let valid = design_score >= AUDIT_PASS_THRESHOLD && threats.is_empty();

        let result = ValidationResult {
            experiment_id: exp_id,
            valid,
            design_score,
            randomization_quality,
            sample_adequacy,
            control_group_quality: control_quality,
            threats,
            recommendations,
        };

        let seed_for_exp = self
            .randomization_pool
            .get(self.experiments.len() % MAX_RANDOMIZATION_SEED_POOL)
            .copied()
            .unwrap_or(xorshift64(&mut self.rng_state));
        let exp = CoopExperimentDef {
            id: exp_id,
            design,
            aspect,
            control_groups,
            sample_size,
            randomization_seed: seed_for_exp,
            created_tick: self.tick,
            validation: Some(result.clone()),
        };
        if self.experiments.len() >= MAX_EXPERIMENTS {
            self.experiments.pop_front();
        }
        self.experiments.push_back(exp);
        self.design_history.insert(exp_id, design_score);

        self.stats.total_validations += 1;
        if valid {
            self.stats.passed_validations += 1;
        } else {
            self.stats.failed_validations += 1;
        }
        self.stats.avg_design_score_ema =
            EMA_ALPHA * design_score + (1.0 - EMA_ALPHA) * self.stats.avg_design_score_ema;
        self.stats.avg_sample_adequacy_ema =
            EMA_ALPHA * sample_adequacy + (1.0 - EMA_ALPHA) * self.stats.avg_sample_adequacy_ema;
        self.stats.avg_randomization_quality_ema =
            EMA_ALPHA * randomization_quality
                + (1.0 - EMA_ALPHA) * self.stats.avg_randomization_quality_ema;
        result
    }

    /// Create a fairness-specific control group
    pub fn fairness_control_group(
        &mut self,
        baseline_fairness: f32,
        group_size: usize,
    ) -> ControlGroup {
        self.tick += 1;
        let id = fnv1a_hash(&self.tick.to_le_bytes()) ^ xorshift64(&mut self.rng_state);
        // Simulate baseline variance
        let variance = xorshift_f32(&mut self.rng_state) * 0.1;
        ControlGroup {
            id,
            name: String::from("Fairness control"),
            is_control: true,
            assigned_count: group_size,
            mean_baseline: baseline_fairness,
            variance_baseline: variance,
            balanced: group_size >= MIN_SAMPLE_SIZE,
        }
    }

    /// Generate randomization for a sharing experiment
    pub fn sharing_randomization(
        &mut self,
        participant_count: usize,
        group_count: usize,
    ) -> Vec<Vec<usize>> {
        self.tick += 1;
        if group_count == 0 {
            return Vec::new();
        }
        let mut assignments: Vec<Vec<usize>> = Vec::with_capacity(group_count);
        for _ in 0..group_count {
            assignments.push(Vec::new());
        }
        // Fisher-Yates-style assignment
        let mut order: Vec<usize> = Vec::with_capacity(participant_count);
        for i in 0..participant_count {
            order.push(i);
        }
        // Shuffle using xorshift
        for i in (1..order.len()).rev() {
            let j = (xorshift64(&mut self.rng_state) as usize) % (i + 1);
            order.swap(i, j);
        }
        // Round-robin assignment
        for (idx, &participant) in order.iter().enumerate() {
            let group = idx % group_count;
            assignments[group].push(participant);
        }
        assignments
    }

    /// Check if the sample size is adequate for the target effect size
    pub fn sample_adequacy(&self, sample_size: usize, target_effect: f32) -> f32 {
        if target_effect <= 0.0 {
            return 0.0;
        }
        let z_alpha = 1.96f32;
        let z_beta = 0.84f32;
        let required = ((z_alpha + z_beta) / target_effect) * ((z_alpha + z_beta) / target_effect) * 2.0;
        let required_n = required.ceil() as usize;
        if sample_size >= required_n {
            1.0
        } else {
            sample_size as f32 / required_n as f32
        }
    }

    /// Audit the methodology of existing experiments
    pub fn methodology_audit(&mut self) -> Vec<AuditFinding> {
        self.tick += 1;
        self.stats.total_audits += 1;
        let mut findings: Vec<AuditFinding> = Vec::new();

        for exp in &self.experiments {
            let validation = match &exp.validation {
                Some(v) => v,
                None => continue,
            };
            for threat in &validation.threats {
                let severity = match threat {
                    ValidityThreat::InsufficientSample => 0.8,
                    ValidityThreat::NoControlGroup => 0.9,
                    ValidityThreat::SelectionBias => 0.7,
                    ValidityThreat::PoorRandomization => 0.6,
                    ValidityThreat::ConfoundingVariable => 0.75,
                    ValidityThreat::MeasurementBias => 0.5,
                    ValidityThreat::HistoryEffect => 0.4,
                };
                let desc = match threat {
                    ValidityThreat::InsufficientSample => String::from("Sample size below minimum"),
                    ValidityThreat::NoControlGroup => String::from("No control group present"),
                    ValidityThreat::SelectionBias => String::from("Group selection may be biased"),
                    ValidityThreat::PoorRandomization => String::from("Randomization quality is low"),
                    ValidityThreat::ConfoundingVariable => String::from("Potential confounding variables"),
                    ValidityThreat::MeasurementBias => String::from("Measurement may be biased"),
                    ValidityThreat::HistoryEffect => String::from("Historical effects not controlled"),
                };
                let remediation = match threat {
                    ValidityThreat::InsufficientSample => String::from("Collect more data points"),
                    ValidityThreat::NoControlGroup => String::from("Add a baseline control group"),
                    ValidityThreat::SelectionBias => String::from("Use stratified random assignment"),
                    ValidityThreat::PoorRandomization => String::from("Use better PRNG and verify balance"),
                    ValidityThreat::ConfoundingVariable => String::from("Add covariates or blocking"),
                    ValidityThreat::MeasurementBias => String::from("Calibrate measurement instruments"),
                    ValidityThreat::HistoryEffect => String::from("Add time-series control"),
                };
                let finding_id = fnv1a_hash(&exp.id.to_le_bytes()) ^ fnv1a_hash(&(*threat as u64).to_le_bytes());
                findings.push(AuditFinding {
                    id: finding_id,
                    experiment_id: exp.id,
                    threat: *threat,
                    severity,
                    description: desc,
                    remediation,
                });
                self.stats.threats_detected += 1;
            }
        }
        if findings.len() > MAX_AUDIT_FINDINGS {
            findings.truncate(MAX_AUDIT_FINDINGS);
        }
        self.audit_findings = findings.clone();
        findings
    }

    /// Suggest design improvements for a specific experiment
    pub fn design_improvement(&mut self, experiment_id: u64) -> Vec<DesignImprovement> {
        self.tick += 1;
        let exp = match self.experiments.iter().find(|e| e.id == experiment_id) {
            Some(e) => e,
            None => return Vec::new(),
        };
        let current_score = exp
            .validation
            .as_ref()
            .map(|v| v.design_score)
            .unwrap_or(0.0);
        let mut improvements: Vec<DesignImprovement> = Vec::new();

        if exp.sample_size < RECOMMENDED_SAMPLE_SIZE {
            let projected = current_score + 0.15;
            improvements.push(DesignImprovement {
                experiment_id,
                current_score,
                projected_score: projected.min(1.0),
                suggestion: String::from("Increase sample size for higher power"),
                priority: 0.9,
            });
        }
        if !exp.control_groups.iter().any(|g| g.is_control) {
            let projected = current_score + 0.20;
            improvements.push(DesignImprovement {
                experiment_id,
                current_score,
                projected_score: projected.min(1.0),
                suggestion: String::from("Add a proper control group"),
                priority: 1.0,
            });
        }
        if exp.control_groups.len() < 2 {
            let projected = current_score + 0.10;
            improvements.push(DesignImprovement {
                experiment_id,
                current_score,
                projected_score: projected.min(1.0),
                suggestion: String::from("Add additional treatment groups for comparison"),
                priority: 0.6,
            });
        }
        if exp.design == ExperimentDesign::QuasiExperimental {
            let projected = current_score + 0.12;
            improvements.push(DesignImprovement {
                experiment_id,
                current_score,
                projected_score: projected.min(1.0),
                suggestion: String::from("Upgrade to randomized controlled design"),
                priority: 0.8,
            });
        }
        self.stats.improvements_suggested += improvements.len() as u64;
        improvements.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(core::cmp::Ordering::Equal));
        improvements
    }

    /// Get current methodology statistics
    #[inline(always)]
    pub fn stats(&self) -> &MethodologyStats {
        &self.stats
    }

    /// Number of experiments tracked
    #[inline(always)]
    pub fn experiment_count(&self) -> usize {
        self.experiments.len()
    }

    // ========================================================================
    // INTERNAL HELPERS
    // ========================================================================

    fn assess_randomization(&self, groups: &[ControlGroup]) -> f32 {
        if groups.len() < 2 {
            return 0.5;
        }
        let total_assigned: usize = groups.iter().map(|g| g.assigned_count).sum();
        if total_assigned == 0 {
            return 0.0;
        }
        let expected_per_group = total_assigned as f32 / groups.len() as f32;
        let mut imbalance = 0.0f32;
        for group in groups {
            let diff = (group.assigned_count as f32 - expected_per_group).abs();
            imbalance += diff / expected_per_group;
        }
        let avg_imbalance = imbalance / groups.len() as f32;
        (1.0 - avg_imbalance).max(0.0).min(1.0)
    }

    fn assess_sample_adequacy(&self, sample_size: usize, design: ExperimentDesign) -> f32 {
        let min_needed = match design {
            ExperimentDesign::RandomizedControlled => RECOMMENDED_SAMPLE_SIZE,
            ExperimentDesign::ABTesting => RECOMMENDED_SAMPLE_SIZE * 2,
            ExperimentDesign::CrossoverDesign => MIN_SAMPLE_SIZE * 2,
            ExperimentDesign::FactorialDesign => RECOMMENDED_SAMPLE_SIZE * 3,
            _ => RECOMMENDED_SAMPLE_SIZE,
        };
        if sample_size >= min_needed {
            1.0
        } else {
            sample_size as f32 / min_needed as f32
        }
    }

    fn assess_control_groups(&self, groups: &[ControlGroup]) -> f32 {
        if groups.is_empty() {
            return 0.0;
        }
        let has_control = groups.iter().any(|g| g.is_control) as u32 as f32;
        let balanced_ratio = groups.iter().filter(|g| g.balanced).count() as f32 / groups.len() as f32;
        has_control * 0.5 + balanced_ratio * 0.5
    }

    fn check_group_balance(&self, groups: &[ControlGroup]) -> bool {
        if groups.len() < 2 {
            return true;
        }
        let sizes: Vec<usize> = groups.iter().map(|g| g.assigned_count).collect();
        let max_size = sizes.iter().copied().max().unwrap_or(0);
        let min_size = sizes.iter().copied().min().unwrap_or(0);
        if max_size == 0 {
            return true;
        }
        let ratio = (max_size - min_size) as f32 / max_size as f32;
        ratio <= BALANCE_TOLERANCE
    }
}
