// SPDX-License-Identifier: GPL-2.0
//! NEXUS Bridge — TID (thread ID management bridge)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// TID allocation policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeTidPolicy {
    Sequential,
    Recycled,
    RandomOffset,
    PerCpuPool,
}

/// TID entry
#[derive(Debug, Clone)]
pub struct BridgeTidEntry {
    pub tid: u64,
    pub pid: u64,
    pub tgid: u64,
    pub cpu: u32,
    pub allocated_time: u64,
}

/// Stats for TID management
#[derive(Debug, Clone)]
pub struct BridgeTidStats {
    pub total_allocated: u64,
    pub total_freed: u64,
    pub active_tids: u64,
    pub peak_tids: u64,
    pub recycle_count: u64,
}

/// Manager for TID bridge operations
pub struct BridgeTidManager {
    tids: BTreeMap<u64, BridgeTidEntry>,
    free_list: Vec<u64>,
    next_tid: u64,
    policy: BridgeTidPolicy,
    stats: BridgeTidStats,
}

impl BridgeTidManager {
    pub fn new() -> Self {
        Self {
            tids: BTreeMap::new(),
            free_list: Vec::new(),
            next_tid: 1,
            policy: BridgeTidPolicy::Sequential,
            stats: BridgeTidStats {
                total_allocated: 0,
                total_freed: 0,
                active_tids: 0,
                peak_tids: 0,
                recycle_count: 0,
            },
        }
    }

    pub fn allocate(&mut self, pid: u64, tgid: u64, cpu: u32) -> u64 {
        let tid = if matches!(self.policy, BridgeTidPolicy::Recycled) && !self.free_list.is_empty() {
            self.stats.recycle_count += 1;
            self.free_list.pop().unwrap()
        } else {
            let t = self.next_tid;
            self.next_tid += 1;
            t
        };
        let entry = BridgeTidEntry {
            tid,
            pid,
            tgid,
            cpu,
            allocated_time: tid.wrapping_mul(47),
        };
        self.tids.insert(tid, entry);
        self.stats.total_allocated += 1;
        self.stats.active_tids += 1;
        if self.stats.active_tids > self.stats.peak_tids {
            self.stats.peak_tids = self.stats.active_tids;
        }
        tid
    }

    pub fn free(&mut self, tid: u64) -> bool {
        if self.tids.remove(&tid).is_some() {
            if matches!(self.policy, BridgeTidPolicy::Recycled) {
                self.free_list.push(tid);
            }
            self.stats.total_freed += 1;
            self.stats.active_tids = self.stats.active_tids.saturating_sub(1);
            true
        } else {
            false
        }
    }

    pub fn set_policy(&mut self, policy: BridgeTidPolicy) {
        self.policy = policy;
    }

    pub fn lookup(&self, tid: u64) -> Option<&BridgeTidEntry> {
        self.tids.get(&tid)
    }

    pub fn stats(&self) -> &BridgeTidStats {
        &self.stats
    }
}
