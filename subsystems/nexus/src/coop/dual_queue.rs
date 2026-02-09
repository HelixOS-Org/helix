// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — Dual Queue (synchronous rendez-vous queue)

extern crate alloc;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DualQueueNodeType {
    Data(u64),
    Request,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DualQueueNodeState {
    Pending,
    Matched,
    Cancelled,
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DualQueueNode {
    pub node_type: DualQueueNodeType,
    pub state: DualQueueNodeState,
    pub thread_id: u32,
    pub result: Option<u64>,
    pub timestamp: u64,
}

impl DualQueueNode {
    #[inline]
    pub fn data(thread_id: u32, value: u64, ts: u64) -> Self {
        Self {
            node_type: DualQueueNodeType::Data(value),
            state: DualQueueNodeState::Pending,
            thread_id, result: None, timestamp: ts,
        }
    }

    #[inline]
    pub fn request(thread_id: u32, ts: u64) -> Self {
        Self {
            node_type: DualQueueNodeType::Request,
            state: DualQueueNodeState::Pending,
            thread_id, result: None, timestamp: ts,
        }
    }

    #[inline(always)]
    pub fn fulfill(&mut self, value: u64) {
        self.state = DualQueueNodeState::Matched;
        self.result = Some(value);
    }

    #[inline(always)]
    pub fn cancel(&mut self) {
        self.state = DualQueueNodeState::Cancelled;
    }

    #[inline(always)]
    pub fn is_data(&self) -> bool { matches!(self.node_type, DualQueueNodeType::Data(_)) }
    #[inline(always)]
    pub fn is_request(&self) -> bool { matches!(self.node_type, DualQueueNodeType::Request) }
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DualQueueState {
    pub queue: Vec<DualQueueNode>,
    pub total_data: u64,
    pub total_requests: u64,
    pub total_matches: u64,
    pub total_cancels: u64,
    pub total_latency_ns: u64,
}

impl DualQueueState {
    pub fn new() -> Self {
        Self {
            queue: Vec::new(),
            total_data: 0, total_requests: 0,
            total_matches: 0, total_cancels: 0,
            total_latency_ns: 0,
        }
    }

    pub fn put(&mut self, thread_id: u32, value: u64, ts: u64) -> bool {
        self.total_data += 1;
        // Try to match with pending request
        for node in self.queue.iter_mut() {
            if node.is_request() && node.state == DualQueueNodeState::Pending {
                node.fulfill(value);
                self.total_matches += 1;
                if ts > node.timestamp {
                    self.total_latency_ns += ts - node.timestamp;
                }
                return true;
            }
        }
        // No match, enqueue data
        self.queue.push(DualQueueNode::data(thread_id, value, ts));
        false
    }

    pub fn take(&mut self, thread_id: u32, ts: u64) -> Option<u64> {
        self.total_requests += 1;
        // Try to match with pending data
        for node in self.queue.iter_mut() {
            if node.is_data() && node.state == DualQueueNodeState::Pending {
                if let DualQueueNodeType::Data(v) = node.node_type {
                    node.state = DualQueueNodeState::Matched;
                    self.total_matches += 1;
                    return Some(v);
                }
            }
        }
        self.queue.push(DualQueueNode::request(thread_id, ts));
        None
    }

    #[inline(always)]
    pub fn cleanup(&mut self) {
        self.queue.retain(|n| n.state == DualQueueNodeState::Pending);
    }

    #[inline(always)]
    pub fn avg_match_latency_ns(&self) -> u64 {
        if self.total_matches == 0 { 0 } else { self.total_latency_ns / self.total_matches }
    }

    #[inline(always)]
    pub fn match_rate(&self) -> u64 {
        let total = self.total_data + self.total_requests;
        if total == 0 { 0 } else { (self.total_matches * 200) / total } // x2 because each match takes 2
    }
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DualQueueStats {
    pub total_queues: u64,
    pub total_data: u64,
    pub total_requests: u64,
    pub total_matches: u64,
}

#[repr(align(64))]
pub struct CoopDualQueue {
    queues: Vec<DualQueueState>,
    stats: DualQueueStats,
}

impl CoopDualQueue {
    pub fn new() -> Self {
        Self {
            queues: Vec::new(),
            stats: DualQueueStats {
                total_queues: 0, total_data: 0,
                total_requests: 0, total_matches: 0,
            },
        }
    }

    #[inline]
    pub fn create_queue(&mut self) -> usize {
        let idx = self.queues.len();
        self.queues.push(DualQueueState::new());
        self.stats.total_queues += 1;
        idx
    }

    #[inline]
    pub fn put(&mut self, queue_idx: usize, thread_id: u32, value: u64, ts: u64) {
        if let Some(q) = self.queues.get_mut(queue_idx) {
            q.put(thread_id, value, ts);
            self.stats.total_data += 1;
        }
    }

    #[inline]
    pub fn take(&mut self, queue_idx: usize, thread_id: u32, ts: u64) -> Option<u64> {
        if let Some(q) = self.queues.get_mut(queue_idx) {
            self.stats.total_requests += 1;
            let result = q.take(thread_id, ts);
            if result.is_some() { self.stats.total_matches += 1; }
            result
        } else { None }
    }

    #[inline(always)]
    pub fn stats(&self) -> &DualQueueStats { &self.stats }
}
