// SPDX-License-Identifier: GPL-2.0
//! Holistic latency — holistic network latency distribution analysis

extern crate alloc;

/// Latency distribution bucket
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatencyBucket { Sub1ms, Sub10ms, Sub100ms, Over100ms, Over1s }

/// Latency holistic record
#[derive(Debug, Clone)]
pub struct LatencyHolisticRecord {
    pub bucket: LatencyBucket,
    pub latency_us: u64,
    pub p99_us: u64,
    pub p50_us: u64,
}

impl LatencyHolisticRecord {
    pub fn new(bucket: LatencyBucket, latency_us: u64) -> Self { Self { bucket, latency_us, p99_us: 0, p50_us: 0 } }
}

/// Latency holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct LatencyHolisticStats { pub total_samples: u64, pub over_100ms: u64, pub worst_us: u64, pub avg_us: u64 }

/// Main holistic latency
#[derive(Debug)]
pub struct HolisticLatency {
    pub stats: LatencyHolisticStats,
    sum_us: u64,
}

impl HolisticLatency {
    pub fn new() -> Self { Self { stats: LatencyHolisticStats { total_samples: 0, over_100ms: 0, worst_us: 0, avg_us: 0 }, sum_us: 0 } }
    #[inline]
    pub fn record(&mut self, rec: &LatencyHolisticRecord) {
        self.stats.total_samples += 1;
        if rec.bucket == LatencyBucket::Over100ms || rec.bucket == LatencyBucket::Over1s { self.stats.over_100ms += 1; }
        if rec.latency_us > self.stats.worst_us { self.stats.worst_us = rec.latency_us; }
        self.sum_us += rec.latency_us;
        self.stats.avg_us = self.sum_us / self.stats.total_samples;
    }
}
