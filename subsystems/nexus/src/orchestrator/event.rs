//! Orchestrator Events
//!
//! Event types and tracking for orchestrator system.

use alloc::string::String;

use super::{DecisionId, EventId, SubsystemId};

// ============================================================================
// EVENT TYPES
// ============================================================================

/// Event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchestratorEventType {
    /// Subsystem registered
    SubsystemRegistered,
    /// Subsystem health changed
    HealthChanged,
    /// Decision created
    DecisionCreated,
    /// Decision executed
    DecisionExecuted,
    /// Policy changed
    PolicyChanged,
    /// Alert raised
    AlertRaised,
    /// Resource contention
    ResourceContention,
    /// Anomaly detected
    AnomalyDetected,
}

impl OrchestratorEventType {
    /// Get type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::SubsystemRegistered => "subsystem_registered",
            Self::HealthChanged => "health_changed",
            Self::DecisionCreated => "decision_created",
            Self::DecisionExecuted => "decision_executed",
            Self::PolicyChanged => "policy_changed",
            Self::AlertRaised => "alert_raised",
            Self::ResourceContention => "resource_contention",
            Self::AnomalyDetected => "anomaly_detected",
        }
    }
}

// ============================================================================
// ORCHESTRATOR EVENT
// ============================================================================

/// Orchestrator event
#[derive(Debug, Clone)]
pub struct OrchestratorEvent {
    /// Event ID
    pub id: EventId,
    /// Event type
    pub event_type: OrchestratorEventType,
    /// Timestamp
    pub timestamp: u64,
    /// Source subsystem
    pub source: Option<SubsystemId>,
    /// Related decision
    pub decision: Option<DecisionId>,
    /// Description
    pub description: String,
}

impl OrchestratorEvent {
    /// Create new event
    pub fn new(id: EventId, event_type: OrchestratorEventType, timestamp: u64) -> Self {
        Self {
            id,
            event_type,
            timestamp,
            source: None,
            decision: None,
            description: String::new(),
        }
    }

    /// Set source
    #[inline(always)]
    pub fn with_source(mut self, source: SubsystemId) -> Self {
        self.source = Some(source);
        self
    }

    /// Set decision
    #[inline(always)]
    pub fn with_decision(mut self, decision: DecisionId) -> Self {
        self.decision = Some(decision);
        self
    }

    /// Set description
    #[inline(always)]
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }
}
