//! Driver Matcher
//!
//! ML-like driver matching with learning capabilities.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::{DeviceId, DeviceInfo, DriverId, DriverInfo};

/// Match score
#[derive(Debug, Clone, Copy)]
pub struct MatchScore {
    /// Driver ID
    pub driver_id: DriverId,
    /// Score (0-100)
    pub score: u8,
    /// Match type
    pub match_type: MatchType,
}

/// Match types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchType {
    /// Exact vendor/device match
    Exact,
    /// Vendor match only
    Vendor,
    /// Class match
    Class,
    /// Generic/fallback match
    Generic,
}

impl MatchType {
    /// Get score for match type
    pub fn base_score(&self) -> u8 {
        match self {
            Self::Exact => 100,
            Self::Vendor => 75,
            Self::Class => 50,
            Self::Generic => 25,
        }
    }
}

/// Match history entry
#[derive(Debug, Clone)]
struct MatchHistoryEntry {
    device_id: DeviceId,
    driver_id: DriverId,
    vendor_id: u32,
    success: bool,
    timestamp: u64,
}

/// Driver matcher with ML-like scoring
pub struct DriverMatcher {
    /// Registered drivers
    drivers: BTreeMap<DriverId, DriverInfo>,
    /// Match history
    match_history: Vec<MatchHistoryEntry>,
    /// Maximum history
    max_history: usize,
    /// Learning rate for score adjustment
    learning_rate: f32,
    /// Driver score adjustments (based on success/failure)
    score_adjustments: BTreeMap<(DriverId, u32), f32>,
}

impl DriverMatcher {
    /// Create new driver matcher
    pub fn new() -> Self {
        Self {
            drivers: BTreeMap::new(),
            match_history: Vec::with_capacity(1000),
            max_history: 1000,
            learning_rate: 0.1,
            score_adjustments: BTreeMap::new(),
        }
    }

    /// Register driver
    pub fn register_driver(&mut self, driver: DriverInfo) {
        self.drivers.insert(driver.id, driver);
    }

    /// Unregister driver
    pub fn unregister_driver(&mut self, id: DriverId) {
        self.drivers.remove(&id);
    }

    /// Find matching drivers for device
    pub fn find_matches(&self, device: &DeviceInfo) -> Vec<MatchScore> {
        let mut matches: Vec<MatchScore> = self
            .drivers
            .values()
            .filter(|d| d.matches(device))
            .map(|d| self.calculate_score(d, device))
            .collect();

        // Sort by score descending
        matches.sort_by(|a, b| b.score.cmp(&a.score));
        matches
    }

    /// Calculate match score
    fn calculate_score(&self, driver: &DriverInfo, device: &DeviceInfo) -> MatchScore {
        let mut match_type = MatchType::Generic;
        let mut base_score = MatchType::Generic.base_score();

        // Determine match type
        if driver.vendor_ids.contains(&device.vendor_id)
            && driver.device_ids.contains(&device.device_id)
        {
            match_type = MatchType::Exact;
            base_score = MatchType::Exact.base_score();
        } else if driver.vendor_ids.contains(&device.vendor_id) {
            match_type = MatchType::Vendor;
            base_score = MatchType::Vendor.base_score();
        } else if !driver.class_codes.is_empty()
            && driver.class_codes.contains(&device.class_code)
        {
            match_type = MatchType::Class;
            base_score = MatchType::Class.base_score();
        }

        // Apply priority adjustment
        let priority_adj = (driver.priority as f32 - 50.0) / 10.0;

        // Apply learned adjustment
        let key = (driver.id, device.vendor_id);
        let learned_adj = self.score_adjustments.get(&key).copied().unwrap_or(0.0);

        // Calculate final score
        let mut score = base_score as f32 + priority_adj + learned_adj;
        score = score.clamp(0.0, 100.0);

        MatchScore {
            driver_id: driver.id,
            score: score as u8,
            match_type,
        }
    }

    /// Record match result (for learning)
    pub fn record_result(
        &mut self,
        device: &DeviceInfo,
        driver_id: DriverId,
        success: bool,
        timestamp: u64,
    ) {
        // Add to history
        let entry = MatchHistoryEntry {
            device_id: device.id,
            driver_id,
            vendor_id: device.vendor_id,
            success,
            timestamp,
        };

        if self.match_history.len() >= self.max_history {
            self.match_history.remove(0);
        }
        self.match_history.push(entry);

        // Update score adjustment
        let key = (driver_id, device.vendor_id);
        let current = self.score_adjustments.get(&key).copied().unwrap_or(0.0);
        let adjustment = if success {
            self.learning_rate
        } else {
            -self.learning_rate * 2.0
        };
        let new_value = (current + adjustment).clamp(-20.0, 20.0);
        self.score_adjustments.insert(key, new_value);

        // Update driver stats
        if let Some(driver) = self.drivers.get_mut(&driver_id) {
            if success {
                driver.bound_count += 1;
            } else {
                driver.failure_count += 1;
            }
        }
    }

    /// Get best matching driver
    pub fn best_match(&self, device: &DeviceInfo) -> Option<MatchScore> {
        self.find_matches(device).into_iter().next()
    }

    /// Get driver by ID
    pub fn get_driver(&self, id: DriverId) -> Option<&DriverInfo> {
        self.drivers.get(&id)
    }

    /// Get all drivers
    pub fn drivers(&self) -> impl Iterator<Item = &DriverInfo> {
        self.drivers.values()
    }

    /// Driver count
    pub fn driver_count(&self) -> usize {
        self.drivers.len()
    }
}

impl Default for DriverMatcher {
    fn default() -> Self {
        Self::new()
    }
}
