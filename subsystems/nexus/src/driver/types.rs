//! Driver types and enumerations.

use alloc::string::String;
use alloc::vec::Vec;

use crate::core::NexusTimestamp;

// ============================================================================
// DRIVER TYPES
// ============================================================================

/// Driver identifier
pub type DriverId = u32;

/// Device class
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeviceClass {
    /// Block storage device
    Storage,
    /// Network interface
    Network,
    /// Graphics/Display
    Graphics,
    /// Input device
    Input,
    /// Audio device
    Audio,
    /// USB controller
    Usb,
    /// PCI device
    Pci,
    /// ACPI device
    Acpi,
    /// Platform device
    Platform,
    /// Virtual device
    Virtual,
    /// Unknown class
    Unknown,
}

impl DeviceClass {
    /// Is this class critical for system operation?
    #[inline(always)]
    pub fn is_critical(&self) -> bool {
        matches!(self, Self::Storage | Self::Acpi | Self::Platform)
    }

    /// Get typical latency tolerance
    #[inline]
    pub fn latency_tolerance_us(&self) -> u64 {
        match self {
            Self::Input => 1000,     // 1ms for input
            Self::Audio => 500,      // 0.5ms for audio
            Self::Network => 10000,  // 10ms for network
            Self::Graphics => 16000, // 16ms for 60fps
            Self::Storage => 100000, // 100ms for storage
            _ => 50000,
        }
    }
}

/// Driver state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverState {
    /// Not loaded
    Unloaded,
    /// Loading
    Loading,
    /// Running normally
    Running,
    /// Suspended
    Suspended,
    /// Error state
    Error,
    /// Crashed
    Crashed,
    /// Being unloaded
    Unloading,
}

/// Driver health level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HealthLevel {
    /// Healthy
    Healthy  = 0,
    /// Minor issues
    Warning  = 1,
    /// Major issues
    Degraded = 2,
    /// Critical issues
    Critical = 3,
    /// Driver failed
    Failed   = 4,
}

// ============================================================================
// DRIVER INFO
// ============================================================================

/// Driver information
#[derive(Debug, Clone)]
pub struct DriverInfo {
    /// Driver ID
    pub id: DriverId,
    /// Driver name
    pub name: String,
    /// Version string
    pub version: String,
    /// Device class
    pub device_class: DeviceClass,
    /// Current state
    pub state: DriverState,
    /// Health level
    pub health: HealthLevel,
    /// Load timestamp
    pub loaded_at: Option<NexusTimestamp>,
    /// Error count
    pub error_count: u32,
    /// Crash count
    pub crash_count: u32,
    /// Restart count
    pub restart_count: u32,
    /// Dependencies
    pub dependencies: Vec<DriverId>,
    /// Dependents
    pub dependents: Vec<DriverId>,
}

impl DriverInfo {
    /// Create new driver info
    pub fn new(id: DriverId, name: &str, device_class: DeviceClass) -> Self {
        Self {
            id,
            name: String::from(name),
            version: String::from("0.0.0"),
            device_class,
            state: DriverState::Unloaded,
            health: HealthLevel::Healthy,
            loaded_at: None,
            error_count: 0,
            crash_count: 0,
            restart_count: 0,
            dependencies: Vec::new(),
            dependents: Vec::new(),
        }
    }

    /// Set version
    #[inline(always)]
    pub fn with_version(mut self, version: &str) -> Self {
        self.version = String::from(version);
        self
    }

    /// Add dependency
    #[inline]
    pub fn with_dependency(mut self, dep: DriverId) -> Self {
        if !self.dependencies.contains(&dep) {
            self.dependencies.push(dep);
        }
        self
    }

    /// Mark as loaded
    #[inline(always)]
    pub fn mark_loaded(&mut self) {
        self.state = DriverState::Running;
        self.loaded_at = Some(NexusTimestamp::now());
    }

    /// Mark as error
    #[inline]
    pub fn mark_error(&mut self) {
        self.error_count += 1;
        if self.error_count >= 10 {
            self.health = HealthLevel::Degraded;
        } else if self.error_count >= 3 {
            self.health = HealthLevel::Warning;
        }
    }

    /// Mark as crashed
    #[inline]
    pub fn mark_crashed(&mut self) {
        self.crash_count += 1;
        self.state = DriverState::Crashed;
        self.health = HealthLevel::Critical;
    }

    /// Mark as restarted
    #[inline]
    pub fn mark_restarted(&mut self) {
        self.restart_count += 1;
        self.state = DriverState::Running;
        // Reset health cautiously
        if self.crash_count < 3 {
            self.health = HealthLevel::Warning;
        }
    }

    /// Get uptime
    #[inline(always)]
    pub fn uptime(&self) -> Option<u64> {
        self.loaded_at
            .map(|t| NexusTimestamp::now().duration_since(t))
    }

    /// Is stable?
    #[inline]
    pub fn is_stable(&self) -> bool {
        self.state == DriverState::Running
            && self.health <= HealthLevel::Warning
            && self.crash_count == 0
    }
}
