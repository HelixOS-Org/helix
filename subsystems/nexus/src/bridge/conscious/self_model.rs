// SPDX-License-Identifier: GPL-2.0
//! # Bridge Self-Model
//!
//! The bridge's complete model of itself. Tracks capabilities such as prediction
//! accuracy, batch efficiency, and cache hit rate alongside known limitations
//! like hardware failure blind spots and novel attack detection delays. All
//! metrics are smoothed with exponential moving averages and bounded by
//! confidence intervals derived from observed variance.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.15;
const CONFIDENCE_Z: f32 = 1.96; // 95% CI
const MAX_CAPABILITY_HISTORY: usize = 256;
const MAX_LIMITATIONS: usize = 64;
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

// ============================================================================
// CAPABILITY TRACKING
// ============================================================================

/// A single tracked capability of the bridge
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct Capability {
    /// Human-readable name
    pub name: String,
    /// Hashed identifier for fast lookup
    pub id: u64,
    /// Current EMA-smoothed score (0.0 – 1.0)
    pub score: f32,
    /// Variance accumulator for confidence intervals
    pub variance_accum: f32,
    /// Number of observations
    pub observations: u64,
    /// Raw sample history (ring buffer)
    history: Vec<f32>,
    /// Write index into history ring
    write_idx: usize,
    /// Last raw sample
    pub last_raw: f32,
    /// Peak score ever observed
    pub peak_score: f32,
    /// Tick of last update
    pub last_update_tick: u64,
}

impl Capability {
    pub fn new(name: String) -> Self {
        let id = fnv1a_hash(name.as_bytes());
        Self {
            name,
            id,
            score: 0.5,
            variance_accum: 0.0,
            observations: 0,
            history: Vec::new(),
            write_idx: 0,
            last_raw: 0.5,
            peak_score: 0.5,
            last_update_tick: 0,
        }
    }

    /// Push a new observation, update EMA and variance
    #[inline]
    pub fn observe(&mut self, raw: f32, tick: u64) {
        let clamped = raw.max(0.0).min(1.0);
        self.last_raw = clamped;
        self.observations += 1;
        self.last_update_tick = tick;

        // EMA update
        self.score = EMA_ALPHA * clamped + (1.0 - EMA_ALPHA) * self.score;
        if self.score > self.peak_score {
            self.peak_score = self.score;
        }

        // Online variance (Welford-like with EMA weighting)
        let diff = clamped - self.score;
        self.variance_accum = EMA_ALPHA * diff * diff + (1.0 - EMA_ALPHA) * self.variance_accum;

        // Ring buffer history
        if self.history.len() < MAX_CAPABILITY_HISTORY {
            self.history.push(clamped);
        } else {
            self.history[self.write_idx] = clamped;
        }
        self.write_idx = (self.write_idx + 1) % MAX_CAPABILITY_HISTORY;
    }

    /// 95% confidence interval half-width
    #[inline]
    pub fn confidence_half_width(&self) -> f32 {
        if self.observations < 2 {
            return 0.5;
        }
        let std_dev = libm::sqrtf(self.variance_accum);
        let n_sqrt = libm::sqrtf(self.observations.min(MAX_CAPABILITY_HISTORY as u64) as f32);
        CONFIDENCE_Z * std_dev / n_sqrt
    }

    /// Confidence interval (low, high) around the EMA score
    #[inline(always)]
    pub fn confidence_interval(&self) -> (f32, f32) {
        let hw = self.confidence_half_width();
        ((self.score - hw).max(0.0), (self.score + hw).min(1.0))
    }

    /// Improvement rate: slope of recent trend (positive = improving)
    pub fn improvement_rate(&self) -> f32 {
        let len = self.history.len();
        if len < 4 {
            return 0.0;
        }
        // Compare first half average to second half average
        let mid = len / 2;
        let first_sum: f32 = self.history[..mid].iter().sum();
        let second_sum: f32 = self.history[mid..].iter().sum();
        let first_avg = first_sum / mid as f32;
        let second_avg = second_sum / (len - mid) as f32;
        second_avg - first_avg
    }
}

// ============================================================================
// LIMITATION TRACKING
// ============================================================================

/// Severity of a known limitation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LimitationSeverity {
    Minor       = 1,
    Moderate    = 2,
    Significant = 3,
    Critical    = 4,
}

/// A known limitation of the bridge
#[derive(Debug, Clone)]
pub struct Limitation {
    pub name: String,
    pub id: u64,
    pub severity: LimitationSeverity,
    /// How often this limitation manifests (0.0 – 1.0)
    pub frequency: f32,
    /// Impact when it manifests (0.0 – 1.0)
    pub impact: f32,
    /// Is mitigation in place?
    pub mitigated: bool,
    /// Mitigation effectiveness (0.0 – 1.0)
    pub mitigation_effectiveness: f32,
    /// Tick when first detected
    pub detected_tick: u64,
    /// Number of times observed
    pub occurrences: u64,
}

// ============================================================================
// SELF-MODEL STATS
// ============================================================================

/// Aggregate statistics about the self-model
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct SelfModelStats {
    pub total_capabilities: usize,
    pub total_limitations: usize,
    pub avg_capability_score: f32,
    pub avg_confidence_width: f32,
    pub overall_improvement_rate: f32,
    pub unmitigated_limitations: usize,
    pub self_evaluation_score: f32,
    pub total_observations: u64,
}

// ============================================================================
// BRIDGE SELF-MODEL
// ============================================================================

/// The bridge's complete model of itself — capabilities, limitations,
/// performance metrics with EMA smoothing and confidence intervals.
#[derive(Debug)]
#[repr(align(64))]
pub struct BridgeSelfModel {
    /// All tracked capabilities keyed by FNV-1a hash of name
    capabilities: BTreeMap<u64, Capability>,
    /// Known limitations keyed by FNV-1a hash of name
    limitations: BTreeMap<u64, Limitation>,
    /// Monotonic tick counter
    tick: u64,
    /// Cached aggregate stats
    cached_stats: SelfModelStats,
    /// Tick at which stats were last computed
    stats_tick: u64,
}

impl BridgeSelfModel {
    pub fn new() -> Self {
        Self {
            capabilities: BTreeMap::new(),
            limitations: BTreeMap::new(),
            tick: 0,
            cached_stats: SelfModelStats::default(),
            stats_tick: 0,
        }
    }

    /// Register or update a capability with a new observation
    #[inline]
    pub fn update_capability(&mut self, name: &str, raw_score: f32) {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes());
        let tick = self.tick;
        let cap = self
            .capabilities
            .entry(id)
            .or_insert_with(|| Capability::new(String::from(name)));
        cap.observe(raw_score, tick);
    }

    /// Register or update a limitation assessment
    #[inline]
    pub fn assess_limitation(
        &mut self,
        name: &str,
        severity: LimitationSeverity,
        frequency: f32,
        impact: f32,
    ) {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes());
        let tick = self.tick;
        let lim = self.limitations.entry(id).or_insert_with(|| Limitation {
            name: String::from(name),
            id,
            severity,
            frequency: 0.0,
            impact: 0.0,
            mitigated: false,
            mitigation_effectiveness: 0.0,
            detected_tick: tick,
            occurrences: 0,
        });
        // EMA smooth frequency and impact
        lim.frequency = EMA_ALPHA * frequency.max(0.0).min(1.0) + (1.0 - EMA_ALPHA) * lim.frequency;
        lim.impact = EMA_ALPHA * impact.max(0.0).min(1.0) + (1.0 - EMA_ALPHA) * lim.impact;
        lim.severity = severity;
        lim.occurrences += 1;
    }

    /// Mark a limitation as mitigated with an effectiveness score
    #[inline]
    pub fn mitigate_limitation(&mut self, name: &str, effectiveness: f32) {
        let id = fnv1a_hash(name.as_bytes());
        if let Some(lim) = self.limitations.get_mut(&id) {
            lim.mitigated = true;
            lim.mitigation_effectiveness = effectiveness.max(0.0).min(1.0);
        }
    }

    /// Compute the bridge's overall self-evaluation score (0.0 – 1.0)
    pub fn self_evaluate(&mut self) -> f32 {
        if self.capabilities.is_empty() {
            return 0.0;
        }

        // Weighted sum: capabilities contribute positively, limitations subtract
        let cap_sum: f32 = self.capabilities.values().map(|c| c.score).sum();
        let cap_count = self.capabilities.len() as f32;
        let avg_cap = cap_sum / cap_count;

        let lim_penalty: f32 = self
            .limitations
            .values()
            .map(|l| {
                let raw_penalty = l.frequency * l.impact * (l.severity as u8 as f32 / 4.0);
                if l.mitigated {
                    raw_penalty * (1.0 - l.mitigation_effectiveness)
                } else {
                    raw_penalty
                }
            })
            .sum();

        let lim_count = self.limitations.len().max(1) as f32;
        let avg_penalty = lim_penalty / lim_count;

        let score = (avg_cap - avg_penalty * 0.5).max(0.0).min(1.0);
        self.cached_stats.self_evaluation_score = score;
        score
    }

    /// Get the EMA-smoothed score for a named capability
    #[inline(always)]
    pub fn capability_score(&self, name: &str) -> Option<f32> {
        let id = fnv1a_hash(name.as_bytes());
        self.capabilities.get(&id).map(|c| c.score)
    }

    /// Get improvement rate across all capabilities
    #[inline]
    pub fn improvement_rate(&self) -> f32 {
        if self.capabilities.is_empty() {
            return 0.0;
        }
        let sum: f32 = self
            .capabilities
            .values()
            .map(|c| c.improvement_rate())
            .sum();
        sum / self.capabilities.len() as f32
    }

    /// Compute and return aggregate statistics
    pub fn stats(&mut self) -> SelfModelStats {
        if self.tick == self.stats_tick && self.cached_stats.total_observations > 0 {
            return self.cached_stats;
        }
        let cap_count = self.capabilities.len();
        let lim_count = self.limitations.len();
        let avg_score = if cap_count > 0 {
            self.capabilities.values().map(|c| c.score).sum::<f32>() / cap_count as f32
        } else {
            0.0
        };
        let avg_ci = if cap_count > 0 {
            self.capabilities
                .values()
                .map(|c| c.confidence_half_width() * 2.0)
                .sum::<f32>()
                / cap_count as f32
        } else {
            1.0
        };
        let total_obs: u64 = self.capabilities.values().map(|c| c.observations).sum();
        let unmitigated = self.limitations.values().filter(|l| !l.mitigated).count();

        self.cached_stats = SelfModelStats {
            total_capabilities: cap_count,
            total_limitations: lim_count,
            avg_capability_score: avg_score,
            avg_confidence_width: avg_ci,
            overall_improvement_rate: self.improvement_rate(),
            unmitigated_limitations: unmitigated,
            self_evaluation_score: self.self_evaluate(),
            total_observations: total_obs,
        };
        self.stats_tick = self.tick;
        self.cached_stats
    }

    /// List all capabilities with their confidence intervals
    #[inline]
    pub fn capability_report(&self) -> Vec<(String, f32, f32, f32)> {
        self.capabilities
            .values()
            .map(|c| {
                let (lo, hi) = c.confidence_interval();
                (c.name.clone(), c.score, lo, hi)
            })
            .collect()
    }

    /// List all unmitigated limitations sorted by severity (descending)
    #[inline]
    pub fn critical_limitations(&self) -> Vec<&Limitation> {
        let mut lims: Vec<&Limitation> =
            self.limitations.values().filter(|l| !l.mitigated).collect();
        lims.sort_by(|a, b| b.severity.cmp(&a.severity));
        lims
    }
}
