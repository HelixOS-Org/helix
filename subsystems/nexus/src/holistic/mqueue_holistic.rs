// SPDX-License-Identifier: GPL-2.0
//! Holistic mqueue — holistic POSIX message queue latency analysis

extern crate alloc;

/// Mqueue latency band
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MqueueLatencyBand { Fast, Normal, Slow, Critical }

/// Mqueue holistic record
#[derive(Debug, Clone)]
pub struct MqueueHolisticRecord {
    pub band: MqueueLatencyBand,
    pub queue_hash: u64,
    pub send_latency_ns: u64,
    pub recv_latency_ns: u64,
    pub depth: u32,
}

impl MqueueHolisticRecord {
    pub fn new(band: MqueueLatencyBand) -> Self { Self { band, queue_hash: 0, send_latency_ns: 0, recv_latency_ns: 0, depth: 0 } }
}

/// Mqueue holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MqueueHolisticStats { pub total_samples: u64, pub slow_ops: u64, pub critical_ops: u64, pub avg_latency_ns: u64 }

/// Main holistic mqueue
#[derive(Debug)]
pub struct HolisticMqueue {
    pub stats: MqueueHolisticStats,
    latency_sum: u64,
}

impl HolisticMqueue {
    pub fn new() -> Self { Self { stats: MqueueHolisticStats { total_samples: 0, slow_ops: 0, critical_ops: 0, avg_latency_ns: 0 }, latency_sum: 0 } }
    #[inline]
    pub fn record(&mut self, rec: &MqueueHolisticRecord) {
        self.stats.total_samples += 1;
        match rec.band {
            MqueueLatencyBand::Slow => self.stats.slow_ops += 1,
            MqueueLatencyBand::Critical => self.stats.critical_ops += 1,
            _ => {}
        }
        self.latency_sum += rec.send_latency_ns + rec.recv_latency_ns;
        self.stats.avg_latency_ns = self.latency_sum / self.stats.total_samples;
    }
}
