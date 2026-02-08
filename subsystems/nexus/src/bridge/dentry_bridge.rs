// SPDX-License-Identifier: GPL-2.0
//! Bridge dentry — directory entry cache bridge with lookup and invalidation

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Dentry bridge operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DentryBridgeOp {
    Lookup,
    Create,
    Delete,
    Rename,
    Revalidate,
    Invalidate,
    Release,
}

/// Dentry bridge result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DentryBridgeResult {
    Found,
    NotFound,
    NegativeHit,
    Created,
    Invalidated,
    Stale,
    Error,
}

/// Dentry bridge record
#[derive(Debug, Clone)]
pub struct DentryBridgeRecord {
    pub op: DentryBridgeOp,
    pub result: DentryBridgeResult,
    pub name_hash: u64,
    pub parent_hash: u64,
    pub inode: u64,
    pub latency_ns: u64,
}

impl DentryBridgeRecord {
    pub fn new(op: DentryBridgeOp, name: &[u8], parent_hash: u64) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in name { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self { op, result: DentryBridgeResult::Found, name_hash: h, parent_hash, inode: 0, latency_ns: 0 }
    }
}

/// Dentry bridge stats
#[derive(Debug, Clone)]
pub struct DentryBridgeStats {
    pub total_ops: u64,
    pub lookups: u64,
    pub cache_hits: u64,
    pub negative_hits: u64,
    pub invalidations: u64,
}

/// Main bridge dentry
#[derive(Debug)]
pub struct BridgeDentry {
    pub stats: DentryBridgeStats,
}

impl BridgeDentry {
    pub fn new() -> Self {
        Self { stats: DentryBridgeStats { total_ops: 0, lookups: 0, cache_hits: 0, negative_hits: 0, invalidations: 0 } }
    }

    pub fn record(&mut self, rec: &DentryBridgeRecord) {
        self.stats.total_ops += 1;
        match rec.op {
            DentryBridgeOp::Lookup => {
                self.stats.lookups += 1;
                match rec.result {
                    DentryBridgeResult::Found => self.stats.cache_hits += 1,
                    DentryBridgeResult::NegativeHit => self.stats.negative_hits += 1,
                    _ => {}
                }
            }
            DentryBridgeOp::Invalidate => self.stats.invalidations += 1,
            _ => {}
        }
    }

    pub fn hit_rate(&self) -> f64 {
        if self.stats.lookups == 0 { 0.0 } else { self.stats.cache_hits as f64 / self.stats.lookups as f64 }
    }
}

// ============================================================================
// Merged from dentry_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DentryV2Event { CacheHit, CacheMiss, Invalidate, Negative, Revalidate }

/// Dentry v2 record
#[derive(Debug, Clone)]
pub struct DentryV2Record {
    pub event: DentryV2Event,
    pub name_hash: u64,
    pub parent_hash: u64,
    pub inode: u64,
}

impl DentryV2Record {
    pub fn new(event: DentryV2Event) -> Self { Self { event, name_hash: 0, parent_hash: 0, inode: 0 } }
}

/// Dentry v2 bridge stats
#[derive(Debug, Clone)]
pub struct DentryV2BridgeStats { pub total_events: u64, pub hits: u64, pub misses: u64, pub invalidations: u64 }

/// Main bridge dentry v2
#[derive(Debug)]
pub struct BridgeDentryV2 { pub stats: DentryV2BridgeStats }

impl BridgeDentryV2 {
    pub fn new() -> Self { Self { stats: DentryV2BridgeStats { total_events: 0, hits: 0, misses: 0, invalidations: 0 } } }
    pub fn record(&mut self, rec: &DentryV2Record) {
        self.stats.total_events += 1;
        match rec.event {
            DentryV2Event::CacheHit => self.stats.hits += 1,
            DentryV2Event::CacheMiss | DentryV2Event::Negative => self.stats.misses += 1,
            DentryV2Event::Invalidate | DentryV2Event::Revalidate => self.stats.invalidations += 1,
        }
    }
}
