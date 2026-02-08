//! # Thermal Management
//!
//! System-wide thermal monitoring and management:
//! - Thermal zone monitoring
//! - Thermal throttling policies
//! - Cooling device management
//! - Temperature-aware scheduling
//! - Thermal prediction
//! - Emergency shutdown triggers

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// THERMAL ZONES
// ============================================================================

/// Thermal zone type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThermalZoneType {
    /// CPU package
    CpuPackage,
    /// Individual CPU core
    CpuCore,
    /// GPU
    Gpu,
    /// Chipset/PCH
    Chipset,
    /// Memory (DIMM)
    Memory,
    /// NVMe SSD
    Storage,
    /// Network adapter
    Network,
    /// Battery
    Battery,
    /// Ambient
    Ambient,
    /// Custom
    Custom,
}

/// Thermal trip point type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TripType {
    /// Passive cooling (throttle)
    Passive,
    /// Active cooling (fan)
    Active,
    /// Hot (aggressive throttle)
    Hot,
    /// Critical (emergency shutdown)
    Critical,
}

/// A thermal trip point
#[derive(Debug, Clone, Copy)]
pub struct TripPoint {
    /// Trip type
    pub trip_type: TripType,
    /// Temperature threshold (millidegrees Celsius)
    pub temperature: i32,
    /// Hysteresis (millidegrees)
    pub hysteresis: i32,
}

/// Thermal zone
#[derive(Debug, Clone)]
pub struct ThermalZone {
    /// Zone ID
    pub id: u32,
    /// Zone type
    pub zone_type: ThermalZoneType,
    /// Current temperature (millidegrees Celsius)
    pub temperature: i32,
    /// Trip points (sorted by temperature)
    pub trip_points: Vec<TripPoint>,
    /// Temperature trend (millidegrees per second)
    pub trend: i32,
    /// Last update time
    pub last_update: u64,
    /// Temperature history
    history: Vec<i32>,
    /// Max history
    max_history: usize,
    /// Is throttling active?
    pub throttling: bool,
}

impl ThermalZone {
    pub fn new(id: u32, zone_type: ThermalZoneType) -> Self {
        let mut trips = Vec::new();

        // Default trip points based on zone type
        let (passive, hot, critical) = match zone_type {
            ThermalZoneType::CpuPackage | ThermalZoneType::CpuCore => (80000, 95000, 105000),
            ThermalZoneType::Gpu => (85000, 100000, 110000),
            ThermalZoneType::Memory => (75000, 85000, 95000),
            ThermalZoneType::Storage => (65000, 75000, 85000),
            _ => (70000, 85000, 100000),
        };

        trips.push(TripPoint {
            trip_type: TripType::Passive,
            temperature: passive,
            hysteresis: 3000,
        });
        trips.push(TripPoint {
            trip_type: TripType::Hot,
            temperature: hot,
            hysteresis: 5000,
        });
        trips.push(TripPoint {
            trip_type: TripType::Critical,
            temperature: critical,
            hysteresis: 0,
        });

        Self {
            id,
            zone_type,
            temperature: 40000, // 40°C default
            trip_points: trips,
            trend: 0,
            last_update: 0,
            history: Vec::new(),
            max_history: 60,
            throttling: false,
        }
    }

    /// Update temperature
    pub fn update(&mut self, temperature: i32, timestamp: u64) {
        let old_temp = self.temperature;
        self.temperature = temperature;

        // Calculate trend
        let elapsed = timestamp.saturating_sub(self.last_update);
        if elapsed > 0 {
            self.trend = ((temperature - old_temp) as i64 * 1000 / elapsed as i64) as i32;
        }

        self.last_update = timestamp;

        // Record history
        self.history.push(temperature);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    /// Check which trip points are exceeded
    pub fn exceeded_trips(&self) -> Vec<&TripPoint> {
        self.trip_points
            .iter()
            .filter(|t| self.temperature >= t.temperature)
            .collect()
    }

    /// Highest exceeded trip type
    pub fn highest_trip(&self) -> Option<TripType> {
        self.exceeded_trips()
            .iter()
            .map(|t| t.trip_type)
            .max()
    }

    /// Temperature in degrees Celsius (float)
    pub fn temp_celsius(&self) -> f64 {
        self.temperature as f64 / 1000.0
    }

    /// Average temperature
    pub fn avg_temperature(&self) -> f64 {
        if self.history.is_empty() {
            return self.temperature as f64;
        }
        let sum: i64 = self.history.iter().map(|&t| t as i64).sum();
        sum as f64 / self.history.len() as f64
    }

    /// Predict temperature at future time (milliseconds)
    pub fn predict_temperature(&self, ms_ahead: u64) -> i32 {
        self.temperature + (self.trend as i64 * ms_ahead as i64 / 1000) as i32
    }
}

// ============================================================================
// COOLING DEVICES
// ============================================================================

/// Cooling device type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoolingType {
    /// Fan
    Fan,
    /// Processor frequency reduction
    CpuFreqReduction,
    /// GPU frequency reduction
    GpuFreqReduction,
    /// Power limit reduction
    PowerLimit,
    /// Passive (no active cooling)
    Passive,
}

/// Cooling device
#[derive(Debug, Clone)]
pub struct CoolingDevice {
    /// Device ID
    pub id: u32,
    /// Type
    pub cooling_type: CoolingType,
    /// Current state (0 = off, max = full cooling)
    pub current_state: u32,
    /// Max state
    pub max_state: u32,
    /// Associated thermal zone
    pub zone_id: u32,
}

impl CoolingDevice {
    pub fn new(id: u32, cooling_type: CoolingType, max_state: u32, zone_id: u32) -> Self {
        Self {
            id,
            cooling_type,
            current_state: 0,
            max_state,
            zone_id,
        }
    }

    /// Set cooling level (0.0 - 1.0)
    pub fn set_level(&mut self, level: f64) {
        let level = if level < 0.0 {
            0.0
        } else if level > 1.0 {
            1.0
        } else {
            level
        };
        self.current_state = (level * self.max_state as f64) as u32;
    }

    /// Current level (0.0 - 1.0)
    pub fn level(&self) -> f64 {
        if self.max_state == 0 {
            return 0.0;
        }
        self.current_state as f64 / self.max_state as f64
    }
}

// ============================================================================
// THROTTLE POLICY
// ============================================================================

/// Throttle level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThrottleLevel {
    /// No throttling
    None,
    /// Light throttling (10% reduction)
    Light,
    /// Moderate throttling (30% reduction)
    Moderate,
    /// Heavy throttling (50% reduction)
    Heavy,
    /// Emergency throttling (80% reduction)
    Emergency,
}

impl ThrottleLevel {
    /// Performance multiplier (basis points)
    pub fn performance_factor(&self) -> u32 {
        match self {
            Self::None => 10000,
            Self::Light => 9000,
            Self::Moderate => 7000,
            Self::Heavy => 5000,
            Self::Emergency => 2000,
        }
    }

    pub fn from_trip(trip: TripType) -> Self {
        match trip {
            TripType::Passive => Self::Light,
            TripType::Active => Self::Moderate,
            TripType::Hot => Self::Heavy,
            TripType::Critical => Self::Emergency,
        }
    }
}

// ============================================================================
// THERMAL MANAGER
// ============================================================================

/// Thermal event
#[derive(Debug, Clone)]
pub struct ThermalEvent {
    /// Zone ID
    pub zone_id: u32,
    /// Trip type exceeded
    pub trip_type: TripType,
    /// Temperature
    pub temperature: i32,
    /// Timestamp
    pub timestamp: u64,
}

/// System-wide thermal manager
pub struct ThermalManager {
    /// Thermal zones
    zones: BTreeMap<u32, ThermalZone>,
    /// Cooling devices
    cooling_devices: BTreeMap<u32, CoolingDevice>,
    /// Current system throttle level
    pub throttle_level: ThrottleLevel,
    /// Thermal events
    events: Vec<ThermalEvent>,
    /// Max events
    max_events: usize,
    /// Total thermal events
    pub total_events: u64,
    /// Emergency shutdown triggered?
    pub emergency_shutdown: bool,
}

impl ThermalManager {
    pub fn new() -> Self {
        Self {
            zones: BTreeMap::new(),
            cooling_devices: BTreeMap::new(),
            throttle_level: ThrottleLevel::None,
            events: Vec::new(),
            max_events: 100,
            total_events: 0,
            emergency_shutdown: false,
        }
    }

    /// Add thermal zone
    pub fn add_zone(&mut self, zone: ThermalZone) {
        self.zones.insert(zone.id, zone);
    }

    /// Add cooling device
    pub fn add_cooling_device(&mut self, device: CoolingDevice) {
        self.cooling_devices.insert(device.id, device);
    }

    /// Update zone temperature
    pub fn update_temperature(&mut self, zone_id: u32, temperature: i32, timestamp: u64) {
        if let Some(zone) = self.zones.get_mut(&zone_id) {
            zone.update(temperature, timestamp);
        }
    }

    /// Evaluate thermal state and adjust cooling
    pub fn evaluate(&mut self, timestamp: u64) -> ThrottleLevel {
        let mut worst_trip: Option<TripType> = None;

        // Check all zones
        let zone_trips: Vec<(u32, i32, Option<TripType>)> = self
            .zones
            .values()
            .map(|z| (z.id, z.temperature, z.highest_trip()))
            .collect();

        for (zone_id, temp, trip) in zone_trips {
            if let Some(trip_type) = trip {
                // Record event
                self.events.push(ThermalEvent {
                    zone_id,
                    trip_type,
                    temperature: temp,
                    timestamp,
                });
                if self.events.len() > self.max_events {
                    self.events.remove(0);
                }
                self.total_events += 1;

                // Track worst
                if worst_trip.map_or(true, |w| trip_type > w) {
                    worst_trip = Some(trip_type);
                }

                // Check critical
                if trip_type == TripType::Critical {
                    self.emergency_shutdown = true;
                }

                // Activate cooling
                self.activate_cooling(zone_id, trip_type);
            }
        }

        // Determine system throttle level
        self.throttle_level = worst_trip
            .map_or(ThrottleLevel::None, ThrottleLevel::from_trip);

        // Update zone throttling state
        for zone in self.zones.values_mut() {
            zone.throttling = zone.highest_trip().is_some();
        }

        self.throttle_level
    }

    /// Activate cooling for a zone
    fn activate_cooling(&mut self, zone_id: u32, trip_type: TripType) {
        let level = match trip_type {
            TripType::Passive => 0.3,
            TripType::Active => 0.6,
            TripType::Hot => 0.9,
            TripType::Critical => 1.0,
        };

        for device in self.cooling_devices.values_mut() {
            if device.zone_id == zone_id {
                device.set_level(level);
            }
        }
    }

    /// Get zone
    pub fn get_zone(&self, id: u32) -> Option<&ThermalZone> {
        self.zones.get(&id)
    }

    /// Max temperature across all zones (millidegrees)
    pub fn max_temperature(&self) -> i32 {
        self.zones.values().map(|z| z.temperature).max().unwrap_or(0)
    }

    /// Zone count
    pub fn zone_count(&self) -> usize {
        self.zones.len()
    }

    /// Cooling device count
    pub fn cooling_device_count(&self) -> usize {
        self.cooling_devices.len()
    }
}

// ============================================================================
// Merged from thermal_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThermalZoneType {
    Cpu,
    Gpu,
    Memory,
    Ssd,
    Battery,
    Skin,
    Ambient,
}

/// Trip point type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TripPointType {
    /// Active cooling (fan ramp-up)
    Active,
    /// Passive cooling (throttling)
    Passive,
    /// Hot (aggressive throttling)
    Hot,
    /// Critical (emergency shutdown)
    Critical,
}

/// Cooling device type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoolingType {
    Fan,
    CpuFreqThrottle,
    GpuFreqThrottle,
    PowerCap,
    DeviceThrottle,
}

/// Thermal zone
#[derive(Debug, Clone)]
pub struct ThermalZone {
    pub zone_id: u32,
    pub zone_type: ThermalZoneType,
    pub current_temp_mc: i32,  // milli-Celsius
    pub trip_active_mc: i32,
    pub trip_passive_mc: i32,
    pub trip_hot_mc: i32,
    pub trip_critical_mc: i32,
    pub temp_history: Vec<i32>,
    pub max_history: usize,
    pub slope_mc_per_sec: f64,
    pub power_mw: u32,
}

impl ThermalZone {
    pub fn new(zone_id: u32, zone_type: ThermalZoneType) -> Self {
        Self {
            zone_id,
            zone_type,
            current_temp_mc: 25_000,
            trip_active_mc: 60_000,
            trip_passive_mc: 80_000,
            trip_hot_mc: 95_000,
            trip_critical_mc: 105_000,
            temp_history: Vec::new(),
            max_history: 64,
            slope_mc_per_sec: 0.0,
            power_mw: 0,
        }
    }

    pub fn update_temp(&mut self, temp_mc: i32) {
        let old = self.current_temp_mc;
        self.current_temp_mc = temp_mc;

        if self.temp_history.len() >= self.max_history {
            self.temp_history.remove(0);
        }
        self.temp_history.push(temp_mc);

        // Compute slope from last few samples
        if self.temp_history.len() >= 2 {
            let n = self.temp_history.len();
            let recent = &self.temp_history[n.saturating_sub(4)..n];
            if recent.len() >= 2 {
                let delta = recent[recent.len() - 1] as f64 - recent[0] as f64;
                self.slope_mc_per_sec = delta / recent.len() as f64;
            }
        }
    }

    /// Current trip state
    pub fn current_trip(&self) -> Option<TripPointType> {
        if self.current_temp_mc >= self.trip_critical_mc {
            Some(TripPointType::Critical)
        } else if self.current_temp_mc >= self.trip_hot_mc {
            Some(TripPointType::Hot)
        } else if self.current_temp_mc >= self.trip_passive_mc {
            Some(TripPointType::Passive)
        } else if self.current_temp_mc >= self.trip_active_mc {
            Some(TripPointType::Active)
        } else {
            None
        }
    }

    /// Predicted time to reach trip point (seconds, <0 if cooling)
    pub fn time_to_trip(&self, trip_mc: i32) -> f64 {
        if self.slope_mc_per_sec <= 0.0 { return f64::MAX; }
        let delta = trip_mc - self.current_temp_mc;
        if delta <= 0 { return 0.0; }
        delta as f64 / self.slope_mc_per_sec
    }

    /// Temperature as Celsius
    pub fn temp_c(&self) -> f64 {
        self.current_temp_mc as f64 / 1000.0
    }

    /// Headroom to passive (mC)
    pub fn headroom_passive_mc(&self) -> i32 {
        self.trip_passive_mc - self.current_temp_mc
    }
}

/// Cooling device
#[derive(Debug, Clone)]
pub struct CoolingDevice {
    pub device_id: u32,
    pub cooling_type: CoolingType,
    pub max_state: u32,
    pub current_state: u32,
    pub linked_zones: Vec<u32>,
    pub effectiveness: f64, // mC reduction per state level
}

impl CoolingDevice {
    pub fn new(device_id: u32, cooling_type: CoolingType, max_state: u32) -> Self {
        Self {
            device_id,
            cooling_type,
            max_state,
            current_state: 0,
            linked_zones: Vec::new(),
            effectiveness: 500.0, // 500mC per state
        }
    }

    pub fn utilization(&self) -> f64 {
        if self.max_state == 0 { return 0.0; }
        self.current_state as f64 / self.max_state as f64
    }

    pub fn set_state(&mut self, state: u32) {
        self.current_state = state.min(self.max_state);
    }

    pub fn can_increase(&self) -> bool {
        self.current_state < self.max_state
    }
}

/// Thermal budget allocation
#[derive(Debug, Clone)]
pub struct ThermalBudget {
    pub total_power_mw: u32,
    pub allocations: BTreeMap<u32, u32>, // zone_id → power_mw
    pub remaining_mw: u32,
}

impl ThermalBudget {
    pub fn new(total_mw: u32) -> Self {
        Self {
            total_power_mw: total_mw,
            allocations: BTreeMap::new(),
            remaining_mw: total_mw,
        }
    }

    pub fn allocate(&mut self, zone_id: u32, power_mw: u32) -> u32 {
        let granted = power_mw.min(self.remaining_mw);
        if granted > 0 {
            *self.allocations.entry(zone_id).or_insert(0) += granted;
            self.remaining_mw -= granted;
        }
        granted
    }

    pub fn release(&mut self, zone_id: u32, power_mw: u32) {
        if let Some(alloc) = self.allocations.get_mut(&zone_id) {
            let released = power_mw.min(*alloc);
            *alloc -= released;
            self.remaining_mw += released;
        }
    }
}

/// Cooling action recommendation
#[derive(Debug, Clone)]
pub struct CoolingAction {
    pub device_id: u32,
    pub target_state: u32,
    pub reason_zone: u32,
    pub urgency: f64,
}

/// Thermal V2 stats
#[derive(Debug, Clone, Default)]
pub struct HolisticThermalV2Stats {
    pub thermal_zones: usize,
    pub cooling_devices: usize,
    pub max_temp_mc: i32,
    pub zones_at_active: usize,
    pub zones_at_passive: usize,
    pub zones_at_hot: usize,
    pub zones_at_critical: usize,
    pub total_power_mw: u32,
    pub budget_remaining_mw: u32,
    pub cooling_actions_pending: usize,
}

/// Holistic Thermal V2 Engine
pub struct HolisticThermalV2 {
    zones: BTreeMap<u32, ThermalZone>,
    cooling_devices: BTreeMap<u32, CoolingDevice>,
    budget: ThermalBudget,
    cooling_actions: Vec<CoolingAction>,
    stats: HolisticThermalV2Stats,
}

impl HolisticThermalV2 {
    pub fn new(total_power_budget_mw: u32) -> Self {
        Self {
            zones: BTreeMap::new(),
            cooling_devices: BTreeMap::new(),
            budget: ThermalBudget::new(total_power_budget_mw),
            cooling_actions: Vec::new(),
            stats: HolisticThermalV2Stats::default(),
        }
    }

    pub fn add_zone(&mut self, zone: ThermalZone) {
        self.zones.insert(zone.zone_id, zone);
        self.recompute();
    }

    pub fn add_cooling_device(&mut self, device: CoolingDevice) {
        self.cooling_devices.insert(device.device_id, device);
        self.recompute();
    }

    pub fn update_temperature(&mut self, zone_id: u32, temp_mc: i32) {
        if let Some(zone) = self.zones.get_mut(&zone_id) {
            zone.update_temp(temp_mc);
        }
    }

    pub fn update_power(&mut self, zone_id: u32, power_mw: u32) {
        if let Some(zone) = self.zones.get_mut(&zone_id) {
            zone.power_mw = power_mw;
        }
    }

    /// Evaluate thermal state and generate cooling actions
    pub fn evaluate(&mut self) {
        self.cooling_actions.clear();

        for zone in self.zones.values() {
            let trip = zone.current_trip();
            if trip.is_none() { continue; }

            let urgency = match trip.unwrap() {
                TripPointType::Active => 0.3,
                TripPointType::Passive => 0.6,
                TripPointType::Hot => 0.9,
                TripPointType::Critical => 1.0,
            };

            // Find cooling devices linked to this zone
            for device in self.cooling_devices.values() {
                if device.linked_zones.contains(&zone.zone_id) && device.can_increase() {
                    // Scale target state by urgency
                    let target = ((device.max_state as f64 * urgency) as u32).max(device.current_state + 1);
                    self.cooling_actions.push(CoolingAction {
                        device_id: device.device_id,
                        target_state: target.min(device.max_state),
                        reason_zone: zone.zone_id,
                        urgency,
                    });
                }
            }
        }

        // Sort by urgency
        self.cooling_actions.sort_by(|a, b|
            b.urgency.partial_cmp(&a.urgency).unwrap_or(core::cmp::Ordering::Equal));

        self.recompute();
    }

    /// Apply a cooling action
    pub fn apply_action(&mut self, idx: usize) -> bool {
        if idx >= self.cooling_actions.len() { return false; }
        let action = self.cooling_actions[idx].clone();
        if let Some(device) = self.cooling_devices.get_mut(&action.device_id) {
            device.set_state(action.target_state);
            true
        } else { false }
    }

    /// Predict skin temperature from zone temps
    pub fn estimate_skin_temp_mc(&self) -> i32 {
        // Weighted average: CPU 40%, GPU 30%, Memory 20%, Ambient 10%
        let mut weighted_sum = 0.0f64;
        let mut weight_total = 0.0f64;

        for zone in self.zones.values() {
            let w = match zone.zone_type {
                ThermalZoneType::Cpu => 0.4,
                ThermalZoneType::Gpu => 0.3,
                ThermalZoneType::Memory => 0.2,
                ThermalZoneType::Ambient => 0.1,
                ThermalZoneType::Skin => return zone.current_temp_mc,
                _ => 0.05,
            };
            weighted_sum += zone.current_temp_mc as f64 * w;
            weight_total += w;
        }

        if weight_total < 0.01 { return 25_000; }
        (weighted_sum / weight_total) as i32
    }

    /// Get budget info
    pub fn budget(&self) -> &ThermalBudget {
        &self.budget
    }

    fn recompute(&mut self) {
        let max_temp = self.zones.values().map(|z| z.current_temp_mc).max().unwrap_or(0);
        let total_power: u32 = self.zones.values().map(|z| z.power_mw).sum();

        let mut at_active = 0;
        let mut at_passive = 0;
        let mut at_hot = 0;
        let mut at_critical = 0;

        for zone in self.zones.values() {
            match zone.current_trip() {
                Some(TripPointType::Active) => at_active += 1,
                Some(TripPointType::Passive) => at_passive += 1,
                Some(TripPointType::Hot) => at_hot += 1,
                Some(TripPointType::Critical) => at_critical += 1,
                None => {}
            }
        }

        self.stats = HolisticThermalV2Stats {
            thermal_zones: self.zones.len(),
            cooling_devices: self.cooling_devices.len(),
            max_temp_mc: max_temp,
            zones_at_active: at_active,
            zones_at_passive: at_passive,
            zones_at_hot: at_hot,
            zones_at_critical: at_critical,
            total_power_mw: total_power,
            budget_remaining_mw: self.budget.remaining_mw,
            cooling_actions_pending: self.cooling_actions.len(),
        };
    }

    pub fn stats(&self) -> &HolisticThermalV2Stats {
        &self.stats
    }

    pub fn zone(&self, zone_id: u32) -> Option<&ThermalZone> {
        self.zones.get(&zone_id)
    }

    pub fn cooling_actions(&self) -> &[CoolingAction] {
        &self.cooling_actions
    }
}
