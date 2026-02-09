// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Nice (process priority/nice application interface)

extern crate alloc;
use alloc::collections::BTreeMap;

/// Scheduling policy for app layer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppSchedPolicy {
    Normal,
    Fifo,
    RoundRobin,
    Batch,
    Idle,
    Deadline,
}

/// Nice entry per process
#[derive(Debug, Clone)]
pub struct AppNiceEntry {
    pub pid: u64,
    pub nice: i32,
    pub policy: AppSchedPolicy,
    pub rt_priority: u32,
    pub cpu_shares: u64,
}

/// Stats for nice operations
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AppNiceStats {
    pub total_ops: u64,
    pub nice_increases: u64,
    pub nice_decreases: u64,
    pub policy_changes: u64,
    pub permission_denied: u64,
}

/// Manager for nice application operations
pub struct AppNiceManager {
    entries: BTreeMap<u64, AppNiceEntry>,
    stats: AppNiceStats,
}

impl AppNiceManager {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            stats: AppNiceStats {
                total_ops: 0,
                nice_increases: 0,
                nice_decreases: 0,
                policy_changes: 0,
                permission_denied: 0,
            },
        }
    }

    #[inline]
    pub fn register(&mut self, pid: u64, nice: i32, policy: AppSchedPolicy) {
        let entry = AppNiceEntry {
            pid,
            nice,
            policy,
            rt_priority: 0,
            cpu_shares: 1024,
        };
        self.entries.insert(pid, entry);
    }

    pub fn set_nice(&mut self, pid: u64, new_nice: i32) -> bool {
        self.stats.total_ops += 1;
        let clamped = new_nice.clamp(-20, 19);
        if let Some(entry) = self.entries.get_mut(&pid) {
            if clamped > entry.nice {
                self.stats.nice_increases += 1;
            } else if clamped < entry.nice {
                self.stats.nice_decreases += 1;
            }
            entry.nice = clamped;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn set_scheduler(&mut self, pid: u64, policy: AppSchedPolicy, priority: u32) -> bool {
        self.stats.total_ops += 1;
        if let Some(entry) = self.entries.get_mut(&pid) {
            entry.policy = policy;
            entry.rt_priority = priority;
            self.stats.policy_changes += 1;
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn get_nice(&self, pid: u64) -> Option<i32> {
        self.entries.get(&pid).map(|e| e.nice)
    }

    #[inline(always)]
    pub fn get_policy(&self, pid: u64) -> Option<AppSchedPolicy> {
        self.entries.get(&pid).map(|e| e.policy)
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppNiceStats {
        &self.stats
    }
}
