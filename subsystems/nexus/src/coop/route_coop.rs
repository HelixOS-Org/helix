// SPDX-License-Identifier: GPL-2.0
//! Coop route — cooperative routing table management with shared path discovery

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Coop route protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopRouteProto {
    Static,
    Connected,
    Ospf,
    Bgp,
    Rip,
    Isis,
    Shared,
    Learned,
}

/// Coop route scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopRouteScope {
    Universe,
    Site,
    Link,
    Host,
    Nowhere,
}

/// Route entry
#[derive(Debug, Clone)]
pub struct CoopRouteEntry {
    pub prefix_hash: u64,
    pub prefix_len: u8,
    pub protocol: CoopRouteProto,
    pub scope: CoopRouteScope,
    pub metric: u32,
    pub gateway_hash: u64,
    pub interface_idx: u32,
    pub ref_count: u32,
    pub use_count: u64,
}

impl CoopRouteEntry {
    pub fn new(prefix: &[u8], prefix_len: u8, protocol: CoopRouteProto) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in prefix { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self {
            prefix_hash: h, prefix_len, protocol, scope: CoopRouteScope::Universe,
            metric: 100, gateway_hash: 0, interface_idx: 0, ref_count: 1, use_count: 0,
        }
    }

    pub fn matches(&self, dest_hash: u64) -> bool {
        let mask = if self.prefix_len >= 64 { u64::MAX } else { !((1u64 << (64 - self.prefix_len)) - 1) };
        (dest_hash & mask) == (self.prefix_hash & mask)
    }

    pub fn use_route(&mut self) { self.use_count += 1; }
    pub fn share(&mut self) { self.ref_count += 1; }
    pub fn unshare(&mut self) { self.ref_count = self.ref_count.saturating_sub(1); }
}

/// Shared route table
#[derive(Debug, Clone)]
pub struct SharedRouteTable {
    pub table_id: u32,
    pub routes: Vec<CoopRouteEntry>,
    pub subscribers: Vec<u64>,
    pub version: u64,
}

impl SharedRouteTable {
    pub fn new(table_id: u32) -> Self {
        Self { table_id, routes: Vec::new(), subscribers: Vec::new(), version: 0 }
    }

    pub fn add_route(&mut self, route: CoopRouteEntry) {
        self.routes.push(route);
        self.version += 1;
    }

    pub fn lookup(&mut self, dest_hash: u64) -> Option<&CoopRouteEntry> {
        let mut best: Option<usize> = None;
        let mut best_len: u8 = 0;
        for (i, r) in self.routes.iter().enumerate() {
            if r.matches(dest_hash) && r.prefix_len >= best_len {
                best = Some(i);
                best_len = r.prefix_len;
            }
        }
        if let Some(idx) = best {
            self.routes[idx].use_route();
            Some(&self.routes[idx])
        } else { None }
    }

    pub fn subscribe(&mut self, ns_id: u64) {
        if !self.subscribers.contains(&ns_id) {
            self.subscribers.push(ns_id);
        }
    }
}

/// Coop route stats
#[derive(Debug, Clone)]
pub struct CoopRouteStats {
    pub total_tables: u64,
    pub total_routes: u64,
    pub total_lookups: u64,
    pub shared_tables: u64,
}

/// Main coop route manager
#[derive(Debug)]
pub struct CoopRoute {
    pub tables: BTreeMap<u32, SharedRouteTable>,
    pub stats: CoopRouteStats,
}

impl CoopRoute {
    pub fn new() -> Self {
        Self {
            tables: BTreeMap::new(),
            stats: CoopRouteStats { total_tables: 0, total_routes: 0, total_lookups: 0, shared_tables: 0 },
        }
    }

    pub fn create_table(&mut self, table_id: u32) {
        self.tables.insert(table_id, SharedRouteTable::new(table_id));
        self.stats.total_tables += 1;
    }

    pub fn add_route(&mut self, table_id: u32, route: CoopRouteEntry) -> bool {
        if let Some(table) = self.tables.get_mut(&table_id) {
            table.add_route(route);
            self.stats.total_routes += 1;
            true
        } else { false }
    }
}
