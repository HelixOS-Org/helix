// SPDX-License-Identifier: GPL-2.0
//! Holistic thermal_mgr — thermal zone management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;

/// Thermal trip type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalTripType {
    Active,
    Passive,
    Hot,
    Critical,
}

/// Thermal trip point
#[derive(Debug)]
pub struct ThermalTrip {
    pub trip_type: ThermalTripType,
    pub temperature_mc: i32,
    pub hysteresis_mc: i32,
    pub triggered: bool,
}

/// Cooling device
#[derive(Debug)]
pub struct CoolingDevice {
    pub id: u64,
    pub name: String,
    pub max_state: u32,
    pub current_state: u32,
}

/// Thermal zone
#[derive(Debug)]
pub struct ThermalZone {
    pub id: u64,
    pub name: String,
    pub temperature_mc: i32,
    pub trips: [Option<ThermalTrip>; 4],
    pub cooling_devices: [Option<u64>; 4],
    pub readings: u64,
    pub max_temp_mc: i32,
    pub min_temp_mc: i32,
    pub throttle_count: u64,
}

impl ThermalZone {
    pub fn new(id: u64, name: String) -> Self {
        Self { id, name, temperature_mc: 25000, trips: [None, None, None, None], cooling_devices: [None, None, None, None], readings: 0, max_temp_mc: i32::MIN, min_temp_mc: i32::MAX, throttle_count: 0 }
    }

    #[inline]
    pub fn update_temp(&mut self, temp_mc: i32) {
        self.temperature_mc = temp_mc;
        if temp_mc > self.max_temp_mc { self.max_temp_mc = temp_mc; }
        if temp_mc < self.min_temp_mc { self.min_temp_mc = temp_mc; }
        self.readings += 1;
        for trip in self.trips.iter_mut().flatten() {
            trip.triggered = temp_mc >= trip.temperature_mc;
            if trip.triggered && trip.trip_type == ThermalTripType::Passive { self.throttle_count += 1; }
        }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ThermalMgrStats {
    pub total_zones: u32,
    pub total_cooling: u32,
    pub max_temp_mc: i32,
    pub avg_temp_mc: i32,
    pub total_throttles: u64,
}

/// Main thermal manager
pub struct HolisticThermalMgr {
    zones: BTreeMap<u64, ThermalZone>,
    cooling: BTreeMap<u64, CoolingDevice>,
    next_id: u64,
}

impl HolisticThermalMgr {
    pub fn new() -> Self { Self { zones: BTreeMap::new(), cooling: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn add_zone(&mut self, name: String) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.zones.insert(id, ThermalZone::new(id, name));
        id
    }

    #[inline(always)]
    pub fn update_zone(&mut self, id: u64, temp_mc: i32) {
        if let Some(z) = self.zones.get_mut(&id) { z.update_temp(temp_mc); }
    }

    #[inline]
    pub fn stats(&self) -> ThermalMgrStats {
        let max_t = self.zones.values().map(|z| z.temperature_mc).max().unwrap_or(0);
        let temps: Vec<i32> = self.zones.values().map(|z| z.temperature_mc).collect();
        let avg = if temps.is_empty() { 0 } else { temps.iter().sum::<i32>() / temps.len() as i32 };
        let throttles: u64 = self.zones.values().map(|z| z.throttle_count).sum();
        ThermalMgrStats { total_zones: self.zones.len() as u32, total_cooling: self.cooling.len() as u32, max_temp_mc: max_t, avg_temp_mc: avg, total_throttles: throttles }
    }
}
