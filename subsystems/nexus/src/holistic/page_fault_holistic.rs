// SPDX-License-Identifier: MIT
//! # Holistic Page Fault Analysis
//!
//! System-wide page fault pattern optimization:
//! - Global fault rate monitoring and anomaly detection
//! - Cross-process prefetch coordination
//! - System-wide working set estimation
//! - Page fault cost attribution per subsystem
//! - Demand paging vs prefetch ratio optimization

extern crate alloc;
use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultAnomaly { None, SpikeMajor, SpikeMinor, Sustained, ThrashPattern }

#[derive(Debug, Clone)]
pub struct SystemFaultSnapshot {
    pub timestamp: u64,
    pub total_major: u64,
    pub total_minor: u64,
    pub major_rate_per_sec: f64,
    pub minor_rate_per_sec: f64,
    pub working_set_pages: u64,
    pub resident_pages: u64,
}

impl SystemFaultSnapshot {
    #[inline]
    pub fn major_ratio(&self) -> f64 {
        let total = self.total_major + self.total_minor;
        if total == 0 { return 0.0; }
        self.total_major as f64 / total as f64
    }
    #[inline(always)]
    pub fn working_set_ratio(&self) -> f64 {
        if self.resident_pages == 0 { return 0.0; }
        self.working_set_pages as f64 / self.resident_pages as f64
    }
}

#[derive(Debug, Clone)]
pub struct SubsystemFaultCost {
    pub subsystem_id: u64,
    pub fault_count: u64,
    pub total_cost_ns: u64,
    pub avg_cost_ns: u64,
    pub percentage_of_total: f64,
}

#[derive(Debug, Clone)]
pub struct PrefetchEfficiency {
    pub total_prefetched: u64,
    pub prefetch_hits: u64,
    pub prefetch_wasted: u64,
    pub bandwidth_used_bps: u64,
}

impl PrefetchEfficiency {
    #[inline(always)]
    pub fn hit_rate(&self) -> f64 {
        if self.total_prefetched == 0 { return 0.0; }
        self.prefetch_hits as f64 / self.total_prefetched as f64
    }
    #[inline(always)]
    pub fn waste_ratio(&self) -> f64 {
        if self.total_prefetched == 0 { return 0.0; }
        self.prefetch_wasted as f64 / self.total_prefetched as f64
    }
}

#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct FaultHolisticStats {
    pub total_faults: u64,
    pub total_major_faults: u64,
    pub anomalies_detected: u64,
    pub prefetch_ratio: f64,
    pub avg_fault_cost_ns: u64,
    pub system_working_set_mb: u64,
}

pub struct PageFaultHolisticManager {
    /// Fault rate history: rolling window
    fault_history: VecDeque<SystemFaultSnapshot>,
    /// Subsystem cost attribution
    subsystem_costs: BTreeMap<u64, SubsystemFaultCost>,
    /// Prefetch tracking
    prefetch: PrefetchEfficiency,
    /// Per-process major fault counts for anomaly detection
    process_majors: LinearMap<u64, 64>,
    history_capacity: usize,
    stats: FaultHolisticStats,
}

impl PageFaultHolisticManager {
    pub fn new(history_capacity: usize) -> Self {
        Self {
            fault_history: Vec::with_capacity(history_capacity),
            subsystem_costs: BTreeMap::new(),
            prefetch: PrefetchEfficiency {
                total_prefetched: 0, prefetch_hits: 0,
                prefetch_wasted: 0, bandwidth_used_bps: 0,
            },
            process_majors: LinearMap::new(),
            history_capacity,
            stats: FaultHolisticStats::default(),
        }
    }

    /// Record a system-wide fault snapshot
    #[inline]
    pub fn record_snapshot(&mut self, snapshot: SystemFaultSnapshot) {
        self.stats.total_faults += snapshot.total_major + snapshot.total_minor;
        self.stats.total_major_faults += snapshot.total_major;
        self.stats.system_working_set_mb = snapshot.working_set_pages * 4 / 1024;

        if self.fault_history.len() >= self.history_capacity {
            self.fault_history.pop_front();
        }
        self.fault_history.push_back(snapshot);
    }

    /// Detect anomalies in fault patterns
    pub fn detect_anomaly(&self) -> FaultAnomaly {
        if self.fault_history.len() < 5 { return FaultAnomaly::None; }

        let recent = &self.fault_history[self.fault_history.len().saturating_sub(5)..];
        let avg_major: f64 = recent.iter().map(|s| s.major_rate_per_sec).sum::<f64>()
            / recent.len() as f64;

        // Check for spike: last sample >> average
        if let Some(last) = recent.last() {
            if last.major_rate_per_sec > avg_major * 3.0 {
                return FaultAnomaly::SpikeMajor;
            }
            if last.minor_rate_per_sec > avg_major * 10.0 {
                return FaultAnomaly::SpikeMinor;
            }
        }

        // Check for sustained high rate
        let all_high = recent.iter()
            .all(|s| s.major_rate_per_sec > 1000.0);
        if all_high { return FaultAnomaly::Sustained; }

        // Check for thrash pattern: alternating high/low
        let rates: Vec<f64> = recent.iter().map(|s| s.major_rate_per_sec).collect();
        let mut alternations = 0;
        for i in 1..rates.len() {
            if (rates[i] > avg_major) != (rates[i - 1] > avg_major) {
                alternations += 1;
            }
        }
        if alternations >= 3 { return FaultAnomaly::ThrashPattern; }

        FaultAnomaly::None
    }

    /// Record fault cost attribution to a subsystem
    pub fn attribute_cost(&mut self, subsystem_id: u64, cost_ns: u64) {
        let entry = self.subsystem_costs.entry(subsystem_id).or_insert(SubsystemFaultCost {
            subsystem_id, fault_count: 0, total_cost_ns: 0,
            avg_cost_ns: 0, percentage_of_total: 0.0,
        });
        entry.fault_count += 1;
        entry.total_cost_ns += cost_ns;
        entry.avg_cost_ns = entry.total_cost_ns / entry.fault_count;

        // Recompute percentages
        let total: u64 = self.subsystem_costs.values().map(|s| s.total_cost_ns).sum();
        for cost in self.subsystem_costs.values_mut() {
            cost.percentage_of_total = if total > 0 {
                cost.total_cost_ns as f64 / total as f64
            } else { 0.0 };
        }
    }

    /// Record prefetch result
    #[inline]
    pub fn record_prefetch(&mut self, pages: u64, hit: bool) {
        self.prefetch.total_prefetched += pages;
        if hit { self.prefetch.prefetch_hits += pages; }
        else { self.prefetch.prefetch_wasted += pages; }
        self.stats.prefetch_ratio = self.prefetch.hit_rate();
    }

    /// Optimal demand-paging vs prefetch ratio
    #[inline]
    pub fn optimal_prefetch_depth(&self) -> u64 {
        let hit_rate = self.prefetch.hit_rate();
        if hit_rate > 0.8 { 16 }      // good prediction: prefetch more
        else if hit_rate > 0.5 { 8 }   // moderate: standard depth
        else if hit_rate > 0.2 { 4 }   // poor: reduce
        else { 1 }                      // very poor: demand-page only
    }

    /// Top fault-costly subsystems
    #[inline]
    pub fn top_costly_subsystems(&self, n: usize) -> Vec<&SubsystemFaultCost> {
        let mut sorted: Vec<_> = self.subsystem_costs.values().collect();
        sorted.sort_by(|a, b| b.total_cost_ns.cmp(&a.total_cost_ns));
        sorted.into_iter().take(n).collect()
    }

    #[inline(always)]
    pub fn stats(&self) -> &FaultHolisticStats { &self.stats }
}
