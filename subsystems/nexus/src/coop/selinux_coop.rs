// SPDX-License-Identifier: GPL-2.0
//! Coop SELinux — cooperative SELinux context propagation

extern crate alloc;
use alloc::vec::Vec;

/// SELinux coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelinuxCoopEvent {
    ContextInherit,
    ContextTransition,
    TypeEnforce,
    BoolSync,
    AvcFlush,
}

/// SELinux coop record
#[derive(Debug, Clone)]
pub struct SelinuxCoopRecord {
    pub event: SelinuxCoopEvent,
    pub source_ctx_hash: u64,
    pub target_ctx_hash: u64,
    pub source_pid: u32,
    pub target_pid: u32,
}

impl SelinuxCoopRecord {
    pub fn new(event: SelinuxCoopEvent, ctx: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in ctx { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self { event, source_ctx_hash: h, target_ctx_hash: 0, source_pid: 0, target_pid: 0 }
    }
}

/// SELinux coop stats
#[derive(Debug, Clone)]
pub struct SelinuxCoopStats {
    pub total_events: u64,
    pub context_inherits: u64,
    pub transitions: u64,
    pub avc_flushes: u64,
}

/// Main coop SELinux
#[derive(Debug)]
pub struct CoopSelinux {
    pub stats: SelinuxCoopStats,
}

impl CoopSelinux {
    pub fn new() -> Self {
        Self { stats: SelinuxCoopStats { total_events: 0, context_inherits: 0, transitions: 0, avc_flushes: 0 } }
    }

    pub fn record(&mut self, rec: &SelinuxCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            SelinuxCoopEvent::ContextInherit => self.stats.context_inherits += 1,
            SelinuxCoopEvent::ContextTransition | SelinuxCoopEvent::TypeEnforce => self.stats.transitions += 1,
            SelinuxCoopEvent::AvcFlush => self.stats.avc_flushes += 1,
            _ => {}
        }
    }
}
