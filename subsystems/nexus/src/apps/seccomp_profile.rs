//! # App Seccomp Profiler
//!
//! Seccomp (secure computing) filter profiling:
//! - BPF filter analysis per process
//! - Syscall allow/deny statistics
//! - Filter chain depth tracking
//! - Violation pattern detection
//! - Security posture scoring

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// SECCOMP TYPES
// ============================================================================

/// Seccomp action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompAction {
    /// Allow syscall
    Allow,
    /// Kill process
    Kill,
    /// Kill thread
    KillThread,
    /// Trap (deliver SIGSYS)
    Trap,
    /// Return errno
    Errno,
    /// Trace (ptrace notification)
    Trace,
    /// Log the syscall
    Log,
    /// User notification
    UserNotif,
}

/// Filter match result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterResult {
    /// Matched explicit rule
    ExplicitMatch,
    /// Fell through to default
    DefaultAction,
    /// No filter installed
    NoFilter,
}

/// Violation severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationSeverity {
    /// Informational
    Info,
    /// Warning
    Warning,
    /// Critical violation
    Critical,
    /// Fatal (process killed)
    Fatal,
}

// ============================================================================
// FILTER RULE
// ============================================================================

/// BPF filter rule representation
#[derive(Debug, Clone)]
pub struct FilterRule {
    /// Syscall number
    pub syscall_nr: u32,
    /// Action on match
    pub action: SeccompAction,
    /// Argument conditions count
    pub arg_conditions: u8,
    /// Hit count
    pub hit_count: u64,
    /// Last hit (ns)
    pub last_hit_ns: u64,
}

impl FilterRule {
    pub fn new(syscall_nr: u32, action: SeccompAction) -> Self {
        Self {
            syscall_nr,
            action,
            arg_conditions: 0,
            hit_count: 0,
            last_hit_ns: 0,
        }
    }

    /// Record hit
    pub fn record_hit(&mut self, now: u64) {
        self.hit_count += 1;
        self.last_hit_ns = now;
    }
}

/// Filter chain
#[derive(Debug, Clone)]
pub struct FilterChain {
    /// Filter ID
    pub filter_id: u64,
    /// Rules
    pub rules: Vec<FilterRule>,
    /// Default action
    pub default_action: SeccompAction,
    /// Chain depth (nested filters)
    pub chain_depth: u32,
    /// Total evaluations
    pub total_evals: u64,
    /// BPF instruction count
    pub bpf_insn_count: u32,
}

impl FilterChain {
    pub fn new(filter_id: u64, default_action: SeccompAction) -> Self {
        Self {
            filter_id,
            rules: Vec::new(),
            default_action,
            chain_depth: 1,
            total_evals: 0,
            bpf_insn_count: 0,
        }
    }

    /// Add rule
    pub fn add_rule(&mut self, rule: FilterRule) {
        self.rules.push(rule);
    }

    /// Evaluate syscall
    pub fn evaluate(&mut self, syscall_nr: u32, now: u64) -> (SeccompAction, FilterResult) {
        self.total_evals += 1;
        for rule in &mut self.rules {
            if rule.syscall_nr == syscall_nr {
                rule.record_hit(now);
                return (rule.action, FilterResult::ExplicitMatch);
            }
        }
        (self.default_action, FilterResult::DefaultAction)
    }

    /// Count allowed syscalls
    pub fn allowed_count(&self) -> usize {
        self.rules.iter()
            .filter(|r| matches!(r.action, SeccompAction::Allow))
            .count()
    }

    /// Count denied syscalls
    pub fn denied_count(&self) -> usize {
        self.rules.iter()
            .filter(|r| !matches!(r.action, SeccompAction::Allow))
            .count()
    }

    /// Security strictness score (0..100)
    pub fn strictness_score(&self) -> f64 {
        let total = self.rules.len();
        if total == 0 {
            return match self.default_action {
                SeccompAction::Kill | SeccompAction::KillThread => 100.0,
                SeccompAction::Errno => 80.0,
                SeccompAction::Allow => 0.0,
                _ => 50.0,
            };
        }
        let denied = self.denied_count() as f64;
        let base = denied / total as f64 * 70.0;
        let default_bonus = match self.default_action {
            SeccompAction::Kill | SeccompAction::KillThread => 30.0,
            SeccompAction::Errno => 20.0,
            _ => 0.0,
        };
        let score = base + default_bonus;
        if score > 100.0 { 100.0 } else { score }
    }
}

// ============================================================================
// VIOLATION RECORD
// ============================================================================

/// Seccomp violation record
#[derive(Debug, Clone)]
pub struct ViolationRecord {
    /// PID
    pub pid: u64,
    /// Syscall number
    pub syscall_nr: u32,
    /// Action taken
    pub action: SeccompAction,
    /// Severity
    pub severity: ViolationSeverity,
    /// Timestamp
    pub timestamp_ns: u64,
    /// Count of this exact violation
    pub repeat_count: u64,
}

// ============================================================================
// PER-PROCESS SECCOMP
// ============================================================================

/// Per-process seccomp profile
#[derive(Debug)]
pub struct ProcessSeccompProfile {
    /// PID
    pub pid: u64,
    /// Installed filter chain
    pub filter: Option<FilterChain>,
    /// Violations
    violations: Vec<ViolationRecord>,
    /// Total syscalls evaluated
    pub total_evals: u64,
    /// Total violations
    pub total_violations: u64,
    /// Violation histogram (syscall_nr -> count)
    violation_hist: BTreeMap<u32, u64>,
}

impl ProcessSeccompProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            filter: None,
            violations: Vec::new(),
            total_evals: 0,
            total_violations: 0,
            violation_hist: BTreeMap::new(),
        }
    }

    /// Install filter
    pub fn install_filter(&mut self, filter: FilterChain) {
        self.filter = Some(filter);
    }

    /// Evaluate syscall
    pub fn evaluate(&mut self, syscall_nr: u32, now: u64) -> SeccompAction {
        self.total_evals += 1;
        if let Some(ref mut filter) = self.filter {
            let (action, _result) = filter.evaluate(syscall_nr, now);
            if !matches!(action, SeccompAction::Allow) {
                self.record_violation(syscall_nr, action, now);
            }
            action
        } else {
            SeccompAction::Allow
        }
    }

    fn record_violation(&mut self, syscall_nr: u32, action: SeccompAction, now: u64) {
        self.total_violations += 1;
        *self.violation_hist.entry(syscall_nr).or_insert(0) += 1;

        let severity = match action {
            SeccompAction::Kill | SeccompAction::KillThread => ViolationSeverity::Fatal,
            SeccompAction::Trap => ViolationSeverity::Critical,
            SeccompAction::Errno => ViolationSeverity::Warning,
            _ => ViolationSeverity::Info,
        };

        // Deduplicate recent violations
        if let Some(last) = self.violations.last_mut() {
            if last.syscall_nr == syscall_nr && last.action as u8 == action as u8 {
                last.repeat_count += 1;
                return;
            }
        }

        if self.violations.len() >= 256 {
            self.violations.remove(0);
        }
        self.violations.push(ViolationRecord {
            pid: self.pid,
            syscall_nr,
            action,
            severity,
            timestamp_ns: now,
            repeat_count: 1,
        });
    }

    /// Violation rate
    pub fn violation_rate(&self) -> f64 {
        if self.total_evals == 0 {
            return 0.0;
        }
        self.total_violations as f64 / self.total_evals as f64
    }

    /// Top violated syscalls
    pub fn top_violated(&self, n: usize) -> Vec<(u32, u64)> {
        let mut sorted: Vec<(u32, u64)> = self.violation_hist.iter()
            .map(|(&k, &v)| (k, v))
            .collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(n);
        sorted
    }

    /// Security score
    pub fn security_score(&self) -> f64 {
        match &self.filter {
            None => 0.0, // No filter = no protection
            Some(f) => f.strictness_score(),
        }
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Seccomp profiler stats
#[derive(Debug, Clone, Default)]
pub struct AppSeccompProfilerStats {
    /// Tracked processes
    pub tracked_processes: usize,
    /// Processes with filters
    pub filtered_processes: usize,
    /// Total violations
    pub total_violations: u64,
    /// Average security score
    pub avg_security_score: f64,
}

/// App seccomp profiler
pub struct AppSeccompProfiler {
    /// Per-process profiles
    processes: BTreeMap<u64, ProcessSeccompProfile>,
    /// Stats
    stats: AppSeccompProfilerStats,
}

impl AppSeccompProfiler {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            stats: AppSeccompProfilerStats::default(),
        }
    }

    /// Get/create process
    pub fn process(&mut self, pid: u64) -> &mut ProcessSeccompProfile {
        self.processes.entry(pid).or_insert_with(|| ProcessSeccompProfile::new(pid))
    }

    /// Install filter
    pub fn install_filter(&mut self, pid: u64, filter: FilterChain) {
        let proc = self.processes.entry(pid).or_insert_with(|| ProcessSeccompProfile::new(pid));
        proc.install_filter(filter);
        self.update_stats();
    }

    /// Evaluate syscall
    pub fn evaluate(&mut self, pid: u64, syscall_nr: u32, now: u64) -> SeccompAction {
        let proc = self.processes.entry(pid).or_insert_with(|| ProcessSeccompProfile::new(pid));
        let action = proc.evaluate(syscall_nr, now);
        if !matches!(action, SeccompAction::Allow) {
            self.stats.total_violations += 1;
        }
        action
    }

    /// Remove process
    pub fn remove_process(&mut self, pid: u64) {
        self.processes.remove(&pid);
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.filtered_processes = self.processes.values()
            .filter(|p| p.filter.is_some())
            .count();
        if !self.processes.is_empty() {
            self.stats.avg_security_score = self.processes.values()
                .map(|p| p.security_score())
                .sum::<f64>() / self.processes.len() as f64;
        }
    }

    /// Stats
    pub fn stats(&self) -> &AppSeccompProfilerStats {
        &self.stats
    }
}
