// SPDX-License-Identifier: GPL-2.0
//! NEXUS Bridge — Priority (process/thread priority bridge)

extern crate alloc;
use alloc::collections::BTreeMap;

/// Priority scheduling policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgePriorityPolicy {
    Normal,
    Fifo,
    RoundRobin,
    Batch,
    Idle,
    Deadline,
}

/// Priority entry
#[derive(Debug, Clone)]
pub struct BridgePriorityEntry {
    pub id: u64,
    pub policy: BridgePriorityPolicy,
    pub static_priority: i32,
    pub dynamic_priority: i32,
    pub nice_value: i32,
    pub rt_priority: u32,
}

/// Stats for priority operations
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BridgePriorityStats {
    pub total_changes: u64,
    pub priority_boosts: u64,
    pub priority_inversions: u64,
    pub policy_changes: u64,
    pub nice_changes: u64,
}

/// Manager for priority bridge operations
#[repr(align(64))]
pub struct BridgePriorityManager {
    entries: BTreeMap<u64, BridgePriorityEntry>,
    stats: BridgePriorityStats,
}

impl BridgePriorityManager {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            stats: BridgePriorityStats {
                total_changes: 0,
                priority_boosts: 0,
                priority_inversions: 0,
                policy_changes: 0,
                nice_changes: 0,
            },
        }
    }

    pub fn set_priority(&mut self, id: u64, policy: BridgePriorityPolicy, priority: i32) {
        self.stats.total_changes += 1;
        if let Some(entry) = self.entries.get_mut(&id) {
            if entry.policy != policy {
                self.stats.policy_changes += 1;
            }
            entry.policy = policy;
            entry.static_priority = priority;
            entry.dynamic_priority = priority;
        } else {
            let entry = BridgePriorityEntry {
                id,
                policy,
                static_priority: priority,
                dynamic_priority: priority,
                nice_value: 0,
                rt_priority: if matches!(policy, BridgePriorityPolicy::Fifo | BridgePriorityPolicy::RoundRobin) { priority as u32 } else { 0 },
            };
            self.entries.insert(id, entry);
        }
    }

    #[inline]
    pub fn set_nice(&mut self, id: u64, nice: i32) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.nice_value = nice.clamp(-20, 19);
            self.stats.nice_changes += 1;
        }
    }

    #[inline]
    pub fn boost_priority(&mut self, id: u64, boost: i32) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.dynamic_priority = entry.static_priority + boost;
            self.stats.priority_boosts += 1;
        }
    }

    #[inline(always)]
    pub fn get_priority(&self, id: u64) -> Option<i32> {
        self.entries.get(&id).map(|e| e.dynamic_priority)
    }

    #[inline(always)]
    pub fn stats(&self) -> &BridgePriorityStats {
        &self.stats
    }
}
