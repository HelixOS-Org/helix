// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — Michael-Scott Queue (lock-free FIFO queue)

extern crate alloc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsQueueOpResult {
    Success,
    Empty,
    Contended,
    Helped,
}

#[derive(Debug, Clone)]
pub struct MsQueueNode {
    pub value: u64,
    pub next: Option<u64>,
    pub enqueue_ts: u64,
}

impl MsQueueNode {
    pub fn new(value: u64, ts: u64) -> Self {
        Self { value, next: None, enqueue_ts: ts }
    }
}

#[derive(Debug, Clone)]
pub struct MsQueueState {
    pub head_tag: u64,
    pub tail_tag: u64,
    pub size: u64,
    pub enqueue_count: u64,
    pub dequeue_count: u64,
    pub help_advance_count: u64,
    pub cas_failures: u64,
    pub max_size: u64,
    pub total_latency_ns: u64,
}

impl MsQueueState {
    pub fn new() -> Self {
        Self {
            head_tag: 0, tail_tag: 0, size: 0,
            enqueue_count: 0, dequeue_count: 0,
            help_advance_count: 0, cas_failures: 0,
            max_size: 0, total_latency_ns: 0,
        }
    }

    pub fn enqueue(&mut self) {
        self.size += 1;
        self.enqueue_count += 1;
        self.tail_tag += 1;
        if self.size > self.max_size { self.max_size = self.size; }
    }

    pub fn dequeue(&mut self, latency_ns: u64) -> bool {
        if self.size == 0 { return false; }
        self.size -= 1;
        self.dequeue_count += 1;
        self.head_tag += 1;
        self.total_latency_ns += latency_ns;
        true
    }

    pub fn avg_latency_ns(&self) -> u64 {
        if self.dequeue_count == 0 { 0 } else { self.total_latency_ns / self.dequeue_count }
    }

    pub fn contention_rate(&self) -> u64 {
        let total = self.enqueue_count + self.dequeue_count;
        if total == 0 { 0 } else { (self.cas_failures * 100) / total }
    }
}

#[derive(Debug, Clone)]
pub struct MsQueueStats {
    pub total_queues: u64,
    pub total_enqueues: u64,
    pub total_dequeues: u64,
    pub total_cas_failures: u64,
    pub total_helps: u64,
}

pub struct CoopMichaelScottQueue {
    queues: Vec<MsQueueState>,
    stats: MsQueueStats,
}

impl CoopMichaelScottQueue {
    pub fn new() -> Self {
        Self {
            queues: Vec::new(),
            stats: MsQueueStats {
                total_queues: 0, total_enqueues: 0,
                total_dequeues: 0, total_cas_failures: 0,
                total_helps: 0,
            },
        }
    }

    pub fn create_queue(&mut self) -> usize {
        let idx = self.queues.len();
        self.queues.push(MsQueueState::new());
        self.stats.total_queues += 1;
        idx
    }

    pub fn enqueue(&mut self, queue_idx: usize) {
        if let Some(q) = self.queues.get_mut(queue_idx) {
            q.enqueue();
            self.stats.total_enqueues += 1;
        }
    }

    pub fn dequeue(&mut self, queue_idx: usize, latency_ns: u64) -> MsQueueOpResult {
        if let Some(q) = self.queues.get_mut(queue_idx) {
            if q.dequeue(latency_ns) {
                self.stats.total_dequeues += 1;
                MsQueueOpResult::Success
            } else { MsQueueOpResult::Empty }
        } else { MsQueueOpResult::Empty }
    }

    pub fn stats(&self) -> &MsQueueStats { &self.stats }
}
