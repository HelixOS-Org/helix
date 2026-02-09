// SPDX-License-Identifier: GPL-2.0
//! Holistic MAC — cross-layer mandatory access control coherence analysis

extern crate alloc;
use alloc::vec::Vec;

/// MAC holistic metric
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacHolisticMetric {
    PolicyConflict,
    LabelConsistency,
    EnforcementGap,
    ModuleInteraction,
    TransitionSafety,
}

/// MAC finding
#[derive(Debug, Clone)]
pub struct MacHolisticFinding {
    pub metric: MacHolisticMetric,
    pub score: u64,
    pub policy_modules: u32,
    pub conflicts: u32,
    pub label_mismatches: u32,
    pub enforcement_gaps: u32,
}

impl MacHolisticFinding {
    pub fn new(metric: MacHolisticMetric) -> Self {
        Self { metric, score: 0, policy_modules: 0, conflicts: 0, label_mismatches: 0, enforcement_gaps: 0 }
    }
}

/// MAC holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MacHolisticStats {
    pub total_analyses: u64,
    pub policy_conflicts: u64,
    pub label_issues: u64,
    pub gaps_found: u64,
}

/// Main holistic MAC
#[derive(Debug)]
pub struct HolisticMac {
    pub stats: MacHolisticStats,
}

impl HolisticMac {
    pub fn new() -> Self {
        Self { stats: MacHolisticStats { total_analyses: 0, policy_conflicts: 0, label_issues: 0, gaps_found: 0 } }
    }

    #[inline]
    pub fn analyze(&mut self, finding: &MacHolisticFinding) {
        self.stats.total_analyses += 1;
        self.stats.policy_conflicts += finding.conflicts as u64;
        self.stats.label_issues += finding.label_mismatches as u64;
        self.stats.gaps_found += finding.enforcement_gaps as u64;
    }
}
