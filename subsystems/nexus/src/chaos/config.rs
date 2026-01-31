//! Fault configuration

#![allow(dead_code)]

use super::target::FaultTarget;
use super::types::FaultType;

// ============================================================================
// FAULT CONFIG
// ============================================================================

/// Configuration for a fault
#[derive(Debug, Clone)]
pub struct FaultConfig {
    /// Fault type
    pub fault_type: FaultType,
    /// Target
    pub target: FaultTarget,
    /// Duration (cycles)
    pub duration_cycles: Option<u64>,
    /// Probability of fault occurring (0.0-1.0)
    pub probability: f32,
    /// Maximum number of occurrences
    pub max_occurrences: Option<u32>,
    /// Latency to inject (for latency faults)
    pub latency_cycles: Option<u64>,
    /// Memory pressure (for memory faults, 0.0-1.0)
    pub memory_pressure: Option<f32>,
    /// Enabled
    pub enabled: bool,
}

impl Default for FaultConfig {
    fn default() -> Self {
        Self {
            fault_type: FaultType::Latency,
            target: FaultTarget::Global,
            duration_cycles: None,
            probability: 0.01, // 1%
            max_occurrences: None,
            latency_cycles: Some(1_000_000), // ~1ms
            memory_pressure: None,
            enabled: true,
        }
    }
}

impl FaultConfig {
    /// Create a memory fault config
    pub fn memory(pressure: f32) -> Self {
        Self {
            fault_type: FaultType::Memory,
            memory_pressure: Some(pressure.clamp(0.0, 1.0)),
            ..Default::default()
        }
    }

    /// Create a latency fault config
    pub fn latency(cycles: u64) -> Self {
        Self {
            fault_type: FaultType::Latency,
            latency_cycles: Some(cycles),
            ..Default::default()
        }
    }

    /// Create a CPU fault config
    pub fn cpu() -> Self {
        Self {
            fault_type: FaultType::Cpu,
            ..Default::default()
        }
    }

    /// Create an I/O fault config
    pub fn io() -> Self {
        Self {
            fault_type: FaultType::Io,
            ..Default::default()
        }
    }

    /// Set probability
    pub fn with_probability(mut self, p: f32) -> Self {
        self.probability = p.clamp(0.0, 1.0);
        self
    }

    /// Set target
    pub fn with_target(mut self, target: FaultTarget) -> Self {
        self.target = target;
        self
    }

    /// Set duration
    pub fn with_duration(mut self, cycles: u64) -> Self {
        self.duration_cycles = Some(cycles);
        self
    }

    /// Set max occurrences
    pub fn with_max_occurrences(mut self, max: u32) -> Self {
        self.max_occurrences = Some(max);
        self
    }
}
