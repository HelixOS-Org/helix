// SPDX-License-Identifier: GPL-2.0
//! # Bridge Analysis Engine — Statistical Analysis of Bridge Research
//!
//! Provides kernel-safe numerical approximations for statistical hypothesis
//! testing on bridge research results. Implements t-tests, effect-size
//! computation (Cohen's d), ANOVA-like multi-group comparisons, power
//! analysis, and significance testing — all without floating-point library
//! functions, using rational polynomial approximations where needed.
//!
//! Every bridge experiment produces raw measurements; this module turns
//! those measurements into actionable conclusions with calibrated
//! confidence levels.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_SAMPLES: usize = 4096;
const MAX_GROUPS: usize = 32;
const MAX_RESULTS: usize = 1024;
const SIGNIFICANCE_THRESHOLD: f32 = 0.05;
const SMALL_EFFECT: f32 = 0.2;
const MEDIUM_EFFECT: f32 = 0.5;
const LARGE_EFFECT: f32 = 0.8;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const MIN_SAMPLE_SIZE: usize = 5;
const POWER_TARGET: f32 = 0.80;
const DEFAULT_ALPHA: f32 = 0.05;
const BONFERRONI_MAX_COMPARISONS: usize = 64;

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

/// Kernel-safe square-root via Newton–Raphson (6 iterations).
fn sqrt_approx(val: f32) -> f32 {
    if val <= 0.0 {
        return 0.0;
    }
    let mut guess = val * 0.5;
    for _ in 0..6 {
        guess = 0.5 * (guess + val / guess);
    }
    guess
}

/// Kernel-safe absolute value.
fn abs_f32(v: f32) -> f32 {
    if v < 0.0 { -v } else { v }
}

/// Approximate the two-tailed p-value from a t-statistic and degrees of
/// freedom using a rational polynomial approximation of the cumulative
/// t-distribution (Abramowitz & Stegun inspired, simplified for kernel).
fn p_value_approx(t_stat: f32, df: f32) -> f32 {
    let x = abs_f32(t_stat);
    // Use normal approximation for large df
    let z = if df > 30.0 {
        x
    } else {
        x * (1.0 - 1.0 / (4.0 * df)) // simple Welch correction
    };
    // Rational poly approx of 2*(1-Phi(z)) for z >= 0
    let b1: f32 = 0.319381530;
    let b2: f32 = -0.356563782;
    let b3: f32 = 1.781477937;
    let b4: f32 = -1.821255978;
    let b5: f32 = 1.330274429;
    let p_coeff: f32 = 0.2316419;
    let t_var = 1.0 / (1.0 + p_coeff * z);
    let t2 = t_var * t_var;
    let t3 = t2 * t_var;
    let t4 = t3 * t_var;
    let t5 = t4 * t_var;
    // exp(-z^2/2) approx via Taylor-ish (kernel safe)
    let zz = z * z * 0.5;
    let exp_neg = 1.0 / (1.0 + zz + zz * zz * 0.5 + zz * zz * zz / 6.0);
    let one_tail = exp_neg * (b1 * t_var + b2 * t2 + b3 * t3 + b4 * t4 + b5 * t5)
        * 0.3989422804; // 1/sqrt(2*pi)
    let two_tail = 2.0 * one_tail;
    if two_tail > 1.0 { 1.0 } else if two_tail < 0.0 { 0.0 } else { two_tail }
}

// ============================================================================
// TYPES
// ============================================================================

/// A single sample group for analysis.
#[derive(Clone)]
struct SampleGroup {
    label: String,
    values: Vec<f32>,
    mean: f32,
    variance: f32,
    n: usize,
}

/// Result of a statistical analysis.
#[derive(Clone)]
pub struct AnalysisResult {
    pub test_name: String,
    pub test_statistic: f32,
    pub p_value_approx: f32,
    pub effect_size: f32,
    pub effect_label: String,
    pub significant: bool,
    pub degrees_of_freedom: f32,
    pub confidence_interval_low: f32,
    pub confidence_interval_high: f32,
    pub sample_size_a: usize,
    pub sample_size_b: usize,
    pub timestamp: u64,
}

/// Cumulative analysis statistics.
#[derive(Clone)]
#[repr(align(64))]
pub struct AnalysisStats {
    pub total_analyses: u64,
    pub significant_results: u64,
    pub avg_effect_size_ema: f32,
    pub avg_p_value_ema: f32,
    pub largest_effect: f32,
    pub smallest_p: f32,
    pub total_samples_processed: u64,
    pub anova_runs: u64,
    pub power_analyses: u64,
}

/// Power analysis result.
#[derive(Clone)]
pub struct PowerResult {
    pub required_sample_size: usize,
    pub expected_power: f32,
    pub effect_size: f32,
    pub alpha: f32,
    pub achieved: bool,
}

/// ANOVA-like comparison result.
#[derive(Clone)]
pub struct AnovaResult {
    pub f_statistic: f32,
    pub p_value_approx: f32,
    pub between_variance: f32,
    pub within_variance: f32,
    pub group_count: usize,
    pub total_n: usize,
    pub significant: bool,
    pub eta_squared: f32,
}

// ============================================================================
// BRIDGE ANALYSIS ENGINE
// ============================================================================

/// Statistical analysis engine for bridge research results.
#[repr(align(64))]
pub struct BridgeAnalysisEngine {
    results: Vec<AnalysisResult>,
    groups: BTreeMap<u64, SampleGroup>,
    stats: AnalysisStats,
    rng_state: u64,
    tick: u64,
    alpha_level: f32,
}

impl BridgeAnalysisEngine {
    /// Create a new analysis engine.
    pub fn new(seed: u64) -> Self {
        Self {
            results: Vec::new(),
            groups: BTreeMap::new(),
            stats: AnalysisStats {
                total_analyses: 0,
                significant_results: 0,
                avg_effect_size_ema: 0.0,
                avg_p_value_ema: 0.5,
                largest_effect: 0.0,
                smallest_p: 1.0,
                total_samples_processed: 0,
                anova_runs: 0,
                power_analyses: 0,
            },
            rng_state: seed ^ 0xA1A2A3A4A5A6A7A8,
            tick: 0,
            alpha_level: DEFAULT_ALPHA,
        }
    }

    /// Register a sample group for analysis.
    pub fn register_group(&mut self, label: &str, values: &[f32]) {
        if values.is_empty() || values.len() > MAX_SAMPLES {
            return;
        }
        let key = fnv1a_hash(label.as_bytes());
        let n = values.len();
        let sum: f32 = values.iter().copied().sum();
        let mean = sum / n as f32;
        let var: f32 = values.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>()
            / if n > 1 { (n - 1) as f32 } else { 1.0 };
        let mut stored = Vec::new();
        for v in values {
            stored.push(*v);
        }
        self.groups.insert(
            key,
            SampleGroup {
                label: String::from(label),
                values: stored,
                mean,
                variance: var,
                n,
            },
        );
        self.stats.total_samples_processed += n as u64;
    }

    /// Independent two-sample t-test (Welch's approximation).
    pub fn t_test(&mut self, label_a: &str, label_b: &str) -> AnalysisResult {
        self.tick += 1;
        let key_a = fnv1a_hash(label_a.as_bytes());
        let key_b = fnv1a_hash(label_b.as_bytes());

        let (mean_a, var_a, n_a) = self.group_stats(key_a);
        let (mean_b, var_b, n_b) = self.group_stats(key_b);

        let se = sqrt_approx(var_a / n_a as f32 + var_b / n_b as f32);
        let t_stat = if se > 1e-9 { (mean_a - mean_b) / se } else { 0.0 };

        // Welch–Satterthwaite degrees of freedom
        let num = (var_a / n_a as f32 + var_b / n_b as f32)
            * (var_a / n_a as f32 + var_b / n_b as f32);
        let den_a = (var_a / n_a as f32) * (var_a / n_a as f32) / ((n_a - 1).max(1)) as f32;
        let den_b = (var_b / n_b as f32) * (var_b / n_b as f32) / ((n_b - 1).max(1)) as f32;
        let df = if den_a + den_b > 1e-12 {
            num / (den_a + den_b)
        } else {
            (n_a + n_b - 2) as f32
        };

        let p_val = p_value_approx(t_stat, df);
        let effect = self.compute_effect_size(mean_a, mean_b, var_a, var_b, n_a, n_b);
        let sig = p_val < self.alpha_level;

        // Confidence interval for mean difference (approx 95%)
        let margin = 1.96 * se;
        let diff = mean_a - mean_b;

        let result = AnalysisResult {
            test_name: String::from("Welch t-test"),
            test_statistic: t_stat,
            p_value_approx: p_val,
            effect_size: effect,
            effect_label: self.effect_label(effect),
            significant: sig,
            degrees_of_freedom: df,
            confidence_interval_low: diff - margin,
            confidence_interval_high: diff + margin,
            sample_size_a: n_a,
            sample_size_b: n_b,
            timestamp: self.tick,
        };

        self.record_result(&result);
        result
    }

    /// Compute Cohen's d effect size between two groups.
    #[inline]
    pub fn effect_size(&self, label_a: &str, label_b: &str) -> f32 {
        let key_a = fnv1a_hash(label_a.as_bytes());
        let key_b = fnv1a_hash(label_b.as_bytes());
        let (mean_a, var_a, n_a) = self.group_stats(key_a);
        let (mean_b, var_b, n_b) = self.group_stats(key_b);
        self.compute_effect_size(mean_a, mean_b, var_a, var_b, n_a, n_b)
    }

    /// ANOVA-like comparison across all registered groups.
    pub fn anova_compare(&mut self) -> AnovaResult {
        self.tick += 1;
        self.stats.anova_runs += 1;

        let groups: Vec<&SampleGroup> = self.groups.values().collect();
        let k = groups.len();
        if k < 2 {
            return AnovaResult {
                f_statistic: 0.0,
                p_value_approx: 1.0,
                between_variance: 0.0,
                within_variance: 0.0,
                group_count: k,
                total_n: 0,
                significant: false,
                eta_squared: 0.0,
            };
        }

        let total_n: usize = groups.iter().map(|g| g.n).sum();
        let grand_mean: f32 = groups.iter().map(|g| g.mean * g.n as f32).sum::<f32>()
            / total_n as f32;

        // Between-group sum of squares
        let ss_between: f32 = groups
            .iter()
            .map(|g| g.n as f32 * (g.mean - grand_mean) * (g.mean - grand_mean))
            .sum();

        // Within-group sum of squares
        let ss_within: f32 = groups
            .iter()
            .map(|g| g.variance * (g.n - 1).max(1) as f32)
            .sum();

        let df_between = (k - 1) as f32;
        let df_within = (total_n - k).max(1) as f32;

        let ms_between = ss_between / df_between.max(1.0);
        let ms_within = ss_within / df_within.max(1.0);
        let f_stat = if ms_within > 1e-9 {
            ms_between / ms_within
        } else {
            0.0
        };

        let ss_total = ss_between + ss_within;
        let eta_sq = if ss_total > 1e-9 {
            ss_between / ss_total
        } else {
            0.0
        };

        // Approximate p-value from F-statistic (simplified)
        let p = self.f_to_p_approx(f_stat, df_between, df_within);

        AnovaResult {
            f_statistic: f_stat,
            p_value_approx: p,
            between_variance: ms_between,
            within_variance: ms_within,
            group_count: k,
            total_n,
            significant: p < self.alpha_level,
            eta_squared: eta_sq,
        }
    }

    /// Estimate required sample size for desired power.
    pub fn power_analysis(&mut self, target_effect: f32) -> PowerResult {
        self.tick += 1;
        self.stats.power_analyses += 1;

        let effect = if abs_f32(target_effect) < 0.01 { MEDIUM_EFFECT } else { abs_f32(target_effect) };

        // Sample size formula: n = (z_alpha + z_beta)^2 / d^2
        // z_alpha/2 ≈ 1.96, z_beta for 0.80 power ≈ 0.842
        let z_alpha = 1.96;
        let z_beta = 0.842;
        let numer = (z_alpha + z_beta) * (z_alpha + z_beta);
        let required_n = (numer / (effect * effect)) as usize + 1;

        // Compute achieved power given the available sample sizes
        let max_n = self.groups.values().map(|g| g.n).max().unwrap_or(0);
        let achieved_power = if max_n > 0 {
            let lambda = effect * sqrt_approx(max_n as f32);
            // Approximation: power ≈ Phi(lambda - z_alpha)
            self.phi_approx(lambda - z_alpha)
        } else {
            0.0
        };

        PowerResult {
            required_sample_size: required_n,
            expected_power: achieved_power,
            effect_size: effect,
            alpha: self.alpha_level,
            achieved: achieved_power >= POWER_TARGET,
        }
    }

    /// Check significance of the most recent result, applying Bonferroni
    /// correction if multiple comparisons were made.
    #[inline]
    pub fn significance_check(&self) -> Vec<(String, bool, f32)> {
        let n_comparisons = self.results.len().min(BONFERRONI_MAX_COMPARISONS).max(1);
        let corrected_alpha = self.alpha_level / n_comparisons as f32;
        let mut out = Vec::new();
        for r in self.results.iter().rev().take(n_comparisons) {
            let sig = r.p_value_approx < corrected_alpha;
            out.push((r.test_name.clone(), sig, corrected_alpha));
        }
        out
    }

    /// Generate a summary of all analysis results.
    pub fn result_summary(&self) -> AnalysisSummary {
        let n = self.results.len();
        let sig_count = self.results.iter().filter(|r| r.significant).count();
        let avg_effect = if n > 0 {
            self.results.iter().map(|r| r.effect_size).sum::<f32>() / n as f32
        } else {
            0.0
        };
        let avg_p = if n > 0 {
            self.results.iter().map(|r| r.p_value_approx).sum::<f32>() / n as f32
        } else {
            1.0
        };
        let median_effect = self.median_effect();
        AnalysisSummary {
            total_tests: n,
            significant_count: sig_count,
            non_significant_count: n - sig_count,
            average_effect_size: avg_effect,
            median_effect_size: median_effect,
            average_p_value: avg_p,
            smallest_p_value: self.stats.smallest_p,
            largest_effect_size: self.stats.largest_effect,
            bonferroni_alpha: self.alpha_level / n.max(1) as f32,
            total_samples: self.stats.total_samples_processed,
        }
    }

    /// Current engine statistics.
    #[inline(always)]
    pub fn stats(&self) -> &AnalysisStats {
        &self.stats
    }

    /// Number of registered groups.
    #[inline(always)]
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    /// Clear all groups and results.
    #[inline(always)]
    pub fn reset(&mut self) {
        self.groups.clear();
        self.results.clear();
    }

    // ========================================================================
    // INTERNAL HELPERS
    // ========================================================================

    fn group_stats(&self, key: u64) -> (f32, f32, usize) {
        match self.groups.get(&key) {
            Some(g) => (g.mean, g.variance, g.n.max(MIN_SAMPLE_SIZE)),
            None => (0.0, 1.0, MIN_SAMPLE_SIZE),
        }
    }

    fn compute_effect_size(
        &self,
        mean_a: f32,
        mean_b: f32,
        var_a: f32,
        var_b: f32,
        n_a: usize,
        n_b: usize,
    ) -> f32 {
        // Pooled standard deviation (Cohen's d)
        let na = n_a.max(1) as f32;
        let nb = n_b.max(1) as f32;
        let pooled_var = ((na - 1.0) * var_a + (nb - 1.0) * var_b)
            / (na + nb - 2.0).max(1.0);
        let pooled_sd = sqrt_approx(pooled_var);
        if pooled_sd > 1e-9 {
            abs_f32(mean_a - mean_b) / pooled_sd
        } else {
            0.0
        }
    }

    fn effect_label(&self, d: f32) -> String {
        let ad = abs_f32(d);
        if ad < SMALL_EFFECT {
            String::from("negligible")
        } else if ad < MEDIUM_EFFECT {
            String::from("small")
        } else if ad < LARGE_EFFECT {
            String::from("medium")
        } else {
            String::from("large")
        }
    }

    fn record_result(&mut self, r: &AnalysisResult) {
        self.stats.total_analyses += 1;
        if r.significant {
            self.stats.significant_results += 1;
        }
        self.stats.avg_effect_size_ema =
            self.stats.avg_effect_size_ema * (1.0 - EMA_ALPHA) + r.effect_size * EMA_ALPHA;
        self.stats.avg_p_value_ema =
            self.stats.avg_p_value_ema * (1.0 - EMA_ALPHA) + r.p_value_approx * EMA_ALPHA;
        if r.effect_size > self.stats.largest_effect {
            self.stats.largest_effect = r.effect_size;
        }
        if r.p_value_approx < self.stats.smallest_p {
            self.stats.smallest_p = r.p_value_approx;
        }
        if self.results.len() < MAX_RESULTS {
            self.results.push(r.clone());
        }
    }

    fn median_effect(&self) -> f32 {
        if self.results.is_empty() {
            return 0.0;
        }
        let mut effects: Vec<f32> = self.results.iter().map(|r| r.effect_size).collect();
        effects.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
        let mid = effects.len() / 2;
        if effects.len() % 2 == 0 {
            (effects[mid - 1] + effects[mid]) / 2.0
        } else {
            effects[mid]
        }
    }

    /// Approximate F-distribution p-value via a rough transformation.
    fn f_to_p_approx(&self, f: f32, df1: f32, df2: f32) -> f32 {
        if f <= 0.0 {
            return 1.0;
        }
        // Transform F to approximate z
        let a = 2.0 / (9.0 * df1);
        let b = 2.0 / (9.0 * df2);
        let f_third = if f > 0.0 {
            // cube root approximation via Newton
            let mut r = f;
            for _ in 0..8 {
                r = (2.0 * r + f / (r * r)) / 3.0;
            }
            r
        } else {
            0.0
        };
        let z_num = (1.0 - b) * f_third - (1.0 - a);
        let z_den = sqrt_approx(b * f_third * f_third + a);
        let z = if z_den > 1e-9 { z_num / z_den } else { 0.0 };
        // Two-tailed p from z
        let p = 1.0 - self.phi_approx(z);
        if p < 0.0 { 0.0 } else if p > 1.0 { 1.0 } else { p }
    }

    /// Approximate standard normal CDF Φ(x).
    fn phi_approx(&self, x: f32) -> f32 {
        if x < -6.0 {
            return 0.0;
        }
        if x > 6.0 {
            return 1.0;
        }
        let ax = abs_f32(x);
        let t = 1.0 / (1.0 + 0.2316419 * ax);
        let t2 = t * t;
        let t3 = t2 * t;
        let t4 = t3 * t;
        let t5 = t4 * t;
        let zz = ax * ax * 0.5;
        let exp_neg = 1.0 / (1.0 + zz + zz * zz * 0.5 + zz * zz * zz / 6.0);
        let poly = 0.319381530 * t - 0.356563782 * t2 + 1.781477937 * t3
            - 1.821255978 * t4 + 1.330274429 * t5;
        let cdf = 1.0 - 0.3989422804 * exp_neg * poly;
        if x >= 0.0 { cdf } else { 1.0 - cdf }
    }
}

// ============================================================================
// SUMMARY TYPE
// ============================================================================

/// A human-readable summary of all analyses performed.
#[derive(Clone)]
#[repr(align(64))]
pub struct AnalysisSummary {
    pub total_tests: usize,
    pub significant_count: usize,
    pub non_significant_count: usize,
    pub average_effect_size: f32,
    pub median_effect_size: f32,
    pub average_p_value: f32,
    pub smallest_p_value: f32,
    pub largest_effect_size: f32,
    pub bonferroni_alpha: f32,
    pub total_samples: u64,
}
