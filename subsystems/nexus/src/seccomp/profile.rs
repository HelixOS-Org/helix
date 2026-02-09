//! Syscall Profile
//!
//! Syscall usage statistics and profiles.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::{Pid, ProfileId, SyscallCategory, SyscallNum};

/// Syscall usage stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct SyscallStats {
    /// Call count
    pub count: u64,
    /// Last called timestamp
    pub last_called: u64,
    /// First called timestamp
    pub first_called: u64,
    /// Error count
    pub error_count: u64,
    /// Blocked count (by seccomp)
    pub blocked_count: u64,
}

/// Process syscall profile
#[derive(Debug, Clone)]
pub struct SyscallProfile {
    /// Profile ID
    pub id: ProfileId,
    /// Process ID
    pub pid: Pid,
    /// Process name
    pub name: Option<String>,
    /// Syscall usage
    pub syscalls: BTreeMap<SyscallNum, SyscallStats>,
    /// Categories used
    pub categories: BTreeMap<SyscallCategory, u64>,
    /// Profile start time
    pub start_time: u64,
    /// Profile end time
    pub end_time: Option<u64>,
    /// Total syscalls
    pub total_calls: u64,
    /// Is complete
    pub complete: bool,
}

impl SyscallProfile {
    /// Create new profile
    pub fn new(id: ProfileId, pid: Pid, start_time: u64) -> Self {
        Self {
            id,
            pid,
            name: None,
            syscalls: BTreeMap::new(),
            categories: BTreeMap::new(),
            start_time,
            end_time: None,
            total_calls: 0,
            complete: false,
        }
    }

    /// Record syscall
    pub fn record_syscall(
        &mut self,
        syscall: SyscallNum,
        category: SyscallCategory,
        timestamp: u64,
        success: bool,
    ) {
        self.total_calls += 1;

        let stats = self.syscalls.entry(syscall).or_default();
        stats.count += 1;
        stats.last_called = timestamp;
        if stats.first_called == 0 {
            stats.first_called = timestamp;
        }
        if !success {
            stats.error_count += 1;
        }

        *self.categories.entry(category).or_insert(0) += 1;
    }

    /// Record blocked syscall
    #[inline]
    pub fn record_blocked(&mut self, syscall: SyscallNum, timestamp: u64) {
        let stats = self.syscalls.entry(syscall).or_default();
        stats.blocked_count += 1;
        stats.last_called = timestamp;
    }

    /// Get unique syscalls used
    #[inline(always)]
    pub fn unique_syscalls(&self) -> usize {
        self.syscalls.len()
    }

    /// Get most frequent syscalls
    #[inline]
    pub fn top_syscalls(&self, n: usize) -> Vec<(SyscallNum, u64)> {
        let mut sorted: Vec<_> = self.syscalls.iter().map(|(s, st)| (*s, st.count)).collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(n);
        sorted
    }

    /// Finish profiling
    #[inline(always)]
    pub fn finish(&mut self, timestamp: u64) {
        self.end_time = Some(timestamp);
        self.complete = true;
    }

    /// Profile duration
    #[inline(always)]
    pub fn duration(&self) -> Option<u64> {
        self.end_time.map(|end| end - self.start_time)
    }
}
