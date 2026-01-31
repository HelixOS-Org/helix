//! # Cognitive Context Management
//!
//! Maintains and shares cognitive context across domains.
//! Tracks the current state of the cognitive cycle.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{ComponentId, DomainId, Timestamp};

// ============================================================================
// CONTEXT TYPES
// ============================================================================

/// Current cognitive context
#[derive(Debug, Clone)]
pub struct CognitiveContext {
    /// Context ID
    pub id: u64,
    /// Current cycle
    pub cycle: u64,
    /// Current phase
    pub phase: CognitivePhase,
    /// Active focus
    pub focus: Option<ContextFocus>,
    /// Current goals
    pub goals: Vec<ContextGoal>,
    /// Active constraints
    pub constraints: Vec<ContextConstraint>,
    /// Environment state
    pub environment: EnvironmentState,
    /// Domain states
    pub domain_states: BTreeMap<DomainId, DomainContextState>,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Parent context (for nested contexts)
    pub parent: Option<u64>,
}

/// Cognitive processing phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CognitivePhase {
    /// Sensing and collecting signals
    Sensing,
    /// Understanding and pattern matching
    Understanding,
    /// Reasoning about causes
    Reasoning,
    /// Making decisions
    Deciding,
    /// Taking action
    Acting,
    /// Reflecting on outcomes
    Reflecting,
    /// Learning from experience
    Learning,
    /// Memory consolidation
    Consolidating,
    /// Idle
    Idle,
}

impl CognitivePhase {
    /// Get phase order
    pub fn order(&self) -> u8 {
        match self {
            Self::Sensing => 0,
            Self::Understanding => 1,
            Self::Reasoning => 2,
            Self::Deciding => 3,
            Self::Acting => 4,
            Self::Reflecting => 5,
            Self::Learning => 6,
            Self::Consolidating => 7,
            Self::Idle => 255,
        }
    }

    /// Get next phase
    pub fn next(&self) -> Self {
        match self {
            Self::Sensing => Self::Understanding,
            Self::Understanding => Self::Reasoning,
            Self::Reasoning => Self::Deciding,
            Self::Deciding => Self::Acting,
            Self::Acting => Self::Reflecting,
            Self::Reflecting => Self::Learning,
            Self::Learning => Self::Consolidating,
            Self::Consolidating => Self::Idle,
            Self::Idle => Self::Sensing,
        }
    }
}

/// Current focus of cognitive attention
#[derive(Debug, Clone)]
pub struct ContextFocus {
    /// Focus type
    pub focus_type: FocusType,
    /// Primary target
    pub target: FocusTarget,
    /// Secondary targets
    pub secondary: Vec<FocusTarget>,
    /// Focus strength (0.0 - 1.0)
    pub strength: f32,
    /// Focus duration (cycles)
    pub duration: u64,
    /// Why this focus
    pub reason: String,
}

/// Type of focus
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusType {
    /// Anomaly detection
    Anomaly,
    /// Problem solving
    Problem,
    /// Optimization
    Optimization,
    /// Exploration
    Exploration,
    /// Maintenance
    Maintenance,
    /// Emergency
    Emergency,
}

/// Focus target
#[derive(Debug, Clone)]
pub enum FocusTarget {
    /// Component
    Component(ComponentId),
    /// Subsystem
    Subsystem(String),
    /// Pattern
    Pattern(u64),
    /// Resource
    Resource(String),
    /// Event
    Event(u64),
}

/// Cognitive goal
#[derive(Debug, Clone)]
pub struct ContextGoal {
    /// Goal ID
    pub id: u64,
    /// Goal type
    pub goal_type: GoalType,
    /// Description
    pub description: String,
    /// Priority
    pub priority: u32,
    /// Progress (0.0 - 1.0)
    pub progress: f32,
    /// Deadline (optional)
    pub deadline: Option<Timestamp>,
    /// Dependencies
    pub depends_on: Vec<u64>,
}

/// Goal type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalType {
    /// Maintain stability
    Stability,
    /// Optimize performance
    Performance,
    /// Heal problems
    Healing,
    /// Learn patterns
    Learning,
    /// Prevent issues
    Prevention,
    /// Custom goal
    Custom,
}

/// Constraint on cognitive processing
#[derive(Debug, Clone)]
pub struct ContextConstraint {
    /// Constraint ID
    pub id: u64,
    /// Constraint type
    pub constraint_type: ConstraintType,
    /// Constraint value
    pub value: ConstraintValue,
    /// Is hard constraint
    pub hard: bool,
    /// Source
    pub source: String,
}

/// Constraint type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintType {
    /// Time limit
    Time,
    /// Resource limit
    Resource,
    /// Action restriction
    Action,
    /// Priority override
    Priority,
    /// Scope limit
    Scope,
}

/// Constraint value
#[derive(Debug, Clone)]
pub enum ConstraintValue {
    /// Maximum value
    Max(f64),
    /// Minimum value
    Min(f64),
    /// Range
    Range(f64, f64),
    /// List of allowed values
    Allowed(Vec<String>),
    /// List of forbidden values
    Forbidden(Vec<String>),
}

/// Environment state
#[derive(Debug, Clone)]
pub struct EnvironmentState {
    /// System load
    pub load: f32,
    /// Memory pressure
    pub memory_pressure: f32,
    /// Active component count
    pub active_components: u32,
    /// Error rate
    pub error_rate: f32,
    /// Is in emergency
    pub emergency: bool,
    /// Current mode
    pub mode: SystemMode,
}

/// System operational mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemMode {
    /// Normal operation
    Normal,
    /// Low power
    LowPower,
    /// High performance
    HighPerformance,
    /// Maintenance
    Maintenance,
    /// Emergency
    Emergency,
    /// Recovery
    Recovery,
}

impl Default for EnvironmentState {
    fn default() -> Self {
        Self {
            load: 0.0,
            memory_pressure: 0.0,
            active_components: 0,
            error_rate: 0.0,
            emergency: false,
            mode: SystemMode::Normal,
        }
    }
}

/// Domain-specific context state
#[derive(Debug, Clone)]
pub struct DomainContextState {
    /// Domain ID
    pub domain_id: DomainId,
    /// Is active
    pub active: bool,
    /// Last update cycle
    pub last_update: u64,
    /// Processing progress
    pub progress: f32,
    /// Has pending output
    pub has_output: bool,
    /// Error count
    pub error_count: u32,
}

// ============================================================================
// CONTEXT MANAGER
// ============================================================================

/// Manages cognitive context
pub struct ContextManager {
    /// Current context
    current: CognitiveContext,
    /// Context history
    history: Vec<CognitiveContext>,
    /// Next context ID
    next_id: AtomicU64,
    /// Next goal ID
    next_goal_id: AtomicU64,
    /// Next constraint ID
    next_constraint_id: AtomicU64,
    /// Configuration
    config: ContextConfig,
    /// Statistics
    stats: ContextStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct ContextConfig {
    /// Maximum history size
    pub max_history: usize,
    /// Default phase duration (cycles)
    pub phase_duration: u64,
    /// Enable context chaining
    pub enable_chaining: bool,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_history: 100,
            phase_duration: 10,
            enable_chaining: true,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct ContextStats {
    /// Total contexts created
    pub total_contexts: u64,
    /// Total phase transitions
    pub phase_transitions: u64,
    /// Goals completed
    pub goals_completed: u64,
    /// Goals failed
    pub goals_failed: u64,
    /// Constraint violations
    pub constraint_violations: u64,
}

impl ContextManager {
    /// Create a new context manager
    pub fn new(config: ContextConfig) -> Self {
        let initial = CognitiveContext {
            id: 1,
            cycle: 0,
            phase: CognitivePhase::Idle,
            focus: None,
            goals: Vec::new(),
            constraints: Vec::new(),
            environment: EnvironmentState::default(),
            domain_states: BTreeMap::new(),
            timestamp: Timestamp::now(),
            parent: None,
        };

        Self {
            current: initial,
            history: Vec::new(),
            next_id: AtomicU64::new(2),
            next_goal_id: AtomicU64::new(1),
            next_constraint_id: AtomicU64::new(1),
            config,
            stats: ContextStats::default(),
        }
    }

    /// Get current context
    pub fn current(&self) -> &CognitiveContext {
        &self.current
    }

    /// Get mutable current context
    pub fn current_mut(&mut self) -> &mut CognitiveContext {
        &mut self.current
    }

    /// Start new cycle
    pub fn start_cycle(&mut self) {
        self.current.cycle += 1;
        self.current.phase = CognitivePhase::Sensing;
        self.current.timestamp = Timestamp::now();
        self.stats.phase_transitions += 1;
    }

    /// Advance to next phase
    pub fn advance_phase(&mut self) {
        let prev = self.current.phase;
        self.current.phase = prev.next();
        self.stats.phase_transitions += 1;
    }

    /// Set focus
    pub fn set_focus(&mut self, focus: ContextFocus) {
        self.current.focus = Some(focus);
    }

    /// Clear focus
    pub fn clear_focus(&mut self) {
        self.current.focus = None;
    }

    /// Add goal
    pub fn add_goal(&mut self, goal_type: GoalType, description: String, priority: u32) -> u64 {
        let id = self.next_goal_id.fetch_add(1, Ordering::Relaxed);

        let goal = ContextGoal {
            id,
            goal_type,
            description,
            priority,
            progress: 0.0,
            deadline: None,
            depends_on: Vec::new(),
        };

        self.current.goals.push(goal);
        id
    }

    /// Update goal progress
    pub fn update_goal(&mut self, goal_id: u64, progress: f32) {
        if let Some(goal) = self.current.goals.iter_mut().find(|g| g.id == goal_id) {
            goal.progress = progress;
            if progress >= 1.0 {
                self.stats.goals_completed += 1;
            }
        }
    }

    /// Remove goal
    pub fn remove_goal(&mut self, goal_id: u64) {
        self.current.goals.retain(|g| g.id != goal_id);
    }

    /// Add constraint
    pub fn add_constraint(
        &mut self,
        constraint_type: ConstraintType,
        value: ConstraintValue,
        hard: bool,
        source: String,
    ) -> u64 {
        let id = self.next_constraint_id.fetch_add(1, Ordering::Relaxed);

        let constraint = ContextConstraint {
            id,
            constraint_type,
            value,
            hard,
            source,
        };

        self.current.constraints.push(constraint);
        id
    }

    /// Remove constraint
    pub fn remove_constraint(&mut self, constraint_id: u64) {
        self.current.constraints.retain(|c| c.id != constraint_id);
    }

    /// Update environment
    pub fn update_environment(&mut self, env: EnvironmentState) {
        self.current.environment = env;
    }

    /// Update domain state
    pub fn update_domain_state(&mut self, domain_id: DomainId, state: DomainContextState) {
        self.current.domain_states.insert(domain_id, state);
    }

    /// Create checkpoint
    pub fn checkpoint(&mut self) {
        let snapshot = self.current.clone();

        if self.history.len() >= self.config.max_history {
            self.history.remove(0);
        }

        self.history.push(snapshot);
    }

    /// Create nested context
    pub fn push_context(&mut self) -> u64 {
        let parent_id = self.current.id;
        let new_id = self.next_id.fetch_add(1, Ordering::Relaxed);

        // Save current to history
        self.checkpoint();

        // Create new context inheriting from current
        let mut new_context = self.current.clone();
        new_context.id = new_id;
        new_context.parent = Some(parent_id);
        new_context.timestamp = Timestamp::now();

        self.current = new_context;
        self.stats.total_contexts += 1;

        new_id
    }

    /// Pop to parent context
    pub fn pop_context(&mut self) -> bool {
        if let Some(parent_id) = self.current.parent {
            // Find parent in history
            if let Some(pos) = self.history.iter().rposition(|c| c.id == parent_id) {
                let parent = self.history.remove(pos);
                self.current = parent;
                return true;
            }
        }
        false
    }

    /// Get history
    pub fn history(&self) -> &[CognitiveContext] {
        &self.history
    }

    /// Get statistics
    pub fn stats(&self) -> &ContextStats {
        &self.stats
    }
}

// ============================================================================
// CONTEXT SNAPSHOT
// ============================================================================

/// Lightweight snapshot of context
#[derive(Debug, Clone)]
pub struct ContextSnapshot {
    /// Context ID
    pub id: u64,
    /// Cycle
    pub cycle: u64,
    /// Phase
    pub phase: CognitivePhase,
    /// System mode
    pub mode: SystemMode,
    /// Active goals count
    pub goal_count: u32,
    /// Active constraints count
    pub constraint_count: u32,
    /// Timestamp
    pub timestamp: Timestamp,
}

impl From<&CognitiveContext> for ContextSnapshot {
    fn from(ctx: &CognitiveContext) -> Self {
        Self {
            id: ctx.id,
            cycle: ctx.cycle,
            phase: ctx.phase,
            mode: ctx.environment.mode,
            goal_count: ctx.goals.len() as u32,
            constraint_count: ctx.constraints.len() as u32,
            timestamp: ctx.timestamp,
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let config = ContextConfig::default();
        let manager = ContextManager::new(config);

        let ctx = manager.current();
        assert_eq!(ctx.phase, CognitivePhase::Idle);
    }

    #[test]
    fn test_phase_advance() {
        let config = ContextConfig::default();
        let mut manager = ContextManager::new(config);

        manager.start_cycle();
        assert_eq!(manager.current().phase, CognitivePhase::Sensing);

        manager.advance_phase();
        assert_eq!(manager.current().phase, CognitivePhase::Understanding);
    }

    #[test]
    fn test_goals() {
        let config = ContextConfig::default();
        let mut manager = ContextManager::new(config);

        let goal_id = manager.add_goal(GoalType::Stability, "Maintain stability".into(), 100);

        assert!(manager.current().goals.iter().any(|g| g.id == goal_id));

        manager.update_goal(goal_id, 1.0);
        assert_eq!(manager.stats().goals_completed, 1);
    }

    #[test]
    fn test_nested_context() {
        let config = ContextConfig::default();
        let mut manager = ContextManager::new(config);

        let parent_id = manager.current().id;
        let child_id = manager.push_context();

        assert_eq!(manager.current().id, child_id);
        assert_eq!(manager.current().parent, Some(parent_id));

        manager.pop_context();
        assert_eq!(manager.current().id, parent_id);
    }
}
