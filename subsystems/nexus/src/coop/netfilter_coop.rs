// SPDX-License-Identifier: GPL-2.0
//! Coop netfilter — cooperative packet filtering coordination

extern crate alloc;

/// Netfilter coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetfilterCoopEvent { RuleShare, ChainOptimize, TableMerge, HookSync }

/// Netfilter coop record
#[derive(Debug, Clone)]
pub struct NetfilterCoopRecord {
    pub event: NetfilterCoopEvent,
    pub rules: u32,
    pub chains: u32,
    pub packets_filtered: u64,
}

impl NetfilterCoopRecord {
    pub fn new(event: NetfilterCoopEvent) -> Self { Self { event, rules: 0, chains: 0, packets_filtered: 0 } }
}

/// Netfilter coop stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct NetfilterCoopStats { pub total_events: u64, pub shared_rules: u64, pub optimized: u64, pub syncs: u64 }

/// Main coop netfilter
#[derive(Debug)]
pub struct CoopNetfilter { pub stats: NetfilterCoopStats }

impl CoopNetfilter {
    pub fn new() -> Self { Self { stats: NetfilterCoopStats { total_events: 0, shared_rules: 0, optimized: 0, syncs: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &NetfilterCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            NetfilterCoopEvent::RuleShare | NetfilterCoopEvent::TableMerge => self.stats.shared_rules += 1,
            NetfilterCoopEvent::ChainOptimize => self.stats.optimized += 1,
            NetfilterCoopEvent::HookSync => self.stats.syncs += 1,
        }
    }
}
