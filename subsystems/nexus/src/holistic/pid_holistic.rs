// SPDX-License-Identifier: GPL-2.0
//! NEXUS Holistic — PID (holistic PID namespace analysis)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// PID namespace health
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticPidHealth {
    Healthy,
    NearLimit,
    Fragmented,
    Exhausted,
}

/// PID analysis entry
#[derive(Debug, Clone)]
pub struct HolisticPidEntry {
    pub ns_id: u64,
    pub depth: u32,
    pub pid_count: u32,
    pub max_pid: u64,
    pub fragmentation: f64,
    pub health: HolisticPidHealth,
}

/// PID holistic stats
#[derive(Debug, Clone)]
pub struct HolisticPidStats {
    pub total_namespaces: u64,
    pub healthy: u64,
    pub near_limit: u64,
    pub fragmented: u64,
    pub max_depth: u32,
    pub avg_utilization: f64,
}

/// Manager for holistic PID analysis
pub struct HolisticPidManager {
    entries: BTreeMap<u64, HolisticPidEntry>,
    stats: HolisticPidStats,
}

impl HolisticPidManager {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            stats: HolisticPidStats {
                total_namespaces: 0,
                healthy: 0,
                near_limit: 0,
                fragmented: 0,
                max_depth: 0,
                avg_utilization: 0.0,
            },
        }
    }

    pub fn analyze_namespace(&mut self, ns_id: u64, depth: u32, count: u32, max: u64, frag: f64) -> HolisticPidHealth {
        let health = if frag > 0.7 {
            self.stats.fragmented += 1;
            HolisticPidHealth::Fragmented
        } else if count as u64 > max * 9 / 10 {
            self.stats.near_limit += 1;
            HolisticPidHealth::NearLimit
        } else {
            self.stats.healthy += 1;
            HolisticPidHealth::Healthy
        };
        if depth > self.stats.max_depth {
            self.stats.max_depth = depth;
        }
        let entry = HolisticPidEntry {
            ns_id,
            depth,
            pid_count: count,
            max_pid: max,
            fragmentation: frag,
            health,
        };
        self.entries.insert(ns_id, entry);
        self.stats.total_namespaces += 1;
        health
    }

    pub fn stats(&self) -> &HolisticPidStats {
        &self.stats
    }
}
