//! Thermal Management Intelligence Module
//!
//! This module provides AI-powered thermal management analysis including temperature
//! monitoring, cooling device control, thermal zone management, and intelligent
//! throttling decisions.
//!
//! # Architecture
//!
//! The module is organized into focused submodules:
//! - `types`: Core types (ThermalZoneId, CoolingDeviceId, Temperature)
//! - `zone`: Thermal zones, trip points, and governors
//! - `cooling`: Cooling device management
//! - `fan`: Fan control and monitoring
//! - `event`: Thermal event tracking
//! - `manager`: Central thermal manager
//! - `intelligence`: AI-powered thermal analysis

#![allow(dead_code)]

extern crate alloc;

// Submodules
pub mod cooling;
pub mod event;
pub mod fan;
pub mod intelligence;
pub mod manager;
pub mod types;
pub mod zone;

// Re-export core types
// Re-export cooling types
pub use cooling::{CoolingDevice, CoolingDeviceType};
// Re-export event types
pub use event::{ThermalEvent, ThermalEventType};
// Re-export fan types
pub use fan::{FanInfo, FanMode};
// Re-export intelligence types
pub use intelligence::{
    ThermalAction, ThermalAnalysis, ThermalIntelligence, ThermalIssue, ThermalIssueType,
    ThermalRecommendation,
};
// Re-export manager types
pub use manager::ThermalManager;
pub use types::{CoolingDeviceId, Temperature, ThermalZoneId};
// Re-export zone types
pub use zone::{
    ThermalGovernor, ThermalZone, ThermalZoneMode, ThermalZoneType, TripPoint, TripPointType,
};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use alloc::string::String;

    use super::*;

    #[test]
    fn test_temperature() {
        let temp = Temperature::from_celsius(45.5);
        assert!((temp.celsius() - 45.5).abs() < 0.1);
        assert_eq!(temp.millidegrees(), 45500);
    }

    #[test]
    fn test_trip_point() {
        let trip = TripPoint::new(0, TripPointType::Passive, Temperature::from_celsius(80.0));

        assert!(trip.is_triggered(Temperature::from_celsius(85.0)));
        assert!(!trip.is_triggered(Temperature::from_celsius(75.0)));
    }

    #[test]
    fn test_thermal_zone() {
        let mut zone = ThermalZone::new(
            ThermalZoneId::new(0),
            String::from("cpu_thermal"),
            ThermalZoneType::CoreTemp,
        );

        zone.update_temperature(Temperature::from_celsius(50.0));
        assert!((zone.temperature().celsius() - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_thermal_intelligence() {
        let mut intel = ThermalIntelligence::new();

        let mut zone = ThermalZone::new(
            ThermalZoneId::new(0),
            String::from("cpu_thermal"),
            ThermalZoneType::CoreTemp,
        );
        zone.add_trip_point(TripPoint::new(
            0,
            TripPointType::Critical,
            Temperature::from_celsius(100.0),
        ));
        zone.update_temperature(Temperature::from_celsius(95.0)); // Near critical

        intel.register_zone(zone);

        let analysis = intel.analyze();
        // Should detect high temperature
        assert!(analysis.issues.iter().any(|i| matches!(
            i.issue_type,
            ThermalIssueType::HighTemperature | ThermalIssueType::CriticalTemperature
        )));
    }
}
