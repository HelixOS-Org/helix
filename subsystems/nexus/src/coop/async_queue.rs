// SPDX-License-Identifier: GPL-2.0
//! Coop async_queue — async-safe multi-producer multi-consumer queue.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Queue ordering mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueOrder {
    Fifo,
    Lifo,
    Priority,
}

/// Queue item priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QueuePriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Queue item state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemState {
    Queued,
    Processing,
    Done,
    Cancelled,
}

/// Queue item
#[derive(Debug, Clone)]
pub struct QueueItem {
    pub id: u64,
    pub priority: QueuePriority,
    pub state: ItemState,
    pub payload_size: usize,
    pub producer_id: u32,
    pub consumer_id: Option<u32>,
    pub enqueue_time: u64,
    pub dequeue_time: u64,
    pub complete_time: u64,
}

impl QueueItem {
    pub fn new(id: u64, priority: QueuePriority, payload_size: usize, producer: u32, now: u64) -> Self {
        Self {
            id, priority, state: ItemState::Queued, payload_size,
            producer_id: producer, consumer_id: None,
            enqueue_time: now, dequeue_time: 0, complete_time: 0,
        }
    }

    pub fn wait_time(&self) -> u64 {
        if self.dequeue_time > 0 { self.dequeue_time - self.enqueue_time } else { 0 }
    }

    pub fn process_time(&self) -> u64 {
        if self.complete_time > 0 && self.dequeue_time > 0 {
            self.complete_time - self.dequeue_time
        } else { 0 }
    }
}

/// Async queue instance
#[derive(Debug)]
pub struct AsyncQueueInstance {
    pub id: u64,
    pub order: QueueOrder,
    pub items: Vec<QueueItem>,
    pub capacity: usize,
    pub producer_count: u32,
    pub consumer_count: u32,
    pub total_enqueued: u64,
    pub total_dequeued: u64,
    pub total_dropped: u64,
    pub total_wait_ns: u64,
    pub peak_depth: usize,
    pub blocked_producers: Vec<u32>,
    pub blocked_consumers: Vec<u32>,
}

impl AsyncQueueInstance {
    pub fn new(id: u64, capacity: usize, order: QueueOrder) -> Self {
        Self {
            id, order, items: Vec::new(), capacity,
            producer_count: 0, consumer_count: 0,
            total_enqueued: 0, total_dequeued: 0, total_dropped: 0,
            total_wait_ns: 0, peak_depth: 0,
            blocked_producers: Vec::new(), blocked_consumers: Vec::new(),
        }
    }

    pub fn enqueue(&mut self, item: QueueItem) -> bool {
        if self.items.len() >= self.capacity {
            self.total_dropped += 1;
            return false;
        }
        self.total_enqueued += 1;
        self.items.push(item);
        if self.items.len() > self.peak_depth { self.peak_depth = self.items.len(); }

        if self.order == QueueOrder::Priority {
            self.items.sort_by(|a, b| b.priority.cmp(&a.priority));
        }
        true
    }

    pub fn dequeue(&mut self, consumer_id: u32, now: u64) -> Option<QueueItem> {
        if self.items.is_empty() { return None; }
        let idx = match self.order {
            QueueOrder::Fifo | QueueOrder::Priority => 0,
            QueueOrder::Lifo => self.items.len() - 1,
        };
        let mut item = self.items.remove(idx);
        item.state = ItemState::Processing;
        item.consumer_id = Some(consumer_id);
        item.dequeue_time = now;
        self.total_dequeued += 1;
        self.total_wait_ns += item.wait_time();
        Some(item)
    }

    pub fn depth(&self) -> usize { self.items.len() }
    pub fn is_full(&self) -> bool { self.items.len() >= self.capacity }
    pub fn is_empty(&self) -> bool { self.items.is_empty() }

    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 { return 0.0; }
        self.items.len() as f64 / self.capacity as f64
    }

    pub fn avg_wait_ns(&self) -> u64 {
        if self.total_dequeued == 0 { 0 } else { self.total_wait_ns / self.total_dequeued }
    }

    pub fn drop_rate(&self) -> f64 {
        let total = self.total_enqueued + self.total_dropped;
        if total == 0 { return 0.0; }
        self.total_dropped as f64 / total as f64
    }
}

/// Async queue stats
#[derive(Debug, Clone)]
pub struct AsyncQueueStats {
    pub total_queues: u32,
    pub total_enqueued: u64,
    pub total_dequeued: u64,
    pub total_dropped: u64,
    pub total_items_pending: u64,
}

/// Main async queue manager
pub struct CoopAsyncQueue {
    queues: BTreeMap<u64, AsyncQueueInstance>,
    next_id: u64,
}

impl CoopAsyncQueue {
    pub fn new() -> Self {
        Self { queues: BTreeMap::new(), next_id: 1 }
    }

    pub fn create_queue(&mut self, capacity: usize, order: QueueOrder) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.queues.insert(id, AsyncQueueInstance::new(id, capacity, order));
        id
    }

    pub fn enqueue(&mut self, queue_id: u64, item: QueueItem) -> bool {
        self.queues.get_mut(&queue_id).map(|q| q.enqueue(item)).unwrap_or(false)
    }

    pub fn dequeue(&mut self, queue_id: u64, consumer_id: u32, now: u64) -> Option<QueueItem> {
        self.queues.get_mut(&queue_id)?.dequeue(consumer_id, now)
    }

    pub fn destroy_queue(&mut self, queue_id: u64) -> bool {
        self.queues.remove(&queue_id).is_some()
    }

    pub fn busiest_queues(&self, n: usize) -> Vec<(u64, usize)> {
        let mut v: Vec<_> = self.queues.iter()
            .map(|(&id, q)| (id, q.depth()))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(n);
        v
    }

    pub fn stats(&self) -> AsyncQueueStats {
        AsyncQueueStats {
            total_queues: self.queues.len() as u32,
            total_enqueued: self.queues.values().map(|q| q.total_enqueued).sum(),
            total_dequeued: self.queues.values().map(|q| q.total_dequeued).sum(),
            total_dropped: self.queues.values().map(|q| q.total_dropped).sum(),
            total_items_pending: self.queues.values().map(|q| q.depth() as u64).sum(),
        }
    }
}
