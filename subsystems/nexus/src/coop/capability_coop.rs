// SPDX-License-Identifier: GPL-2.0
//! Coop capability — cooperative capability bounding set management

extern crate alloc;
use alloc::vec::Vec;

/// Capability coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapCoopEvent {
    BoundingSetInherit,
    AmbientPropagate,
    CapDrop,
    CapDelegate,
    NamespaceMap,
}

/// Capability coop record
#[derive(Debug, Clone)]
pub struct CapCoopRecord {
    pub event: CapCoopEvent,
    pub cap_id: u32,
    pub source_pid: u32,
    pub target_pid: u32,
    pub effective: bool,
}

impl CapCoopRecord {
    pub fn new(event: CapCoopEvent, cap_id: u32) -> Self {
        Self { event, cap_id, source_pid: 0, target_pid: 0, effective: true }
    }
}

/// Capability coop stats
#[derive(Debug, Clone)]
pub struct CapCoopStats {
    pub total_events: u64,
    pub inherits: u64,
    pub propagations: u64,
    pub drops: u64,
}

/// Main coop capability
#[derive(Debug)]
pub struct CoopCapability {
    pub stats: CapCoopStats,
}

impl CoopCapability {
    pub fn new() -> Self {
        Self { stats: CapCoopStats { total_events: 0, inherits: 0, propagations: 0, drops: 0 } }
    }

    pub fn record(&mut self, rec: &CapCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            CapCoopEvent::BoundingSetInherit => self.stats.inherits += 1,
            CapCoopEvent::AmbientPropagate | CapCoopEvent::CapDelegate => self.stats.propagations += 1,
            CapCoopEvent::CapDrop => self.stats.drops += 1,
            _ => {}
        }
    }
}
