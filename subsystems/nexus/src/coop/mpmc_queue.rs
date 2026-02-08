// SPDX-License-Identifier: GPL-2.0
//! Coop mpmc_queue — multi-producer multi-consumer queue.

extern crate alloc;

use alloc::vec::Vec;

/// Queue state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpmcState {
    Active,
    Draining,
    Closed,
}

/// Queue entry
#[derive(Debug, Clone)]
pub struct MpmcEntry {
    pub seq: u64,
    pub data_hash: u64,
    pub producer_id: u64,
    pub timestamp: u64,
}

/// MPMC queue instance
#[derive(Debug)]
pub struct MpmcQueue {
    pub id: u64,
    pub state: MpmcState,
    pub buffer: Vec<MpmcEntry>,
    pub capacity: usize,
    pub head: u64,
    pub tail: u64,
    pub producers: u32,
    pub consumers: u32,
    pub enqueue_count: u64,
    pub dequeue_count: u64,
    pub full_count: u64,
    pub empty_count: u64,
}

impl MpmcQueue {
    pub fn new(id: u64, capacity: usize) -> Self {
        Self { id, state: MpmcState::Active, buffer: Vec::new(), capacity, head: 0, tail: 0, producers: 0, consumers: 0, enqueue_count: 0, dequeue_count: 0, full_count: 0, empty_count: 0 }
    }

    pub fn enqueue(&mut self, data_hash: u64, producer: u64, now: u64) -> bool {
        if self.buffer.len() >= self.capacity { self.full_count += 1; return false; }
        self.tail += 1;
        self.buffer.push(MpmcEntry { seq: self.tail, data_hash, producer_id: producer, timestamp: now });
        self.enqueue_count += 1;
        true
    }

    pub fn dequeue(&mut self) -> Option<MpmcEntry> {
        if self.buffer.is_empty() { self.empty_count += 1; return None; }
        self.head += 1;
        self.dequeue_count += 1;
        Some(self.buffer.remove(0))
    }

    pub fn len(&self) -> usize { self.buffer.len() }
    pub fn utilization(&self) -> f64 { self.buffer.len() as f64 / self.capacity as f64 }
    pub fn is_empty(&self) -> bool { self.buffer.is_empty() }
}

/// Stats
#[derive(Debug, Clone)]
pub struct MpmcQueueStats {
    pub total_queues: u32,
    pub total_entries: u32,
    pub total_enqueued: u64,
    pub total_dequeued: u64,
    pub total_full_events: u64,
    pub avg_utilization: f64,
}

/// Main MPMC queue manager
pub struct CoopMpmcQueue {
    queues: Vec<MpmcQueue>,
    next_id: u64,
}

impl CoopMpmcQueue {
    pub fn new() -> Self { Self { queues: Vec::new(), next_id: 1 } }

    pub fn create(&mut self, capacity: usize) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.queues.push(MpmcQueue::new(id, capacity));
        id
    }

    pub fn stats(&self) -> MpmcQueueStats {
        let entries: u32 = self.queues.iter().map(|q| q.len() as u32).sum();
        let enqueued: u64 = self.queues.iter().map(|q| q.enqueue_count).sum();
        let dequeued: u64 = self.queues.iter().map(|q| q.dequeue_count).sum();
        let full: u64 = self.queues.iter().map(|q| q.full_count).sum();
        let utils: Vec<f64> = self.queues.iter().map(|q| q.utilization()).collect();
        let avg = if utils.is_empty() { 0.0 } else { utils.iter().sum::<f64>() / utils.len() as f64 };
        MpmcQueueStats { total_queues: self.queues.len() as u32, total_entries: entries, total_enqueued: enqueued, total_dequeued: dequeued, total_full_events: full, avg_utilization: avg }
    }
}
