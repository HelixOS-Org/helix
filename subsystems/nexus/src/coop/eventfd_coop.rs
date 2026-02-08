// SPDX-License-Identifier: GPL-2.0
//! Coop eventfd — cooperative eventfd counter coordination

extern crate alloc;

/// Eventfd coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventfdCoopEvent { CounterAggregate, WakeupFanout, SemaphoreChain, BarrierSync }

/// Eventfd coop record
#[derive(Debug, Clone)]
pub struct EventfdCoopRecord {
    pub event: EventfdCoopEvent,
    pub fd_count: u32,
    pub waiters: u32,
    pub counter_sum: u64,
}

impl EventfdCoopRecord {
    pub fn new(event: EventfdCoopEvent) -> Self { Self { event, fd_count: 0, waiters: 0, counter_sum: 0 } }
}

/// Eventfd coop stats
#[derive(Debug, Clone)]
pub struct EventfdCoopStats { pub total_events: u64, pub aggregates: u64, pub fanouts: u64, pub barriers: u64 }

/// Main coop eventfd
#[derive(Debug)]
pub struct CoopEventfd { pub stats: EventfdCoopStats }

impl CoopEventfd {
    pub fn new() -> Self { Self { stats: EventfdCoopStats { total_events: 0, aggregates: 0, fanouts: 0, barriers: 0 } } }
    pub fn record(&mut self, rec: &EventfdCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            EventfdCoopEvent::CounterAggregate => self.stats.aggregates += 1,
            EventfdCoopEvent::WakeupFanout => self.stats.fanouts += 1,
            EventfdCoopEvent::BarrierSync | EventfdCoopEvent::SemaphoreChain => self.stats.barriers += 1,
        }
    }
}
