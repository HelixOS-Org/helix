// SPDX-License-Identifier: GPL-2.0
//! Coop Landlock — cooperative Landlock sandbox layering

extern crate alloc;
use alloc::vec::Vec;

/// Landlock coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandlockCoopEvent {
    RulesetStack,
    RulesetInherit,
    AccessNarrow,
    DomainMerge,
    PortShare,
}

/// Landlock coop record
#[derive(Debug, Clone)]
pub struct LandlockCoopRecord {
    pub event: LandlockCoopEvent,
    pub ruleset_layers: u32,
    pub access_mask: u64,
    pub source_pid: u32,
    pub child_pid: u32,
}

impl LandlockCoopRecord {
    pub fn new(event: LandlockCoopEvent) -> Self {
        Self { event, ruleset_layers: 1, access_mask: 0, source_pid: 0, child_pid: 0 }
    }
}

/// Landlock coop stats
#[derive(Debug, Clone)]
pub struct LandlockCoopStats {
    pub total_events: u64,
    pub stacks: u64,
    pub inherits: u64,
    pub narrows: u64,
}

/// Main coop Landlock
#[derive(Debug)]
pub struct CoopLandlock {
    pub stats: LandlockCoopStats,
}

impl CoopLandlock {
    pub fn new() -> Self {
        Self { stats: LandlockCoopStats { total_events: 0, stacks: 0, inherits: 0, narrows: 0 } }
    }

    pub fn record(&mut self, rec: &LandlockCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            LandlockCoopEvent::RulesetStack => self.stats.stacks += 1,
            LandlockCoopEvent::RulesetInherit => self.stats.inherits += 1,
            LandlockCoopEvent::AccessNarrow => self.stats.narrows += 1,
            _ => {}
        }
    }
}
