// SPDX-License-Identifier: GPL-2.0
//! Holistic LSM — cross-layer LSM stacking analysis

extern crate alloc;
use alloc::vec::Vec;

/// LSM holistic metric
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LsmHolisticMetric {
    ModuleConflict,
    HookLatency,
    PolicyCoherence,
    StackDepth,
    DecisionAgreement,
}

/// LSM holistic finding
#[derive(Debug, Clone)]
pub struct LsmFinding {
    pub metric: LsmHolisticMetric,
    pub score: u64,
    pub active_modules: u32,
    pub conflict_count: u32,
    pub agreement_rate: f64,
}

impl LsmFinding {
    pub fn new(metric: LsmHolisticMetric) -> Self {
        Self { metric, score: 0, active_modules: 0, conflict_count: 0, agreement_rate: 1.0 }
    }
}

/// LSM holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct LsmHolisticStats {
    pub total_analyses: u64,
    pub conflicts_detected: u64,
    pub avg_agreement: f64,
    pub max_stack_depth: u32,
}

/// Main holistic LSM
#[derive(Debug)]
pub struct HolisticLsm {
    pub stats: LsmHolisticStats,
}

impl HolisticLsm {
    pub fn new() -> Self {
        Self { stats: LsmHolisticStats { total_analyses: 0, conflicts_detected: 0, avg_agreement: 1.0, max_stack_depth: 0 } }
    }

    #[inline]
    pub fn analyze(&mut self, finding: &LsmFinding) {
        self.stats.total_analyses += 1;
        self.stats.conflicts_detected += finding.conflict_count as u64;
        if finding.active_modules > self.stats.max_stack_depth { self.stats.max_stack_depth = finding.active_modules; }
        let n = self.stats.total_analyses as f64;
        self.stats.avg_agreement = self.stats.avg_agreement * ((n - 1.0) / n) + finding.agreement_rate / n;
    }
}
