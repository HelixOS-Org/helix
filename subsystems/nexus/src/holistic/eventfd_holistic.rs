// SPDX-License-Identifier: GPL-2.0
//! Holistic eventfd — holistic eventfd counter pattern analysis

extern crate alloc;

/// Eventfd pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventfdPattern { CounterBurst, SteadyState, Dormant, Saturated }

/// Eventfd holistic record
#[derive(Debug, Clone)]
pub struct EventfdHolisticRecord {
    pub pattern: EventfdPattern,
    pub counter_value: u64,
    pub wakeups_sec: u32,
    pub waiters: u32,
}

impl EventfdHolisticRecord {
    pub fn new(pattern: EventfdPattern) -> Self { Self { pattern, counter_value: 0, wakeups_sec: 0, waiters: 0 } }
}

/// Eventfd holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EventfdHolisticStats { pub total_samples: u64, pub bursts: u64, pub saturated: u64, pub peak_wakeups: u32 }

/// Main holistic eventfd
#[derive(Debug)]
pub struct HolisticEventfd { pub stats: EventfdHolisticStats }

impl HolisticEventfd {
    pub fn new() -> Self { Self { stats: EventfdHolisticStats { total_samples: 0, bursts: 0, saturated: 0, peak_wakeups: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &EventfdHolisticRecord) {
        self.stats.total_samples += 1;
        match rec.pattern {
            EventfdPattern::CounterBurst => self.stats.bursts += 1,
            EventfdPattern::Saturated => self.stats.saturated += 1,
            _ => {}
        }
        if rec.wakeups_sec > self.stats.peak_wakeups { self.stats.peak_wakeups = rec.wakeups_sec; }
    }
}
