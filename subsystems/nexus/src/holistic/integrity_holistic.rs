// SPDX-License-Identifier: GPL-2.0
//! Holistic integrity — cross-layer integrity measurement analysis

extern crate alloc;
use alloc::vec::Vec;

/// Integrity holistic metric
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrityHolisticMetric {
    MeasurementGap,
    AppraisalFailRate,
    DigestStale,
    PolicyCoverage,
    TamperDetection,
}

/// Integrity finding
#[derive(Debug, Clone)]
pub struct IntegrityHolisticFinding {
    pub metric: IntegrityHolisticMetric,
    pub score: u64,
    pub measured_files: u64,
    pub total_files: u64,
    pub stale_digests: u32,
    pub appraisal_failures: u32,
}

impl IntegrityHolisticFinding {
    pub fn new(metric: IntegrityHolisticMetric) -> Self {
        Self { metric, score: 0, measured_files: 0, total_files: 0, stale_digests: 0, appraisal_failures: 0 }
    }

    pub fn coverage(&self) -> f64 {
        if self.total_files == 0 { 0.0 } else { self.measured_files as f64 / self.total_files as f64 }
    }
}

/// Integrity holistic stats
#[derive(Debug, Clone)]
pub struct IntegrityHolisticStats {
    pub total_analyses: u64,
    pub coverage_gaps: u64,
    pub stale_detections: u64,
    pub tamper_alerts: u64,
}

/// Main holistic integrity
#[derive(Debug)]
pub struct HolisticIntegrity {
    pub stats: IntegrityHolisticStats,
}

impl HolisticIntegrity {
    pub fn new() -> Self {
        Self { stats: IntegrityHolisticStats { total_analyses: 0, coverage_gaps: 0, stale_detections: 0, tamper_alerts: 0 } }
    }

    pub fn analyze(&mut self, finding: &IntegrityHolisticFinding) {
        self.stats.total_analyses += 1;
        if finding.coverage() < 0.8 { self.stats.coverage_gaps += 1; }
        self.stats.stale_detections += finding.stale_digests as u64;
        if matches!(finding.metric, IntegrityHolisticMetric::TamperDetection) { self.stats.tamper_alerts += 1; }
    }
}
