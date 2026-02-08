// SPDX-License-Identifier: GPL-2.0
//! Holistic quota — disk quota analysis with usage trends

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Quota type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticQuotaType {
    User,
    Group,
    Project,
}

/// Quota state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaState {
    Ok,
    SoftLimitExceeded,
    HardLimitReached,
    GracePeriodExpired,
}

/// Quota entry
#[derive(Debug, Clone)]
pub struct HolisticQuotaEntry {
    pub id: u32,
    pub quota_type: HolisticQuotaType,
    pub block_soft: u64,
    pub block_hard: u64,
    pub block_used: u64,
    pub inode_soft: u64,
    pub inode_hard: u64,
    pub inode_used: u64,
    pub grace_ns: u64,
}

impl HolisticQuotaEntry {
    pub fn new(id: u32, qt: HolisticQuotaType) -> Self {
        Self { id, quota_type: qt, block_soft: 0, block_hard: 0, block_used: 0, inode_soft: 0, inode_hard: 0, inode_used: 0, grace_ns: 0 }
    }

    pub fn state(&self) -> QuotaState {
        if self.block_hard > 0 && self.block_used >= self.block_hard { QuotaState::HardLimitReached }
        else if self.block_soft > 0 && self.block_used >= self.block_soft { QuotaState::SoftLimitExceeded }
        else { QuotaState::Ok }
    }

    pub fn block_usage_pct(&self) -> f64 {
        if self.block_hard == 0 { 0.0 } else { self.block_used as f64 / self.block_hard as f64 }
    }

    pub fn inode_usage_pct(&self) -> f64 {
        if self.inode_hard == 0 { 0.0 } else { self.inode_used as f64 / self.inode_hard as f64 }
    }
}

/// Holistic quota stats
#[derive(Debug, Clone)]
pub struct HolisticQuotaStats {
    pub total_quotas: u64,
    pub over_soft: u64,
    pub over_hard: u64,
    pub checks: u64,
}

/// Main holistic quota
#[derive(Debug)]
pub struct HolisticQuota {
    pub quotas: BTreeMap<u64, HolisticQuotaEntry>,
    pub stats: HolisticQuotaStats,
}

impl HolisticQuota {
    pub fn new() -> Self {
        Self { quotas: BTreeMap::new(), stats: HolisticQuotaStats { total_quotas: 0, over_soft: 0, over_hard: 0, checks: 0 } }
    }

    pub fn set_quota(&mut self, key: u64, entry: HolisticQuotaEntry) {
        self.stats.total_quotas += 1;
        self.quotas.insert(key, entry);
    }

    pub fn check(&mut self, key: u64) -> QuotaState {
        self.stats.checks += 1;
        if let Some(q) = self.quotas.get(&key) {
            let s = q.state();
            match s {
                QuotaState::SoftLimitExceeded => self.stats.over_soft += 1,
                QuotaState::HardLimitReached => self.stats.over_hard += 1,
                _ => {}
            }
            s
        } else {
            QuotaState::Ok
        }
    }
}

// ============================================================================
// Merged from quota_v2_holistic
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticQuotaV2Metric {
    UsageRatio,
    GracePeriodRemaining,
    DenialRate,
    WarningFrequency,
    AllocationPattern,
    FairnessIndex,
}

/// Quota analysis sample
#[derive(Debug, Clone)]
pub struct HolisticQuotaV2Sample {
    pub metric: HolisticQuotaV2Metric,
    pub value: u64,
    pub owner_id: u32,
    pub timestamp: u64,
}

/// Quota health assessment
#[derive(Debug, Clone)]
pub struct HolisticQuotaV2Health {
    pub usage_health: u64,
    pub fairness_score: u64,
    pub enforcement_score: u64,
    pub overall: u64,
}

/// Stats for quota analysis
#[derive(Debug, Clone)]
pub struct HolisticQuotaV2Stats {
    pub samples: u64,
    pub analyses: u64,
    pub quota_exceeded_alerts: u64,
    pub fairness_warnings: u64,
}

/// Manager for quota holistic analysis
pub struct HolisticQuotaV2Manager {
    samples: Vec<HolisticQuotaV2Sample>,
    per_owner: BTreeMap<u32, Vec<HolisticQuotaV2Sample>>,
    health: HolisticQuotaV2Health,
    stats: HolisticQuotaV2Stats,
}

impl HolisticQuotaV2Manager {
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
            per_owner: BTreeMap::new(),
            health: HolisticQuotaV2Health {
                usage_health: 100,
                fairness_score: 100,
                enforcement_score: 100,
                overall: 100,
            },
            stats: HolisticQuotaV2Stats {
                samples: 0,
                analyses: 0,
                quota_exceeded_alerts: 0,
                fairness_warnings: 0,
            },
        }
    }

    pub fn record(&mut self, metric: HolisticQuotaV2Metric, value: u64, owner_id: u32) {
        let sample = HolisticQuotaV2Sample {
            metric,
            value,
            owner_id,
            timestamp: self.samples.len() as u64,
        };
        self.per_owner.entry(owner_id).or_insert_with(Vec::new).push(sample.clone());
        self.samples.push(sample);
        self.stats.samples += 1;
    }

    pub fn analyze(&mut self) -> &HolisticQuotaV2Health {
        self.stats.analyses += 1;
        let usage: Vec<&HolisticQuotaV2Sample> = self.samples.iter()
            .filter(|s| matches!(s.metric, HolisticQuotaV2Metric::UsageRatio))
            .collect();
        if !usage.is_empty() {
            let avg: u64 = usage.iter().map(|s| s.value).sum::<u64>() / usage.len() as u64;
            self.health.usage_health = 100u64.saturating_sub(avg);
            if avg > 90 {
                self.stats.quota_exceeded_alerts += 1;
            }
        }
        let denial: Vec<&HolisticQuotaV2Sample> = self.samples.iter()
            .filter(|s| matches!(s.metric, HolisticQuotaV2Metric::DenialRate))
            .collect();
        if !denial.is_empty() {
            let avg: u64 = denial.iter().map(|s| s.value).sum::<u64>() / denial.len() as u64;
            self.health.enforcement_score = 100u64.saturating_sub(avg.min(100));
        }
        self.health.overall = (self.health.usage_health + self.health.fairness_score + self.health.enforcement_score) / 3;
        &self.health
    }

    pub fn stats(&self) -> &HolisticQuotaV2Stats {
        &self.stats
    }
}
