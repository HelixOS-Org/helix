//! Power state types and definitions.

extern crate alloc;

use crate::math;

// ============================================================================
// C-STATE
// ============================================================================

/// CPU C-State (idle state)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CState {
    /// C0 - Active
    C0,
    /// C1 - Halt
    C1,
    /// C1E - Enhanced halt
    C1E,
    /// C3 - Deep sleep
    C3,
    /// C6 - Deep power down
    C6,
    /// C7 - Package sleep
    C7,
    /// C10 - Deepest sleep
    C10,
}

impl CState {
    /// Get wakeup latency (microseconds)
    pub fn wakeup_latency(&self) -> u32 {
        match self {
            Self::C0 => 0,
            Self::C1 => 1,
            Self::C1E => 3,
            Self::C3 => 20,
            Self::C6 => 100,
            Self::C7 => 500,
            Self::C10 => 2000,
        }
    }

    /// Get power reduction (relative to C0)
    pub fn power_reduction(&self) -> f64 {
        match self {
            Self::C0 => 1.0,
            Self::C1 => 0.8,
            Self::C1E => 0.6,
            Self::C3 => 0.3,
            Self::C6 => 0.1,
            Self::C7 => 0.05,
            Self::C10 => 0.02,
        }
    }

    /// From numeric depth
    pub fn from_depth(depth: u8) -> Self {
        match depth {
            0 => Self::C0,
            1 => Self::C1,
            2 => Self::C1E,
            3 => Self::C3,
            6 => Self::C6,
            7 => Self::C7,
            10 => Self::C10,
            _ => Self::C1,
        }
    }
}

// ============================================================================
// P-STATE
// ============================================================================

/// CPU P-State (performance state)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PState {
    /// Frequency in MHz
    pub frequency_mhz: u32,
    /// Voltage in mV
    pub voltage_mv: u32,
    /// Relative performance (0.0-1.0)
    pub relative_perf: f64,
    /// Relative power consumption
    pub relative_power: f64,
}

impl PState {
    /// Create new P-State
    pub fn new(frequency_mhz: u32, voltage_mv: u32) -> Self {
        let max_freq = 4000.0; // Assumed max
        let relative_perf = frequency_mhz as f64 / max_freq;
        // Power ~ V^2 * F
        let relative_power =
            math::powi(voltage_mv as f64 / 1200.0, 2) * (frequency_mhz as f64 / max_freq);

        Self {
            frequency_mhz,
            voltage_mv,
            relative_perf,
            relative_power,
        }
    }

    /// Energy efficiency (performance per watt)
    pub fn efficiency(&self) -> f64 {
        if self.relative_power > 0.0 {
            self.relative_perf / self.relative_power
        } else {
            0.0
        }
    }
}

// ============================================================================
// POWER MODE
// ============================================================================

/// Power mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerMode {
    /// Maximum performance
    Performance,
    /// Balanced
    Balanced,
    /// Power saver
    PowerSaver,
    /// Battery saver (aggressive)
    BatterySaver,
    /// Custom
    Custom,
}

impl PowerMode {
    /// Get target performance level (0.0-1.0)
    pub fn target_performance(&self) -> f64 {
        match self {
            Self::Performance => 1.0,
            Self::Balanced => 0.7,
            Self::PowerSaver => 0.5,
            Self::BatterySaver => 0.3,
            Self::Custom => 0.6,
        }
    }

    /// Get C-State depth limit
    pub fn max_cstate(&self) -> CState {
        match self {
            Self::Performance => CState::C1,
            Self::Balanced => CState::C6,
            Self::PowerSaver => CState::C7,
            Self::BatterySaver => CState::C10,
            Self::Custom => CState::C6,
        }
    }
}
