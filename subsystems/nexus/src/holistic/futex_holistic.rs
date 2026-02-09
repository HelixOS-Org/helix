// SPDX-License-Identifier: GPL-2.0
//! Holistic futex — holistic futex wait-queue depth analysis

extern crate alloc;

/// Futex queue health
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FutexQueueHealth { Balanced, Skewed, Starving, Thundering }

/// Futex holistic record
#[derive(Debug, Clone)]
pub struct FutexHolisticRecord {
    pub health: FutexQueueHealth,
    pub futex_addr: u64,
    pub queue_depth: u32,
    pub wake_batch: u32,
    pub wait_ns: u64,
}

impl FutexHolisticRecord {
    pub fn new(health: FutexQueueHealth) -> Self { Self { health, futex_addr: 0, queue_depth: 0, wake_batch: 0, wait_ns: 0 } }
}

/// Futex holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FutexHolisticStats { pub total_samples: u64, pub thundering_herds: u64, pub starved: u64, pub peak_depth: u32 }

/// Main holistic futex
#[derive(Debug)]
pub struct HolisticFutex { pub stats: FutexHolisticStats }

impl HolisticFutex {
    pub fn new() -> Self { Self { stats: FutexHolisticStats { total_samples: 0, thundering_herds: 0, starved: 0, peak_depth: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &FutexHolisticRecord) {
        self.stats.total_samples += 1;
        match rec.health {
            FutexQueueHealth::Thundering => self.stats.thundering_herds += 1,
            FutexQueueHealth::Starving => self.stats.starved += 1,
            _ => {}
        }
        if rec.queue_depth > self.stats.peak_depth { self.stats.peak_depth = rec.queue_depth; }
    }
}
