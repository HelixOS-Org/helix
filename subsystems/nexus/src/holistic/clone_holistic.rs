// SPDX-License-Identifier: GPL-2.0
//! NEXUS Holistic — Clone (holistic clone analysis)

extern crate alloc;
use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Clone pattern classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticClonePattern {
    ThreadCreate,
    ProcessFork,
    ContainerSpawn,
    NamespaceIsolation,
    Hybrid,
}

/// Clone analysis record
#[derive(Debug, Clone)]
pub struct HolisticCloneRecord {
    pub parent: u64,
    pub child: u64,
    pub pattern: HolisticClonePattern,
    pub flags_count: u32,
    pub ns_created: u32,
    pub latency_us: u64,
}

/// Clone holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct HolisticCloneStats {
    pub total_analyzed: u64,
    pub thread_creates: u64,
    pub process_forks: u64,
    pub container_spawns: u64,
    pub avg_latency_us: f64,
    pub ns_isolation_ratio: f64,
}

/// Manager for holistic clone analysis
pub struct HolisticCloneManager {
    records: Vec<HolisticCloneRecord>,
    parent_clone_count: LinearMap<u32, 64>,
    stats: HolisticCloneStats,
}

impl HolisticCloneManager {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            parent_clone_count: LinearMap::new(),
            stats: HolisticCloneStats {
                total_analyzed: 0,
                thread_creates: 0,
                process_forks: 0,
                container_spawns: 0,
                avg_latency_us: 0.0,
                ns_isolation_ratio: 0.0,
            },
        }
    }

    pub fn analyze_clone(&mut self, parent: u64, child: u64, flags: u32, ns_count: u32, latency: u64) -> HolisticClonePattern {
        let count = self.parent_clone_count.entry(parent).or_insert(0);
        *count += 1;
        let pattern = if ns_count >= 4 {
            self.stats.container_spawns += 1;
            HolisticClonePattern::ContainerSpawn
        } else if ns_count > 0 {
            HolisticClonePattern::NamespaceIsolation
        } else if flags > 4 {
            self.stats.thread_creates += 1;
            HolisticClonePattern::ThreadCreate
        } else {
            self.stats.process_forks += 1;
            HolisticClonePattern::ProcessFork
        };
        let record = HolisticCloneRecord {
            parent,
            child,
            pattern,
            flags_count: flags,
            ns_created: ns_count,
            latency_us: latency,
        };
        self.records.push(record);
        self.stats.total_analyzed += 1;
        let n = self.stats.total_analyzed as f64;
        self.stats.avg_latency_us = (self.stats.avg_latency_us * (n - 1.0) + latency as f64) / n;
        pattern
    }

    #[inline(always)]
    pub fn stats(&self) -> &HolisticCloneStats {
        &self.stats
    }
}
