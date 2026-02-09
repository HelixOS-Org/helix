// SPDX-License-Identifier: GPL-2.0
//! Coop credential — cooperative credential sharing across namespaces

extern crate alloc;
use alloc::vec::Vec;

/// Credential coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredCoopEvent {
    CredShare,
    CredInherit,
    CredOverride,
    NsMap,
    UidTranslate,
    GidTranslate,
}

/// Credential coop record
#[derive(Debug, Clone)]
pub struct CredCoopRecord {
    pub event: CredCoopEvent,
    pub source_uid: u32,
    pub target_uid: u32,
    pub source_ns: u32,
    pub target_ns: u32,
    pub pid: u32,
}

impl CredCoopRecord {
    pub fn new(event: CredCoopEvent) -> Self {
        Self { event, source_uid: 0, target_uid: 0, source_ns: 0, target_ns: 0, pid: 0 }
    }
}

/// Credential coop stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CredCoopStats {
    pub total_events: u64,
    pub shares: u64,
    pub inherits: u64,
    pub translations: u64,
}

/// Main coop credential
#[derive(Debug)]
pub struct CoopCredential {
    pub stats: CredCoopStats,
}

impl CoopCredential {
    pub fn new() -> Self {
        Self { stats: CredCoopStats { total_events: 0, shares: 0, inherits: 0, translations: 0 } }
    }

    #[inline]
    pub fn record(&mut self, rec: &CredCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            CredCoopEvent::CredShare => self.stats.shares += 1,
            CredCoopEvent::CredInherit => self.stats.inherits += 1,
            CredCoopEvent::UidTranslate | CredCoopEvent::GidTranslate | CredCoopEvent::NsMap => self.stats.translations += 1,
            _ => {}
        }
    }
}
