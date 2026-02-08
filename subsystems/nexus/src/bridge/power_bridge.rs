//! # Bridge Power Bridge
//!
//! Bridges power management operations:
//! - Power domain management
//! - Device runtime PM state tracking
//! - System sleep state transitions
//! - Wakeup source management
//! - Power supply monitoring
//! - Energy consumption tracking

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// System sleep state
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SleepState {
    S0,
    S1,
    S2,
    S3,
    S4,
    S5,
}

/// Runtime PM state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePmState {
    Active,
    Suspended,
    Suspending,
    Resuming,
    Error,
}

/// Power domain state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainState {
    On,
    Off,
    Retention,
    Standby,
}

/// Power domain
#[derive(Debug, Clone)]
pub struct PowerDomain {
    pub id: u64,
    pub name: String,
    pub state: DomainState,
    pub parent_id: Option<u64>,
    pub children: Vec<u64>,
    pub device_ids: Vec<u64>,
    pub power_on_latency_us: u64,
    pub power_off_latency_us: u64,
    pub usage_count: u32,
    pub on_count: u64,
    pub off_count: u64,
}

impl PowerDomain {
    pub fn new(id: u64, name: String) -> Self {
        Self {
            id, name, state: DomainState::On, parent_id: None,
            children: Vec::new(), device_ids: Vec::new(),
            power_on_latency_us: 100, power_off_latency_us: 50,
            usage_count: 0, on_count: 0, off_count: 0,
        }
    }

    pub fn power_on(&mut self) { self.state = DomainState::On; self.on_count += 1; }
    pub fn power_off(&mut self) -> bool {
        if self.usage_count > 0 { return false; }
        self.state = DomainState::Off; self.off_count += 1; true
    }
    pub fn acquire(&mut self) { self.usage_count += 1; if self.state != DomainState::On { self.power_on(); } }
    pub fn release(&mut self) { self.usage_count = self.usage_count.saturating_sub(1); }
}

/// Device power state
#[derive(Debug, Clone)]
pub struct DevicePower {
    pub device_id: u64,
    pub name: String,
    pub rpm_state: RuntimePmState,
    pub domain_id: Option<u64>,
    pub usage_count: u32,
    pub child_count: u32,
    pub disable_depth: u32,
    pub runtime_auto: bool,
    pub idle_timeout_ms: u64,
    pub last_busy_ts: u64,
    pub suspend_count: u64,
    pub resume_count: u64,
    pub active_time_us: u64,
    pub suspended_time_us: u64,
    pub wakeup_capable: bool,
    pub wakeup_enabled: bool,
    pub wakeup_count: u64,
}

impl DevicePower {
    pub fn new(id: u64, name: String) -> Self {
        Self {
            device_id: id, name, rpm_state: RuntimePmState::Active,
            domain_id: None, usage_count: 0, child_count: 0,
            disable_depth: 0, runtime_auto: false, idle_timeout_ms: 5000,
            last_busy_ts: 0, suspend_count: 0, resume_count: 0,
            active_time_us: 0, suspended_time_us: 0,
            wakeup_capable: false, wakeup_enabled: false, wakeup_count: 0,
        }
    }

    pub fn mark_busy(&mut self, ts: u64) { self.last_busy_ts = ts; self.rpm_state = RuntimePmState::Active; }

    pub fn suspend(&mut self) -> bool {
        if self.usage_count > 0 || self.disable_depth > 0 { return false; }
        self.rpm_state = RuntimePmState::Suspended; self.suspend_count += 1; true
    }

    pub fn resume(&mut self) { self.rpm_state = RuntimePmState::Active; self.resume_count += 1; }
    pub fn get_ref(&mut self) { self.usage_count += 1; if self.rpm_state == RuntimePmState::Suspended { self.resume(); } }
    pub fn put_ref(&mut self) { self.usage_count = self.usage_count.saturating_sub(1); }
    pub fn is_idle(&self, now: u64) -> bool { self.usage_count == 0 && now - self.last_busy_ts > self.idle_timeout_ms * 1000 }
}

/// Wakeup source
#[derive(Debug, Clone)]
pub struct WakeupSource {
    pub id: u64,
    pub name: String,
    pub active: bool,
    pub event_count: u64,
    pub wakeup_count: u64,
    pub expire_count: u64,
    pub active_since: u64,
    pub total_time_us: u64,
    pub max_time_us: u64,
    pub last_time_us: u64,
    pub prevent_sleep: bool,
}

impl WakeupSource {
    pub fn new(id: u64, name: String) -> Self {
        Self {
            id, name, active: false, event_count: 0, wakeup_count: 0,
            expire_count: 0, active_since: 0, total_time_us: 0,
            max_time_us: 0, last_time_us: 0, prevent_sleep: false,
        }
    }

    pub fn activate(&mut self, ts: u64) { self.active = true; self.active_since = ts; self.event_count += 1; }

    pub fn deactivate(&mut self, ts: u64) {
        if self.active {
            let dur = ts.saturating_sub(self.active_since);
            self.total_time_us += dur;
            self.last_time_us = dur;
            if dur > self.max_time_us { self.max_time_us = dur; }
        }
        self.active = false;
    }
}

/// Power bridge stats
#[derive(Debug, Clone, Default)]
pub struct PowerBridgeStats {
    pub total_domains: usize,
    pub domains_off: usize,
    pub total_devices: usize,
    pub suspended_devices: usize,
    pub total_wakeup_sources: usize,
    pub active_wakeup_sources: usize,
    pub total_suspends: u64,
    pub total_resumes: u64,
}

/// Bridge power manager
pub struct BridgePowerBridge {
    domains: BTreeMap<u64, PowerDomain>,
    devices: BTreeMap<u64, DevicePower>,
    wakeup_sources: BTreeMap<u64, WakeupSource>,
    system_state: SleepState,
    stats: PowerBridgeStats,
    next_id: u64,
}

impl BridgePowerBridge {
    pub fn new() -> Self {
        Self { domains: BTreeMap::new(), devices: BTreeMap::new(), wakeup_sources: BTreeMap::new(), system_state: SleepState::S0, stats: PowerBridgeStats::default(), next_id: 1 }
    }

    pub fn create_domain(&mut self, name: String) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.domains.insert(id, PowerDomain::new(id, name));
        id
    }

    pub fn register_device(&mut self, name: String, domain: Option<u64>) -> u64 {
        let id = self.next_id; self.next_id += 1;
        let mut dev = DevicePower::new(id, name);
        dev.domain_id = domain;
        self.devices.insert(id, dev);
        if let Some(did) = domain { if let Some(d) = self.domains.get_mut(&did) { d.device_ids.push(id); d.acquire(); } }
        id
    }

    pub fn register_wakeup_source(&mut self, name: String) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.wakeup_sources.insert(id, WakeupSource::new(id, name));
        id
    }

    pub fn device_get(&mut self, id: u64) { if let Some(d) = self.devices.get_mut(&id) { d.get_ref(); } }
    pub fn device_put(&mut self, id: u64) { if let Some(d) = self.devices.get_mut(&id) { d.put_ref(); } }
    pub fn device_busy(&mut self, id: u64, ts: u64) { if let Some(d) = self.devices.get_mut(&id) { d.mark_busy(ts); } }

    pub fn suspend_device(&mut self, id: u64) -> bool { self.devices.get_mut(&id).map(|d| d.suspend()).unwrap_or(false) }
    pub fn resume_device(&mut self, id: u64) { if let Some(d) = self.devices.get_mut(&id) { d.resume(); } }

    pub fn activate_wakeup(&mut self, id: u64, ts: u64) { if let Some(w) = self.wakeup_sources.get_mut(&id) { w.activate(ts); } }
    pub fn deactivate_wakeup(&mut self, id: u64, ts: u64) { if let Some(w) = self.wakeup_sources.get_mut(&id) { w.deactivate(ts); } }

    pub fn system_sleep(&self) -> SleepState { self.system_state }
    pub fn set_sleep_state(&mut self, s: SleepState) { self.system_state = s; }

    pub fn auto_suspend_idle(&mut self, now: u64) -> Vec<u64> {
        let idle: Vec<u64> = self.devices.values().filter(|d| d.runtime_auto && d.is_idle(now) && d.rpm_state == RuntimePmState::Active).map(|d| d.device_id).collect();
        for &id in &idle { self.suspend_device(id); }
        idle
    }

    pub fn recompute(&mut self) {
        self.stats.total_domains = self.domains.len();
        self.stats.domains_off = self.domains.values().filter(|d| d.state == DomainState::Off).count();
        self.stats.total_devices = self.devices.len();
        self.stats.suspended_devices = self.devices.values().filter(|d| d.rpm_state == RuntimePmState::Suspended).count();
        self.stats.total_wakeup_sources = self.wakeup_sources.len();
        self.stats.active_wakeup_sources = self.wakeup_sources.values().filter(|w| w.active).count();
        self.stats.total_suspends = self.devices.values().map(|d| d.suspend_count).sum();
        self.stats.total_resumes = self.devices.values().map(|d| d.resume_count).sum();
    }

    pub fn domain(&self, id: u64) -> Option<&PowerDomain> { self.domains.get(&id) }
    pub fn device(&self, id: u64) -> Option<&DevicePower> { self.devices.get(&id) }
    pub fn stats(&self) -> &PowerBridgeStats { &self.stats }
}
