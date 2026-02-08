// SPDX-License-Identifier: GPL-2.0
//! Holistic writeback — dirty page writeback analysis

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Writeback reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WritebackReason {
    Background,
    Sync,
    Periodic,
    DirtyThreshold,
    Reclaim,
    Fsync,
    Close,
}

/// Writeback state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticWbState {
    Idle,
    Running,
    Flushing,
    Congested,
}

/// Per-device writeback tracker
#[derive(Debug, Clone)]
pub struct DeviceWriteback {
    pub dev_id: u64,
    pub state: HolisticWbState,
    pub dirty_pages: u64,
    pub writeback_pages: u64,
    pub pages_written: u64,
    pub bandwidth_bps: u64,
}

impl DeviceWriteback {
    pub fn new(dev_id: u64) -> Self {
        Self { dev_id, state: HolisticWbState::Idle, dirty_pages: 0, writeback_pages: 0, pages_written: 0, bandwidth_bps: 0 }
    }

    pub fn start_writeback(&mut self, pages: u64) {
        self.writeback_pages += pages;
        self.dirty_pages = self.dirty_pages.saturating_sub(pages);
        self.state = HolisticWbState::Running;
    }

    pub fn complete_writeback(&mut self, pages: u64) {
        self.writeback_pages = self.writeback_pages.saturating_sub(pages);
        self.pages_written += pages;
        if self.writeback_pages == 0 { self.state = HolisticWbState::Idle; }
    }

    pub fn dirty(&mut self, pages: u64) { self.dirty_pages += pages; }
}

/// Holistic writeback stats
#[derive(Debug, Clone)]
pub struct HolisticWritebackStats {
    pub total_writebacks: u64,
    pub pages_written: u64,
    pub sync_triggered: u64,
    pub threshold_triggered: u64,
    pub congestion_events: u64,
}

/// Main holistic writeback
#[derive(Debug)]
pub struct HolisticWriteback {
    pub devices: BTreeMap<u64, DeviceWriteback>,
    pub stats: HolisticWritebackStats,
}

impl HolisticWriteback {
    pub fn new() -> Self {
        Self { devices: BTreeMap::new(), stats: HolisticWritebackStats { total_writebacks: 0, pages_written: 0, sync_triggered: 0, threshold_triggered: 0, congestion_events: 0 } }
    }

    pub fn record_writeback(&mut self, dev_id: u64, pages: u64, reason: WritebackReason) {
        self.stats.total_writebacks += 1;
        self.stats.pages_written += pages;
        match reason {
            WritebackReason::Sync | WritebackReason::Fsync => self.stats.sync_triggered += 1,
            WritebackReason::DirtyThreshold => self.stats.threshold_triggered += 1,
            _ => {}
        }
        let dev = self.devices.entry(dev_id).or_insert_with(|| DeviceWriteback::new(dev_id));
        dev.start_writeback(pages);
    }
}

// ============================================================================
// Merged from writeback_v2_holistic
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticWritebackV2Metric {
    DirtyPageCount,
    WritebackRate,
    FlushLatency,
    PressureEvents,
    BackgroundWriteRate,
    CongestedTime,
}

/// Writeback analysis sample
#[derive(Debug, Clone)]
pub struct HolisticWritebackV2Sample {
    pub metric: HolisticWritebackV2Metric,
    pub value: u64,
    pub timestamp: u64,
}

/// Writeback health assessment
#[derive(Debug, Clone)]
pub struct HolisticWritebackV2Health {
    pub dirty_ratio_health: u64,
    pub writeback_throughput: u64,
    pub pressure_score: u64,
    pub overall: u64,
}

/// Stats for writeback analysis
#[derive(Debug, Clone)]
pub struct HolisticWritebackV2Stats {
    pub samples: u64,
    pub analyses: u64,
    pub pressure_alerts: u64,
    pub congestion_alerts: u64,
}

/// Manager for writeback holistic analysis
pub struct HolisticWritebackV2Manager {
    samples: Vec<HolisticWritebackV2Sample>,
    health: HolisticWritebackV2Health,
    stats: HolisticWritebackV2Stats,
}

impl HolisticWritebackV2Manager {
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
            health: HolisticWritebackV2Health {
                dirty_ratio_health: 100,
                writeback_throughput: 100,
                pressure_score: 0,
                overall: 100,
            },
            stats: HolisticWritebackV2Stats {
                samples: 0,
                analyses: 0,
                pressure_alerts: 0,
                congestion_alerts: 0,
            },
        }
    }

    pub fn record(&mut self, metric: HolisticWritebackV2Metric, value: u64) {
        let sample = HolisticWritebackV2Sample {
            metric,
            value,
            timestamp: self.samples.len() as u64,
        };
        self.samples.push(sample);
        self.stats.samples += 1;
    }

    pub fn analyze(&mut self) -> &HolisticWritebackV2Health {
        self.stats.analyses += 1;
        let pressure: Vec<&HolisticWritebackV2Sample> = self.samples.iter()
            .filter(|s| matches!(s.metric, HolisticWritebackV2Metric::PressureEvents))
            .collect();
        if !pressure.is_empty() {
            let total: u64 = pressure.iter().map(|s| s.value).sum();
            self.health.pressure_score = (total / pressure.len() as u64).min(100);
            if self.health.pressure_score > 70 {
                self.stats.pressure_alerts += 1;
            }
        }
        self.health.overall = (self.health.dirty_ratio_health + self.health.writeback_throughput + (100 - self.health.pressure_score)) / 3;
        &self.health
    }

    pub fn stats(&self) -> &HolisticWritebackV2Stats {
        &self.stats
    }
}
