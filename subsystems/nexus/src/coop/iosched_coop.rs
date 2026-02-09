// SPDX-License-Identifier: GPL-2.0
//! Coop IO scheduler — cooperative IO scheduling with priority donation

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Coop IO priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoopIoPrio {
    RealTime,
    High,
    Normal,
    Low,
    Idle,
}

/// Coop IO request
#[derive(Debug, Clone)]
pub struct CoopIoRequest {
    pub id: u64,
    pub owner_id: u64,
    pub priority: CoopIoPrio,
    pub sector: u64,
    pub nr_sectors: u32,
    pub is_write: bool,
    pub donated_prio: Option<CoopIoPrio>,
    pub enqueue_ns: u64,
    pub dispatch_ns: u64,
}

impl CoopIoRequest {
    pub fn new(id: u64, owner_id: u64, prio: CoopIoPrio) -> Self {
        Self {
            id,
            owner_id,
            priority: prio,
            sector: 0,
            nr_sectors: 0,
            is_write: false,
            donated_prio: None,
            enqueue_ns: 0,
            dispatch_ns: 0,
        }
    }

    #[inline]
    pub fn donate_priority(&mut self, prio: CoopIoPrio) {
        if prio < self.priority {
            self.donated_prio = Some(prio);
        }
    }

    #[inline(always)]
    pub fn effective_priority(&self) -> CoopIoPrio {
        self.donated_prio.unwrap_or(self.priority)
    }

    #[inline(always)]
    pub fn wait_ns(&self) -> u64 {
        self.dispatch_ns.saturating_sub(self.enqueue_ns)
    }
}

/// Coop IO scheduler stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CoopIoSchedStats {
    pub total_requests: u64,
    pub dispatched: u64,
    pub donations: u64,
    pub total_wait_ns: u64,
}

/// Main coop IO scheduler
#[derive(Debug)]
pub struct CoopIoSched {
    pub queues: BTreeMap<CoopIoPrio, Vec<CoopIoRequest>>,
    pub stats: CoopIoSchedStats,
}

impl CoopIoSched {
    pub fn new() -> Self {
        Self {
            queues: BTreeMap::new(),
            stats: CoopIoSchedStats {
                total_requests: 0,
                dispatched: 0,
                donations: 0,
                total_wait_ns: 0,
            },
        }
    }

    #[inline]
    pub fn enqueue(&mut self, req: CoopIoRequest) {
        self.stats.total_requests += 1;
        if req.donated_prio.is_some() {
            self.stats.donations += 1;
        }
        let prio = req.effective_priority();
        self.queues.entry(prio).or_insert_with(Vec::new).push(req);
    }

    #[inline]
    pub fn dispatch(&mut self) -> Option<CoopIoRequest> {
        for (_prio, queue) in self.queues.iter_mut() {
            if !queue.is_empty() {
                let req = queue.pop_front().unwrap();
                self.stats.dispatched += 1;
                self.stats.total_wait_ns += req.wait_ns();
                return Some(req);
            }
        }
        None
    }

    #[inline]
    pub fn avg_wait_ns(&self) -> u64 {
        if self.stats.dispatched == 0 {
            0
        } else {
            self.stats.total_wait_ns / self.stats.dispatched
        }
    }
}
