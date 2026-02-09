//! # Holistic CPU Frequency Governor
//!
//! System-wide CPU frequency scaling with holistic awareness:
//! - Multi-domain frequency coordination
//! - Workload-adaptive frequency transitions
//! - Energy-performance preference (EPP) tuning
//! - C-state residency optimization
//! - Turbo boost budget management
//! - Cross-core frequency impact analysis

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// CPU frequency domain governor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuFreqGovernor {
    Performance,
    Powersave,
    Ondemand,
    Conservative,
    Schedutil,
    Holistic,
}

/// Frequency transition reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreqTransitionReason {
    LoadIncrease,
    LoadDecrease,
    ThermalThrottle,
    PowerBudget,
    LatencyTarget,
    UserRequest,
    TurboBoost,
    IdleEntry,
}

/// Energy-Performance Preference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EppHint {
    Default,
    Performance,
    BalancePerformance,
    BalancePower,
    Power,
}

/// Per-CPU frequency state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CpuFreqState {
    pub cpu_id: u32,
    pub current_freq_khz: u32,
    pub min_freq_khz: u32,
    pub max_freq_khz: u32,
    pub base_freq_khz: u32,
    pub turbo_freq_khz: u32,
    pub governor: CpuFreqGovernor,
    pub epp: EppHint,
    pub utilization: f64,
    pub ipc_estimate: f64,
    pub transitions: u64,
    pub time_in_turbo_ns: u64,
    pub time_in_min_ns: u64,
    pub last_transition_ns: u64,
}

impl CpuFreqState {
    pub fn new(cpu_id: u32, min_khz: u32, max_khz: u32, base_khz: u32) -> Self {
        Self {
            cpu_id,
            current_freq_khz: base_khz,
            min_freq_khz: min_khz,
            max_freq_khz: max_khz,
            base_freq_khz: base_khz,
            turbo_freq_khz: max_khz,
            governor: CpuFreqGovernor::Holistic,
            epp: EppHint::Default,
            utilization: 0.0,
            ipc_estimate: 1.0,
            transitions: 0,
            time_in_turbo_ns: 0,
            time_in_min_ns: 0,
            last_transition_ns: 0,
        }
    }

    #[inline(always)]
    pub fn freq_ratio(&self) -> f64 {
        if self.max_freq_khz == 0 { return 0.0; }
        self.current_freq_khz as f64 / self.max_freq_khz as f64
    }

    #[inline(always)]
    pub fn is_turbo(&self) -> bool {
        self.current_freq_khz > self.base_freq_khz
    }

    #[inline(always)]
    pub fn is_minimum(&self) -> bool {
        self.current_freq_khz <= self.min_freq_khz
    }
}

/// Frequency domain (group of CPUs sharing frequency)
#[derive(Debug, Clone)]
pub struct FreqDomain {
    pub domain_id: u32,
    pub cpus: Vec<u32>,
    pub shared_freq_khz: u32,
    pub avg_utilization: f64,
    pub max_utilization: f64,
    pub energy_budget_uw: u64,
    pub current_power_uw: u64,
}

impl FreqDomain {
    pub fn new(domain_id: u32) -> Self {
        Self {
            domain_id,
            cpus: Vec::new(),
            shared_freq_khz: 0,
            avg_utilization: 0.0,
            max_utilization: 0.0,
            energy_budget_uw: u64::MAX,
            current_power_uw: 0,
        }
    }

    #[inline]
    pub fn power_headroom(&self) -> f64 {
        if self.energy_budget_uw == u64::MAX || self.energy_budget_uw == 0 { return 1.0; }
        let remaining = self.energy_budget_uw.saturating_sub(self.current_power_uw);
        remaining as f64 / self.energy_budget_uw as f64
    }
}

/// Turbo budget tracker
#[derive(Debug, Clone)]
pub struct TurboBudget {
    pub total_budget_ns: u64,
    pub consumed_ns: u64,
    pub window_ns: u64,
    pub active_turbo_cpus: u32,
    pub max_turbo_cpus: u32,
}

impl TurboBudget {
    pub fn new(total_ns: u64, window_ns: u64, max_cpus: u32) -> Self {
        Self {
            total_budget_ns: total_ns,
            consumed_ns: 0,
            window_ns,
            active_turbo_cpus: 0,
            max_turbo_cpus: max_cpus,
        }
    }

    #[inline]
    pub fn remaining_ratio(&self) -> f64 {
        if self.total_budget_ns == 0 { return 0.0; }
        let rem = self.total_budget_ns.saturating_sub(self.consumed_ns);
        rem as f64 / self.total_budget_ns as f64
    }

    #[inline(always)]
    pub fn can_turbo(&self) -> bool {
        self.consumed_ns < self.total_budget_ns && self.active_turbo_cpus < self.max_turbo_cpus
    }

    #[inline(always)]
    pub fn consume(&mut self, ns: u64) {
        self.consumed_ns = self.consumed_ns.saturating_add(ns);
    }

    #[inline(always)]
    pub fn reset_window(&mut self) {
        self.consumed_ns = 0;
    }
}

/// Holistic CPU Frequency Governor stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticCpuFreqStats {
    pub total_cpus: usize,
    pub total_domains: usize,
    pub avg_freq_ratio: f64,
    pub turbo_utilization: f64,
    pub total_transitions: u64,
}

/// Holistic CPU Frequency Governor
pub struct HolisticCpuFreqGov {
    cpus: BTreeMap<u32, CpuFreqState>,
    domains: BTreeMap<u32, FreqDomain>,
    turbo_budget: TurboBudget,
    target_latency_ns: u64,
    stats: HolisticCpuFreqStats,
}

impl HolisticCpuFreqGov {
    pub fn new(turbo_budget_ns: u64, turbo_window_ns: u64, max_turbo_cpus: u32) -> Self {
        Self {
            cpus: BTreeMap::new(),
            domains: BTreeMap::new(),
            turbo_budget: TurboBudget::new(turbo_budget_ns, turbo_window_ns, max_turbo_cpus),
            target_latency_ns: 1_000_000,
            stats: HolisticCpuFreqStats::default(),
        }
    }

    #[inline]
    pub fn add_cpu(&mut self, state: CpuFreqState, domain_id: u32) {
        let cpu_id = state.cpu_id;
        self.cpus.insert(cpu_id, state);
        self.domains.entry(domain_id)
            .or_insert_with(|| FreqDomain::new(domain_id))
            .cpus.push(cpu_id);
    }

    #[inline]
    pub fn update_utilization(&mut self, cpu_id: u32, util: f64, ipc: f64) {
        if let Some(cpu) = self.cpus.get_mut(&cpu_id) {
            cpu.utilization = util;
            cpu.ipc_estimate = ipc;
        }
    }

    /// Compute optimal frequencies for all CPUs
    pub fn compute_frequencies(&mut self, now_ns: u64) {
        // Update domain aggregates
        for domain in self.domains.values_mut() {
            let mut sum_util = 0.0f64;
            let mut max_util = 0.0f64;
            let mut count = 0u32;
            for &cpu_id in &domain.cpus {
                if let Some(cpu) = self.cpus.get(&cpu_id) {
                    sum_util += cpu.utilization;
                    if cpu.utilization > max_util { max_util = cpu.utilization; }
                    count += 1;
                }
            }
            domain.avg_utilization = if count > 0 { sum_util / count as f64 } else { 0.0 };
            domain.max_utilization = max_util;
        }

        // Compute per-CPU target frequencies
        let cpu_ids: Vec<u32> = self.cpus.keys().copied().collect();
        for cpu_id in cpu_ids {
            let (target_khz, reason) = self.compute_target(cpu_id);
            if let Some(cpu) = self.cpus.get_mut(&cpu_id) {
                if cpu.current_freq_khz != target_khz {
                    let was_turbo = cpu.is_turbo();
                    cpu.current_freq_khz = target_khz;
                    cpu.transitions += 1;
                    cpu.last_transition_ns = now_ns;

                    // Track turbo
                    if target_khz > cpu.base_freq_khz && !was_turbo {
                        self.turbo_budget.active_turbo_cpus += 1;
                    } else if target_khz <= cpu.base_freq_khz && was_turbo {
                        self.turbo_budget.active_turbo_cpus =
                            self.turbo_budget.active_turbo_cpus.saturating_sub(1);
                    }
                    let _ = reason;
                }
            }
        }

        self.recompute_stats();
    }

    fn compute_target(&self, cpu_id: u32) -> (u32, FreqTransitionReason) {
        if let Some(cpu) = self.cpus.get(&cpu_id) {
            let range = cpu.max_freq_khz - cpu.min_freq_khz;

            // High utilization → boost frequency
            if cpu.utilization > 0.85 {
                if self.turbo_budget.can_turbo() && cpu.utilization > 0.95 {
                    return (cpu.turbo_freq_khz, FreqTransitionReason::TurboBoost);
                }
                return (cpu.max_freq_khz, FreqTransitionReason::LoadIncrease);
            }

            // Low utilization → save power
            if cpu.utilization < 0.1 {
                return (cpu.min_freq_khz, FreqTransitionReason::IdleEntry);
            }

            // Proportional scaling with IPC weighting
            let util_factor = cpu.utilization;
            let ipc_factor = if cpu.ipc_estimate > 1.5 { 1.1 } else { 1.0 };
            let target = cpu.min_freq_khz + (range as f64 * util_factor * ipc_factor) as u32;
            let clamped = target.min(cpu.max_freq_khz).max(cpu.min_freq_khz);
            (clamped, FreqTransitionReason::LoadIncrease)
        } else {
            (0, FreqTransitionReason::LoadDecrease)
        }
    }

    fn recompute_stats(&mut self) {
        self.stats.total_cpus = self.cpus.len();
        self.stats.total_domains = self.domains.len();
        let total_ratio: f64 = self.cpus.values().map(|c| c.freq_ratio()).sum();
        self.stats.avg_freq_ratio = if self.cpus.is_empty() { 0.0 }
        else { total_ratio / self.cpus.len() as f64 };
        let turbo_count = self.cpus.values().filter(|c| c.is_turbo()).count();
        self.stats.turbo_utilization = if self.cpus.is_empty() { 0.0 }
        else { turbo_count as f64 / self.cpus.len() as f64 };
        self.stats.total_transitions = self.cpus.values().map(|c| c.transitions).sum();
    }

    #[inline(always)]
    pub fn cpu(&self, id: u32) -> Option<&CpuFreqState> { self.cpus.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &HolisticCpuFreqStats { &self.stats }
    #[inline(always)]
    pub fn turbo_budget(&self) -> &TurboBudget { &self.turbo_budget }
}
