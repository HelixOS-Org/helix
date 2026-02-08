// SPDX-License-Identifier: GPL-2.0
//! Holistic IO scheduler — multi-queue block IO scheduling with priorities

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// IO scheduler type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoSchedType {
    None,
    Mq,
    Bfq,
    Kyber,
    Deadline,
    Custom,
}

/// IO priority class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoPrioClass {
    RealTime,
    BestEffort,
    Idle,
    None,
}

/// IO request type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoReqType {
    Read,
    Write,
    Flush,
    Discard,
    Passthrough,
}

/// IO scheduler queue
#[derive(Debug, Clone)]
pub struct IoSchedQueue {
    pub queue_id: u32,
    pub pending: u32,
    pub dispatched: u64,
    pub completed: u64,
    pub merged: u64,
    pub requeued: u64,
    pub avg_latency_ns: u64,
    pub total_latency_ns: u64,
}

impl IoSchedQueue {
    pub fn new(queue_id: u32) -> Self {
        Self { queue_id, pending: 0, dispatched: 0, completed: 0, merged: 0, requeued: 0, avg_latency_ns: 0, total_latency_ns: 0 }
    }

    pub fn enqueue(&mut self) { self.pending += 1; }
    pub fn dispatch(&mut self) { self.pending = self.pending.saturating_sub(1); self.dispatched += 1; }
    pub fn complete(&mut self, latency_ns: u64) {
        self.completed += 1;
        self.total_latency_ns += latency_ns;
        if self.completed > 0 { self.avg_latency_ns = self.total_latency_ns / self.completed; }
    }

    pub fn merge(&mut self) { self.merged += 1; }
    pub fn requeue(&mut self) { self.requeued += 1; self.pending += 1; }
    pub fn utilization(&self) -> f64 { self.pending as f64 }
}

/// BFQ budget accounting
#[derive(Debug, Clone)]
pub struct BfqBudget {
    pub pid: u32,
    pub weight: u32,
    pub budget_sectors: u64,
    pub used_sectors: u64,
    pub slices: u64,
}

impl BfqBudget {
    pub fn new(pid: u32, weight: u32) -> Self {
        Self { pid, weight, budget_sectors: 0, used_sectors: 0, slices: 0 }
    }

    pub fn charge(&mut self, sectors: u64) { self.used_sectors += sectors; }
    pub fn new_slice(&mut self, budget: u64) { self.budget_sectors = budget; self.used_sectors = 0; self.slices += 1; }
    pub fn remaining(&self) -> u64 { self.budget_sectors.saturating_sub(self.used_sectors) }
}

/// IO scheduler holistic stats
#[derive(Debug, Clone)]
pub struct HolisticIoSchedStats {
    pub total_requests: u64,
    pub total_merges: u64,
    pub total_dispatched: u64,
    pub total_completed: u64,
    pub avg_queue_depth: f64,
}

/// Main holistic IO scheduler
#[derive(Debug)]
pub struct HolisticIoScheduler {
    pub sched_type: IoSchedType,
    pub queues: BTreeMap<u32, IoSchedQueue>,
    pub bfq_budgets: BTreeMap<u32, BfqBudget>,
    pub stats: HolisticIoSchedStats,
}

impl HolisticIoScheduler {
    pub fn new(sched_type: IoSchedType) -> Self {
        Self {
            sched_type,
            queues: BTreeMap::new(),
            bfq_budgets: BTreeMap::new(),
            stats: HolisticIoSchedStats { total_requests: 0, total_merges: 0, total_dispatched: 0, total_completed: 0, avg_queue_depth: 0.0 },
        }
    }

    pub fn add_queue(&mut self, queue_id: u32) {
        self.queues.insert(queue_id, IoSchedQueue::new(queue_id));
    }

    pub fn submit(&mut self, queue_id: u32) {
        if let Some(q) = self.queues.get_mut(&queue_id) {
            q.enqueue();
            self.stats.total_requests += 1;
        }
    }

    pub fn dispatch(&mut self, queue_id: u32) {
        if let Some(q) = self.queues.get_mut(&queue_id) {
            q.dispatch();
            self.stats.total_dispatched += 1;
        }
    }

    pub fn complete(&mut self, queue_id: u32, latency_ns: u64) {
        if let Some(q) = self.queues.get_mut(&queue_id) {
            q.complete(latency_ns);
            self.stats.total_completed += 1;
        }
    }
}
