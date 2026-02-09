//! # Bridge Isolation Manager
//!
//! Syscall namespace and isolation management:
//! - Syscall namespace filtering
//! - Seccomp-like policy enforcement
//! - Syscall allowlists/denylists
//! - Permission inheritance
//! - Audit logging for denied calls

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

// ============================================================================
// ISOLATION TYPES
// ============================================================================

/// Filter action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterAction {
    /// Allow syscall
    Allow,
    /// Deny with errno
    Deny,
    /// Log and allow
    LogAllow,
    /// Log and deny
    LogDeny,
    /// Trap (signal)
    Trap,
    /// Kill process
    Kill,
}

/// Filter match type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMatch {
    /// Exact syscall number
    Exact,
    /// Range of syscalls
    Range,
    /// All syscalls
    All,
}

/// Argument comparison
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgComparison {
    /// Equal
    Equal,
    /// Not equal
    NotEqual,
    /// Less than
    LessThan,
    /// Greater than
    GreaterThan,
    /// Masked equal (arg & mask == value)
    MaskedEqual,
}

// ============================================================================
// FILTER RULE
// ============================================================================

/// Argument filter
#[derive(Debug, Clone)]
pub struct ArgFilter {
    /// Argument index (0-5)
    pub arg_index: u8,
    /// Comparison type
    pub comparison: ArgComparison,
    /// Value to compare against
    pub value: u64,
    /// Mask (for MaskedEqual)
    pub mask: u64,
}

impl ArgFilter {
    pub fn new(arg_index: u8, comparison: ArgComparison, value: u64) -> Self {
        Self {
            arg_index,
            comparison,
            value,
            mask: u64::MAX,
        }
    }

    /// Check argument
    #[inline]
    pub fn matches(&self, arg_value: u64) -> bool {
        match self.comparison {
            ArgComparison::Equal => arg_value == self.value,
            ArgComparison::NotEqual => arg_value != self.value,
            ArgComparison::LessThan => arg_value < self.value,
            ArgComparison::GreaterThan => arg_value > self.value,
            ArgComparison::MaskedEqual => (arg_value & self.mask) == self.value,
        }
    }
}

/// A syscall filter rule
#[derive(Debug, Clone)]
pub struct FilterRule {
    /// Rule id
    pub id: u64,
    /// Priority (higher = evaluated first)
    pub priority: u32,
    /// Syscall number (None = match all)
    pub syscall_nr: Option<u32>,
    /// Syscall range (inclusive)
    pub syscall_range: Option<(u32, u32)>,
    /// Argument filters
    pub arg_filters: Vec<ArgFilter>,
    /// Action
    pub action: FilterAction,
    /// Hit count
    pub hit_count: u64,
}

impl FilterRule {
    pub fn new(id: u64, priority: u32, action: FilterAction) -> Self {
        Self {
            id,
            priority,
            syscall_nr: None,
            syscall_range: None,
            arg_filters: Vec::new(),
            action,
            hit_count: 0,
        }
    }

    /// Match against syscall
    pub fn matches(&self, syscall_nr: u32, args: &[u64]) -> bool {
        // Check syscall number
        if let Some(nr) = self.syscall_nr {
            if syscall_nr != nr {
                return false;
            }
        }
        if let Some((lo, hi)) = self.syscall_range {
            if syscall_nr < lo || syscall_nr > hi {
                return false;
            }
        }
        // Check args
        for filter in &self.arg_filters {
            let idx = filter.arg_index as usize;
            if idx < args.len() {
                if !filter.matches(args[idx]) {
                    return false;
                }
            }
        }
        true
    }
}

// ============================================================================
// FILTER CHAIN
// ============================================================================

/// Syscall filter chain
#[derive(Debug)]
pub struct FilterChain {
    /// Rules sorted by priority
    rules: Vec<FilterRule>,
    /// Default action
    pub default_action: FilterAction,
    /// Next rule id
    next_id: u64,
}

impl FilterChain {
    pub fn new(default_action: FilterAction) -> Self {
        Self {
            rules: Vec::new(),
            default_action,
            next_id: 1,
        }
    }

    /// Add rule
    #[inline]
    pub fn add_rule(&mut self, mut rule: FilterRule) -> u64 {
        rule.id = self.next_id;
        self.next_id += 1;
        let id = rule.id;
        self.rules.push(rule);
        // Sort by priority (descending)
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        id
    }

    /// Remove rule
    #[inline(always)]
    pub fn remove_rule(&mut self, id: u64) {
        self.rules.retain(|r| r.id != id);
    }

    /// Evaluate chain
    #[inline]
    pub fn evaluate(&mut self, syscall_nr: u32, args: &[u64]) -> FilterAction {
        for rule in &mut self.rules {
            if rule.matches(syscall_nr, args) {
                rule.hit_count += 1;
                return rule.action;
            }
        }
        self.default_action
    }

    /// Rule count
    #[inline(always)]
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

// ============================================================================
// AUDIT LOG
// ============================================================================

/// Audit entry
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// Process id
    pub pid: u64,
    /// Syscall number
    pub syscall_nr: u32,
    /// Action taken
    pub action: FilterAction,
    /// Rule id that matched
    pub rule_id: Option<u64>,
    /// Timestamp
    pub timestamp: u64,
}

/// Audit log
#[derive(Debug)]
pub struct AuditLog {
    /// Entries
    entries: VecDeque<AuditEntry>,
    /// Max entries
    max_entries: usize,
    /// Total logged
    pub total_logged: u64,
}

impl AuditLog {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries,
            total_logged: 0,
        }
    }

    /// Log entry
    #[inline]
    pub fn log(&mut self, entry: AuditEntry) {
        if self.entries.len() >= self.max_entries {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
        self.total_logged += 1;
    }

    /// Recent entries
    #[inline]
    pub fn recent(&self, count: usize) -> &[AuditEntry] {
        let start = if self.entries.len() > count {
            self.entries.len() - count
        } else {
            0
        };
        &self.entries[start..]
    }

    /// Denied entries for pid
    #[inline]
    pub fn denied_for(&self, pid: u64) -> Vec<&AuditEntry> {
        self.entries.iter()
            .filter(|e| e.pid == pid && matches!(e.action, FilterAction::Deny | FilterAction::LogDeny | FilterAction::Kill))
            .collect()
    }
}

// ============================================================================
// ISOLATION ENGINE
// ============================================================================

/// Isolation stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BridgeIsolationStats {
    /// Processes with filters
    pub filtered_processes: usize,
    /// Total evaluations
    pub total_evaluations: u64,
    /// Total denials
    pub total_denials: u64,
    /// Audit entries
    pub audit_entries: u64,
}

/// Bridge isolation manager
#[repr(align(64))]
pub struct BridgeIsolationManager {
    /// Per-process filter chains
    chains: BTreeMap<u64, FilterChain>,
    /// Global filter chain
    global_chain: FilterChain,
    /// Audit log
    audit: AuditLog,
    /// Stats
    stats: BridgeIsolationStats,
}

impl BridgeIsolationManager {
    pub fn new() -> Self {
        Self {
            chains: BTreeMap::new(),
            global_chain: FilterChain::new(FilterAction::Allow),
            audit: AuditLog::new(10000),
            stats: BridgeIsolationStats::default(),
        }
    }

    /// Create filter chain for process
    #[inline(always)]
    pub fn create_chain(&mut self, pid: u64, default_action: FilterAction) {
        self.chains.insert(pid, FilterChain::new(default_action));
        self.stats.filtered_processes = self.chains.len();
    }

    /// Add rule to process chain
    #[inline(always)]
    pub fn add_process_rule(&mut self, pid: u64, rule: FilterRule) -> Option<u64> {
        self.chains.get_mut(&pid).map(|chain| chain.add_rule(rule))
    }

    /// Add global rule
    #[inline(always)]
    pub fn add_global_rule(&mut self, rule: FilterRule) -> u64 {
        self.global_chain.add_rule(rule)
    }

    /// Evaluate syscall
    pub fn evaluate(&mut self, pid: u64, syscall_nr: u32, args: &[u64], now: u64) -> FilterAction {
        self.stats.total_evaluations += 1;

        // Process chain first
        let action = if let Some(chain) = self.chains.get_mut(&pid) {
            chain.evaluate(syscall_nr, args)
        } else {
            self.global_chain.evaluate(syscall_nr, args)
        };

        // Audit if denied or logged
        match action {
            FilterAction::Deny | FilterAction::LogDeny | FilterAction::LogAllow
            | FilterAction::Trap | FilterAction::Kill => {
                self.audit.log(AuditEntry {
                    pid,
                    syscall_nr,
                    action,
                    rule_id: None,
                    timestamp: now,
                });
                self.stats.audit_entries = self.audit.total_logged;
            }
            _ => {}
        }

        if matches!(action, FilterAction::Deny | FilterAction::LogDeny | FilterAction::Kill) {
            self.stats.total_denials += 1;
        }

        action
    }

    /// Remove process chain
    #[inline(always)]
    pub fn remove_chain(&mut self, pid: u64) {
        self.chains.remove(&pid);
        self.stats.filtered_processes = self.chains.len();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &BridgeIsolationStats {
        &self.stats
    }
}
