// SPDX-License-Identifier: GPL-2.0
//! Coop bounded_queue — bounded MPMC queue with backpressure.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Queue state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundedQueueState {
    Empty,
    Partial,
    Full,
    Closed,
}

/// Queue item
#[derive(Debug)]
pub struct QueueItem {
    pub id: u64,
    pub priority: u8,
    pub data_hash: u64,
    pub enqueued_at: u64,
    pub size: u32,
}

/// Bounded queue
#[derive(Debug)]
pub struct BoundedQueue {
    pub id: u64,
    pub capacity: u32,
    pub items: Vec<QueueItem>,
    pub state: BoundedQueueState,
    pub total_enqueued: u64,
    pub total_dequeued: u64,
    pub total_dropped: u64,
    pub backpressure_count: u64,
}

impl BoundedQueue {
    pub fn new(id: u64, cap: u32) -> Self {
        Self { id, capacity: cap, items: Vec::new(), state: BoundedQueueState::Empty, total_enqueued: 0, total_dequeued: 0, total_dropped: 0, backpressure_count: 0 }
    }

    pub fn enqueue(&mut self, item: QueueItem) -> bool {
        if self.items.len() >= self.capacity as usize {
            self.backpressure_count += 1;
            self.total_dropped += 1;
            return false;
        }
        self.items.push(item);
        self.total_enqueued += 1;
        self.update_state();
        true
    }

    pub fn dequeue(&mut self) -> Option<QueueItem> {
        if self.items.is_empty() { return None; }
        let item = self.items.remove(0);
        self.total_dequeued += 1;
        self.update_state();
        Some(item)
    }

    fn update_state(&mut self) {
        if self.items.is_empty() { self.state = BoundedQueueState::Empty; }
        else if self.items.len() >= self.capacity as usize { self.state = BoundedQueueState::Full; }
        else { self.state = BoundedQueueState::Partial; }
    }

    pub fn close(&mut self) { self.state = BoundedQueueState::Closed; }

    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 { return 0.0; }
        self.items.len() as f64 / self.capacity as f64
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct BoundedQueueStats {
    pub total_queues: u32,
    pub total_enqueued: u64,
    pub total_dequeued: u64,
    pub total_dropped: u64,
    pub avg_utilization: f64,
}

/// Main coop bounded queue manager
pub struct CoopBoundedQueue {
    queues: BTreeMap<u64, BoundedQueue>,
    next_id: u64,
}

impl CoopBoundedQueue {
    pub fn new() -> Self { Self { queues: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self, cap: u32) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.queues.insert(id, BoundedQueue::new(id, cap));
        id
    }

    pub fn enqueue(&mut self, qid: u64, item: QueueItem) -> bool {
        if let Some(q) = self.queues.get_mut(&qid) { q.enqueue(item) }
        else { false }
    }

    pub fn dequeue(&mut self, qid: u64) -> Option<QueueItem> {
        if let Some(q) = self.queues.get_mut(&qid) { q.dequeue() }
        else { None }
    }

    pub fn destroy(&mut self, qid: u64) { self.queues.remove(&qid); }

    pub fn stats(&self) -> BoundedQueueStats {
        let enq: u64 = self.queues.values().map(|q| q.total_enqueued).sum();
        let deq: u64 = self.queues.values().map(|q| q.total_dequeued).sum();
        let drop: u64 = self.queues.values().map(|q| q.total_dropped).sum();
        let avg = if self.queues.is_empty() { 0.0 }
            else { self.queues.values().map(|q| q.utilization()).sum::<f64>() / self.queues.len() as f64 };
        BoundedQueueStats { total_queues: self.queues.len() as u32, total_enqueued: enq, total_dequeued: deq, total_dropped: drop, avg_utilization: avg }
    }
}
