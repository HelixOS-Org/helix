//! Attack Surface Analysis
//!
//! Security analysis for syscall filtering.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::{RiskLevel, SeccompFilter, SyscallCategory, SyscallInfo, SyscallNum};

/// Attack surface analysis
#[derive(Debug, Clone)]
pub struct AttackSurfaceAnalysis {
    /// Total syscalls available
    pub total_syscalls: usize,
    /// Allowed syscalls
    pub allowed_syscalls: usize,
    /// Blocked syscalls
    pub blocked_syscalls: usize,
    /// Attack surface reduction (%)
    pub reduction_percent: f32,
    /// Risk score (0-100)
    pub risk_score: f32,
    /// Critical syscalls allowed
    pub critical_allowed: Vec<SyscallNum>,
    /// High risk syscalls allowed
    pub high_risk_allowed: Vec<SyscallNum>,
}

impl AttackSurfaceAnalysis {
    /// Create new analysis
    pub fn new() -> Self {
        Self {
            total_syscalls: 0,
            allowed_syscalls: 0,
            blocked_syscalls: 0,
            reduction_percent: 0.0,
            risk_score: 0.0,
            critical_allowed: Vec::new(),
            high_risk_allowed: Vec::new(),
        }
    }
}

impl Default for AttackSurfaceAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

/// Attack surface analyzer
pub struct AttackSurfaceAnalyzer {
    /// Syscall database
    syscall_db: BTreeMap<SyscallNum, SyscallInfo>,
    /// Critical syscalls
    critical_syscalls: Vec<SyscallNum>,
    /// High risk syscalls
    high_risk_syscalls: Vec<SyscallNum>,
}

impl AttackSurfaceAnalyzer {
    /// Create new analyzer
    pub fn new() -> Self {
        let mut analyzer = Self {
            syscall_db: BTreeMap::new(),
            critical_syscalls: Vec::new(),
            high_risk_syscalls: Vec::new(),
        };
        analyzer.init_syscall_db();
        analyzer
    }

    /// Initialize syscall database
    fn init_syscall_db(&mut self) {
        // Critical syscalls
        self.critical_syscalls = vec![
            SyscallNum::INIT_MODULE,
            SyscallNum::DELETE_MODULE,
            SyscallNum::REBOOT,
            SyscallNum::KEXEC_LOAD,
            SyscallNum::MOUNT,
            SyscallNum::UMOUNT2,
        ];

        // High risk syscalls
        self.high_risk_syscalls = vec![
            SyscallNum::PTRACE,
            SyscallNum::SETUID,
            SyscallNum::SETGID,
            SyscallNum::SETREUID,
            SyscallNum::SETREGID,
            SyscallNum::SETHOSTNAME,
            SyscallNum::SETDOMAINNAME,
            SyscallNum::EXECVE,
        ];

        // Add some key syscalls to database
        self.syscall_db.insert(
            SyscallNum::READ,
            SyscallInfo::new(
                SyscallNum::READ,
                "read",
                SyscallCategory::File,
                RiskLevel::Safe,
                "Read from file descriptor",
                3,
            ),
        );
        self.syscall_db.insert(
            SyscallNum::WRITE,
            SyscallInfo::new(
                SyscallNum::WRITE,
                "write",
                SyscallCategory::File,
                RiskLevel::Safe,
                "Write to file descriptor",
                3,
            ),
        );
        self.syscall_db.insert(
            SyscallNum::EXECVE,
            SyscallInfo::new(
                SyscallNum::EXECVE,
                "execve",
                SyscallCategory::Process,
                RiskLevel::High,
                "Execute program",
                3,
            ),
        );
        self.syscall_db.insert(
            SyscallNum::PTRACE,
            SyscallInfo::new(
                SyscallNum::PTRACE,
                "ptrace",
                SyscallCategory::Debug,
                RiskLevel::High,
                "Process trace",
                4,
            ),
        );
        self.syscall_db.insert(
            SyscallNum::MOUNT,
            SyscallInfo::new(
                SyscallNum::MOUNT,
                "mount",
                SyscallCategory::System,
                RiskLevel::Critical,
                "Mount filesystem",
                5,
            ),
        );
        self.syscall_db.insert(
            SyscallNum::REBOOT,
            SyscallInfo::new(
                SyscallNum::REBOOT,
                "reboot",
                SyscallCategory::System,
                RiskLevel::Critical,
                "Reboot system",
                4,
            ),
        );
    }

    /// Analyze filter attack surface
    pub fn analyze(&self, filter: &SeccompFilter) -> AttackSurfaceAnalysis {
        let total_syscalls = 400; // Approximate number of syscalls
        let mut allowed_syscalls = 0;
        let mut critical_allowed = Vec::new();
        let mut high_risk_allowed = Vec::new();
        let mut risk_score = 0.0f32;

        // Count based on default action
        let default_is_allow = matches!(filter.default_action, super::FilterAction::Allow);

        if default_is_allow {
            allowed_syscalls =
                total_syscalls - filter.rules.iter().filter(|(_, a)| a.is_blocking()).count();
        } else {
            allowed_syscalls = filter
                .rules
                .iter()
                .filter(|(_, a)| !a.is_blocking())
                .count();
        }

        // Check critical syscalls
        for syscall in &self.critical_syscalls {
            let action = filter.get_action(*syscall);
            if !action.is_blocking() {
                critical_allowed.push(*syscall);
                risk_score += 20.0;
            }
        }

        // Check high risk syscalls
        for syscall in &self.high_risk_syscalls {
            let action = filter.get_action(*syscall);
            if !action.is_blocking() {
                high_risk_allowed.push(*syscall);
                risk_score += 10.0;
            }
        }

        let blocked_syscalls = total_syscalls - allowed_syscalls;
        let reduction_percent = (blocked_syscalls as f32 / total_syscalls as f32) * 100.0;

        risk_score = risk_score.min(100.0);

        AttackSurfaceAnalysis {
            total_syscalls,
            allowed_syscalls,
            blocked_syscalls,
            reduction_percent,
            risk_score,
            critical_allowed,
            high_risk_allowed,
        }
    }

    /// Get syscall info
    #[inline(always)]
    pub fn get_syscall_info(&self, syscall: SyscallNum) -> Option<&SyscallInfo> {
        self.syscall_db.get(&syscall)
    }

    /// Get critical syscalls
    #[inline(always)]
    pub fn critical_syscalls(&self) -> &[SyscallNum] {
        &self.critical_syscalls
    }

    /// Get high risk syscalls
    #[inline(always)]
    pub fn high_risk_syscalls(&self) -> &[SyscallNum] {
        &self.high_risk_syscalls
    }
}

impl Default for AttackSurfaceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
