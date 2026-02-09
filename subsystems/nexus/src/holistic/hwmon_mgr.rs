// SPDX-License-Identifier: GPL-2.0
//! Holistic hwmon_mgr — hardware monitoring manager.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;

/// Sensor type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HwmonSensorType {
    Temperature,
    Voltage,
    FanSpeed,
    Power,
    Current,
    Humidity,
    Energy,
}

/// Sensor alarm state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorAlarm {
    Normal,
    Warning,
    Critical,
    Emergency,
}

/// Sensor reading
#[derive(Debug)]
pub struct HwmonSensor {
    pub id: u64,
    pub name: String,
    pub sensor_type: HwmonSensorType,
    pub value: i64,
    pub min_value: i64,
    pub max_value: i64,
    pub crit_low: i64,
    pub crit_high: i64,
    pub alarm: SensorAlarm,
    pub readings: u64,
    pub unit_scale: i32,
}

impl HwmonSensor {
    pub fn new(id: u64, name: String, st: HwmonSensorType) -> Self {
        Self { id, name, sensor_type: st, value: 0, min_value: i64::MAX, max_value: i64::MIN, crit_low: i64::MIN, crit_high: i64::MAX, alarm: SensorAlarm::Normal, readings: 0, unit_scale: 1000 }
    }

    #[inline]
    pub fn update(&mut self, value: i64) {
        self.value = value;
        if value < self.min_value { self.min_value = value; }
        if value > self.max_value { self.max_value = value; }
        self.readings += 1;
        self.alarm = if value >= self.crit_high { SensorAlarm::Emergency }
            else if value >= (self.crit_high * 90 / 100) { SensorAlarm::Critical }
            else if value >= (self.crit_high * 80 / 100) { SensorAlarm::Warning }
            else { SensorAlarm::Normal };
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct HwmonMgrStats {
    pub total_sensors: u32,
    pub alarms_warning: u32,
    pub alarms_critical: u32,
    pub alarms_emergency: u32,
    pub total_readings: u64,
}

/// Main hwmon manager
pub struct HolisticHwmonMgr {
    sensors: BTreeMap<u64, HwmonSensor>,
    next_id: u64,
}

impl HolisticHwmonMgr {
    pub fn new() -> Self { Self { sensors: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn register(&mut self, name: String, st: HwmonSensorType) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.sensors.insert(id, HwmonSensor::new(id, name, st));
        id
    }

    #[inline(always)]
    pub fn update_sensor(&mut self, id: u64, value: i64) {
        if let Some(s) = self.sensors.get_mut(&id) { s.update(value); }
    }

    #[inline]
    pub fn stats(&self) -> HwmonMgrStats {
        let warn = self.sensors.values().filter(|s| s.alarm == SensorAlarm::Warning).count() as u32;
        let crit = self.sensors.values().filter(|s| s.alarm == SensorAlarm::Critical).count() as u32;
        let emrg = self.sensors.values().filter(|s| s.alarm == SensorAlarm::Emergency).count() as u32;
        let readings: u64 = self.sensors.values().map(|s| s.readings).sum();
        HwmonMgrStats { total_sensors: self.sensors.len() as u32, alarms_warning: warn, alarms_critical: crit, alarms_emergency: emrg, total_readings: readings }
    }
}
