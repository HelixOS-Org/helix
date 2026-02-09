// SPDX-License-Identifier: GPL-2.0
//! Holistic power_domain — power domain hierarchy and PM management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Power state (C-states / D-states)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PowerState {
    D0Active,
    D0Idle,
    D1Light,
    D2Deep,
    D3Hot,
    D3Cold,
    Off,
}

/// Power domain type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerDomainType {
    Cpu,
    CpuCluster,
    Gpu,
    Memory,
    Io,
    Pcie,
    Platform,
    Always,
}

/// Governor policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerGovernor {
    Performance,
    PowerSave,
    OnDemand,
    Conservative,
    Schedutil,
    UserSpace,
}

/// Power constraint
#[derive(Debug, Clone, Copy)]
pub struct PowerConstraint {
    pub max_power_mw: u32,
    pub min_state: PowerState,
    pub max_latency_us: u32,
}

/// Device in a power domain
#[derive(Debug, Clone)]
pub struct PowerDevice {
    pub id: u64,
    pub domain_id: u32,
    pub state: PowerState,
    pub power_mw: u32,
    pub transitions: u64,
    pub last_transition: u64,
    pub active_time_ns: u64,
    pub idle_time_ns: u64,
    pub resume_latency_us: u32,
}

impl PowerDevice {
    pub fn new(id: u64, domain: u32) -> Self {
        Self {
            id, domain_id: domain, state: PowerState::D0Active,
            power_mw: 0, transitions: 0, last_transition: 0,
            active_time_ns: 0, idle_time_ns: 0, resume_latency_us: 0,
        }
    }

    #[inline]
    pub fn transition(&mut self, new_state: PowerState, now: u64) {
        self.state = new_state;
        self.transitions += 1;
        self.last_transition = now;
    }

    #[inline]
    pub fn activity_ratio(&self) -> f64 {
        let total = self.active_time_ns + self.idle_time_ns;
        if total == 0 { return 0.0; }
        self.active_time_ns as f64 / total as f64
    }
}

/// Power domain
#[derive(Debug)]
pub struct PowerDomain {
    pub id: u32,
    pub parent_id: Option<u32>,
    pub domain_type: PowerDomainType,
    pub state: PowerState,
    pub governor: PowerGovernor,
    pub constraint: Option<PowerConstraint>,
    pub devices: Vec<u64>,
    pub children: Vec<u32>,
    pub total_power_mw: u32,
    pub on_transitions: u64,
    pub off_transitions: u64,
}

impl PowerDomain {
    pub fn new(id: u32, dtype: PowerDomainType) -> Self {
        Self {
            id, parent_id: None, domain_type: dtype,
            state: PowerState::D0Active, governor: PowerGovernor::OnDemand,
            constraint: None, devices: Vec::new(), children: Vec::new(),
            total_power_mw: 0, on_transitions: 0, off_transitions: 0,
        }
    }

    #[inline(always)]
    pub fn add_device(&mut self, dev_id: u64) { self.devices.push(dev_id); }
    #[inline(always)]
    pub fn add_child(&mut self, child_id: u32) { self.children.push(child_id); }

    #[inline(always)]
    pub fn can_power_down(&self) -> bool {
        self.devices.is_empty() || self.state >= PowerState::D1Light
    }

    #[inline(always)]
    pub fn power_on(&mut self) {
        self.state = PowerState::D0Active;
        self.on_transitions += 1;
    }

    #[inline(always)]
    pub fn power_off(&mut self) {
        self.state = PowerState::D3Cold;
        self.off_transitions += 1;
    }
}

/// Power domain stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PowerDomainStats {
    pub total_domains: u32,
    pub active_domains: u32,
    pub total_devices: u32,
    pub total_power_mw: u64,
    pub total_transitions: u64,
    pub deepest_sleep: PowerState,
}

/// Main power domain manager
pub struct HolisticPowerDomain {
    domains: BTreeMap<u32, PowerDomain>,
    devices: BTreeMap<u64, PowerDevice>,
    next_domain_id: u32,
}

impl HolisticPowerDomain {
    pub fn new() -> Self {
        Self { domains: BTreeMap::new(), devices: BTreeMap::new(), next_domain_id: 1 }
    }

    #[inline]
    pub fn create_domain(&mut self, dtype: PowerDomainType) -> u32 {
        let id = self.next_domain_id;
        self.next_domain_id += 1;
        self.domains.insert(id, PowerDomain::new(id, dtype));
        id
    }

    #[inline(always)]
    pub fn set_parent(&mut self, domain_id: u32, parent_id: u32) {
        if let Some(d) = self.domains.get_mut(&domain_id) { d.parent_id = Some(parent_id); }
        if let Some(p) = self.domains.get_mut(&parent_id) { p.add_child(domain_id); }
    }

    #[inline(always)]
    pub fn add_device(&mut self, dev_id: u64, domain_id: u32) {
        self.devices.insert(dev_id, PowerDevice::new(dev_id, domain_id));
        if let Some(d) = self.domains.get_mut(&domain_id) { d.add_device(dev_id); }
    }

    #[inline(always)]
    pub fn transition_device(&mut self, dev_id: u64, state: PowerState, now: u64) {
        if let Some(dev) = self.devices.get_mut(&dev_id) { dev.transition(state, now); }
    }

    #[inline]
    pub fn domain_power(&self, domain_id: u32) -> u64 {
        let domain = match self.domains.get(&domain_id) { Some(d) => d, None => return 0 };
        domain.devices.iter()
            .filter_map(|id| self.devices.get(id))
            .map(|d| d.power_mw as u64)
            .sum()
    }

    #[inline]
    pub fn stats(&self) -> PowerDomainStats {
        let active = self.domains.values().filter(|d| d.state <= PowerState::D0Idle).count() as u32;
        let total_power: u64 = self.devices.values().map(|d| d.power_mw as u64).sum();
        let total_trans: u64 = self.devices.values().map(|d| d.transitions).sum();
        let deepest = self.devices.values().map(|d| d.state).max().unwrap_or(PowerState::D0Active);
        PowerDomainStats {
            total_domains: self.domains.len() as u32, active_domains: active,
            total_devices: self.devices.len() as u32, total_power_mw: total_power,
            total_transitions: total_trans, deepest_sleep: deepest,
        }
    }
}
