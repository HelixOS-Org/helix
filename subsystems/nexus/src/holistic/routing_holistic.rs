// SPDX-License-Identifier: GPL-2.0
//! Holistic routing — holistic routing table efficiency analysis

extern crate alloc;

/// Routing efficiency grade
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingEfficiency { Optimal, Suboptimal, Blackhole, Unreachable }

/// Routing holistic record
#[derive(Debug, Clone)]
pub struct RoutingHolisticRecord {
    pub efficiency: RoutingEfficiency,
    pub routes: u32,
    pub lookup_ns: u64,
    pub cache_hit_pct: u8,
}

impl RoutingHolisticRecord {
    pub fn new(efficiency: RoutingEfficiency) -> Self { Self { efficiency, routes: 0, lookup_ns: 0, cache_hit_pct: 0 } }
}

/// Routing holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct RoutingHolisticStats { pub total_samples: u64, pub blackholes: u64, pub avg_lookup_ns: u64, pub max_routes: u32 }

/// Main holistic routing
#[derive(Debug)]
pub struct HolisticRouting {
    pub stats: RoutingHolisticStats,
    lookup_sum: u64,
}

impl HolisticRouting {
    pub fn new() -> Self { Self { stats: RoutingHolisticStats { total_samples: 0, blackholes: 0, avg_lookup_ns: 0, max_routes: 0 }, lookup_sum: 0 } }
    #[inline]
    pub fn record(&mut self, rec: &RoutingHolisticRecord) {
        self.stats.total_samples += 1;
        if rec.efficiency == RoutingEfficiency::Blackhole { self.stats.blackholes += 1; }
        if rec.routes > self.stats.max_routes { self.stats.max_routes = rec.routes; }
        self.lookup_sum += rec.lookup_ns;
        self.stats.avg_lookup_ns = self.lookup_sum / self.stats.total_samples;
    }
}
