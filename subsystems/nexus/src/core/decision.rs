//! NEXUS decision types.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::timestamp::NexusTimestamp;

/// A decision made by NEXUS
#[derive(Debug, Clone)]
pub struct NexusDecision {
    /// Unique decision ID
    pub id: u64,
    /// Timestamp of decision
    pub timestamp: NexusTimestamp,
    /// Decision type
    pub kind: DecisionKind,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f32,
    /// Reasoning chain
    pub reasoning: Vec<String>,
    /// Time taken to decide (cycles)
    pub decision_time: u64,
    /// Was this decision correct? (filled in later)
    pub outcome: Option<DecisionOutcome>,
}

/// Kind of decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecisionKind {
    /// Do nothing
    NoAction,
    /// Log an observation
    Log,
    /// Warn about potential issue
    Warn,
    /// Soft recovery attempt
    SoftRecover,
    /// Hard recovery attempt
    HardRecover,
    /// Isolate a component
    Isolate,
    /// Substitute a component
    Substitute,
    /// Rollback to checkpoint
    Rollback,
    /// Enter survival mode
    SurvivalMode,
    /// Predictive action
    Predictive,
    /// Performance optimization
    Optimize,
}

/// Outcome of a decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecisionOutcome {
    /// Decision was successful
    Success,
    /// Decision partially succeeded
    Partial,
    /// Decision had no effect
    NoEffect,
    /// Decision failed
    Failed,
    /// Decision made things worse
    Regression,
}

impl NexusDecision {
    /// Create a new decision
    pub fn new(kind: DecisionKind, confidence: f32) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            timestamp: NexusTimestamp::now(),
            kind,
            confidence,
            reasoning: Vec::new(),
            decision_time: 0,
            outcome: None,
        }
    }

    /// Add reasoning step
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reasoning.push(reason.into());
        self
    }

    /// Set decision time
    pub fn with_decision_time(mut self, cycles: u64) -> Self {
        self.decision_time = cycles;
        self
    }

    /// Record outcome
    pub fn record_outcome(&mut self, outcome: DecisionOutcome) {
        self.outcome = Some(outcome);
    }
}
