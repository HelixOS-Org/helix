//! # Cooperative Reputation System
//!
//! Long-term reputation tracking for cooperative behavior:
//! - Multi-dimensional reputation scoring
//! - Reputation decay over time
//! - Reputation categories
//! - Behavioral pattern analysis
//! - Reputation-based access control

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// REPUTATION DIMENSIONS
// ============================================================================

/// Reputation dimension
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReputationDimension {
    /// Resource sharing reliability
    SharingReliability,
    /// Agreement fulfillment
    AgreementFulfillment,
    /// Priority fairness
    PriorityFairness,
    /// Response timeliness
    Timeliness,
    /// Resource efficiency
    Efficiency,
    /// Cooperation willingness
    CoopWillingness,
    /// System stability contribution
    SystemStability,
}

/// Reputation level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReputationLevel {
    /// Untrusted
    Untrusted,
    /// New (no history)
    New,
    /// Basic
    Basic,
    /// Trusted
    Trusted,
    /// Highly trusted
    HighlyTrusted,
    /// Exemplary
    Exemplary,
}

impl ReputationLevel {
    /// From score
    pub fn from_score(score: f64) -> Self {
        if score < 0.2 {
            Self::Untrusted
        } else if score < 0.4 {
            Self::New
        } else if score < 0.6 {
            Self::Basic
        } else if score < 0.75 {
            Self::Trusted
        } else if score < 0.9 {
            Self::HighlyTrusted
        } else {
            Self::Exemplary
        }
    }

    /// Privileges enabled at this level
    pub fn privileges(&self) -> u32 {
        match self {
            Self::Untrusted => 0,
            Self::New => 1,
            Self::Basic => 3,
            Self::Trusted => 7,
            Self::HighlyTrusted => 15,
            Self::Exemplary => 31,
        }
    }
}

// ============================================================================
// REPUTATION SCORE
// ============================================================================

/// Per-dimension score with history
#[derive(Debug, Clone)]
pub struct DimensionScore {
    /// Dimension
    pub dimension: ReputationDimension,
    /// Current score (0.0-1.0)
    pub score: f64,
    /// Number of observations
    pub observations: u64,
    /// Positive events
    pub positive: u64,
    /// Negative events
    pub negative: u64,
    /// History (recent scores)
    history: Vec<f64>,
    /// Max history
    max_history: usize,
    /// Decay rate per epoch
    decay_rate: f64,
}

impl DimensionScore {
    pub fn new(dimension: ReputationDimension) -> Self {
        Self {
            dimension,
            score: 0.5, // neutral start
            observations: 0,
            positive: 0,
            negative: 0,
            history: Vec::new(),
            max_history: 64,
            decay_rate: 0.01,
        }
    }

    /// Record observation
    pub fn observe(&mut self, value: f64) {
        self.observations += 1;
        if value >= 0.5 {
            self.positive += 1;
        } else {
            self.negative += 1;
        }

        // Exponential moving average
        let alpha = 2.0 / (self.observations.min(50) as f64 + 1.0);
        self.score = self.score * (1.0 - alpha) + value * alpha;
        self.score = self.score.max(0.0).min(1.0);

        self.history.push(self.score);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    /// Apply time decay
    pub fn decay(&mut self) {
        self.score *= 1.0 - self.decay_rate;
        // Don't decay below baseline
        if self.score < 0.3 && self.positive > self.negative {
            self.score = 0.3;
        }
    }

    /// Trend (positive or negative)
    pub fn trend(&self) -> f64 {
        if self.history.len() < 2 {
            return 0.0;
        }
        let recent = self.history.len().min(8);
        let start_idx = self.history.len() - recent;
        let first_half = &self.history[start_idx..start_idx + recent / 2];
        let second_half = &self.history[start_idx + recent / 2..];

        let avg1: f64 = first_half.iter().sum::<f64>() / first_half.len() as f64;
        let avg2: f64 = second_half.iter().sum::<f64>() / second_half.len() as f64;
        avg2 - avg1
    }
}

// ============================================================================
// PROCESS REPUTATION
// ============================================================================

/// Per-process reputation profile
#[derive(Debug, Clone)]
pub struct ProcessReputation {
    /// Process ID
    pub pid: u64,
    /// Per-dimension scores
    pub dimensions: BTreeMap<u8, DimensionScore>,
    /// Overall score
    pub overall_score: f64,
    /// Level
    pub level: ReputationLevel,
    /// Created at
    pub created_at: u64,
    /// Last updated
    pub last_updated: u64,
    /// Total events
    pub total_events: u64,
}

impl ProcessReputation {
    pub fn new(pid: u64, now: u64) -> Self {
        let mut dims = BTreeMap::new();
        dims.insert(
            ReputationDimension::SharingReliability as u8,
            DimensionScore::new(ReputationDimension::SharingReliability),
        );
        dims.insert(
            ReputationDimension::AgreementFulfillment as u8,
            DimensionScore::new(ReputationDimension::AgreementFulfillment),
        );
        dims.insert(
            ReputationDimension::PriorityFairness as u8,
            DimensionScore::new(ReputationDimension::PriorityFairness),
        );
        dims.insert(
            ReputationDimension::Timeliness as u8,
            DimensionScore::new(ReputationDimension::Timeliness),
        );
        dims.insert(
            ReputationDimension::Efficiency as u8,
            DimensionScore::new(ReputationDimension::Efficiency),
        );
        dims.insert(
            ReputationDimension::CoopWillingness as u8,
            DimensionScore::new(ReputationDimension::CoopWillingness),
        );
        dims.insert(
            ReputationDimension::SystemStability as u8,
            DimensionScore::new(ReputationDimension::SystemStability),
        );

        Self {
            pid,
            dimensions: dims,
            overall_score: 0.5,
            level: ReputationLevel::New,
            created_at: now,
            last_updated: now,
            total_events: 0,
        }
    }

    /// Record event
    pub fn record_event(&mut self, dimension: ReputationDimension, value: f64, now: u64) {
        if let Some(dim) = self.dimensions.get_mut(&(dimension as u8)) {
            dim.observe(value);
        }
        self.total_events += 1;
        self.last_updated = now;
        self.recalculate();
    }

    /// Recalculate overall score
    fn recalculate(&mut self) {
        if self.dimensions.is_empty() {
            return;
        }
        let weights = [0.2, 0.2, 0.1, 0.1, 0.15, 0.15, 0.1];
        let mut total = 0.0;
        let mut weight_sum = 0.0;

        for (i, dim) in self.dimensions.values().enumerate() {
            let w = if i < weights.len() { weights[i] } else { 0.1 };
            total += dim.score * w;
            weight_sum += w;
        }

        self.overall_score = if weight_sum > 0.0 {
            total / weight_sum
        } else {
            0.5
        };
        self.level = ReputationLevel::from_score(self.overall_score);
    }

    /// Apply decay
    pub fn apply_decay(&mut self) {
        for dim in self.dimensions.values_mut() {
            dim.decay();
        }
        self.recalculate();
    }

    /// Get dimension score
    pub fn dimension_score(&self, dimension: ReputationDimension) -> f64 {
        self.dimensions
            .get(&(dimension as u8))
            .map(|d| d.score)
            .unwrap_or(0.5)
    }
}

// ============================================================================
// REPUTATION MANAGER
// ============================================================================

/// Reputation manager stats
#[derive(Debug, Clone, Default)]
pub struct CoopReputationStats {
    /// Tracked processes
    pub process_count: usize,
    /// Average reputation
    pub avg_reputation: f64,
    /// Untrusted count
    pub untrusted_count: usize,
    /// Exemplary count
    pub exemplary_count: usize,
    /// Total events
    pub total_events: u64,
}

/// Cooperative reputation manager
pub struct CoopReputationManager {
    /// Per-process reputation
    reputations: BTreeMap<u64, ProcessReputation>,
    /// Stats
    stats: CoopReputationStats,
}

impl CoopReputationManager {
    pub fn new() -> Self {
        Self {
            reputations: BTreeMap::new(),
            stats: CoopReputationStats::default(),
        }
    }

    /// Register process
    pub fn register(&mut self, pid: u64, now: u64) {
        self.reputations.insert(pid, ProcessReputation::new(pid, now));
        self.update_stats();
    }

    /// Record event
    pub fn record_event(
        &mut self,
        pid: u64,
        dimension: ReputationDimension,
        value: f64,
        now: u64,
    ) {
        if let Some(rep) = self.reputations.get_mut(&pid) {
            rep.record_event(dimension, value, now);
            self.stats.total_events += 1;
        }
        self.update_stats();
    }

    /// Apply decay to all
    pub fn apply_decay(&mut self) {
        for rep in self.reputations.values_mut() {
            rep.apply_decay();
        }
        self.update_stats();
    }

    /// Get reputation
    pub fn reputation(&self, pid: u64) -> Option<&ProcessReputation> {
        self.reputations.get(&pid)
    }

    /// Get level
    pub fn level(&self, pid: u64) -> ReputationLevel {
        self.reputations
            .get(&pid)
            .map(|r| r.level)
            .unwrap_or(ReputationLevel::New)
    }

    /// Check if process meets minimum level
    pub fn meets_level(&self, pid: u64, min_level: ReputationLevel) -> bool {
        self.level(pid) >= min_level
    }

    /// Ranking by overall score
    pub fn ranking(&self) -> Vec<(u64, f64)> {
        let mut ranked: Vec<_> = self
            .reputations
            .iter()
            .map(|(&pid, rep)| (pid, rep.overall_score))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        ranked
    }

    fn update_stats(&mut self) {
        self.stats.process_count = self.reputations.len();
        if self.reputations.is_empty() {
            self.stats.avg_reputation = 0.0;
            return;
        }
        self.stats.avg_reputation = self
            .reputations
            .values()
            .map(|r| r.overall_score)
            .sum::<f64>()
            / self.reputations.len() as f64;
        self.stats.untrusted_count = self
            .reputations
            .values()
            .filter(|r| r.level == ReputationLevel::Untrusted)
            .count();
        self.stats.exemplary_count = self
            .reputations
            .values()
            .filter(|r| r.level == ReputationLevel::Exemplary)
            .count();
    }

    /// Unregister
    pub fn unregister(&mut self, pid: u64) {
        self.reputations.remove(&pid);
        self.update_stats();
    }

    /// Stats
    pub fn stats(&self) -> &CoopReputationStats {
        &self.stats
    }
}
