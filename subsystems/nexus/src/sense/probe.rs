//! Probe Types and Configuration
//!
//! Defines probe types, states, and configuration.

#![allow(dead_code)]

use alloc::string::String;

use crate::types::{Priority, ProbeId, Timestamp};

// ============================================================================
// PROBE TYPES
// ============================================================================

/// Type of probe
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProbeType {
    /// CPU metrics
    Cpu,
    /// Memory metrics
    Memory,
    /// Block I/O
    BlockIo,
    /// Network I/O
    NetworkIo,
    /// Scheduler events
    Scheduler,
    /// Interrupt events
    Interrupt,
    /// System call events
    Syscall,
    /// Page fault events
    PageFault,
    /// Timer events
    Timer,
    /// Power management
    Power,
    /// Thermal events
    Thermal,
    /// Device events
    Device,
    /// Filesystem events
    Filesystem,
    /// Security events
    Security,
    /// Custom probe
    Custom,
}

impl ProbeType {
    /// Get probe type name
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Memory => "memory",
            Self::BlockIo => "block_io",
            Self::NetworkIo => "network_io",
            Self::Scheduler => "scheduler",
            Self::Interrupt => "interrupt",
            Self::Syscall => "syscall",
            Self::PageFault => "page_fault",
            Self::Timer => "timer",
            Self::Power => "power",
            Self::Thermal => "thermal",
            Self::Device => "device",
            Self::Filesystem => "filesystem",
            Self::Security => "security",
            Self::Custom => "custom",
        }
    }

    /// Default sampling rate in Hz
    pub const fn default_sample_rate(&self) -> u32 {
        match self {
            Self::Cpu => 100,
            Self::Memory => 10,
            Self::BlockIo => 1000,
            Self::NetworkIo => 1000,
            Self::Scheduler => 100,
            Self::Interrupt => 10000,
            Self::Syscall => 10000,
            Self::PageFault => 1000,
            Self::Timer => 100,
            Self::Power => 1,
            Self::Thermal => 1,
            Self::Device => 100,
            Self::Filesystem => 100,
            Self::Security => 1000,
            Self::Custom => 100,
        }
    }

    /// All probe types
    pub const fn all() -> [ProbeType; 15] {
        [
            Self::Cpu,
            Self::Memory,
            Self::BlockIo,
            Self::NetworkIo,
            Self::Scheduler,
            Self::Interrupt,
            Self::Syscall,
            Self::PageFault,
            Self::Timer,
            Self::Power,
            Self::Thermal,
            Self::Device,
            Self::Filesystem,
            Self::Security,
            Self::Custom,
        ]
    }
}

// ============================================================================
// PROBE STATE
// ============================================================================

/// Probe state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeState {
    /// Probe is registered but not active
    Registered,
    /// Probe is initializing
    Initializing,
    /// Probe is active and collecting
    Active,
    /// Probe is paused
    Paused,
    /// Probe has failed
    Failed,
    /// Probe is shutting down
    ShuttingDown,
    /// Probe is stopped
    Stopped,
}

impl ProbeState {
    /// Is collecting data
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Is paused
    pub const fn is_paused(&self) -> bool {
        matches!(self, Self::Paused)
    }

    /// Is stopped
    pub const fn is_stopped(&self) -> bool {
        matches!(self, Self::Stopped)
    }

    /// Is in error state
    pub const fn is_failed(&self) -> bool {
        matches!(self, Self::Failed)
    }

    /// Can transition to
    pub fn can_transition_to(&self, target: &Self) -> bool {
        match (self, target) {
            (Self::Registered, Self::Initializing) => true,
            (Self::Initializing, Self::Active) => true,
            (Self::Initializing, Self::Failed) => true,
            (Self::Active, Self::Paused) => true,
            (Self::Active, Self::Failed) => true,
            (Self::Active, Self::ShuttingDown) => true,
            (Self::Paused, Self::Active) => true,
            (Self::Paused, Self::ShuttingDown) => true,
            (Self::Failed, Self::Initializing) => true,
            (Self::Failed, Self::Stopped) => true,
            (Self::ShuttingDown, Self::Stopped) => true,
            _ => false,
        }
    }
}

// ============================================================================
// PROBE CONFIGURATION
// ============================================================================

/// Probe configuration
#[derive(Debug, Clone)]
pub struct ProbeConfig {
    /// Probe type
    pub probe_type: ProbeType,
    /// Sampling rate in Hz (0 = event-driven)
    pub sample_rate: u32,
    /// Buffer size for collected events
    pub buffer_size: usize,
    /// Enable probe on start
    pub auto_enable: bool,
    /// Filter expression (optional)
    pub filter: Option<String>,
    /// CPU affinity (None = any CPU)
    pub cpu_affinity: Option<u32>,
    /// Priority (higher = more important)
    pub priority: Priority,
}

impl Default for ProbeConfig {
    fn default() -> Self {
        Self {
            probe_type: ProbeType::Custom,
            sample_rate: 100,
            buffer_size: 1024,
            auto_enable: true,
            filter: None,
            cpu_affinity: None,
            priority: Priority::Normal,
        }
    }
}

impl ProbeConfig {
    /// Create config for specific probe type
    pub fn for_type(probe_type: ProbeType) -> Self {
        Self {
            probe_type,
            sample_rate: probe_type.default_sample_rate(),
            ..Default::default()
        }
    }

    /// With sample rate
    pub fn with_sample_rate(mut self, rate: u32) -> Self {
        self.sample_rate = rate;
        self
    }

    /// With buffer size
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    /// With auto enable setting
    pub fn with_auto_enable(mut self, enable: bool) -> Self {
        self.auto_enable = enable;
        self
    }

    /// With filter
    pub fn with_filter(mut self, filter: impl Into<String>) -> Self {
        self.filter = Some(filter.into());
        self
    }
}

// ============================================================================
// PROBE STATISTICS
// ============================================================================

/// Probe statistics
#[derive(Debug, Clone, Default)]
pub struct ProbeStats {
    /// Events collected
    pub events_collected: u64,
    /// Events dropped
    pub events_dropped: u64,
    /// Bytes collected
    pub bytes_collected: u64,
    /// Errors encountered
    pub errors: u64,
    /// Last event timestamp
    pub last_event: Option<Timestamp>,
    /// Start time
    pub start_time: Option<Timestamp>,
}

impl ProbeStats {
    /// Create new stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Get drop rate
    pub fn drop_rate(&self) -> f64 {
        let total = self.events_collected + self.events_dropped;
        if total == 0 {
            0.0
        } else {
            self.events_dropped as f64 / total as f64
        }
    }

    /// Get uptime
    pub fn uptime(&self) -> Option<crate::types::Duration> {
        self.start_time
            .map(|start| Timestamp::now().elapsed_since(start))
    }
}

// ============================================================================
// PROBE ERROR
// ============================================================================

/// Probe error
#[derive(Debug, Clone)]
pub struct ProbeError {
    /// Error code
    pub code: ProbeErrorCode,
    /// Error message
    pub message: String,
}

impl ProbeError {
    /// Create new error
    pub fn new(code: ProbeErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// Probe error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeErrorCode {
    /// Probe not initialized
    NotInitialized,
    /// Probe already running
    AlreadyRunning,
    /// Probe not running
    NotRunning,
    /// Invalid configuration
    InvalidConfig,
    /// Permission denied
    PermissionDenied,
    /// Resource not available
    ResourceUnavailable,
    /// Buffer overflow
    BufferOverflow,
    /// Internal error
    Internal,
}

// ============================================================================
// PROBE TRAIT
// ============================================================================

/// Probe trait - implemented by all probes
pub trait Probe: Send + Sync {
    /// Get probe ID
    fn id(&self) -> ProbeId;

    /// Get probe type
    fn probe_type(&self) -> ProbeType;

    /// Get probe name
    fn name(&self) -> &str;

    /// Get current state
    fn state(&self) -> ProbeState;

    /// Get configuration
    fn config(&self) -> &ProbeConfig;

    /// Initialize the probe
    fn init(&mut self) -> Result<(), ProbeError>;

    /// Start collecting
    fn start(&mut self) -> Result<(), ProbeError>;

    /// Stop collecting
    fn stop(&mut self) -> Result<(), ProbeError>;

    /// Pause collecting
    fn pause(&mut self) -> Result<(), ProbeError>;

    /// Resume collecting
    fn resume(&mut self) -> Result<(), ProbeError>;

    /// Poll for events (non-blocking)
    fn poll(&mut self) -> Option<super::events::RawEvent>;

    /// Get statistics
    fn stats(&self) -> ProbeStats;
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_type() {
        assert_eq!(ProbeType::Cpu.name(), "cpu");
        assert_eq!(ProbeType::Cpu.default_sample_rate(), 100);
    }

    #[test]
    fn test_probe_state_transitions() {
        assert!(ProbeState::Registered.can_transition_to(&ProbeState::Initializing));
        assert!(ProbeState::Active.can_transition_to(&ProbeState::Paused));
        assert!(!ProbeState::Stopped.can_transition_to(&ProbeState::Active));
    }

    #[test]
    fn test_probe_config() {
        let config = ProbeConfig::for_type(ProbeType::Memory)
            .with_sample_rate(50)
            .with_buffer_size(2048);

        assert_eq!(config.probe_type, ProbeType::Memory);
        assert_eq!(config.sample_rate, 50);
        assert_eq!(config.buffer_size, 2048);
    }
}
