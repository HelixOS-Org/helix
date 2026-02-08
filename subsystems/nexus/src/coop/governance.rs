//! # Coop Governance Engine
//!
//! Policy-driven governance for cooperative resource management:
//! - Policy rule evaluation engine
//! - Resource governance policies
//! - Compliance tracking and enforcement
//! - Policy versioning and rollback
//! - Multi-tenant governance boundaries

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// POLICY TYPES
// ============================================================================

/// Policy action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyAction {
    /// Allow the operation
    Allow,
    /// Deny the operation
    Deny,
    /// Audit (allow but log)
    Audit,
    /// Throttle the operation
    Throttle,
    /// Redirect to alternative
    Redirect,
}

/// Policy scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyScope {
    /// Single process
    Process,
    /// Process group
    Group,
    /// Tenant/namespace
    Tenant,
    /// System-wide
    System,
}

/// Policy priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PolicyPriority {
    /// Low priority (can be overridden)
    Low,
    /// Normal priority
    Normal,
    /// High priority
    High,
    /// Mandatory (cannot be overridden)
    Mandatory,
}

/// Condition operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionOp {
    /// Equal
    Eq,
    /// Not equal
    Ne,
    /// Greater than
    Gt,
    /// Less than
    Lt,
    /// Greater than or equal
    Gte,
    /// Less than or equal
    Lte,
}

// ============================================================================
// POLICY RULE
// ============================================================================

/// Policy condition
#[derive(Debug, Clone)]
pub struct PolicyCondition {
    /// Field to check (FNV-1a hash of field name)
    pub field_hash: u64,
    /// Operator
    pub op: ConditionOp,
    /// Threshold value
    pub threshold: f64,
}

impl PolicyCondition {
    pub fn new(field_name: &str, op: ConditionOp, threshold: f64) -> Self {
        Self {
            field_hash: Self::hash_field(field_name),
            op,
            threshold,
        }
    }

    fn hash_field(name: &str) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in name.as_bytes() {
            hash ^= *b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    /// Evaluate condition
    pub fn evaluate(&self, value: f64) -> bool {
        match self.op {
            ConditionOp::Eq => libm::fabs(value - self.threshold) < 0.0001,
            ConditionOp::Ne => libm::fabs(value - self.threshold) >= 0.0001,
            ConditionOp::Gt => value > self.threshold,
            ConditionOp::Lt => value < self.threshold,
            ConditionOp::Gte => value >= self.threshold,
            ConditionOp::Lte => value <= self.threshold,
        }
    }
}

/// Policy rule
#[derive(Debug, Clone)]
pub struct PolicyRule {
    /// Rule ID
    pub rule_id: u64,
    /// Rule name hash
    pub name_hash: u64,
    /// Conditions (all must match)
    pub conditions: Vec<PolicyCondition>,
    /// Action if matched
    pub action: PolicyAction,
    /// Scope
    pub scope: PolicyScope,
    /// Priority
    pub priority: PolicyPriority,
    /// Version
    pub version: u32,
    /// Active
    pub active: bool,
    /// Hit count
    pub hit_count: u64,
}

impl PolicyRule {
    pub fn new(rule_id: u64, action: PolicyAction) -> Self {
        Self {
            rule_id,
            name_hash: 0,
            conditions: Vec::new(),
            action,
            scope: PolicyScope::System,
            priority: PolicyPriority::Normal,
            version: 1,
            active: true,
            hit_count: 0,
        }
    }

    /// Add condition
    pub fn add_condition(&mut self, condition: PolicyCondition) {
        self.conditions.push(condition);
    }

    /// Evaluate rule against values
    pub fn evaluate(&mut self, values: &BTreeMap<u64, f64>) -> Option<PolicyAction> {
        if !self.active {
            return None;
        }
        for cond in &self.conditions {
            let value = values.get(&cond.field_hash).copied().unwrap_or(0.0);
            if !cond.evaluate(value) {
                return None;
            }
        }
        self.hit_count += 1;
        Some(self.action)
    }
}

// ============================================================================
// POLICY SET
// ============================================================================

/// Policy set (collection of rules)
#[derive(Debug)]
pub struct PolicySet {
    /// Set ID
    pub set_id: u64,
    /// Rules (sorted by priority)
    pub rules: Vec<PolicyRule>,
    /// Version
    pub version: u32,
    /// Created (ns)
    pub created_ns: u64,
}

impl PolicySet {
    pub fn new(set_id: u64, now: u64) -> Self {
        Self {
            set_id,
            rules: Vec::new(),
            version: 1,
            created_ns: now,
        }
    }

    /// Add rule
    pub fn add_rule(&mut self, rule: PolicyRule) {
        self.rules.push(rule);
        // Sort by priority (highest first)
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Evaluate all rules (first match wins)
    pub fn evaluate(&mut self, values: &BTreeMap<u64, f64>) -> PolicyAction {
        for rule in &mut self.rules {
            if let Some(action) = rule.evaluate(values) {
                return action;
            }
        }
        PolicyAction::Allow // Default: allow
    }
}

// ============================================================================
// TENANT BOUNDARY
// ============================================================================

/// Tenant governance boundary
#[derive(Debug)]
pub struct TenantBoundary {
    /// Tenant ID
    pub tenant_id: u64,
    /// Member PIDs
    pub members: Vec<u64>,
    /// Policy set
    pub policy_set_id: u64,
    /// Resource limits
    pub resource_limits: BTreeMap<u64, f64>,
    /// Current usage
    pub resource_usage: BTreeMap<u64, f64>,
}

impl TenantBoundary {
    pub fn new(tenant_id: u64, policy_set_id: u64) -> Self {
        Self {
            tenant_id,
            members: Vec::new(),
            policy_set_id,
            resource_limits: BTreeMap::new(),
            resource_usage: BTreeMap::new(),
        }
    }

    /// Add member
    pub fn add_member(&mut self, pid: u64) {
        if !self.members.contains(&pid) {
            self.members.push(pid);
        }
    }

    /// Remove member
    pub fn remove_member(&mut self, pid: u64) {
        self.members.retain(|&p| p != pid);
    }

    /// Set limit
    pub fn set_limit(&mut self, resource_hash: u64, limit: f64) {
        self.resource_limits.insert(resource_hash, limit);
    }

    /// Update usage
    pub fn update_usage(&mut self, resource_hash: u64, usage: f64) {
        self.resource_usage.insert(resource_hash, usage);
    }

    /// Check within limits
    pub fn within_limits(&self) -> bool {
        for (&resource, &limit) in &self.resource_limits {
            let usage = self.resource_usage.get(&resource).copied().unwrap_or(0.0);
            if usage > limit {
                return false;
            }
        }
        true
    }

    /// Utilization per resource
    pub fn utilization(&self, resource_hash: u64) -> f64 {
        let usage = self.resource_usage.get(&resource_hash).copied().unwrap_or(0.0);
        let limit = self.resource_limits.get(&resource_hash).copied().unwrap_or(1.0);
        if limit > 0.0 { usage / limit } else { 0.0 }
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Governance stats
#[derive(Debug, Clone, Default)]
pub struct CoopGovernanceStats {
    /// Policy sets
    pub policy_sets: usize,
    /// Total rules
    pub total_rules: usize,
    /// Tenants
    pub tenants: usize,
    /// Total evaluations
    pub total_evaluations: u64,
    /// Deny actions
    pub deny_count: u64,
}

/// Coop governance engine
pub struct CoopGovernanceEngine {
    /// Policy sets
    policies: BTreeMap<u64, PolicySet>,
    /// Tenants
    tenants: BTreeMap<u64, TenantBoundary>,
    /// Process -> tenant mapping
    process_tenant: BTreeMap<u64, u64>,
    /// Stats
    stats: CoopGovernanceStats,
    /// Next policy set ID
    next_set_id: u64,
}

impl CoopGovernanceEngine {
    pub fn new() -> Self {
        Self {
            policies: BTreeMap::new(),
            tenants: BTreeMap::new(),
            process_tenant: BTreeMap::new(),
            stats: CoopGovernanceStats::default(),
            next_set_id: 1,
        }
    }

    /// Create policy set
    pub fn create_policy_set(&mut self, now: u64) -> u64 {
        let id = self.next_set_id;
        self.next_set_id += 1;
        self.policies.insert(id, PolicySet::new(id, now));
        self.update_stats();
        id
    }

    /// Add rule to policy set
    pub fn add_rule(&mut self, set_id: u64, rule: PolicyRule) {
        if let Some(set) = self.policies.get_mut(&set_id) {
            set.add_rule(rule);
        }
        self.update_stats();
    }

    /// Create tenant
    pub fn create_tenant(&mut self, tenant_id: u64, policy_set_id: u64) {
        self.tenants.insert(tenant_id, TenantBoundary::new(tenant_id, policy_set_id));
        self.update_stats();
    }

    /// Add process to tenant
    pub fn assign_tenant(&mut self, pid: u64, tenant_id: u64) {
        if let Some(tenant) = self.tenants.get_mut(&tenant_id) {
            tenant.add_member(pid);
            self.process_tenant.insert(pid, tenant_id);
        }
    }

    /// Evaluate action for process
    pub fn evaluate(&mut self, pid: u64, values: &BTreeMap<u64, f64>) -> PolicyAction {
        self.stats.total_evaluations += 1;

        // Find tenant-specific policy
        if let Some(&tenant_id) = self.process_tenant.get(&pid) {
            if let Some(tenant) = self.tenants.get(&tenant_id) {
                let set_id = tenant.policy_set_id;
                if let Some(policy_set) = self.policies.get_mut(&set_id) {
                    let action = policy_set.evaluate(values);
                    if matches!(action, PolicyAction::Deny) {
                        self.stats.deny_count += 1;
                    }
                    return action;
                }
            }
        }

        // Default: evaluate system-wide policy set (ID 0 if exists)
        if let Some(system_set) = self.policies.get_mut(&0) {
            let action = system_set.evaluate(values);
            if matches!(action, PolicyAction::Deny) {
                self.stats.deny_count += 1;
            }
            return action;
        }

        PolicyAction::Allow
    }

    /// Remove process
    pub fn remove_process(&mut self, pid: u64) {
        if let Some(tenant_id) = self.process_tenant.remove(&pid) {
            if let Some(tenant) = self.tenants.get_mut(&tenant_id) {
                tenant.remove_member(pid);
            }
        }
    }

    fn update_stats(&mut self) {
        self.stats.policy_sets = self.policies.len();
        self.stats.total_rules = self.policies.values().map(|p| p.rules.len()).sum();
        self.stats.tenants = self.tenants.len();
    }

    /// Stats
    pub fn stats(&self) -> &CoopGovernanceStats {
        &self.stats
    }
}
