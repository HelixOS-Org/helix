// SPDX-License-Identifier: GPL-2.0
//! Holistic power_mgr — system power management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Power state (ACPI S-states)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemPowerState {
    S0Working,
    S1Standby,
    S2Sleep,
    S3SuspendToRam,
    S4Hibernate,
    S5SoftOff,
}

/// Device power state (D-states)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevicePowerState {
    D0Full,
    D1Light,
    D2Medium,
    D3Hot,
    D3Cold,
}

/// Power domain
#[derive(Debug)]
pub struct PowerDomain {
    pub id: u64,
    pub device_count: u32,
    pub power_mw: u64,
    pub devices: Vec<u64>,
    pub governor_active: bool,
}

/// Device power entry
#[derive(Debug)]
pub struct DevicePowerEntry {
    pub device_id: u64,
    pub state: DevicePowerState,
    pub resume_latency_us: u64,
    pub power_consumption_mw: u64,
    pub runtime_active_ms: u64,
    pub runtime_suspended_ms: u64,
    pub transitions: u64,
}

impl DevicePowerEntry {
    pub fn new(id: u64) -> Self {
        Self { device_id: id, state: DevicePowerState::D0Full, resume_latency_us: 0, power_consumption_mw: 0, runtime_active_ms: 0, runtime_suspended_ms: 0, transitions: 0 }
    }

    pub fn suspend(&mut self, state: DevicePowerState) { self.state = state; self.transitions += 1; }
    pub fn resume(&mut self) { self.state = DevicePowerState::D0Full; self.transitions += 1; }
}

/// Stats
#[derive(Debug, Clone)]
pub struct PowerMgrStats {
    pub system_state: SystemPowerState,
    pub total_devices: u32,
    pub active_devices: u32,
    pub suspended_devices: u32,
    pub total_power_mw: u64,
    pub total_transitions: u64,
}

/// Main power manager
pub struct HolisticPowerMgr {
    system_state: SystemPowerState,
    devices: BTreeMap<u64, DevicePowerEntry>,
    domains: Vec<PowerDomain>,
}

impl HolisticPowerMgr {
    pub fn new() -> Self { Self { system_state: SystemPowerState::S0Working, devices: BTreeMap::new(), domains: Vec::new() } }

    pub fn register_device(&mut self, id: u64) { self.devices.insert(id, DevicePowerEntry::new(id)); }

    pub fn suspend_device(&mut self, id: u64, state: DevicePowerState) {
        if let Some(d) = self.devices.get_mut(&id) { d.suspend(state); }
    }

    pub fn resume_device(&mut self, id: u64) {
        if let Some(d) = self.devices.get_mut(&id) { d.resume(); }
    }

    pub fn stats(&self) -> PowerMgrStats {
        let active = self.devices.values().filter(|d| d.state == DevicePowerState::D0Full).count() as u32;
        let suspended = self.devices.len() as u32 - active;
        let power: u64 = self.devices.values().map(|d| d.power_consumption_mw).sum();
        let transitions: u64 = self.devices.values().map(|d| d.transitions).sum();
        PowerMgrStats { system_state: self.system_state, total_devices: self.devices.len() as u32, active_devices: active, suspended_devices: suspended, total_power_mw: power, total_transitions: transitions }
    }
}
