//! Thermal intelligence coordinator
//!
//! This module provides the central ThermalIntelligence coordinator
//! for analyzing thermal subsystem health and providing recommendations.

#![allow(dead_code)]
#![allow(clippy::excessive_nesting)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use super::fan::FanMode;
use super::manager::ThermalManager;
use super::types::{Temperature, ThermalZoneId};
use super::zone::{ThermalZone, TripPointType};

/// Thermal analysis
#[derive(Debug, Clone)]
pub struct ThermalAnalysis {
    /// Health score (0-100)
    pub health_score: f32,
    /// Cooling effectiveness (0-100)
    pub cooling_score: f32,
    /// Issues
    pub issues: Vec<ThermalIssue>,
    /// Recommendations
    pub recommendations: Vec<ThermalRecommendation>,
}

/// Thermal issue
#[derive(Debug, Clone)]
pub struct ThermalIssue {
    /// Issue type
    pub issue_type: ThermalIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
    /// Zone
    pub zone: Option<ThermalZoneId>,
}

/// Thermal issue type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalIssueType {
    /// High temperature
    HighTemperature,
    /// Critical temperature
    CriticalTemperature,
    /// Thermal throttling
    Throttling,
    /// Rising temperature trend
    RisingTrend,
    /// Fan failure
    FanFailure,
    /// Cooling insufficient
    CoolingInsufficient,
    /// Trip point triggered
    TripTriggered,
}

/// Thermal recommendation
#[derive(Debug, Clone)]
pub struct ThermalRecommendation {
    /// Action
    pub action: ThermalAction,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
}

/// Thermal action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalAction {
    /// Increase fan speed
    IncreaseFanSpeed,
    /// Reduce workload
    ReduceWorkload,
    /// Clean cooling system
    CleanCooling,
    /// Check thermal paste
    CheckThermalPaste,
    /// Improve airflow
    ImproveAirflow,
    /// Enable aggressive throttling
    EnableThrottling,
}

/// Thermal Intelligence
pub struct ThermalIntelligence {
    /// Manager
    manager: ThermalManager,
}

impl ThermalIntelligence {
    /// Create new intelligence
    pub fn new() -> Self {
        Self {
            manager: ThermalManager::new(),
        }
    }

    /// Register zone
    pub fn register_zone(&mut self, zone: ThermalZone) {
        self.manager.register_zone(zone);
    }

    /// Update temperature
    pub fn update_temperature(
        &mut self,
        zone_id: ThermalZoneId,
        temp: Temperature,
        timestamp: u64,
    ) {
        self.manager.update_temperature(zone_id, temp, timestamp);
    }

    /// Analyze thermal subsystem
    pub fn analyze(&self) -> ThermalAnalysis {
        let mut health_score = 100.0f32;
        let mut cooling_score = 100.0f32;
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        for zone in self.manager.zones.values() {
            let temp = zone.temperature();
            let critical = zone
                .critical_temperature()
                .unwrap_or(Temperature::from_celsius(100.0));

            // Check critical temperature proximity
            let critical_proximity = (temp.0 as f32 / critical.0 as f32) * 100.0;

            if critical_proximity > 95.0 {
                health_score -= 50.0;
                issues.push(ThermalIssue {
                    issue_type: ThermalIssueType::CriticalTemperature,
                    severity: 10,
                    description: alloc::format!(
                        "Zone {} near critical: {} (critical: {})",
                        zone.name,
                        temp.to_string(),
                        critical.to_string()
                    ),
                    zone: Some(zone.id),
                });
                recommendations.push(ThermalRecommendation {
                    action: ThermalAction::ReduceWorkload,
                    expected_improvement: 30.0,
                    reason: String::from("Immediately reduce workload to prevent shutdown"),
                });
            } else if critical_proximity > 85.0 {
                health_score -= 25.0;
                issues.push(ThermalIssue {
                    issue_type: ThermalIssueType::HighTemperature,
                    severity: 7,
                    description: alloc::format!(
                        "Zone {} running hot: {}",
                        zone.name,
                        temp.to_string()
                    ),
                    zone: Some(zone.id),
                });
                recommendations.push(ThermalRecommendation {
                    action: ThermalAction::IncreaseFanSpeed,
                    expected_improvement: 15.0,
                    reason: String::from("Increase cooling to reduce temperature"),
                });
            }

            // Check temperature trend
            let trend = zone.temperature_trend();
            if trend > 5000 {
                // Rising more than 5 degrees per window
                health_score -= 10.0;
                issues.push(ThermalIssue {
                    issue_type: ThermalIssueType::RisingTrend,
                    severity: 6,
                    description: alloc::format!("Zone {} temperature rising rapidly", zone.name),
                    zone: Some(zone.id),
                });
            }

            // Check triggered trip points
            let triggered = zone.triggered_trips();
            for trip in triggered {
                if matches!(trip.trip_type, TripPointType::Passive | TripPointType::Hot) {
                    health_score -= 5.0;
                    issues.push(ThermalIssue {
                        issue_type: ThermalIssueType::Throttling,
                        severity: 5,
                        description: alloc::format!(
                            "Zone {} {} trip triggered at {}",
                            zone.name,
                            trip.trip_type.name(),
                            temp.to_string()
                        ),
                        zone: Some(zone.id),
                    });
                }
            }
        }

        // Check fans
        for (device_id, fan) in &self.manager.fans {
            if fan.mode == FanMode::Auto
                && fan.rpm() == 0
                && self
                    .manager
                    .hottest_zone()
                    .map(|z| z.temperature().celsius() > 50.0)
                    .unwrap_or(false)
            {
                cooling_score -= 30.0;
                issues.push(ThermalIssue {
                    issue_type: ThermalIssueType::FanFailure,
                    severity: 9,
                    description: String::from("Fan not spinning despite high temperature"),
                    zone: None,
                });
                recommendations.push(ThermalRecommendation {
                    action: ThermalAction::CleanCooling,
                    expected_improvement: 25.0,
                    reason: String::from("Check fan operation and clean cooling system"),
                });
            }

            // Check if fan is at max but still hot
            if let Some(device) = self.manager.get_cooling_device(*device_id) {
                if device.is_at_max() {
                    if let Some(hottest) = self.manager.hottest_zone() {
                        if hottest.temperature().celsius() > 80.0 {
                            cooling_score -= 15.0;
                            issues.push(ThermalIssue {
                                issue_type: ThermalIssueType::CoolingInsufficient,
                                severity: 6,
                                description: String::from(
                                    "Cooling at maximum but temperature still high",
                                ),
                                zone: None,
                            });
                            recommendations.push(ThermalRecommendation {
                                action: ThermalAction::ImproveAirflow,
                                expected_improvement: 15.0,
                                reason: String::from(
                                    "Improve case airflow or consider better cooling",
                                ),
                            });
                        }
                    }
                }
            }
        }

        health_score = health_score.max(0.0);
        cooling_score = cooling_score.max(0.0);

        ThermalAnalysis {
            health_score,
            cooling_score,
            issues,
            recommendations,
        }
    }

    /// Get manager
    pub fn manager(&self) -> &ThermalManager {
        &self.manager
    }

    /// Get manager mutably
    pub fn manager_mut(&mut self) -> &mut ThermalManager {
        &mut self.manager
    }
}

impl Default for ThermalIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
