//! # Coop Fairness Monitor
//!
//! System-wide fairness monitoring for cooperative resource sharing:
//! - Gini coefficient computation for resource distribution
//! - Jain's fairness index tracking
//! - Starvation detection and alerts
//! - Historical fairness trending
//! - Per-resource fairness metrics

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;

use crate::fast::linear_map::LinearMap;

// ============================================================================
// FAIRNESS TYPES
// ============================================================================

/// Fairness resource
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FairnessResource {
    /// CPU time
    CpuTime,
    /// Memory allocation
    Memory,
    /// IO bandwidth
    IoBandwidth,
    /// Network bandwidth
    NetworkBandwidth,
    /// IPC channel slots
    IpcSlots,
    /// Cache occupancy
    CacheOccupancy,
}

/// Fairness verdict
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FairnessVerdict {
    /// Fair distribution
    Fair,
    /// Slightly unfair
    Marginal,
    /// Significantly unfair
    Unfair,
    /// Severely unfair (starvation likely)
    Severe,
}

/// Starvation alert
#[derive(Debug, Clone)]
pub struct StarvationAlert {
    /// Affected PID
    pub pid: u64,
    /// Resource
    pub resource: FairnessResource,
    /// Duration starved (ns)
    pub duration_ns: u64,
    /// Share received (fraction of fair share)
    pub share_received: f64,
    /// Timestamp
    pub timestamp_ns: u64,
}

// ============================================================================
// FAIRNESS METRICS
// ============================================================================

/// Per-resource fairness state
#[derive(Debug, Clone)]
pub struct ResourceFairness {
    /// Resource
    pub resource: FairnessResource,
    /// Per-process allocation
    allocations: LinearMap<f64, 64>,
    /// Jain's fairness index
    pub jains_index: f64,
    /// Gini coefficient
    pub gini: f64,
    /// Max/min ratio
    pub max_min_ratio: f64,
    /// Coefficient of variation
    pub cv: f64,
    /// Verdict
    pub verdict: FairnessVerdict,
    /// History of Jain's index
    jains_history: VecDeque<f64>,
}

impl ResourceFairness {
    pub fn new(resource: FairnessResource) -> Self {
        Self {
            resource,
            allocations: LinearMap::new(),
            jains_index: 1.0,
            gini: 0.0,
            max_min_ratio: 1.0,
            cv: 0.0,
            verdict: FairnessVerdict::Fair,
            jains_history: VecDeque::new(),
        }
    }

    /// Update allocation for process
    #[inline(always)]
    pub fn set_allocation(&mut self, pid: u64, amount: f64) {
        self.allocations.insert(pid, amount);
    }

    /// Remove process
    #[inline(always)]
    pub fn remove_process(&mut self, pid: u64) {
        self.allocations.remove(pid);
    }

    /// Recompute fairness metrics
    pub fn recompute(&mut self) {
        let values: Vec<f64> = self.allocations.values().cloned().collect();
        let n = values.len();
        if n < 2 {
            self.jains_index = 1.0;
            self.gini = 0.0;
            self.max_min_ratio = 1.0;
            self.cv = 0.0;
            self.verdict = FairnessVerdict::Fair;
            return;
        }

        // Jain's fairness index: (sum(xi))^2 / (n * sum(xi^2))
        let sum: f64 = values.iter().sum();
        let sum_sq: f64 = values.iter().map(|x| x * x).sum();
        self.jains_index = if sum_sq > 0.0 {
            (sum * sum) / (n as f64 * sum_sq)
        } else {
            1.0
        };

        // Gini coefficient
        let mean = sum / n as f64;
        let mut abs_diff_sum = 0.0;
        for i in 0..n {
            for j in 0..n {
                abs_diff_sum += libm::fabs(values[i] - values[j]);
            }
        }
        self.gini = if mean > 0.0 {
            abs_diff_sum / (2.0 * n as f64 * n as f64 * mean)
        } else {
            0.0
        };

        // Max/min ratio
        let max_val = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min_val = values.iter().cloned().fold(f64::INFINITY, f64::min);
        self.max_min_ratio = if min_val > 0.0 {
            max_val / min_val
        } else {
            f64::INFINITY
        };

        // Coefficient of variation
        let variance = sum_sq / n as f64 - mean * mean;
        let stddev = libm::sqrt(if variance > 0.0 { variance } else { 0.0 });
        self.cv = if mean > 0.0 { stddev / mean } else { 0.0 };

        // Verdict
        self.verdict = if self.jains_index > 0.95 {
            FairnessVerdict::Fair
        } else if self.jains_index > 0.8 {
            FairnessVerdict::Marginal
        } else if self.jains_index > 0.5 {
            FairnessVerdict::Unfair
        } else {
            FairnessVerdict::Severe
        };

        // Track history
        if self.jains_history.len() >= 128 {
            self.jains_history.pop_front();
        }
        self.jains_history.push_back(self.jains_index);
    }

    /// Fairness trend (positive = improving)
    #[inline]
    pub fn trend(&self) -> f64 {
        if self.jains_history.len() < 4 {
            return 0.0;
        }
        let n = self.jains_history.len();
        let recent: f64 = self.jains_history.iter().skip(n - 2).sum::<f64>() / 2.0;
        let earlier: f64 = self.jains_history.iter().take(2).sum::<f64>() / 2.0;
        recent - earlier
    }

    /// Detect starved processes (getting < threshold fraction of fair share)
    pub fn detect_starvation(&self, threshold: f64) -> Vec<u64> {
        let n = self.allocations.len();
        if n == 0 {
            return Vec::new();
        }
        let total: f64 = self.allocations.values().sum();
        let fair_share = total / n as f64;

        self.allocations
            .iter()
            .filter(|(_, &v)| v < fair_share * threshold)
            .map(|(&pid, _)| pid)
            .collect()
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Fairness monitor stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CoopFairnessStats {
    /// Tracked resources
    pub tracked_resources: usize,
    /// Overall Jain's index (average across resources)
    pub overall_jains: f64,
    /// Starvation alerts
    pub starvation_alerts: usize,
    /// Resources with verdict Unfair or worse
    pub unfair_resources: usize,
}

/// Coop fairness monitor
pub struct CoopFairnessMonitor {
    /// Per-resource fairness
    resources: BTreeMap<u8, ResourceFairness>,
    /// Starvation alerts
    alerts: VecDeque<StarvationAlert>,
    /// Stats
    stats: CoopFairnessStats,
}

impl CoopFairnessMonitor {
    pub fn new() -> Self {
        Self {
            resources: BTreeMap::new(),
            alerts: VecDeque::new(),
            stats: CoopFairnessStats::default(),
        }
    }

    /// Get/create resource tracker
    #[inline(always)]
    pub fn resource(&mut self, resource: FairnessResource) -> &mut ResourceFairness {
        self.resources
            .entry(resource as u8)
            .or_insert_with(|| ResourceFairness::new(resource))
    }

    /// Update allocation
    #[inline]
    pub fn update_allocation(&mut self, resource: FairnessResource, pid: u64, amount: f64) {
        let rf = self
            .resources
            .entry(resource as u8)
            .or_insert_with(|| ResourceFairness::new(resource));
        rf.set_allocation(pid, amount);
    }

    /// Recompute all
    #[inline]
    pub fn recompute_all(&mut self) {
        for rf in self.resources.values_mut() {
            rf.recompute();
        }
        self.check_starvation(0.2); // 20% of fair share
        self.update_stats();
    }

    /// Check starvation across all resources
    fn check_starvation(&mut self, threshold: f64) {
        for rf in self.resources.values() {
            let starved = rf.detect_starvation(threshold);
            for pid in starved {
                if self.alerts.len() >= 256 {
                    self.alerts.pop_front();
                }
                let alloc = rf.allocations.get(&pid).cloned().unwrap_or(0.0);
                let total: f64 = rf.allocations.values().sum();
                let fair = total / rf.allocations.len().max(1) as f64;
                self.alerts.push_back(StarvationAlert {
                    pid,
                    resource: rf.resource,
                    duration_ns: 0,
                    share_received: if fair > 0.0 { alloc / fair } else { 0.0 },
                    timestamp_ns: 0,
                });
            }
        }
    }

    /// Remove process
    #[inline]
    pub fn remove_process(&mut self, pid: u64) {
        for rf in self.resources.values_mut() {
            rf.remove_process(pid);
        }
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.tracked_resources = self.resources.len();
        if !self.resources.is_empty() {
            self.stats.overall_jains = self.resources.values().map(|r| r.jains_index).sum::<f64>()
                / self.resources.len() as f64;
        }
        self.stats.starvation_alerts = self.alerts.len();
        self.stats.unfair_resources = self
            .resources
            .values()
            .filter(|r| matches!(r.verdict, FairnessVerdict::Unfair | FairnessVerdict::Severe))
            .count();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &CoopFairnessStats {
        &self.stats
    }
}
