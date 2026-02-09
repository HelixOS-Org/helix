// SPDX-License-Identifier: GPL-2.0
//! # Bridge Synthesis — Code/Strategy Synthesis from Research Discoveries
//!
//! Translates validated research discoveries into concrete optimization
//! strategies with tunable parameters. Each synthesized strategy is
//! versioned, can be applied atomically, and supports rollback to the
//! previous version. Integration testing verifies that a synthesized
//! strategy actually improves metrics before committing it live.
//!
//! The bridge that turns papers into performance.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_STRATEGIES: usize = 128;
const MAX_PARAMS: usize = 32;
const MAX_HISTORY: usize = 64;
const ROLLBACK_WINDOW: usize = 8;
const EMA_ALPHA: f32 = 0.12;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const MIN_IMPROVEMENT_FOR_COMMIT: f32 = 0.01;
const INTEGRATION_TRIALS: usize = 5;

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

// ============================================================================
// SYNTHESIS TYPES
// ============================================================================

/// Status of a synthesized strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StrategyStatus {
    Draft,
    Testing,
    Active,
    RolledBack,
    Retired,
}

/// A single parameter in a synthesized strategy
#[derive(Debug, Clone)]
pub struct SynthesizedParam {
    pub name: String,
    pub value: f32,
    pub source_discovery: u64,
    pub confidence: f32,
}

/// A synthesized optimization strategy
#[derive(Debug, Clone)]
pub struct SynthesizedStrategy {
    pub strategy_id: u64,
    pub name: String,
    pub version: u32,
    pub status: StrategyStatus,
    pub params: Vec<SynthesizedParam>,
    pub discovery_ids: Vec<u64>,
    pub baseline_metric: f32,
    pub current_metric: f32,
    pub improvement: f32,
    pub created_tick: u64,
    pub applied_tick: Option<u64>,
}

/// Integration test result
#[derive(Debug, Clone)]
pub struct IntegrationTestResult {
    pub strategy_id: u64,
    pub passed: bool,
    pub baseline_metric: f32,
    pub strategy_metric: f32,
    pub improvement: f32,
    pub trial_results: Vec<f32>,
    pub mean_improvement: f32,
    pub consistency: f32,
}

/// Impact assessment of a synthesis
#[derive(Debug, Clone)]
pub struct SynthesisImpact {
    pub strategy_id: u64,
    pub name: String,
    pub status: StrategyStatus,
    pub improvement: f32,
    pub param_count: usize,
    pub discovery_count: usize,
    pub version: u32,
    pub stability_score: f32,
}

/// History entry for rollback support
#[derive(Debug, Clone)]
struct HistoryEntry {
    strategy_id: u64,
    version: u32,
    params: Vec<SynthesizedParam>,
    metric_before: f32,
    tick: u64,
}

// ============================================================================
// SYNTHESIS STATS
// ============================================================================

/// Aggregate synthesis statistics
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct SynthesisStats {
    pub total_synthesized: u64,
    pub total_applied: u64,
    pub total_rolled_back: u64,
    pub total_retired: u64,
    pub total_integration_tests: u64,
    pub integration_pass_rate_ema: f32,
    pub avg_improvement_ema: f32,
    pub active_strategies: u32,
    pub avg_stability_ema: f32,
}

// ============================================================================
// VERSION TRACKER
// ============================================================================

/// Tracks strategy versions and rollback history
#[derive(Debug)]
struct VersionTracker {
    history: VecDeque<HistoryEntry>,
    version_map: LinearMap<u32, 64>,
}

impl VersionTracker {
    fn new() -> Self {
        Self {
            history: VecDeque::new(),
            version_map: LinearMap::new(),
        }
    }

    fn record(&mut self, strategy_id: u64, params: &[SynthesizedParam], metric: f32, tick: u64) {
        let version = self.version_map.entry(strategy_id).or_insert(0);
        *version += 1;
        self.history.push_back(HistoryEntry {
            strategy_id,
            version: *version,
            params: params.to_vec(),
            metric_before: metric,
            tick,
        });
        // Limit history size
        while self.history.len() > MAX_HISTORY {
            self.history.pop_front();
        }
    }

    fn previous_version(&self, strategy_id: u64) -> Option<&HistoryEntry> {
        self.history
            .iter()
            .rev()
            .filter(|h| h.strategy_id == strategy_id)
            .nth(1)
    }

    fn current_version(&self, strategy_id: u64) -> u32 {
        self.version_map.get(strategy_id).copied().unwrap_or(0)
    }

    fn rollback_available(&self, strategy_id: u64) -> bool {
        let entries: Vec<&HistoryEntry> = self
            .history
            .iter()
            .filter(|h| h.strategy_id == strategy_id)
            .collect();
        entries.len() >= 2
    }
}

// ============================================================================
// BRIDGE SYNTHESIS
// ============================================================================

/// Strategy synthesis engine from validated research discoveries
#[derive(Debug)]
#[repr(align(64))]
pub struct BridgeSynthesis {
    strategies: BTreeMap<u64, SynthesizedStrategy>,
    version_tracker: VersionTracker,
    rng_state: u64,
    stats: SynthesisStats,
}

impl BridgeSynthesis {
    /// Create a new synthesis engine
    pub fn new(seed: u64) -> Self {
        Self {
            strategies: BTreeMap::new(),
            version_tracker: VersionTracker::new(),
            rng_state: seed | 1,
            stats: SynthesisStats::default(),
        }
    }

    /// Synthesize a new strategy from one or more validated discoveries
    pub fn synthesize_strategy(
        &mut self,
        name: String,
        discovery_ids: Vec<u64>,
        params: Vec<SynthesizedParam>,
        baseline_metric: f32,
        tick: u64,
    ) -> SynthesizedStrategy {
        let id = fnv1a_hash(name.as_bytes()) ^ xorshift64(&mut self.rng_state);

        let strategy = SynthesizedStrategy {
            strategy_id: id,
            name: name.clone(),
            version: 1,
            status: StrategyStatus::Draft,
            params: params.iter().take(MAX_PARAMS).cloned().collect(),
            discovery_ids,
            baseline_metric,
            current_metric: baseline_metric,
            improvement: 0.0,
            created_tick: tick,
            applied_tick: None,
        };

        self.version_tracker
            .record(id, &strategy.params, baseline_metric, tick);
        self.strategies.insert(id, strategy.clone());
        self.stats.total_synthesized += 1;

        // Evict oldest retired if over capacity
        while self.strategies.len() > MAX_STRATEGIES {
            let retired = self
                .strategies
                .iter()
                .filter(|(_, s)| s.status == StrategyStatus::Retired)
                .min_by_key(|(_, s)| s.created_tick)
                .map(|(&k, _)| k);
            if let Some(k) = retired {
                self.strategies.remove(&k);
            } else {
                break;
            }
        }

        strategy
    }

    /// Apply a discovery's parameters to an existing strategy
    #[inline]
    pub fn apply_discovery(
        &mut self,
        strategy_id: u64,
        new_params: Vec<SynthesizedParam>,
        new_metric: f32,
        tick: u64,
    ) -> bool {
        let strategy = match self.strategies.get_mut(&strategy_id) {
            Some(s) => s,
            None => return false,
        };

        // Record previous state for rollback
        self.version_tracker
            .record(strategy_id, &strategy.params, strategy.current_metric, tick);

        strategy.params = new_params.into_iter().take(MAX_PARAMS).collect();
        strategy.current_metric = new_metric;
        strategy.improvement = if strategy.baseline_metric > 1e-10 {
            (new_metric - strategy.baseline_metric) / strategy.baseline_metric
        } else {
            0.0
        };
        strategy.version = self.version_tracker.current_version(strategy_id);
        strategy.status = StrategyStatus::Active;
        strategy.applied_tick = Some(tick);

        self.stats.total_applied += 1;
        self.stats.avg_improvement_ema =
            EMA_ALPHA * strategy.improvement + (1.0 - EMA_ALPHA) * self.stats.avg_improvement_ema;

        // Count active strategies
        self.stats.active_strategies = self
            .strategies
            .values()
            .filter(|s| s.status == StrategyStatus::Active)
            .count() as u32;

        true
    }

    /// Rollback a strategy to its previous version
    pub fn rollback_strategy(&mut self, strategy_id: u64) -> bool {
        if !self.version_tracker.rollback_available(strategy_id) {
            return false;
        }

        let prev = match self.version_tracker.previous_version(strategy_id) {
            Some(h) => h.clone(),
            None => return false,
        };

        let strategy = match self.strategies.get_mut(&strategy_id) {
            Some(s) => s,
            None => return false,
        };

        strategy.params = prev.params;
        strategy.current_metric = prev.metric_before;
        strategy.improvement = if strategy.baseline_metric > 1e-10 {
            (prev.metric_before - strategy.baseline_metric) / strategy.baseline_metric
        } else {
            0.0
        };
        strategy.status = StrategyStatus::RolledBack;
        strategy.version = prev.version;

        self.stats.total_rolled_back += 1;
        self.stats.active_strategies = self
            .strategies
            .values()
            .filter(|s| s.status == StrategyStatus::Active)
            .count() as u32;

        true
    }

    /// Assess the impact of a synthesized strategy
    pub fn synthesis_impact(&self, strategy_id: u64) -> Option<SynthesisImpact> {
        let strategy = self.strategies.get(&strategy_id)?;

        // Stability: how consistent is the improvement across versions
        let version_count = self.version_tracker.current_version(strategy_id);
        let version_entries: Vec<&HistoryEntry> = self
            .version_tracker
            .history
            .iter()
            .filter(|h| h.strategy_id == strategy_id)
            .collect();

        let stability = if version_entries.len() >= 2 {
            let metrics: Vec<f32> = version_entries.iter().map(|h| h.metric_before).collect();
            let mean = metrics.iter().sum::<f32>() / metrics.len() as f32;
            let variance = metrics
                .iter()
                .map(|&m| (m - mean) * (m - mean))
                .sum::<f32>()
                / metrics.len() as f32;
            let cv = if mean > 1e-10 {
                (variance / (mean * mean)).min(1.0)
            } else {
                1.0
            };
            1.0 - cv
        } else {
            0.5
        };

        Some(SynthesisImpact {
            strategy_id,
            name: strategy.name.clone(),
            status: strategy.status,
            improvement: strategy.improvement,
            param_count: strategy.params.len(),
            discovery_count: strategy.discovery_ids.len(),
            version: version_count,
            stability_score: stability.clamp(0.0, 1.0),
        })
    }

    /// Run integration test: compare baseline vs strategy across multiple trials
    pub fn integration_test(
        &mut self,
        strategy_id: u64,
        trial_metrics: Vec<(f32, f32)>,
    ) -> Option<IntegrationTestResult> {
        let strategy = self.strategies.get_mut(&strategy_id)?;
        strategy.status = StrategyStatus::Testing;

        if trial_metrics.is_empty() {
            return None;
        }

        let mut improvements: Vec<f32> = Vec::new();
        let mut baseline_sum: f32 = 0.0;
        let mut strategy_sum: f32 = 0.0;

        for &(baseline, strat_metric) in &trial_metrics {
            let imp = if baseline > 1e-10 {
                (strat_metric - baseline) / baseline
            } else {
                0.0
            };
            improvements.push(imp);
            baseline_sum += baseline;
            strategy_sum += strat_metric;
        }

        let n = trial_metrics.len() as f32;
        let mean_baseline = baseline_sum / n;
        let mean_strategy = strategy_sum / n;
        let mean_improvement = improvements.iter().sum::<f32>() / n;

        // Consistency: what fraction of trials showed improvement
        let positive = improvements.iter().filter(|&&i| i > 0.0).count();
        let consistency = positive as f32 / n;

        // Variance of improvements
        let var = improvements
            .iter()
            .map(|&i| (i - mean_improvement) * (i - mean_improvement))
            .sum::<f32>()
            / n.max(1.0);
        let cv = if abs_f32(mean_improvement) > 1e-10 {
            var / (mean_improvement * mean_improvement)
        } else {
            1.0
        };

        let passed =
            mean_improvement > MIN_IMPROVEMENT_FOR_COMMIT && consistency >= 0.6 && cv < 2.0;

        if passed {
            strategy.status = StrategyStatus::Active;
            strategy.current_metric = mean_strategy;
            strategy.improvement = mean_improvement;
        } else {
            strategy.status = StrategyStatus::Draft;
        }

        self.stats.total_integration_tests += 1;
        let pass_indicator = if passed { 1.0_f32 } else { 0.0 };
        self.stats.integration_pass_rate_ema =
            EMA_ALPHA * pass_indicator + (1.0 - EMA_ALPHA) * self.stats.integration_pass_rate_ema;
        self.stats.avg_stability_ema =
            EMA_ALPHA * consistency + (1.0 - EMA_ALPHA) * self.stats.avg_stability_ema;

        Some(IntegrationTestResult {
            strategy_id,
            passed,
            baseline_metric: mean_baseline,
            strategy_metric: mean_strategy,
            improvement: mean_improvement,
            trial_results: improvements,
            mean_improvement,
            consistency,
        })
    }

    /// Retire a strategy that is no longer needed
    pub fn retire(&mut self, strategy_id: u64) -> bool {
        if let Some(s) = self.strategies.get_mut(&strategy_id) {
            s.status = StrategyStatus::Retired;
            self.stats.total_retired += 1;
            self.stats.active_strategies = self
                .strategies
                .values()
                .filter(|s| s.status == StrategyStatus::Active)
                .count() as u32;
            true
        } else {
            false
        }
    }

    /// Get a strategy by ID
    #[inline(always)]
    pub fn get_strategy(&self, strategy_id: u64) -> Option<&SynthesizedStrategy> {
        self.strategies.get(&strategy_id)
    }

    /// List all active strategies
    #[inline]
    pub fn active_strategies(&self) -> Vec<&SynthesizedStrategy> {
        self.strategies
            .values()
            .filter(|s| s.status == StrategyStatus::Active)
            .collect()
    }

    /// Get aggregate stats
    #[inline(always)]
    pub fn stats(&self) -> SynthesisStats {
        self.stats
    }
}
