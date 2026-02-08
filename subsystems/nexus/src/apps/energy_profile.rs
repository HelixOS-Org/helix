//! # Application Energy Profiler
//!
//! Per-process energy consumption tracking:
//! - CPU energy (RAPL domains) attribution
//! - Package/core/DRAM energy breakdown
//! - Energy-per-instruction estimation
//! - C-state residency correlation
//! - Power budget enforcement
//! - Energy-efficiency scoring

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// RAPL domain type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RaplDomain {
    Package,
    Core,
    Uncore,
    Dram,
    Psys,
}

/// C-state type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CState {
    C0Active,
    C1Halt,
    C1eAutoHalt,
    C3Sleep,
    C6DeepSleep,
    C7Offline,
    C10Package,
}

/// Power phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerPhase {
    Idle,
    Light,
    Medium,
    Heavy,
    Burst,
}

/// Energy sample for a domain
#[derive(Debug, Clone)]
pub struct EnergySample {
    pub domain: RaplDomain,
    pub energy_uj: u64,
    pub timestamp: u64,
    pub duration_ns: u64,
}

impl EnergySample {
    pub fn power_watts(&self) -> f64 {
        if self.duration_ns == 0 { return 0.0; }
        (self.energy_uj as f64 / 1_000_000.0) / (self.duration_ns as f64 / 1_000_000_000.0)
    }
}

/// C-state residency record
#[derive(Debug, Clone)]
pub struct CStateResidency {
    pub state: CState,
    pub total_ns: u64,
    pub entry_count: u64,
    pub avg_duration_ns: u64,
}

impl CStateResidency {
    pub fn new(state: CState) -> Self {
        Self { state, total_ns: 0, entry_count: 0, avg_duration_ns: 0 }
    }

    pub fn record(&mut self, duration_ns: u64) {
        self.total_ns += duration_ns;
        self.entry_count += 1;
        self.avg_duration_ns = if self.entry_count > 0 { self.total_ns / self.entry_count } else { 0 };
    }

    pub fn ratio(&self, total_ns: u64) -> f64 {
        if total_ns == 0 { return 0.0; }
        self.total_ns as f64 / total_ns as f64
    }
}

/// Per-process energy profile
#[derive(Debug, Clone)]
pub struct ProcessEnergyProfile {
    pub pid: u64,
    pub domain_energy: BTreeMap<u8, u64>, // domain_id -> energy_uj
    pub instructions_retired: u64,
    pub cycles: u64,
    pub active_time_ns: u64,
    pub idle_time_ns: u64,
    pub cstate_residency: Vec<CStateResidency>,
    pub power_phase: PowerPhase,
    pub energy_budget_uj: u64,
    pub budget_consumed_uj: u64,
    pub sample_count: u64,
}

impl ProcessEnergyProfile {
    pub fn new(pid: u64) -> Self {
        let cstates = alloc::vec![
            CStateResidency::new(CState::C0Active),
            CStateResidency::new(CState::C1Halt),
            CStateResidency::new(CState::C3Sleep),
            CStateResidency::new(CState::C6DeepSleep),
        ];
        Self {
            pid,
            domain_energy: BTreeMap::new(),
            instructions_retired: 0,
            cycles: 0,
            active_time_ns: 0,
            idle_time_ns: 0,
            cstate_residency: cstates,
            power_phase: PowerPhase::Idle,
            energy_budget_uj: 0,
            budget_consumed_uj: 0,
            sample_count: 0,
        }
    }

    pub fn total_energy_uj(&self) -> u64 {
        self.domain_energy.values().sum()
    }

    pub fn energy_per_instruction(&self) -> f64 {
        if self.instructions_retired == 0 { return 0.0; }
        self.total_energy_uj() as f64 / self.instructions_retired as f64
    }

    pub fn ipc(&self) -> f64 {
        if self.cycles == 0 { return 0.0; }
        self.instructions_retired as f64 / self.cycles as f64
    }

    /// Energy efficiency score: higher = more efficient
    pub fn efficiency_score(&self) -> f64 {
        let epi = self.energy_per_instruction();
        if epi <= 0.0 { return 0.0; }
        // Normalized inverse — lower EPI = higher score
        1.0 / (1.0 + epi * 100.0)
    }

    pub fn avg_power_watts(&self) -> f64 {
        let total_time = self.active_time_ns + self.idle_time_ns;
        if total_time == 0 { return 0.0; }
        (self.total_energy_uj() as f64 / 1_000_000.0) / (total_time as f64 / 1_000_000_000.0)
    }

    pub fn active_ratio(&self) -> f64 {
        let total = self.active_time_ns + self.idle_time_ns;
        if total == 0 { return 0.0; }
        self.active_time_ns as f64 / total as f64
    }

    pub fn budget_remaining_uj(&self) -> u64 {
        if self.energy_budget_uj == 0 { return u64::MAX; }
        self.energy_budget_uj.saturating_sub(self.budget_consumed_uj)
    }

    pub fn is_over_budget(&self) -> bool {
        self.energy_budget_uj > 0 && self.budget_consumed_uj > self.energy_budget_uj
    }

    pub fn record_energy(&mut self, domain_id: u8, energy_uj: u64) {
        *self.domain_energy.entry(domain_id).or_insert(0) += energy_uj;
        self.budget_consumed_uj += energy_uj;
        self.sample_count += 1;
    }

    pub fn update_phase(&mut self) {
        let ratio = self.active_ratio();
        self.power_phase = if ratio < 0.05 { PowerPhase::Idle }
        else if ratio < 0.25 { PowerPhase::Light }
        else if ratio < 0.6 { PowerPhase::Medium }
        else if ratio < 0.9 { PowerPhase::Heavy }
        else { PowerPhase::Burst };
    }
}

/// App energy profiler stats
#[derive(Debug, Clone, Default)]
pub struct AppEnergyProfilerStats {
    pub total_processes: usize,
    pub total_energy_uj: u64,
    pub total_samples: u64,
    pub over_budget_count: usize,
    pub avg_efficiency: f64,
    pub total_power_watts: f64,
}

/// Application Energy Profiler
pub struct AppEnergyProfiler {
    profiles: BTreeMap<u64, ProcessEnergyProfile>,
    stats: AppEnergyProfilerStats,
}

impl AppEnergyProfiler {
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            stats: AppEnergyProfilerStats::default(),
        }
    }

    pub fn register_process(&mut self, pid: u64) {
        self.profiles.entry(pid).or_insert_with(|| ProcessEnergyProfile::new(pid));
    }

    pub fn set_budget(&mut self, pid: u64, budget_uj: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.energy_budget_uj = budget_uj;
        }
    }

    pub fn record_energy(&mut self, pid: u64, domain_id: u8, energy_uj: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.record_energy(domain_id, energy_uj);
        }
        self.recompute();
    }

    pub fn record_activity(&mut self, pid: u64, active_ns: u64, idle_ns: u64,
                           instructions: u64, cycles: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.active_time_ns += active_ns;
            profile.idle_time_ns += idle_ns;
            profile.instructions_retired += instructions;
            profile.cycles += cycles;
            profile.update_phase();
        }
    }

    pub fn record_cstate(&mut self, pid: u64, state_idx: usize, duration_ns: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            if state_idx < profile.cstate_residency.len() {
                profile.cstate_residency[state_idx].record(duration_ns);
            }
        }
    }

    pub fn over_budget_processes(&self) -> Vec<u64> {
        self.profiles.values()
            .filter(|p| p.is_over_budget())
            .map(|p| p.pid)
            .collect()
    }

    pub fn top_consumers(&self, n: usize) -> Vec<(u64, u64)> {
        let mut sorted: Vec<_> = self.profiles.values()
            .map(|p| (p.pid, p.total_energy_uj()))
            .collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(n);
        sorted
    }

    fn recompute(&mut self) {
        self.stats.total_processes = self.profiles.len();
        self.stats.total_energy_uj = self.profiles.values().map(|p| p.total_energy_uj()).sum();
        self.stats.total_samples = self.profiles.values().map(|p| p.sample_count).sum();
        self.stats.over_budget_count = self.profiles.values().filter(|p| p.is_over_budget()).count();
        self.stats.total_power_watts = self.profiles.values().map(|p| p.avg_power_watts()).sum();

        let count = self.profiles.len();
        self.stats.avg_efficiency = if count > 0 {
            self.profiles.values().map(|p| p.efficiency_score()).sum::<f64>() / count as f64
        } else { 0.0 };
    }

    pub fn profile(&self, pid: u64) -> Option<&ProcessEnergyProfile> {
        self.profiles.get(&pid)
    }

    pub fn stats(&self) -> &AppEnergyProfilerStats {
        &self.stats
    }

    pub fn remove_process(&mut self, pid: u64) {
        self.profiles.remove(&pid);
        self.recompute();
    }
}
