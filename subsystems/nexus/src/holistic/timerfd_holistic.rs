// SPDX-License-Identifier: GPL-2.0
//! Holistic timerfd — holistic timer precision analysis

extern crate alloc;

/// Timer precision grade
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerPrecision { Nanosecond, Microsecond, Millisecond, Coarse }

/// Timerfd holistic record
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TimerfdHolisticRecord {
    pub precision: TimerPrecision,
    pub interval_ns: u64,
    pub jitter_ns: u64,
    pub overruns: u32,
}

impl TimerfdHolisticRecord {
    pub fn new(precision: TimerPrecision) -> Self { Self { precision, interval_ns: 0, jitter_ns: 0, overruns: 0 } }
}

/// Timerfd holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TimerfdHolisticStats { pub total_samples: u64, pub coarse_timers: u64, pub total_overruns: u64, pub max_jitter_ns: u64 }

/// Main holistic timerfd
#[derive(Debug)]
#[repr(align(64))]
pub struct HolisticTimerfd { pub stats: TimerfdHolisticStats }

impl HolisticTimerfd {
    pub fn new() -> Self { Self { stats: TimerfdHolisticStats { total_samples: 0, coarse_timers: 0, total_overruns: 0, max_jitter_ns: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &TimerfdHolisticRecord) {
        self.stats.total_samples += 1;
        if rec.precision == TimerPrecision::Coarse { self.stats.coarse_timers += 1; }
        self.stats.total_overruns += rec.overruns as u64;
        if rec.jitter_ns > self.stats.max_jitter_ns { self.stats.max_jitter_ns = rec.jitter_ns; }
    }
}
