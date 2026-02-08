// SPDX-License-Identifier: GPL-2.0
//! Holistic UDP — holistic UDP loss analysis

extern crate alloc;

/// UDP quality grade
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UdpQuality { Excellent, Good, LossyLow, LossyHigh }

/// UDP holistic record
#[derive(Debug, Clone)]
pub struct UdpHolisticRecord {
    pub quality: UdpQuality,
    pub sent: u64,
    pub received: u64,
    pub dropped: u64,
    pub jitter_us: u32,
}

impl UdpHolisticRecord {
    pub fn new(quality: UdpQuality) -> Self { Self { quality, sent: 0, received: 0, dropped: 0, jitter_us: 0 } }
}

/// UDP holistic stats
#[derive(Debug, Clone)]
pub struct UdpHolisticStats { pub total_samples: u64, pub total_loss: u64, pub high_loss_events: u64, pub peak_jitter: u32 }

/// Main holistic UDP
#[derive(Debug)]
pub struct HolisticUdp { pub stats: UdpHolisticStats }

impl HolisticUdp {
    pub fn new() -> Self { Self { stats: UdpHolisticStats { total_samples: 0, total_loss: 0, high_loss_events: 0, peak_jitter: 0 } } }
    pub fn record(&mut self, rec: &UdpHolisticRecord) {
        self.stats.total_samples += 1;
        self.stats.total_loss += rec.dropped;
        if rec.quality == UdpQuality::LossyHigh { self.stats.high_loss_events += 1; }
        if rec.jitter_us > self.stats.peak_jitter { self.stats.peak_jitter = rec.jitter_us; }
    }
}
