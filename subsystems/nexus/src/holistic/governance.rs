//! # Holistic Governance Engine
//!
//! System-wide resource governance policies:
//! - Policy definition and enforcement
//! - Resource access control
//! - Compliance monitoring
//! - Policy violation handling
//! - Dynamic policy adaptation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// GOVERNANCE TYPES
// ============================================================================

/// Governance scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GovernanceScope {
    /// System-wide
    System,
    /// Per-subsystem
    Subsystem,
    /// Per-cgroup
    CgroupLevel,
    /// Per-process
    Process,
    /// Per-thread
    Thread,
}

/// Governed resource
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GovernedResource {
    /// CPU
    Cpu,
    /// Memory
    Memory,
    /// I/O bandwidth
    Io,
    /// Network bandwidth
    Network,
    /// GPU
    Gpu,
    /// File descriptors
    FileDescriptors,
    /// Threads
    Threads,
    /// Power
    Power,
}

/// Policy enforcement mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnforcementMode {
    /// Permissive (log only)
    Permissive,
    /// Advisory (warn and allow)
    Advisory,
    /// Enforcing (block violations)
    Enforcing,
    /// Strict (block + penalize)
    Strict,
}

/// Policy state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovernancePolicyState {
    /// Draft
    Draft,
    /// Active
    Active,
    /// Suspended
    Suspended,
    /// Retired
    Retired,
}

// ============================================================================
// POLICY DEFINITION
// ============================================================================

/// Resource limit in a policy
#[derive(Debug, Clone, Copy)]
pub struct ResourceLimit {
    /// Resource type
    pub resource: GovernedResource,
    /// Soft limit
    pub soft_limit: f64,
    /// Hard limit
    pub hard_limit: f64,
    /// Burst limit
    pub burst_limit: f64,
    /// Burst duration (ns)
    pub burst_duration_ns: u64,
}

impl ResourceLimit {
    pub fn new(resource: GovernedResource, soft: f64, hard: f64) -> Self {
        Self {
            resource,
            soft_limit: soft,
            hard_limit: hard,
            burst_limit: hard * 1.2,
            burst_duration_ns: 1_000_000_000,
        }
    }

    /// Check if value violates soft
    pub fn soft_violated(&self, value: f64) -> bool {
        value > self.soft_limit
    }

    /// Check if value violates hard
    pub fn hard_violated(&self, value: f64) -> bool {
        value > self.hard_limit
    }

    /// Utilization fraction of soft
    pub fn soft_utilization(&self, value: f64) -> f64 {
        if self.soft_limit > 0.0 {
            value / self.soft_limit
        } else {
            0.0
        }
    }
}

/// Governance policy
#[derive(Debug, Clone)]
pub struct GovernancePolicy {
    /// Policy id
    pub id: u64,
    /// Scope
    pub scope: GovernanceScope,
    /// State
    pub state: GovernancePolicyState,
    /// Enforcement mode
    pub mode: EnforcementMode,
    /// Priority
    pub priority: u32,
    /// Resource limits
    pub limits: Vec<ResourceLimit>,
    /// Applicable targets (process ids or group ids)
    pub targets: Vec<u64>,
    /// Created at
    pub created_at: u64,
    /// Last modified
    pub modified_at: u64,
}

impl GovernancePolicy {
    pub fn new(id: u64, scope: GovernanceScope, mode: EnforcementMode) -> Self {
        Self {
            id,
            scope,
            state: GovernancePolicyState::Draft,
            mode,
            priority: 100,
            limits: Vec::new(),
            targets: Vec::new(),
            created_at: 0,
            modified_at: 0,
        }
    }

    /// Add limit
    pub fn add_limit(&mut self, limit: ResourceLimit) {
        self.limits.push(limit);
    }

    /// Add target
    pub fn add_target(&mut self, target: u64) {
        if !self.targets.contains(&target) {
            self.targets.push(target);
        }
    }

    /// Activate
    pub fn activate(&mut self) {
        if matches!(self.state, GovernancePolicyState::Draft | GovernancePolicyState::Suspended) {
            self.state = GovernancePolicyState::Active;
        }
    }

    /// Suspend
    pub fn suspend(&mut self) {
        if matches!(self.state, GovernancePolicyState::Active) {
            self.state = GovernancePolicyState::Suspended;
        }
    }

    /// Retire
    pub fn retire(&mut self) {
        self.state = GovernancePolicyState::Retired;
    }

    /// Find limit for resource
    pub fn limit_for(&self, resource: GovernedResource) -> Option<&ResourceLimit> {
        self.limits.iter().find(|l| l.resource == resource)
    }

    /// Check if policy applies to target
    pub fn applies_to(&self, target: u64) -> bool {
        self.targets.is_empty() || self.targets.contains(&target)
    }
}

// ============================================================================
// VIOLATION
// ============================================================================

/// Violation severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationSeverity {
    /// Warning
    Warning,
    /// Minor
    Minor,
    /// Major
    Major,
    /// Critical
    Critical,
}

/// Governance violation
#[derive(Debug, Clone)]
pub struct GovernanceViolation {
    /// Violation id
    pub id: u64,
    /// Policy that was violated
    pub policy_id: u64,
    /// Target that violated
    pub target: u64,
    /// Resource
    pub resource: GovernedResource,
    /// Current value
    pub current_value: f64,
    /// Limit value
    pub limit_value: f64,
    /// Severity
    pub severity: ViolationSeverity,
    /// Timestamp
    pub timestamp: u64,
    /// Was action taken
    pub action_taken: bool,
}

impl GovernanceViolation {
    /// Over-limit fraction
    pub fn overage(&self) -> f64 {
        if self.limit_value > 0.0 {
            (self.current_value - self.limit_value) / self.limit_value
        } else {
            0.0
        }
    }
}

// ============================================================================
// COMPLIANCE RECORD
// ============================================================================

/// Compliance status for a target
#[derive(Debug, Clone)]
pub struct ComplianceRecord {
    /// Target
    pub target: u64,
    /// Total checks
    pub total_checks: u64,
    /// Violations
    pub violation_count: u64,
    /// Last violation time
    pub last_violation: u64,
    /// Score (0.0 - 1.0)
    pub score: f64,
}

impl ComplianceRecord {
    pub fn new(target: u64) -> Self {
        Self {
            target,
            total_checks: 0,
            violation_count: 0,
            last_violation: 0,
            score: 1.0,
        }
    }

    /// Record a check
    pub fn record_check(&mut self, violated: bool, now: u64) {
        self.total_checks += 1;
        if violated {
            self.violation_count += 1;
            self.last_violation = now;
        }
        if self.total_checks > 0 {
            self.score =
                1.0 - (self.violation_count as f64 / self.total_checks as f64);
        }
    }

    /// Compliance rate
    pub fn compliance_rate(&self) -> f64 {
        self.score
    }
}

// ============================================================================
// GOVERNANCE ENGINE
// ============================================================================

/// Governance stats
#[derive(Debug, Clone, Default)]
pub struct HolisticGovernanceStats {
    /// Active policies
    pub active_policies: usize,
    /// Total violations
    pub total_violations: u64,
    /// Targets monitored
    pub targets_monitored: usize,
    /// Average compliance
    pub avg_compliance: f64,
    /// Critical violations
    pub critical_violations: u64,
}

/// Holistic governance engine
pub struct HolisticGovernanceEngine {
    /// Policies
    policies: BTreeMap<u64, GovernancePolicy>,
    /// Violations
    violations: Vec<GovernanceViolation>,
    /// Compliance records
    compliance: BTreeMap<u64, ComplianceRecord>,
    /// Next violation id
    next_violation_id: u64,
    /// Max violations to keep
    max_violations: usize,
    /// Stats
    stats: HolisticGovernanceStats,
}

impl HolisticGovernanceEngine {
    pub fn new() -> Self {
        Self {
            policies: BTreeMap::new(),
            violations: Vec::new(),
            compliance: BTreeMap::new(),
            next_violation_id: 1,
            max_violations: 4096,
            stats: HolisticGovernanceStats::default(),
        }
    }

    /// Register policy
    pub fn register_policy(&mut self, policy: GovernancePolicy) {
        self.policies.insert(policy.id, policy);
        self.update_stats();
    }

    /// Activate policy
    pub fn activate_policy(&mut self, policy_id: u64) -> bool {
        if let Some(p) = self.policies.get_mut(&policy_id) {
            p.activate();
            self.update_stats();
            true
        } else {
            false
        }
    }

    /// Suspend policy
    pub fn suspend_policy(&mut self, policy_id: u64) -> bool {
        if let Some(p) = self.policies.get_mut(&policy_id) {
            p.suspend();
            self.update_stats();
            true
        } else {
            false
        }
    }

    /// Check compliance of target against all policies
    pub fn check_compliance(
        &mut self,
        target: u64,
        resource: GovernedResource,
        value: f64,
        now: u64,
    ) -> Vec<GovernanceViolation> {
        let mut new_violations = Vec::new();

        // Collect policies that apply
        let policies: Vec<(u64, EnforcementMode, f64, f64)> = self
            .policies
            .values()
            .filter(|p| {
                matches!(p.state, GovernancePolicyState::Active) && p.applies_to(target)
            })
            .filter_map(|p| {
                p.limit_for(resource).map(|l| (p.id, p.mode, l.soft_limit, l.hard_limit))
            })
            .collect();

        for (pid, mode, soft, hard) in policies {
            let violated_hard = value > hard;
            let violated_soft = value > soft;

            let compliance = self
                .compliance
                .entry(target)
                .or_insert_with(|| ComplianceRecord::new(target));
            compliance.record_check(violated_hard || violated_soft, now);

            if violated_hard {
                let severity = match mode {
                    EnforcementMode::Strict => ViolationSeverity::Critical,
                    EnforcementMode::Enforcing => ViolationSeverity::Major,
                    _ => ViolationSeverity::Minor,
                };

                let v = GovernanceViolation {
                    id: self.next_violation_id,
                    policy_id: pid,
                    target,
                    resource,
                    current_value: value,
                    limit_value: hard,
                    severity,
                    timestamp: now,
                    action_taken: matches!(
                        mode,
                        EnforcementMode::Enforcing | EnforcementMode::Strict
                    ),
                };
                self.next_violation_id += 1;
                new_violations.push(v);
            } else if violated_soft {
                let v = GovernanceViolation {
                    id: self.next_violation_id,
                    policy_id: pid,
                    target,
                    resource,
                    current_value: value,
                    limit_value: soft,
                    severity: ViolationSeverity::Warning,
                    timestamp: now,
                    action_taken: false,
                };
                self.next_violation_id += 1;
                new_violations.push(v);
            }
        }

        for v in &new_violations {
            if matches!(v.severity, ViolationSeverity::Critical) {
                self.stats.critical_violations += 1;
            }
            self.stats.total_violations += 1;
        }
        self.violations.extend(new_violations.clone());
        if self.violations.len() > self.max_violations {
            let drain = self.violations.len() - self.max_violations;
            self.violations.drain(..drain);
        }

        self.stats.targets_monitored = self.compliance.len();
        self.update_avg_compliance();

        new_violations
    }

    /// Get violations for target
    pub fn violations_for(&self, target: u64) -> Vec<&GovernanceViolation> {
        self.violations.iter().filter(|v| v.target == target).collect()
    }

    /// Compliance of target
    pub fn compliance_of(&self, target: u64) -> Option<&ComplianceRecord> {
        self.compliance.get(&target)
    }

    /// Worst compliance targets
    pub fn worst_compliance(&self, count: usize) -> Vec<(u64, f64)> {
        let mut records: Vec<(u64, f64)> = self
            .compliance
            .values()
            .map(|r| (r.target, r.score))
            .collect();
        records.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));
        records.truncate(count);
        records
    }

    fn update_stats(&mut self) {
        self.stats.active_policies = self
            .policies
            .values()
            .filter(|p| matches!(p.state, GovernancePolicyState::Active))
            .count();
    }

    fn update_avg_compliance(&mut self) {
        if self.compliance.is_empty() {
            self.stats.avg_compliance = 1.0;
        } else {
            let sum: f64 = self.compliance.values().map(|r| r.score).sum();
            self.stats.avg_compliance = sum / self.compliance.len() as f64;
        }
    }

    /// Stats
    pub fn stats(&self) -> &HolisticGovernanceStats {
        &self.stats
    }
}
