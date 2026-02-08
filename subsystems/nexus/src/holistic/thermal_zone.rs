// SPDX-License-Identifier: GPL-2.0
//! Holistic thermal_zone — thermal zone monitoring, throttling, and cooling.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Thermal zone type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalZoneKind {
    Cpu,
    Gpu,
    Memory,
    Nvme,
    Battery,
    Chassis,
    Acpi,
    Custom,
}

/// Trip point type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TripType {
    Active,
    Passive,
    Hot,
    Critical,
}

impl TripType {
    pub fn severity(&self) -> u8 {
        match self {
            Self::Active => 1,
            Self::Passive => 2,
            Self::Hot => 3,
            Self::Critical => 4,
        }
    }
}

/// A trip point
#[derive(Debug, Clone)]
pub struct TripPoint {
    pub id: u32,
    pub trip_type: TripType,
    pub temperature_mc: i32,
    pub hysteresis_mc: i32,
}

impl TripPoint {
    pub fn temp_celsius(&self) -> f64 {
        self.temperature_mc as f64 / 1000.0
    }

    pub fn is_exceeded(&self, current_mc: i32) -> bool {
        current_mc >= self.temperature_mc
    }

    pub fn is_cleared(&self, current_mc: i32) -> bool {
        current_mc < (self.temperature_mc - self.hysteresis_mc)
    }
}

/// Cooling device type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoolingType {
    Fan,
    Processor,
    MemoryBandwidth,
    DevFreq,
    Power,
}

/// A cooling device
#[derive(Debug, Clone)]
pub struct CoolingDevice {
    pub id: u32,
    pub name: String,
    pub cooling_type: CoolingType,
    pub max_state: u32,
    pub current_state: u32,
    pub bound_zones: Vec<u32>,
}

impl CoolingDevice {
    pub fn utilization(&self) -> f64 {
        if self.max_state == 0 { return 0.0; }
        self.current_state as f64 / self.max_state as f64
    }

    pub fn can_increase(&self) -> bool {
        self.current_state < self.max_state
    }

    pub fn can_decrease(&self) -> bool {
        self.current_state > 0
    }
}

/// Thermal governor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalGovernor {
    StepWise,
    FairShare,
    BangBang,
    PowerAllocator,
    UserSpace,
}

/// A thermal zone
#[derive(Debug)]
pub struct ThermalZone {
    pub id: u32,
    pub name: String,
    pub kind: ThermalZoneKind,
    pub temperature_mc: i32,
    pub trip_points: Vec<TripPoint>,
    pub cooling_devices: Vec<u32>,
    pub governor: ThermalGovernor,
    pub passive: bool,
    pub polling_interval_ms: u32,
    pub mode_enabled: bool,
    pub last_update: u64,
    pub temp_history: Vec<i32>,
    max_history: usize,
}

impl ThermalZone {
    pub fn new(id: u32, name: String, kind: ThermalZoneKind) -> Self {
        Self {
            id, name, kind,
            temperature_mc: 0,
            trip_points: Vec::new(),
            cooling_devices: Vec::new(),
            governor: ThermalGovernor::StepWise,
            passive: false,
            polling_interval_ms: 1000,
            mode_enabled: true,
            last_update: 0,
            temp_history: Vec::new(),
            max_history: 256,
        }
    }

    pub fn temp_celsius(&self) -> f64 {
        self.temperature_mc as f64 / 1000.0
    }

    pub fn update_temp(&mut self, temp_mc: i32, timestamp: u64) {
        self.temperature_mc = temp_mc;
        self.last_update = timestamp;
        if self.temp_history.len() >= self.max_history {
            self.temp_history.remove(0);
        }
        self.temp_history.push(temp_mc);
    }

    pub fn active_trips(&self) -> Vec<&TripPoint> {
        self.trip_points.iter()
            .filter(|tp| tp.is_exceeded(self.temperature_mc))
            .collect()
    }

    pub fn highest_trip(&self) -> Option<&TripPoint> {
        self.active_trips().into_iter()
            .max_by_key(|tp| tp.trip_type.severity())
    }

    pub fn is_critical(&self) -> bool {
        self.trip_points.iter().any(|tp| {
            tp.trip_type == TripType::Critical && tp.is_exceeded(self.temperature_mc)
        })
    }

    pub fn trend(&self) -> f64 {
        if self.temp_history.len() < 2 { return 0.0; }
        let n = self.temp_history.len();
        let recent = self.temp_history[n - 1] as f64;
        let prev = self.temp_history[n - 2] as f64;
        recent - prev
    }

    pub fn avg_temp_mc(&self) -> i32 {
        if self.temp_history.is_empty() { return self.temperature_mc; }
        let sum: i64 = self.temp_history.iter().map(|&t| t as i64).sum();
        (sum / self.temp_history.len() as i64) as i32
    }

    pub fn max_temp_mc(&self) -> i32 {
        self.temp_history.iter().copied().max().unwrap_or(self.temperature_mc)
    }
}

/// Thermal event
#[derive(Debug, Clone)]
pub struct ThermalEvent {
    pub zone_id: u32,
    pub trip_id: Option<u32>,
    pub temperature_mc: i32,
    pub event_type: ThermalEventType,
    pub timestamp: u64,
}

/// Thermal event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalEventType {
    TripCrossUp,
    TripCrossDown,
    ThrottleStart,
    ThrottleStop,
    CriticalReached,
    CoolingChanged,
}

/// Thermal stats
#[derive(Debug, Clone)]
pub struct ThermalStats {
    pub zone_count: u32,
    pub cooling_device_count: u32,
    pub total_throttle_events: u64,
    pub critical_events: u64,
    pub max_temp_seen_mc: i32,
    pub throttle_active_zones: u32,
}

/// Main thermal zone manager
pub struct HolisticThermalZone {
    zones: BTreeMap<u32, ThermalZone>,
    cooling_devices: BTreeMap<u32, CoolingDevice>,
    events: Vec<ThermalEvent>,
    max_events: usize,
    stats: ThermalStats,
}

impl HolisticThermalZone {
    pub fn new() -> Self {
        Self {
            zones: BTreeMap::new(),
            cooling_devices: BTreeMap::new(),
            events: Vec::new(),
            max_events: 2048,
            stats: ThermalStats {
                zone_count: 0, cooling_device_count: 0,
                total_throttle_events: 0, critical_events: 0,
                max_temp_seen_mc: 0, throttle_active_zones: 0,
            },
        }
    }

    pub fn add_zone(&mut self, zone: ThermalZone) {
        self.stats.zone_count += 1;
        self.zones.insert(zone.id, zone);
    }

    pub fn add_cooling_device(&mut self, dev: CoolingDevice) {
        self.stats.cooling_device_count += 1;
        self.cooling_devices.insert(dev.id, dev);
    }

    pub fn update_temperature(&mut self, zone_id: u32, temp_mc: i32, timestamp: u64) {
        if temp_mc > self.stats.max_temp_seen_mc {
            self.stats.max_temp_seen_mc = temp_mc;
        }
        if let Some(zone) = self.zones.get_mut(&zone_id) {
            zone.update_temp(temp_mc, timestamp);
        }
    }

    pub fn record_event(&mut self, event: ThermalEvent) {
        match event.event_type {
            ThermalEventType::ThrottleStart => self.stats.total_throttle_events += 1,
            ThermalEventType::CriticalReached => self.stats.critical_events += 1,
            _ => {}
        }
        if self.events.len() >= self.max_events {
            self.events.remove(0);
        }
        self.events.push(event);
    }

    pub fn set_cooling_state(&mut self, dev_id: u32, state: u32) {
        if let Some(dev) = self.cooling_devices.get_mut(&dev_id) {
            dev.current_state = state.min(dev.max_state);
        }
    }

    pub fn critical_zones(&self) -> Vec<u32> {
        self.zones.iter()
            .filter(|(_, z)| z.is_critical())
            .map(|(&id, _)| id)
            .collect()
    }

    pub fn hottest_zones(&self, n: usize) -> Vec<(u32, f64)> {
        let mut v: Vec<_> = self.zones.iter()
            .map(|(&id, z)| (id, z.temp_celsius()))
            .collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        v.truncate(n);
        v
    }

    pub fn zones_exceeding(&self, temp_mc: i32) -> Vec<u32> {
        self.zones.iter()
            .filter(|(_, z)| z.temperature_mc > temp_mc)
            .map(|(&id, _)| id)
            .collect()
    }

    pub fn cooling_utilization(&self) -> f64 {
        if self.cooling_devices.is_empty() { return 0.0; }
        let sum: f64 = self.cooling_devices.values().map(|d| d.utilization()).sum();
        sum / self.cooling_devices.len() as f64
    }

    pub fn stats(&self) -> &ThermalStats {
        &self.stats
    }
}
