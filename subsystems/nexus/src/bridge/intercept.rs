//! # Syscall Interception Engine
//!
//! Dynamic syscall interception and monitoring:
//! - Pre/post syscall hooks
//! - Conditional interception
//! - Argument inspection and modification
//! - Return value overrides
//! - Transparent instrumentation
//! - BPF-like filter programs

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// INTERCEPT TYPES
// ============================================================================

/// When to intercept
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterceptPoint {
    /// Before syscall execution
    PreSyscall,
    /// After syscall execution
    PostSyscall,
    /// Both pre and post
    Both,
}

/// Intercept action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterceptAction {
    /// Allow syscall to proceed
    Allow,
    /// Block syscall (return error)
    Block,
    /// Modify arguments
    ModifyArgs,
    /// Override return value
    OverrideReturn,
    /// Log and allow
    LogAllow,
    /// Log and block
    LogBlock,
    /// Redirect to different handler
    Redirect,
    /// Delay execution
    Delay,
    /// Allow but trace
    Trace,
}

/// Intercept verdict (result of filter evaluation)
#[derive(Debug, Clone)]
pub struct InterceptVerdict {
    /// Action to take
    pub action: InterceptAction,
    /// Modified arguments (if action is ModifyArgs)
    pub modified_args: Option<SyscallArgs>,
    /// Override return value (if action is OverrideReturn)
    pub override_return: Option<i64>,
    /// Redirect handler (if action is Redirect)
    pub redirect_handler: Option<u32>,
    /// Delay microseconds (if action is Delay)
    pub delay_us: Option<u64>,
    /// Log message
    pub log_data: Option<u64>,
}

impl InterceptVerdict {
    pub fn allow() -> Self {
        Self {
            action: InterceptAction::Allow,
            modified_args: None,
            override_return: None,
            redirect_handler: None,
            delay_us: None,
            log_data: None,
        }
    }

    pub fn block() -> Self {
        Self {
            action: InterceptAction::Block,
            modified_args: None,
            override_return: None,
            redirect_handler: None,
            delay_us: None,
            log_data: None,
        }
    }

    pub fn override_return(value: i64) -> Self {
        Self {
            action: InterceptAction::OverrideReturn,
            modified_args: None,
            override_return: Some(value),
            redirect_handler: None,
            delay_us: None,
            log_data: None,
        }
    }
}

// ============================================================================
// SYSCALL ARGUMENTS
// ============================================================================

/// Syscall arguments (up to 6)
#[derive(Debug, Clone, Copy)]
pub struct SyscallArgs {
    pub args: [u64; 6],
    pub count: u8,
}

impl SyscallArgs {
    pub fn new() -> Self {
        Self {
            args: [0; 6],
            count: 0,
        }
    }

    pub fn get(&self, idx: usize) -> Option<u64> {
        if idx < self.count as usize {
            Some(self.args[idx])
        } else {
            None
        }
    }

    pub fn set(&mut self, idx: usize, value: u64) {
        if idx < 6 {
            self.args[idx] = value;
            if idx >= self.count as usize {
                self.count = (idx + 1) as u8;
            }
        }
    }
}

// ============================================================================
// FILTER PROGRAMS
// ============================================================================

/// Filter condition
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterCondition {
    /// Match specific PID
    Pid(u64),
    /// Match PID range
    PidRange(u64, u64),
    /// Match syscall number
    SyscallNr(u32),
    /// Match syscall range
    SyscallRange(u32, u32),
    /// Arg[n] equals value
    ArgEquals(u8, u64),
    /// Arg[n] greater than
    ArgGreaterThan(u8, u64),
    /// Arg[n] less than
    ArgLessThan(u8, u64),
    /// Arg[n] has bits set (mask)
    ArgBitSet(u8, u64),
    /// Process is in group
    ProcessGroup(u64),
    /// Always match
    Always,
}

impl FilterCondition {
    /// Evaluate condition
    pub fn evaluate(&self, pid: u64, syscall_nr: u32, args: &SyscallArgs, pgroup: u64) -> bool {
        match self {
            Self::Pid(p) => pid == *p,
            Self::PidRange(lo, hi) => pid >= *lo && pid <= *hi,
            Self::SyscallNr(nr) => syscall_nr == *nr,
            Self::SyscallRange(lo, hi) => syscall_nr >= *lo && syscall_nr <= *hi,
            Self::ArgEquals(idx, val) => args.get(*idx as usize) == Some(*val),
            Self::ArgGreaterThan(idx, val) => {
                args.get(*idx as usize).map_or(false, |v| v > *val)
            }
            Self::ArgLessThan(idx, val) => {
                args.get(*idx as usize).map_or(false, |v| v < *val)
            }
            Self::ArgBitSet(idx, mask) => {
                args.get(*idx as usize).map_or(false, |v| v & mask == *mask)
            }
            Self::ProcessGroup(g) => pgroup == *g,
            Self::Always => true,
        }
    }
}

/// Filter program (sequence of conditions)
#[derive(Debug, Clone)]
pub struct FilterProgram {
    /// Conditions (all must match = AND logic)
    pub conditions: Vec<FilterCondition>,
    /// Action if all conditions match
    pub action: InterceptAction,
    /// Priority (higher = evaluated first)
    pub priority: u32,
    /// Is enabled
    pub enabled: bool,
}

impl FilterProgram {
    pub fn new(action: InterceptAction, priority: u32) -> Self {
        Self {
            conditions: Vec::new(),
            action,
            priority,
            enabled: true,
        }
    }

    pub fn add_condition(&mut self, condition: FilterCondition) {
        self.conditions.push(condition);
    }

    /// Evaluate all conditions
    pub fn evaluate(&self, pid: u64, syscall_nr: u32, args: &SyscallArgs, pgroup: u64) -> bool {
        if !self.enabled {
            return false;
        }
        self.conditions
            .iter()
            .all(|c| c.evaluate(pid, syscall_nr, args, pgroup))
    }
}

// ============================================================================
// INTERCEPT HOOK
// ============================================================================

/// An installed intercept hook
#[derive(Debug, Clone)]
pub struct InterceptHook {
    /// Hook ID
    pub id: u32,
    /// Intercept point
    pub point: InterceptPoint,
    /// Filter program
    pub filter: FilterProgram,
    /// Statistics
    pub stats: InterceptHookStats,
    /// Expiry timestamp (0 = never)
    pub expires_at: u64,
}

/// Hook statistics
#[derive(Debug, Clone, Default)]
pub struct InterceptHookStats {
    /// Times evaluated
    pub evaluations: u64,
    /// Times matched
    pub matches: u64,
    /// Times executed
    pub executions: u64,
    /// Errors
    pub errors: u64,
}

// ============================================================================
// INTERCEPT LOG
// ============================================================================

/// Log entry for intercepted syscall
#[derive(Debug, Clone)]
pub struct InterceptLogEntry {
    /// Timestamp
    pub timestamp: u64,
    /// Process ID
    pub pid: u64,
    /// Syscall number
    pub syscall_nr: u32,
    /// Hook that matched
    pub hook_id: u32,
    /// Action taken
    pub action: InterceptAction,
    /// Arguments
    pub args: SyscallArgs,
    /// Return value (post-syscall)
    pub return_value: Option<i64>,
}

// ============================================================================
// INTERCEPT ENGINE
// ============================================================================

/// Syscall interception engine
pub struct InterceptEngine {
    /// Pre-syscall hooks
    pre_hooks: Vec<InterceptHook>,
    /// Post-syscall hooks
    post_hooks: Vec<InterceptHook>,
    /// Intercept log
    log: Vec<InterceptLogEntry>,
    /// Max log entries
    max_log: usize,
    /// Next hook ID
    next_hook_id: u32,
    /// Total interceptions
    pub total_interceptions: u64,
    /// Total blocked
    pub total_blocked: u64,
    /// Total modified
    pub total_modified: u64,
    /// Enabled
    pub enabled: bool,
}

impl InterceptEngine {
    pub fn new() -> Self {
        Self {
            pre_hooks: Vec::new(),
            post_hooks: Vec::new(),
            log: Vec::new(),
            max_log: 1000,
            next_hook_id: 1,
            total_interceptions: 0,
            total_blocked: 0,
            total_modified: 0,
            enabled: true,
        }
    }

    /// Install hook
    pub fn install_hook(&mut self, point: InterceptPoint, filter: FilterProgram, expires_at: u64) -> u32 {
        let id = self.next_hook_id;
        self.next_hook_id += 1;

        let hook = InterceptHook {
            id,
            point,
            filter,
            stats: InterceptHookStats::default(),
            expires_at,
        };

        match point {
            InterceptPoint::PreSyscall => self.pre_hooks.push(hook),
            InterceptPoint::PostSyscall => self.post_hooks.push(hook),
            InterceptPoint::Both => {
                self.pre_hooks.push(hook.clone());
                self.post_hooks.push(hook);
            }
        }

        // Sort by priority (descending)
        self.pre_hooks
            .sort_by(|a, b| b.filter.priority.cmp(&a.filter.priority));
        self.post_hooks
            .sort_by(|a, b| b.filter.priority.cmp(&a.filter.priority));

        id
    }

    /// Remove hook
    pub fn remove_hook(&mut self, hook_id: u32) -> bool {
        let pre_len = self.pre_hooks.len();
        self.pre_hooks.retain(|h| h.id != hook_id);
        let post_len = self.post_hooks.len();
        self.post_hooks.retain(|h| h.id != hook_id);
        self.pre_hooks.len() < pre_len || self.post_hooks.len() < post_len
    }

    /// Evaluate pre-syscall hooks
    pub fn pre_syscall(
        &mut self,
        pid: u64,
        syscall_nr: u32,
        args: &SyscallArgs,
        pgroup: u64,
        timestamp: u64,
    ) -> InterceptVerdict {
        if !self.enabled {
            return InterceptVerdict::allow();
        }

        for hook in &mut self.pre_hooks {
            // Check expiry
            if hook.expires_at > 0 && timestamp > hook.expires_at {
                continue;
            }

            hook.stats.evaluations += 1;

            if hook.filter.evaluate(pid, syscall_nr, args, pgroup) {
                hook.stats.matches += 1;
                hook.stats.executions += 1;
                self.total_interceptions += 1;

                let verdict = match hook.filter.action {
                    InterceptAction::Block | InterceptAction::LogBlock => {
                        self.total_blocked += 1;
                        InterceptVerdict::block()
                    }
                    InterceptAction::OverrideReturn => {
                        InterceptVerdict::override_return(-1)
                    }
                    _ => InterceptVerdict::allow(),
                };

                // Log
                self.log_intercept(timestamp, pid, syscall_nr, hook.id, hook.filter.action, args, None);

                return verdict;
            }
        }

        InterceptVerdict::allow()
    }

    /// Evaluate post-syscall hooks
    pub fn post_syscall(
        &mut self,
        pid: u64,
        syscall_nr: u32,
        args: &SyscallArgs,
        return_value: i64,
        pgroup: u64,
        timestamp: u64,
    ) -> InterceptVerdict {
        if !self.enabled {
            return InterceptVerdict::allow();
        }

        for hook in &mut self.post_hooks {
            if hook.expires_at > 0 && timestamp > hook.expires_at {
                continue;
            }

            hook.stats.evaluations += 1;

            if hook.filter.evaluate(pid, syscall_nr, args, pgroup) {
                hook.stats.matches += 1;
                hook.stats.executions += 1;

                self.log_intercept(
                    timestamp,
                    pid,
                    syscall_nr,
                    hook.id,
                    hook.filter.action,
                    args,
                    Some(return_value),
                );
            }
        }

        InterceptVerdict::allow()
    }

    /// Add log entry
    fn log_intercept(
        &mut self,
        timestamp: u64,
        pid: u64,
        syscall_nr: u32,
        hook_id: u32,
        action: InterceptAction,
        args: &SyscallArgs,
        return_value: Option<i64>,
    ) {
        self.log.push(InterceptLogEntry {
            timestamp,
            pid,
            syscall_nr,
            hook_id,
            action,
            args: *args,
            return_value,
        });
        if self.log.len() > self.max_log {
            self.log.remove(0);
        }
    }

    /// Expire old hooks
    pub fn expire_hooks(&mut self, current_time: u64) {
        self.pre_hooks
            .retain(|h| h.expires_at == 0 || current_time <= h.expires_at);
        self.post_hooks
            .retain(|h| h.expires_at == 0 || current_time <= h.expires_at);
    }

    /// Hook count
    pub fn hook_count(&self) -> usize {
        self.pre_hooks.len() + self.post_hooks.len()
    }

    /// Log entries
    pub fn log_entries(&self) -> &[InterceptLogEntry] {
        &self.log
    }
}
