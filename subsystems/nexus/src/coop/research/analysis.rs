// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Analysis Engine — Statistical Analysis of Cooperation Research
//!
//! Performs rigorous statistical analysis of cooperation research outcomes.
//! Analyzes fairness improvements, trust dynamics, and contention reduction
//! across all cooperation experiments. Computes effect sizes (Cohen's d) for
//! cooperation policy changes, runs significance tests, and generates
//! comprehensive summary reports. Every analysis feeds back into the
//! cooperation research loop, informing future hypothesis generation and
//! experiment design with hard quantitative evidence.
//!
//! The engine that measures whether cooperation is actually improving.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_ANALYSES: usize = 512;
const MAX_DATA_POINTS: usize = 1024;
const MIN_SAMPLE_SIZE: usize = 8;
const SIGNIFICANCE_LEVEL: f32 = 0.05;
const SMALL_EFFECT: f32 = 0.20;
const MEDIUM_EFFECT: f32 = 0.50;
const LARGE_EFFECT: f32 = 0.80;
const EMA_ALPHA: f32 = 0.12;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const POWER_THRESHOLD: f32 = 0.80;
const FAIRNESS_IMPROVEMENT_MIN: f32 = 0.02;
const TRUST_SIGNIFICANCE_MIN: f32 = 0.05;
const CONTENTION_REDUCTION_MIN: f32 = 0.03;
const MAX_REPORT_ENTRIES: usize = 64;
const DECAY_RATE: f32 = 0.005;

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
// ANALYSIS TYPES
// ============================================================================

/// Domain of cooperation being analyzed
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoopAnalysisDomain {
    FairnessImprovement,
    TrustDynamics,
    ContentionReduction,
    NegotiationEfficiency,
    ResourceSharingEquity,
    ConflictResolutionSpeed,
    AuctionOutcomeBalance,
}

/// Classification of effect size
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EffectMagnitude {
    Negligible,
    Small,
    Medium,
    Large,
    VeryLarge,
}

/// A single data point in the analysis
#[derive(Debug, Clone)]
pub struct AnalysisDataPoint {
    pub tick: u64,
    pub value: f32,
    pub group_id: u64,
    pub domain: CoopAnalysisDomain,
    pub metadata_hash: u64,
}

/// Result of an effect size computation
#[derive(Debug, Clone)]
pub struct EffectSizeResult {
    pub analysis_id: u64,
    pub cohens_d: f32,
    pub magnitude: EffectMagnitude,
    pub control_mean: f32,
    pub treatment_mean: f32,
    pub pooled_std: f32,
    pub domain: CoopAnalysisDomain,
    pub significant: bool,
}

/// Result of a significance test
#[derive(Debug, Clone)]
pub struct SignificanceResult {
    pub analysis_id: u64,
    pub t_statistic: f32,
    pub degrees_of_freedom: f32,
    pub p_value_approx: f32,
    pub significant: bool,
    pub domain: CoopAnalysisDomain,
}

/// Power analysis result
#[derive(Debug, Clone)]
pub struct PowerResult {
    pub required_sample_size: usize,
    pub current_power: f32,
    pub target_power: f32,
    pub target_effect_size: f32,
    pub adequate: bool,
}

/// A complete analysis record
#[derive(Debug, Clone)]
pub struct CoopAnalysisRecord {
    pub id: u64,
    pub domain: CoopAnalysisDomain,
    pub tick: u64,
    pub control_data: Vec<f32>,
    pub treatment_data: Vec<f32>,
    pub effect_size: Option<EffectSizeResult>,
    pub significance: Option<SignificanceResult>,
    pub power: Option<PowerResult>,
    pub conclusion: String,
}

/// Summary report entry
#[derive(Debug, Clone)]
pub struct ReportEntry {
    pub domain: CoopAnalysisDomain,
    pub total_analyses: u64,
    pub significant_count: u64,
    pub avg_effect_size: f32,
    pub largest_effect: f32,
    pub overall_trend: f32,
}

// ============================================================================
// ANALYSIS STATS
// ============================================================================

/// Aggregate statistics for the analysis engine
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct AnalysisStats {
    pub total_analyses: u64,
    pub significant_findings: u64,
    pub fairness_analyses: u64,
    pub trust_analyses: u64,
    pub contention_analyses: u64,
    pub avg_effect_size_ema: f32,
    pub power_adequate_ratio: f32,
    pub reports_generated: u64,
    pub mean_sample_size_ema: f32,
    pub strongest_effect_ever: f32,
}

// ============================================================================
// COOPERATION ANALYSIS ENGINE
// ============================================================================

/// Statistical analysis engine for cooperation research findings
#[derive(Debug)]
pub struct CoopAnalysisEngine {
    analyses: VecDeque<CoopAnalysisRecord>,
    domain_history: BTreeMap<u64, Vec<f32>>,
    report_cache: Vec<ReportEntry>,
    effect_ema: LinearMap<f32, 64>,
    rng_state: u64,
    tick: u64,
    stats: AnalysisStats,
}

impl CoopAnalysisEngine {
    /// Create a new cooperation analysis engine with the given PRNG seed
    pub fn new(seed: u64) -> Self {
        Self {
            analyses: VecDeque::new(),
            domain_history: BTreeMap::new(),
            report_cache: Vec::new(),
            effect_ema: LinearMap::new(),
            rng_state: seed | 1,
            tick: 0,
            stats: AnalysisStats::default(),
        }
    }

    /// Run a full cooperation analysis comparing control vs treatment data
    #[inline]
    pub fn analyze_cooperation(
        &mut self,
        domain: CoopAnalysisDomain,
        control: &[f32],
        treatment: &[f32],
    ) -> Option<CoopAnalysisRecord> {
        self.tick += 1;
        if control.len() < MIN_SAMPLE_SIZE || treatment.len() < MIN_SAMPLE_SIZE {
            return None;
        }
        let id = fnv1a_hash(&self.tick.to_le_bytes()) ^ xorshift64(&mut self.rng_state);

        let ctrl_mean = self.compute_mean(control);
        let treat_mean = self.compute_mean(treatment);
        let ctrl_var = self.compute_variance(control, ctrl_mean);
        let treat_var = self.compute_variance(treatment, treat_mean);

        let effect = self.compute_effect_size(id, domain, ctrl_mean, treat_mean, ctrl_var, treat_var, control.len(), treatment.len());
        let sig = self.compute_significance(id, domain, ctrl_mean, treat_mean, ctrl_var, treat_var, control.len(), treatment.len());
        let power = self.compute_power(effect.cohens_d, control.len(), treatment.len());

        let conclusion = self.form_conclusion(&effect, &sig, &power);

        let domain_key = domain as u64;
        let hist = self.domain_history.entry(domain_key).or_insert_with(Vec::new);
        hist.push(effect.cohens_d);
        if hist.len() > MAX_DATA_POINTS {
            hist.pop_front();
        }

        let prev_ema = self.effect_ema.get(domain_key).copied().unwrap_or(0.0);
        let new_ema = EMA_ALPHA * effect.cohens_d.abs() + (1.0 - EMA_ALPHA) * prev_ema;
        self.effect_ema.insert(domain_key, new_ema);

        let record = CoopAnalysisRecord {
            id,
            domain,
            tick: self.tick,
            control_data: control.to_vec(),
            treatment_data: treatment.to_vec(),
            effect_size: Some(effect),
            significance: Some(sig),
            power: Some(power),
            conclusion,
        };

        self.stats.total_analyses += 1;
        if record.significance.as_ref().map_or(false, |s| s.significant) {
            self.stats.significant_findings += 1;
        }
        match domain {
            CoopAnalysisDomain::FairnessImprovement => self.stats.fairness_analyses += 1,
            CoopAnalysisDomain::TrustDynamics => self.stats.trust_analyses += 1,
            CoopAnalysisDomain::ContentionReduction => self.stats.contention_analyses += 1,
            _ => {}
        }
        self.stats.avg_effect_size_ema = new_ema;
        let sample_total = (control.len() + treatment.len()) as f32;
        self.stats.mean_sample_size_ema =
            EMA_ALPHA * sample_total + (1.0 - EMA_ALPHA) * self.stats.mean_sample_size_ema;
        if record.effect_size.as_ref().map_or(0.0, |e| e.cohens_d.abs()) > self.stats.strongest_effect_ever {
            self.stats.strongest_effect_ever = record.effect_size.as_ref().map_or(0.0, |e| e.cohens_d.abs());
        }

        if self.analyses.len() >= MAX_ANALYSES {
            self.analyses.pop_front();
        }
        self.analyses.push_back(record.clone());
        Some(record)
    }

    /// Compute fairness-specific effect size for a policy change
    pub fn fairness_effect_size(
        &mut self,
        before_fairness: &[f32],
        after_fairness: &[f32],
    ) -> Option<EffectSizeResult> {
        let record = self.analyze_cooperation(
            CoopAnalysisDomain::FairnessImprovement,
            before_fairness,
            after_fairness,
        )?;
        let effect = record.effect_size?;
        if effect.cohens_d.abs() < FAIRNESS_IMPROVEMENT_MIN {
            return None;
        }
        Some(effect)
    }

    /// Assess statistical significance of trust dynamic changes
    pub fn trust_significance(
        &mut self,
        baseline_trust: &[f32],
        new_trust: &[f32],
    ) -> Option<SignificanceResult> {
        let record = self.analyze_cooperation(
            CoopAnalysisDomain::TrustDynamics,
            baseline_trust,
            new_trust,
        )?;
        let sig = record.significance?;
        if !sig.significant {
            return None;
        }
        Some(sig)
    }

    /// Analyze contention reduction between old and new cooperation strategies
    pub fn contention_analysis(
        &mut self,
        old_contention: &[f32],
        new_contention: &[f32],
    ) -> Option<EffectSizeResult> {
        let record = self.analyze_cooperation(
            CoopAnalysisDomain::ContentionReduction,
            old_contention,
            new_contention,
        )?;
        let effect = record.effect_size?;
        if effect.cohens_d.abs() < CONTENTION_REDUCTION_MIN {
            return None;
        }
        Some(effect)
    }

    /// Perform power analysis to determine if sample sizes are adequate
    pub fn power_analysis(
        &mut self,
        target_effect: f32,
        n_control: usize,
        n_treatment: usize,
    ) -> PowerResult {
        let result = self.compute_power(target_effect, n_control, n_treatment);
        if result.adequate {
            let adequate_count = self.stats.reports_generated.max(1) as f32;
            self.stats.power_adequate_ratio =
                EMA_ALPHA + (1.0 - EMA_ALPHA) * self.stats.power_adequate_ratio;
            let _ = adequate_count;
        } else {
            self.stats.power_adequate_ratio =
                (1.0 - EMA_ALPHA) * self.stats.power_adequate_ratio;
        }
        result
    }

    /// Generate a comprehensive summary report across all cooperation domains
    pub fn summary_report(&mut self) -> Vec<ReportEntry> {
        self.stats.reports_generated += 1;
        let mut report: Vec<ReportEntry> = Vec::new();
        let domains = [
            CoopAnalysisDomain::FairnessImprovement,
            CoopAnalysisDomain::TrustDynamics,
            CoopAnalysisDomain::ContentionReduction,
            CoopAnalysisDomain::NegotiationEfficiency,
            CoopAnalysisDomain::ResourceSharingEquity,
            CoopAnalysisDomain::ConflictResolutionSpeed,
            CoopAnalysisDomain::AuctionOutcomeBalance,
        ];
        for &domain in &domains {
            let domain_analyses: VecDeque<&CoopAnalysisRecord> =
                self.analyses.iter().filter(|a| a.domain == domain).collect();
            if domain_analyses.is_empty() {
                continue;
            }
            let total = domain_analyses.len() as u64;
            let sig_count = domain_analyses
                .iter()
                .filter(|a| a.significance.as_ref().map_or(false, |s| s.significant))
                .count() as u64;
            let effect_sizes: Vec<f32> = domain_analyses
                .iter()
                .filter_map(|a| a.effect_size.as_ref().map(|e| e.cohens_d.abs()))
                .collect();
            let avg_effect = if effect_sizes.is_empty() {
                0.0
            } else {
                effect_sizes.iter().sum::<f32>() / effect_sizes.len() as f32
            };
            let largest = effect_sizes
                .iter()
                .copied()
                .fold(0.0f32, |a, b| if b > a { b } else { a });
            let trend = self.effect_ema.get(&(domain as u64)).copied().unwrap_or(0.0);
            report.push(ReportEntry {
                domain,
                total_analyses: total,
                significant_count: sig_count,
                avg_effect_size: avg_effect,
                largest_effect: largest,
                overall_trend: trend,
            });
        }
        if report.len() > MAX_REPORT_ENTRIES {
            report.truncate(MAX_REPORT_ENTRIES);
        }
        self.report_cache = report.clone();
        report
    }

    /// Get current analysis statistics
    #[inline(always)]
    pub fn stats(&self) -> &AnalysisStats {
        &self.stats
    }

    /// Number of analyses conducted
    #[inline(always)]
    pub fn analysis_count(&self) -> usize {
        self.analyses.len()
    }

    /// Retrieve analyses for a specific domain
    #[inline(always)]
    pub fn domain_analyses(&self, domain: CoopAnalysisDomain) -> Vec<&CoopAnalysisRecord> {
        self.analyses.iter().filter(|a| a.domain == domain).collect()
    }

    // ========================================================================
    // INTERNAL HELPERS
    // ========================================================================

    fn compute_mean(&self, data: &[f32]) -> f32 {
        if data.is_empty() {
            return 0.0;
        }
        data.iter().sum::<f32>() / data.len() as f32
    }

    fn compute_variance(&self, data: &[f32], mean: f32) -> f32 {
        if data.len() < 2 {
            return 0.0;
        }
        let ss: f32 = data.iter().map(|&x| (x - mean) * (x - mean)).sum();
        ss / (data.len() - 1) as f32
    }

    fn compute_effect_size(
        &self,
        analysis_id: u64,
        domain: CoopAnalysisDomain,
        ctrl_mean: f32,
        treat_mean: f32,
        ctrl_var: f32,
        treat_var: f32,
        n_ctrl: usize,
        n_treat: usize,
    ) -> EffectSizeResult {
        let n1 = n_ctrl as f32;
        let n2 = n_treat as f32;
        let pooled_var = ((n1 - 1.0) * ctrl_var + (n2 - 1.0) * treat_var) / (n1 + n2 - 2.0);
        let pooled_std = self.fast_sqrt(pooled_var.max(0.0001));
        let cohens_d = (treat_mean - ctrl_mean) / pooled_std;
        let abs_d = cohens_d.abs();
        let magnitude = if abs_d < SMALL_EFFECT {
            EffectMagnitude::Negligible
        } else if abs_d < MEDIUM_EFFECT {
            EffectMagnitude::Small
        } else if abs_d < LARGE_EFFECT {
            EffectMagnitude::Medium
        } else if abs_d < 1.2 {
            EffectMagnitude::Large
        } else {
            EffectMagnitude::VeryLarge
        };
        let significant = abs_d >= SMALL_EFFECT && n_ctrl >= MIN_SAMPLE_SIZE && n_treat >= MIN_SAMPLE_SIZE;
        EffectSizeResult {
            analysis_id,
            cohens_d,
            magnitude,
            control_mean: ctrl_mean,
            treatment_mean: treat_mean,
            pooled_std,
            domain,
            significant,
        }
    }

    fn compute_significance(
        &self,
        analysis_id: u64,
        domain: CoopAnalysisDomain,
        ctrl_mean: f32,
        treat_mean: f32,
        ctrl_var: f32,
        treat_var: f32,
        n_ctrl: usize,
        n_treat: usize,
    ) -> SignificanceResult {
        let n1 = n_ctrl as f32;
        let n2 = n_treat as f32;
        let se = self.fast_sqrt((ctrl_var / n1 + treat_var / n2).max(0.0001));
        let t_stat = (treat_mean - ctrl_mean) / se;
        // Welch-Satterthwaite degrees of freedom approximation
        let num = (ctrl_var / n1 + treat_var / n2) * (ctrl_var / n1 + treat_var / n2);
        let d1 = (ctrl_var / n1) * (ctrl_var / n1) / (n1 - 1.0);
        let d2 = (treat_var / n2) * (treat_var / n2) / (n2 - 1.0);
        let df = if d1 + d2 > 0.0001 { num / (d1 + d2) } else { n1 + n2 - 2.0 };
        // Approximate p-value using a rough t-distribution tail
        let abs_t = t_stat.abs();
        let p_approx = if df > 1.0 {
            let x = df / (df + abs_t * abs_t);
            x * 0.5
        } else {
            0.5
        };
        let significant = p_approx < SIGNIFICANCE_LEVEL && n_ctrl >= MIN_SAMPLE_SIZE;
        SignificanceResult {
            analysis_id,
            t_statistic: t_stat,
            degrees_of_freedom: df,
            p_value_approx: p_approx,
            significant,
            domain,
        }
    }

    fn compute_power(&self, effect_size: f32, n_ctrl: usize, n_treat: usize) -> PowerResult {
        let n = (n_ctrl.min(n_treat)) as f32;
        let abs_effect = effect_size.abs().max(0.01);
        // Power approximation: 1 - beta ≈ Phi(|d|*sqrt(n/2) - z_alpha)
        let noncentrality = abs_effect * self.fast_sqrt(n / 2.0);
        let z_alpha = 1.96; // two-tailed 0.05
        let z_power = noncentrality - z_alpha;
        let power = self.standard_normal_cdf(z_power);
        let required_n = if abs_effect > 0.01 {
            let z_beta = 0.84; // 80% power
            let needed = ((z_alpha + z_beta) / abs_effect) * ((z_alpha + z_beta) / abs_effect) * 2.0;
            needed.ceil() as usize
        } else {
            MAX_DATA_POINTS
        };
        let adequate = power >= POWER_THRESHOLD;
        PowerResult {
            required_sample_size: required_n,
            current_power: power.min(1.0).max(0.0),
            target_power: POWER_THRESHOLD,
            target_effect_size: abs_effect,
            adequate,
        }
    }

    fn form_conclusion(&self, effect: &EffectSizeResult, sig: &SignificanceResult, power: &PowerResult) -> String {
        let mut s = String::from("Analysis: ");
        if sig.significant && effect.cohens_d.abs() >= MEDIUM_EFFECT {
            s.push_str("Significant improvement detected with ");
            match effect.magnitude {
                EffectMagnitude::Large | EffectMagnitude::VeryLarge => s.push_str("large effect"),
                EffectMagnitude::Medium => s.push_str("medium effect"),
                _ => s.push_str("notable effect"),
            }
        } else if sig.significant {
            s.push_str("Statistically significant but small practical effect");
        } else if !power.adequate {
            s.push_str("Insufficient power - more data needed");
        } else {
            s.push_str("No significant cooperation improvement detected");
        }
        s
    }

    fn fast_sqrt(&self, x: f32) -> f32 {
        if x <= 0.0 {
            return 0.0;
        }
        let mut guess = x * 0.5;
        for _ in 0..8 {
            if guess > 0.0 {
                guess = (guess + x / guess) * 0.5;
            }
        }
        guess
    }

    fn standard_normal_cdf(&self, z: f32) -> f32 {
        // Abramowitz and Stegun approximation
        let abs_z = z.abs();
        let t = 1.0 / (1.0 + 0.2316419 * abs_z);
        let d = 0.3989423 * (-abs_z * abs_z * 0.5).exp_approx();
        let p = d * t * (0.3193815 + t * (-0.3565638 + t * (1.781478 + t * (-1.821256 + t * 1.330274))));
        if z > 0.0 { 1.0 - p } else { p }
    }
}

/// Minimal exp approximation for no_std environments
trait ExpApprox {
    fn exp_approx(self) -> f32;
}

impl ExpApprox for f32 {
    fn exp_approx(self) -> f32 {
        let x = self.max(-20.0).min(20.0);
        let mut result = 1.0f32;
        let mut term = 1.0f32;
        for i in 1..16 {
            term *= x / i as f32;
            result += term;
        }
        if result < 0.0 { 0.0 } else { result }
    }
}
