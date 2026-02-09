// SPDX-License-Identifier: GPL-2.0
//! Holistic IPC — holistic cross-mechanism IPC analysis

extern crate alloc;

use alloc::collections::BTreeMap;

/// IPC mechanism type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IpcMechanism { Pipe, Socket, SharedMem, MessageQueue, Signal, Futex, Eventfd }

/// IPC holistic record
#[derive(Debug, Clone)]
pub struct IpcHolisticRecord {
    pub mechanism: IpcMechanism,
    pub throughput_bps: u64,
    pub latency_ns: u64,
    pub ops_sec: u64,
}

impl IpcHolisticRecord {
    pub fn new(mechanism: IpcMechanism) -> Self { Self { mechanism, throughput_bps: 0, latency_ns: 0, ops_sec: 0 } }
}

/// IPC holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct IpcHolisticStats {
    pub total_samples: u64,
    pub mechanism_counts: BTreeMap<u8, u64>,
    pub best_throughput: u64,
    pub worst_latency: u64,
}

/// Main holistic IPC
#[derive(Debug)]
pub struct HolisticIpc { pub stats: IpcHolisticStats }

impl HolisticIpc {
    pub fn new() -> Self {
        Self { stats: IpcHolisticStats { total_samples: 0, mechanism_counts: BTreeMap::new(), best_throughput: 0, worst_latency: 0 } }
    }
    #[inline]
    pub fn record(&mut self, rec: &IpcHolisticRecord) {
        self.stats.total_samples += 1;
        let key = rec.mechanism as u8;
        let count = self.stats.mechanism_counts.entry(key).or_insert(0);
        *count += 1;
        if rec.throughput_bps > self.stats.best_throughput { self.stats.best_throughput = rec.throughput_bps; }
        if rec.latency_ns > self.stats.worst_latency { self.stats.worst_latency = rec.latency_ns; }
    }
}
