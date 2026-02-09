//! Thermal management types and manager.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

use crate::core::NexusTimestamp;

// ============================================================================
// THERMAL ZONE
// ============================================================================

/// Thermal zone
#[derive(Debug, Clone)]
pub struct ThermalZone {
    /// Zone ID
    pub id: u32,
    /// Zone name
    pub name: String,
    /// Current temperature (Celsius * 1000)
    pub temperature: i32,
    /// Critical temperature
    pub critical_temp: i32,
    /// Hot temperature (throttling starts)
    pub hot_temp: i32,
    /// Passive cooling temp
    pub passive_temp: i32,
    /// Current cooling level (0-100)
    pub cooling_level: u8,
}

impl ThermalZone {
    /// Create new zone
    pub fn new(id: u32, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            temperature: 40_000,    // 40°C
            critical_temp: 100_000, // 100°C
            hot_temp: 85_000,       // 85°C
            passive_temp: 75_000,   // 75°C
            cooling_level: 0,
        }
    }

    /// Get temperature in Celsius
    #[inline(always)]
    pub fn temp_celsius(&self) -> f64 {
        self.temperature as f64 / 1000.0
    }

    /// Is in critical state?
    #[inline(always)]
    pub fn is_critical(&self) -> bool {
        self.temperature >= self.critical_temp
    }

    /// Is hot?
    #[inline(always)]
    pub fn is_hot(&self) -> bool {
        self.temperature >= self.hot_temp
    }

    /// Should throttle?
    #[inline(always)]
    pub fn should_throttle(&self) -> bool {
        self.temperature >= self.passive_temp
    }

    /// Calculate required throttle percentage
    pub fn throttle_percentage(&self) -> u8 {
        if self.temperature < self.passive_temp {
            return 0;
        }

        let range = (self.hot_temp - self.passive_temp) as f64;
        let excess = (self.temperature - self.passive_temp) as f64;
        let throttle = (excess / range * 50.0).min(100.0) as u8;

        if self.temperature >= self.hot_temp {
            100.min(throttle + 50)
        } else {
            throttle
        }
    }
}

// ============================================================================
// THERMAL MANAGER
// ============================================================================

/// Thermal manager
pub struct ThermalManager {
    /// Thermal zones
    zones: BTreeMap<u32, ThermalZone>,
    /// Temperature history
    history: VecDeque<(NexusTimestamp, i32)>, // (time, avg_temp)
    /// Max history entries
    max_history: usize,
    /// Current throttle level
    current_throttle: u8,
    /// Emergency shutdown threshold
    emergency_temp: i32,
}

impl ThermalManager {
    /// Create new thermal manager
    pub fn new() -> Self {
        Self {
            zones: BTreeMap::new(),
            history: VecDeque::new(),
            max_history: 1000,
            current_throttle: 0,
            emergency_temp: 105_000, // 105°C
        }
    }

    /// Add thermal zone
    #[inline(always)]
    pub fn add_zone(&mut self, zone: ThermalZone) {
        self.zones.insert(zone.id, zone);
    }

    /// Update zone temperature
    pub fn update_temperature(&mut self, zone_id: u32, temperature: i32) {
        if let Some(zone) = self.zones.get_mut(&zone_id) {
            zone.temperature = temperature;

            // Recalculate throttle
            self.recalculate_throttle();
        }

        // Record history
        let avg_temp = self.average_temperature();
        self.history.push_back((NexusTimestamp::now(), avg_temp));
        if self.history.len() > self.max_history {
            self.history.pop_front();
        }
    }

    /// Recalculate throttle level
    fn recalculate_throttle(&mut self) {
        // Use maximum throttle from all zones
        self.current_throttle = self
            .zones
            .values()
            .map(|z| z.throttle_percentage())
            .max()
            .unwrap_or(0);
    }

    /// Get average temperature
    #[inline]
    pub fn average_temperature(&self) -> i32 {
        if self.zones.is_empty() {
            return 0;
        }

        let sum: i32 = self.zones.values().map(|z| z.temperature).sum();
        sum / self.zones.len() as i32
    }

    /// Get maximum temperature
    #[inline]
    pub fn max_temperature(&self) -> i32 {
        self.zones
            .values()
            .map(|z| z.temperature)
            .max()
            .unwrap_or(0)
    }

    /// Get current throttle level
    #[inline(always)]
    pub fn throttle_level(&self) -> u8 {
        self.current_throttle
    }

    /// Need emergency shutdown?
    #[inline]
    pub fn needs_emergency_shutdown(&self) -> bool {
        self.zones
            .values()
            .any(|z| z.temperature >= self.emergency_temp)
    }

    /// Get hottest zone
    #[inline(always)]
    pub fn hottest_zone(&self) -> Option<&ThermalZone> {
        self.zones.values().max_by_key(|z| z.temperature)
    }

    /// Get temperature trend (°C per second)
    pub fn temperature_trend(&self) -> f64 {
        if self.history.len() < 10 {
            return 0.0;
        }

        let recent_start = self.history.len().saturating_sub(10);
        let recent: Vec<_> = self.history[recent_start..].to_vec();

        let first = &recent[0];
        let last = recent.last().unwrap();

        let time_diff = last.0.duration_since(first.0) as f64 / 1_000_000_000.0; // seconds
        if time_diff == 0.0 {
            return 0.0;
        }

        let temp_diff = (last.1 - first.1) as f64 / 1000.0; // Celsius
        temp_diff / time_diff
    }

    /// Predict temperature in N seconds
    #[inline]
    pub fn predict_temperature(&self, seconds: f64) -> i32 {
        let trend = self.temperature_trend();
        let current = self.average_temperature();
        (current as f64 + trend * 1000.0 * seconds) as i32
    }
}

impl Default for ThermalManager {
    fn default() -> Self {
        Self::new()
    }
}
