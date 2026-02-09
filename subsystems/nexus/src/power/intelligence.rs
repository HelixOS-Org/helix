//! Central power management intelligence.

extern crate alloc;

use alloc::vec::Vec;

use super::cstate::CStateSelector;
use super::energy::EnergyProfiler;
use super::pstate::PStateGovernor;
use super::thermal::ThermalManager;
use super::types::{CState, PState, PowerMode};

// ============================================================================
// POWER DECISION
// ============================================================================

/// Power decision output
#[derive(Debug, Clone)]
pub struct PowerDecision {
    /// Recommended P-State
    pub p_state: PState,
    /// Recommended C-State for idle
    pub c_state: CState,
    /// Thermal throttle percentage
    pub thermal_throttle: u8,
    /// Current power mode
    pub power_mode: PowerMode,
    /// Need emergency shutdown?
    pub emergency_shutdown: bool,
}

// ============================================================================
// POWER INTELLIGENCE
// ============================================================================

/// Central power management intelligence
pub struct PowerIntelligence {
    /// Thermal manager
    thermal: ThermalManager,
    /// C-State selector
    cstate: CStateSelector,
    /// P-State governor
    pstate: PStateGovernor,
    /// Energy profiler
    profiler: EnergyProfiler,
    /// Current power mode
    mode: PowerMode,
    /// Battery level (0-100, or None if AC)
    battery_level: Option<u8>,
    /// Is on AC power?
    on_ac: bool,
}

impl PowerIntelligence {
    /// Create new power intelligence
    pub fn new(p_states: Vec<PState>, c_states: Vec<CState>) -> Self {
        Self {
            thermal: ThermalManager::new(),
            cstate: CStateSelector::new(c_states),
            pstate: PStateGovernor::new(p_states),
            profiler: EnergyProfiler::new(),
            mode: PowerMode::Balanced,
            battery_level: None,
            on_ac: true,
        }
    }

    /// Update power state with current conditions
    pub fn update(&mut self, cpu_load: f64, minute: u8) -> PowerDecision {
        // Update P-state based on load
        let p_state = *self.pstate.update(cpu_load, minute);

        // Select C-state for next idle
        let c_state = self.cstate.select();

        // Check thermal constraints
        let thermal_throttle = self.thermal.throttle_level();

        // Adjust for battery
        let adjusted_mode = if let Some(level) = self.battery_level {
            if level < 10 && !self.on_ac {
                PowerMode::BatterySaver
            } else if level < 30 && !self.on_ac {
                PowerMode::PowerSaver
            } else {
                self.mode
            }
        } else {
            self.mode
        };

        PowerDecision {
            p_state,
            c_state,
            thermal_throttle,
            power_mode: adjusted_mode,
            emergency_shutdown: self.thermal.needs_emergency_shutdown(),
        }
    }

    /// Set battery status
    #[inline(always)]
    pub fn set_battery(&mut self, level: u8, on_ac: bool) {
        self.battery_level = Some(level);
        self.on_ac = on_ac;
    }

    /// Set power mode
    #[inline]
    pub fn set_mode(&mut self, mode: PowerMode) {
        self.mode = mode;
        self.pstate.set_power_mode(mode);
        self.cstate.set_power_mode(mode);
    }

    /// Get thermal manager
    #[inline(always)]
    pub fn thermal(&self) -> &ThermalManager {
        &self.thermal
    }

    /// Get mutable thermal manager
    #[inline(always)]
    pub fn thermal_mut(&mut self) -> &mut ThermalManager {
        &mut self.thermal
    }

    /// Get C-state selector
    #[inline(always)]
    pub fn cstate_selector(&self) -> &CStateSelector {
        &self.cstate
    }

    /// Get mutable C-state selector
    #[inline(always)]
    pub fn cstate_selector_mut(&mut self) -> &mut CStateSelector {
        &mut self.cstate
    }

    /// Get P-state governor
    #[inline(always)]
    pub fn pstate_governor(&self) -> &PStateGovernor {
        &self.pstate
    }

    /// Get mutable P-state governor
    #[inline(always)]
    pub fn pstate_governor_mut(&mut self) -> &mut PStateGovernor {
        &mut self.pstate
    }

    /// Get energy profiler
    #[inline(always)]
    pub fn profiler(&self) -> &EnergyProfiler {
        &self.profiler
    }

    /// Get mutable energy profiler
    #[inline(always)]
    pub fn profiler_mut(&mut self) -> &mut EnergyProfiler {
        &mut self.profiler
    }
}
