//! NEXUS event representation

#![allow(dead_code)]

use super::kind::NexusEventKind;
use super::types::{EventId, EventPriority};
use crate::core::{ComponentId, NexusTimestamp};

// ============================================================================
// NEXUS EVENT
// ============================================================================

/// A NEXUS event
#[derive(Debug, Clone)]
pub struct NexusEvent {
    /// Unique event ID
    pub id: EventId,
    /// Event timestamp
    pub timestamp: NexusTimestamp,
    /// Event priority
    pub priority: EventPriority,
    /// Event kind
    pub kind: NexusEventKind,
    /// Source component (if any)
    pub source: Option<ComponentId>,
    /// Correlation ID for related events
    pub correlation_id: Option<u64>,
    /// Parent event (for causal chains)
    pub parent_id: Option<EventId>,
}

impl NexusEvent {
    /// Create a new event
    pub fn new(kind: NexusEventKind) -> Self {
        Self {
            id: EventId::new(),
            timestamp: NexusTimestamp::now(),
            priority: Self::default_priority(&kind),
            kind,
            source: None,
            correlation_id: None,
            parent_id: None,
        }
    }

    /// Create with specific priority
    pub fn with_priority(kind: NexusEventKind, priority: EventPriority) -> Self {
        Self {
            priority,
            ..Self::new(kind)
        }
    }

    /// Set source component
    pub fn from_component(mut self, component: ComponentId) -> Self {
        self.source = Some(component);
        self
    }

    /// Set correlation ID
    pub fn with_correlation(mut self, correlation_id: u64) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    /// Set parent event
    pub fn with_parent(mut self, parent: EventId) -> Self {
        self.parent_id = Some(parent);
        self
    }

    /// Get default priority for an event kind
    fn default_priority(kind: &NexusEventKind) -> EventPriority {
        match kind {
            NexusEventKind::SystemInit | NexusEventKind::SystemShutdown => EventPriority::Critical,
            NexusEventKind::CrashPredicted { confidence, .. } if *confidence > 0.9 => {
                EventPriority::Emergency
            },
            NexusEventKind::CrashPredicted { .. } => EventPriority::Critical,
            NexusEventKind::ComponentPanic { .. } => EventPriority::Emergency,
            NexusEventKind::DeadlockPredicted { .. } => EventPriority::Critical,
            NexusEventKind::HealingStarted { .. } | NexusEventKind::HealingCompleted { .. } => {
                EventPriority::High
            },
            NexusEventKind::AnomalyDetected { severity, .. } if *severity > 0.8 => {
                EventPriority::High
            },
            NexusEventKind::Tick { .. } | NexusEventKind::Heartbeat => EventPriority::Low,
            NexusEventKind::SpanStarted { .. } | NexusEventKind::SpanEnded { .. } => {
                EventPriority::Background
            },
            _ => EventPriority::Normal,
        }
    }

    /// Check if this is a critical event
    pub fn is_critical(&self) -> bool {
        self.priority >= EventPriority::Critical
    }

    /// Check if this is a prediction event
    pub fn is_prediction(&self) -> bool {
        matches!(
            self.kind,
            NexusEventKind::CrashPredicted { .. }
                | NexusEventKind::DegradationDetected { .. }
                | NexusEventKind::DeadlockPredicted { .. }
                | NexusEventKind::MemoryLeakDetected { .. }
                | NexusEventKind::ResourceExhaustionPredicted { .. }
        )
    }

    /// Check if this is a healing event
    pub fn is_healing(&self) -> bool {
        matches!(
            self.kind,
            NexusEventKind::HealingStarted { .. }
                | NexusEventKind::HealingCompleted { .. }
                | NexusEventKind::RollbackStarted { .. }
                | NexusEventKind::RollbackCompleted { .. }
                | NexusEventKind::ComponentQuarantined { .. }
                | NexusEventKind::ComponentRestored { .. }
        )
    }
}
