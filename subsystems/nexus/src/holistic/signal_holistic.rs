// SPDX-License-Identifier: GPL-2.0
//! Holistic signal — holistic signal delivery analysis

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Signal delivery pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalPattern { BurstDelivery, CascadeSignal, SignalStorm, NormalDelivery }

/// Signal holistic record
#[derive(Debug, Clone)]
pub struct SignalHolisticRecord {
    pub pattern: SignalPattern,
    pub signal_nr: u32,
    pub source_pid: u32,
    pub target_count: u32,
    pub latency_ns: u64,
}

impl SignalHolisticRecord {
    pub fn new(pattern: SignalPattern, signal_nr: u32) -> Self {
        Self { pattern, signal_nr, source_pid: 0, target_count: 0, latency_ns: 0 }
    }
}

/// Signal holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SignalHolisticStats {
    pub total_signals: u64,
    pub storms_detected: u64,
    pub avg_latency_ns: u64,
    pub peak_burst: u32,
}

/// Main holistic signal
#[derive(Debug)]
pub struct HolisticSignal {
    pub stats: SignalHolisticStats,
    pub recent_latencies: VecDeque<u64>,
}

impl HolisticSignal {
    pub fn new() -> Self {
        Self {
            stats: SignalHolisticStats { total_signals: 0, storms_detected: 0, avg_latency_ns: 0, peak_burst: 0 },
            recent_latencies: VecDeque::new(),
        }
    }
    #[inline]
    pub fn record(&mut self, rec: &SignalHolisticRecord) {
        self.stats.total_signals += 1;
        if rec.pattern == SignalPattern::SignalStorm { self.stats.storms_detected += 1; }
        if rec.target_count > self.stats.peak_burst { self.stats.peak_burst = rec.target_count; }
        self.recent_latencies.push_back(rec.latency_ns);
        if self.recent_latencies.len() > 256 { self.recent_latencies.pop_front(); }
        let sum: u64 = self.recent_latencies.iter().sum();
        self.stats.avg_latency_ns = sum / self.recent_latencies.len() as u64;
    }
}
