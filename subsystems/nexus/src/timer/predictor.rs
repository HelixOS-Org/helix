//! Deadline Predictor
//!
//! Predicts timer deadlines and patterns.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::core::NexusTimestamp;
use crate::math;

use super::TimerId;

/// Timer pattern
#[derive(Debug, Clone)]
pub struct TimerPattern {
    /// Timer ID
    pub timer_id: TimerId,
    /// Pattern type
    pub pattern: PatternType,
    /// Predicted period (ns)
    pub predicted_period_ns: f64,
    /// Variance
    pub variance: f64,
    /// Confidence
    pub confidence: f64,
}

/// Pattern type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternType {
    /// Fixed period
    Periodic,
    /// Varying period with trend
    Trending,
    /// Bursty
    Bursty,
    /// Random
    Random,
    /// Unknown
    Unknown,
}

/// Deadline sample
#[derive(Debug, Clone, Copy)]
struct DeadlineSample {
    /// Timestamp
    timestamp: u64,
    /// Deadline
    deadline: u64,
    /// Actual fire time
    actual: u64,
}

/// Predicts timer deadlines and patterns
pub struct DeadlinePredictor {
    /// Per-timer patterns
    patterns: BTreeMap<TimerId, TimerPattern>,
    /// Samples
    samples: BTreeMap<TimerId, Vec<DeadlineSample>>,
    /// Max samples
    max_samples: usize,
}

impl DeadlinePredictor {
    /// Create new predictor
    pub fn new() -> Self {
        Self {
            patterns: BTreeMap::new(),
            samples: BTreeMap::new(),
            max_samples: 100,
        }
    }

    /// Record timer event
    pub fn record(&mut self, timer_id: TimerId, deadline: u64, actual: u64) {
        let sample = DeadlineSample {
            timestamp: NexusTimestamp::now().raw(),
            deadline,
            actual,
        };

        let samples = self.samples.entry(timer_id).or_default();
        samples.push(sample);
        if samples.len() > self.max_samples {
            samples.remove(0);
        }

        self.analyze_pattern(timer_id);
    }

    /// Analyze pattern
    fn analyze_pattern(&mut self, timer_id: TimerId) {
        let samples = match self.samples.get(&timer_id) {
            Some(s) if s.len() >= 5 => s,
            _ => return,
        };

        // Calculate intervals
        let intervals: Vec<f64> = samples
            .windows(2)
            .map(|w| (w[1].deadline - w[0].deadline) as f64)
            .collect();

        if intervals.is_empty() {
            return;
        }

        let mean = intervals.iter().sum::<f64>() / intervals.len() as f64;
        let variance = intervals
            .iter()
            .map(|&i| (i - mean) * (i - mean))
            .sum::<f64>()
            / intervals.len() as f64;

        let std_dev = math::sqrt(variance);
        let cv = if mean > 0.0 { std_dev / mean } else { 1.0 };

        let pattern_type = if cv < 0.01 {
            PatternType::Periodic
        } else if cv < 0.1 {
            PatternType::Trending
        } else if cv < 0.5 {
            PatternType::Bursty
        } else {
            PatternType::Random
        };

        self.patterns.insert(
            timer_id,
            TimerPattern {
                timer_id,
                pattern: pattern_type,
                predicted_period_ns: mean,
                variance,
                confidence: (samples.len() as f64 / self.max_samples as f64).min(1.0),
            },
        );
    }

    /// Predict next deadline
    pub fn predict(&self, timer_id: TimerId, current_deadline: u64) -> Option<u64> {
        let pattern = self.patterns.get(&timer_id)?;

        if pattern.pattern == PatternType::Random {
            return None;
        }

        Some(current_deadline + pattern.predicted_period_ns as u64)
    }

    /// Get pattern
    pub fn get_pattern(&self, timer_id: TimerId) -> Option<&TimerPattern> {
        self.patterns.get(&timer_id)
    }

    /// Get periodic timers
    pub fn periodic_timers(&self) -> Vec<TimerId> {
        self.patterns
            .iter()
            .filter(|(_, p)| p.pattern == PatternType::Periodic)
            .map(|(&id, _)| id)
            .collect()
    }
}

impl Default for DeadlinePredictor {
    fn default() -> Self {
        Self::new()
    }
}
