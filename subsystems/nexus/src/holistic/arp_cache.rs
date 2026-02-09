// SPDX-License-Identifier: GPL-2.0
//! Holistic ARP cache — neighbor discovery and resolution with NUD state machine

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Neighbor Unreachability Detection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NudState {
    Incomplete,
    Reachable,
    Stale,
    Delay,
    Probe,
    Failed,
    Permanent,
    Noarp,
}

/// ARP operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArpOpType {
    Request,
    Reply,
    ReverseRequest,
    ReverseReply,
    GratuitousRequest,
    GratuitousReply,
}

/// Hardware type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareType {
    Ethernet,
    Loopback,
    Tunnel,
    Infiniband,
    Ieee80211,
    VirtualBridge,
}

/// MAC address (6 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacAddress(pub [u8; 6]);

impl MacAddress {
    #[inline(always)]
    pub fn broadcast() -> Self {
        Self([0xff; 6])
    }

    #[inline(always)]
    pub fn is_broadcast(&self) -> bool {
        self.0 == [0xff; 6]
    }

    #[inline(always)]
    pub fn is_multicast(&self) -> bool {
        self.0[0] & 0x01 != 0
    }

    #[inline]
    pub fn hash(&self) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in &self.0 {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }
}

/// ARP cache entry
#[derive(Debug, Clone)]
pub struct ArpEntry {
    pub ip_addr: u32,
    pub mac: MacAddress,
    pub state: NudState,
    pub hw_type: HardwareType,
    pub interface_id: u32,
    pub created_ns: u64,
    pub updated_ns: u64,
    pub confirmed_ns: u64,
    pub used_ns: u64,
    pub probes_sent: u32,
    pub max_probes: u32,
    pub reachable_time_ms: u32,
    pub gc_staletime_ms: u32,
    pub queue_len: u32,
    pub flags: u32,
}

impl ArpEntry {
    pub fn new(ip_addr: u32, mac: MacAddress, interface_id: u32) -> Self {
        Self {
            ip_addr,
            mac,
            state: NudState::Reachable,
            hw_type: HardwareType::Ethernet,
            interface_id,
            created_ns: 0,
            updated_ns: 0,
            confirmed_ns: 0,
            used_ns: 0,
            probes_sent: 0,
            max_probes: 3,
            reachable_time_ms: 30000,
            gc_staletime_ms: 60000,
            queue_len: 0,
            flags: 0,
        }
    }

    #[inline]
    pub fn confirm(&mut self, ts_ns: u64) {
        self.state = NudState::Reachable;
        self.confirmed_ns = ts_ns;
        self.updated_ns = ts_ns;
        self.probes_sent = 0;
    }

    #[inline]
    pub fn mark_stale(&mut self, ts_ns: u64) {
        if self.state == NudState::Reachable {
            self.state = NudState::Stale;
            self.updated_ns = ts_ns;
        }
    }

    #[inline]
    pub fn start_probe(&mut self, ts_ns: u64) {
        self.state = NudState::Probe;
        self.probes_sent += 1;
        self.updated_ns = ts_ns;
    }

    #[inline(always)]
    pub fn mark_failed(&mut self, ts_ns: u64) {
        self.state = NudState::Failed;
        self.updated_ns = ts_ns;
    }

    #[inline(always)]
    pub fn is_valid(&self) -> bool {
        matches!(self.state, NudState::Reachable | NudState::Stale | NudState::Delay | NudState::Probe | NudState::Permanent)
    }

    #[inline(always)]
    pub fn age_ms(&self, now_ns: u64) -> u64 {
        (now_ns.saturating_sub(self.updated_ns)) / 1_000_000
    }

    pub fn needs_probe(&self, now_ns: u64) -> bool {
        if self.state == NudState::Stale || self.state == NudState::Delay {
            return true;
        }
        if self.state == NudState::Probe && self.probes_sent < self.max_probes {
            return true;
        }
        if self.state == NudState::Reachable {
            let age = self.age_ms(now_ns);
            return age > self.reachable_time_ms as u64;
        }
        false
    }
}

/// ARP cache stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ArpCacheStats {
    pub total_entries: u64,
    pub reachable_entries: u64,
    pub stale_entries: u64,
    pub failed_entries: u64,
    pub total_lookups: u64,
    pub cache_hits: u64,
    pub probes_sent: u64,
    pub gc_runs: u64,
}

/// Main holistic ARP cache manager
#[derive(Debug)]
#[repr(align(64))]
pub struct HolisticArpCache {
    pub entries: BTreeMap<u32, ArpEntry>,
    pub stats: ArpCacheStats,
    pub max_entries: u32,
    pub base_reachable_ms: u32,
    pub gc_interval_ms: u32,
    pub gc_thresh1: u32,
    pub gc_thresh2: u32,
    pub gc_thresh3: u32,
}

impl HolisticArpCache {
    pub fn new(max_entries: u32) -> Self {
        Self {
            entries: BTreeMap::new(),
            stats: ArpCacheStats {
                total_entries: 0,
                reachable_entries: 0,
                stale_entries: 0,
                failed_entries: 0,
                total_lookups: 0,
                cache_hits: 0,
                probes_sent: 0,
                gc_runs: 0,
            },
            max_entries,
            base_reachable_ms: 30000,
            gc_interval_ms: 30000,
            gc_thresh1: 128,
            gc_thresh2: 512,
            gc_thresh3: 1024,
        }
    }

    #[inline]
    pub fn lookup(&mut self, ip_addr: u32) -> Option<&ArpEntry> {
        self.stats.total_lookups += 1;
        if let Some(entry) = self.entries.get(&ip_addr) {
            if entry.is_valid() {
                self.stats.cache_hits += 1;
                return Some(entry);
            }
        }
        None
    }

    pub fn insert(&mut self, entry: ArpEntry) {
        let ip = entry.ip_addr;
        if !self.entries.contains_key(&ip) {
            self.stats.total_entries += 1;
        }
        match entry.state {
            NudState::Reachable => self.stats.reachable_entries += 1,
            NudState::Stale => self.stats.stale_entries += 1,
            NudState::Failed => self.stats.failed_entries += 1,
            _ => {}
        }
        self.entries.insert(ip, entry);
    }

    #[inline]
    pub fn gc(&mut self, now_ns: u64) {
        self.stats.gc_runs += 1;
        let stale_keys: Vec<u32> = self.entries.iter()
            .filter(|(_, e)| e.state == NudState::Failed || e.age_ms(now_ns) > e.gc_staletime_ms as u64)
            .map(|(&k, _)| k)
            .collect();
        for k in stale_keys {
            self.entries.remove(&k);
            self.stats.total_entries = self.stats.total_entries.saturating_sub(1);
        }
    }

    #[inline]
    pub fn hit_rate(&self) -> f64 {
        if self.stats.total_lookups == 0 {
            return 0.0;
        }
        self.stats.cache_hits as f64 / self.stats.total_lookups as f64
    }
}
