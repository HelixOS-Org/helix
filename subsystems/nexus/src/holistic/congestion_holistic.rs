// SPDX-License-Identifier: GPL-2.0
//! Holistic congestion — holistic congestion pattern analysis

extern crate alloc;

/// Congestion pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CongestionPattern { SlowStart, SteadyState, CongestionAvoidance, FastRecovery, Collapse }

/// Congestion holistic record
#[derive(Debug, Clone)]
pub struct CongestionHolisticRecord {
    pub pattern: CongestionPattern,
    pub cwnd: u32,
    pub loss_rate_pct: u8,
    pub throughput_bps: u64,
}

impl CongestionHolisticRecord {
    pub fn new(pattern: CongestionPattern) -> Self { Self { pattern, cwnd: 0, loss_rate_pct: 0, throughput_bps: 0 } }
}

/// Congestion holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CongestionHolisticStats { pub total_samples: u64, pub collapses: u64, pub recoveries: u64, pub avg_cwnd: u32 }

/// Main holistic congestion
#[derive(Debug)]
pub struct HolisticCongestion {
    pub stats: CongestionHolisticStats,
    cwnd_sum: u64,
}

impl HolisticCongestion {
    pub fn new() -> Self { Self { stats: CongestionHolisticStats { total_samples: 0, collapses: 0, recoveries: 0, avg_cwnd: 0 }, cwnd_sum: 0 } }
    #[inline]
    pub fn record(&mut self, rec: &CongestionHolisticRecord) {
        self.stats.total_samples += 1;
        match rec.pattern {
            CongestionPattern::Collapse => self.stats.collapses += 1,
            CongestionPattern::FastRecovery => self.stats.recoveries += 1,
            _ => {}
        }
        self.cwnd_sum += rec.cwnd as u64;
        self.stats.avg_cwnd = (self.cwnd_sum / self.stats.total_samples) as u32;
    }
}
