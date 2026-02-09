//! # Application Thermal Impact Analysis
//!
//! Per-application thermal behavior profiling:
//! - Per-process heat contribution estimation
//! - Thermal throttle impact tracking
//! - Core hotspot attribution
//! - Thermal-aware placement hints
//! - Cooling requirement estimation
//! - Thermal budget management

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

// ============================================================================
// THERMAL ZONES
// ============================================================================

/// Thermal zone type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThermalZone {
    /// CPU package
    CpuPackage(u32),
    /// Individual core
    CpuCore(u32),
    /// Memory (DIMM)
    Memory(u32),
    /// GPU
    Gpu,
    /// SSD/NVMe
    Storage(u32),
    /// VRM
    Vrm,
    /// PCH / chipset
    Chipset,
    /// Ambient
    Ambient,
}

/// Temperature reading (millidegrees Celsius)
pub type MilliCelsius = i32;

/// Temperature reading
#[derive(Debug, Clone, Copy)]
pub struct ThermalReading {
    /// Zone
    pub zone: ThermalZone,
    /// Temperature (millidegrees C)
    pub temp_mc: MilliCelsius,
    /// Timestamp
    pub timestamp: u64,
}

// ============================================================================
// THERMAL STATE
// ============================================================================

/// Thermal state
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThermalState {
    /// Cool - well below limits
    Cool       = 0,
    /// Normal operating range
    Normal     = 1,
    /// Warm - approaching limits
    Warm       = 2,
    /// Hot - throttling imminent
    Hot        = 3,
    /// Throttling active
    Throttling = 4,
    /// Critical - emergency shutdown range
    Critical   = 5,
}

impl ThermalState {
    /// From temperature relative to limits
    pub fn from_temp(
        temp_mc: MilliCelsius,
        passive_mc: MilliCelsius,
        critical_mc: MilliCelsius,
    ) -> Self {
        if temp_mc >= critical_mc {
            Self::Critical
        } else if temp_mc >= passive_mc {
            Self::Throttling
        } else if temp_mc >= passive_mc - 5000 {
            Self::Hot
        } else if temp_mc >= passive_mc - 15000 {
            Self::Warm
        } else if temp_mc >= passive_mc - 30000 {
            Self::Normal
        } else {
            Self::Cool
        }
    }
}

// ============================================================================
// HEAT CONTRIBUTION
// ============================================================================

/// Heat contribution from a process
#[derive(Debug, Clone)]
pub struct HeatContribution {
    /// Process ID
    pub pid: u64,
    /// CPU time in period (ms)
    pub cpu_time_ms: u64,
    /// Instructions retired (approximate)
    pub instructions: u64,
    /// Cache misses (approximate)
    pub cache_misses: u64,
    /// Estimated heat (milliwatts)
    pub estimated_mw: u32,
    /// Percentage of total system heat
    pub heat_pct: f64,
    /// Cores used
    pub cores_used: Vec<u32>,
}

/// Per-core heat attribution
#[derive(Debug, Clone)]
pub struct CoreHeatMap {
    /// Core ID to temperature contribution (mW by process)
    pub core_contributions: BTreeMap<u32, BTreeMap<u64, u32>>,
    /// Core temperatures
    pub core_temps: BTreeMap<u32, MilliCelsius>,
    /// Hottest core
    pub hottest_core: Option<u32>,
    /// Coolest core
    pub coolest_core: Option<u32>,
}

impl CoreHeatMap {
    pub fn new() -> Self {
        Self {
            core_contributions: BTreeMap::new(),
            core_temps: BTreeMap::new(),
            hottest_core: None,
            coolest_core: None,
        }
    }

    /// Record contribution
    #[inline]
    pub fn record(&mut self, core: u32, pid: u64, mw: u32) {
        let core_map = self
            .core_contributions
            .entry(core)
            .or_insert_with(BTreeMap::new);
        *core_map.entry(pid).or_insert(0) += mw;
    }

    /// Update temperatures
    #[inline]
    pub fn update_temps(&mut self, temps: &[(u32, MilliCelsius)]) {
        for &(core, temp) in temps {
            self.core_temps.insert(core, temp);
        }

        self.hottest_core = self.core_temps.iter().max_by_key(|e| e.1).map(|e| *e.0);
        self.coolest_core = self.core_temps.iter().min_by_key(|e| e.1).map(|e| *e.0);
    }

    /// Get top heat contributors for a core
    #[inline]
    pub fn top_contributors(&self, core: u32, n: usize) -> Vec<(u64, u32)> {
        let empty = BTreeMap::new();
        let contribs = self.core_contributions.get(&core).unwrap_or(&empty);
        let mut sorted: Vec<(u64, u32)> = contribs.iter().map(|(&pid, &mw)| (pid, mw)).collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(n);
        sorted
    }

    /// Temperature spread (max - min)
    #[inline]
    pub fn temperature_spread(&self) -> MilliCelsius {
        let max = self.core_temps.values().max().copied().unwrap_or(0);
        let min = self.core_temps.values().min().copied().unwrap_or(0);
        max - min
    }
}

// ============================================================================
// THERMAL PROFILE
// ============================================================================

/// Thermal profile for a process
#[derive(Debug, Clone)]
pub struct ProcessThermalProfile {
    /// Process ID
    pub pid: u64,
    /// Thermal impact rating
    pub impact: ThermalImpact,
    /// Average heat contribution (mW)
    pub avg_heat_mw: u32,
    /// Peak heat contribution (mW)
    pub peak_heat_mw: u32,
    /// Times caused throttling
    pub throttle_contributions: u32,
    /// Preferred cores (cooler ones)
    pub preferred_cores: Vec<u32>,
    /// Cores to avoid (hot)
    pub avoid_cores: Vec<u32>,
    /// Thermal budget remaining (mW)
    pub budget_remaining_mw: u32,
}

/// Thermal impact level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThermalImpact {
    /// Negligible heat
    Negligible = 0,
    /// Low impact
    Low        = 1,
    /// Moderate
    Moderate   = 2,
    /// High
    High       = 3,
    /// Extreme
    Extreme    = 4,
}

// ============================================================================
// THERMAL BUDGET
// ============================================================================

/// Thermal power budget
#[derive(Debug, Clone)]
pub struct ThermalBudget {
    /// Total system thermal budget (mW)
    pub system_budget_mw: u32,
    /// Currently allocated (mW)
    pub allocated_mw: u32,
    /// Per-process allocation
    pub process_budgets: LinearMap<u32, 64>,
}

impl ThermalBudget {
    pub fn new(system_budget_mw: u32) -> Self {
        Self {
            system_budget_mw,
            allocated_mw: 0,
            process_budgets: LinearMap::new(),
        }
    }

    /// Remaining budget
    #[inline(always)]
    pub fn remaining(&self) -> u32 {
        self.system_budget_mw.saturating_sub(self.allocated_mw)
    }

    /// Allocate budget to process
    #[inline]
    pub fn allocate(&mut self, pid: u64, mw: u32) -> bool {
        if mw > self.remaining() {
            return false;
        }
        let existing = self.process_budgets.get(pid).copied().unwrap_or(0);
        self.allocated_mw = self.allocated_mw.saturating_sub(existing);
        self.process_budgets.insert(pid, mw);
        self.allocated_mw += mw;
        true
    }

    /// Release budget
    #[inline]
    pub fn release(&mut self, pid: u64) {
        if let Some(mw) = self.process_budgets.remove(pid) {
            self.allocated_mw = self.allocated_mw.saturating_sub(mw);
        }
    }

    /// Is process over budget
    #[inline]
    pub fn is_over_budget(&self, pid: u64, current_mw: u32) -> bool {
        self.process_budgets
            .get(&pid)
            .map(|&budget| current_mw > budget)
            .unwrap_or(false)
    }
}

// ============================================================================
// THROTTLE EVENT
// ============================================================================

/// Thermal throttle event
#[derive(Debug, Clone)]
pub struct ThrottleEvent {
    /// Zone that triggered
    pub zone: ThermalZone,
    /// Temperature at trigger
    pub temp_mc: MilliCelsius,
    /// Timestamp
    pub timestamp: u64,
    /// Duration (ms)
    pub duration_ms: u64,
    /// Processes affected
    pub affected_pids: Vec<u64>,
    /// Frequency reduction (MHz)
    pub freq_reduction_mhz: u32,
}

// ============================================================================
// THERMAL ANALYZER
// ============================================================================

/// Thermal analyzer stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppThermalStats {
    /// Total processes tracked
    pub tracked: usize,
    /// Throttle events
    pub throttle_events: u64,
    /// Current system temperature (mC)
    pub system_temp_mc: MilliCelsius,
    /// Current thermal state
    pub system_state: u8,
    /// Budget utilization (%)
    pub budget_utilization_pct: u32,
}

/// Application thermal analyzer
pub struct AppThermalAnalyzer {
    /// Per-process heat samples
    heat_samples: BTreeMap<u64, Vec<HeatContribution>>,
    /// Profiles
    profiles: BTreeMap<u64, ProcessThermalProfile>,
    /// Core heat map
    core_map: CoreHeatMap,
    /// Thermal budget
    budget: ThermalBudget,
    /// Throttle history
    throttle_history: Vec<ThrottleEvent>,
    /// Current thermal state
    state: ThermalState,
    /// Passive trip point (mC)
    passive_trip_mc: MilliCelsius,
    /// Critical trip point (mC)
    critical_trip_mc: MilliCelsius,
    /// Max samples per process
    max_samples: usize,
    /// Stats
    stats: AppThermalStats,
}

impl AppThermalAnalyzer {
    pub fn new(budget_mw: u32, passive_mc: MilliCelsius, critical_mc: MilliCelsius) -> Self {
        Self {
            heat_samples: BTreeMap::new(),
            profiles: BTreeMap::new(),
            core_map: CoreHeatMap::new(),
            budget: ThermalBudget::new(budget_mw),
            throttle_history: Vec::new(),
            state: ThermalState::Cool,
            passive_trip_mc: passive_mc,
            critical_trip_mc: critical_mc,
            max_samples: 256,
            stats: AppThermalStats::default(),
        }
    }

    /// Record heat contribution
    pub fn record_heat(&mut self, contrib: HeatContribution) {
        let pid = contrib.pid;

        // Update core heat map
        for &core in &contrib.cores_used {
            self.core_map.record(core, pid, contrib.estimated_mw);
        }

        let samples = self.heat_samples.entry(pid).or_insert_with(Vec::new);
        samples.push(contrib);
        if samples.len() > self.max_samples {
            samples.pop_front();
        }
    }

    /// Update zone temperature
    pub fn update_temperature(&mut self, reading: ThermalReading) {
        if let ThermalZone::CpuCore(core) = reading.zone {
            self.core_map.update_temps(&[(core, reading.temp_mc)]);
        }

        if matches!(reading.zone, ThermalZone::CpuPackage(_)) {
            self.state = ThermalState::from_temp(
                reading.temp_mc,
                self.passive_trip_mc,
                self.critical_trip_mc,
            );
            self.stats.system_temp_mc = reading.temp_mc;
            self.stats.system_state = self.state as u8;
        }
    }

    /// Record throttle event
    #[inline]
    pub fn record_throttle(&mut self, event: ThrottleEvent) {
        // Update affected process profiles
        for &pid in &event.affected_pids {
            if let Some(profile) = self.profiles.get_mut(&pid) {
                profile.throttle_contributions += 1;
            }
        }

        self.throttle_history.push(event);
        self.stats.throttle_events += 1;
    }

    /// Analyze process thermal impact
    pub fn analyze(&mut self, pid: u64) -> Option<&ProcessThermalProfile> {
        let samples = self.heat_samples.get(&pid)?;
        if samples.is_empty() {
            return None;
        }

        let avg_heat = (samples.iter().map(|s| s.estimated_mw as u64).sum::<u64>()
            / samples.len() as u64) as u32;
        let peak_heat = samples.iter().map(|s| s.estimated_mw).max().unwrap_or(0);

        let impact = if avg_heat < 100 {
            ThermalImpact::Negligible
        } else if avg_heat < 500 {
            ThermalImpact::Low
        } else if avg_heat < 2000 {
            ThermalImpact::Moderate
        } else if avg_heat < 5000 {
            ThermalImpact::High
        } else {
            ThermalImpact::Extreme
        };

        // Preferred cores: coolest
        let mut preferred = Vec::new();
        let mut avoid = Vec::new();
        let spread = self.core_map.temperature_spread();

        if spread > 5000 {
            // Significant spread
            if let Some(coolest) = self.core_map.coolest_core {
                preferred.push(coolest);
            }
            if let Some(hottest) = self.core_map.hottest_core {
                avoid.push(hottest);
            }
        }

        let budget_remaining = self
            .budget
            .process_budgets
            .get(&pid)
            .copied()
            .unwrap_or(0)
            .saturating_sub(avg_heat);

        let throttle_contributions = self
            .profiles
            .get(&pid)
            .map(|p| p.throttle_contributions)
            .unwrap_or(0);

        self.profiles.insert(pid, ProcessThermalProfile {
            pid,
            impact,
            avg_heat_mw: avg_heat,
            peak_heat_mw: peak_heat,
            throttle_contributions,
            preferred_cores: preferred,
            avoid_cores: avoid,
            budget_remaining_mw: budget_remaining,
        });

        self.stats.tracked = self.profiles.len();
        self.profiles.get(&pid)
    }

    /// Allocate thermal budget
    #[inline]
    pub fn allocate_budget(&mut self, pid: u64, mw: u32) -> bool {
        let result = self.budget.allocate(pid, mw);
        if self.budget.system_budget_mw > 0 {
            self.stats.budget_utilization_pct = (self.budget.allocated_mw as u64 * 100
                / self.budget.system_budget_mw as u64)
                as u32;
        }
        result
    }

    /// Get current state
    #[inline(always)]
    pub fn state(&self) -> ThermalState {
        self.state
    }

    /// Get core heat map
    #[inline(always)]
    pub fn core_map(&self) -> &CoreHeatMap {
        &self.core_map
    }

    /// Get profile
    #[inline(always)]
    pub fn profile(&self, pid: u64) -> Option<&ProcessThermalProfile> {
        self.profiles.get(&pid)
    }

    /// Get stats
    #[inline(always)]
    pub fn thermal_stats(&self) -> &AppThermalStats {
        &self.stats
    }

    /// Unregister
    #[inline]
    pub fn unregister(&mut self, pid: u64) {
        self.heat_samples.remove(&pid);
        self.profiles.remove(&pid);
        self.budget.release(pid);
        self.stats.tracked = self.profiles.len();
    }
}
