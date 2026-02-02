//! Sensor and Perception Traits
//!
//! Traits for the SENSE domain - perception and data collection.

#![allow(dead_code)]

use alloc::boxed::Box;
use alloc::vec::Vec;

use super::component::NexusComponent;
use crate::types::{MetricUnit, NexusResult};

// ============================================================================
// SENSOR TRAIT
// ============================================================================

/// Trait for perception sensors/probes
pub trait Sensor: NexusComponent {
    /// Type of signal this sensor produces
    type SignalType;

    /// Sample the current value
    fn sample(&self) -> NexusResult<Self::SignalType>;

    /// Get the sampling rate (samples per second)
    fn sample_rate(&self) -> u32;

    /// Set the sampling rate
    fn set_sample_rate(&mut self, rate: u32) -> NexusResult<()>;

    /// Check if sensor is calibrated
    fn is_calibrated(&self) -> bool;

    /// Calibrate the sensor
    fn calibrate(&mut self) -> NexusResult<()>;

    /// Get signal quality (0.0 to 1.0)
    fn quality(&self) -> f32;

    /// Get sensor metadata
    fn metadata(&self) -> SensorMetadata;
}

// ============================================================================
// SENSOR METADATA
// ============================================================================

/// Sensor metadata
#[derive(Debug, Clone)]
pub struct SensorMetadata {
    /// Sensor type
    pub sensor_type: SensorType,
    /// Unit of measurement
    pub unit: MetricUnit,
    /// Minimum value
    pub min_value: Option<f64>,
    /// Maximum value
    pub max_value: Option<f64>,
    /// Precision
    pub precision: f64,
    /// Is hardware sensor
    pub is_hardware: bool,
}

impl Default for SensorMetadata {
    fn default() -> Self {
        Self {
            sensor_type: SensorType::Custom,
            unit: MetricUnit::None,
            min_value: None,
            max_value: None,
            precision: 1.0,
            is_hardware: false,
        }
    }
}

// ============================================================================
// SENSOR TYPE
// ============================================================================

/// Types of sensors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SensorType {
    /// CPU metrics (load, frequency, usage)
    Cpu,
    /// Memory metrics (usage, pressure, allocation)
    Memory,
    /// Disk metrics (I/O, latency, throughput)
    Disk,
    /// Network metrics (bandwidth, packets, errors)
    Network,
    /// Power metrics (consumption, state)
    Power,
    /// Thermal metrics (temperature, cooling)
    Thermal,
    /// Process metrics (count, state, resources)
    Process,
    /// Interrupt metrics
    Interrupt,
    /// Syscall metrics
    Syscall,
    /// Scheduler metrics
    Scheduler,
    /// Kernel events
    KernelEvent,
    /// BPF probe
    Bpf,
    /// Custom/user-defined sensor
    Custom,
}

impl SensorType {
    /// Get sensor type name
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Memory => "memory",
            Self::Disk => "disk",
            Self::Network => "network",
            Self::Power => "power",
            Self::Thermal => "thermal",
            Self::Process => "process",
            Self::Interrupt => "interrupt",
            Self::Syscall => "syscall",
            Self::Scheduler => "scheduler",
            Self::KernelEvent => "kernel_event",
            Self::Bpf => "bpf",
            Self::Custom => "custom",
        }
    }

    /// Is this a hardware sensor?
    pub const fn is_hardware(&self) -> bool {
        matches!(
            self,
            Self::Cpu | Self::Memory | Self::Disk | Self::Network | Self::Power | Self::Thermal
        )
    }
}

// ============================================================================
// EVENT STREAM TRAIT
// ============================================================================

/// Event stream from perception
pub trait EventStream: NexusComponent {
    /// Type of events this stream produces
    type EventType;

    /// Poll for next event (non-blocking)
    fn poll(&mut self) -> Option<Self::EventType>;

    /// Subscribe to events with callback
    fn subscribe(&mut self, callback: Box<dyn Fn(&Self::EventType) + Send + Sync>);

    /// Unsubscribe all callbacks
    fn unsubscribe_all(&mut self);

    /// Get pending event count
    fn pending_count(&self) -> usize;

    /// Is stream active
    fn is_active(&self) -> bool;

    /// Drain all pending events
    fn drain(&mut self) -> Vec<Self::EventType>;
}

// ============================================================================
// SIGNAL AGGREGATOR TRAIT
// ============================================================================

/// Aggregates multiple signals into a composite view
pub trait SignalAggregator: NexusComponent {
    /// Input signal type
    type Signal;
    /// Aggregated output type
    type Aggregate;

    /// Add a signal to aggregation window
    fn add(&mut self, signal: Self::Signal);

    /// Compute aggregate
    fn aggregate(&self) -> Self::Aggregate;

    /// Clear aggregation window
    fn clear(&mut self);

    /// Get window size
    fn window_size(&self) -> usize;

    /// Set window size
    fn set_window_size(&mut self, size: usize);
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensor_type() {
        assert!(SensorType::Cpu.is_hardware());
        assert!(SensorType::Memory.is_hardware());
        assert!(!SensorType::KernelEvent.is_hardware());
        assert!(!SensorType::Custom.is_hardware());
    }

    #[test]
    fn test_sensor_type_name() {
        assert_eq!(SensorType::Cpu.name(), "cpu");
        assert_eq!(SensorType::Network.name(), "network");
    }
}
