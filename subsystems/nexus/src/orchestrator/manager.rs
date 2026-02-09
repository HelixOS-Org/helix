//! Orchestrator Manager
//!
//! Central manager for subsystems, decisions, and events.

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::{
    Decision, DecisionId, DecisionStatus, DecisionType, EventId, HealthLevel, OrchestratorEvent,
    OrchestratorEventType, SubsystemId, SubsystemState, SystemPolicy,
};

// ============================================================================
// ORCHESTRATOR MANAGER
// ============================================================================

/// Orchestrator manager
pub struct OrchestratorManager {
    /// Subsystems
    pub(crate) subsystems: BTreeMap<SubsystemId, SubsystemState>,
    /// Pending decisions
    pub(crate) pending_decisions: Vec<Decision>,
    /// Completed decisions
    completed_decisions: Vec<Decision>,
    /// Events
    events: VecDeque<OrchestratorEvent>,
    /// Max events
    max_events: usize,
    /// Current policy
    policy: SystemPolicy,
    /// Decision counter
    decision_counter: AtomicU64,
    /// Event counter
    event_counter: AtomicU64,
    /// Enabled
    enabled: AtomicBool,
    /// Total decisions
    total_decisions: AtomicU64,
}

impl OrchestratorManager {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            subsystems: BTreeMap::new(),
            pending_decisions: Vec::new(),
            completed_decisions: Vec::new(),
            events: VecDeque::new(),
            max_events: 1000,
            policy: SystemPolicy::default(),
            decision_counter: AtomicU64::new(0),
            event_counter: AtomicU64::new(0),
            enabled: AtomicBool::new(true),
            total_decisions: AtomicU64::new(0),
        }
    }

    /// Register subsystem
    #[inline(always)]
    pub fn register_subsystem(&mut self, state: SubsystemState) {
        self.subsystems.insert(state.id, state);
    }

    /// Get subsystem
    #[inline(always)]
    pub fn get_subsystem(&self, id: SubsystemId) -> Option<&SubsystemState> {
        self.subsystems.get(&id)
    }

    /// Get subsystem mutably
    #[inline(always)]
    pub fn get_subsystem_mut(&mut self, id: SubsystemId) -> Option<&mut SubsystemState> {
        self.subsystems.get_mut(&id)
    }

    /// Create decision
    #[inline]
    pub fn create_decision(
        &mut self,
        decision_type: DecisionType,
        source: SubsystemId,
        reason: String,
        timestamp: u64,
    ) -> DecisionId {
        let id = DecisionId(self.decision_counter.fetch_add(1, Ordering::Relaxed));
        let mut decision = Decision::new(id, decision_type, source, reason);
        decision.created_at = timestamp;
        decision.updated_at = timestamp;
        self.pending_decisions.push(decision);
        self.total_decisions.fetch_add(1, Ordering::Relaxed);
        id
    }

    /// Get pending decision
    #[inline(always)]
    pub fn get_pending_decision(&self, id: DecisionId) -> Option<&Decision> {
        self.pending_decisions.iter().find(|d| d.id == id)
    }

    /// Get pending decision mutably
    #[inline(always)]
    pub fn get_pending_decision_mut(&mut self, id: DecisionId) -> Option<&mut Decision> {
        self.pending_decisions.iter_mut().find(|d| d.id == id)
    }

    /// Approve decision
    #[inline]
    pub fn approve_decision(&mut self, id: DecisionId) -> bool {
        if let Some(decision) = self.get_pending_decision_mut(id) {
            decision.status = DecisionStatus::Approved;
            return true;
        }
        false
    }

    /// Execute decision
    #[inline]
    pub fn execute_decision(&mut self, id: DecisionId) -> bool {
        if let Some(decision) = self.get_pending_decision_mut(id) {
            if decision.status == DecisionStatus::Approved {
                decision.status = DecisionStatus::Executing;
                return true;
            }
        }
        false
    }

    /// Complete decision
    #[inline]
    pub fn complete_decision(&mut self, id: DecisionId) -> bool {
        if let Some(idx) = self.pending_decisions.iter().position(|d| d.id == id) {
            let mut decision = self.pending_decisions.remove(idx);
            decision.status = DecisionStatus::Completed;
            self.completed_decisions.push(decision);
            return true;
        }
        false
    }

    /// Record event
    #[inline]
    pub fn record_event(&mut self, event_type: OrchestratorEventType, timestamp: u64) -> EventId {
        let id = EventId(self.event_counter.fetch_add(1, Ordering::Relaxed));
        let event = OrchestratorEvent::new(id, event_type, timestamp);

        if self.events.len() >= self.max_events {
            self.events.pop_front();
        }
        self.events.push_back(event);
        id
    }

    /// Set policy
    #[inline(always)]
    pub fn set_policy(&mut self, policy: SystemPolicy) {
        self.policy = policy;
    }

    /// Get policy
    #[inline(always)]
    pub fn policy(&self) -> &SystemPolicy {
        &self.policy
    }

    /// High priority pending decisions
    #[inline]
    pub fn high_priority_decisions(&self) -> Vec<&Decision> {
        self.pending_decisions
            .iter()
            .filter(|d| d.is_high_priority())
            .collect()
    }

    /// Overall system health
    pub fn system_health(&self) -> HealthLevel {
        if self.subsystems.is_empty() {
            return HealthLevel::Healthy;
        }

        let avg_score: u32 = self
            .subsystems
            .values()
            .map(|s| s.health_score())
            .sum::<u32>()
            / self.subsystems.len() as u32;

        HealthLevel::from_score(avg_score as u8)
    }

    /// Critical subsystems
    #[inline]
    pub fn critical_subsystems(&self) -> Vec<&SubsystemState> {
        self.subsystems
            .values()
            .filter(|s| matches!(s.health, HealthLevel::Critical | HealthLevel::Degraded))
            .collect()
    }

    /// Subsystem count
    #[inline(always)]
    pub fn subsystem_count(&self) -> usize {
        self.subsystems.len()
    }

    /// Total decisions made
    #[inline(always)]
    pub fn total_decisions(&self) -> u64 {
        self.total_decisions.load(Ordering::Relaxed)
    }

    /// Is enabled
    #[inline(always)]
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Enable/disable
    #[inline(always)]
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Pending decisions count
    #[inline(always)]
    pub fn pending_count(&self) -> usize {
        self.pending_decisions.len()
    }

    /// Completed decisions count
    #[inline(always)]
    pub fn completed_count(&self) -> usize {
        self.completed_decisions.len()
    }

    /// Events count
    #[inline(always)]
    pub fn events_count(&self) -> usize {
        self.events.len()
    }
}

impl Default for OrchestratorManager {
    fn default() -> Self {
        Self::new()
    }
}
