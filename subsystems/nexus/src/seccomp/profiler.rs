//! Syscall Profiler
//!
//! Active syscall profiling for processes.

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::{Pid, ProfileId, SyscallCategory, SyscallNum, SyscallProfile};

/// Syscall profiler
pub struct SyscallProfiler {
    /// Active profiles
    profiles: BTreeMap<Pid, SyscallProfile>,
    /// Completed profiles
    completed: VecDeque<SyscallProfile>,
    /// Max completed profiles
    max_completed: usize,
    /// Next profile ID
    next_id: AtomicU64,
    /// Enabled
    enabled: AtomicBool,
}

impl SyscallProfiler {
    /// Create new profiler
    pub fn new(max_completed: usize) -> Self {
        Self {
            profiles: BTreeMap::new(),
            completed: VecDeque::new(),
            max_completed,
            next_id: AtomicU64::new(1),
            enabled: AtomicBool::new(true),
        }
    }

    /// Start profiling process
    #[inline]
    pub fn start_profile(&mut self, pid: Pid, timestamp: u64) -> ProfileId {
        let id = ProfileId::new(self.next_id.fetch_add(1, Ordering::Relaxed));
        let profile = SyscallProfile::new(id, pid, timestamp);
        self.profiles.insert(pid, profile);
        id
    }

    /// Stop profiling process
    pub fn stop_profile(&mut self, pid: Pid, timestamp: u64) -> Option<SyscallProfile> {
        if let Some(mut profile) = self.profiles.remove(&pid) {
            profile.finish(timestamp);

            if self.completed.len() >= self.max_completed {
                self.completed.pop_front();
            }
            self.completed.push_back(profile.clone());

            Some(profile)
        } else {
            None
        }
    }

    /// Record syscall
    #[inline]
    pub fn record(
        &mut self,
        pid: Pid,
        syscall: SyscallNum,
        category: SyscallCategory,
        timestamp: u64,
        success: bool,
    ) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.record_syscall(syscall, category, timestamp, success);
        }
    }

    /// Get active profile
    #[inline(always)]
    pub fn get_profile(&self, pid: Pid) -> Option<&SyscallProfile> {
        self.profiles.get(&pid)
    }

    /// Get completed profiles
    #[inline(always)]
    pub fn completed_profiles(&self) -> &[SyscallProfile] {
        &self.completed
    }

    /// Enable/disable
    #[inline(always)]
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Is enabled
    #[inline(always)]
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Active profile count
    #[inline(always)]
    pub fn active_count(&self) -> usize {
        self.profiles.len()
    }

    /// Completed profile count
    #[inline(always)]
    pub fn completed_count(&self) -> usize {
        self.completed.len()
    }
}

impl Default for SyscallProfiler {
    fn default() -> Self {
        Self::new(100)
    }
}
