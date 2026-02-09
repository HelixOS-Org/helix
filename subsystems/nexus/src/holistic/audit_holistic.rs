// SPDX-License-Identifier: GPL-2.0
//! Holistic audit — cross-layer audit trail completeness analysis

extern crate alloc;
use alloc::vec::Vec;

/// Audit holistic metric
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditHolisticMetric {
    TrailCompleteness,
    RuleEffectiveness,
    EventCorrelation,
    BufferPressure,
    LogIntegrity,
}

/// Audit finding
#[derive(Debug, Clone)]
pub struct AuditHolisticFinding {
    pub metric: AuditHolisticMetric,
    pub score: u64,
    pub events_logged: u64,
    pub events_dropped: u64,
    pub rules_active: u32,
    pub rules_effective: u32,
}

impl AuditHolisticFinding {
    pub fn new(metric: AuditHolisticMetric) -> Self {
        Self { metric, score: 0, events_logged: 0, events_dropped: 0, rules_active: 0, rules_effective: 0 }
    }

    #[inline(always)]
    pub fn drop_rate(&self) -> f64 {
        let total = self.events_logged + self.events_dropped;
        if total == 0 { 0.0 } else { self.events_dropped as f64 / total as f64 }
    }
}

/// Audit holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AuditHolisticStats {
    pub total_analyses: u64,
    pub high_drop_rates: u64,
    pub ineffective_rules: u64,
    pub integrity_issues: u64,
}

/// Main holistic audit
#[derive(Debug)]
pub struct HolisticAudit {
    pub stats: AuditHolisticStats,
}

impl HolisticAudit {
    pub fn new() -> Self {
        Self { stats: AuditHolisticStats { total_analyses: 0, high_drop_rates: 0, ineffective_rules: 0, integrity_issues: 0 } }
    }

    #[inline]
    pub fn analyze(&mut self, finding: &AuditHolisticFinding) {
        self.stats.total_analyses += 1;
        if finding.drop_rate() > 0.01 { self.stats.high_drop_rates += 1; }
        if finding.rules_active > 0 && finding.rules_effective == 0 { self.stats.ineffective_rules += 1; }
        if matches!(finding.metric, AuditHolisticMetric::LogIntegrity) { self.stats.integrity_issues += 1; }
    }
}
