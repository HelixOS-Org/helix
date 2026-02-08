// SPDX-License-Identifier: GPL-2.0
//! Coop routing — cooperative routing table coordination

extern crate alloc;

/// Routing coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingCoopEvent { TableSync, RouteShare, NexthopGroup, PolicyMerge }

/// Routing coop record
#[derive(Debug, Clone)]
pub struct RoutingCoopRecord {
    pub event: RoutingCoopEvent,
    pub routes: u32,
    pub nexthops: u32,
    pub prefix_len: u8,
}

impl RoutingCoopRecord {
    pub fn new(event: RoutingCoopEvent) -> Self { Self { event, routes: 0, nexthops: 0, prefix_len: 0 } }
}

/// Routing coop stats
#[derive(Debug, Clone)]
pub struct RoutingCoopStats { pub total_events: u64, pub syncs: u64, pub shared_routes: u64, pub groups: u64 }

/// Main coop routing
#[derive(Debug)]
pub struct CoopRouting { pub stats: RoutingCoopStats }

impl CoopRouting {
    pub fn new() -> Self { Self { stats: RoutingCoopStats { total_events: 0, syncs: 0, shared_routes: 0, groups: 0 } } }
    pub fn record(&mut self, rec: &RoutingCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            RoutingCoopEvent::TableSync => self.stats.syncs += 1,
            RoutingCoopEvent::RouteShare | RoutingCoopEvent::PolicyMerge => self.stats.shared_routes += 1,
            RoutingCoopEvent::NexthopGroup => self.stats.groups += 1,
        }
    }
}
