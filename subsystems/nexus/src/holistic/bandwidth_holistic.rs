// SPDX-License-Identifier: GPL-2.0
//! Holistic bandwidth — holistic bandwidth utilization analysis

extern crate alloc;

/// Bandwidth utilization grade
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BandwidthGrade { Underused, Balanced, Saturated, Oversubscribed }

/// Bandwidth holistic record
#[derive(Debug, Clone)]
pub struct BandwidthHolisticRecord {
    pub grade: BandwidthGrade,
    pub tx_bps: u64,
    pub rx_bps: u64,
    pub capacity_bps: u64,
}

impl BandwidthHolisticRecord {
    pub fn new(grade: BandwidthGrade) -> Self { Self { grade, tx_bps: 0, rx_bps: 0, capacity_bps: 0 } }
}

/// Bandwidth holistic stats
#[derive(Debug, Clone)]
pub struct BandwidthHolisticStats { pub total_samples: u64, pub saturated: u64, pub peak_tx: u64, pub peak_rx: u64 }

/// Main holistic bandwidth
#[derive(Debug)]
pub struct HolisticBandwidth { pub stats: BandwidthHolisticStats }

impl HolisticBandwidth {
    pub fn new() -> Self { Self { stats: BandwidthHolisticStats { total_samples: 0, saturated: 0, peak_tx: 0, peak_rx: 0 } } }
    pub fn record(&mut self, rec: &BandwidthHolisticRecord) {
        self.stats.total_samples += 1;
        if rec.grade == BandwidthGrade::Saturated || rec.grade == BandwidthGrade::Oversubscribed { self.stats.saturated += 1; }
        if rec.tx_bps > self.stats.peak_tx { self.stats.peak_tx = rec.tx_bps; }
        if rec.rx_bps > self.stats.peak_rx { self.stats.peak_rx = rec.rx_bps; }
    }
}
