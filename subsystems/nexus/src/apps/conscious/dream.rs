// SPDX-License-Identifier: GPL-2.0
//! # Apps Dream Engine
//!
//! Offline consolidation for application understanding. When the system is idle
//! (low CPU utilization, few active tasks), the dream engine activates to
//! replay application behavior histories, discover missed correlations between
//! applications, and build new classification dimensions that the online
//! engine may have overlooked.
//!
//! Inspired by memory consolidation during sleep in biological systems, this
//! engine performs three key operations:
//!
//! 1. **Replay** — Re-process stored behavioral traces to reinforce stable
//!    patterns and weaken noisy ones.
//! 2. **Correlation discovery** — Identify cross-app relationships (e.g.,
//!    app A's memory spike predicts app B's I/O burst 3 ticks later).
//! 3. **Classification consolidation** — Merge redundant categories, split
//!    overly broad ones, and construct new feature dimensions.
//!
//! Dream cycles are bounded by a configurable time budget and produce
//! `DreamInsight` records that the online engine can integrate.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.10;
const MAX_APPS: usize = 512;
const MAX_HISTORY_PER_APP: usize = 256;
const MAX_INSIGHTS: usize = 128;
const MAX_CORRELATIONS: usize = 256;
const CORRELATION_THRESHOLD: f32 = 0.6;
const CONSOLIDATION_MERGE_THRESHOLD: f32 = 0.85;
const DREAM_CYCLE_BUDGET: u32 = 500;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

// ============================================================================
// FNV-1a HASHING
// ============================================================================

fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Xorshift64 PRNG for stochastic replay selection
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ============================================================================
// DREAM DATA TYPES
// ============================================================================

/// A single behavioral trace sample for an app
#[derive(Debug, Clone)]
pub struct BehaviorSample {
    pub tick: u64,
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub io_rate: f32,
    pub net_rate: f32,
    pub classification_hash: u64,
}

/// Stored behavioral history for an app
#[derive(Debug, Clone)]
pub struct AppBehaviorHistory {
    pub app_id: u64,
    pub app_name: String,
    pub samples: Vec<BehaviorSample>,
    pub write_idx: usize,
    pub replay_count: u64,
    pub pattern_strength: f32,
    pub last_replay_tick: u64,
}

impl AppBehaviorHistory {
    fn new(app_id: u64, app_name: String) -> Self {
        Self {
            app_id,
            app_name,
            samples: Vec::new(),
            write_idx: 0,
            replay_count: 0,
            pattern_strength: 0.5,
            last_replay_tick: 0,
        }
    }

    fn add_sample(&mut self, sample: BehaviorSample) {
        if self.samples.len() < MAX_HISTORY_PER_APP {
            self.samples.push(sample);
        } else {
            self.samples[self.write_idx] = sample;
        }
        self.write_idx = (self.write_idx + 1) % MAX_HISTORY_PER_APP;
    }

    fn mean_cpu(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        self.samples.iter().map(|s| s.cpu_usage).sum::<f32>() / self.samples.len() as f32
    }

    fn mean_memory(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        self.samples.iter().map(|s| s.memory_usage).sum::<f32>() / self.samples.len() as f32
    }

    fn mean_io(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        self.samples.iter().map(|s| s.io_rate).sum::<f32>() / self.samples.len() as f32
    }

    fn mean_net(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        self.samples.iter().map(|s| s.net_rate).sum::<f32>() / self.samples.len() as f32
    }
}

/// Discovered correlation between two applications
#[derive(Debug, Clone)]
pub struct AppCorrelation {
    pub app_a: u64,
    pub app_b: u64,
    pub correlation_strength: f32,
    pub dimension: String,
    pub lag_ticks: i32,
    pub discovery_tick: u64,
    pub confirmed: bool,
}

/// An insight generated during a dream cycle
#[derive(Debug, Clone)]
pub struct DreamInsight {
    pub discovery: String,
    pub apps_involved: Vec<u64>,
    pub impact: f32,
    pub cycle_id: u64,
    pub discovery_tick: u64,
    pub actionable: bool,
}

/// Classification dimension (discovered or reinforced during dreaming)
#[derive(Debug, Clone)]
pub struct ClassificationDimension {
    pub name: String,
    pub id: u64,
    pub apps_in_category: Vec<u64>,
    pub coherence: f32,
    pub last_consolidated_tick: u64,
    pub merge_candidate: bool,
}

// ============================================================================
// STATS
// ============================================================================

/// Dream engine aggregate statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DreamStats {
    pub total_cycles: u64,
    pub total_insights: usize,
    pub total_correlations: usize,
    pub total_replays: u64,
    pub mean_pattern_strength: f32,
    pub actionable_insight_count: usize,
    pub apps_tracked: usize,
    pub dimension_count: usize,
}

// ============================================================================
// APPS DREAM ENGINE
// ============================================================================

/// Offline consolidation engine for app behavior understanding
#[derive(Debug)]
pub struct AppsDreamEngine {
    histories: BTreeMap<u64, AppBehaviorHistory>,
    correlations: Vec<AppCorrelation>,
    insights: Vec<DreamInsight>,
    insight_write_idx: usize,
    dimensions: BTreeMap<u64, ClassificationDimension>,
    total_cycles: u64,
    total_replays: u64,
    tick: u64,
    rng_state: u64,
}

impl AppsDreamEngine {
    pub fn new(seed: u64) -> Self {
        Self {
            histories: BTreeMap::new(),
            correlations: Vec::new(),
            insights: Vec::new(),
            insight_write_idx: 0,
            dimensions: BTreeMap::new(),
            total_cycles: 0,
            total_replays: 0,
            tick: 0,
            rng_state: if seed == 0 { 0xD2EA_CAFE_1234_5678 } else { seed },
        }
    }

    /// Record a behavioral sample for an app (feed during online operation)
    pub fn record_behavior(
        &mut self,
        app_id: u64,
        app_name: &str,
        cpu: f32,
        memory: f32,
        io: f32,
        net: f32,
        classification_hash: u64,
        tick: u64,
    ) {
        self.tick = tick;
        let history = self
            .histories
            .entry(app_id)
            .or_insert_with(|| AppBehaviorHistory::new(app_id, String::from(app_name)));

        history.add_sample(BehaviorSample {
            tick,
            cpu_usage: cpu,
            memory_usage: memory,
            io_rate: io,
            net_rate: net,
            classification_hash,
        });

        // Evict if over capacity
        if self.histories.len() > MAX_APPS {
            self.evict_weakest(app_id);
        }
    }

    /// Execute a full dream cycle — replay, correlate, consolidate
    pub fn dream_cycle(&mut self) -> Vec<DreamInsight> {
        self.total_cycles += 1;
        let cycle_id = self.total_cycles;
        let mut budget_remaining = DREAM_CYCLE_BUDGET;
        let mut cycle_insights = Vec::new();

        // Phase 1: Replay app histories
        let app_ids: Vec<u64> = self.histories.keys().copied().collect();
        for app_id in &app_ids {
            if budget_remaining < 5 {
                break;
            }
            if let Some(insight) = self.replay_single(cycle_id, *app_id) {
                cycle_insights.push(insight);
            }
            budget_remaining = budget_remaining.saturating_sub(3);
        }

        // Phase 2: Discover correlations
        let pair_count = app_ids.len().min(30);
        for i in 0..pair_count {
            if budget_remaining < 5 {
                break;
            }
            for j in (i + 1)..pair_count.min(app_ids.len()) {
                if budget_remaining < 2 {
                    break;
                }
                if let Some(insight) =
                    self.discover_single_correlation(cycle_id, app_ids[i], app_ids[j])
                {
                    cycle_insights.push(insight);
                }
                budget_remaining = budget_remaining.saturating_sub(2);
            }
        }

        // Phase 3: Consolidate classifications
        if budget_remaining > 10 {
            if let Some(insight) = self.consolidate_once(cycle_id) {
                cycle_insights.push(insight);
            }
        }

        // Store insights
        for insight in &cycle_insights {
            self.store_insight(insight.clone());
        }

        cycle_insights
    }

    /// Replay behavioral history for a specific app
    #[inline]
    pub fn replay_app_history(&mut self, app_id: u64) -> Option<f32> {
        let history = self.histories.get_mut(&app_id)?;
        history.replay_count += 1;
        history.last_replay_tick = self.tick;
        self.total_replays += 1;

        // Strengthen stable patterns, weaken noisy ones
        if history.samples.len() < 4 {
            return Some(history.pattern_strength);
        }

        let mean_cpu = history.mean_cpu();
        let mean_mem = history.mean_memory();

        let mut variance_sum = 0.0_f32;
        for sample in &history.samples {
            let d_cpu = sample.cpu_usage - mean_cpu;
            let d_mem = sample.memory_usage - mean_mem;
            variance_sum += d_cpu * d_cpu + d_mem * d_mem;
        }
        let variance = variance_sum / history.samples.len() as f32;

        // Low variance → strong pattern, high variance → weak
        let stability = 1.0 / (1.0 + variance * 4.0);
        history.pattern_strength =
            EMA_ALPHA * stability + (1.0 - EMA_ALPHA) * history.pattern_strength;

        Some(history.pattern_strength)
    }

    /// Discover correlation between two specific apps
    pub fn discover_correlation(&mut self, app_a: u64, app_b: u64) -> Option<AppCorrelation> {
        let hist_a = self.histories.get(&app_a)?;
        let hist_b = self.histories.get(&app_b)?;

        if hist_a.samples.len() < 8 || hist_b.samples.len() < 8 {
            return None;
        }

        // Simple Pearson-like correlation on CPU usage
        let n = hist_a.samples.len().min(hist_b.samples.len());
        let mean_a = hist_a.mean_cpu();
        let mean_b = hist_b.mean_cpu();

        let mut cov = 0.0_f32;
        let mut var_a = 0.0_f32;
        let mut var_b = 0.0_f32;

        for i in 0..n {
            let da = hist_a.samples[i].cpu_usage - mean_a;
            let db = hist_b.samples[i].cpu_usage - mean_b;
            cov += da * db;
            var_a += da * da;
            var_b += db * db;
        }

        let denom = (var_a * var_b).sqrt();
        if denom < 0.001 {
            return None;
        }

        let r = cov / denom;
        let abs_r = if r < 0.0 { -r } else { r };

        if abs_r < CORRELATION_THRESHOLD {
            return None;
        }

        let corr = AppCorrelation {
            app_a,
            app_b,
            correlation_strength: abs_r,
            dimension: String::from("cpu_usage"),
            lag_ticks: 0,
            discovery_tick: self.tick,
            confirmed: false,
        };

        if self.correlations.len() < MAX_CORRELATIONS {
            self.correlations.push(corr.clone());
        }

        Some(corr)
    }

    /// Consolidate existing classification dimensions
    pub fn consolidate_classifications(&mut self) -> usize {
        let mut merged = 0usize;
        let dim_ids: Vec<u64> = self.dimensions.keys().copied().collect();

        for i in 0..dim_ids.len() {
            for j in (i + 1)..dim_ids.len() {
                let id_a = dim_ids[i];
                let id_b = dim_ids[j];
                let overlap = self.dimension_overlap(id_a, id_b);
                if overlap > CONSOLIDATION_MERGE_THRESHOLD {
                    // Merge b into a
                    if let Some(dim_b) = self.dimensions.remove(&id_b) {
                        if let Some(dim_a) = self.dimensions.get_mut(&id_a) {
                            for app in dim_b.apps_in_category {
                                if !dim_a.apps_in_category.contains(&app) {
                                    dim_a.apps_in_category.push(app);
                                }
                            }
                            dim_a.coherence =
                                (dim_a.coherence + dim_b.coherence) / 2.0;
                            dim_a.last_consolidated_tick = self.tick;
                            merged += 1;
                        }
                    }
                }
            }
        }
        merged
    }

    /// Return all insights collected so far
    #[inline(always)]
    pub fn dream_insights(&self) -> &[DreamInsight] {
        &self.insights
    }

    /// Perform idle learning — lightweight version of dream cycle
    pub fn idle_learning(&mut self) -> Option<DreamInsight> {
        // Pick a random app to replay
        let app_ids: Vec<u64> = self.histories.keys().copied().collect();
        if app_ids.is_empty() {
            return None;
        }

        let idx = (xorshift64(&mut self.rng_state) as usize) % app_ids.len();
        let app_id = app_ids[idx];
        let strength = self.replay_app_history(app_id)?;

        if strength > 0.8 {
            let insight = DreamInsight {
                discovery: String::from("pattern_reinforcement_during_idle"),
                apps_involved: alloc::vec![app_id],
                impact: strength * 0.3,
                cycle_id: 0,
                discovery_tick: self.tick,
                actionable: false,
            };
            self.store_insight(insight.clone());
            Some(insight)
        } else {
            None
        }
    }

    /// Add a classification dimension
    pub fn add_dimension(&mut self, name: &str, apps: Vec<u64>) {
        let id = fnv1a_hash(name.as_bytes());
        let dim = ClassificationDimension {
            name: String::from(name),
            id,
            apps_in_category: apps,
            coherence: 0.5,
            last_consolidated_tick: self.tick,
            merge_candidate: false,
        };
        self.dimensions.insert(id, dim);
    }

    /// Full stats
    pub fn stats(&self) -> DreamStats {
        let mean_strength = if self.histories.is_empty() {
            0.0
        } else {
            self.histories
                .values()
                .map(|h| h.pattern_strength)
                .sum::<f32>()
                / self.histories.len() as f32
        };

        let actionable = self.insights.iter().filter(|i| i.actionable).count();

        DreamStats {
            total_cycles: self.total_cycles,
            total_insights: self.insights.len(),
            total_correlations: self.correlations.len(),
            total_replays: self.total_replays,
            mean_pattern_strength: mean_strength,
            actionable_insight_count: actionable,
            apps_tracked: self.histories.len(),
            dimension_count: self.dimensions.len(),
        }
    }

    // ========================================================================
    // INTERNAL
    // ========================================================================

    fn replay_single(&mut self, cycle_id: u64, app_id: u64) -> Option<DreamInsight> {
        let strength = self.replay_app_history(app_id)?;
        if strength > 0.75 {
            Some(DreamInsight {
                discovery: String::from("pattern_confirmed_via_replay"),
                apps_involved: alloc::vec![app_id],
                impact: strength * 0.2,
                cycle_id,
                discovery_tick: self.tick,
                actionable: true,
            })
        } else {
            None
        }
    }

    fn discover_single_correlation(
        &mut self,
        cycle_id: u64,
        app_a: u64,
        app_b: u64,
    ) -> Option<DreamInsight> {
        let corr = self.discover_correlation(app_a, app_b)?;
        Some(DreamInsight {
            discovery: String::from("cross_app_correlation_discovered"),
            apps_involved: alloc::vec![app_a, app_b],
            impact: corr.correlation_strength * 0.5,
            cycle_id,
            discovery_tick: self.tick,
            actionable: true,
        })
    }

    fn consolidate_once(&mut self, cycle_id: u64) -> Option<DreamInsight> {
        let merged = self.consolidate_classifications();
        if merged > 0 {
            Some(DreamInsight {
                discovery: String::from("classification_dimensions_consolidated"),
                apps_involved: Vec::new(),
                impact: merged as f32 * 0.1,
                cycle_id,
                discovery_tick: self.tick,
                actionable: true,
            })
        } else {
            None
        }
    }

    fn dimension_overlap(&self, id_a: u64, id_b: u64) -> f32 {
        let dim_a = match self.dimensions.get(&id_a) {
            Some(d) => d,
            None => return 0.0,
        };
        let dim_b = match self.dimensions.get(&id_b) {
            Some(d) => d,
            None => return 0.0,
        };

        if dim_a.apps_in_category.is_empty() || dim_b.apps_in_category.is_empty() {
            return 0.0;
        }

        let mut shared = 0usize;
        for app in &dim_a.apps_in_category {
            if dim_b.apps_in_category.contains(app) {
                shared += 1;
            }
        }

        let union = dim_a.apps_in_category.len() + dim_b.apps_in_category.len() - shared;
        if union == 0 {
            return 0.0;
        }
        shared as f32 / union as f32
    }

    fn store_insight(&mut self, insight: DreamInsight) {
        if self.insights.len() < MAX_INSIGHTS {
            self.insights.push(insight);
        } else {
            self.insights[self.insight_write_idx] = insight;
        }
        self.insight_write_idx = (self.insight_write_idx + 1) % MAX_INSIGHTS;
    }

    fn evict_weakest(&mut self, keep_id: u64) {
        let mut weakest_id = 0u64;
        let mut weakest_strength = f32::MAX;
        for (id, hist) in &self.histories {
            if *id != keep_id && hist.pattern_strength < weakest_strength {
                weakest_strength = hist.pattern_strength;
                weakest_id = *id;
            }
        }
        if weakest_id != 0 {
            self.histories.remove(&weakest_id);
        }
    }
}
