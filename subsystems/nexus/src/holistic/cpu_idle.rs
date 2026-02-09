//! # Holistic CPU Idle Manager
//!
//! CPU idle state (C-state) management with holistic awareness:
//! - C-state residency tracking per CPU
//! - Latency/power trade-off analysis
//! - Governor selection (menu, ladder, teo)
//! - Idle prediction with history
//! - Wake-up source accounting
//! - Cross-CPU coordination for package C-states

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// C-state level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CStateLevel {
    C0,
    C1,
    C1e,
    C2,
    C3,
    C6,
    C7,
    C8,
    C10,
}

impl CStateLevel {
    pub fn exit_latency_us(&self) -> u64 {
        match self {
            CStateLevel::C0 => 0,
            CStateLevel::C1 => 1,
            CStateLevel::C1e => 10,
            CStateLevel::C2 => 100,
            CStateLevel::C3 => 500,
            CStateLevel::C6 => 1500,
            CStateLevel::C7 => 5000,
            CStateLevel::C8 => 10000,
            CStateLevel::C10 => 50000,
        }
    }

    pub fn power_mw(&self) -> u32 {
        match self {
            CStateLevel::C0 => 1000,
            CStateLevel::C1 => 500,
            CStateLevel::C1e => 200,
            CStateLevel::C2 => 100,
            CStateLevel::C3 => 50,
            CStateLevel::C6 => 10,
            CStateLevel::C7 => 5,
            CStateLevel::C8 => 2,
            CStateLevel::C10 => 1,
        }
    }

    #[inline]
    pub fn depth(&self) -> u8 {
        match self {
            CStateLevel::C0 => 0, CStateLevel::C1 => 1, CStateLevel::C1e => 2,
            CStateLevel::C2 => 3, CStateLevel::C3 => 4, CStateLevel::C6 => 5,
            CStateLevel::C7 => 6, CStateLevel::C8 => 7, CStateLevel::C10 => 8,
        }
    }
}

/// Idle governor type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleGovernor {
    Menu,
    Ladder,
    Teo,
    Haltpoll,
}

/// Wake-up source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeSource {
    Timer,
    Interrupt,
    Ipi,
    IoCompletion,
    NetworkRx,
    UserInput,
    Unknown,
}

/// Per-CPU idle state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CpuIdleState {
    pub cpu_id: u32,
    pub current_state: CStateLevel,
    pub governor: IdleGovernor,
    pub residency_us: BTreeMap<u8, u64>,
    pub entry_count: BTreeMap<u8, u64>,
    pub predicted_us: u64,
    pub actual_us: u64,
    pub last_wake: WakeSource,
    pub total_idle_us: u64,
    pub total_active_us: u64,
    pub deepest_allowed: CStateLevel,
    pub disable_mask: u16,
    pub prediction_history: Vec<u64>,
    history_idx: usize,
}

impl CpuIdleState {
    pub fn new(cpu_id: u32) -> Self {
        Self {
            cpu_id, current_state: CStateLevel::C0, governor: IdleGovernor::Menu,
            residency_us: BTreeMap::new(), entry_count: BTreeMap::new(),
            predicted_us: 0, actual_us: 0, last_wake: WakeSource::Unknown,
            total_idle_us: 0, total_active_us: 0,
            deepest_allowed: CStateLevel::C10, disable_mask: 0,
            prediction_history: alloc::vec![0u64; 8], history_idx: 0,
        }
    }

    #[inline]
    pub fn enter_idle(&mut self, state: CStateLevel, predicted: u64) {
        self.current_state = state;
        self.predicted_us = predicted;
        *self.entry_count.entry(state.depth()).or_insert(0) += 1;
    }

    pub fn exit_idle(&mut self, actual: u64, wake: WakeSource) {
        let depth = self.current_state.depth();
        *self.residency_us.entry(depth).or_insert(0) += actual;
        self.actual_us = actual;
        self.last_wake = wake;
        self.total_idle_us += actual;
        if self.history_idx < self.prediction_history.len() {
            self.prediction_history[self.history_idx] = actual;
        }
        self.history_idx = (self.history_idx + 1) % self.prediction_history.len();
        self.current_state = CStateLevel::C0;
    }

    #[inline(always)]
    pub fn idle_pct(&self) -> f64 {
        let total = self.total_idle_us + self.total_active_us;
        if total == 0 { 0.0 } else { self.total_idle_us as f64 / total as f64 * 100.0 }
    }

    #[inline]
    pub fn prediction_accuracy(&self) -> f64 {
        if self.predicted_us == 0 || self.actual_us == 0 { return 0.0; }
        let diff = if self.predicted_us > self.actual_us { self.predicted_us - self.actual_us } else { self.actual_us - self.predicted_us };
        let max = if self.predicted_us > self.actual_us { self.predicted_us } else { self.actual_us };
        if max == 0 { 0.0 } else { (1.0 - diff as f64 / max as f64) * 100.0 }
    }

    #[inline(always)]
    pub fn avg_idle_us(&self) -> u64 {
        let non_zero: Vec<u64> = self.prediction_history.iter().copied().filter(|&v| v > 0).collect();
        if non_zero.is_empty() { 0 } else { non_zero.iter().sum::<u64>() / non_zero.len() as u64 }
    }

    #[inline(always)]
    pub fn is_disabled(&self, state: CStateLevel) -> bool { (self.disable_mask >> state.depth()) & 1 != 0 }
    #[inline(always)]
    pub fn disable_state(&mut self, state: CStateLevel) { self.disable_mask |= 1 << state.depth(); }
    #[inline(always)]
    pub fn enable_state(&mut self, state: CStateLevel) { self.disable_mask &= !(1 << state.depth()); }

    #[inline]
    pub fn select_state(&self) -> CStateLevel {
        let avg = self.avg_idle_us();
        let states = [CStateLevel::C10, CStateLevel::C8, CStateLevel::C7, CStateLevel::C6, CStateLevel::C3, CStateLevel::C2, CStateLevel::C1e, CStateLevel::C1];
        for &s in &states {
            if s > self.deepest_allowed { continue; }
            if self.is_disabled(s) { continue; }
            if avg >= s.exit_latency_us() * 3 { return s; }
        }
        CStateLevel::C0
    }
}

/// Package C-state info
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PackageCState {
    pub package_id: u32,
    pub cpu_ids: Vec<u32>,
    pub deepest_pkg: CStateLevel,
    pub residency_us: BTreeMap<u8, u64>,
    pub all_idle: bool,
}

impl PackageCState {
    pub fn new(pkg: u32) -> Self {
        Self { package_id: pkg, cpu_ids: Vec::new(), deepest_pkg: CStateLevel::C0, residency_us: BTreeMap::new(), all_idle: false }
    }
}

/// CPU idle stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CpuIdleStats {
    pub total_cpus: usize,
    pub avg_idle_pct: f64,
    pub avg_prediction_accuracy: f64,
    pub deepest_used: u8,
    pub total_idle_entries: u64,
    pub total_wakes_by_timer: u64,
    pub total_wakes_by_irq: u64,
}

/// Holistic CPU idle manager
pub struct HolisticCpuIdle {
    cpus: BTreeMap<u32, CpuIdleState>,
    packages: BTreeMap<u32, PackageCState>,
    stats: CpuIdleStats,
    global_governor: IdleGovernor,
}

impl HolisticCpuIdle {
    pub fn new() -> Self {
        Self { cpus: BTreeMap::new(), packages: BTreeMap::new(), stats: CpuIdleStats::default(), global_governor: IdleGovernor::Menu }
    }

    #[inline(always)]
    pub fn add_cpu(&mut self, id: u32, pkg: u32) {
        self.cpus.insert(id, CpuIdleState::new(id));
        self.packages.entry(pkg).or_insert_with(|| PackageCState::new(pkg)).cpu_ids.push(id);
    }

    #[inline(always)]
    pub fn set_governor(&mut self, gov: IdleGovernor) {
        self.global_governor = gov;
        for c in self.cpus.values_mut() { c.governor = gov; }
    }

    #[inline]
    pub fn enter_idle(&mut self, cpu: u32, predicted: u64) {
        if let Some(c) = self.cpus.get_mut(&cpu) {
            let state = c.select_state();
            c.enter_idle(state, predicted);
        }
    }

    #[inline(always)]
    pub fn exit_idle(&mut self, cpu: u32, actual: u64, wake: WakeSource) {
        if let Some(c) = self.cpus.get_mut(&cpu) { c.exit_idle(actual, wake); }
    }

    #[inline(always)]
    pub fn disable_state(&mut self, cpu: u32, state: CStateLevel) { if let Some(c) = self.cpus.get_mut(&cpu) { c.disable_state(state); } }
    #[inline(always)]
    pub fn enable_state(&mut self, cpu: u32, state: CStateLevel) { if let Some(c) = self.cpus.get_mut(&cpu) { c.enable_state(state); } }

    #[inline(always)]
    pub fn set_max_depth(&mut self, cpu: u32, max: CStateLevel) { if let Some(c) = self.cpus.get_mut(&cpu) { c.deepest_allowed = max; } }

    pub fn update_packages(&mut self) {
        let cpu_snap: BTreeMap<u32, (CStateLevel, u64)> = self.cpus.iter().map(|(&id, c)| (id, (c.current_state, c.total_idle_us))).collect();
        for pkg in self.packages.values_mut() {
            pkg.all_idle = pkg.cpu_ids.iter().all(|id| cpu_snap.get(id).map(|(s, _)| *s != CStateLevel::C0).unwrap_or(false));
            if pkg.all_idle {
                let min = pkg.cpu_ids.iter().filter_map(|id| cpu_snap.get(id).map(|(s, _)| *s)).min().unwrap_or(CStateLevel::C0);
                pkg.deepest_pkg = min;
            } else {
                pkg.deepest_pkg = CStateLevel::C0;
            }
        }
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_cpus = self.cpus.len();
        if !self.cpus.is_empty() {
            self.stats.avg_idle_pct = self.cpus.values().map(|c| c.idle_pct()).sum::<f64>() / self.cpus.len() as f64;
            self.stats.avg_prediction_accuracy = self.cpus.values().map(|c| c.prediction_accuracy()).sum::<f64>() / self.cpus.len() as f64;
        }
        self.stats.total_idle_entries = self.cpus.values().flat_map(|c| c.entry_count.values()).sum();
        self.stats.deepest_used = self.cpus.values().flat_map(|c| c.residency_us.keys()).copied().max().unwrap_or(0);
    }

    #[inline(always)]
    pub fn cpu(&self, id: u32) -> Option<&CpuIdleState> { self.cpus.get(&id) }
    #[inline(always)]
    pub fn package(&self, id: u32) -> Option<&PackageCState> { self.packages.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &CpuIdleStats { &self.stats }
}

// ============================================================================
// Merged from cpu_idle_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CState {
    C0,  // Active
    C1,  // Halt
    C1E, // Enhanced halt
    C2,  // Stop clock
    C3,  // Sleep
    C6,  // Deep power down
    C7,  // Package C7
    C8,  // Package C8
    C10, // Package C10
}

/// Idle governor type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleGovernor {
    Menu,
    Ladder,
    Teo,
    Haltpoll,
}

/// Per-CPU idle state
#[derive(Debug)]
#[repr(align(64))]
pub struct CpuIdleState {
    pub cpu_id: u32,
    pub current_cstate: CState,
    pub governor: IdleGovernor,
    pub residency_ns: BTreeMap<u8, u64>,
    pub entry_count: BTreeMap<u8, u64>,
    pub total_idle_ns: u64,
    pub total_active_ns: u64,
    pub last_idle_enter: u64,
    pub predicted_idle_ns: u64,
    pub actual_idle_ns: u64,
    pub mispredictions: u64,
}

impl CpuIdleState {
    pub fn new(cpu_id: u32, governor: IdleGovernor) -> Self {
        Self {
            cpu_id, current_cstate: CState::C0, governor,
            residency_ns: BTreeMap::new(), entry_count: BTreeMap::new(),
            total_idle_ns: 0, total_active_ns: 0, last_idle_enter: 0,
            predicted_idle_ns: 0, actual_idle_ns: 0, mispredictions: 0,
        }
    }

    #[inline]
    pub fn enter_idle(&mut self, target: CState, predicted_ns: u64, now: u64) {
        self.current_cstate = target;
        self.last_idle_enter = now;
        self.predicted_idle_ns = predicted_ns;
        let key = target as u8;
        *self.entry_count.entry(key).or_insert(0) += 1;
    }

    #[inline]
    pub fn exit_idle(&mut self, now: u64) {
        let duration = now.saturating_sub(self.last_idle_enter);
        let key = self.current_cstate as u8;
        *self.residency_ns.entry(key).or_insert(0) += duration;
        self.total_idle_ns += duration;
        self.actual_idle_ns = duration;
        if duration < self.predicted_idle_ns / 2 || duration > self.predicted_idle_ns * 2 {
            self.mispredictions += 1;
        }
        self.current_cstate = CState::C0;
    }

    #[inline]
    pub fn idle_ratio(&self) -> f64 {
        let total = self.total_idle_ns + self.total_active_ns;
        if total == 0 { return 0.0; }
        self.total_idle_ns as f64 / total as f64
    }

    #[inline]
    pub fn prediction_accuracy(&self) -> f64 {
        let total_entries: u64 = self.entry_count.values().sum();
        if total_entries == 0 { return 1.0; }
        1.0 - (self.mispredictions as f64 / total_entries as f64)
    }
}

/// C-state latency table
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CStateLatency {
    pub cstate: CState,
    pub entry_latency_ns: u64,
    pub exit_latency_ns: u64,
    pub target_residency_ns: u64,
    pub power_mw: u32,
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CpuIdleV2Stats {
    pub total_cpus: u32,
    pub idle_cpus: u32,
    pub avg_idle_ratio: f64,
    pub avg_prediction_accuracy: f64,
    pub deepest_cstate_used: CState,
    pub total_idle_entries: u64,
}

/// Main CPU idle v2 manager
pub struct HolisticCpuIdleV2 {
    cpus: BTreeMap<u32, CpuIdleState>,
    latency_table: Vec<CStateLatency>,
}

impl HolisticCpuIdleV2 {
    pub fn new() -> Self {
        let table = alloc::vec![
            CStateLatency { cstate: CState::C1, entry_latency_ns: 1_000, exit_latency_ns: 1_000, target_residency_ns: 10_000, power_mw: 800 },
            CStateLatency { cstate: CState::C1E, entry_latency_ns: 10_000, exit_latency_ns: 20_000, target_residency_ns: 100_000, power_mw: 500 },
            CStateLatency { cstate: CState::C3, entry_latency_ns: 100_000, exit_latency_ns: 200_000, target_residency_ns: 1_000_000, power_mw: 200 },
            CStateLatency { cstate: CState::C6, entry_latency_ns: 500_000, exit_latency_ns: 1_000_000, target_residency_ns: 5_000_000, power_mw: 50 },
            CStateLatency { cstate: CState::C7, entry_latency_ns: 1_000_000, exit_latency_ns: 2_000_000, target_residency_ns: 10_000_000, power_mw: 20 },
        ];
        Self { cpus: BTreeMap::new(), latency_table: table }
    }

    #[inline(always)]
    pub fn register_cpu(&mut self, cpu_id: u32, governor: IdleGovernor) {
        self.cpus.insert(cpu_id, CpuIdleState::new(cpu_id, governor));
    }

    #[inline(always)]
    pub fn enter_idle(&mut self, cpu_id: u32, target: CState, predicted: u64, now: u64) {
        if let Some(cpu) = self.cpus.get_mut(&cpu_id) { cpu.enter_idle(target, predicted, now); }
    }

    #[inline(always)]
    pub fn exit_idle(&mut self, cpu_id: u32, now: u64) {
        if let Some(cpu) = self.cpus.get_mut(&cpu_id) { cpu.exit_idle(now); }
    }

    pub fn stats(&self) -> CpuIdleV2Stats {
        let idle = self.cpus.values().filter(|c| c.current_cstate != CState::C0).count() as u32;
        let ratios: Vec<f64> = self.cpus.values().map(|c| c.idle_ratio()).collect();
        let avg_idle = if ratios.is_empty() { 0.0 } else { ratios.iter().sum::<f64>() / ratios.len() as f64 };
        let accs: Vec<f64> = self.cpus.values().map(|c| c.prediction_accuracy()).collect();
        let avg_acc = if accs.is_empty() { 1.0 } else { accs.iter().sum::<f64>() / accs.len() as f64 };
        let entries: u64 = self.cpus.values().flat_map(|c| c.entry_count.values()).sum();
        CpuIdleV2Stats {
            total_cpus: self.cpus.len() as u32, idle_cpus: idle,
            avg_idle_ratio: avg_idle, avg_prediction_accuracy: avg_acc,
            deepest_cstate_used: CState::C7, total_idle_entries: entries,
        }
    }
}
