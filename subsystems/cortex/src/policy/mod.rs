//! # CORTEX Policy Engine
//!
//! The Policy Engine provides a declarative way to define kernel behavior.
//! Instead of hardcoded logic, policies can be:
//!
//! - **Loaded at runtime**: Change behavior without recompilation
//! - **Versioned**: Track policy changes over time
//! - **Composed**: Combine multiple policies with priorities
//! - **Verified**: Check policies for consistency and completeness
//! - **Audited**: Track which policies made which decisions
//!
//! ## Policy Language
//!
//! Policies are defined using a simple, verifiable language:
//!
//! ```text
//! policy memory_pressure {
//!     when memory_usage > 80% {
//!         if priority(process) == low {
//!             action: migrate_to_swap
//!         } else {
//!             action: compress_memory
//!         }
//!     }
//! }
//! ```

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::{DecisionAction, SubsystemId, Timestamp};

// =============================================================================
// POLICY DEFINITION
// =============================================================================

/// Policy identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PolicyId(pub u64);

/// Policy version
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PolicyVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl PolicyVersion {
    pub fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl Default for PolicyVersion {
    fn default() -> Self {
        Self::new(1, 0, 0)
    }
}

/// Policy status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyStatus {
    /// Policy is being drafted
    Draft,

    /// Policy is active
    Active,

    /// Policy is disabled
    Disabled,

    /// Policy is deprecated
    Deprecated,

    /// Policy is archived
    Archived,
}

/// Policy priority (higher = more important)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Priority(pub u8);

impl Priority {
    pub const LOWEST: Priority = Priority(0);
    pub const LOW: Priority = Priority(25);
    pub const NORMAL: Priority = Priority(50);
    pub const HIGH: Priority = Priority(75);
    pub const HIGHEST: Priority = Priority(100);
    pub const CRITICAL: Priority = Priority(255);
}

impl Default for Priority {
    fn default() -> Self {
        Self::NORMAL
    }
}

/// Policy definition
#[derive(Clone)]
pub struct Policy {
    /// Policy ID
    pub id: PolicyId,

    /// Policy name
    pub name: String,

    /// Description
    pub description: String,

    /// Version
    pub version: PolicyVersion,

    /// Status
    pub status: PolicyStatus,

    /// Priority
    pub priority: Priority,

    /// Target subsystem (None = all)
    pub target: Option<SubsystemId>,

    /// Rules
    pub rules: Vec<PolicyRule>,

    /// Created timestamp
    pub created_at: Timestamp,

    /// Modified timestamp
    pub modified_at: Timestamp,

    /// Author
    pub author: String,

    /// Tags
    pub tags: Vec<String>,
}

impl Policy {
    /// Create new policy
    pub fn new(id: PolicyId, name: &str) -> Self {
        Self {
            id,
            name: String::from(name),
            description: String::new(),
            version: PolicyVersion::default(),
            status: PolicyStatus::Draft,
            priority: Priority::default(),
            target: None,
            rules: Vec::new(),
            created_at: 0,
            modified_at: 0,
            author: String::new(),
            tags: Vec::new(),
        }
    }

    /// Add description
    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = String::from(desc);
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Set target subsystem
    pub fn for_subsystem(mut self, subsystem: SubsystemId) -> Self {
        self.target = Some(subsystem);
        self
    }

    /// Add rule
    pub fn with_rule(mut self, rule: PolicyRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Activate policy
    pub fn activate(&mut self, timestamp: Timestamp) {
        self.status = PolicyStatus::Active;
        self.modified_at = timestamp;
    }

    /// Disable policy
    pub fn disable(&mut self, timestamp: Timestamp) {
        self.status = PolicyStatus::Disabled;
        self.modified_at = timestamp;
    }

    /// Is policy active?
    pub fn is_active(&self) -> bool {
        self.status == PolicyStatus::Active
    }
}

// =============================================================================
// POLICY RULE
// =============================================================================

/// Policy rule
#[derive(Clone)]
pub struct PolicyRule {
    /// Rule name
    pub name: String,

    /// Condition
    pub condition: Condition,

    /// Action to take
    pub action: DecisionAction,

    /// Priority within policy
    pub priority: Priority,

    /// Is rule enabled?
    pub enabled: bool,
}

impl PolicyRule {
    /// Create new rule
    pub fn new(name: &str, condition: Condition, action: DecisionAction) -> Self {
        Self {
            name: String::from(name),
            condition,
            action,
            priority: Priority::default(),
            enabled: true,
        }
    }

    /// Set priority
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }
}

// =============================================================================
// CONDITIONS
// =============================================================================

/// Condition for policy evaluation
#[derive(Clone)]
pub enum Condition {
    /// Always true
    Always,

    /// Never true
    Never,

    /// Comparison condition
    Compare(Comparison),

    /// Logical AND
    And(Box<Condition>, Box<Condition>),

    /// Logical OR
    Or(Box<Condition>, Box<Condition>),

    /// Logical NOT
    Not(Box<Condition>),

    /// Check if value is in range
    InRange(String, i64, i64),

    /// Check if value matches pattern
    Matches(String, String),

    /// Custom condition (with callback)
    Custom(String),
}

impl Condition {
    /// Create AND condition
    pub fn and(a: Condition, b: Condition) -> Self {
        Self::And(Box::new(a), Box::new(b))
    }

    /// Create OR condition
    pub fn or(a: Condition, b: Condition) -> Self {
        Self::Or(Box::new(a), Box::new(b))
    }

    /// Create NOT condition
    pub fn not(c: Condition) -> Self {
        Self::Not(Box::new(c))
    }

    /// Evaluate condition against context
    pub fn evaluate(&self, context: &PolicyContext) -> bool {
        match self {
            Self::Always => true,
            Self::Never => false,

            Self::Compare(cmp) => cmp.evaluate(context),

            Self::And(a, b) => a.evaluate(context) && b.evaluate(context),
            Self::Or(a, b) => a.evaluate(context) || b.evaluate(context),
            Self::Not(c) => !c.evaluate(context),

            Self::InRange(var, min, max) => context
                .get_i64(var)
                .map_or(false, |v| v >= *min && v <= *max),

            Self::Matches(var, pattern) => context
                .get_string(var)
                .map_or(false, |v| v.contains(pattern.as_str())),

            Self::Custom(name) => context.get_bool(name).unwrap_or(false),
        }
    }
}

/// Comparison operation
#[derive(Clone)]
pub struct Comparison {
    /// Variable name
    pub variable: String,

    /// Operator
    pub operator: ComparisonOp,

    /// Value to compare against
    pub value: Value,
}

impl Comparison {
    /// Create new comparison
    pub fn new(variable: &str, op: ComparisonOp, value: Value) -> Self {
        Self {
            variable: String::from(variable),
            operator: op,
            value,
        }
    }

    /// Evaluate comparison
    pub fn evaluate(&self, context: &PolicyContext) -> bool {
        match &self.value {
            Value::Int(v) => {
                if let Some(actual) = context.get_i64(&self.variable) {
                    self.operator.compare_i64(actual, *v)
                } else {
                    false
                }
            },

            Value::Float(v) => {
                if let Some(actual) = context.get_f64(&self.variable) {
                    self.operator.compare_f64(actual, *v)
                } else {
                    false
                }
            },

            Value::String(v) => {
                if let Some(actual) = context.get_string(&self.variable) {
                    self.operator.compare_str(&actual, v)
                } else {
                    false
                }
            },

            Value::Bool(v) => {
                if let Some(actual) = context.get_bool(&self.variable) {
                    match self.operator {
                        ComparisonOp::Eq => actual == *v,
                        ComparisonOp::Ne => actual != *v,
                        _ => false,
                    }
                } else {
                    false
                }
            },
        }
    }
}

/// Comparison operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOp {
    /// Equal
    Eq,
    /// Not equal
    Ne,
    /// Less than
    Lt,
    /// Less than or equal
    Le,
    /// Greater than
    Gt,
    /// Greater than or equal
    Ge,
}

impl ComparisonOp {
    fn compare_i64(&self, a: i64, b: i64) -> bool {
        match self {
            Self::Eq => a == b,
            Self::Ne => a != b,
            Self::Lt => a < b,
            Self::Le => a <= b,
            Self::Gt => a > b,
            Self::Ge => a >= b,
        }
    }

    fn compare_f64(&self, a: f64, b: f64) -> bool {
        match self {
            Self::Eq => (a - b).abs() < 1e-10,
            Self::Ne => (a - b).abs() >= 1e-10,
            Self::Lt => a < b,
            Self::Le => a <= b,
            Self::Gt => a > b,
            Self::Ge => a >= b,
        }
    }

    fn compare_str(&self, a: &str, b: &str) -> bool {
        match self {
            Self::Eq => a == b,
            Self::Ne => a != b,
            Self::Lt => a < b,
            Self::Le => a <= b,
            Self::Gt => a > b,
            Self::Ge => a >= b,
        }
    }
}

/// Value type
#[derive(Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
}

// =============================================================================
// POLICY CONTEXT
// =============================================================================

/// Context for policy evaluation
#[derive(Default)]
pub struct PolicyContext {
    /// Integer variables
    ints: BTreeMap<String, i64>,

    /// Float variables
    floats: BTreeMap<String, f64>,

    /// String variables
    strings: BTreeMap<String, String>,

    /// Boolean variables
    bools: BTreeMap<String, bool>,
}

impl PolicyContext {
    /// Create new context
    pub fn new() -> Self {
        Self::default()
    }

    /// Set integer variable
    pub fn set_i64(&mut self, name: &str, value: i64) {
        self.ints.insert(String::from(name), value);
    }

    /// Set float variable
    pub fn set_f64(&mut self, name: &str, value: f64) {
        self.floats.insert(String::from(name), value);
    }

    /// Set string variable
    pub fn set_string(&mut self, name: &str, value: &str) {
        self.strings.insert(String::from(name), String::from(value));
    }

    /// Set boolean variable
    pub fn set_bool(&mut self, name: &str, value: bool) {
        self.bools.insert(String::from(name), value);
    }

    /// Get integer variable
    pub fn get_i64(&self, name: &str) -> Option<i64> {
        self.ints.get(name).copied()
    }

    /// Get float variable
    pub fn get_f64(&self, name: &str) -> Option<f64> {
        self.floats.get(name).copied()
    }

    /// Get string variable
    pub fn get_string(&self, name: &str) -> Option<String> {
        self.strings.get(name).cloned()
    }

    /// Get boolean variable
    pub fn get_bool(&self, name: &str) -> Option<bool> {
        self.bools.get(name).copied()
    }

    /// Clear all variables
    pub fn clear(&mut self) {
        self.ints.clear();
        self.floats.clear();
        self.strings.clear();
        self.bools.clear();
    }
}

// =============================================================================
// POLICY EVALUATION
// =============================================================================

/// Policy evaluation result
#[derive(Clone)]
pub struct PolicyResult {
    /// Policy that matched
    pub policy_id: PolicyId,

    /// Rule that matched
    pub rule_name: String,

    /// Action to take
    pub action: DecisionAction,

    /// Evaluation timestamp
    pub timestamp: Timestamp,

    /// Confidence (based on policy priority)
    pub confidence: f64,
}

/// Policy conflict resolution strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Use highest priority policy
    HighestPriority,

    /// Use most specific policy
    MostSpecific,

    /// Use first match
    FirstMatch,

    /// Use last match
    LastMatch,

    /// Deny on conflict
    DenyOnConflict,
}

impl Default for ConflictResolution {
    fn default() -> Self {
        Self::HighestPriority
    }
}

// =============================================================================
// POLICY ENGINE
// =============================================================================

/// Policy engine configuration
#[derive(Clone)]
pub struct PolicyEngineConfig {
    /// Enable policy engine
    pub enabled: bool,

    /// Conflict resolution strategy
    pub conflict_resolution: ConflictResolution,

    /// Maximum policies
    pub max_policies: usize,

    /// Enable audit logging
    pub audit_enabled: bool,

    /// Maximum audit entries
    pub max_audit_entries: usize,
}

impl Default for PolicyEngineConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            conflict_resolution: ConflictResolution::HighestPriority,
            max_policies: 100,
            audit_enabled: true,
            max_audit_entries: 10000,
        }
    }
}

/// Audit entry
#[derive(Clone)]
pub struct AuditEntry {
    /// Timestamp
    pub timestamp: Timestamp,

    /// Policy ID
    pub policy_id: PolicyId,

    /// Rule name
    pub rule_name: String,

    /// Action taken
    pub action: DecisionAction,

    /// Context snapshot
    pub context_summary: String,
}

/// Policy engine
pub struct PolicyEngine {
    /// Configuration
    config: PolicyEngineConfig,

    /// Registered policies
    policies: BTreeMap<PolicyId, Policy>,

    /// Next policy ID
    next_id: u64,

    /// Audit log
    audit_log: Vec<AuditEntry>,

    /// Evaluation count
    eval_count: u64,

    /// Match count
    match_count: u64,

    /// Conflict count
    conflict_count: u64,
}

impl PolicyEngine {
    /// Create new engine
    pub fn new(config: PolicyEngineConfig) -> Self {
        Self {
            config,
            policies: BTreeMap::new(),
            next_id: 1,
            audit_log: Vec::new(),
            eval_count: 0,
            match_count: 0,
            conflict_count: 0,
        }
    }

    /// Register policy
    pub fn register(&mut self, mut policy: Policy, timestamp: Timestamp) -> PolicyId {
        let id = PolicyId(self.next_id);
        self.next_id += 1;

        policy.id = id;
        policy.created_at = timestamp;
        policy.modified_at = timestamp;

        self.policies.insert(id, policy);
        id
    }

    /// Get policy
    pub fn get(&self, id: PolicyId) -> Option<&Policy> {
        self.policies.get(&id)
    }

    /// Get policy mutably
    pub fn get_mut(&mut self, id: PolicyId) -> Option<&mut Policy> {
        self.policies.get_mut(&id)
    }

    /// Remove policy
    pub fn remove(&mut self, id: PolicyId) -> Option<Policy> {
        self.policies.remove(&id)
    }

    /// Activate policy
    pub fn activate(&mut self, id: PolicyId, timestamp: Timestamp) -> bool {
        if let Some(policy) = self.policies.get_mut(&id) {
            policy.activate(timestamp);
            true
        } else {
            false
        }
    }

    /// Disable policy
    pub fn disable(&mut self, id: PolicyId, timestamp: Timestamp) -> bool {
        if let Some(policy) = self.policies.get_mut(&id) {
            policy.disable(timestamp);
            true
        } else {
            false
        }
    }

    /// Evaluate policies against context
    pub fn evaluate(
        &mut self,
        context: &PolicyContext,
        subsystem: Option<SubsystemId>,
        timestamp: Timestamp,
    ) -> Option<PolicyResult> {
        if !self.config.enabled {
            return None;
        }

        self.eval_count += 1;

        // Find matching policies
        let mut matches: Vec<(PolicyId, &str, &DecisionAction, Priority)> = Vec::new();

        for policy in self.policies.values() {
            if !policy.is_active() {
                continue;
            }

            // Check subsystem filter
            if let Some(target) = policy.target {
                if subsystem != Some(target) {
                    continue;
                }
            }

            // Evaluate rules
            for rule in &policy.rules {
                if rule.enabled && rule.condition.evaluate(context) {
                    matches.push((
                        policy.id,
                        &rule.name,
                        &rule.action,
                        Priority(policy.priority.0 + rule.priority.0 / 2),
                    ));
                }
            }
        }

        if matches.is_empty() {
            return None;
        }

        self.match_count += 1;

        // Resolve conflicts
        let result = if matches.len() > 1 {
            self.conflict_count += 1;
            self.resolve_conflict(&matches)?
        } else {
            let m = &matches[0];
            (m.0, m.1.to_string(), m.2.clone(), m.3)
        };

        // Audit log
        if self.config.audit_enabled {
            self.audit(result.0, &result.1, &result.2, timestamp);
        }

        Some(PolicyResult {
            policy_id: result.0,
            rule_name: result.1,
            action: result.2,
            timestamp,
            confidence: result.3 .0 as f64 / 255.0,
        })
    }

    /// Resolve conflict between matches
    fn resolve_conflict(
        &self,
        matches: &[(PolicyId, &str, &DecisionAction, Priority)],
    ) -> Option<(PolicyId, String, DecisionAction, Priority)> {
        match self.config.conflict_resolution {
            ConflictResolution::HighestPriority => matches
                .iter()
                .max_by_key(|m| m.3)
                .map(|m| (m.0, m.1.to_string(), m.2.clone(), m.3)),

            ConflictResolution::FirstMatch => matches
                .first()
                .map(|m| (m.0, m.1.to_string(), m.2.clone(), m.3)),

            ConflictResolution::LastMatch => matches
                .last()
                .map(|m| (m.0, m.1.to_string(), m.2.clone(), m.3)),

            ConflictResolution::DenyOnConflict => None,

            ConflictResolution::MostSpecific => {
                // For now, use highest priority
                matches
                    .iter()
                    .max_by_key(|m| m.3)
                    .map(|m| (m.0, m.1.to_string(), m.2.clone(), m.3))
            },
        }
    }

    /// Add audit entry
    fn audit(
        &mut self,
        policy_id: PolicyId,
        rule_name: &str,
        action: &DecisionAction,
        timestamp: Timestamp,
    ) {
        if self.audit_log.len() >= self.config.max_audit_entries {
            self.audit_log.remove(0);
        }

        self.audit_log.push(AuditEntry {
            timestamp,
            policy_id,
            rule_name: String::from(rule_name),
            action: action.clone(),
            context_summary: String::new(),
        });
    }

    /// Get audit log
    pub fn audit_log(&self) -> &[AuditEntry] {
        &self.audit_log
    }

    /// Get statistics
    pub fn stats(&self) -> PolicyStats {
        PolicyStats {
            total_policies: self.policies.len(),
            active_policies: self.policies.values().filter(|p| p.is_active()).count(),
            eval_count: self.eval_count,
            match_count: self.match_count,
            conflict_count: self.conflict_count,
            audit_entries: self.audit_log.len(),
        }
    }

    /// List all policies
    pub fn list(&self) -> Vec<&Policy> {
        self.policies.values().collect()
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new(PolicyEngineConfig::default())
    }
}

/// Policy statistics
#[derive(Debug, Clone, Default)]
pub struct PolicyStats {
    /// Total policies
    pub total_policies: usize,

    /// Active policies
    pub active_policies: usize,

    /// Evaluation count
    pub eval_count: u64,

    /// Match count
    pub match_count: u64,

    /// Conflict count
    pub conflict_count: u64,

    /// Audit entries
    pub audit_entries: usize,
}

// =============================================================================
// BUILT-IN POLICIES
// =============================================================================

/// Create default memory management policy
pub fn memory_policy() -> Policy {
    Policy::new(PolicyId(0), "memory_management")
        .with_description("Default memory management policy")
        .with_priority(Priority::HIGH)
        .with_rule(PolicyRule::new(
            "high_memory_pressure",
            Condition::Compare(Comparison::new(
                "memory_usage_percent",
                ComparisonOp::Ge,
                Value::Int(85),
            )),
            DecisionAction::AdjustMemory,
        ))
        .with_rule(
            PolicyRule::new(
                "critical_memory_pressure",
                Condition::Compare(Comparison::new(
                    "memory_usage_percent",
                    ComparisonOp::Ge,
                    Value::Int(95),
                )),
                DecisionAction::IsolateSubsystem(SubsystemId(0)),
            )
            .with_priority(Priority::HIGH),
        )
}

/// Create default security policy
pub fn security_policy() -> Policy {
    Policy::new(PolicyId(0), "security")
        .with_description("Default security policy")
        .with_priority(Priority::CRITICAL)
        .with_rule(
            PolicyRule::new(
                "threat_detected",
                Condition::Compare(Comparison::new(
                    "threat_level",
                    ComparisonOp::Ge,
                    Value::Int(3),
                )),
                DecisionAction::IsolateSubsystem(SubsystemId(0)),
            )
            .with_priority(Priority::CRITICAL),
        )
        .with_rule(PolicyRule::new(
            "anomaly_detected",
            Condition::Compare(Comparison::new(
                "anomaly_score",
                ComparisonOp::Ge,
                Value::Float(0.8),
            )),
            DecisionAction::NoOp, // Log and observe
        ))
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_condition_always() {
        let ctx = PolicyContext::new();
        assert!(Condition::Always.evaluate(&ctx));
        assert!(!Condition::Never.evaluate(&ctx));
    }

    #[test]
    fn test_condition_compare() {
        let mut ctx = PolicyContext::new();
        ctx.set_i64("memory", 80);

        let cond = Condition::Compare(Comparison::new("memory", ComparisonOp::Ge, Value::Int(75)));

        assert!(cond.evaluate(&ctx));

        ctx.set_i64("memory", 50);
        assert!(!cond.evaluate(&ctx));
    }

    #[test]
    fn test_condition_and_or() {
        let mut ctx = PolicyContext::new();
        ctx.set_i64("a", 10);
        ctx.set_i64("b", 20);

        let cond_a = Condition::Compare(Comparison::new("a", ComparisonOp::Gt, Value::Int(5)));
        let cond_b = Condition::Compare(Comparison::new("b", ComparisonOp::Gt, Value::Int(15)));

        let and_cond = Condition::and(cond_a.clone(), cond_b.clone());
        let or_cond = Condition::or(
            cond_a.clone(),
            Condition::Compare(Comparison::new("b", ComparisonOp::Lt, Value::Int(10))),
        );

        assert!(and_cond.evaluate(&ctx));
        assert!(or_cond.evaluate(&ctx));
    }

    #[test]
    fn test_policy_engine() {
        let mut engine = PolicyEngine::default();

        let policy = Policy::new(PolicyId(0), "test").with_rule(PolicyRule::new(
            "test_rule",
            Condition::Compare(Comparison::new("value", ComparisonOp::Gt, Value::Int(50))),
            DecisionAction::NoOp,
        ));

        let id = engine.register(policy, 1000);
        engine.activate(id, 1000);

        let mut ctx = PolicyContext::new();
        ctx.set_i64("value", 75);

        let result = engine.evaluate(&ctx, None, 2000);
        assert!(result.is_some());
        assert_eq!(result.unwrap().rule_name, "test_rule");
    }
}
