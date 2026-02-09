// SPDX-License-Identifier: GPL-2.0
//! Holistic keyring — cross-layer key management health analysis

extern crate alloc;
use alloc::vec::Vec;

/// Keyring holistic metric
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyringHolisticMetric {
    KeyExpiry,
    QuotaPressure,
    OrphanedKeys,
    TypeDistribution,
    AccessPattern,
}

/// Keyring finding
#[derive(Debug, Clone)]
pub struct KeyringHolisticFinding {
    pub metric: KeyringHolisticMetric,
    pub score: u64,
    pub total_keys: u32,
    pub expired_keys: u32,
    pub orphaned_keys: u32,
    pub quota_used_pct: u32,
}

impl KeyringHolisticFinding {
    pub fn new(metric: KeyringHolisticMetric) -> Self {
        Self { metric, score: 0, total_keys: 0, expired_keys: 0, orphaned_keys: 0, quota_used_pct: 0 }
    }

    #[inline]
    pub fn health_score(&self) -> f64 {
        if self.total_keys == 0 { return 1.0; }
        let healthy = self.total_keys.saturating_sub(self.expired_keys + self.orphaned_keys);
        healthy as f64 / self.total_keys as f64
    }
}

/// Keyring holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct KeyringHolisticStats {
    pub total_analyses: u64,
    pub expiry_warnings: u64,
    pub orphan_detections: u64,
    pub quota_alerts: u64,
}

/// Main holistic keyring
#[derive(Debug)]
pub struct HolisticKeyring {
    pub stats: KeyringHolisticStats,
}

impl HolisticKeyring {
    pub fn new() -> Self {
        Self { stats: KeyringHolisticStats { total_analyses: 0, expiry_warnings: 0, orphan_detections: 0, quota_alerts: 0 } }
    }

    #[inline]
    pub fn analyze(&mut self, finding: &KeyringHolisticFinding) {
        self.stats.total_analyses += 1;
        if finding.expired_keys > 0 { self.stats.expiry_warnings += 1; }
        if finding.orphaned_keys > 0 { self.stats.orphan_detections += 1; }
        if finding.quota_used_pct > 80 { self.stats.quota_alerts += 1; }
    }
}
