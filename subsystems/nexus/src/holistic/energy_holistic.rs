//! # Holistic Energy Optimizer
//!
//! System-wide energy/power management and optimization:
//! - DVFS coordination
//! - Power domain management
//! - Energy-performance tradeoff
//! - Thermal-aware power budgeting
//! - Battery/UPS awareness

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// ENERGY TYPES
// ============================================================================

/// Power state
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PowerState {
    /// Full performance
    Performance,
    /// Balanced
    Balanced,
    /// Power save
    PowerSave,
    /// Deep power save
    DeepSave,
    /// Standby
    Standby,
    /// Off
    Off,
}

impl PowerState {
    /// Performance multiplier
    pub fn performance_factor(&self) -> f64 {
        match self {
            PowerState::Performance => 1.0,
            PowerState::Balanced => 0.85,
            PowerState::PowerSave => 0.6,
            PowerState::DeepSave => 0.35,
            PowerState::Standby => 0.05,
            PowerState::Off => 0.0,
        }
    }

    /// Power consumption factor (relative to max)
    pub fn power_factor(&self) -> f64 {
        match self {
            PowerState::Performance => 1.0,
            PowerState::Balanced => 0.7,
            PowerState::PowerSave => 0.4,
            PowerState::DeepSave => 0.2,
            PowerState::Standby => 0.02,
            PowerState::Off => 0.0,
        }
    }
}

/// Power source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerSource {
    /// AC mains
    Mains,
    /// Battery
    Battery,
    /// UPS
    Ups,
    /// Solar/renewable
    Renewable,
}

/// Energy profile type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyProfile {
    /// Maximum performance
    MaxPerformance,
    /// Balanced performance and power
    Balanced,
    /// Minimum power
    MinPower,
    /// Custom
    Custom,
}

// ============================================================================
// POWER DOMAIN
// ============================================================================

/// A power domain (group of components sharing power rail)
#[derive(Debug)]
pub struct PowerDomain {
    /// Domain id
    pub id: u64,
    /// Current state
    pub state: PowerState,
    /// Max power (milliwatts)
    pub max_power_mw: u64,
    /// Current power (milliwatts)
    pub current_power_mw: u64,
    /// Frequency (MHz)
    pub frequency_mhz: u32,
    /// Min frequency
    pub min_freq_mhz: u32,
    /// Max frequency
    pub max_freq_mhz: u32,
    /// Voltage (mV)
    pub voltage_mv: u32,
    /// Temperature (mC = millidegrees Celsius)
    pub temperature_mc: i32,
    /// Thermal limit (mC)
    pub thermal_limit_mc: i32,
    /// Processes assigned
    pub processes: Vec<u64>,
}

impl PowerDomain {
    pub fn new(id: u64, max_power_mw: u64) -> Self {
        Self {
            id,
            state: PowerState::Balanced,
            max_power_mw,
            current_power_mw: 0,
            frequency_mhz: 2000,
            min_freq_mhz: 800,
            max_freq_mhz: 4000,
            voltage_mv: 1000,
            temperature_mc: 40_000,
            thermal_limit_mc: 95_000,
            processes: Vec::new(),
        }
    }

    /// Power utilization
    pub fn power_utilization(&self) -> f64 {
        if self.max_power_mw == 0 {
            return 0.0;
        }
        self.current_power_mw as f64 / self.max_power_mw as f64
    }

    /// Frequency utilization
    pub fn freq_utilization(&self) -> f64 {
        if self.max_freq_mhz == 0 {
            return 0.0;
        }
        self.frequency_mhz as f64 / self.max_freq_mhz as f64
    }

    /// Thermal headroom (degrees C)
    pub fn thermal_headroom_c(&self) -> f64 {
        (self.thermal_limit_mc - self.temperature_mc) as f64 / 1000.0
    }

    /// Is thermally throttled?
    pub fn is_thermal_throttled(&self) -> bool {
        self.temperature_mc >= (self.thermal_limit_mc - 5000) // within 5°C
    }

    /// Set DVFS state
    pub fn set_frequency(&mut self, freq_mhz: u32) {
        self.frequency_mhz = freq_mhz.clamp(self.min_freq_mhz, self.max_freq_mhz);
    }

    /// Energy per operation estimate (arbitrary units)
    pub fn energy_per_op(&self) -> f64 {
        // P = C * V^2 * f, energy per op ~ V^2 / f ∝ V^2
        let v = self.voltage_mv as f64 / 1000.0;
        v * v
    }
}

// ============================================================================
// BATTERY STATE
// ============================================================================

/// Battery state
#[derive(Debug, Clone)]
pub struct BatteryState {
    /// Charge percentage (0-100)
    pub charge_pct: f64,
    /// Discharging rate (mW)
    pub discharge_rate_mw: u64,
    /// Charging?
    pub charging: bool,
    /// Estimated time remaining (seconds)
    pub time_remaining_s: u64,
    /// Battery health (0-100)
    pub health_pct: f64,
    /// Cycle count
    pub cycle_count: u32,
}

impl BatteryState {
    pub fn new() -> Self {
        Self {
            charge_pct: 100.0,
            discharge_rate_mw: 0,
            charging: true,
            time_remaining_s: u64::MAX,
            health_pct: 100.0,
            cycle_count: 0,
        }
    }

    /// Is battery low?
    pub fn is_low(&self) -> bool {
        self.charge_pct < 20.0
    }

    /// Is critical?
    pub fn is_critical(&self) -> bool {
        self.charge_pct < 5.0
    }
}

// ============================================================================
// ENERGY BUDGET
// ============================================================================

/// System energy budget
#[derive(Debug, Clone)]
pub struct EnergyBudget {
    /// Total power budget (mW)
    pub total_mw: u64,
    /// Allocated power (mW)
    pub allocated_mw: u64,
    /// Per-domain allocations
    allocations: BTreeMap<u64, u64>,
}

impl EnergyBudget {
    pub fn new(total_mw: u64) -> Self {
        Self {
            total_mw,
            allocated_mw: 0,
            allocations: BTreeMap::new(),
        }
    }

    /// Allocate power to domain
    pub fn allocate(&mut self, domain: u64, power_mw: u64) -> bool {
        let current = self.allocations.get(&domain).copied().unwrap_or(0);
        let delta = power_mw.saturating_sub(current);
        if self.allocated_mw + delta > self.total_mw {
            return false;
        }
        self.allocated_mw = self.allocated_mw - current + power_mw;
        self.allocations.insert(domain, power_mw);
        true
    }

    /// Release allocation
    pub fn release(&mut self, domain: u64) {
        if let Some(amount) = self.allocations.remove(&domain) {
            self.allocated_mw = self.allocated_mw.saturating_sub(amount);
        }
    }

    /// Remaining budget
    pub fn remaining_mw(&self) -> u64 {
        self.total_mw.saturating_sub(self.allocated_mw)
    }

    /// Utilization
    pub fn utilization(&self) -> f64 {
        if self.total_mw == 0 {
            return 0.0;
        }
        self.allocated_mw as f64 / self.total_mw as f64
    }
}

// ============================================================================
// ENERGY ENGINE
// ============================================================================

/// Energy stats
#[derive(Debug, Clone, Default)]
pub struct HolisticEnergyStats {
    /// Power domains
    pub domain_count: usize,
    /// Total power draw (mW)
    pub total_power_mw: u64,
    /// Budget utilization
    pub budget_utilization: f64,
    /// Throttled domains
    pub throttled_domains: usize,
}

/// Holistic energy optimizer
pub struct HolisticEnergyEngine {
    /// Power domains
    domains: BTreeMap<u64, PowerDomain>,
    /// Active profile
    pub profile: EnergyProfile,
    /// Power source
    pub source: PowerSource,
    /// Battery state
    pub battery: BatteryState,
    /// Energy budget
    budget: EnergyBudget,
    /// Stats
    stats: HolisticEnergyStats,
}

impl HolisticEnergyEngine {
    pub fn new(total_budget_mw: u64) -> Self {
        Self {
            domains: BTreeMap::new(),
            profile: EnergyProfile::Balanced,
            source: PowerSource::Mains,
            battery: BatteryState::new(),
            budget: EnergyBudget::new(total_budget_mw),
            stats: HolisticEnergyStats::default(),
        }
    }

    /// Add domain
    pub fn add_domain(&mut self, id: u64, max_power_mw: u64) {
        self.domains.insert(id, PowerDomain::new(id, max_power_mw));
        self.update_stats();
    }

    /// Update domain power
    pub fn update_power(&mut self, domain_id: u64, current_mw: u64) {
        if let Some(domain) = self.domains.get_mut(&domain_id) {
            domain.current_power_mw = current_mw;
        }
        self.update_stats();
    }

    /// Update domain temperature
    pub fn update_temperature(&mut self, domain_id: u64, temp_mc: i32) {
        if let Some(domain) = self.domains.get_mut(&domain_id) {
            domain.temperature_mc = temp_mc;
            // Auto-throttle on thermal
            if domain.is_thermal_throttled() {
                domain.state = PowerState::PowerSave;
            }
        }
        self.update_stats();
    }

    /// Set frequency
    pub fn set_frequency(&mut self, domain_id: u64, freq_mhz: u32) {
        if let Some(domain) = self.domains.get_mut(&domain_id) {
            domain.set_frequency(freq_mhz);
        }
    }

    /// Get thermally throttled domains
    pub fn throttled_domains(&self) -> Vec<u64> {
        self.domains
            .values()
            .filter(|d| d.is_thermal_throttled())
            .map(|d| d.id)
            .collect()
    }

    /// Total current power draw
    pub fn total_power_mw(&self) -> u64 {
        self.domains.values().map(|d| d.current_power_mw).sum()
    }

    /// Apply energy profile
    pub fn apply_profile(&mut self, profile: EnergyProfile) {
        self.profile = profile;
        let target_state = match profile {
            EnergyProfile::MaxPerformance => PowerState::Performance,
            EnergyProfile::Balanced => PowerState::Balanced,
            EnergyProfile::MinPower => PowerState::PowerSave,
            EnergyProfile::Custom => return,
        };
        for domain in self.domains.values_mut() {
            domain.state = target_state;
        }
    }

    fn update_stats(&mut self) {
        self.stats.domain_count = self.domains.len();
        self.stats.total_power_mw = self.total_power_mw();
        self.stats.budget_utilization = self.budget.utilization();
        self.stats.throttled_domains = self.throttled_domains().len();
    }

    /// Stats
    pub fn stats(&self) -> &HolisticEnergyStats {
        &self.stats
    }
}
