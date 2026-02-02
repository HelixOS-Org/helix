//! # Safety Checker
//!
//! The Safety Checker ensures all AI decisions comply with safety constraints
//! and invariants to prevent harmful or destabilizing actions.
//!
//! ## Safety Principles
//!
//! 1. **Non-maleficence**: Actions must not harm the system or users
//! 2. **Reversibility**: Prefer reversible actions over irreversible ones
//! 3. **Proportionality**: Response magnitude should match threat severity
//! 4. **Transparency**: All decisions must be explainable
//! 5. **Human Override**: Users can always override AI decisions
//!
//! ## Architecture
//!
//! ```text
//!    AI Decision ──────►┌──────────────────────────────────────────┐
//!                       │            Safety Checker                │
//!                       │                                          │
//!                       │  ┌────────────────────────────────┐     │
//!                       │  │      Invariant Checker         │     │
//!                       │  │   - Memory bounds             │     │
//!                       │  │   - Resource limits           │     │
//!                       │  │   - Timing constraints        │     │
//!                       │  └──────────────┬─────────────────┘     │
//!                       │                 │                       │
//!                       │                 ▼                       │
//!                       │  ┌────────────────────────────────┐     │
//!                       │  │      Constraint Validator      │     │
//!                       │  │   - Action blacklist          │     │
//!                       │  │   - Rate limits               │     │
//!                       │  │   - Confidence thresholds     │     │
//!                       │  └──────────────┬─────────────────┘     │
//!                       │                 │                       │
//!                       │                 ▼                       │
//!                       │  ┌────────────────────────────────┐     │
//!                       │  │      Risk Assessor             │     │
//!                       │  │   - Impact analysis           │     │
//!                       │  │   - Rollback feasibility      │     │
//!                       │  │   - Cascade effects           │     │
//!                       │  └──────────────┬─────────────────┘     │
//!                       │                 │                       │
//!                       │                 ▼                       │
//!                       │  ┌────────────────────────────────┐     │
//!                       │  │      Decision Gate             │────────► Allow/Deny/Modify
//!                       │  └────────────────────────────────┘     │
//!                       │                                          │
//!                       └──────────────────────────────────────────┘
//! ```

use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::{format, vec};
use core::sync::atomic::{AtomicU64, Ordering};

use spin::{Mutex, RwLock};

use crate::core::{AiAction, AiDecision, AiPriority, SafetyLevel};

// =============================================================================
// Safety Invariants
// =============================================================================

/// A safety invariant that must never be violated
#[derive(Debug, Clone)]
pub struct Invariant {
    /// Invariant ID
    pub id: u64,
    /// Human-readable name
    pub name: String,
    /// Description of what the invariant ensures
    pub description: String,
    /// Invariant category
    pub category: InvariantCategory,
    /// Severity if violated
    pub severity: ViolationSeverity,
    /// Check function (represented as type)
    pub check_type: InvariantCheckType,
    /// Associated parameters
    pub parameters: BTreeMap<String, InvariantParameter>,
}

/// Categories of invariants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvariantCategory {
    /// Memory-related invariants
    Memory,
    /// CPU/scheduling invariants
    Cpu,
    /// I/O invariants
    Io,
    /// Security invariants
    Security,
    /// Process invariants
    Process,
    /// Module invariants
    Module,
    /// General system invariants
    System,
}

/// Severity of invariant violations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ViolationSeverity {
    /// Advisory - log but allow
    Advisory,
    /// Warning - log and reduce confidence
    Warning,
    /// Error - block the action
    Error,
    /// Critical - block and enter safe mode
    Critical,
    /// Fatal - immediate halt
    Fatal,
}

/// Type of invariant check
#[derive(Debug, Clone)]
pub enum InvariantCheckType {
    /// Value must be above minimum
    MinValue(f64),
    /// Value must be below maximum
    MaxValue(f64),
    /// Value must be in range
    Range(f64, f64),
    /// Must match pattern
    Pattern(String),
    /// Must not match pattern
    NotPattern(String),
    /// Must be in allowed list
    AllowList(Vec<String>),
    /// Must not be in blocked list
    BlockList(Vec<String>),
    /// Custom check (identified by name)
    Custom(String),
}

/// Parameter for invariant checks
#[derive(Debug, Clone)]
pub enum InvariantParameter {
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),
    List(Vec<String>),
}

// =============================================================================
// Safety Constraints
// =============================================================================

/// A constraint on AI actions
#[derive(Debug, Clone)]
pub struct SafetyConstraint {
    /// Constraint ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Description
    pub description: String,
    /// Constraint type
    pub constraint_type: ConstraintType,
    /// When this constraint applies
    pub applies_to: ConstraintScope,
    /// Actions when violated
    pub on_violation: ViolationAction,
    /// Is enabled
    pub enabled: bool,
}

/// Types of constraints
#[derive(Debug, Clone)]
pub enum ConstraintType {
    /// Minimum confidence required
    MinConfidence(f32),
    /// Maximum actions per time window
    RateLimit { max_count: u32, window_ms: u64 },
    /// Require human approval
    RequireApproval,
    /// Require specific conditions
    Condition(String),
    /// Delay before action
    DelayMs(u64),
    /// Cooling period after action
    CooldownMs(u64),
    /// Action must be reversible
    MustBeReversible,
    /// Cannot affect critical processes
    ProtectCritical,
    /// Cannot exceed resource limit
    ResourceLimit { resource: String, max_value: u64 },
}

/// Scope of constraint application
#[derive(Debug, Clone)]
pub enum ConstraintScope {
    /// Applies to all actions
    All,
    /// Applies to specific action types
    ActionTypes(Vec<u32>),
    /// Applies based on priority
    PriorityLevel(AiPriority),
    /// Applies based on confidence level
    ConfidenceBelow(f32),
    /// Applies based on safety level
    SafetyLevel(SafetyLevel),
}

/// Action to take on constraint violation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationAction {
    /// Log and allow
    Allow,
    /// Log and warn user
    Warn,
    /// Block the action
    Block,
    /// Modify the action (reduce scope)
    Modify,
    /// Escalate for review
    Escalate,
}

// =============================================================================
// Safety Violations
// =============================================================================

/// A recorded safety violation
#[derive(Debug, Clone)]
pub struct SafetyViolation {
    /// Violation ID
    pub id: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Which invariant or constraint was violated
    pub violated: ViolatedEntity,
    /// The action that caused the violation
    pub action: AiAction,
    /// Severity
    pub severity: ViolationSeverity,
    /// Description of the violation
    pub description: String,
    /// Action taken
    pub action_taken: ViolationAction,
    /// Was it resolved
    pub resolved: bool,
}

/// What was violated
#[derive(Debug, Clone)]
pub enum ViolatedEntity {
    Invariant(u64, String),
    Constraint(u64, String),
}

// =============================================================================
// Risk Assessment
// =============================================================================

/// Risk assessment for an action
#[derive(Debug, Clone)]
pub struct RiskAssessment {
    /// Overall risk level (0.0 - 1.0)
    pub risk_level: f32,
    /// Risk category
    pub risk_category: RiskCategory,
    /// Impact assessment
    pub impact: ImpactAssessment,
    /// Is the action reversible
    pub reversible: bool,
    /// Estimated time to reverse (microseconds)
    pub reversal_time_us: Option<u64>,
    /// Potential cascade effects
    pub cascade_effects: Vec<CascadeEffect>,
    /// Mitigation recommendations
    pub mitigations: Vec<String>,
}

/// Risk categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskCategory {
    /// No risk
    None,
    /// Low risk - routine actions
    Low,
    /// Medium risk - may cause issues
    Medium,
    /// High risk - likely to cause issues
    High,
    /// Critical risk - may cause system failure
    Critical,
}

impl RiskCategory {
    /// Get numeric value
    pub fn value(&self) -> f32 {
        match self {
            RiskCategory::None => 0.0,
            RiskCategory::Low => 0.25,
            RiskCategory::Medium => 0.5,
            RiskCategory::High => 0.75,
            RiskCategory::Critical => 1.0,
        }
    }

    /// From numeric value
    pub fn from_value(value: f32) -> Self {
        match value {
            v if v < 0.1 => RiskCategory::None,
            v if v < 0.35 => RiskCategory::Low,
            v if v < 0.6 => RiskCategory::Medium,
            v if v < 0.85 => RiskCategory::High,
            _ => RiskCategory::Critical,
        }
    }
}

/// Impact assessment
#[derive(Debug, Clone)]
pub struct ImpactAssessment {
    /// Number of processes potentially affected
    pub processes_affected: u32,
    /// Memory impact (bytes)
    pub memory_impact_bytes: i64,
    /// CPU impact (percentage points)
    pub cpu_impact_percent: i8,
    /// I/O impact
    pub io_impact: IoImpact,
    /// User-visible effects
    pub user_visible: bool,
    /// Data at risk
    pub data_at_risk: bool,
}

/// I/O impact level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoImpact {
    None,
    Read,
    Write,
    Both,
    Blocking,
}

/// Potential cascade effect
#[derive(Debug, Clone)]
pub struct CascadeEffect {
    /// Effect description
    pub description: String,
    /// Probability (0.0 - 1.0)
    pub probability: f32,
    /// Severity if occurs
    pub severity: ViolationSeverity,
}

// =============================================================================
// Safety Checker Engine
// =============================================================================

/// The Safety Checker engine
pub struct SafetyChecker {
    /// Current safety level
    safety_level: RwLock<SafetyLevel>,

    /// Registered invariants
    invariants: RwLock<Vec<Invariant>>,

    /// Registered constraints
    constraints: RwLock<Vec<SafetyConstraint>>,

    /// Recent violations
    violations: Mutex<VecDeque<SafetyViolation>>,

    /// Action rate tracking
    action_rates: Mutex<BTreeMap<u32, VecDeque<u64>>>,

    /// Violation counter
    violation_counter: AtomicU64,

    /// Statistics
    stats: SafetyStats,
}

struct SafetyStats {
    checks_performed: AtomicU64,
    actions_allowed: AtomicU64,
    actions_blocked: AtomicU64,
    actions_modified: AtomicU64,
    invariant_violations: AtomicU64,
    constraint_violations: AtomicU64,
    escalations: AtomicU64,
}

impl Default for SafetyStats {
    fn default() -> Self {
        Self {
            checks_performed: AtomicU64::new(0),
            actions_allowed: AtomicU64::new(0),
            actions_blocked: AtomicU64::new(0),
            actions_modified: AtomicU64::new(0),
            invariant_violations: AtomicU64::new(0),
            constraint_violations: AtomicU64::new(0),
            escalations: AtomicU64::new(0),
        }
    }
}

impl SafetyChecker {
    /// Maximum violations to keep
    const MAX_VIOLATIONS: usize = 1000;

    /// Maximum rate history
    const MAX_RATE_HISTORY: usize = 100;

    /// Create a new Safety Checker
    pub fn new(safety_level: SafetyLevel) -> Self {
        let checker = Self {
            safety_level: RwLock::new(safety_level),
            invariants: RwLock::new(Vec::new()),
            constraints: RwLock::new(Vec::new()),
            violations: Mutex::new(VecDeque::with_capacity(Self::MAX_VIOLATIONS)),
            action_rates: Mutex::new(BTreeMap::new()),
            violation_counter: AtomicU64::new(1),
            stats: SafetyStats::default(),
        };

        checker.register_default_invariants();
        checker.register_default_constraints();
        checker
    }

    /// Register default invariants
    fn register_default_invariants(&self) {
        let mut invariants = self.invariants.write();

        // Memory invariants
        invariants.push(Invariant {
            id: 1,
            name: "memory_bounds".to_string(),
            description: "Memory operations must stay within allocated bounds".to_string(),
            category: InvariantCategory::Memory,
            severity: ViolationSeverity::Critical,
            check_type: InvariantCheckType::Custom("memory_bounds_check".to_string()),
            parameters: BTreeMap::new(),
        });

        invariants.push(Invariant {
            id: 2,
            name: "memory_limit".to_string(),
            description: "Total memory usage must not exceed system limits".to_string(),
            category: InvariantCategory::Memory,
            severity: ViolationSeverity::Error,
            check_type: InvariantCheckType::MaxValue(95.0), // 95% max usage
            parameters: BTreeMap::new(),
        });

        // CPU invariants
        invariants.push(Invariant {
            id: 3,
            name: "scheduler_fairness".to_string(),
            description: "Scheduler must maintain minimum fairness".to_string(),
            category: InvariantCategory::Cpu,
            severity: ViolationSeverity::Warning,
            check_type: InvariantCheckType::MinValue(0.1), // 10% minimum share
            parameters: BTreeMap::new(),
        });

        // Security invariants
        invariants.push(Invariant {
            id: 4,
            name: "no_privilege_escalation".to_string(),
            description: "Actions cannot grant elevated privileges".to_string(),
            category: InvariantCategory::Security,
            severity: ViolationSeverity::Critical,
            check_type: InvariantCheckType::Custom("privilege_check".to_string()),
            parameters: BTreeMap::new(),
        });

        invariants.push(Invariant {
            id: 5,
            name: "critical_process_protection".to_string(),
            description: "Critical system processes cannot be terminated".to_string(),
            category: InvariantCategory::Process,
            severity: ViolationSeverity::Fatal,
            check_type: InvariantCheckType::BlockList(vec![
                "init".to_string(),
                "kernel".to_string(),
                "scheduler".to_string(),
            ]),
            parameters: BTreeMap::new(),
        });
    }

    /// Register default constraints
    fn register_default_constraints(&self) {
        let mut constraints = self.constraints.write();

        // Confidence constraint
        constraints.push(SafetyConstraint {
            id: 1,
            name: "minimum_confidence".to_string(),
            description: "Require minimum confidence for actions".to_string(),
            constraint_type: ConstraintType::MinConfidence(0.6),
            applies_to: ConstraintScope::All,
            on_violation: ViolationAction::Block,
            enabled: true,
        });

        // Rate limit constraint
        constraints.push(SafetyConstraint {
            id: 2,
            name: "action_rate_limit".to_string(),
            description: "Limit rate of AI actions".to_string(),
            constraint_type: ConstraintType::RateLimit {
                max_count: 100,
                window_ms: 1000,
            },
            applies_to: ConstraintScope::All,
            on_violation: ViolationAction::Block,
            enabled: true,
        });

        // Reversibility constraint for high-risk actions
        constraints.push(SafetyConstraint {
            id: 3,
            name: "require_reversibility".to_string(),
            description: "High-risk actions must be reversible".to_string(),
            constraint_type: ConstraintType::MustBeReversible,
            applies_to: ConstraintScope::ConfidenceBelow(0.8),
            on_violation: ViolationAction::Block,
            enabled: true,
        });

        // Critical process protection
        constraints.push(SafetyConstraint {
            id: 4,
            name: "protect_critical".to_string(),
            description: "Cannot affect critical system processes".to_string(),
            constraint_type: ConstraintType::ProtectCritical,
            applies_to: ConstraintScope::All,
            on_violation: ViolationAction::Block,
            enabled: true,
        });

        // Cooldown constraint
        constraints.push(SafetyConstraint {
            id: 5,
            name: "action_cooldown".to_string(),
            description: "Minimum time between similar actions".to_string(),
            constraint_type: ConstraintType::CooldownMs(100),
            applies_to: ConstraintScope::All,
            on_violation: ViolationAction::Block,
            enabled: true,
        });
    }

    /// Check if a decision is safe to execute
    pub fn check(&self, decision: &AiDecision) -> SafetyCheckResult {
        self.stats.checks_performed.fetch_add(1, Ordering::Relaxed);

        let mut result = SafetyCheckResult {
            allowed: true,
            modified_action: None,
            violations: Vec::new(),
            risk_assessment: self.assess_risk(&decision.action),
            required_approval: false,
            required_delay_ms: 0,
        };

        let safety_level = *self.safety_level.read();

        // Check invariants
        for violation in self.check_invariants(decision) {
            result.violations.push(violation.clone());
            self.record_violation(violation);

            if result.allowed {
                result.allowed = false;
                self.stats.actions_blocked.fetch_add(1, Ordering::Relaxed);
            }
        }

        // Check constraints
        for (violation, action) in self.check_constraints(decision, safety_level) {
            result.violations.push(violation.clone());
            self.record_violation(violation);

            match action {
                ViolationAction::Block => {
                    if result.allowed {
                        result.allowed = false;
                        self.stats.actions_blocked.fetch_add(1, Ordering::Relaxed);
                    }
                },
                ViolationAction::Modify => {
                    result.modified_action = self.modify_action(&decision.action);
                    self.stats.actions_modified.fetch_add(1, Ordering::Relaxed);
                },
                ViolationAction::Escalate => {
                    result.required_approval = true;
                    self.stats.escalations.fetch_add(1, Ordering::Relaxed);
                },
                ViolationAction::Warn => {
                    // Allow but log
                },
                ViolationAction::Allow => {},
            }
        }

        // Check risk level
        if result.allowed && result.risk_assessment.risk_category >= RiskCategory::High {
            match safety_level {
                SafetyLevel::Paranoid => {
                    result.allowed = false;
                    self.stats.actions_blocked.fetch_add(1, Ordering::Relaxed);
                },
                SafetyLevel::Cautious => {
                    result.required_approval = true;
                },
                _ => {},
            }
        }

        // Apply safety level modifiers
        result.required_delay_ms = self.get_required_delay(decision, safety_level);

        if result.allowed && result.violations.is_empty() {
            self.stats.actions_allowed.fetch_add(1, Ordering::Relaxed);
            self.record_action(&decision.action);
        }

        result
    }

    /// Check invariants
    fn check_invariants(&self, decision: &AiDecision) -> Vec<SafetyViolation> {
        let mut violations = Vec::new();
        let invariants = self.invariants.read();

        for invariant in invariants.iter() {
            if let Some(violation) = self.check_single_invariant(invariant, decision) {
                violations.push(violation);
                self.stats
                    .invariant_violations
                    .fetch_add(1, Ordering::Relaxed);
            }
        }

        violations
    }

    /// Check a single invariant
    fn check_single_invariant(
        &self,
        invariant: &Invariant,
        decision: &AiDecision,
    ) -> Option<SafetyViolation> {
        let violated = match (&invariant.check_type, &decision.action) {
            // Check if action terminates critical process
            (InvariantCheckType::BlockList(blocked), AiAction::TerminateProcess { pid }) => {
                // In real implementation, would map PID to process name
                false // Placeholder
            },

            // Check if action targets blocked process
            (
                InvariantCheckType::BlockList(blocked),
                AiAction::RestartModule { module_name, .. },
            ) => blocked.iter().any(|b| module_name.contains(b)),

            // Check privilege escalation
            (
                InvariantCheckType::Custom(check),
                AiAction::AdjustProcessPriority { new_priority, .. },
            ) => {
                if check == "privilege_check" {
                    *new_priority < -10 // Very high priority is suspicious
                } else {
                    false
                }
            },

            _ => false,
        };

        if violated {
            Some(SafetyViolation {
                id: self.violation_counter.fetch_add(1, Ordering::SeqCst),
                timestamp: 0,
                violated: ViolatedEntity::Invariant(invariant.id, invariant.name.clone()),
                action: decision.action.clone(),
                severity: invariant.severity,
                description: format!("Invariant '{}' violated", invariant.name),
                action_taken: ViolationAction::Block,
                resolved: false,
            })
        } else {
            None
        }
    }

    /// Check constraints
    fn check_constraints(
        &self,
        decision: &AiDecision,
        safety_level: SafetyLevel,
    ) -> Vec<(SafetyViolation, ViolationAction)> {
        let mut violations = Vec::new();
        let constraints = self.constraints.read();

        for constraint in constraints.iter() {
            if !constraint.enabled {
                continue;
            }

            if !self.constraint_applies(constraint, decision, safety_level) {
                continue;
            }

            if let Some(violation) = self.check_single_constraint(constraint, decision) {
                violations.push((violation, constraint.on_violation));
                self.stats
                    .constraint_violations
                    .fetch_add(1, Ordering::Relaxed);
            }
        }

        violations
    }

    /// Check if constraint applies to this decision
    fn constraint_applies(
        &self,
        constraint: &SafetyConstraint,
        decision: &AiDecision,
        safety_level: SafetyLevel,
    ) -> bool {
        match &constraint.applies_to {
            ConstraintScope::All => true,
            ConstraintScope::ActionTypes(types) => {
                types.contains(&self.action_type_id(&decision.action))
            },
            ConstraintScope::PriorityLevel(p) => decision.priority == *p,
            ConstraintScope::ConfidenceBelow(c) => decision.confidence.value() < *c,
            ConstraintScope::SafetyLevel(s) => safety_level == *s,
        }
    }

    /// Get action type ID
    fn action_type_id(&self, action: &AiAction) -> u32 {
        match action {
            AiAction::NoOp => 0,
            AiAction::TuneScheduler { .. } => 1,
            AiAction::TuneAllocator { .. } => 2,
            AiAction::RestartModule { .. } => 3,
            AiAction::BlockProcess { .. } => 4,
            AiAction::TerminateProcess { .. } => 5,
            AiAction::OffloadToGpu { .. } => 6,
            AiAction::OffloadToNpu { .. } => 7,
            AiAction::ApplyPatch { .. } => 8,
            AiAction::SetPowerProfile { .. } => 9,
            AiAction::AdjustProcessPriority { .. } => 10,
            AiAction::PreallocateResources { .. } => 11,
            AiAction::MigrateProcess { .. } => 12,
            AiAction::ForceGarbageCollection => 13,
            AiAction::SuspendIdleProcesses { .. } => 14,
            _ => 255,
        }
    }

    /// Check a single constraint
    fn check_single_constraint(
        &self,
        constraint: &SafetyConstraint,
        decision: &AiDecision,
    ) -> Option<SafetyViolation> {
        let violated = match &constraint.constraint_type {
            ConstraintType::MinConfidence(min) => decision.confidence.value() < *min,

            ConstraintType::RateLimit {
                max_count,
                window_ms,
            } => {
                let action_type = self.action_type_id(&decision.action);
                self.check_rate_limit(action_type, *max_count, *window_ms)
            },

            ConstraintType::MustBeReversible => decision.rollback.is_none(),

            ConstraintType::ProtectCritical => self.would_affect_critical(&decision.action),

            _ => false,
        };

        if violated {
            Some(SafetyViolation {
                id: self.violation_counter.fetch_add(1, Ordering::SeqCst),
                timestamp: 0,
                violated: ViolatedEntity::Constraint(constraint.id, constraint.name.clone()),
                action: decision.action.clone(),
                severity: ViolationSeverity::Warning,
                description: format!("Constraint '{}' violated", constraint.name),
                action_taken: constraint.on_violation,
                resolved: false,
            })
        } else {
            None
        }
    }

    /// Check rate limit
    fn check_rate_limit(&self, action_type: u32, max_count: u32, window_ms: u64) -> bool {
        let mut rates = self.action_rates.lock();
        let timestamps = rates.entry(action_type).or_default();

        // Count actions in window
        // Note: In real implementation, would use actual time
        let current_time = 0u64; // Placeholder
        let window_start = current_time.saturating_sub(window_ms * 1000);

        let count = timestamps.iter().filter(|&&t| t >= window_start).count();

        count as u32 >= max_count
    }

    /// Check if action would affect critical processes
    fn would_affect_critical(&self, action: &AiAction) -> bool {
        match action {
            AiAction::TerminateProcess { pid } => {
                // Would check if PID is critical
                *pid < 10 // Placeholder: low PIDs are critical
            },
            AiAction::RestartModule { module_name, .. } => {
                // Critical modules
                matches!(
                    module_name.as_str(),
                    "scheduler" | "allocator" | "interrupt_handler" | "paging"
                )
            },
            _ => false,
        }
    }

    /// Record an action for rate limiting
    fn record_action(&self, action: &AiAction) {
        let action_type = self.action_type_id(action);
        let mut rates = self.action_rates.lock();
        let timestamps = rates.entry(action_type).or_default();

        if timestamps.len() >= Self::MAX_RATE_HISTORY {
            timestamps.pop_front();
        }
        timestamps.push_back(0); // Would use actual timestamp
    }

    /// Record a violation
    fn record_violation(&self, violation: SafetyViolation) {
        let mut violations = self.violations.lock();
        if violations.len() >= Self::MAX_VIOLATIONS {
            violations.pop_front();
        }
        violations.push_back(violation);
    }

    /// Modify an action to make it safer
    fn modify_action(&self, action: &AiAction) -> Option<AiAction> {
        match action {
            AiAction::TerminateProcess { pid } => {
                // Convert to graceful shutdown instead
                Some(AiAction::SuspendIdleProcesses {
                    threshold_seconds: 0,
                })
            },

            AiAction::TuneScheduler {
                granularity_ns,
                preemption,
            } => {
                // Reduce the magnitude of changes
                Some(AiAction::TuneScheduler {
                    granularity_ns: (*granularity_ns).max(1_000_000), // At least 1ms
                    preemption: *preemption,
                })
            },

            AiAction::AdjustProcessPriority {
                pid,
                old_priority,
                new_priority,
            } => {
                // Limit priority change magnitude
                let max_change = 5;
                let limited_new = if *new_priority < *old_priority {
                    (*old_priority - max_change).max(*new_priority)
                } else {
                    (*old_priority + max_change).min(*new_priority)
                };
                Some(AiAction::AdjustProcessPriority {
                    pid: *pid,
                    old_priority: *old_priority,
                    new_priority: limited_new,
                })
            },

            _ => None,
        }
    }

    /// Assess risk of an action
    fn assess_risk(&self, action: &AiAction) -> RiskAssessment {
        let (risk_level, reversible) = match action {
            AiAction::NoOp => (0.0, true),
            AiAction::TuneScheduler { .. } => (0.3, true),
            AiAction::TuneAllocator { .. } => (0.3, true),
            AiAction::RestartModule { .. } => (0.5, true),
            AiAction::BlockProcess { .. } => (0.4, true),
            AiAction::TerminateProcess { .. } => (0.8, false),
            AiAction::ApplyPatch { .. } => (0.7, true),
            AiAction::SetPowerProfile { .. } => (0.2, true),
            AiAction::AdjustProcessPriority { .. } => (0.3, true),
            AiAction::ForceGarbageCollection => (0.2, false),
            _ => (0.5, false),
        };

        RiskAssessment {
            risk_level,
            risk_category: RiskCategory::from_value(risk_level),
            impact: ImpactAssessment {
                processes_affected: 1,
                memory_impact_bytes: 0,
                cpu_impact_percent: 5,
                io_impact: IoImpact::None,
                user_visible: false,
                data_at_risk: false,
            },
            reversible,
            reversal_time_us: if reversible { Some(1000) } else { None },
            cascade_effects: Vec::new(),
            mitigations: Vec::new(),
        }
    }

    /// Get required delay based on safety level
    fn get_required_delay(&self, decision: &AiDecision, safety_level: SafetyLevel) -> u64 {
        let base_delay = match safety_level {
            SafetyLevel::Relaxed => 0,
            SafetyLevel::Standard => 10,
            SafetyLevel::Cautious => 100,
            SafetyLevel::Paranoid => 1000,
        };

        // High priority actions get less delay
        let priority_factor = match decision.priority {
            AiPriority::Emergency => 0.0,
            AiPriority::Critical => 0.1,
            AiPriority::High => 0.5,
            AiPriority::Normal => 1.0,
            AiPriority::Low => 2.0,
            AiPriority::Background => 5.0,
        };

        (base_delay as f64 * priority_factor) as u64
    }

    /// Set safety level
    pub fn set_safety_level(&self, level: SafetyLevel) {
        *self.safety_level.write() = level;
    }

    /// Get current safety level
    pub fn safety_level(&self) -> SafetyLevel {
        *self.safety_level.read()
    }

    /// Register a custom invariant
    pub fn register_invariant(&self, invariant: Invariant) {
        self.invariants.write().push(invariant);
    }

    /// Register a custom constraint
    pub fn register_constraint(&self, constraint: SafetyConstraint) {
        self.constraints.write().push(constraint);
    }

    /// Enable/disable a constraint
    pub fn set_constraint_enabled(&self, id: u64, enabled: bool) {
        let mut constraints = self.constraints.write();
        if let Some(constraint) = constraints.iter_mut().find(|c| c.id == id) {
            constraint.enabled = enabled;
        }
    }

    /// Get recent violations
    pub fn recent_violations(&self, count: usize) -> Vec<SafetyViolation> {
        self.violations
            .lock()
            .iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    /// Get statistics
    pub fn statistics(&self) -> SafetyStatistics {
        SafetyStatistics {
            safety_level: *self.safety_level.read(),
            invariants_count: self.invariants.read().len(),
            constraints_count: self.constraints.read().len(),
            checks_performed: self.stats.checks_performed.load(Ordering::Relaxed),
            actions_allowed: self.stats.actions_allowed.load(Ordering::Relaxed),
            actions_blocked: self.stats.actions_blocked.load(Ordering::Relaxed),
            actions_modified: self.stats.actions_modified.load(Ordering::Relaxed),
            invariant_violations: self.stats.invariant_violations.load(Ordering::Relaxed),
            constraint_violations: self.stats.constraint_violations.load(Ordering::Relaxed),
            escalations: self.stats.escalations.load(Ordering::Relaxed),
            recent_violations: self.violations.lock().len(),
        }
    }

    /// Clear violation history
    pub fn clear_violations(&self) {
        self.violations.lock().clear();
    }
}

/// Result of a safety check
#[derive(Debug, Clone)]
pub struct SafetyCheckResult {
    /// Whether action is allowed
    pub allowed: bool,
    /// Modified version of action (if applicable)
    pub modified_action: Option<AiAction>,
    /// List of violations found
    pub violations: Vec<SafetyViolation>,
    /// Risk assessment
    pub risk_assessment: RiskAssessment,
    /// Requires human approval
    pub required_approval: bool,
    /// Required delay before execution (ms)
    pub required_delay_ms: u64,
}

/// Public statistics
#[derive(Debug, Clone)]
pub struct SafetyStatistics {
    pub safety_level: SafetyLevel,
    pub invariants_count: usize,
    pub constraints_count: usize,
    pub checks_performed: u64,
    pub actions_allowed: u64,
    pub actions_blocked: u64,
    pub actions_modified: u64,
    pub invariant_violations: u64,
    pub constraint_violations: u64,
    pub escalations: u64,
    pub recent_violations: usize,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::DecisionId;

    fn make_decision(action: AiAction, confidence: f32) -> AiDecision {
        AiDecision {
            id: DecisionId::new(),
            timestamp: 0,
            action,
            confidence: Confidence::new(confidence),
            priority: AiPriority::Normal,
            reasoning: vec!["Test decision".to_string()],
            expected_outcome: "Test outcome".to_string(),
            rollback: None,
            context: Default::default(),
        }
    }

    #[test]
    fn test_safety_checker_creation() {
        let checker = SafetyChecker::new(SafetyLevel::Standard);

        let stats = checker.statistics();
        assert!(stats.invariants_count > 0);
        assert!(stats.constraints_count > 0);
    }

    #[test]
    fn test_low_confidence_blocked() {
        let checker = SafetyChecker::new(SafetyLevel::Standard);

        let decision = make_decision(AiAction::NoOp, 0.3);
        let result = checker.check(&decision);

        assert!(!result.allowed);
    }

    #[test]
    fn test_safe_action_allowed() {
        let checker = SafetyChecker::new(SafetyLevel::Relaxed);

        let decision = make_decision(AiAction::NoOp, 0.9);
        let result = checker.check(&decision);

        assert!(result.allowed);
    }

    #[test]
    fn test_risk_assessment() {
        let checker = SafetyChecker::new(SafetyLevel::Standard);

        // Low risk action
        let noop_risk = checker.assess_risk(&AiAction::NoOp);
        assert_eq!(noop_risk.risk_category, RiskCategory::None);

        // High risk action
        let terminate_risk = checker.assess_risk(&AiAction::TerminateProcess { pid: 123 });
        assert!(terminate_risk.risk_level > 0.7);
        assert!(!terminate_risk.reversible);
    }

    #[test]
    fn test_safety_level_affects_delay() {
        let decision = make_decision(AiAction::NoOp, 0.9);

        let relaxed = SafetyChecker::new(SafetyLevel::Relaxed);
        let paranoid = SafetyChecker::new(SafetyLevel::Paranoid);

        let relaxed_result = relaxed.check(&decision);
        let paranoid_result = paranoid.check(&decision);

        assert!(relaxed_result.required_delay_ms < paranoid_result.required_delay_ms);
    }

    #[test]
    fn test_critical_process_protection() {
        let checker = SafetyChecker::new(SafetyLevel::Standard);

        let decision = make_decision(
            AiAction::RestartModule {
                module_id: 1,
                module_name: "scheduler".to_string(),
            },
            0.95,
        );

        let result = checker.check(&decision);
        // Should be blocked due to critical module
        assert!(!result.allowed);
    }

    #[test]
    fn test_action_modification() {
        let checker = SafetyChecker::new(SafetyLevel::Standard);

        // Extreme priority change
        let modified = checker.modify_action(&AiAction::AdjustProcessPriority {
            pid: 100,
            old_priority: 0,
            new_priority: -15,
        });

        assert!(modified.is_some());
        if let Some(AiAction::AdjustProcessPriority { new_priority, .. }) = modified {
            assert!(new_priority > -15); // Should be limited
        }
    }

    #[test]
    fn test_violation_recording() {
        let checker = SafetyChecker::new(SafetyLevel::Standard);

        // Make a blocked action
        let decision = make_decision(AiAction::NoOp, 0.3);
        checker.check(&decision);

        let violations = checker.recent_violations(10);
        assert!(!violations.is_empty());
    }
}
