//! Thermal zone management
//!
//! This module provides thermal zone representation, trip points, governors,
//! and temperature history tracking for thermal management.

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicI32, AtomicU64, Ordering};

use super::types::{CoolingDeviceId, Temperature, ThermalZoneId};

/// Thermal zone type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalZoneType {
    /// x86 package temperature
    X86Pkg,
    /// ACPI thermal zone
    Acpi,
    /// Core temperature
    CoreTemp,
    /// GPU temperature
    Gpu,
    /// Memory temperature
    Memory,
    /// NVMe temperature
    Nvme,
    /// Battery temperature
    Battery,
    /// SoC temperature
    Soc,
    /// PCH temperature
    Pch,
    /// Wireless temperature
    Wireless,
    /// Virtual/software zone
    Virtual,
    /// Unknown
    Unknown,
}

impl ThermalZoneType {
    /// Get type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::X86Pkg => "x86_pkg_temp",
            Self::Acpi => "acpitz",
            Self::CoreTemp => "coretemp",
            Self::Gpu => "gpu",
            Self::Memory => "memory",
            Self::Nvme => "nvme",
            Self::Battery => "battery",
            Self::Soc => "soc",
            Self::Pch => "pch",
            Self::Wireless => "wireless",
            Self::Virtual => "virtual",
            Self::Unknown => "unknown",
        }
    }

    /// Is CPU related
    pub fn is_cpu(&self) -> bool {
        matches!(self, Self::X86Pkg | Self::CoreTemp | Self::Soc)
    }
}

/// Thermal zone mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalZoneMode {
    /// Enabled
    Enabled,
    /// Disabled
    Disabled,
}

impl ThermalZoneMode {
    /// Get mode name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
        }
    }
}

/// Trip point type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TripPointType {
    /// Active - activate cooling
    Active,
    /// Passive - slow down
    Passive,
    /// Hot - throttle aggressively
    Hot,
    /// Critical - shut down
    Critical,
}

impl TripPointType {
    /// Get type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Passive => "passive",
            Self::Hot => "hot",
            Self::Critical => "critical",
        }
    }

    /// Priority (higher is more severe)
    pub fn severity(&self) -> u8 {
        match self {
            Self::Active => 1,
            Self::Passive => 2,
            Self::Hot => 3,
            Self::Critical => 4,
        }
    }
}

/// Trip point
#[derive(Debug, Clone)]
pub struct TripPoint {
    /// Trip point index
    pub index: u32,
    /// Type
    pub trip_type: TripPointType,
    /// Temperature
    pub temperature: Temperature,
    /// Hysteresis
    pub hysteresis: Temperature,
    /// Associated cooling devices
    pub cooling_devices: Vec<CoolingDeviceId>,
}

impl TripPoint {
    /// Create new trip point
    pub fn new(index: u32, trip_type: TripPointType, temperature: Temperature) -> Self {
        Self {
            index,
            trip_type,
            temperature,
            hysteresis: Temperature::from_millidegrees(0),
            cooling_devices: Vec::new(),
        }
    }

    /// Is triggered
    pub fn is_triggered(&self, current: Temperature) -> bool {
        current.0 >= self.temperature.0
    }

    /// Is cleared (with hysteresis)
    pub fn is_cleared(&self, current: Temperature) -> bool {
        current.0 < (self.temperature.0 - self.hysteresis.0)
    }
}

/// Thermal governor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalGovernor {
    /// Step-wise
    StepWise,
    /// Fair share
    FairShare,
    /// Bang bang
    BangBang,
    /// User space
    UserSpace,
    /// Power allocator
    PowerAllocator,
}

impl ThermalGovernor {
    /// Get governor name
    pub fn name(&self) -> &'static str {
        match self {
            Self::StepWise => "step_wise",
            Self::FairShare => "fair_share",
            Self::BangBang => "bang_bang",
            Self::UserSpace => "user_space",
            Self::PowerAllocator => "power_allocator",
        }
    }
}

/// Thermal zone
#[derive(Debug)]
pub struct ThermalZone {
    /// Zone ID
    pub id: ThermalZoneId,
    /// Zone name
    pub name: String,
    /// Zone type
    pub zone_type: ThermalZoneType,
    /// Mode
    pub mode: ThermalZoneMode,
    /// Current temperature
    temperature: AtomicI32,
    /// Trip points
    pub trip_points: Vec<TripPoint>,
    /// Bound cooling devices
    pub cooling_devices: Vec<CoolingDeviceId>,
    /// Polling interval (ms)
    pub polling_interval_ms: u32,
    /// Governor
    pub governor: ThermalGovernor,
    /// Temperature history
    temp_history: Vec<Temperature>,
    /// Max history size
    max_history: usize,
    /// Peak temperature
    peak_temp: AtomicI32,
    /// Sample count
    sample_count: AtomicU64,
}

impl ThermalZone {
    /// Create new zone
    pub fn new(id: ThermalZoneId, name: String, zone_type: ThermalZoneType) -> Self {
        Self {
            id,
            name,
            zone_type,
            mode: ThermalZoneMode::Enabled,
            temperature: AtomicI32::new(0),
            trip_points: Vec::new(),
            cooling_devices: Vec::new(),
            polling_interval_ms: 1000,
            governor: ThermalGovernor::StepWise,
            temp_history: Vec::new(),
            max_history: 100,
            peak_temp: AtomicI32::new(0),
            sample_count: AtomicU64::new(0),
        }
    }

    /// Get current temperature
    pub fn temperature(&self) -> Temperature {
        Temperature(self.temperature.load(Ordering::Relaxed))
    }

    /// Update temperature
    pub fn update_temperature(&mut self, temp: Temperature) {
        self.temperature.store(temp.0, Ordering::Relaxed);
        self.sample_count.fetch_add(1, Ordering::Relaxed);

        // Update peak
        let current_peak = self.peak_temp.load(Ordering::Relaxed);
        if temp.0 > current_peak {
            self.peak_temp.store(temp.0, Ordering::Relaxed);
        }

        // Add to history
        if self.temp_history.len() >= self.max_history {
            self.temp_history.remove(0);
        }
        self.temp_history.push(temp);
    }

    /// Get peak temperature
    pub fn peak_temperature(&self) -> Temperature {
        Temperature(self.peak_temp.load(Ordering::Relaxed))
    }

    /// Add trip point
    pub fn add_trip_point(&mut self, trip: TripPoint) {
        self.trip_points.push(trip);
        self.trip_points.sort_by_key(|t| t.temperature.0);
    }

    /// Get critical temperature
    pub fn critical_temperature(&self) -> Option<Temperature> {
        self.trip_points
            .iter()
            .find(|t| matches!(t.trip_type, TripPointType::Critical))
            .map(|t| t.temperature)
    }

    /// Get triggered trip points
    pub fn triggered_trips(&self) -> Vec<&TripPoint> {
        let current = self.temperature();
        self.trip_points
            .iter()
            .filter(|t| t.is_triggered(current))
            .collect()
    }

    /// Average temperature
    pub fn average_temperature(&self) -> Temperature {
        if self.temp_history.is_empty() {
            return self.temperature();
        }
        let sum: i32 = self.temp_history.iter().map(|t| t.0).sum();
        Temperature(sum / self.temp_history.len() as i32)
    }

    /// Temperature trend (millidegrees per sample)
    pub fn temperature_trend(&self) -> i32 {
        if self.temp_history.len() < 2 {
            return 0;
        }
        let recent_avg: i32 = self
            .temp_history
            .iter()
            .rev()
            .take(5)
            .map(|t| t.0)
            .sum::<i32>()
            / 5.min(self.temp_history.len()) as i32;

        let older_avg: i32 = self.temp_history.iter().take(5).map(|t| t.0).sum::<i32>()
            / 5.min(self.temp_history.len()) as i32;

        recent_avg - older_avg
    }
}
