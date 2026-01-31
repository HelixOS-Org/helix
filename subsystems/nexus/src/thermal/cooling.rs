//! Cooling device management
//!
//! This module provides cooling device representation and control
//! for managing processor throttling, fans, and other cooling mechanisms.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use super::types::CoolingDeviceId;

/// Cooling device type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoolingDeviceType {
    /// Processor (frequency scaling)
    Processor,
    /// Fan
    Fan,
    /// LCD backlight
    LcdBacklight,
    /// GPU frequency
    GpuFreq,
    /// ACPI device
    AcpiDevice,
    /// Power limit
    PowerLimit,
    /// Unknown
    Unknown,
}

impl CoolingDeviceType {
    /// Get type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Processor => "processor",
            Self::Fan => "fan",
            Self::LcdBacklight => "lcd",
            Self::GpuFreq => "gpu_freq",
            Self::AcpiDevice => "acpi_device",
            Self::PowerLimit => "power_limit",
            Self::Unknown => "unknown",
        }
    }

    /// Is active cooling
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Fan)
    }

    /// Is passive cooling
    pub fn is_passive(&self) -> bool {
        matches!(self, Self::Processor | Self::GpuFreq | Self::PowerLimit)
    }
}

/// Cooling device
#[derive(Debug)]
pub struct CoolingDevice {
    /// Device ID
    pub id: CoolingDeviceId,
    /// Device name
    pub name: String,
    /// Device type
    pub device_type: CoolingDeviceType,
    /// Current state
    cur_state: AtomicU32,
    /// Maximum state
    pub max_state: u32,
    /// Statistics
    state_transitions: AtomicU64,
    /// Time in each state (us)
    pub state_time: BTreeMap<u32, u64>,
}

impl CoolingDevice {
    /// Create new cooling device
    pub fn new(
        id: CoolingDeviceId,
        name: String,
        device_type: CoolingDeviceType,
        max_state: u32,
    ) -> Self {
        Self {
            id,
            name,
            device_type,
            cur_state: AtomicU32::new(0),
            max_state,
            state_transitions: AtomicU64::new(0),
            state_time: BTreeMap::new(),
        }
    }

    /// Get current state
    pub fn current_state(&self) -> u32 {
        self.cur_state.load(Ordering::Relaxed)
    }

    /// Set current state
    pub fn set_state(&self, state: u32) {
        let new_state = state.min(self.max_state);
        self.cur_state.store(new_state, Ordering::Relaxed);
        self.state_transitions.fetch_add(1, Ordering::Relaxed);
    }

    /// Get cooling percentage
    pub fn cooling_percentage(&self) -> f32 {
        if self.max_state == 0 {
            return 0.0;
        }
        self.current_state() as f32 / self.max_state as f32 * 100.0
    }

    /// Is at max cooling
    pub fn is_at_max(&self) -> bool {
        self.current_state() >= self.max_state
    }

    /// Get state transitions count
    pub fn state_transitions(&self) -> u64 {
        self.state_transitions.load(Ordering::Relaxed)
    }
}
