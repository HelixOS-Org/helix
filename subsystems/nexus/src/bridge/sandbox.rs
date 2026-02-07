//! # Bridge Sandbox Engine
//!
//! Syscall sandboxing and filtering:
//! - Syscall allowlists/denylists
//! - Argument validation rules
//! - Seccomp-like filtering
//! - Sandbox profiles
//! - Violation logging and actions
//! - Nested sandbox support

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// FILTER RULES
// ============================================================================

/// Filter action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterAction {
    /// Allow the syscall
    Allow,
    /// Deny with errno
    Deny(i32),
    /// Kill the process
    Kill,
    /// Log and allow
    LogAllow,
    /// Log and deny
    LogDeny(i32),
    /// Trap (notify handler)
    Trap,
    /// Return specific value
    ReturnValue(u64),
}

/// Argument comparison operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgOp {
    /// Equal
    Eq,
    /// Not equal
    Ne,
    /// Less than
    Lt,
    /// Less or equal
    Le,
    /// Greater than
    Gt,
    /// Greater or equal
    Ge,
    /// Bitmask set (arg & mask == mask)
    MaskSet,
    /// Bitmask clear (arg & mask == 0)
    MaskClear,
}

/// Argument filter
#[derive(Debug, Clone)]
pub struct ArgFilter {
    /// Argument index (0-5)
    pub arg_index: u8,
    /// Comparison operator
    pub op: ArgOp,
    /// Comparison value
    pub value: u64,
}

impl ArgFilter {
    pub fn new(arg_index: u8, op: ArgOp, value: u64) -> Self {
        Self {
            arg_index,
            op,
            value,
        }
    }

    /// Evaluate against actual argument
    pub fn matches(&self, arg_value: u64) -> bool {
        match self.op {
            ArgOp::Eq => arg_value == self.value,
            ArgOp::Ne => arg_value != self.value,
            ArgOp::Lt => arg_value < self.value,
            ArgOp::Le => arg_value <= self.value,
            ArgOp::Gt => arg_value > self.value,
            ArgOp::Ge => arg_value >= self.value,
            ArgOp::MaskSet => (arg_value & self.value) == self.value,
            ArgOp::MaskClear => (arg_value & self.value) == 0,
        }
    }
}

/// A filter rule
#[derive(Debug, Clone)]
pub struct SandboxRule {
    /// Rule ID
    pub id: u64,
    /// Syscall number (u32::MAX = wildcard)
    pub syscall_nr: u32,
    /// Argument filters (all must match)
    pub arg_filters: Vec<ArgFilter>,
    /// Action
    pub action: FilterAction,
    /// Priority (higher = evaluated first)
    pub priority: u32,
    /// Hit count
    pub hits: u64,
}

impl SandboxRule {
    pub fn new(id: u64, syscall_nr: u32, action: FilterAction) -> Self {
        Self {
            id,
            syscall_nr,
            arg_filters: Vec::new(),
            action,
            priority: 0,
            hits: 0,
        }
    }

    pub fn with_arg_filter(mut self, filter: ArgFilter) -> Self {
        self.arg_filters.push(filter);
        self
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Evaluate against syscall
    pub fn matches(&self, syscall_nr: u32, args: &[u64]) -> bool {
        if self.syscall_nr != u32::MAX && self.syscall_nr != syscall_nr {
            return false;
        }
        for filter in &self.arg_filters {
            let arg_value = args.get(filter.arg_index as usize).copied().unwrap_or(0);
            if !filter.matches(arg_value) {
                return false;
            }
        }
        true
    }
}

// ============================================================================
// SANDBOX PROFILE
// ============================================================================

/// Profile strictness
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SandboxStrictness {
    /// Minimal restrictions
    Permissive,
    /// Standard restrictions
    Standard,
    /// Strict (most syscalls denied)
    Strict,
    /// Paranoid (minimal allowlist)
    Paranoid,
}

/// Sandbox profile
#[derive(Debug, Clone)]
pub struct SandboxProfile {
    /// Profile ID
    pub id: u64,
    /// Name
    pub name: u64,
    /// Strictness
    pub strictness: SandboxStrictness,
    /// Default action for unmatched syscalls
    pub default_action: FilterAction,
    /// Rules (sorted by priority, high first)
    pub rules: Vec<SandboxRule>,
    /// Inherited profile ID (0 = none)
    pub parent: u64,
}

impl SandboxProfile {
    pub fn new(id: u64, strictness: SandboxStrictness) -> Self {
        let default_action = match strictness {
            SandboxStrictness::Permissive => FilterAction::Allow,
            SandboxStrictness::Standard => FilterAction::LogAllow,
            SandboxStrictness::Strict => FilterAction::Deny(1), // EPERM
            SandboxStrictness::Paranoid => FilterAction::Kill,
        };

        Self {
            id,
            name: id,
            strictness,
            default_action,
            rules: Vec::new(),
            parent: 0,
        }
    }

    /// Add rule (maintains priority ordering)
    pub fn add_rule(&mut self, rule: SandboxRule) {
        let pos = self.rules.partition_point(|r| r.priority > rule.priority);
        self.rules.insert(pos, rule);
    }

    /// Evaluate syscall
    pub fn evaluate(&mut self, syscall_nr: u32, args: &[u64]) -> FilterAction {
        for rule in &mut self.rules {
            if rule.matches(syscall_nr, args) {
                rule.hits += 1;
                return rule.action;
            }
        }
        self.default_action
    }

    /// Rule count
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

// ============================================================================
// SANDBOX VIOLATION
// ============================================================================

/// Violation record
#[derive(Debug, Clone)]
pub struct SandboxViolation {
    /// Process
    pub pid: u64,
    /// Syscall number
    pub syscall_nr: u32,
    /// Arguments
    pub args: Vec<u64>,
    /// Action taken
    pub action: FilterAction,
    /// Matched rule ID (0 = default)
    pub rule_id: u64,
    /// Timestamp
    pub timestamp: u64,
}

// ============================================================================
// SANDBOX INSTANCE
// ============================================================================

/// Active sandbox
#[derive(Debug)]
pub struct SandboxInstance {
    /// Process ID
    pub pid: u64,
    /// Profile
    pub profile: SandboxProfile,
    /// Violation count
    pub violations: u64,
    /// Allow count
    pub allows: u64,
    /// Deny count
    pub denies: u64,
    /// Created at
    pub created_at: u64,
}

impl SandboxInstance {
    pub fn new(pid: u64, profile: SandboxProfile, now: u64) -> Self {
        Self {
            pid,
            profile,
            violations: 0,
            allows: 0,
            denies: 0,
            created_at: now,
        }
    }

    /// Check syscall
    pub fn check(&mut self, syscall_nr: u32, args: &[u64]) -> FilterAction {
        let action = self.profile.evaluate(syscall_nr, args);
        match action {
            FilterAction::Allow | FilterAction::LogAllow | FilterAction::ReturnValue(_) => {
                self.allows += 1;
            },
            FilterAction::Deny(_) | FilterAction::LogDeny(_) | FilterAction::Kill => {
                self.denies += 1;
                self.violations += 1;
            },
            FilterAction::Trap => {
                self.violations += 1;
            },
        }
        action
    }

    /// Deny rate
    pub fn deny_rate(&self) -> f64 {
        let total = self.allows + self.denies;
        if total == 0 {
            return 0.0;
        }
        self.denies as f64 / total as f64
    }
}

// ============================================================================
// SANDBOX MANAGER
// ============================================================================

/// Sandbox manager stats
#[derive(Debug, Clone, Default)]
pub struct SandboxManagerStats {
    /// Active sandboxes
    pub active_sandboxes: usize,
    /// Total violations
    pub total_violations: u64,
    /// Total syscalls checked
    pub total_checked: u64,
    /// Deny rate
    pub deny_rate: f64,
}

/// Bridge sandbox manager
pub struct BridgeSandboxManager {
    /// Active instances (pid → instance)
    instances: BTreeMap<u64, SandboxInstance>,
    /// Profile templates
    profiles: BTreeMap<u64, SandboxProfile>,
    /// Violation log
    violation_log: Vec<SandboxViolation>,
    /// Max log size
    max_log: usize,
    /// Stats
    stats: SandboxManagerStats,
}

impl BridgeSandboxManager {
    pub fn new() -> Self {
        Self {
            instances: BTreeMap::new(),
            profiles: BTreeMap::new(),
            violation_log: Vec::new(),
            max_log: 1000,
            stats: SandboxManagerStats::default(),
        }
    }

    /// Register profile template
    pub fn register_profile(&mut self, profile: SandboxProfile) {
        self.profiles.insert(profile.id, profile);
    }

    /// Attach sandbox to process
    pub fn attach(&mut self, pid: u64, profile: SandboxProfile, now: u64) {
        self.instances
            .insert(pid, SandboxInstance::new(pid, profile, now));
        self.stats.active_sandboxes = self.instances.len();
    }

    /// Detach sandbox
    pub fn detach(&mut self, pid: u64) {
        self.instances.remove(&pid);
        self.stats.active_sandboxes = self.instances.len();
    }

    /// Check syscall against sandbox
    pub fn check(&mut self, pid: u64, syscall_nr: u32, args: &[u64], now: u64) -> FilterAction {
        self.stats.total_checked += 1;

        let Some(instance) = self.instances.get_mut(&pid) else {
            return FilterAction::Allow;
        };

        let action = instance.check(syscall_nr, args);

        // Log violations
        match action {
            FilterAction::Deny(_)
            | FilterAction::LogDeny(_)
            | FilterAction::Kill
            | FilterAction::Trap => {
                self.stats.total_violations += 1;
                let violation = SandboxViolation {
                    pid,
                    syscall_nr,
                    args: args.to_vec(),
                    action,
                    rule_id: 0,
                    timestamp: now,
                };
                self.violation_log.push(violation);
                if self.violation_log.len() > self.max_log {
                    self.violation_log.remove(0);
                }
            },
            _ => {},
        }

        if self.stats.total_checked > 0 {
            self.stats.deny_rate =
                self.stats.total_violations as f64 / self.stats.total_checked as f64;
        }

        action
    }

    /// Get violations
    pub fn violations(&self) -> &[SandboxViolation] {
        &self.violation_log
    }

    /// Stats
    pub fn stats(&self) -> &SandboxManagerStats {
        &self.stats
    }
}
