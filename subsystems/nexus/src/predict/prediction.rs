//! Crash prediction structures
//!
//! This module provides the CrashPrediction struct and related types
//! for representing predicted failures with contributing factors and actions.

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::types::{PredictionConfidence, PredictionKind, Trend};
use crate::core::{ComponentId, NexusTimestamp};

/// A crash prediction
#[derive(Debug, Clone)]
pub struct CrashPrediction {
    /// Unique prediction ID
    pub id: u64,
    /// Timestamp of prediction
    pub timestamp: NexusTimestamp,
    /// Kind of predicted failure
    pub kind: PredictionKind,
    /// Confidence level
    pub confidence: PredictionConfidence,
    /// Estimated time to failure in milliseconds
    pub time_to_failure_ms: u64,
    /// Affected component (if identifiable)
    pub component: Option<ComponentId>,
    /// Contributing factors
    pub factors: Vec<PredictionFactor>,
    /// Recommended action
    pub recommended_action: RecommendedAction,
    /// Whether this prediction was validated (after the fact)
    pub validated: Option<bool>,
}

impl CrashPrediction {
    /// Create a new prediction
    pub fn new(kind: PredictionKind, confidence: f32, time_to_failure_ms: u64) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            timestamp: NexusTimestamp::now(),
            kind,
            confidence: PredictionConfidence::new(confidence),
            time_to_failure_ms,
            component: None,
            factors: Vec::new(),
            recommended_action: RecommendedAction::from_kind(kind, confidence),
            validated: None,
        }
    }

    /// Set affected component
    pub fn with_component(mut self, component: ComponentId) -> Self {
        self.component = Some(component);
        self
    }

    /// Add a contributing factor
    pub fn with_factor(mut self, factor: PredictionFactor) -> Self {
        self.factors.push(factor);
        self
    }

    /// Set recommended action
    pub fn with_action(mut self, action: RecommendedAction) -> Self {
        self.recommended_action = action;
        self
    }

    /// Mark prediction as validated
    pub fn validate(&mut self, correct: bool) {
        self.validated = Some(correct);
    }

    /// Check if prediction is urgent (< 5s)
    pub fn is_urgent(&self) -> bool {
        self.time_to_failure_ms < 5000
    }

    /// Check if prediction is critical
    pub fn is_critical(&self) -> bool {
        self.kind.is_critical() || (self.confidence.is_high() && self.is_urgent())
    }
}

/// A factor contributing to a prediction
#[derive(Debug, Clone)]
pub struct PredictionFactor {
    /// Factor name
    pub name: String,
    /// Current value
    pub current_value: f64,
    /// Threshold value
    pub threshold: f64,
    /// Trend direction
    pub trend: Trend,
    /// Weight in prediction (0.0 - 1.0)
    pub weight: f32,
}

/// Recommended action for a prediction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecommendedAction {
    /// Just monitor
    Monitor,
    /// Log and alert
    Alert,
    /// Prepare for recovery
    Prepare,
    /// Soft recovery
    SoftRecover,
    /// Hard recovery
    HardRecover,
    /// Immediate rollback
    Rollback,
    /// Quarantine component
    Quarantine,
    /// Enter survival mode
    SurvivalMode,
}

impl RecommendedAction {
    /// Determine action from prediction kind and confidence
    pub fn from_kind(kind: PredictionKind, confidence: f32) -> Self {
        if confidence < 0.5 {
            return Self::Monitor;
        }

        match kind {
            PredictionKind::Crash | PredictionKind::StackOverflow if confidence > 0.8 => {
                Self::SurvivalMode
            },
            PredictionKind::Crash | PredictionKind::StackOverflow => Self::Rollback,
            PredictionKind::Deadlock if confidence > 0.8 => Self::HardRecover,
            PredictionKind::Deadlock => Self::SoftRecover,
            PredictionKind::OutOfMemory if confidence > 0.8 => Self::SoftRecover,
            PredictionKind::OutOfMemory => Self::Prepare,
            PredictionKind::MemoryLeak => Self::Alert,
            PredictionKind::Corruption | PredictionKind::SecurityViolation => Self::Quarantine,
            PredictionKind::Degradation => Self::Monitor,
            _ if confidence > 0.8 => Self::SoftRecover,
            _ => Self::Alert,
        }
    }

    /// Get urgency level (1-10)
    pub fn urgency(&self) -> u8 {
        match self {
            Self::Monitor => 1,
            Self::Alert => 2,
            Self::Prepare => 4,
            Self::SoftRecover => 6,
            Self::HardRecover => 7,
            Self::Rollback => 8,
            Self::Quarantine => 8,
            Self::SurvivalMode => 10,
        }
    }
}
