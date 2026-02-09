//! Thermal events
//!
//! This module provides thermal event types for tracking temperature changes,
//! trip point crossings, cooling state changes, and throttling events.

#![allow(dead_code)]

use super::types::{CoolingDeviceId, Temperature, ThermalZoneId};

/// Thermal event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalEventType {
    /// Temperature update
    TemperatureUpdate,
    /// Trip crossed (heating)
    TripCrossed,
    /// Trip cleared (cooling)
    TripCleared,
    /// Cooling state changed
    CoolingStateChanged,
    /// Critical temperature
    CriticalTemp,
    /// Throttling started
    ThrottlingStarted,
    /// Throttling stopped
    ThrottlingStopped,
}

impl ThermalEventType {
    /// Get event name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::TemperatureUpdate => "temp_update",
            Self::TripCrossed => "trip_crossed",
            Self::TripCleared => "trip_cleared",
            Self::CoolingStateChanged => "cooling_changed",
            Self::CriticalTemp => "critical_temp",
            Self::ThrottlingStarted => "throttle_start",
            Self::ThrottlingStopped => "throttle_stop",
        }
    }
}

/// Thermal event
#[derive(Debug, Clone)]
pub struct ThermalEvent {
    /// Event type
    pub event_type: ThermalEventType,
    /// Timestamp
    pub timestamp: u64,
    /// Zone ID
    pub zone: Option<ThermalZoneId>,
    /// Cooling device ID
    pub cooling_device: Option<CoolingDeviceId>,
    /// Temperature
    pub temperature: Option<Temperature>,
    /// Trip index
    pub trip_index: Option<u32>,
}

impl ThermalEvent {
    /// Create new event
    pub fn new(event_type: ThermalEventType, timestamp: u64) -> Self {
        Self {
            event_type,
            timestamp,
            zone: None,
            cooling_device: None,
            temperature: None,
            trip_index: None,
        }
    }

    /// With zone
    #[inline(always)]
    pub fn with_zone(mut self, zone: ThermalZoneId) -> Self {
        self.zone = Some(zone);
        self
    }

    /// With temperature
    #[inline(always)]
    pub fn with_temperature(mut self, temp: Temperature) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// With trip index
    #[inline(always)]
    pub fn with_trip_index(mut self, index: u32) -> Self {
        self.trip_index = Some(index);
        self
    }

    /// With cooling device
    #[inline(always)]
    pub fn with_cooling_device(mut self, device: CoolingDeviceId) -> Self {
        self.cooling_device = Some(device);
        self
    }
}
