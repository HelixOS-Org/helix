// SPDX-License-Identifier: GPL-2.0
//! Holistic blk_mq — multi-queue block I/O scheduling.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Block I/O operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlkIoOp {
    Read,
    Write,
    Flush,
    Discard,
    WriteZeros,
    SecureErase,
}

/// Hardware queue
#[derive(Debug)]
pub struct HwQueue {
    pub id: u32,
    pub cpu_affinity: u32,
    pub depth: u32,
    pub pending: u32,
    pub completed: u64,
    pub total_bytes: u64,
    pub total_latency_ns: u64,
}

impl HwQueue {
    pub fn new(id: u32, cpu: u32, depth: u32) -> Self {
        Self { id, cpu_affinity: cpu, depth, pending: 0, completed: 0, total_bytes: 0, total_latency_ns: 0 }
    }

    pub fn submit(&mut self) { self.pending += 1; }
    pub fn complete(&mut self, bytes: u64, lat: u64) { if self.pending > 0 { self.pending -= 1; } self.completed += 1; self.total_bytes += bytes; self.total_latency_ns += lat; }
    pub fn avg_latency_ns(&self) -> u64 { if self.completed == 0 { 0 } else { self.total_latency_ns / self.completed } }
    pub fn utilization(&self) -> f64 { if self.depth == 0 { 0.0 } else { self.pending as f64 / self.depth as f64 } }
}

/// Block request
#[derive(Debug)]
pub struct BlkRequest {
    pub id: u64,
    pub op: BlkIoOp,
    pub sector: u64,
    pub nr_sectors: u32,
    pub queue_id: u32,
    pub submit_time: u64,
}

/// Stats
#[derive(Debug, Clone)]
pub struct BlkMqStats {
    pub total_queues: u32,
    pub total_completed: u64,
    pub total_bytes: u64,
    pub avg_latency_ns: u64,
    pub avg_utilization: f64,
}

/// Main holistic blk_mq
pub struct HolisticBlkMq {
    queues: BTreeMap<u32, HwQueue>,
}

impl HolisticBlkMq {
    pub fn new() -> Self { Self { queues: BTreeMap::new() } }

    pub fn add_queue(&mut self, id: u32, cpu: u32, depth: u32) { self.queues.insert(id, HwQueue::new(id, cpu, depth)); }

    pub fn submit(&mut self, queue_id: u32) { if let Some(q) = self.queues.get_mut(&queue_id) { q.submit(); } }

    pub fn complete(&mut self, queue_id: u32, bytes: u64, lat: u64) {
        if let Some(q) = self.queues.get_mut(&queue_id) { q.complete(bytes, lat); }
    }

    pub fn stats(&self) -> BlkMqStats {
        let completed: u64 = self.queues.values().map(|q| q.completed).sum();
        let bytes: u64 = self.queues.values().map(|q| q.total_bytes).sum();
        let lat: u64 = self.queues.values().map(|q| q.total_latency_ns).sum();
        let avg_lat = if completed == 0 { 0 } else { lat / completed };
        let util: f64 = if self.queues.is_empty() { 0.0 } else { self.queues.values().map(|q| q.utilization()).sum::<f64>() / self.queues.len() as f64 };
        BlkMqStats { total_queues: self.queues.len() as u32, total_completed: completed, total_bytes: bytes, avg_latency_ns: avg_lat, avg_utilization: util }
    }
}
