// SPDX-License-Identifier: GPL-2.0
//! Holistic DNS cache — domain name resolution cache with TTL and negative caching

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;

/// DNS record type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsRecordType {
    A,
    Aaaa,
    Cname,
    Mx,
    Ns,
    Ptr,
    Srv,
    Txt,
    Soa,
    Caa,
    Dnskey,
    Ds,
    Rrsig,
}

/// DNS response code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsRcode {
    NoError,
    FormErr,
    ServFail,
    NxDomain,
    NotImp,
    Refused,
    YxDomain,
    NotAuth,
}

/// DNS cache entry state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsCacheState {
    Fresh,
    Stale,
    Expired,
    NegativeCached,
    Pinned,
}

/// DNS cache entry
#[derive(Debug, Clone)]
pub struct DnsCacheEntry {
    pub name_hash: u64,
    pub record_type: DnsRecordType,
    pub state: DnsCacheState,
    pub address: u64,
    pub ttl_sec: u32,
    pub original_ttl_sec: u32,
    pub created_ns: u64,
    pub last_used_ns: u64,
    pub hit_count: u64,
    pub rcode: DnsRcode,
    pub dnssec_valid: bool,
    pub source_server: u32,
}

impl DnsCacheEntry {
    pub fn new(name: &[u8], record_type: DnsRecordType, address: u64, ttl_sec: u32) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in name {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Self {
            name_hash: h,
            record_type,
            state: DnsCacheState::Fresh,
            address,
            ttl_sec,
            original_ttl_sec: ttl_sec,
            created_ns: 0,
            last_used_ns: 0,
            hit_count: 0,
            rcode: DnsRcode::NoError,
            dnssec_valid: false,
            source_server: 0,
        }
    }

    pub fn is_expired(&self, now_ns: u64) -> bool {
        if self.state == DnsCacheState::Pinned {
            return false;
        }
        let age_sec = (now_ns.saturating_sub(self.created_ns)) / 1_000_000_000;
        age_sec > self.ttl_sec as u64
    }

    pub fn remaining_ttl_sec(&self, now_ns: u64) -> u64 {
        if self.state == DnsCacheState::Pinned {
            return u64::MAX;
        }
        let age_sec = (now_ns.saturating_sub(self.created_ns)) / 1_000_000_000;
        if age_sec >= self.ttl_sec as u64 {
            0
        } else {
            self.ttl_sec as u64 - age_sec
        }
    }

    pub fn touch(&mut self, now_ns: u64) {
        self.last_used_ns = now_ns;
        self.hit_count += 1;
    }

    pub fn mark_stale(&mut self) {
        if self.state == DnsCacheState::Fresh {
            self.state = DnsCacheState::Stale;
        }
    }

    pub fn refresh(&mut self, address: u64, ttl_sec: u32, now_ns: u64) {
        self.address = address;
        self.ttl_sec = ttl_sec;
        self.original_ttl_sec = ttl_sec;
        self.created_ns = now_ns;
        self.state = DnsCacheState::Fresh;
    }
}

/// DNS server state
#[derive(Debug, Clone)]
pub struct DnsServerState {
    pub server_ip: u32,
    pub queries_sent: u64,
    pub responses_received: u64,
    pub failures: u64,
    pub avg_latency_ns: u64,
    pub total_latency_ns: u64,
}

impl DnsServerState {
    pub fn new(server_ip: u32) -> Self {
        Self {
            server_ip,
            queries_sent: 0,
            responses_received: 0,
            failures: 0,
            avg_latency_ns: 0,
            total_latency_ns: 0,
        }
    }

    pub fn record_query(&mut self) {
        self.queries_sent += 1;
    }

    pub fn record_response(&mut self, latency_ns: u64) {
        self.responses_received += 1;
        self.total_latency_ns += latency_ns;
        self.avg_latency_ns = self.total_latency_ns / self.responses_received;
    }

    pub fn success_rate(&self) -> f64 {
        if self.queries_sent == 0 {
            return 0.0;
        }
        self.responses_received as f64 / self.queries_sent as f64
    }
}

/// DNS cache stats
#[derive(Debug, Clone)]
pub struct DnsCacheStats {
    pub total_entries: u64,
    pub total_lookups: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub negative_hits: u64,
    pub evictions: u64,
    pub expirations: u64,
}

/// Main holistic DNS cache manager
#[derive(Debug)]
pub struct HolisticDnsCache {
    pub entries: BTreeMap<u64, DnsCacheEntry>,
    pub servers: Vec<DnsServerState>,
    pub stats: DnsCacheStats,
    pub max_entries: u32,
    pub negative_ttl_sec: u32,
    pub min_ttl_sec: u32,
    pub max_ttl_sec: u32,
    pub stale_serve_sec: u32,
}

impl HolisticDnsCache {
    pub fn new(max_entries: u32) -> Self {
        Self {
            entries: BTreeMap::new(),
            servers: Vec::new(),
            stats: DnsCacheStats {
                total_entries: 0,
                total_lookups: 0,
                cache_hits: 0,
                cache_misses: 0,
                negative_hits: 0,
                evictions: 0,
                expirations: 0,
            },
            max_entries,
            negative_ttl_sec: 300,
            min_ttl_sec: 30,
            max_ttl_sec: 86400,
            stale_serve_sec: 3600,
        }
    }

    pub fn lookup(&mut self, name_hash: u64, now_ns: u64) -> Option<&DnsCacheEntry> {
        self.stats.total_lookups += 1;
        if let Some(entry) = self.entries.get_mut(&name_hash) {
            if entry.is_expired(now_ns) {
                self.stats.cache_misses += 1;
                return None;
            }
            entry.touch(now_ns);
            if entry.state == DnsCacheState::NegativeCached {
                self.stats.negative_hits += 1;
            } else {
                self.stats.cache_hits += 1;
            }
            return self.entries.get(&name_hash);
        }
        self.stats.cache_misses += 1;
        None
    }

    pub fn insert(&mut self, entry: DnsCacheEntry) {
        let hash = entry.name_hash;
        if self.entries.len() as u32 >= self.max_entries && !self.entries.contains_key(&hash) {
            self.evict_oldest();
        }
        if !self.entries.contains_key(&hash) {
            self.stats.total_entries += 1;
        }
        self.entries.insert(hash, entry);
    }

    pub fn evict_oldest(&mut self) {
        if let Some((&key, _)) = self.entries.iter().min_by_key(|(_, e)| e.last_used_ns) {
            self.entries.remove(&key);
            self.stats.evictions += 1;
            self.stats.total_entries = self.stats.total_entries.saturating_sub(1);
        }
    }

    pub fn gc_expired(&mut self, now_ns: u64) {
        let expired: Vec<u64> = self.entries.iter()
            .filter(|(_, e)| e.is_expired(now_ns))
            .map(|(&k, _)| k)
            .collect();
        for k in expired {
            self.entries.remove(&k);
            self.stats.expirations += 1;
            self.stats.total_entries = self.stats.total_entries.saturating_sub(1);
        }
    }

    pub fn hit_rate(&self) -> f64 {
        if self.stats.total_lookups == 0 {
            return 0.0;
        }
        self.stats.cache_hits as f64 / self.stats.total_lookups as f64
    }

    pub fn add_server(&mut self, server_ip: u32) {
        self.servers.push(DnsServerState::new(server_ip));
    }
}
