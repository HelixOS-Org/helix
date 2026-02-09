// SPDX-License-Identifier: GPL-2.0
//! # Holistic Analysis Engine — System-Wide Statistical Meta-Analysis
//!
//! The master statistical analysis engine for the entire NEXUS kernel
//! intelligence framework. While individual subsystems produce experiment
//! results and effect measurements, this engine aggregates everything into
//! a coherent, system-wide statistical picture.
//!
//! ## Capabilities
//!
//! - **Meta-analysis** across all experiment results from every subsystem
//! - **Cross-subsystem synthesis** combining disjoint statistical signals
//! - **Global effect-size estimation** with heterogeneity detection
//! - **System significance testing** using permutation-based methods
//! - **Power landscape mapping** showing where more data is needed
//! - **Completeness scoring** to ensure no research dimension is neglected
//!
//! The engine that turns thousands of tiny measurements into system wisdom.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_RESULTS: usize = 4096;
const MAX_SUBSYSTEMS: usize = 32;
const MAX_EFFECT_RECORDS: usize = 1024;
const MAX_POWER_CELLS: usize = 256;
const META_WINDOW: usize = 512;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const SIGNIFICANCE_ALPHA: f32 = 0.05;
const HETEROGENEITY_THRESHOLD: f32 = 0.35;
const EFFECT_SIZE_FLOOR: f32 = 0.01;
const POWER_MINIMUM: f32 = 0.80;
const COMPLETENESS_TARGET: f32 = 0.95;
const SYNTHESIS_DECAY: f32 = 0.97;
const PERMUTATION_ROUNDS: u64 = 1000;

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
// TYPES
// ============================================================================

/// Which subsystem produced a result
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceSubsystem {
    Bridge,
    Application,
    Cooperation,
    Memory,
    Scheduler,
    Ipc,
    Trust,
    Energy,
    FileSystem,
    Networking,
}

/// Category of a research result
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResultCategory {
    Performance,
    Latency,
    Throughput,
    Fairness,
    Reliability,
    Efficiency,
    Scalability,
    Security,
}

/// A single experiment result ingested for meta-analysis
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub id: u64,
    pub source: SourceSubsystem,
    pub category: ResultCategory,
    pub effect_size: f32,
    pub variance: f32,
    pub sample_size: u64,
    pub confidence: f32,
    pub tick: u64,
    pub hash: u64,
}

/// Meta-analysis outcome for a specific category
#[derive(Debug, Clone)]
pub struct MetaAnalysisOutcome {
    pub category: ResultCategory,
    pub pooled_effect: f32,
    pub heterogeneity_i2: f32,
    pub confidence_lower: f32,
    pub confidence_upper: f32,
    pub contributing_results: u64,
    pub subsystem_count: u64,
    pub is_significant: bool,
    pub tick: u64,
}

/// Cross-subsystem synthesis record
#[derive(Debug, Clone)]
pub struct CrossSynthesisRecord {
    pub id: u64,
    pub subsystem_a: SourceSubsystem,
    pub subsystem_b: SourceSubsystem,
    pub combined_effect: f32,
    pub interaction_term: f32,
    pub confidence: f32,
    pub tick: u64,
}

/// Global effect-size estimate
#[derive(Debug, Clone)]
pub struct GlobalEffectEstimate {
    pub category: ResultCategory,
    pub global_effect: f32,
    pub weight_sum: f32,
    pub heterogeneity: f32,
    pub prediction_interval: (f32, f32),
    pub tau_squared: f32,
    pub updated_tick: u64,
}

/// A cell in the power landscape
#[derive(Debug, Clone)]
pub struct PowerCell {
    pub subsystem: SourceSubsystem,
    pub category: ResultCategory,
    pub current_power: f32,
    pub samples_for_target: u64,
    pub current_samples: u64,
    pub deficit: f32,
}

/// Completeness dimension
#[derive(Debug, Clone)]
pub struct CompletenessDimension {
    pub subsystem: SourceSubsystem,
    pub category: ResultCategory,
    pub coverage: f32,
    pub last_result_tick: u64,
    pub gap_severity: f32,
}

/// Running statistics for the analysis engine
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AnalysisStats {
    pub total_results_ingested: u64,
    pub meta_analyses_run: u64,
    pub cross_syntheses: u64,
    pub global_effect_updates: u64,
    pub avg_pooled_effect_ema: f32,
    pub avg_heterogeneity_ema: f32,
    pub system_significance_rate: f32,
    pub power_landscape_coverage: f32,
    pub completeness_score: f32,
    pub last_analysis_tick: u64,
    pub permutation_tests_run: u64,
    pub significant_findings: u64,
}

// ============================================================================
// HOLISTIC ANALYSIS ENGINE
// ============================================================================

/// System-wide statistical meta-analysis engine
pub struct HolisticAnalysisEngine {
    results: VecDeque<AnalysisResult>,
    meta_outcomes: BTreeMap<u64, MetaAnalysisOutcome>,
    cross_syntheses: BTreeMap<u64, CrossSynthesisRecord>,
    global_effects: BTreeMap<u64, GlobalEffectEstimate>,
    power_landscape: Vec<PowerCell>,
    completeness_map: BTreeMap<u64, CompletenessDimension>,
    subsystem_weights: LinearMap<f32, 64>,
    category_weights: LinearMap<f32, 64>,
    rng_state: u64,
    tick: u64,
    stats: AnalysisStats,
}

impl HolisticAnalysisEngine {
    /// Create a new holistic analysis engine
    pub fn new(seed: u64) -> Self {
        Self {
            results: VecDeque::new(),
            meta_outcomes: BTreeMap::new(),
            cross_syntheses: BTreeMap::new(),
            global_effects: BTreeMap::new(),
            power_landscape: Vec::new(),
            completeness_map: BTreeMap::new(),
            subsystem_weights: LinearMap::new(),
            category_weights: LinearMap::new(),
            rng_state: seed | 1,
            tick: 0,
            stats: AnalysisStats {
                total_results_ingested: 0,
                meta_analyses_run: 0,
                cross_syntheses: 0,
                global_effect_updates: 0,
                avg_pooled_effect_ema: 0.0,
                avg_heterogeneity_ema: 0.0,
                system_significance_rate: 0.0,
                power_landscape_coverage: 0.0,
                completeness_score: 0.0,
                last_analysis_tick: 0,
                permutation_tests_run: 0,
                significant_findings: 0,
            },
        }
    }

    /// Ingest a result from any subsystem
    pub fn ingest_result(&mut self, source: SourceSubsystem, cat: ResultCategory,
                         effect: f32, variance: f32, sample_size: u64, confidence: f32) {
        let hash = fnv1a_hash(
            &[source as u8, cat as u8, (self.tick & 0xFF) as u8],
        );
        let id = self.stats.total_results_ingested;
        let result = AnalysisResult {
            id, source, category: cat, effect_size: effect,
            variance, sample_size, confidence, tick: self.tick, hash,
        };
        if self.results.len() >= MAX_RESULTS {
            self.results.pop_front();
        }
        self.results.push_back(result);
        self.stats.total_results_ingested += 1;
    }

    /// Run meta-analysis across all results for a given category
    pub fn meta_analysis(&mut self, cat: ResultCategory) -> MetaAnalysisOutcome {
        let relevant: Vec<&AnalysisResult> = self.results.iter()
            .filter(|r| r.category == cat && r.variance > 0.0)
            .collect();
        let n = relevant.len() as u64;
        if n == 0 {
            return MetaAnalysisOutcome {
                category: cat, pooled_effect: 0.0, heterogeneity_i2: 0.0,
                confidence_lower: 0.0, confidence_upper: 0.0,
                contributing_results: 0, subsystem_count: 0,
                is_significant: false, tick: self.tick,
            };
        }
        let mut weight_sum = 0.0f32;
        let mut weighted_effect = 0.0f32;
        let mut subsystems_seen: Vec<SourceSubsystem> = Vec::new();
        for r in &relevant {
            let w = 1.0 / (r.variance + 0.001);
            weight_sum += w;
            weighted_effect += w * r.effect_size;
            if !subsystems_seen.contains(&r.source) {
                subsystems_seen.push(r.source);
            }
        }
        let pooled = if weight_sum > 0.0 { weighted_effect / weight_sum } else { 0.0 };
        let mut q_stat = 0.0f32;
        for r in &relevant {
            let w = 1.0 / (r.variance + 0.001);
            let diff = r.effect_size - pooled;
            q_stat += w * diff * diff;
        }
        let df = if n > 1 { (n - 1) as f32 } else { 1.0 };
        let i_squared = if q_stat > df { (q_stat - df) / q_stat } else { 0.0 };
        let se = if weight_sum > 0.0 { (1.0 / weight_sum).sqrt() } else { 1.0 };
        let z = 1.96;
        let ci_lower = pooled - z * se;
        let ci_upper = pooled + z * se;
        let is_sig = pooled.abs() > z * se && pooled.abs() > EFFECT_SIZE_FLOOR;
        let outcome = MetaAnalysisOutcome {
            category: cat,
            pooled_effect: pooled,
            heterogeneity_i2: i_squared,
            confidence_lower: ci_lower,
            confidence_upper: ci_upper,
            contributing_results: n,
            subsystem_count: subsystems_seen.len() as u64,
            is_significant: is_sig,
            tick: self.tick,
        };
        self.stats.avg_pooled_effect_ema = self.stats.avg_pooled_effect_ema
            * (1.0 - EMA_ALPHA) + pooled.abs() * EMA_ALPHA;
        self.stats.avg_heterogeneity_ema = self.stats.avg_heterogeneity_ema
            * (1.0 - EMA_ALPHA) + i_squared * EMA_ALPHA;
        if is_sig { self.stats.significant_findings += 1; }
        self.stats.meta_analyses_run += 1;
        let key = fnv1a_hash(&[cat as u8, (self.tick & 0xFF) as u8]);
        self.meta_outcomes.insert(key, outcome.clone());
        outcome
    }

    /// Cross-subsystem synthesis — combine signals from two subsystems
    pub fn cross_subsystem_synthesis(&mut self, a: SourceSubsystem, b: SourceSubsystem)
        -> CrossSynthesisRecord
    {
        let results_a: Vec<&AnalysisResult> = self.results.iter()
            .filter(|r| r.source == a).collect();
        let results_b: Vec<&AnalysisResult> = self.results.iter()
            .filter(|r| r.source == b).collect();
        let mean_a = if results_a.is_empty() { 0.0 } else {
            results_a.iter().map(|r| r.effect_size).sum::<f32>() / results_a.len() as f32
        };
        let mean_b = if results_b.is_empty() { 0.0 } else {
            results_b.iter().map(|r| r.effect_size).sum::<f32>() / results_b.len() as f32
        };
        let combined = (mean_a + mean_b) * 0.5;
        let interaction = (mean_a * mean_b).abs().sqrt();
        let noise = xorshift_f32(&mut self.rng_state) * 0.02;
        let conf = ((results_a.len() + results_b.len()) as f32 /
            (MAX_RESULTS as f32 * 0.1)).min(1.0) * (1.0 - noise);
        let id = self.stats.cross_syntheses;
        let record = CrossSynthesisRecord {
            id, subsystem_a: a, subsystem_b: b,
            combined_effect: combined, interaction_term: interaction,
            confidence: conf, tick: self.tick,
        };
        let key = fnv1a_hash(&[a as u8, b as u8, (self.tick & 0xFF) as u8]);
        self.cross_syntheses.insert(key, record.clone());
        self.stats.cross_syntheses += 1;
        record
    }

    /// Compute the global effect size for a category using random-effects model
    pub fn global_effect_size(&mut self, cat: ResultCategory) -> GlobalEffectEstimate {
        let relevant: Vec<&AnalysisResult> = self.results.iter()
            .filter(|r| r.category == cat && r.variance > 0.0).collect();
        if relevant.is_empty() {
            return GlobalEffectEstimate {
                category: cat, global_effect: 0.0, weight_sum: 0.0,
                heterogeneity: 0.0, prediction_interval: (0.0, 0.0),
                tau_squared: 0.0, updated_tick: self.tick,
            };
        }
        let mut w_sum = 0.0f32;
        let mut we_sum = 0.0f32;
        let mut w2_sum = 0.0f32;
        for r in &relevant {
            let w = 1.0 / (r.variance + 0.001);
            w_sum += w;
            we_sum += w * r.effect_size;
            w2_sum += w * w;
        }
        let fixed_est = if w_sum > 0.0 { we_sum / w_sum } else { 0.0 };
        let mut q = 0.0f32;
        for r in &relevant {
            let w = 1.0 / (r.variance + 0.001);
            q += w * (r.effect_size - fixed_est) * (r.effect_size - fixed_est);
        }
        let k = relevant.len() as f32;
        let c = w_sum - w2_sum / w_sum;
        let tau2 = if q > k - 1.0 && c > 0.0 { (q - (k - 1.0)) / c } else { 0.0 };
        let mut rw_sum = 0.0f32;
        let mut rwe_sum = 0.0f32;
        for r in &relevant {
            let rw = 1.0 / (r.variance + tau2 + 0.001);
            rw_sum += rw;
            rwe_sum += rw * r.effect_size;
        }
        let global = if rw_sum > 0.0 { rwe_sum / rw_sum } else { 0.0 };
        let se = if rw_sum > 0.0 { (1.0 / rw_sum).sqrt() } else { 1.0 };
        let pred_se = (se * se + tau2).sqrt();
        let estimate = GlobalEffectEstimate {
            category: cat,
            global_effect: global,
            weight_sum: rw_sum,
            heterogeneity: if q > 0.0 { ((q - (k - 1.0)).max(0.0)) / q } else { 0.0 },
            prediction_interval: (global - 1.96 * pred_se, global + 1.96 * pred_se),
            tau_squared: tau2,
            updated_tick: self.tick,
        };
        let key = fnv1a_hash(&[cat as u8, 0xAA, (self.tick & 0xFF) as u8]);
        self.global_effects.insert(key, estimate.clone());
        self.stats.global_effect_updates += 1;
        estimate
    }

    /// System-wide significance testing using permutation approach
    pub fn system_significance(&mut self) -> f32 {
        if self.results.len() < 4 {
            return 0.0;
        }
        let observed_mean: f32 = self.results.iter()
            .map(|r| r.effect_size).sum::<f32>() / self.results.len() as f32;
        let mut extreme_count = 0u64;
        let n = self.results.len();
        let rounds = PERMUTATION_ROUNDS.min(n as u64 * 10);
        for _ in 0..rounds {
            let idx_a = (xorshift64(&mut self.rng_state) as usize) % n;
            let idx_b = (xorshift64(&mut self.rng_state) as usize) % n;
            let perm_mean = (self.results[idx_a].effect_size
                + self.results[idx_b].effect_size) * 0.5;
            if perm_mean.abs() >= observed_mean.abs() {
                extreme_count += 1;
            }
            self.stats.permutation_tests_run += 1;
        }
        let p_value = extreme_count as f32 / rounds as f32;
        let sig_rate = if p_value < SIGNIFICANCE_ALPHA { 1.0 } else { 0.0 };
        self.stats.system_significance_rate = self.stats.system_significance_rate
            * (1.0 - EMA_ALPHA) + sig_rate * EMA_ALPHA;
        self.stats.last_analysis_tick = self.tick;
        p_value
    }

    /// Map the power landscape — where do we need more data?
    pub fn power_landscape(&mut self) -> Vec<PowerCell> {
        let subsystems = [
            SourceSubsystem::Bridge, SourceSubsystem::Application,
            SourceSubsystem::Cooperation, SourceSubsystem::Memory,
            SourceSubsystem::Scheduler, SourceSubsystem::Ipc,
            SourceSubsystem::Trust, SourceSubsystem::Energy,
        ];
        let categories = [
            ResultCategory::Performance, ResultCategory::Latency,
            ResultCategory::Throughput, ResultCategory::Fairness,
            ResultCategory::Reliability, ResultCategory::Efficiency,
        ];
        let mut landscape = Vec::new();
        for &sub in &subsystems {
            for &cat in &categories {
                let matching: Vec<&AnalysisResult> = self.results.iter()
                    .filter(|r| r.source == sub && r.category == cat).collect();
                let current_n = matching.len() as u64;
                let avg_var = if matching.is_empty() { 1.0 } else {
                    matching.iter().map(|r| r.variance).sum::<f32>()
                        / matching.len() as f32
                };
                let needed = if avg_var > 0.001 {
                    ((1.96 * 1.96 * avg_var) / (EFFECT_SIZE_FLOOR * EFFECT_SIZE_FLOOR))
                        as u64
                } else { 10 };
                let current_power = (current_n as f32 / needed.max(1) as f32).min(1.0);
                let deficit = (POWER_MINIMUM - current_power).max(0.0);
                landscape.push(PowerCell {
                    subsystem: sub, category: cat, current_power,
                    samples_for_target: needed, current_samples: current_n, deficit,
                });
            }
        }
        let coverage = if landscape.is_empty() { 0.0 } else {
            landscape.iter().filter(|c| c.current_power >= POWER_MINIMUM).count() as f32
                / landscape.len() as f32
        };
        self.stats.power_landscape_coverage = coverage;
        if landscape.len() > MAX_POWER_CELLS {
            landscape.truncate(MAX_POWER_CELLS);
        }
        self.power_landscape = landscape.clone();
        landscape
    }

    /// Compute analysis completeness across all dimensions
    pub fn analysis_completeness(&mut self) -> f32 {
        let subsystems = [
            SourceSubsystem::Bridge, SourceSubsystem::Application,
            SourceSubsystem::Cooperation, SourceSubsystem::Memory,
            SourceSubsystem::Scheduler, SourceSubsystem::Ipc,
        ];
        let categories = [
            ResultCategory::Performance, ResultCategory::Latency,
            ResultCategory::Throughput, ResultCategory::Fairness,
        ];
        let mut covered = 0u64;
        let mut total = 0u64;
        for &sub in &subsystems {
            for &cat in &categories {
                total += 1;
                let has_result = self.results.iter()
                    .any(|r| r.source == sub && r.category == cat);
                let coverage = if has_result {
                    let count = self.results.iter()
                        .filter(|r| r.source == sub && r.category == cat).count();
                    (count as f32 / 10.0).min(1.0)
                } else { 0.0 };
                let gap = (1.0 - coverage).max(0.0);
                let key = fnv1a_hash(&[sub as u8, cat as u8, 0xCC]);
                self.completeness_map.insert(key, CompletenessDimension {
                    subsystem: sub, category: cat, coverage,
                    last_result_tick: self.tick, gap_severity: gap,
                });
                if coverage >= COMPLETENESS_TARGET { covered += 1; }
            }
        }
        let completeness = if total > 0 { covered as f32 / total as f32 } else { 0.0 };
        self.stats.completeness_score = completeness;
        completeness
    }

    /// Advance the engine tick
    #[inline(always)]
    pub fn tick(&mut self) {
        self.tick += 1;
    }

    /// Get current statistics
    #[inline(always)]
    pub fn stats(&self) -> &AnalysisStats {
        &self.stats
    }

    /// Get total ingested results count
    #[inline(always)]
    pub fn result_count(&self) -> usize {
        self.results.len()
    }

    /// Get meta-analysis outcomes
    #[inline(always)]
    pub fn meta_outcomes(&self) -> &BTreeMap<u64, MetaAnalysisOutcome> {
        &self.meta_outcomes
    }

    /// Get cross-subsystem synthesis records
    #[inline(always)]
    pub fn cross_synthesis_records(&self) -> &BTreeMap<u64, CrossSynthesisRecord> {
        &self.cross_syntheses
    }

    /// Get global effect estimates
    #[inline(always)]
    pub fn global_effect_estimates(&self) -> &BTreeMap<u64, GlobalEffectEstimate> {
        &self.global_effects
    }

    /// Get the power landscape
    #[inline(always)]
    pub fn power_cells(&self) -> &[PowerCell] {
        &self.power_landscape
    }

    /// Get completeness map
    #[inline(always)]
    pub fn completeness_dimensions(&self) -> &BTreeMap<u64, CompletenessDimension> {
        &self.completeness_map
    }
}
