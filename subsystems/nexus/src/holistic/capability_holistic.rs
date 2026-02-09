// SPDX-License-Identifier: GPL-2.0
//! Holistic capability — cross-layer capability privilege analysis

extern crate alloc;
use alloc::vec::Vec;

/// Capability holistic metric
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapHolisticMetric {
    PrivilegeExcess,
    CapUsageRatio,
    BoundingSetReduction,
    AmbientRisk,
    NamespaceEscalation,
}

/// Capability finding
#[derive(Debug, Clone)]
pub struct CapHolisticFinding {
    pub metric: CapHolisticMetric,
    pub score: u64,
    pub pid: u32,
    pub caps_held: u32,
    pub caps_used: u32,
    pub risk_level: u32,
}

impl CapHolisticFinding {
    pub fn new(metric: CapHolisticMetric) -> Self {
        Self { metric, score: 0, pid: 0, caps_held: 0, caps_used: 0, risk_level: 0 }
    }

    #[inline(always)]
    pub fn usage_ratio(&self) -> f64 {
        if self.caps_held == 0 { 0.0 } else { self.caps_used as f64 / self.caps_held as f64 }
    }
}

/// Capability holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CapHolisticStats {
    pub total_analyses: u64,
    pub excessive_privileges: u64,
    pub avg_usage_ratio: f64,
    pub high_risk: u64,
}

/// Main holistic capability
#[derive(Debug)]
pub struct HolisticCapability {
    pub stats: CapHolisticStats,
}

impl HolisticCapability {
    pub fn new() -> Self {
        Self { stats: CapHolisticStats { total_analyses: 0, excessive_privileges: 0, avg_usage_ratio: 0.0, high_risk: 0 } }
    }

    #[inline]
    pub fn analyze(&mut self, finding: &CapHolisticFinding) {
        self.stats.total_analyses += 1;
        if finding.usage_ratio() < 0.3 { self.stats.excessive_privileges += 1; }
        if finding.risk_level > 7 { self.stats.high_risk += 1; }
        let n = self.stats.total_analyses as f64;
        self.stats.avg_usage_ratio = self.stats.avg_usage_ratio * ((n - 1.0) / n) + finding.usage_ratio() / n;
    }
}
