//! # Cooperation Feedback & Metrics
//!
//! Collects and analyzes feedback on the cooperation protocol's effectiveness.
//! Tracks compliance, benefit metrics, and overall cooperation health.

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

// ============================================================================
// FEEDBACK TYPES
// ============================================================================

/// Type of feedback event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackType {
    /// App followed a kernel advisory
    AdvisoryFollowed,
    /// App ignored a kernel advisory
    AdvisoryIgnored,
    /// App hint was accurate
    HintAccurate,
    /// App hint was inaccurate
    HintInaccurate,
    /// Contract was fulfilled by kernel
    ContractFulfilled,
    /// Contract was violated by kernel
    ContractViolatedByKernel,
    /// Contract was violated by app
    ContractViolatedByApp,
    /// Cooperation resulted in measurable improvement
    PerformanceImproved,
    /// Cooperation had no measurable effect
    NoEffect,
    /// Cooperation had negative effect
    PerformanceDegraded,
}

/// A feedback event
#[derive(Debug, Clone)]
pub struct CoopFeedback {
    /// Type of feedback
    pub feedback_type: FeedbackType,
    /// Process ID
    pub pid: u64,
    /// Session ID
    pub session_id: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Measured improvement ratio (1.0 = no change, >1.0 = improvement)
    pub improvement_ratio: f64,
    /// Additional context (e.g., latency delta in microseconds)
    pub context_value: i64,
}

impl CoopFeedback {
    pub fn new(feedback_type: FeedbackType, pid: u64, session_id: u64) -> Self {
        Self {
            feedback_type,
            pid,
            session_id,
            timestamp: 0,
            improvement_ratio: 1.0,
            context_value: 0,
        }
    }

    #[inline(always)]
    pub fn with_improvement(mut self, ratio: f64) -> Self {
        self.improvement_ratio = ratio;
        self
    }

    #[inline(always)]
    pub fn with_context(mut self, value: i64) -> Self {
        self.context_value = value;
        self
    }

    #[inline(always)]
    pub fn with_timestamp(mut self, ts: u64) -> Self {
        self.timestamp = ts;
        self
    }
}

// ============================================================================
// COOPERATION METRICS (per session)
// ============================================================================

/// Metrics for a single cooperation session
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CoopMetrics {
    /// Session ID
    pub session_id: u64,
    /// Process ID
    pub pid: u64,
    /// Advisories sent
    pub advisories_sent: u64,
    /// Advisories followed
    pub advisories_followed: u64,
    /// Advisories ignored
    pub advisories_ignored: u64,
    /// Hints submitted by app
    pub hints_submitted: u64,
    /// Hints that were accurate
    pub hints_accurate: u64,
    /// Hints that were inaccurate
    pub hints_inaccurate: u64,
    /// Contracts established
    pub contracts_total: u64,
    /// Contracts fulfilled
    pub contracts_fulfilled: u64,
    /// Contracts violated
    pub contracts_violated: u64,
    /// Total measured improvement ratio (sum)
    improvement_sum: f64,
    /// Number of improvement samples
    improvement_count: u64,
}

impl CoopMetrics {
    pub fn new(session_id: u64, pid: u64) -> Self {
        Self {
            session_id,
            pid,
            advisories_sent: 0,
            advisories_followed: 0,
            advisories_ignored: 0,
            hints_submitted: 0,
            hints_accurate: 0,
            hints_inaccurate: 0,
            contracts_total: 0,
            contracts_fulfilled: 0,
            contracts_violated: 0,
            improvement_sum: 0.0,
            improvement_count: 0,
        }
    }

    /// Advisory compliance rate (0.0 - 1.0)
    #[inline]
    pub fn advisory_compliance(&self) -> f64 {
        let total = self.advisories_followed + self.advisories_ignored;
        if total == 0 {
            return 1.0; // No advisories yet, assume compliant
        }
        self.advisories_followed as f64 / total as f64
    }

    /// Hint accuracy rate (0.0 - 1.0)
    #[inline]
    pub fn hint_accuracy(&self) -> f64 {
        let total = self.hints_accurate + self.hints_inaccurate;
        if total == 0 {
            return 0.5; // Unknown accuracy
        }
        self.hints_accurate as f64 / total as f64
    }

    /// Contract fulfillment rate (0.0 - 1.0)
    #[inline]
    pub fn contract_fulfillment(&self) -> f64 {
        if self.contracts_total == 0 {
            return 1.0;
        }
        self.contracts_fulfilled as f64 / self.contracts_total as f64
    }

    /// Average measured improvement
    #[inline]
    pub fn average_improvement(&self) -> f64 {
        if self.improvement_count == 0 {
            return 1.0;
        }
        self.improvement_sum / self.improvement_count as f64
    }

    /// Overall cooperation health score (0.0 - 1.0)
    #[inline]
    pub fn health_score(&self) -> f64 {
        let compliance = self.advisory_compliance();
        let accuracy = self.hint_accuracy();
        let fulfillment = self.contract_fulfillment();
        let improvement = (self.average_improvement() - 1.0).max(0.0).min(1.0);

        // Weighted score
        compliance * 0.3 + accuracy * 0.25 + fulfillment * 0.25 + improvement * 0.2
    }
}

// ============================================================================
// FEEDBACK COLLECTOR
// ============================================================================

const MAX_HISTORY_PER_SESSION: usize = 100;

/// Collects and aggregates cooperation feedback
pub struct FeedbackCollector {
    /// Metrics per session
    metrics: BTreeMap<u64, CoopMetrics>,
    /// Recent feedback history per session
    history: BTreeMap<u64, Vec<CoopFeedback>>,
    /// Global totals
    global_feedback_count: u64,
    /// Global improvement sum
    global_improvement_sum: f64,
    /// Global improvement count
    global_improvement_count: u64,
}

impl FeedbackCollector {
    pub fn new() -> Self {
        Self {
            metrics: BTreeMap::new(),
            history: BTreeMap::new(),
            global_feedback_count: 0,
            global_improvement_sum: 0.0,
            global_improvement_count: 0,
        }
    }

    /// Initialize metrics for a session
    #[inline]
    pub fn init_session(&mut self, session_id: u64, pid: u64) {
        self.metrics
            .entry(session_id)
            .or_insert_with(|| CoopMetrics::new(session_id, pid));
        self.history.entry(session_id).or_insert_with(Vec::new);
    }

    /// Record feedback
    pub fn record(&mut self, feedback: CoopFeedback) {
        self.global_feedback_count += 1;

        let session_id = feedback.session_id;
        let pid = feedback.pid;

        // Ensure metrics exist
        let metrics = self
            .metrics
            .entry(session_id)
            .or_insert_with(|| CoopMetrics::new(session_id, pid));

        // Update metrics based on feedback type
        match feedback.feedback_type {
            FeedbackType::AdvisoryFollowed => metrics.advisories_followed += 1,
            FeedbackType::AdvisoryIgnored => metrics.advisories_ignored += 1,
            FeedbackType::HintAccurate => metrics.hints_accurate += 1,
            FeedbackType::HintInaccurate => metrics.hints_inaccurate += 1,
            FeedbackType::ContractFulfilled => metrics.contracts_fulfilled += 1,
            FeedbackType::ContractViolatedByKernel | FeedbackType::ContractViolatedByApp => {
                metrics.contracts_violated += 1;
            },
            FeedbackType::PerformanceImproved
            | FeedbackType::NoEffect
            | FeedbackType::PerformanceDegraded => {
                metrics.improvement_sum += feedback.improvement_ratio;
                metrics.improvement_count += 1;
                self.global_improvement_sum += feedback.improvement_ratio;
                self.global_improvement_count += 1;
            },
        }

        // Store in history
        let hist = self.history.entry(session_id).or_insert_with(Vec::new);
        if hist.len() >= MAX_HISTORY_PER_SESSION {
            hist.pop_front();
        }
        hist.push(feedback);
    }

    /// Get metrics for a session
    #[inline(always)]
    pub fn get_metrics(&self, session_id: u64) -> Option<&CoopMetrics> {
        self.metrics.get(&session_id)
    }

    /// Get global average improvement ratio
    #[inline]
    pub fn global_improvement(&self) -> f64 {
        if self.global_improvement_count == 0 {
            return 1.0;
        }
        self.global_improvement_sum / self.global_improvement_count as f64
    }

    /// Get the total number of sessions being tracked
    #[inline(always)]
    pub fn session_count(&self) -> usize {
        self.metrics.len()
    }

    /// Get the least cooperative sessions (by health score)
    #[inline]
    pub fn least_cooperative(&self, n: usize) -> Vec<(u64, f64)> {
        let mut scores: Vec<(u64, f64)> = self
            .metrics
            .iter()
            .map(|(&sid, m)| (sid, m.health_score()))
            .collect();
        scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));
        scores.truncate(n);
        scores
    }

    /// Remove session data
    #[inline(always)]
    pub fn remove_session(&mut self, session_id: u64) {
        self.metrics.remove(&session_id);
        self.history.remove(&session_id);
    }
}
