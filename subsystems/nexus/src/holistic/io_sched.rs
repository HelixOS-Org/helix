// SPDX-License-Identifier: GPL-2.0
//! Holistic io_sched — I/O scheduler with multiple policies.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// I/O scheduler policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoSchedPolicy {
    Noop,
    Deadline,
    Cfq,
    Bfq,
    Mq,
    Kyber,
}

/// I/O priority class
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IoPrioClass {
    RealTime,
    BestEffort,
    Idle,
}

/// I/O request
#[derive(Debug)]
pub struct IoRequest {
    pub id: u64,
    pub sector: u64,
    pub nr_sectors: u32,
    pub is_write: bool,
    pub prio_class: IoPrioClass,
    pub prio_level: u8,
    pub submit_time: u64,
    pub deadline_ns: u64,
    pub completed: bool,
}

impl IoRequest {
    pub fn new(id: u64, sector: u64, nr_sectors: u32, is_write: bool, prio: IoPrioClass, level: u8, now: u64) -> Self {
        let deadline = now + if is_write { 5_000_000_000 } else { 500_000_000 };
        Self { id, sector, nr_sectors, is_write, prio_class: prio, prio_level: level, submit_time: now, deadline_ns: deadline, completed: false }
    }
}

/// Scheduler queue
#[derive(Debug)]
#[repr(align(64))]
pub struct SchedQueue {
    pub requests: VecDeque<IoRequest>,
    pub policy: IoSchedPolicy,
    pub dispatched: u64,
    pub merged: u64,
}

impl SchedQueue {
    pub fn new(policy: IoSchedPolicy) -> Self { Self { requests: VecDeque::new(), policy, dispatched: 0, merged: 0 } }

    #[inline(always)]
    pub fn enqueue(&mut self, req: IoRequest) { self.requests.push_back(req); }

    #[inline]
    pub fn dispatch(&mut self) -> Option<IoRequest> {
        if self.requests.is_empty() { return None; }
        self.dispatched += 1;
        match self.policy {
            IoSchedPolicy::Deadline => { self.requests.sort_by_key(|r| r.deadline_ns); self.requests.pop_front() }
            IoSchedPolicy::Noop => self.requests.pop_front(),
            _ => { self.requests.sort_by_key(|r| r.sector); self.requests.pop_front() }
        }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct IoSchedStats {
    pub policy: IoSchedPolicy,
    pub pending_requests: u32,
    pub total_dispatched: u64,
    pub total_merged: u64,
}

/// Main holistic I/O scheduler
pub struct HolisticIoSched {
    queues: BTreeMap<u32, SchedQueue>,
}

impl HolisticIoSched {
    pub fn new() -> Self { Self { queues: BTreeMap::new() } }
    #[inline(always)]
    pub fn add_queue(&mut self, id: u32, policy: IoSchedPolicy) { self.queues.insert(id, SchedQueue::new(policy)); }
    #[inline(always)]
    pub fn enqueue(&mut self, queue_id: u32, req: IoRequest) { if let Some(q) = self.queues.get_mut(&queue_id) { q.enqueue(req); } }
    #[inline(always)]
    pub fn dispatch(&mut self, queue_id: u32) -> Option<IoRequest> { self.queues.get_mut(&queue_id).and_then(|q| q.dispatch()) }

    #[inline(always)]
    pub fn stats(&self) -> Vec<IoSchedStats> {
        self.queues.values().map(|q| IoSchedStats { policy: q.policy, pending_requests: q.requests.len() as u32, total_dispatched: q.dispatched, total_merged: q.merged }).collect()
    }
}
