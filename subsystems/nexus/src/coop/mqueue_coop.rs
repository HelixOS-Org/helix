// SPDX-License-Identifier: GPL-2.0
//! Coop mqueue — cooperative POSIX message queue load balancing

extern crate alloc;

/// Mqueue coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MqueueCoopEvent { ConsumerBalance, ProducerThrottle, PriorityBoost, OverflowRedirect }

/// Mqueue coop record
#[derive(Debug, Clone)]
pub struct MqueueCoopRecord {
    pub event: MqueueCoopEvent,
    pub queue_hash: u64,
    pub consumers: u32,
    pub producers: u32,
    pub depth: u32,
}

impl MqueueCoopRecord {
    pub fn new(event: MqueueCoopEvent) -> Self { Self { event, queue_hash: 0, consumers: 0, producers: 0, depth: 0 } }
}

/// Mqueue coop stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MqueueCoopStats { pub total_events: u64, pub balances: u64, pub throttles: u64, pub redirects: u64 }

/// Main coop mqueue
#[derive(Debug)]
pub struct CoopMqueue { pub stats: MqueueCoopStats }

impl CoopMqueue {
    pub fn new() -> Self { Self { stats: MqueueCoopStats { total_events: 0, balances: 0, throttles: 0, redirects: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &MqueueCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            MqueueCoopEvent::ConsumerBalance => self.stats.balances += 1,
            MqueueCoopEvent::ProducerThrottle => self.stats.throttles += 1,
            MqueueCoopEvent::OverflowRedirect => self.stats.redirects += 1,
            _ => {}
        }
    }
}
