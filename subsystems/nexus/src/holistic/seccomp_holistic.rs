// SPDX-License-Identifier: GPL-2.0
//! Holistic seccomp — cross-layer seccomp filter analysis

extern crate alloc;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

/// Seccomp holistic metric
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompHolisticMetric {
    FilterComplexity,
    SyscallCoverage,
    NotifLatency,
    FilterOverlap,
    AttackSurface,
}

/// Seccomp holistic finding
#[derive(Debug, Clone)]
pub struct SeccompFinding {
    pub metric: SeccompHolisticMetric,
    pub score: u64,
    pub pid: u32,
    pub filter_count: u32,
    pub covered_syscalls: u32,
    pub total_syscalls: u32,
}

impl SeccompFinding {
    pub fn new(metric: SeccompHolisticMetric) -> Self {
        Self { metric, score: 0, pid: 0, filter_count: 0, covered_syscalls: 0, total_syscalls: 0 }
    }

    #[inline(always)]
    pub fn coverage_ratio(&self) -> f64 {
        if self.total_syscalls == 0 { 0.0 } else { self.covered_syscalls as f64 / self.total_syscalls as f64 }
    }
}

/// Seccomp holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SeccompHolisticStats {
    pub total_analyses: u64,
    pub high_complexity: u64,
    pub low_coverage: u64,
    pub avg_filter_count: f64,
}

/// Main holistic seccomp
#[derive(Debug)]
pub struct HolisticSeccomp {
    pub stats: SeccompHolisticStats,
}

impl HolisticSeccomp {
    pub fn new() -> Self {
        Self { stats: SeccompHolisticStats { total_analyses: 0, high_complexity: 0, low_coverage: 0, avg_filter_count: 0.0 } }
    }

    #[inline]
    pub fn analyze(&mut self, finding: &SeccompFinding) {
        self.stats.total_analyses += 1;
        if finding.filter_count > 16 { self.stats.high_complexity += 1; }
        if finding.coverage_ratio() < 0.5 { self.stats.low_coverage += 1; }
        let n = self.stats.total_analyses as f64;
        self.stats.avg_filter_count = self.stats.avg_filter_count * ((n - 1.0) / n) + finding.filter_count as f64 / n;
    }
}
