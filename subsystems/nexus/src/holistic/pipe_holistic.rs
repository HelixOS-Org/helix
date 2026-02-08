// SPDX-License-Identifier: GPL-2.0
//! Holistic pipe — holistic pipe throughput analysis

extern crate alloc;

/// Pipe health state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipeHealth { Healthy, Congested, Stalled, Broken }

/// Pipe holistic record
#[derive(Debug, Clone)]
pub struct PipeHolisticRecord {
    pub health: PipeHealth,
    pub throughput_bps: u64,
    pub buffer_usage_pct: u8,
    pub readers: u32,
    pub writers: u32,
}

impl PipeHolisticRecord {
    pub fn new(health: PipeHealth) -> Self { Self { health, throughput_bps: 0, buffer_usage_pct: 0, readers: 0, writers: 0 } }
}

/// Pipe holistic stats
#[derive(Debug, Clone)]
pub struct PipeHolisticStats { pub total_samples: u64, pub congestions: u64, pub stalls: u64, pub peak_throughput: u64 }

/// Main holistic pipe
#[derive(Debug)]
pub struct HolisticPipe { pub stats: PipeHolisticStats }

impl HolisticPipe {
    pub fn new() -> Self { Self { stats: PipeHolisticStats { total_samples: 0, congestions: 0, stalls: 0, peak_throughput: 0 } } }
    pub fn record(&mut self, rec: &PipeHolisticRecord) {
        self.stats.total_samples += 1;
        match rec.health {
            PipeHealth::Congested => self.stats.congestions += 1,
            PipeHealth::Stalled | PipeHealth::Broken => self.stats.stalls += 1,
            _ => {}
        }
        if rec.throughput_bps > self.stats.peak_throughput { self.stats.peak_throughput = rec.throughput_bps; }
    }
}
