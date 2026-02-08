// SPDX-License-Identifier: GPL-2.0
//! Holistic devfreq_mgr — device frequency manager.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;

/// Device frequency governor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevFreqGovernor {
    SimpleonDemand,
    Performance,
    Powersave,
    Passive,
    Userspace,
}

/// Device power state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevPowerState {
    Active,
    Idle,
    Suspended,
    RuntimeSuspended,
}

/// Device frequency profile
#[derive(Debug)]
pub struct DevFreqProfile {
    pub device_id: u64,
    pub name: String,
    pub current_freq_hz: u64,
    pub min_freq_hz: u64,
    pub max_freq_hz: u64,
    pub governor: DevFreqGovernor,
    pub power_state: DevPowerState,
    pub busy_time_ns: u64,
    pub total_time_ns: u64,
    pub transitions: u64,
}

impl DevFreqProfile {
    pub fn new(id: u64, name: String, min: u64, max: u64) -> Self {
        Self { device_id: id, name, current_freq_hz: max, min_freq_hz: min, max_freq_hz: max, governor: DevFreqGovernor::SimpleonDemand, power_state: DevPowerState::Active, busy_time_ns: 0, total_time_ns: 0, transitions: 0 }
    }

    pub fn utilization(&self) -> f64 { if self.total_time_ns == 0 { 0.0 } else { self.busy_time_ns as f64 / self.total_time_ns as f64 } }

    pub fn set_freq(&mut self, freq: u64) {
        let f = freq.clamp(self.min_freq_hz, self.max_freq_hz);
        if f != self.current_freq_hz { self.current_freq_hz = f; self.transitions += 1; }
    }

    pub fn update_utilization(&mut self, busy_ns: u64, total_ns: u64) {
        self.busy_time_ns += busy_ns;
        self.total_time_ns += total_ns;
        let util = if total_ns == 0 { 0.0 } else { busy_ns as f64 / total_ns as f64 };
        let range = self.max_freq_hz - self.min_freq_hz;
        let target = self.min_freq_hz + (range as f64 * util) as u64;
        self.set_freq(target);
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct DevFreqMgrStats {
    pub total_devices: u32,
    pub active_devices: u32,
    pub total_transitions: u64,
    pub avg_utilization: f64,
}

/// Main device frequency manager
pub struct HolisticDevFreqMgr {
    devices: BTreeMap<u64, DevFreqProfile>,
    next_id: u64,
}

impl HolisticDevFreqMgr {
    pub fn new() -> Self { Self { devices: BTreeMap::new(), next_id: 1 } }

    pub fn register(&mut self, name: String, min: u64, max: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.devices.insert(id, DevFreqProfile::new(id, name, min, max));
        id
    }

    pub fn stats(&self) -> DevFreqMgrStats {
        let active = self.devices.values().filter(|d| d.power_state == DevPowerState::Active).count() as u32;
        let transitions: u64 = self.devices.values().map(|d| d.transitions).sum();
        let utils: Vec<f64> = self.devices.values().map(|d| d.utilization()).collect();
        let avg = if utils.is_empty() { 0.0 } else { utils.iter().sum::<f64>() / utils.len() as f64 };
        DevFreqMgrStats { total_devices: self.devices.len() as u32, active_devices: active, total_transitions: transitions, avg_utilization: avg }
    }
}
