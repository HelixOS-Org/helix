// SPDX-License-Identifier: GPL-2.0
//! Coop priority_inherit — priority inheritance protocol.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Thread priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ThreadPriority(pub u32);

/// Priority inheritance chain
#[derive(Debug)]
pub struct PiChainEntry {
    pub blocker_tid: u64,
    pub blocked_tid: u64,
    pub resource_id: u64,
    pub original_priority: ThreadPriority,
    pub boosted_priority: ThreadPriority,
}

/// PI-aware mutex
#[derive(Debug)]
pub struct PiMutex {
    pub id: u64,
    pub owner: Option<u64>,
    pub waiters: Vec<(u64, ThreadPriority)>,
    pub boost_count: u64,
    pub acquisitions: u64,
}

impl PiMutex {
    pub fn new(id: u64) -> Self { Self { id, owner: None, waiters: Vec::new(), boost_count: 0, acquisitions: 0 } }

    pub fn lock(&mut self, tid: u64, prio: ThreadPriority) -> bool {
        if self.owner.is_none() { self.owner = Some(tid); self.acquisitions += 1; return true; }
        self.waiters.push((tid, prio));
        self.waiters.sort_by(|a, b| b.1.cmp(&a.1));
        false
    }

    pub fn unlock(&mut self) -> Option<u64> {
        self.owner = None;
        if let Some((tid, _prio)) = self.waiters.first().cloned() {
            self.waiters.remove(0);
            self.owner = Some(tid);
            self.acquisitions += 1;
            Some(tid)
        } else { None }
    }

    pub fn highest_waiter_prio(&self) -> Option<ThreadPriority> {
        self.waiters.first().map(|(_tid, prio)| *prio)
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct PriorityInheritStats {
    pub total_mutexes: u32,
    pub active_chains: u32,
    pub total_boosts: u64,
    pub max_chain_depth: u32,
    pub total_acquisitions: u64,
}

/// Main priority inheritance manager
pub struct CoopPriorityInherit {
    mutexes: BTreeMap<u64, PiMutex>,
    chains: Vec<PiChainEntry>,
    next_id: u64,
}

impl CoopPriorityInherit {
    pub fn new() -> Self { Self { mutexes: BTreeMap::new(), chains: Vec::new(), next_id: 1 } }

    pub fn create_mutex(&mut self) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.mutexes.insert(id, PiMutex::new(id));
        id
    }

    pub fn lock(&mut self, mutex_id: u64, tid: u64, prio: ThreadPriority) -> bool {
        if let Some(m) = self.mutexes.get_mut(&mutex_id) { m.lock(tid, prio) } else { false }
    }

    pub fn stats(&self) -> PriorityInheritStats {
        let boosts: u64 = self.mutexes.values().map(|m| m.boost_count).sum();
        let acqs: u64 = self.mutexes.values().map(|m| m.acquisitions).sum();
        PriorityInheritStats { total_mutexes: self.mutexes.len() as u32, active_chains: self.chains.len() as u32, total_boosts: boosts, max_chain_depth: 0, total_acquisitions: acqs }
    }
}
