// SPDX-License-Identifier: GPL-2.0
//! Holistic semaphore — holistic semaphore contention analysis

extern crate alloc;

/// Semaphore contention level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemContentionLevel { None, Low, Medium, High, Deadlocked }

/// Semaphore holistic record
#[derive(Debug, Clone)]
pub struct SemHolisticRecord {
    pub contention: SemContentionLevel,
    pub semid: i32,
    pub waiter_count: u32,
    pub hold_time_ns: u64,
}

impl SemHolisticRecord {
    pub fn new(contention: SemContentionLevel) -> Self { Self { contention, semid: -1, waiter_count: 0, hold_time_ns: 0 } }
}

/// Semaphore holistic stats
#[derive(Debug, Clone)]
pub struct SemHolisticStats { pub total_samples: u64, pub high_contention: u64, pub deadlocks: u64, pub avg_hold_ns: u64 }

/// Main holistic semaphore
#[derive(Debug)]
pub struct HolisticSemaphore {
    pub stats: SemHolisticStats,
    hold_sum: u64,
}

impl HolisticSemaphore {
    pub fn new() -> Self { Self { stats: SemHolisticStats { total_samples: 0, high_contention: 0, deadlocks: 0, avg_hold_ns: 0 }, hold_sum: 0 } }
    pub fn record(&mut self, rec: &SemHolisticRecord) {
        self.stats.total_samples += 1;
        match rec.contention {
            SemContentionLevel::High => self.stats.high_contention += 1,
            SemContentionLevel::Deadlocked => self.stats.deadlocks += 1,
            _ => {}
        }
        self.hold_sum += rec.hold_time_ns;
        self.stats.avg_hold_ns = self.hold_sum / self.stats.total_samples;
    }
}
