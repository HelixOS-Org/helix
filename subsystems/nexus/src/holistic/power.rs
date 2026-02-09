//! # Power Management
//!
//! System-wide power/energy management:
//! - Power state management (P-states, C-states)
//! - Energy-performance preference
//! - Power budget allocation
//! - Battery management
//! - Power profile selection
//! - Per-device power management
//! - Power consumption estimation

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;

// ============================================================================
// POWER STATES
// ============================================================================

/// CPU performance state (P-state)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PState {
    /// Maximum performance
    P0,
    /// High performance
    P1,
    /// Normal
    P2,
    /// Reduced
    P3,
    /// Low power
    P4,
    /// Minimum
    P5,
}

impl PState {
    /// Typical frequency ratio (percent)
    #[inline]
    pub fn freq_ratio(&self) -> u32 {
        match self {
            Self::P0 => 100,
            Self::P1 => 90,
            Self::P2 => 75,
            Self::P3 => 60,
            Self::P4 => 40,
            Self::P5 => 20,
        }
    }

    /// Typical power ratio (percent)
    #[inline]
    pub fn power_ratio(&self) -> u32 {
        match self {
            Self::P0 => 100,
            Self::P1 => 80,
            Self::P2 => 55,
            Self::P3 => 35,
            Self::P4 => 18,
            Self::P5 => 8,
        }
    }
}

/// CPU idle state (C-state)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CState {
    /// Active (running)
    C0,
    /// Halt
    C1,
    /// Stop clock
    C2,
    /// Sleep
    C3,
    /// Deep sleep
    C6,
    /// Package sleep
    C7,
    /// Deepest sleep
    C10,
}

impl CState {
    /// Wakeup latency (microseconds)
    #[inline]
    pub fn wakeup_latency_us(&self) -> u64 {
        match self {
            Self::C0 => 0,
            Self::C1 => 1,
            Self::C2 => 10,
            Self::C3 => 100,
            Self::C6 => 500,
            Self::C7 => 1000,
            Self::C10 => 5000,
        }
    }

    /// Power savings (relative to C0, percent)
    #[inline]
    pub fn power_savings(&self) -> u32 {
        match self {
            Self::C0 => 0,
            Self::C1 => 20,
            Self::C2 => 40,
            Self::C3 => 60,
            Self::C6 => 85,
            Self::C7 => 92,
            Self::C10 => 98,
        }
    }
}

// ============================================================================
// POWER PROFILES
// ============================================================================

/// System power profile
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerProfile {
    /// Maximum performance
    Performance,
    /// Balanced performance and efficiency
    Balanced,
    /// Power saving
    PowerSave,
    /// Battery saver (aggressive power saving)
    BatterySaver,
    /// Custom
    Custom,
}

impl PowerProfile {
    /// Energy-Performance Preference value (0 = max perf, 255 = max efficiency)
    #[inline]
    pub fn epp(&self) -> u8 {
        match self {
            Self::Performance => 0,
            Self::Balanced => 128,
            Self::PowerSave => 192,
            Self::BatterySaver => 255,
            Self::Custom => 128,
        }
    }

    /// Max P-state allowed
    #[inline]
    pub fn max_pstate(&self) -> PState {
        match self {
            Self::Performance => PState::P0,
            Self::Balanced => PState::P0,
            Self::PowerSave => PState::P2,
            Self::BatterySaver => PState::P3,
            Self::Custom => PState::P0,
        }
    }

    /// Deepest C-state allowed
    #[inline]
    pub fn max_cstate(&self) -> CState {
        match self {
            Self::Performance => CState::C1,
            Self::Balanced => CState::C6,
            Self::PowerSave => CState::C7,
            Self::BatterySaver => CState::C10,
            Self::Custom => CState::C6,
        }
    }
}

// ============================================================================
// BATTERY
// ============================================================================

/// Battery state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryState {
    /// AC powered, not charging
    AcPowered,
    /// Charging
    Charging,
    /// Discharging
    Discharging,
    /// Full
    Full,
    /// Not present
    NotPresent,
}

/// Battery information
#[derive(Debug, Clone)]
pub struct BatteryInfo {
    /// State
    pub state: BatteryState,
    /// Capacity (percent)
    pub capacity: u32,
    /// Voltage (millivolts)
    pub voltage_mv: u32,
    /// Current (milliamps, + = charging, - = discharging)
    pub current_ma: i32,
    /// Energy remaining (milliwatt-hours)
    pub energy_mwh: u64,
    /// Full charge capacity (mWh)
    pub full_charge_mwh: u64,
    /// Design capacity (mWh)
    pub design_mwh: u64,
    /// Estimated time remaining (seconds)
    pub time_remaining_secs: u64,
    /// Charge cycles
    pub cycles: u32,
    /// Health (percent)
    pub health: u32,
    /// Temperature (millidegrees C)
    pub temperature: i32,
}

impl BatteryInfo {
    pub fn ac_powered() -> Self {
        Self {
            state: BatteryState::AcPowered,
            capacity: 100,
            voltage_mv: 0,
            current_ma: 0,
            energy_mwh: 0,
            full_charge_mwh: 0,
            design_mwh: 0,
            time_remaining_secs: 0,
            cycles: 0,
            health: 100,
            temperature: 25000,
        }
    }

    #[inline(always)]
    pub fn is_low(&self) -> bool {
        self.capacity <= 20 && self.state == BatteryState::Discharging
    }

    #[inline(always)]
    pub fn is_critical(&self) -> bool {
        self.capacity <= 5 && self.state == BatteryState::Discharging
    }
}

// ============================================================================
// POWER BUDGET
// ============================================================================

/// Power domain
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PowerDomain {
    /// CPU package
    CpuPackage,
    /// CPU core
    CpuCore,
    /// Memory
    Memory,
    /// GPU
    Gpu,
    /// Platform/other
    Platform,
}

/// Power budget entry
#[derive(Debug, Clone)]
pub struct PowerBudget {
    /// Domain
    pub domain: PowerDomain,
    /// TDP (milliwatts)
    pub tdp_mw: u64,
    /// Current power (milliwatts)
    pub current_mw: u64,
    /// Power limit PL1 (milliwatts)
    pub pl1_mw: u64,
    /// Power limit PL2 (milliwatts, short-term boost)
    pub pl2_mw: u64,
    /// Time window for PL1 (milliseconds)
    pub pl1_window_ms: u64,
    /// Energy consumed (millijoules)
    pub energy_mj: u64,
}

impl PowerBudget {
    pub fn new(domain: PowerDomain, tdp_mw: u64) -> Self {
        Self {
            domain,
            tdp_mw,
            current_mw: 0,
            pl1_mw: tdp_mw,
            pl2_mw: tdp_mw * 125 / 100, // 1.25x TDP
            pl1_window_ms: 28000,
            energy_mj: 0,
        }
    }

    /// Is over budget?
    #[inline(always)]
    pub fn over_budget(&self) -> bool {
        self.current_mw > self.pl1_mw
    }

    /// Utilization ratio
    #[inline]
    pub fn utilization(&self) -> f64 {
        if self.pl1_mw == 0 {
            return 0.0;
        }
        self.current_mw as f64 / self.pl1_mw as f64
    }
}

// ============================================================================
// POWER MANAGER
// ============================================================================

/// Power consumption estimate
#[derive(Debug, Clone)]
pub struct PowerEstimate {
    /// Total system power (milliwatts)
    pub total_mw: u64,
    /// CPU power
    pub cpu_mw: u64,
    /// Memory power
    pub memory_mw: u64,
    /// GPU power
    pub gpu_mw: u64,
    /// Other power
    pub other_mw: u64,
    /// Estimated battery life remaining (seconds, 0 if AC)
    pub battery_life_secs: u64,
}

/// System-wide power manager
pub struct PowerManager {
    /// Current power profile
    pub profile: PowerProfile,
    /// Battery info
    pub battery: BatteryInfo,
    /// Power budgets per domain
    budgets: BTreeMap<u8, PowerBudget>,
    /// Per-core P-state
    core_pstates: Vec<PState>,
    /// Per-core C-state
    core_cstates: Vec<CState>,
    /// Power history (total mW)
    power_history: VecDeque<u64>,
    /// Max history
    max_history: usize,
    /// Profile change count
    pub profile_changes: u64,
    /// Total energy consumed (millijoules)
    pub total_energy_mj: u64,
}

impl PowerManager {
    pub fn new(num_cores: usize) -> Self {
        Self {
            profile: PowerProfile::Balanced,
            battery: BatteryInfo::ac_powered(),
            budgets: BTreeMap::new(),
            core_pstates: vec![PState::P2; num_cores],
            core_cstates: vec![CState::C0; num_cores],
            power_history: VecDeque::new(),
            max_history: 60,
            profile_changes: 0,
            total_energy_mj: 0,
        }
    }

    /// Set power profile
    #[inline(always)]
    pub fn set_profile(&mut self, profile: PowerProfile) {
        self.profile = profile;
        self.profile_changes += 1;
    }

    /// Auto-select profile based on battery
    pub fn auto_profile(&mut self) {
        let profile = match self.battery.state {
            BatteryState::AcPowered | BatteryState::Full => PowerProfile::Balanced,
            BatteryState::Charging => PowerProfile::Balanced,
            BatteryState::Discharging => {
                if self.battery.is_critical() {
                    PowerProfile::BatterySaver
                } else if self.battery.is_low() {
                    PowerProfile::PowerSave
                } else {
                    PowerProfile::Balanced
                }
            },
            BatteryState::NotPresent => PowerProfile::Performance,
        };
        if profile != self.profile {
            self.set_profile(profile);
        }
    }

    /// Update battery info
    #[inline(always)]
    pub fn update_battery(&mut self, info: BatteryInfo) {
        self.battery = info;
    }

    /// Add power budget
    #[inline(always)]
    pub fn add_budget(&mut self, budget: PowerBudget) {
        self.budgets.insert(budget.domain as u8, budget);
    }

    /// Update power for domain
    #[inline]
    pub fn update_power(&mut self, domain: PowerDomain, current_mw: u64) {
        if let Some(b) = self.budgets.get_mut(&(domain as u8)) {
            b.current_mw = current_mw;
        }
    }

    /// Set core P-state
    #[inline]
    pub fn set_pstate(&mut self, core: usize, pstate: PState) {
        if let Some(p) = self.core_pstates.get_mut(core) {
            *p = pstate;
        }
    }

    /// Set core C-state
    #[inline]
    pub fn set_cstate(&mut self, core: usize, cstate: CState) {
        if let Some(c) = self.core_cstates.get_mut(core) {
            *c = cstate;
        }
    }

    /// Estimate total power
    pub fn estimate_power(&self) -> PowerEstimate {
        let cpu_mw = self
            .budgets
            .get(&(PowerDomain::CpuPackage as u8))
            .map_or(0, |b| b.current_mw);
        let memory_mw = self
            .budgets
            .get(&(PowerDomain::Memory as u8))
            .map_or(0, |b| b.current_mw);
        let gpu_mw = self
            .budgets
            .get(&(PowerDomain::Gpu as u8))
            .map_or(0, |b| b.current_mw);
        let other_mw = self
            .budgets
            .get(&(PowerDomain::Platform as u8))
            .map_or(0, |b| b.current_mw);

        let total_mw = cpu_mw + memory_mw + gpu_mw + other_mw;

        let battery_life_secs = if total_mw > 0 && self.battery.energy_mwh > 0 {
            (self.battery.energy_mwh * 3600) / total_mw
        } else {
            0
        };

        PowerEstimate {
            total_mw,
            cpu_mw,
            memory_mw,
            gpu_mw,
            other_mw,
            battery_life_secs,
        }
    }

    /// Record power sample
    #[inline]
    pub fn record_power(&mut self, total_mw: u64, duration_ms: u64) {
        self.power_history.push_back(total_mw);
        if self.power_history.len() > self.max_history {
            self.power_history.pop_front();
        }
        // Accumulate energy
        self.total_energy_mj += total_mw * duration_ms / 1000;
    }

    /// Average power (mW)
    #[inline]
    pub fn average_power(&self) -> f64 {
        if self.power_history.is_empty() {
            return 0.0;
        }
        let sum: u64 = self.power_history.iter().sum();
        sum as f64 / self.power_history.len() as f64
    }

    /// Core count
    #[inline(always)]
    pub fn core_count(&self) -> usize {
        self.core_pstates.len()
    }

    /// Domain count
    #[inline(always)]
    pub fn domain_count(&self) -> usize {
        self.budgets.len()
    }
}
