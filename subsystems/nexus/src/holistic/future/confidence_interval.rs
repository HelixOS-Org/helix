// SPDX-License-Identifier: GPL-2.0
//! # Holistic Confidence Interval — System-Wide Uncertainty Quantification
//!
//! Every system prediction carries uncertainty. This module quantifies that
//! uncertainty at the **global** level: each prediction from every subsystem
//! gets rigorous confidence bounds, and those bounds propagate through the
//! cross-subsystem dependency graph so the kernel knows not just *what* it
//! predicts, but *how sure* it is, everywhere, all at once.
//!
//! ## Capabilities
//!
//! - Per-prediction confidence intervals (lower bound, upper bound, width)
//! - Global uncertainty aggregation across all subsystems
//! - Uncertainty propagation through causal/dependency chains
//! - Calibration auditing: are our intervals actually well-calibrated?
//! - Interval reliability scoring: historical hit rate tracking
//! - Uncertainty budget allocation across subsystem models

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_PREDICTION_ENTRIES: usize = 4096;
const MAX_CALIBRATION_HISTORY: usize = 1024;
const MAX_BUDGET_ENTRIES: usize = 64;
const MAX_PROPAGATION_DEPTH: usize = 32;
const DEFAULT_CONFIDENCE_LEVEL: f32 = 0.95;
const MIN_INTERVAL_WIDTH: f32 = 0.001;
const CALIBRATION_WINDOW: usize = 256;
const EMA_ALPHA: f32 = 0.12;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

// ============================================================================
// HELPER FUNCTIONS
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

fn ema_update(current: f32, sample: f32) -> f32 {
    EMA_ALPHA * sample + (1.0 - EMA_ALPHA) * current
}

// ============================================================================
// DOMAIN TYPES
// ============================================================================

/// Subsystem model identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModelSource {
    Scheduler,
    Memory,
    Io,
    Network,
    Thermal,
    Power,
    Ipc,
    FileSystem,
    Security,
    Ensemble,
    MonteCarlo,
    Causal,
}

/// Confidence level for interval computation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfidenceLevel {
    Percent90,
    Percent95,
    Percent99,
    Percent999,
}

impl ConfidenceLevel {
    fn z_factor(self) -> f32 {
        match self {
            Self::Percent90 => 1.645,
            Self::Percent95 => 1.960,
            Self::Percent99 => 2.576,
            Self::Percent999 => 3.291,
        }
    }

    fn nominal(self) -> f32 {
        match self {
            Self::Percent90 => 0.90,
            Self::Percent95 => 0.95,
            Self::Percent99 => 0.99,
            Self::Percent999 => 0.999,
        }
    }
}

// ============================================================================
// CONFIDENCE INTERVAL STRUCTURES
// ============================================================================

/// A single prediction with confidence interval
#[derive(Debug, Clone)]
pub struct PredictionCI {
    pub prediction_id: u64,
    pub source: ModelSource,
    pub point_estimate: f32,
    pub lower_bound: f32,
    pub upper_bound: f32,
    pub interval_width: f32,
    pub confidence_level: ConfidenceLevel,
    pub timestamp_us: u64,
    pub actual_value: Option<f32>,
    pub hit: Option<bool>,
}

/// Global uncertainty aggregation result
#[derive(Debug, Clone)]
pub struct GlobalUncertainty {
    pub total_predictions: usize,
    pub mean_interval_width: f32,
    pub max_interval_width: f32,
    pub min_interval_width: f32,
    pub uncertainty_by_source: BTreeMap<u64, f32>,
    pub overall_uncertainty: f32,
    pub entropy_estimate: f32,
    pub timestamp_us: u64,
}

/// Uncertainty propagation result through dependency chain
#[derive(Debug, Clone)]
pub struct UncertaintyPropagation {
    pub root_source: ModelSource,
    pub root_uncertainty: f32,
    pub propagated_uncertainties: Vec<PropagatedUncertainty>,
    pub total_amplification: f32,
    pub chain_depth: usize,
    pub worst_amplification_path: Vec<ModelSource>,
}

/// A single node in the propagation chain
#[derive(Debug, Clone)]
pub struct PropagatedUncertainty {
    pub source: ModelSource,
    pub input_uncertainty: f32,
    pub output_uncertainty: f32,
    pub amplification_factor: f32,
    pub depth: usize,
}

/// Calibration audit result
#[derive(Debug, Clone)]
pub struct CalibrationAudit {
    pub source: ModelSource,
    pub confidence_level: ConfidenceLevel,
    pub expected_coverage: f32,
    pub actual_coverage: f32,
    pub calibration_error: f32,
    pub is_well_calibrated: bool,
    pub sample_size: usize,
    pub recommendation: CalibrationAction,
}

/// Calibration action recommendation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationAction {
    NoChange,
    WidenIntervals,
    NarrowIntervals,
    RecalibrateModel,
    InsufficientData,
}

/// Interval reliability report
#[derive(Debug, Clone)]
pub struct IntervalReliability {
    pub source: ModelSource,
    pub total_predictions: usize,
    pub hits: usize,
    pub misses: usize,
    pub hit_rate: f32,
    pub running_hit_rate: f32,
    pub reliability_score: f32,
    pub trend: f32,
}

/// Uncertainty budget for a subsystem model
#[derive(Debug, Clone)]
pub struct UncertaintyBudgetEntry {
    pub source: ModelSource,
    pub allocated_uncertainty: f32,
    pub consumed_uncertainty: f32,
    pub utilization: f32,
    pub over_budget: bool,
}

/// Full uncertainty budget report
#[derive(Debug, Clone)]
pub struct UncertaintyBudget {
    pub entries: Vec<UncertaintyBudgetEntry>,
    pub total_allocated: f32,
    pub total_consumed: f32,
    pub system_utilization: f32,
    pub over_budget_count: usize,
}

// ============================================================================
// STATISTICS
// ============================================================================

/// Runtime statistics for the confidence interval engine
#[derive(Debug, Clone)]
pub struct ConfidenceIntervalStats {
    pub ci_computed: u64,
    pub global_uncertainty_queries: u64,
    pub propagations_run: u64,
    pub calibration_audits: u64,
    pub reliability_queries: u64,
    pub budget_evaluations: u64,
    pub avg_interval_width: f32,
    pub avg_hit_rate: f32,
    pub avg_calibration_error: f32,
    pub avg_uncertainty: f32,
}

impl ConfidenceIntervalStats {
    fn new() -> Self {
        Self {
            ci_computed: 0,
            global_uncertainty_queries: 0,
            propagations_run: 0,
            calibration_audits: 0,
            reliability_queries: 0,
            budget_evaluations: 0,
            avg_interval_width: 0.0,
            avg_hit_rate: 0.0,
            avg_calibration_error: 0.0,
            avg_uncertainty: 0.0,
        }
    }
}

// ============================================================================
// HOLISTIC CONFIDENCE INTERVAL ENGINE
// ============================================================================

/// System-wide confidence interval and uncertainty engine
pub struct HolisticConfidenceInterval {
    predictions: Vec<PredictionCI>,
    source_variances: BTreeMap<u64, f32>,
    source_counts: BTreeMap<u64, u64>,
    dependency_graph: BTreeMap<u64, Vec<u64>>,
    calibration_history: Vec<(ModelSource, bool)>,
    budget_allocations: BTreeMap<u64, f32>,
    rng_state: u64,
    next_prediction_id: u64,
    stats: ConfidenceIntervalStats,
    generation: u64,
}

impl HolisticConfidenceInterval {
    /// Create a new holistic confidence interval engine
    pub fn new(seed: u64) -> Self {
        Self {
            predictions: Vec::new(),
            source_variances: BTreeMap::new(),
            source_counts: BTreeMap::new(),
            dependency_graph: BTreeMap::new(),
            calibration_history: Vec::new(),
            budget_allocations: BTreeMap::new(),
            rng_state: seed ^ 0x1234_5678_ABCD_EF00,
            next_prediction_id: 1,
            stats: ConfidenceIntervalStats::new(),
            generation: 0,
        }
    }

    /// Register a model dependency: `from` feeds predictions into `to`
    pub fn register_dependency(&mut self, from: ModelSource, to: ModelSource) {
        let fk = fnv1a_hash(&[from as u8]);
        let tk = fnv1a_hash(&[to as u8]);
        self.dependency_graph.entry(fk).or_insert_with(Vec::new).push(tk);
    }

    /// Allocate an uncertainty budget for a model source
    pub fn allocate_budget(&mut self, source: ModelSource, budget: f32) {
        let key = fnv1a_hash(&[source as u8]);
        self.budget_allocations.insert(key, budget.max(0.0));
    }

    /// Compute a confidence interval for a single prediction
    pub fn system_prediction_ci(
        &mut self,
        source: ModelSource,
        point_estimate: f32,
        sample_variance: f32,
        level: ConfidenceLevel,
        timestamp_us: u64,
    ) -> PredictionCI {
        self.stats.ci_computed += 1;
        self.generation += 1;

        let sk = fnv1a_hash(&[source as u8]);
        let base_var = self.source_variances.get(&sk).copied().unwrap_or(sample_variance);
        let blended_var = ema_update(base_var, sample_variance);
        self.source_variances.insert(sk, blended_var);

        let count = self.source_counts.entry(sk).or_insert(0);
        *count += 1;

        let std_dev = blended_var.max(0.0).sqrt();
        let z = level.z_factor();
        let half_width = z * std_dev;
        let lower = point_estimate - half_width;
        let upper = point_estimate + half_width;
        let width = (upper - lower).max(MIN_INTERVAL_WIDTH);

        self.stats.avg_interval_width = ema_update(self.stats.avg_interval_width, width);

        let id = self.next_prediction_id;
        self.next_prediction_id += 1;

        let ci = PredictionCI {
            prediction_id: id,
            source,
            point_estimate,
            lower_bound: lower,
            upper_bound: upper,
            interval_width: width,
            confidence_level: level,
            timestamp_us,
            actual_value: None,
            hit: None,
        };

        if self.predictions.len() < MAX_PREDICTION_ENTRIES {
            self.predictions.push(ci.clone());
        }
        ci
    }

    /// Record the actual value for a past prediction (for calibration)
    pub fn record_actual(&mut self, prediction_id: u64, actual: f32) {
        for pred in &mut self.predictions {
            if pred.prediction_id == prediction_id {
                pred.actual_value = Some(actual);
                let hit = actual >= pred.lower_bound && actual <= pred.upper_bound;
                pred.hit = Some(hit);
                if self.calibration_history.len() < MAX_CALIBRATION_HISTORY {
                    self.calibration_history.push((pred.source, hit));
                }
                break;
            }
        }
    }

    /// Compute global uncertainty across all prediction sources
    pub fn global_uncertainty(&mut self, timestamp_us: u64) -> GlobalUncertainty {
        self.stats.global_uncertainty_queries += 1;

        let mut by_source: BTreeMap<u64, (f32, usize)> = BTreeMap::new();
        let mut total_width = 0.0_f32;
        let mut max_width = 0.0_f32;
        let mut min_width = f32::MAX;
        let count = self.predictions.len();

        for pred in &self.predictions {
            total_width += pred.interval_width;
            if pred.interval_width > max_width {
                max_width = pred.interval_width;
            }
            if pred.interval_width < min_width {
                min_width = pred.interval_width;
            }
            let sk = fnv1a_hash(&[pred.source as u8]);
            let entry = by_source.entry(sk).or_insert((0.0, 0));
            entry.0 += pred.interval_width;
            entry.1 += 1;
        }

        let mean_width = if count > 0 { total_width / count as f32 } else { 0.0 };
        if min_width == f32::MAX {
            min_width = 0.0;
        }

        let mut source_uncertainty: BTreeMap<u64, f32> = BTreeMap::new();
        for (k, (sum, cnt)) in &by_source {
            source_uncertainty.insert(*k, if *cnt > 0 { sum / *cnt as f32 } else { 0.0 });
        }

        let entropy = self.estimate_entropy(&source_uncertainty);
        self.stats.avg_uncertainty = ema_update(self.stats.avg_uncertainty, mean_width);

        GlobalUncertainty {
            total_predictions: count,
            mean_interval_width: mean_width,
            max_interval_width: max_width,
            min_interval_width: min_width,
            uncertainty_by_source: source_uncertainty,
            overall_uncertainty: mean_width,
            entropy_estimate: entropy,
            timestamp_us,
        }
    }

    /// Propagate uncertainty through the dependency graph from a root source
    pub fn uncertainty_propagation(&mut self, root: ModelSource) -> UncertaintyPropagation {
        self.stats.propagations_run += 1;
        let rk = fnv1a_hash(&[root as u8]);
        let root_var = self.source_variances.get(&rk).copied().unwrap_or(0.01);
        let root_unc = root_var.sqrt();

        let mut propagated: Vec<PropagatedUncertainty> = Vec::new();
        let mut frontier: Vec<(u64, f32, usize)> = Vec::new();
        let mut visited: BTreeMap<u64, bool> = BTreeMap::new();
        let mut total_amp = 1.0_f32;
        let mut worst_path: Vec<ModelSource> = Vec::new();
        let mut worst_amp = 0.0_f32;

        frontier.push((rk, root_unc, 0));
        visited.insert(rk, true);

        while let Some((current_key, input_unc, depth)) = frontier.pop() {
            if depth >= MAX_PROPAGATION_DEPTH {
                continue;
            }
            let dependents = self.dependency_graph.get(&current_key).cloned().unwrap_or_default();
            for &dep_key in &dependents {
                if visited.contains_key(&dep_key) {
                    continue;
                }
                visited.insert(dep_key, true);

                let noise = (xorshift64(&mut self.rng_state) % 50) as f32 / 100.0 + 0.8;
                let amp_factor = noise;
                let output_unc = input_unc * amp_factor;
                total_amp *= amp_factor;

                let source = self.key_to_source(dep_key);
                propagated.push(PropagatedUncertainty {
                    source,
                    input_uncertainty: input_unc,
                    output_uncertainty: output_unc,
                    amplification_factor: amp_factor,
                    depth: depth + 1,
                });

                if amp_factor > worst_amp {
                    worst_amp = amp_factor;
                    worst_path.push(source);
                }
                frontier.push((dep_key, output_unc, depth + 1));
            }
        }

        UncertaintyPropagation {
            root_source: root,
            root_uncertainty: root_unc,
            propagated_uncertainties: propagated,
            total_amplification: total_amp,
            chain_depth: visited.len(),
            worst_amplification_path: worst_path,
        }
    }

    /// Audit the calibration of a specific model's confidence intervals
    pub fn calibration_audit(
        &mut self,
        source: ModelSource,
        level: ConfidenceLevel,
    ) -> CalibrationAudit {
        self.stats.calibration_audits += 1;
        let relevant: Vec<&(ModelSource, bool)> = self
            .calibration_history
            .iter()
            .filter(|(s, _)| *s == source)
            .collect();

        let sample_size = relevant.len();
        let hits = relevant.iter().filter(|(_, h)| *h).count();
        let actual_coverage = if sample_size > 0 {
            hits as f32 / sample_size as f32
        } else {
            0.0
        };
        let expected = level.nominal();
        let error = (actual_coverage - expected).abs();
        let well_calibrated = error < 0.05;

        let action = if sample_size < 30 {
            CalibrationAction::InsufficientData
        } else if actual_coverage < expected - 0.05 {
            CalibrationAction::WidenIntervals
        } else if actual_coverage > expected + 0.05 {
            CalibrationAction::NarrowIntervals
        } else if error > 0.10 {
            CalibrationAction::RecalibrateModel
        } else {
            CalibrationAction::NoChange
        };

        self.stats.avg_calibration_error = ema_update(self.stats.avg_calibration_error, error);

        CalibrationAudit {
            source,
            confidence_level: level,
            expected_coverage: expected,
            actual_coverage,
            calibration_error: error,
            is_well_calibrated: well_calibrated,
            sample_size,
            recommendation: action,
        }
    }

    /// Query interval reliability for a specific model source
    pub fn interval_reliability(&mut self, source: ModelSource) -> IntervalReliability {
        self.stats.reliability_queries += 1;
        let relevant: Vec<&PredictionCI> = self
            .predictions
            .iter()
            .filter(|p| p.source == source && p.hit.is_some())
            .collect();

        let total = relevant.len();
        let hits = relevant.iter().filter(|p| p.hit == Some(true)).count();
        let misses = total - hits;
        let hit_rate = if total > 0 { hits as f32 / total as f32 } else { 0.0 };

        let recent = relevant.iter().rev().take(CALIBRATION_WINDOW);
        let recent_hits = recent.filter(|p| p.hit == Some(true)).count();
        let recent_total = relevant.len().min(CALIBRATION_WINDOW);
        let running = if recent_total > 0 {
            recent_hits as f32 / recent_total as f32
        } else {
            0.0
        };

        let reliability = hit_rate * 0.6 + running * 0.4;
        let trend = running - hit_rate;

        self.stats.avg_hit_rate = ema_update(self.stats.avg_hit_rate, hit_rate);

        IntervalReliability {
            source,
            total_predictions: total,
            hits,
            misses,
            hit_rate,
            running_hit_rate: running,
            reliability_score: reliability,
            trend,
        }
    }

    /// Compute uncertainty budget utilization across all models
    pub fn uncertainty_budget(&self) -> UncertaintyBudget {
        let mut entries: Vec<UncertaintyBudgetEntry> = Vec::new();
        let mut total_alloc = 0.0_f32;
        let mut total_consumed = 0.0_f32;
        let mut over_count = 0_usize;

        let sources = [
            ModelSource::Scheduler, ModelSource::Memory, ModelSource::Io,
            ModelSource::Network, ModelSource::Thermal, ModelSource::Power,
            ModelSource::Ipc, ModelSource::FileSystem, ModelSource::Security,
            ModelSource::Ensemble, ModelSource::MonteCarlo, ModelSource::Causal,
        ];

        for &src in &sources {
            let key = fnv1a_hash(&[src as u8]);
            let allocated = self.budget_allocations.get(&key).copied().unwrap_or(0.1);
            let consumed = self.source_variances.get(&key).copied().unwrap_or(0.0).sqrt();
            let utilization = if allocated > 0.0 { consumed / allocated } else { 0.0 };
            let over = consumed > allocated;
            if over {
                over_count += 1;
            }
            total_alloc += allocated;
            total_consumed += consumed;
            if entries.len() < MAX_BUDGET_ENTRIES {
                entries.push(UncertaintyBudgetEntry {
                    source: src,
                    allocated_uncertainty: allocated,
                    consumed_uncertainty: consumed,
                    utilization,
                    over_budget: over,
                });
            }
        }

        let sys_util = if total_alloc > 0.0 { total_consumed / total_alloc } else { 0.0 };

        UncertaintyBudget {
            entries,
            total_allocated: total_alloc,
            total_consumed: total_consumed,
            system_utilization: sys_util,
            over_budget_count: over_count,
        }
    }

    /// Get current statistics
    pub fn stats(&self) -> &ConfidenceIntervalStats {
        &self.stats
    }

    // ========================================================================
    // PRIVATE HELPERS
    // ========================================================================

    fn estimate_entropy(&self, source_unc: &BTreeMap<u64, f32>) -> f32 {
        let total: f32 = source_unc.values().sum();
        if total <= 0.0 {
            return 0.0;
        }
        let mut entropy = 0.0_f32;
        for v in source_unc.values() {
            let p = v / total;
            if p > 0.0 {
                entropy -= p * p.ln();
            }
        }
        entropy.max(0.0)
    }

    fn key_to_source(&self, key: u64) -> ModelSource {
        let sources = [
            ModelSource::Scheduler, ModelSource::Memory, ModelSource::Io,
            ModelSource::Network, ModelSource::Thermal, ModelSource::Power,
            ModelSource::Ipc, ModelSource::FileSystem, ModelSource::Security,
            ModelSource::Ensemble, ModelSource::MonteCarlo, ModelSource::Causal,
        ];
        for &s in &sources {
            if fnv1a_hash(&[s as u8]) == key {
                return s;
            }
        }
        ModelSource::Ensemble
    }
}
