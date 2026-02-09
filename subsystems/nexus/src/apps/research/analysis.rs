// SPDX-License-Identifier: GPL-2.0
//! # Apps Analysis Engine — Statistical Analysis of App Research Results
//!
//! Computes effect sizes for app optimizations, significance tests for
//! classification improvements, ANOVA-like comparisons across app categories.
//! Every research finding passes through this engine so that the NEXUS
//! research pipeline only acts on statistically sound evidence rather than
//! noise or one-off anomalies.
//!
//! The engine that separates real signal from noise in app research.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_RESULTS: usize = 2048;
const MAX_CATEGORIES: usize = 64;
const MAX_COMPARISONS: usize = 256;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const DEFAULT_ALPHA: f32 = 0.05;
const T_CRITICAL_95: f32 = 1.96;
const T_CRITICAL_99: f32 = 2.576;
const MIN_SAMPLES: usize = 30;
const SMALL_EFFECT: f32 = 0.20;
const MEDIUM_EFFECT: f32 = 0.50;
const LARGE_EFFECT: f32 = 0.80;
const MAX_SUMMARY_HISTORY: usize = 512;
const POWER_TARGET: f32 = 0.80;
const BONFERRONI_LIMIT: usize = 20;

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
// TYPES
// ============================================================================

/// A single research result awaiting analysis.
#[derive(Clone)]
pub struct AnalysisResult {
    pub result_id: u64,
    pub category_tag: String,
    pub control_values: Vec<f32>,
    pub treatment_values: Vec<f32>,
    pub timestamp: u64,
    pub metadata_hash: u64,
}

/// Effect-size measurement for an optimization.
#[derive(Clone)]
pub struct EffectSizeReport {
    pub result_id: u64,
    pub cohens_d: f32,
    pub magnitude: EffectMagnitude,
    pub mean_diff: f32,
    pub pooled_std: f32,
    pub confidence_interval_lo: f32,
    pub confidence_interval_hi: f32,
}

/// Qualitative magnitude label.
#[derive(Clone, Copy, PartialEq)]
pub enum EffectMagnitude {
    Negligible,
    Small,
    Medium,
    Large,
}

/// Significance-test outcome.
#[derive(Clone)]
pub struct SignificanceReport {
    pub result_id: u64,
    pub t_statistic: f32,
    pub p_estimate: f32,
    pub significant: bool,
    pub alpha_used: f32,
    pub sample_n_control: usize,
    pub sample_n_treatment: usize,
}

/// ANOVA-like category comparison report.
#[derive(Clone)]
pub struct CategoryComparison {
    pub comparison_id: u64,
    pub categories: Vec<String>,
    pub f_statistic: f32,
    pub between_variance: f32,
    pub within_variance: f32,
    pub significant: bool,
    pub post_hoc_pairs: Vec<(String, String, f32)>,
}

/// Power-analysis report.
#[derive(Clone)]
pub struct PowerReport {
    pub required_n: usize,
    pub current_n: usize,
    pub achieved_power: f32,
    pub target_power: f32,
    pub target_effect: f32,
}

/// Summary statistics snapshot.
#[derive(Clone)]
pub struct SummaryStatistics {
    pub total_results: usize,
    pub significant_count: usize,
    pub avg_effect_size: f32,
    pub avg_power: f32,
    pub category_count: usize,
    pub ema_significance_rate: f32,
    pub ema_effect_size: f32,
}

/// Running stats for the engine.
#[derive(Clone)]
pub struct AnalysisStats {
    pub results_analyzed: u64,
    pub significant_found: u64,
    pub comparisons_run: u64,
    pub power_analyses: u64,
    pub ema_effect: f32,
    pub ema_significance_rate: f32,
    pub ema_power: f32,
}

// ============================================================================
// ENGINE
// ============================================================================

/// Statistical analysis engine for app research results.
pub struct AppsAnalysisEngine {
    results: BTreeMap<u64, AnalysisResult>,
    effects: BTreeMap<u64, EffectSizeReport>,
    significance: BTreeMap<u64, SignificanceReport>,
    comparisons: Vec<CategoryComparison>,
    category_values: BTreeMap<String, Vec<f32>>,
    summary_history: Vec<SummaryStatistics>,
    stats: AnalysisStats,
    rng_state: u64,
    tick: u64,
}

impl AppsAnalysisEngine {
    /// Create a new analysis engine.
    pub fn new(seed: u64) -> Self {
        Self {
            results: BTreeMap::new(),
            effects: BTreeMap::new(),
            significance: BTreeMap::new(),
            comparisons: Vec::new(),
            category_values: BTreeMap::new(),
            summary_history: Vec::new(),
            stats: AnalysisStats {
                results_analyzed: 0,
                significant_found: 0,
                comparisons_run: 0,
                power_analyses: 0,
                ema_effect: 0.0,
                ema_significance_rate: 0.0,
                ema_power: 0.0,
            },
            rng_state: seed ^ 0xa57b23de01fc948a,
            tick: 0,
        }
    }

    // ── Primary API ────────────────────────────────────────────────────

    /// Ingest a new research result for analysis.
    pub fn analyze_result(&mut self, category: &str, control: &[f32], treatment: &[f32]) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(category.as_bytes()) ^ self.tick;

        let result = AnalysisResult {
            result_id: id,
            category_tag: String::from(category),
            control_values: Vec::from(control),
            treatment_values: Vec::from(treatment),
            timestamp: self.tick,
            metadata_hash: fnv1a_hash(&id.to_le_bytes()),
        };

        // Update per-category pool with treatment means
        let mean_t = Self::mean(treatment);
        let entry = self.category_values.entry(String::from(category)).or_insert_with(Vec::new);
        entry.push(mean_t);
        if entry.len() > MAX_RESULTS {
            entry.remove(0);
        }

        if self.results.len() >= MAX_RESULTS {
            if let Some(oldest) = self.results.keys().next().cloned() {
                self.results.remove(&oldest);
            }
        }
        self.results.insert(id, result);

        // Auto-compute effect & significance
        let effect = self.compute_effect_size(id, control, treatment);
        self.effects.insert(id, effect);

        let sig = self.compute_significance(id, control, treatment);
        if sig.significant {
            self.stats.significant_found += 1;
        }
        self.significance.insert(id, sig);

        self.stats.results_analyzed += 1;
        let sig_rate = self.stats.significant_found as f32 / self.stats.results_analyzed.max(1) as f32;
        self.stats.ema_significance_rate =
            EMA_ALPHA * sig_rate + (1.0 - EMA_ALPHA) * self.stats.ema_significance_rate;

        id
    }

    /// Compute Cohen's d effect size for an optimization comparison.
    pub fn effect_size(&self, result_id: u64) -> Option<EffectSizeReport> {
        self.effects.get(&result_id).cloned()
    }

    /// Run a significance test for a given result.
    pub fn significance_test(&self, result_id: u64) -> Option<SignificanceReport> {
        self.significance.get(&result_id).cloned()
    }

    /// Run ANOVA-like comparison across all accumulated categories.
    pub fn category_comparison(&mut self) -> Option<CategoryComparison> {
        if self.category_values.len() < 2 {
            return None;
        }
        self.stats.comparisons_run += 1;

        // Grand mean
        let mut grand_sum = 0.0f32;
        let mut grand_n = 0usize;
        for vals in self.category_values.values() {
            for &v in vals.iter() {
                grand_sum += v;
                grand_n += 1;
            }
        }
        if grand_n == 0 {
            return None;
        }
        let grand_mean = grand_sum / grand_n as f32;

        // Between-group and within-group variance
        let k = self.category_values.len() as f32;
        let mut ss_between = 0.0f32;
        let mut ss_within = 0.0f32;
        let mut cat_names = Vec::new();

        for (name, vals) in self.category_values.iter() {
            cat_names.push(name.clone());
            if vals.is_empty() {
                continue;
            }
            let group_mean = Self::mean(vals);
            let n_j = vals.len() as f32;
            ss_between += n_j * (group_mean - grand_mean) * (group_mean - grand_mean);
            for &v in vals.iter() {
                ss_within += (v - group_mean) * (v - group_mean);
            }
        }

        let df_between = k - 1.0;
        let df_within = (grand_n as f32) - k;
        if df_between <= 0.0 || df_within <= 0.0 {
            return None;
        }

        let ms_between = ss_between / df_between;
        let ms_within = ss_within / df_within;
        let f_stat = if ms_within > 1e-9 { ms_between / ms_within } else { 0.0 };

        // Rough significance heuristic: F > 3.0 for moderate alpha
        let significant = f_stat > 3.0 && grand_n >= MIN_SAMPLES;

        // Post-hoc pairwise comparisons (Bonferroni-corrected)
        let mut post_hoc = Vec::new();
        let n_pairs = cat_names.len() * (cat_names.len() - 1) / 2;
        let corrected_alpha = if n_pairs > 0 && n_pairs <= BONFERRONI_LIMIT {
            DEFAULT_ALPHA / n_pairs as f32
        } else {
            DEFAULT_ALPHA
        };

        for i in 0..cat_names.len() {
            for j in (i + 1)..cat_names.len() {
                if let (Some(a), Some(b)) = (
                    self.category_values.get(&cat_names[i]),
                    self.category_values.get(&cat_names[j]),
                ) {
                    if a.len() >= 2 && b.len() >= 2 {
                        let d = self.welch_t(a, b);
                        let sig_pair = abs_f32(d) > T_CRITICAL_99 / corrected_alpha.max(0.001);
                        if sig_pair {
                            post_hoc.push((cat_names[i].clone(), cat_names[j].clone(), d));
                        }
                    }
                }
                if post_hoc.len() >= MAX_COMPARISONS {
                    break;
                }
            }
        }

        let cmp_id = fnv1a_hash(&self.stats.comparisons_run.to_le_bytes());
        let comparison = CategoryComparison {
            comparison_id: cmp_id,
            categories: cat_names,
            f_statistic: f_stat,
            between_variance: ms_between,
            within_variance: ms_within,
            significant,
            post_hoc_pairs: post_hoc,
        };

        if self.comparisons.len() >= MAX_COMPARISONS {
            self.comparisons.remove(0);
        }
        self.comparisons.push(comparison.clone());
        Some(comparison)
    }

    /// Compute the required sample size to achieve target statistical power.
    pub fn power_analysis(&mut self, target_effect: f32, current_n: usize) -> PowerReport {
        self.stats.power_analyses += 1;

        // Required n ≈ (z_α + z_β)² / d²  for two-tailed test
        let z_alpha = T_CRITICAL_95;
        let z_beta = 0.842; // z for 80% power
        let d = if target_effect > 0.01 { target_effect } else { MEDIUM_EFFECT };
        let required_n_f = ((z_alpha + z_beta) * (z_alpha + z_beta)) / (d * d);
        let required_n = (required_n_f as usize).max(MIN_SAMPLES);

        // Achieved power estimate — increases as current_n approaches required_n
        let ratio = current_n as f32 / required_n as f32;
        let achieved_power = if ratio >= 1.0 {
            0.99_f32.min(POWER_TARGET + (ratio - 1.0) * 0.1)
        } else {
            POWER_TARGET * ratio
        };

        self.stats.ema_power = EMA_ALPHA * achieved_power + (1.0 - EMA_ALPHA) * self.stats.ema_power;

        PowerReport {
            required_n,
            current_n,
            achieved_power,
            target_power: POWER_TARGET,
            target_effect: d,
        }
    }

    /// Produce an aggregate summary of all analysis activity.
    pub fn summary_statistics(&mut self) -> SummaryStatistics {
        let total = self.stats.results_analyzed as usize;
        let sig_count = self.stats.significant_found as usize;

        // Average effect size across stored effects
        let mut sum_eff = 0.0f32;
        let n_eff = self.effects.len().max(1);
        for eff in self.effects.values() {
            sum_eff += abs_f32(eff.cohens_d);
        }
        let avg_effect = sum_eff / n_eff as f32;

        self.stats.ema_effect = EMA_ALPHA * avg_effect + (1.0 - EMA_ALPHA) * self.stats.ema_effect;

        let summary = SummaryStatistics {
            total_results: total,
            significant_count: sig_count,
            avg_effect_size: avg_effect,
            avg_power: self.stats.ema_power,
            category_count: self.category_values.len(),
            ema_significance_rate: self.stats.ema_significance_rate,
            ema_effect_size: self.stats.ema_effect,
        };

        if self.summary_history.len() >= MAX_SUMMARY_HISTORY {
            self.summary_history.remove(0);
        }
        self.summary_history.push(summary.clone());
        summary
    }

    /// Return engine stats.
    pub fn stats(&self) -> &AnalysisStats {
        &self.stats
    }

    // ── Internal Helpers ───────────────────────────────────────────────

    fn compute_effect_size(&mut self, id: u64, control: &[f32], treatment: &[f32]) -> EffectSizeReport {
        let mean_c = Self::mean(control);
        let mean_t = Self::mean(treatment);
        let std_c = Self::std_dev(control, mean_c);
        let std_t = Self::std_dev(treatment, mean_t);

        let n_c = control.len().max(1) as f32;
        let n_t = treatment.len().max(1) as f32;
        let pooled = sqrt_approx(
            ((n_c - 1.0) * std_c * std_c + (n_t - 1.0) * std_t * std_t)
                / (n_c + n_t - 2.0).max(1.0),
        );

        let d = if pooled > 1e-9 { (mean_t - mean_c) / pooled } else { 0.0 };
        let abs_d = abs_f32(d);
        let magnitude = if abs_d >= LARGE_EFFECT {
            EffectMagnitude::Large
        } else if abs_d >= MEDIUM_EFFECT {
            EffectMagnitude::Medium
        } else if abs_d >= SMALL_EFFECT {
            EffectMagnitude::Small
        } else {
            EffectMagnitude::Negligible
        };

        let se = pooled * sqrt_approx(1.0 / n_c + 1.0 / n_t);
        let ci_lo = d - T_CRITICAL_95 * se;
        let ci_hi = d + T_CRITICAL_95 * se;

        EffectSizeReport {
            result_id: id,
            cohens_d: d,
            magnitude,
            mean_diff: mean_t - mean_c,
            pooled_std: pooled,
            confidence_interval_lo: ci_lo,
            confidence_interval_hi: ci_hi,
        }
    }

    fn compute_significance(&self, id: u64, control: &[f32], treatment: &[f32]) -> SignificanceReport {
        let mean_c = Self::mean(control);
        let mean_t = Self::mean(treatment);
        let var_c = Self::variance(control, mean_c);
        let var_t = Self::variance(treatment, mean_t);
        let n_c = control.len();
        let n_t = treatment.len();

        // Welch's t-test
        let se = sqrt_approx(var_c / n_c.max(1) as f32 + var_t / n_t.max(1) as f32);
        let t_stat = if se > 1e-9 { (mean_t - mean_c) / se } else { 0.0 };
        let significant = abs_f32(t_stat) > T_CRITICAL_95 && n_c >= MIN_SAMPLES && n_t >= MIN_SAMPLES;

        // Rough p-value estimate from normal approximation
        let abs_t = abs_f32(t_stat);
        let p_est = if abs_t > 4.0 {
            0.0001
        } else if abs_t > 3.0 {
            0.003
        } else if abs_t > T_CRITICAL_95 {
            0.04
        } else {
            0.5 - abs_t * 0.15
        };

        SignificanceReport {
            result_id: id,
            t_statistic: t_stat,
            p_estimate: p_est.max(0.0),
            significant,
            alpha_used: DEFAULT_ALPHA,
            sample_n_control: n_c,
            sample_n_treatment: n_t,
        }
    }

    fn welch_t(&self, a: &[f32], b: &[f32]) -> f32 {
        let mean_a = Self::mean(a);
        let mean_b = Self::mean(b);
        let var_a = Self::variance(a, mean_a);
        let var_b = Self::variance(b, mean_b);
        let se = sqrt_approx(var_a / a.len().max(1) as f32 + var_b / b.len().max(1) as f32);
        if se > 1e-9 { (mean_a - mean_b) / se } else { 0.0 }
    }

    fn mean(vals: &[f32]) -> f32 {
        if vals.is_empty() {
            return 0.0;
        }
        let mut s = 0.0f32;
        for &v in vals {
            s += v;
        }
        s / vals.len() as f32
    }

    fn variance(vals: &[f32], mean: f32) -> f32 {
        if vals.len() < 2 {
            return 0.0;
        }
        let mut s = 0.0f32;
        for &v in vals {
            let d = v - mean;
            s += d * d;
        }
        s / (vals.len() - 1) as f32
    }

    fn std_dev(vals: &[f32], mean: f32) -> f32 {
        sqrt_approx(Self::variance(vals, mean))
    }
}
