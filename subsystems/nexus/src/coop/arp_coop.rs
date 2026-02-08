// SPDX-License-Identifier: GPL-2.0
//! Coop ARP — cooperative ARP cache with neighbor discovery sharing

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Coop ARP state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopArpState {
    Incomplete,
    Reachable,
    Stale,
    Delay,
    Probe,
    Failed,
    Shared,
}

/// Coop ARP entry
#[derive(Debug, Clone)]
pub struct CoopArpEntry {
    pub ip_hash: u64,
    pub mac_hash: u64,
    pub state: CoopArpState,
    pub confirmed_ns: u64,
    pub probes_sent: u32,
    pub ref_count: u32,
    pub shared_by: Vec<u64>,
}

impl CoopArpEntry {
    pub fn new(ip: &[u8], mac: &[u8]) -> Self {
        let hash = |data: &[u8]| -> u64 {
            let mut h: u64 = 0xcbf29ce484222325;
            for b in data { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
            h
        };
        Self {
            ip_hash: hash(ip), mac_hash: hash(mac), state: CoopArpState::Reachable,
            confirmed_ns: 0, probes_sent: 0, ref_count: 1, shared_by: Vec::new(),
        }
    }

    pub fn confirm(&mut self, ts_ns: u64) {
        self.state = CoopArpState::Reachable;
        self.confirmed_ns = ts_ns;
        self.probes_sent = 0;
    }

    pub fn mark_stale(&mut self) { self.state = CoopArpState::Stale; }
    pub fn probe(&mut self) { self.probes_sent += 1; self.state = CoopArpState::Probe; }
    pub fn fail(&mut self) { self.state = CoopArpState::Failed; }

    pub fn share_with(&mut self, ns_id: u64) {
        if !self.shared_by.contains(&ns_id) {
            self.shared_by.push(ns_id);
            self.ref_count += 1;
        }
        self.state = CoopArpState::Shared;
    }

    pub fn is_valid(&self) -> bool {
        matches!(self.state, CoopArpState::Reachable | CoopArpState::Shared | CoopArpState::Stale)
    }

    pub fn age_ms(&self, now_ns: u64) -> u64 {
        if now_ns > self.confirmed_ns { (now_ns - self.confirmed_ns) / 1_000_000 } else { 0 }
    }
}

/// Coop ARP cache
#[derive(Debug, Clone)]
pub struct SharedArpCache {
    pub entries: BTreeMap<u64, CoopArpEntry>,
    pub max_entries: u32,
    pub gc_threshold: u64,
}

impl SharedArpCache {
    pub fn new(max_entries: u32, gc_threshold_ms: u64) -> Self {
        Self { entries: BTreeMap::new(), max_entries, gc_threshold: gc_threshold_ms * 1_000_000 }
    }

    pub fn lookup(&self, ip_hash: u64) -> Option<&CoopArpEntry> {
        self.entries.get(&ip_hash).filter(|e| e.is_valid())
    }

    pub fn insert(&mut self, entry: CoopArpEntry) {
        self.entries.insert(entry.ip_hash, entry);
    }

    pub fn gc(&mut self, now_ns: u64) -> u32 {
        let before = self.entries.len();
        self.entries.retain(|_, e| {
            e.state != CoopArpState::Failed && (now_ns - e.confirmed_ns) < self.gc_threshold
        });
        (before - self.entries.len()) as u32
    }
}

/// Coop ARP stats
#[derive(Debug, Clone)]
pub struct CoopArpStats {
    pub total_entries: u64,
    pub shared_entries: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub gc_runs: u64,
}

/// Main coop ARP manager
#[derive(Debug)]
pub struct CoopArp {
    pub cache: SharedArpCache,
    pub stats: CoopArpStats,
}

impl CoopArp {
    pub fn new(max_entries: u32) -> Self {
        Self {
            cache: SharedArpCache::new(max_entries, 300_000),
            stats: CoopArpStats { total_entries: 0, shared_entries: 0, cache_hits: 0, cache_misses: 0, gc_runs: 0 },
        }
    }

    pub fn lookup(&mut self, ip_hash: u64) -> bool {
        if self.cache.lookup(ip_hash).is_some() {
            self.stats.cache_hits += 1;
            true
        } else {
            self.stats.cache_misses += 1;
            false
        }
    }

    pub fn insert(&mut self, entry: CoopArpEntry) {
        self.cache.insert(entry);
        self.stats.total_entries += 1;
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.stats.cache_hits + self.stats.cache_misses;
        if total == 0 { 0.0 } else { self.stats.cache_hits as f64 / total as f64 }
    }
}
