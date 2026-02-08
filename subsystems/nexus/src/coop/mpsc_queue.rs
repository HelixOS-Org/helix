// SPDX-License-Identifier: GPL-2.0
//! Coop mpsc_queue — multi-producer single-consumer lock-free queue.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Queue overflow policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowAction {
    /// Block producer until space available
    Block,
    /// Drop oldest message
    DropOldest,
    /// Drop the new message
    DropNewest,
    /// Expand capacity dynamically
    Expand,
}

/// Message priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MsgPriority {
    Bulk = 0,
    Normal = 1,
    High = 2,
    Realtime = 3,
}

/// A message in the queue
#[derive(Debug, Clone)]
pub struct QueueMsg {
    pub seq: u64,
    pub producer_id: u64,
    pub payload_size: usize,
    pub payload_hash: u64,
    pub priority: MsgPriority,
    pub enqueue_ns: u64,
}

impl QueueMsg {
    pub fn new(seq: u64, producer: u64, size: usize, now_ns: u64) -> Self {
        Self {
            seq,
            producer_id: producer,
            payload_size: size,
            payload_hash: 0,
            priority: MsgPriority::Normal,
            enqueue_ns: now_ns,
        }
    }

    pub fn latency_ns(&self, now_ns: u64) -> u64 {
        now_ns.saturating_sub(self.enqueue_ns)
    }
}

/// Producer handle
#[derive(Debug)]
pub struct Producer {
    pub id: u64,
    pub pid: u64,
    pub enqueued: u64,
    pub blocked_count: u64,
    pub dropped_count: u64,
    pub bytes_sent: u64,
}

impl Producer {
    pub fn new(id: u64, pid: u64) -> Self {
        Self { id, pid, enqueued: 0, blocked_count: 0, dropped_count: 0, bytes_sent: 0 }
    }

    pub fn record_enqueue(&mut self, size: usize) {
        self.enqueued += 1;
        self.bytes_sent += size as u64;
    }

    pub fn record_drop(&mut self) {
        self.dropped_count += 1;
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.enqueued + self.dropped_count;
        if total == 0 { return 1.0; }
        self.enqueued as f64 / total as f64
    }
}

/// Consumer handle
#[derive(Debug)]
pub struct Consumer {
    pub id: u64,
    pub pid: u64,
    pub dequeued: u64,
    pub empty_polls: u64,
    pub bytes_received: u64,
    pub total_latency_ns: u64,
}

impl Consumer {
    pub fn new(id: u64, pid: u64) -> Self {
        Self { id, pid, dequeued: 0, empty_polls: 0, bytes_received: 0, total_latency_ns: 0 }
    }

    pub fn record_dequeue(&mut self, size: usize, latency_ns: u64) {
        self.dequeued += 1;
        self.bytes_received += size as u64;
        self.total_latency_ns += latency_ns;
    }

    pub fn avg_latency_ns(&self) -> f64 {
        if self.dequeued == 0 { return 0.0; }
        self.total_latency_ns as f64 / self.dequeued as f64
    }

    pub fn empty_rate(&self) -> f64 {
        let total = self.dequeued + self.empty_polls;
        if total == 0 { return 0.0; }
        self.empty_polls as f64 / total as f64
    }
}

/// An MPSC queue instance
#[derive(Debug)]
pub struct MpscInstance {
    pub id: u64,
    pub name: String,
    pub capacity: usize,
    pub overflow_action: OverflowAction,
    queue: Vec<QueueMsg>,
    producers: Vec<Producer>,
    consumer: Option<Consumer>,
    next_seq: u64,
    pub total_enqueues: u64,
    pub total_dequeues: u64,
    pub total_drops: u64,
    pub total_bytes: u64,
    pub peak_depth: usize,
    pub watermark_high: usize,
    pub watermark_low: usize,
    pub high_watermark_events: u64,
}

impl MpscInstance {
    pub fn new(id: u64, name: String, capacity: usize) -> Self {
        let wh = capacity * 80 / 100;
        let wl = capacity * 20 / 100;
        Self {
            id,
            name,
            capacity,
            overflow_action: OverflowAction::DropNewest,
            queue: Vec::new(),
            producers: Vec::new(),
            consumer: None,
            next_seq: 1,
            total_enqueues: 0,
            total_dequeues: 0,
            total_drops: 0,
            total_bytes: 0,
            peak_depth: 0,
            watermark_high: wh,
            watermark_low: wl,
            high_watermark_events: 0,
        }
    }

    pub fn add_producer(&mut self, prod_id: u64, pid: u64) {
        self.producers.push(Producer::new(prod_id, pid));
    }

    pub fn set_consumer(&mut self, cons_id: u64, pid: u64) {
        self.consumer = Some(Consumer::new(cons_id, pid));
    }

    pub fn enqueue(&mut self, producer_pid: u64, payload_size: usize, now_ns: u64) -> Option<u64> {
        if self.queue.len() >= self.capacity {
            match self.overflow_action {
                OverflowAction::DropNewest => {
                    if let Some(p) = self.producers.iter_mut().find(|p| p.pid == producer_pid) {
                        p.record_drop();
                    }
                    self.total_drops += 1;
                    return None;
                }
                OverflowAction::DropOldest => {
                    self.queue.remove(0);
                    self.total_drops += 1;
                }
                OverflowAction::Expand => {
                    self.capacity = self.capacity * 3 / 2;
                }
                OverflowAction::Block => {
                    if let Some(p) = self.producers.iter_mut().find(|p| p.pid == producer_pid) {
                        p.blocked_count += 1;
                    }
                    return None;
                }
            }
        }

        let seq = self.next_seq;
        self.next_seq += 1;
        let msg = QueueMsg::new(seq, producer_pid, payload_size, now_ns);
        self.queue.push(msg);

        if let Some(p) = self.producers.iter_mut().find(|p| p.pid == producer_pid) {
            p.record_enqueue(payload_size);
        }
        self.total_enqueues += 1;
        self.total_bytes += payload_size as u64;
        if self.queue.len() > self.peak_depth {
            self.peak_depth = self.queue.len();
        }
        if self.queue.len() >= self.watermark_high {
            self.high_watermark_events += 1;
        }
        Some(seq)
    }

    pub fn dequeue(&mut self, now_ns: u64) -> Option<QueueMsg> {
        if self.queue.is_empty() {
            if let Some(ref mut c) = self.consumer {
                c.empty_polls += 1;
            }
            return None;
        }
        let msg = self.queue.remove(0);
        let latency = now_ns.saturating_sub(msg.enqueue_ns);
        if let Some(ref mut c) = self.consumer {
            c.record_dequeue(msg.payload_size, latency);
        }
        self.total_dequeues += 1;
        Some(msg)
    }

    pub fn depth(&self) -> usize {
        self.queue.len()
    }

    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 { return 0.0; }
        self.queue.len() as f64 / self.capacity as f64
    }

    pub fn throughput_ratio(&self) -> f64 {
        if self.total_enqueues == 0 { return 0.0; }
        self.total_dequeues as f64 / self.total_enqueues as f64
    }

    pub fn producer_count(&self) -> usize {
        self.producers.len()
    }
}

/// MPSC stats
#[derive(Debug, Clone)]
pub struct MpscStats {
    pub total_queues: u64,
    pub total_enqueues: u64,
    pub total_dequeues: u64,
    pub total_drops: u64,
    pub total_bytes: u64,
}

/// Main MPSC queue manager
pub struct CoopMpscQueue {
    queues: BTreeMap<u64, MpscInstance>,
    next_id: u64,
    next_endpoint: u64,
    stats: MpscStats,
}

impl CoopMpscQueue {
    pub fn new() -> Self {
        Self {
            queues: BTreeMap::new(),
            next_id: 1,
            next_endpoint: 1,
            stats: MpscStats {
                total_queues: 0,
                total_enqueues: 0,
                total_dequeues: 0,
                total_drops: 0,
                total_bytes: 0,
            },
        }
    }

    pub fn create_queue(&mut self, name: String, capacity: usize) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.queues.insert(id, MpscInstance::new(id, name, capacity));
        self.stats.total_queues += 1;
        id
    }

    pub fn add_producer(&mut self, queue_id: u64, pid: u64) -> u64 {
        let eid = self.next_endpoint;
        self.next_endpoint += 1;
        if let Some(q) = self.queues.get_mut(&queue_id) {
            q.add_producer(eid, pid);
        }
        eid
    }

    pub fn set_consumer(&mut self, queue_id: u64, pid: u64) -> u64 {
        let eid = self.next_endpoint;
        self.next_endpoint += 1;
        if let Some(q) = self.queues.get_mut(&queue_id) {
            q.set_consumer(eid, pid);
        }
        eid
    }

    pub fn enqueue(&mut self, queue_id: u64, producer_pid: u64, size: usize, now_ns: u64) -> Option<u64> {
        if let Some(q) = self.queues.get_mut(&queue_id) {
            let result = q.enqueue(producer_pid, size, now_ns);
            if result.is_some() {
                self.stats.total_enqueues += 1;
                self.stats.total_bytes += size as u64;
            } else {
                self.stats.total_drops += 1;
            }
            result
        } else {
            None
        }
    }

    pub fn dequeue(&mut self, queue_id: u64, now_ns: u64) -> Option<QueueMsg> {
        if let Some(q) = self.queues.get_mut(&queue_id) {
            let msg = q.dequeue(now_ns);
            if msg.is_some() {
                self.stats.total_dequeues += 1;
            }
            msg
        } else {
            None
        }
    }

    pub fn fullest_queues(&self, top: usize) -> Vec<(u64, f64)> {
        let mut v: Vec<(u64, f64)> = self.queues.iter()
            .map(|(&id, q)| (id, q.utilization()))
            .collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        v.truncate(top);
        v
    }

    pub fn get_queue(&self, id: u64) -> Option<&MpscInstance> {
        self.queues.get(&id)
    }

    pub fn stats(&self) -> &MpscStats {
        &self.stats
    }
}
