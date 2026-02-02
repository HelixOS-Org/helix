//! # Power Management Intelligence
//!
//! AI-powered power management and energy efficiency optimization.
//!
//! ## Key Features
//!
//! - **Workload Prediction**: Predict CPU/memory needs for DVFS
//! - **C-State Intelligence**: Smart C-state selection
//! - **P-State Optimization**: Dynamic P-state management
//! - **Thermal Management**: Temperature-aware power decisions
//! - **Battery Optimization**: Extend battery life on mobile
//! - **Energy Profiling**: Per-task energy accounting

#![allow(dead_code)]

extern crate alloc;

// Submodules
mod cstate;
mod energy;
mod intelligence;
mod pstate;
mod thermal;
mod types;
mod workload;

// Re-exports
pub use cstate::{CStateSelector, CStateStats, IdleTimePredictor};
pub use energy::{EnergyProfiler, PowerSensor, SystemEnergy, TaskEnergy};
pub use intelligence::{PowerDecision, PowerIntelligence};
pub use pstate::{GovernorAlgorithm, PStateGovernor, PStateStats};
pub use thermal::{ThermalManager, ThermalZone};
pub use types::{CState, PState, PowerMode};
pub use workload::WorkloadPredictor;

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn test_cstate_properties() {
        assert!(CState::C6.wakeup_latency() > CState::C1.wakeup_latency());
        assert!(CState::C6.power_reduction() < CState::C1.power_reduction());
    }

    #[test]
    fn test_pstate() {
        let p1 = PState::new(2000, 900);
        let p2 = PState::new(4000, 1200);

        assert!(p2.relative_perf > p1.relative_perf);
        assert!(p2.relative_power > p1.relative_power);
    }

    #[test]
    fn test_thermal_zone() {
        let mut zone = ThermalZone::new(0, "cpu");
        zone.temperature = 80_000; // 80°C

        assert!(!zone.is_critical());
        assert!(zone.should_throttle());

        zone.temperature = 90_000; // 90°C
        assert!(zone.is_hot());
    }

    #[test]
    fn test_workload_predictor() {
        let mut predictor = WorkloadPredictor::new();

        for i in 0..50 {
            predictor.record(0.5 + (i as f64 * 0.01), 0);
        }

        assert!(predictor.is_increasing());
        assert!(!predictor.is_idle());
    }

    #[test]
    fn test_cstate_selector() {
        let selector = CStateSelector::new(vec![
            CState::C0,
            CState::C1,
            CState::C1E,
            CState::C3,
            CState::C6,
        ]);

        // With default settings, should select a C-state
        let selected = selector.select();
        assert!(selected >= CState::C0);
    }

    #[test]
    fn test_energy_profiler() {
        let mut profiler = EnergyProfiler::new();
        let p_state = PState::new(3000, 1000);

        profiler.record_cpu(1, 1_000_000, &p_state);
        profiler.record_memory(1, 1000);
        profiler.record_runtime(1, 1_000_000);

        let energy = profiler.get_task_energy(1).unwrap();
        assert!(energy.total() > 0.0);
    }
}
