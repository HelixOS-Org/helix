//! Thermal manager
//!
//! This module provides the central ThermalManager for coordinating
//! thermal zones, cooling devices, fans, and thermal events.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use super::cooling::CoolingDevice;
use super::event::{ThermalEvent, ThermalEventType};
use super::fan::FanInfo;
use super::types::{CoolingDeviceId, Temperature, ThermalZoneId};
use super::zone::ThermalZone;

/// Thermal manager
pub struct ThermalManager {
    /// Thermal zones
    pub(crate) zones: BTreeMap<ThermalZoneId, ThermalZone>,
    /// Cooling devices
    cooling_devices: BTreeMap<CoolingDeviceId, CoolingDevice>,
    /// Fans
    pub(crate) fans: BTreeMap<CoolingDeviceId, FanInfo>,
    /// Event history
    events: VecDeque<ThermalEvent>,
    /// Max events
    max_events: usize,
    /// Zone count
    zone_count: AtomicU32,
    /// Enabled
    enabled: AtomicBool,
}

impl ThermalManager {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            zones: BTreeMap::new(),
            cooling_devices: BTreeMap::new(),
            fans: BTreeMap::new(),
            events: VecDeque::new(),
            max_events: 1000,
            zone_count: AtomicU32::new(0),
            enabled: AtomicBool::new(true),
        }
    }

    /// Register zone
    #[inline(always)]
    pub fn register_zone(&mut self, zone: ThermalZone) {
        self.zone_count.fetch_add(1, Ordering::Relaxed);
        self.zones.insert(zone.id, zone);
    }

    /// Get zone
    #[inline(always)]
    pub fn get_zone(&self, id: ThermalZoneId) -> Option<&ThermalZone> {
        self.zones.get(&id)
    }

    /// Get zone mutably
    #[inline(always)]
    pub fn get_zone_mut(&mut self, id: ThermalZoneId) -> Option<&mut ThermalZone> {
        self.zones.get_mut(&id)
    }

    /// Register cooling device
    #[inline(always)]
    pub fn register_cooling_device(&mut self, device: CoolingDevice) {
        self.cooling_devices.insert(device.id, device);
    }

    /// Get cooling device
    #[inline(always)]
    pub fn get_cooling_device(&self, id: CoolingDeviceId) -> Option<&CoolingDevice> {
        self.cooling_devices.get(&id)
    }

    /// Register fan
    #[inline(always)]
    pub fn register_fan(&mut self, fan: FanInfo) {
        self.fans.insert(fan.cooling_device, fan);
    }

    /// Get fan
    #[inline(always)]
    pub fn get_fan(&self, id: CoolingDeviceId) -> Option<&FanInfo> {
        self.fans.get(&id)
    }

    /// Record event
    #[inline]
    pub fn record_event(&mut self, event: ThermalEvent) {
        if self.events.len() >= self.max_events {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    /// Get hottest zone
    #[inline(always)]
    pub fn hottest_zone(&self) -> Option<&ThermalZone> {
        self.zones.values().max_by_key(|z| z.temperature().0)
    }

    /// Get all zones above temperature
    #[inline]
    pub fn zones_above(&self, temp: Temperature) -> Vec<&ThermalZone> {
        self.zones
            .values()
            .filter(|z| z.temperature().0 > temp.0)
            .collect()
    }

    /// Get CPU zones
    #[inline]
    pub fn cpu_zones(&self) -> Vec<&ThermalZone> {
        self.zones
            .values()
            .filter(|z| z.zone_type.is_cpu())
            .collect()
    }

    /// Zone count
    #[inline(always)]
    pub fn zone_count(&self) -> u32 {
        self.zone_count.load(Ordering::Relaxed)
    }

    /// Is enabled
    #[inline(always)]
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Get all zones
    #[inline(always)]
    pub fn zones(&self) -> impl Iterator<Item = &ThermalZone> {
        self.zones.values()
    }

    /// Get events
    #[inline(always)]
    pub fn events(&self) -> &[ThermalEvent] {
        &self.events
    }

    /// Update zone temperature
    pub fn update_temperature(
        &mut self,
        zone_id: ThermalZoneId,
        temp: Temperature,
        timestamp: u64,
    ) {
        if let Some(zone) = self.zones.get_mut(&zone_id) {
            let old_temp = zone.temperature();
            zone.update_temperature(temp);

            // Check for trip point crossings
            for trip in &zone.trip_points {
                let was_triggered = trip.is_triggered(old_temp);
                let is_triggered = trip.is_triggered(temp);

                if !was_triggered && is_triggered {
                    let mut event = ThermalEvent::new(ThermalEventType::TripCrossed, timestamp);
                    event.zone = Some(zone_id);
                    event.temperature = Some(temp);
                    event.trip_index = Some(trip.index);
                    self.events.push_back(event);
                } else if was_triggered && trip.is_cleared(temp) {
                    let mut event = ThermalEvent::new(ThermalEventType::TripCleared, timestamp);
                    event.zone = Some(zone_id);
                    event.temperature = Some(temp);
                    event.trip_index = Some(trip.index);
                    self.events.push_back(event);
                }
            }
        }
    }
}

impl Default for ThermalManager {
    fn default() -> Self {
        Self::new()
    }
}
