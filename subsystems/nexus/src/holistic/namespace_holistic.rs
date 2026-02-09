// SPDX-License-Identifier: GPL-2.0
//! Holistic namespace — cross-layer namespace isolation analysis

extern crate alloc;
use alloc::vec::Vec;

/// Namespace holistic metric
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NsHolisticMetric {
    IsolationStrength,
    ResourceLeakage,
    NestingDepth,
    MappingConsistency,
    CrossNsAccess,
}

/// Namespace type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NsHolisticType {
    User,
    Pid,
    Net,
    Mount,
    Uts,
    Ipc,
    Cgroup,
    Time,
}

/// Namespace finding
#[derive(Debug, Clone)]
pub struct NsHolisticFinding {
    pub metric: NsHolisticMetric,
    pub ns_type: NsHolisticType,
    pub score: u64,
    pub nesting_depth: u32,
    pub cross_ns_accesses: u32,
    pub leaked_resources: u32,
}

impl NsHolisticFinding {
    pub fn new(metric: NsHolisticMetric, ns_type: NsHolisticType) -> Self {
        Self { metric, ns_type, score: 0, nesting_depth: 0, cross_ns_accesses: 0, leaked_resources: 0 }
    }
}

/// Namespace holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct NsHolisticStats {
    pub total_analyses: u64,
    pub weak_isolation: u64,
    pub resource_leaks: u64,
    pub deep_nesting: u64,
}

/// Main holistic namespace
#[derive(Debug)]
pub struct HolisticNamespace {
    pub stats: NsHolisticStats,
}

impl HolisticNamespace {
    pub fn new() -> Self {
        Self { stats: NsHolisticStats { total_analyses: 0, weak_isolation: 0, resource_leaks: 0, deep_nesting: 0 } }
    }

    #[inline]
    pub fn analyze(&mut self, finding: &NsHolisticFinding) {
        self.stats.total_analyses += 1;
        if finding.cross_ns_accesses > 3 { self.stats.weak_isolation += 1; }
        if finding.leaked_resources > 0 { self.stats.resource_leaks += finding.leaked_resources as u64; }
        if finding.nesting_depth > 4 { self.stats.deep_nesting += 1; }
    }
}
