// SPDX-License-Identifier: GPL-2.0
//! Holistic TCP — holistic TCP performance analysis

extern crate alloc;

/// TCP health indicator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpHealth { Optimal, Degraded, Retransmitting, Stalled, Reset }

/// TCP holistic record
#[derive(Debug, Clone)]
pub struct TcpHolisticRecord {
    pub health: TcpHealth,
    pub rtt_us: u32,
    pub retransmits: u32,
    pub cwnd: u32,
    pub throughput_bps: u64,
}

impl TcpHolisticRecord {
    pub fn new(health: TcpHealth) -> Self { Self { health, rtt_us: 0, retransmits: 0, cwnd: 0, throughput_bps: 0 } }
}

/// TCP holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TcpHolisticStats { pub total_samples: u64, pub degraded: u64, pub retransmit_total: u64, pub peak_throughput: u64 }

/// Main holistic TCP
#[derive(Debug)]
pub struct HolisticTcp { pub stats: TcpHolisticStats }

impl HolisticTcp {
    pub fn new() -> Self { Self { stats: TcpHolisticStats { total_samples: 0, degraded: 0, retransmit_total: 0, peak_throughput: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &TcpHolisticRecord) {
        self.stats.total_samples += 1;
        match rec.health {
            TcpHealth::Degraded | TcpHealth::Stalled | TcpHealth::Reset => self.stats.degraded += 1,
            _ => {}
        }
        self.stats.retransmit_total += rec.retransmits as u64;
        if rec.throughput_bps > self.stats.peak_throughput { self.stats.peak_throughput = rec.throughput_bps; }
    }
}
