// SPDX-License-Identifier: GPL-2.0
//! Holistic credential — cross-layer credential flow analysis

extern crate alloc;
use alloc::vec::Vec;

/// Credential holistic metric
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredHolisticMetric {
    UidFlowAnomaly,
    PrivilegeEscalation,
    CredentialReuse,
    NamespaceLeakage,
    IdentityConsistency,
}

/// Credential finding
#[derive(Debug, Clone)]
pub struct CredHolisticFinding {
    pub metric: CredHolisticMetric,
    pub score: u64,
    pub pid: u32,
    pub uid_transitions: u32,
    pub ns_crossings: u32,
    pub escalations: u32,
}

impl CredHolisticFinding {
    pub fn new(metric: CredHolisticMetric) -> Self {
        Self { metric, score: 0, pid: 0, uid_transitions: 0, ns_crossings: 0, escalations: 0 }
    }
}

/// Credential holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CredHolisticStats {
    pub total_analyses: u64,
    pub anomalies: u64,
    pub escalation_attempts: u64,
    pub ns_leakages: u64,
}

/// Main holistic credential
#[derive(Debug)]
pub struct HolisticCredential {
    pub stats: CredHolisticStats,
}

impl HolisticCredential {
    pub fn new() -> Self {
        Self { stats: CredHolisticStats { total_analyses: 0, anomalies: 0, escalation_attempts: 0, ns_leakages: 0 } }
    }

    #[inline]
    pub fn analyze(&mut self, finding: &CredHolisticFinding) {
        self.stats.total_analyses += 1;
        if finding.escalations > 0 { self.stats.escalation_attempts += finding.escalations as u64; }
        if finding.ns_crossings > 2 { self.stats.ns_leakages += 1; }
        if matches!(finding.metric, CredHolisticMetric::UidFlowAnomaly | CredHolisticMetric::CredentialReuse) {
            self.stats.anomalies += 1;
        }
    }
}
