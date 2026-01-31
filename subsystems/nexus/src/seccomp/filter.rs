//! Seccomp Filter
//!
//! BPF-based syscall filtering.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::{Architecture, BpfInsn, FilterAction, FilterId, Pid, SyscallNum};

/// Seccomp filter
#[derive(Debug)]
pub struct SeccompFilter {
    /// Filter ID
    pub id: FilterId,
    /// Instructions
    pub insns: Vec<BpfInsn>,
    /// Architecture
    pub arch: Architecture,
    /// Syscall rules
    pub rules: BTreeMap<SyscallNum, FilterAction>,
    /// Default action
    pub default_action: FilterAction,
    /// Created timestamp
    pub created_at: u64,
    /// Attached process
    pub attached_pid: Option<Pid>,
    /// Is active
    pub active: AtomicBool,
    /// Trigger count
    pub triggers: AtomicU64,
}

impl SeccompFilter {
    /// Create new filter
    pub fn new(
        id: FilterId,
        arch: Architecture,
        default_action: FilterAction,
        timestamp: u64,
    ) -> Self {
        Self {
            id,
            insns: Vec::new(),
            arch,
            rules: BTreeMap::new(),
            default_action,
            created_at: timestamp,
            attached_pid: None,
            active: AtomicBool::new(false),
            triggers: AtomicU64::new(0),
        }
    }

    /// Add syscall rule
    pub fn add_rule(&mut self, syscall: SyscallNum, action: FilterAction) {
        self.rules.insert(syscall, action);
    }

    /// Remove syscall rule
    pub fn remove_rule(&mut self, syscall: SyscallNum) -> Option<FilterAction> {
        self.rules.remove(&syscall)
    }

    /// Get action for syscall
    pub fn get_action(&self, syscall: SyscallNum) -> FilterAction {
        self.rules
            .get(&syscall)
            .copied()
            .unwrap_or(self.default_action)
    }

    /// Check if syscall is allowed
    pub fn is_allowed(&self, syscall: SyscallNum) -> bool {
        matches!(self.get_action(syscall), FilterAction::Allow)
    }

    /// Compile filter to BPF
    pub fn compile(&mut self) {
        self.insns.clear();

        // Load architecture and verify
        self.insns.push(BpfInsn::load_arch());
        let arch_check_jf = (self.rules.len() + 2) as u8;
        self.insns.push(BpfInsn::jeq(
            self.arch.audit_arch(),
            0,
            arch_check_jf.min(255),
        ));

        // Load syscall number
        self.insns.push(BpfInsn::load_syscall());

        // Add rule checks
        let rules_count = self.rules.len();
        for (i, (syscall, action)) in self.rules.iter().enumerate() {
            let remaining = rules_count - i - 1;
            self.insns
                .push(BpfInsn::jeq(syscall.raw(), 0, (remaining as u8) + 1));
            self.insns.push(BpfInsn::ret(self.action_to_bpf(*action)));
        }

        // Default action
        self.insns
            .push(BpfInsn::ret(self.action_to_bpf(self.default_action)));
    }

    /// Convert action to BPF return value
    fn action_to_bpf(&self, action: FilterAction) -> u32 {
        const SECCOMP_RET_KILL_PROCESS: u32 = 0x80000000;
        const SECCOMP_RET_KILL_THREAD: u32 = 0x00000000;
        const SECCOMP_RET_TRAP: u32 = 0x00030000;
        const SECCOMP_RET_ERRNO: u32 = 0x00050000;
        const SECCOMP_RET_TRACE: u32 = 0x7ff00000;
        const SECCOMP_RET_LOG: u32 = 0x7ffc0000;
        const SECCOMP_RET_ALLOW: u32 = 0x7fff0000;
        const SECCOMP_RET_USER_NOTIF: u32 = 0x7fc00000;

        match action {
            FilterAction::Kill => SECCOMP_RET_KILL_PROCESS,
            FilterAction::KillThread => SECCOMP_RET_KILL_THREAD,
            FilterAction::Trap => SECCOMP_RET_TRAP,
            FilterAction::Errno(e) => SECCOMP_RET_ERRNO | (e as u32),
            FilterAction::Trace(t) => SECCOMP_RET_TRACE | (t as u32),
            FilterAction::Log => SECCOMP_RET_LOG,
            FilterAction::Allow => SECCOMP_RET_ALLOW,
            FilterAction::Notify => SECCOMP_RET_USER_NOTIF,
        }
    }

    /// Get instruction count
    pub fn instruction_count(&self) -> usize {
        self.insns.len()
    }

    /// Get rule count
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Activate filter
    pub fn activate(&self) {
        self.active.store(true, Ordering::Relaxed);
    }

    /// Deactivate filter
    pub fn deactivate(&self) {
        self.active.store(false, Ordering::Relaxed);
    }

    /// Is active
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    /// Record trigger
    pub fn record_trigger(&self) {
        self.triggers.fetch_add(1, Ordering::Relaxed);
    }

    /// Get trigger count
    pub fn trigger_count(&self) -> u64 {
        self.triggers.load(Ordering::Relaxed)
    }
}

impl Clone for SeccompFilter {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            insns: self.insns.clone(),
            arch: self.arch,
            rules: self.rules.clone(),
            default_action: self.default_action,
            created_at: self.created_at,
            attached_pid: self.attached_pid,
            active: AtomicBool::new(self.active.load(Ordering::Relaxed)),
            triggers: AtomicU64::new(self.triggers.load(Ordering::Relaxed)),
        }
    }
}
