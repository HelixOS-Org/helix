// SPDX-License-Identifier: GPL-2.0
//! Holistic IP routing — routing table management with policy-based and ECMP routing

extern crate alloc;
use crate::fast::array_map::ArrayMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Route type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteType {
    Connected,
    Static,
    Ospf,
    Bgp,
    Rip,
    Isis,
    Kernel,
    Default,
    Blackhole,
    Unreachable,
    Prohibit,
}

/// Route scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteScope {
    Universe,
    Site,
    Link,
    Host,
    Nowhere,
}

/// Route protocol origin
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteProto {
    Boot,
    Kernel,
    Static,
    Redirect,
    DynamicRouting,
    Dhcp,
    Ra,
}

/// Route next hop
#[derive(Debug, Clone)]
pub struct RouteNextHop {
    pub gateway: u32,
    pub interface_id: u32,
    pub weight: u16,
    pub flags: u16,
    pub reachable: bool,
    pub probe_count: u32,
    pub last_probe_ns: u64,
}

impl RouteNextHop {
    pub fn new(gateway: u32, interface_id: u32, weight: u16) -> Self {
        Self {
            gateway,
            interface_id,
            weight,
            flags: 0,
            reachable: true,
            probe_count: 0,
            last_probe_ns: 0,
        }
    }

    #[inline(always)]
    pub fn probe(&mut self, ts_ns: u64) {
        self.probe_count += 1;
        self.last_probe_ns = ts_ns;
    }
}

/// Routing table entry
#[derive(Debug, Clone)]
pub struct RouteEntry {
    pub prefix: u32,
    pub prefix_len: u8,
    pub route_type: RouteType,
    pub scope: RouteScope,
    pub proto: RouteProto,
    pub metric: u32,
    pub next_hops: Vec<RouteNextHop>,
    pub mtu: u32,
    pub table_id: u32,
    pub hit_count: u64,
    pub last_used_ns: u64,
}

impl RouteEntry {
    pub fn new(prefix: u32, prefix_len: u8, route_type: RouteType) -> Self {
        Self {
            prefix,
            prefix_len,
            route_type,
            scope: RouteScope::Universe,
            proto: RouteProto::Static,
            metric: 100,
            next_hops: Vec::new(),
            mtu: 1500,
            table_id: 254,
            hit_count: 0,
            last_used_ns: 0,
        }
    }

    #[inline(always)]
    pub fn add_next_hop(&mut self, hop: RouteNextHop) {
        self.next_hops.push(hop);
    }

    #[inline]
    pub fn matches(&self, dest_ip: u32) -> bool {
        if self.prefix_len == 0 {
            return true;
        }
        let mask = !((1u32 << (32 - self.prefix_len)) - 1);
        (dest_ip & mask) == (self.prefix & mask)
    }

    pub fn select_next_hop(&self, seed: u64) -> Option<&RouteNextHop> {
        let reachable: Vec<&RouteNextHop> = self.next_hops.iter().filter(|h| h.reachable).collect();
        if reachable.is_empty() {
            return None;
        }
        if reachable.len() == 1 {
            return Some(reachable[0]);
        }
        let total_weight: u64 = reachable.iter().map(|h| h.weight as u64).sum();
        if total_weight == 0 {
            return reachable.first().copied();
        }
        let pick = seed % total_weight;
        let mut acc = 0u64;
        for hop in &reachable {
            acc += hop.weight as u64;
            if pick < acc {
                return Some(hop);
            }
        }
        reachable.last().copied()
    }

    #[inline(always)]
    pub fn ecmp_count(&self) -> usize {
        self.next_hops.iter().filter(|h| h.reachable).count()
    }
}

/// Policy routing rule
#[derive(Debug, Clone)]
pub struct PolicyRule {
    pub priority: u32,
    pub src_prefix: u32,
    pub src_len: u8,
    pub dst_prefix: u32,
    pub dst_len: u8,
    pub fwmark: u32,
    pub table_id: u32,
    pub hit_count: u64,
}

impl PolicyRule {
    pub fn matches_packet(&self, src: u32, dst: u32, mark: u32) -> bool {
        let src_match = if self.src_len == 0 {
            true
        } else {
            let mask = !((1u32 << (32 - self.src_len)) - 1);
            (src & mask) == (self.src_prefix & mask)
        };
        let dst_match = if self.dst_len == 0 {
            true
        } else {
            let mask = !((1u32 << (32 - self.dst_len)) - 1);
            (dst & mask) == (self.dst_prefix & mask)
        };
        let mark_match = self.fwmark == 0 || self.fwmark == mark;
        src_match && dst_match && mark_match
    }
}

/// IP routing stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct IpRoutingStats {
    pub total_routes: u64,
    pub total_lookups: u64,
    pub cache_hits: u64,
    pub ecmp_selections: u64,
    pub policy_matches: u64,
}

/// Main holistic IP routing manager
#[derive(Debug)]
pub struct HolisticIpRouting {
    pub tables: BTreeMap<u32, Vec<RouteEntry>>,
    pub policy_rules: Vec<PolicyRule>,
    pub route_cache: ArrayMap<u64, 32>,
    pub stats: IpRoutingStats,
}

impl HolisticIpRouting {
    pub fn new() -> Self {
        let mut tables = BTreeMap::new();
        tables.insert(254, Vec::new());
        tables.insert(255, Vec::new());
        Self {
            tables,
            policy_rules: Vec::new(),
            route_cache: ArrayMap::new(0),
            stats: IpRoutingStats {
                total_routes: 0,
                total_lookups: 0,
                cache_hits: 0,
                ecmp_selections: 0,
                policy_matches: 0,
            },
        }
    }

    #[inline]
    pub fn add_route(&mut self, table_id: u32, entry: RouteEntry) {
        let table = self.tables.entry(table_id).or_insert_with(Vec::new);
        table.push(entry);
        self.stats.total_routes += 1;
    }

    pub fn lookup(&mut self, dest_ip: u32, seed: u64) -> Option<u32> {
        self.stats.total_lookups += 1;
        if let Some(&cached_gw) = self.route_cache.try_get(dest_ip as usize) {
            self.stats.cache_hits += 1;
            return Some(cached_gw as u32);
        }
        if let Some(table) = self.tables.get_mut(&254) {
            let mut best: Option<(u8, usize)> = None;
            for (i, entry) in table.iter().enumerate() {
                if entry.matches(dest_ip) {
                    if best.is_none() || entry.prefix_len > best.unwrap().0 {
                        best = Some((entry.prefix_len, i));
                    }
                }
            }
            if let Some((_, idx)) = best {
                table[idx].hit_count += 1;
                if let Some(hop) = table[idx].select_next_hop(seed) {
                    let gw = hop.gateway;
                    self.route_cache.insert(dest_ip, gw as u64);
                    if table[idx].ecmp_count() > 1 {
                        self.stats.ecmp_selections += 1;
                    }
                    return Some(gw);
                }
            }
        }
        None
    }

    #[inline]
    pub fn cache_hit_rate(&self) -> f64 {
        if self.stats.total_lookups == 0 {
            return 0.0;
        }
        self.stats.cache_hits as f64 / self.stats.total_lookups as f64
    }

    #[inline(always)]
    pub fn flush_cache(&mut self) {
        self.route_cache.clear();
    }
}
