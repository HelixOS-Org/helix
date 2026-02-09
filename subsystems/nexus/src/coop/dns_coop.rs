// SPDX-License-Identifier: GPL-2.0
//! Coop DNS — cooperative DNS resolution with shared cache

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Coop DNS record type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopDnsType {
    A,
    Aaaa,
    Cname,
    Mx,
    Ns,
    Ptr,
    Srv,
    Txt,
    Soa,
}

/// Coop DNS cache state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopDnsCacheState {
    Fresh,
    Stale,
    Refreshing,
    Expired,
    Pinned,
}

/// Shared DNS cache entry
#[derive(Debug, Clone)]
pub struct SharedDnsEntry {
    pub name_hash: u64,
    pub record_type: CoopDnsType,
    pub value_hash: u64,
    pub ttl_sec: u32,
    pub inserted_ns: u64,
    pub state: CoopDnsCacheState,
    pub hit_count: u64,
    pub shared_by: Vec<u64>,
}

impl SharedDnsEntry {
    pub fn new(name: &[u8], record_type: CoopDnsType, value: &[u8], ttl_sec: u32) -> Self {
        let hash = |data: &[u8]| -> u64 {
            let mut h: u64 = 0xcbf29ce484222325;
            for b in data { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
            h
        };
        Self {
            name_hash: hash(name), record_type, value_hash: hash(value),
            ttl_sec, inserted_ns: 0, state: CoopDnsCacheState::Fresh,
            hit_count: 0, shared_by: Vec::new(),
        }
    }

    #[inline]
    pub fn is_expired(&self, now_ns: u64) -> bool {
        if self.state == CoopDnsCacheState::Pinned { return false; }
        let elapsed_sec = (now_ns.saturating_sub(self.inserted_ns)) / 1_000_000_000;
        elapsed_sec > self.ttl_sec as u64
    }

    #[inline(always)]
    pub fn touch(&mut self) { self.hit_count += 1; }

    #[inline(always)]
    pub fn share_with(&mut self, ns_id: u64) {
        if !self.shared_by.contains(&ns_id) { self.shared_by.push(ns_id); }
    }

    #[inline]
    pub fn refresh(&mut self, new_ttl: u32, now_ns: u64) {
        self.ttl_sec = new_ttl;
        self.inserted_ns = now_ns;
        self.state = CoopDnsCacheState::Fresh;
    }
}

/// DNS query tracker
#[derive(Debug, Clone)]
pub struct DnsQueryTracker {
    pub total_queries: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub upstream_queries: u64,
    pub failures: u64,
    pub avg_latency_ns: u64,
    pub total_latency_ns: u64,
}

impl DnsQueryTracker {
    pub fn new() -> Self {
        Self { total_queries: 0, cache_hits: 0, cache_misses: 0, upstream_queries: 0, failures: 0, avg_latency_ns: 0, total_latency_ns: 0 }
    }

    #[inline(always)]
    pub fn record_hit(&mut self) { self.total_queries += 1; self.cache_hits += 1; }
    #[inline]
    pub fn record_miss(&mut self, latency_ns: u64) {
        self.total_queries += 1;
        self.cache_misses += 1;
        self.upstream_queries += 1;
        self.total_latency_ns += latency_ns;
        if self.upstream_queries > 0 { self.avg_latency_ns = self.total_latency_ns / self.upstream_queries; }
    }

    #[inline(always)]
    pub fn hit_rate(&self) -> f64 {
        if self.total_queries == 0 { 0.0 } else { self.cache_hits as f64 / self.total_queries as f64 }
    }
}

/// Coop DNS stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CoopDnsStats {
    pub total_entries: u64,
    pub shared_entries: u64,
    pub total_queries: u64,
    pub gc_runs: u64,
}

/// Main coop DNS manager
#[derive(Debug)]
pub struct CoopDns {
    pub cache: BTreeMap<u64, SharedDnsEntry>,
    pub tracker: DnsQueryTracker,
    pub stats: CoopDnsStats,
    pub max_entries: u32,
}

impl CoopDns {
    pub fn new(max_entries: u32) -> Self {
        Self {
            cache: BTreeMap::new(),
            tracker: DnsQueryTracker::new(),
            stats: CoopDnsStats { total_entries: 0, shared_entries: 0, total_queries: 0, gc_runs: 0 },
            max_entries,
        }
    }

    pub fn lookup(&mut self, name_hash: u64, now_ns: u64) -> bool {
        self.stats.total_queries += 1;
        if let Some(entry) = self.cache.get_mut(&name_hash) {
            if !entry.is_expired(now_ns) {
                entry.touch();
                self.tracker.record_hit();
                return true;
            }
        }
        self.tracker.record_miss(0);
        false
    }

    #[inline(always)]
    pub fn insert(&mut self, entry: SharedDnsEntry) {
        self.cache.insert(entry.name_hash, entry);
        self.stats.total_entries += 1;
    }

    #[inline]
    pub fn gc_expired(&mut self, now_ns: u64) -> u32 {
        let before = self.cache.len();
        self.cache.retain(|_, e| !e.is_expired(now_ns));
        self.stats.gc_runs += 1;
        (before - self.cache.len()) as u32
    }
}
