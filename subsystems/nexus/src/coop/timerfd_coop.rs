// SPDX-License-Identifier: GPL-2.0
//! Coop timerfd — cooperative timer coordination

extern crate alloc;

/// Timerfd coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerfdCoopEvent { TimerGroup, ExpiryCoalesce, IntervalAlign, WakeupBatch }

/// Timerfd coop record
#[derive(Debug, Clone)]
pub struct TimerfdCoopRecord {
    pub event: TimerfdCoopEvent,
    pub timers: u32,
    pub interval_ns: u64,
    pub coalesced_wakes: u32,
}

impl TimerfdCoopRecord {
    pub fn new(event: TimerfdCoopEvent) -> Self { Self { event, timers: 0, interval_ns: 0, coalesced_wakes: 0 } }
}

/// Timerfd coop stats
#[derive(Debug, Clone)]
pub struct TimerfdCoopStats { pub total_events: u64, pub groups: u64, pub coalesced: u64, pub batches: u64 }

/// Main coop timerfd
#[derive(Debug)]
pub struct CoopTimerfd { pub stats: TimerfdCoopStats }

impl CoopTimerfd {
    pub fn new() -> Self { Self { stats: TimerfdCoopStats { total_events: 0, groups: 0, coalesced: 0, batches: 0 } } }
    pub fn record(&mut self, rec: &TimerfdCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            TimerfdCoopEvent::TimerGroup => self.stats.groups += 1,
            TimerfdCoopEvent::ExpiryCoalesce | TimerfdCoopEvent::IntervalAlign => self.stats.coalesced += 1,
            TimerfdCoopEvent::WakeupBatch => self.stats.batches += 1,
        }
    }
}
