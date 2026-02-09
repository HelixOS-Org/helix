// SPDX-License-Identifier: GPL-2.0
//! Holistic shm — holistic shared memory utilization analysis

extern crate alloc;

/// Shm utilization state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShmUtilization { Idle, Active, HotSpot, Thrashing }

/// Shm holistic record
#[derive(Debug, Clone)]
pub struct ShmHolisticRecord {
    pub utilization: ShmUtilization,
    pub shmid: i32,
    pub pages_active: u64,
    pub pages_dirty: u64,
    pub attach_count: u32,
}

impl ShmHolisticRecord {
    pub fn new(utilization: ShmUtilization) -> Self { Self { utilization, shmid: -1, pages_active: 0, pages_dirty: 0, attach_count: 0 } }
}

/// Shm holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ShmHolisticStats { pub total_samples: u64, pub hotspots: u64, pub thrashing: u64, pub peak_dirty: u64 }

/// Main holistic shm
#[derive(Debug)]
pub struct HolisticShm { pub stats: ShmHolisticStats }

impl HolisticShm {
    pub fn new() -> Self { Self { stats: ShmHolisticStats { total_samples: 0, hotspots: 0, thrashing: 0, peak_dirty: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &ShmHolisticRecord) {
        self.stats.total_samples += 1;
        match rec.utilization {
            ShmUtilization::HotSpot => self.stats.hotspots += 1,
            ShmUtilization::Thrashing => self.stats.thrashing += 1,
            _ => {}
        }
        if rec.pages_dirty > self.stats.peak_dirty { self.stats.peak_dirty = rec.pages_dirty; }
    }
}
