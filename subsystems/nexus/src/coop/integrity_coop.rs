// SPDX-License-Identifier: GPL-2.0
//! Coop integrity — cooperative integrity verification

extern crate alloc;
use alloc::vec::Vec;

/// Integrity coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrityCoopEvent {
    DigestShare,
    MeasurementSync,
    AppraisalDelegate,
    PolicyPropagate,
    CacheInvalidate,
}

/// Integrity coop record
#[derive(Debug, Clone)]
pub struct IntegrityCoopRecord {
    pub event: IntegrityCoopEvent,
    pub path_hash: u64,
    pub source_pid: u32,
    pub target_count: u32,
    pub digest_matches: bool,
}

impl IntegrityCoopRecord {
    pub fn new(event: IntegrityCoopEvent, path: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self { event, path_hash: h, source_pid: 0, target_count: 0, digest_matches: true }
    }
}

/// Integrity coop stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct IntegrityCoopStats {
    pub total_events: u64,
    pub digest_shares: u64,
    pub measurement_syncs: u64,
    pub mismatches: u64,
}

/// Main coop integrity
#[derive(Debug)]
pub struct CoopIntegrity {
    pub stats: IntegrityCoopStats,
}

impl CoopIntegrity {
    pub fn new() -> Self {
        Self { stats: IntegrityCoopStats { total_events: 0, digest_shares: 0, measurement_syncs: 0, mismatches: 0 } }
    }

    #[inline]
    pub fn record(&mut self, rec: &IntegrityCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            IntegrityCoopEvent::DigestShare => self.stats.digest_shares += 1,
            IntegrityCoopEvent::MeasurementSync => self.stats.measurement_syncs += 1,
            _ => {}
        }
        if !rec.digest_matches { self.stats.mismatches += 1; }
    }
}
