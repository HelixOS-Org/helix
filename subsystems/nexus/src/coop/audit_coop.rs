// SPDX-License-Identifier: GPL-2.0
//! Coop audit — cooperative audit trail coordination

extern crate alloc;
use alloc::vec::Vec;

/// Audit coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditCoopEvent {
    TrailMerge,
    RuleSync,
    BufferShare,
    FilterPropagate,
    SessionLink,
}

/// Audit coop record
#[derive(Debug, Clone)]
pub struct AuditCoopRecord {
    pub event: AuditCoopEvent,
    pub serial: u64,
    pub source_pid: u32,
    pub target_count: u32,
    pub records_merged: u32,
}

impl AuditCoopRecord {
    pub fn new(event: AuditCoopEvent) -> Self {
        Self { event, serial: 0, source_pid: 0, target_count: 0, records_merged: 0 }
    }
}

/// Audit coop stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AuditCoopStats {
    pub total_events: u64,
    pub trail_merges: u64,
    pub rule_syncs: u64,
    pub records_merged: u64,
}

/// Main coop audit
#[derive(Debug)]
pub struct CoopAudit {
    pub stats: AuditCoopStats,
}

impl CoopAudit {
    pub fn new() -> Self {
        Self { stats: AuditCoopStats { total_events: 0, trail_merges: 0, rule_syncs: 0, records_merged: 0 } }
    }

    #[inline]
    pub fn record(&mut self, rec: &AuditCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            AuditCoopEvent::TrailMerge => self.stats.trail_merges += 1,
            AuditCoopEvent::RuleSync | AuditCoopEvent::FilterPropagate => self.stats.rule_syncs += 1,
            _ => {}
        }
        self.stats.records_merged += rec.records_merged as u64;
    }
}
