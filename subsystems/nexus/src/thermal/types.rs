//! Core thermal types
//!
//! This module defines fundamental types for thermal management including
//! zone identifiers, cooling device identifiers, and temperature representation.

#![allow(dead_code)]

extern crate alloc;

/// Thermal zone ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ThermalZoneId(pub u32);

impl ThermalZoneId {
    /// Create new zone ID
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Cooling device ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CoolingDeviceId(pub u32);

impl CoolingDeviceId {
    /// Create new device ID
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Temperature in millidegrees Celsius
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Temperature(pub i32);

impl Temperature {
    /// Create from millidegrees
    pub const fn from_millidegrees(md: i32) -> Self {
        Self(md)
    }

    /// Create from degrees Celsius
    pub fn from_celsius(c: f32) -> Self {
        Self((c * 1000.0) as i32)
    }

    /// Get as millidegrees
    pub fn millidegrees(&self) -> i32 {
        self.0
    }

    /// Get as degrees Celsius
    pub fn celsius(&self) -> f32 {
        self.0 as f32 / 1000.0
    }

    /// Get as degrees Fahrenheit
    pub fn fahrenheit(&self) -> f32 {
        self.celsius() * 9.0 / 5.0 + 32.0
    }
}

impl core::fmt::Display for Temperature {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:.1}°C", self.celsius())
    }
}
