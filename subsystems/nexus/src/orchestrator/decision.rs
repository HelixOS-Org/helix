//! Orchestrator Decision System
//!
//! Decision types, actions, and lifecycle management.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::{DecisionId, SubsystemId};

// ============================================================================
// DECISION TYPES
// ============================================================================

/// Decision type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionType {
    /// Resource allocation
    ResourceAllocation,
    /// Performance optimization
    PerformanceOptimization,
    /// Power management
    PowerManagement,
    /// Thermal management
    ThermalManagement,
    /// Security response
    SecurityResponse,
    /// Fault recovery
    FaultRecovery,
    /// Load balancing
    LoadBalancing,
    /// Capacity planning
    CapacityPlanning,
    /// Policy adjustment
    PolicyAdjustment,
}

impl DecisionType {
    /// Get type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::ResourceAllocation => "resource_allocation",
            Self::PerformanceOptimization => "performance_optimization",
            Self::PowerManagement => "power_management",
            Self::ThermalManagement => "thermal_management",
            Self::SecurityResponse => "security_response",
            Self::FaultRecovery => "fault_recovery",
            Self::LoadBalancing => "load_balancing",
            Self::CapacityPlanning => "capacity_planning",
            Self::PolicyAdjustment => "policy_adjustment",
        }
    }
}

/// Decision urgency
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DecisionUrgency {
    /// Deferred - can wait
    Deferred = 0,
    /// Normal - process in order
    Normal   = 1,
    /// Elevated - process soon
    Elevated = 2,
    /// Urgent - process immediately
    Urgent   = 3,
    /// Critical - interrupt current work
    Critical = 4,
}

impl DecisionUrgency {
    /// Get urgency name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Deferred => "deferred",
            Self::Normal => "normal",
            Self::Elevated => "elevated",
            Self::Urgent => "urgent",
            Self::Critical => "critical",
        }
    }
}

/// Decision status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionStatus {
    /// Pending evaluation
    Pending,
    /// Being evaluated
    Evaluating,
    /// Approved
    Approved,
    /// Rejected
    Rejected,
    /// Executing
    Executing,
    /// Completed
    Completed,
    /// Failed
    Failed,
    /// Cancelled
    Cancelled,
}

impl DecisionStatus {
    /// Get status name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Evaluating => "evaluating",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Executing => "executing",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    /// Is terminal state
    #[inline]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Cancelled | Self::Rejected
        )
    }
}

// ============================================================================
// DECISION ACTION
// ============================================================================

/// Decision action
#[derive(Debug, Clone)]
pub struct DecisionAction {
    /// Action name
    pub name: String,
    /// Target subsystem
    pub target: SubsystemId,
    /// Parameters
    pub parameters: BTreeMap<String, String>,
    /// Estimated impact (positive = good)
    pub estimated_impact: i32,
    /// Estimated duration (us)
    pub estimated_duration_us: u64,
}

impl DecisionAction {
    /// Create new action
    pub fn new(name: String, target: SubsystemId) -> Self {
        Self {
            name,
            target,
            parameters: BTreeMap::new(),
            estimated_impact: 0,
            estimated_duration_us: 0,
        }
    }

    /// Add parameter
    #[inline]
    pub fn with_param(mut self, key: &str, value: &str) -> Self {
        self.parameters
            .insert(String::from(key), String::from(value));
        self
    }
}

// ============================================================================
// DECISION
// ============================================================================

/// Decision
#[derive(Debug)]
pub struct Decision {
    /// Decision ID
    pub id: DecisionId,
    /// Decision type
    pub decision_type: DecisionType,
    /// Urgency
    pub urgency: DecisionUrgency,
    /// Status
    pub status: DecisionStatus,
    /// Source subsystem
    pub source: SubsystemId,
    /// Reason
    pub reason: String,
    /// Actions to take
    pub actions: Vec<DecisionAction>,
    /// Confidence (0-100)
    pub confidence: u8,
    /// Created timestamp
    pub created_at: u64,
    /// Updated timestamp
    pub updated_at: u64,
    /// Dependencies
    pub dependencies: Vec<DecisionId>,
}

impl Decision {
    /// Create new decision
    pub fn new(
        id: DecisionId,
        decision_type: DecisionType,
        source: SubsystemId,
        reason: String,
    ) -> Self {
        Self {
            id,
            decision_type,
            urgency: DecisionUrgency::Normal,
            status: DecisionStatus::Pending,
            source,
            reason,
            actions: Vec::new(),
            confidence: 50,
            created_at: 0,
            updated_at: 0,
            dependencies: Vec::new(),
        }
    }

    /// Add action
    #[inline(always)]
    pub fn add_action(&mut self, action: DecisionAction) {
        self.actions.push(action);
    }

    /// Set urgency
    #[inline(always)]
    pub fn with_urgency(mut self, urgency: DecisionUrgency) -> Self {
        self.urgency = urgency;
        self
    }

    /// Set confidence
    #[inline(always)]
    pub fn with_confidence(mut self, confidence: u8) -> Self {
        self.confidence = confidence.min(100);
        self
    }

    /// Total estimated impact
    #[inline(always)]
    pub fn total_impact(&self) -> i32 {
        self.actions.iter().map(|a| a.estimated_impact).sum()
    }

    /// Is high priority
    #[inline]
    pub fn is_high_priority(&self) -> bool {
        matches!(
            self.urgency,
            DecisionUrgency::Urgent | DecisionUrgency::Critical
        )
    }
}
