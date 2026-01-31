//! PMU (Performance Monitoring Unit)
//!
//! PMU types, capabilities, and management.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

use super::{CpuId, EventId, PmuId};

// ============================================================================
// PMU TYPES
// ============================================================================

/// PMU type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmuType {
    /// Core CPU PMU
    Core,
    /// Uncore PMU
    Uncore,
    /// Software PMU
    Software,
    /// Tracepoint
    Tracepoint,
    /// Raw hardware
    RawHardware,
    /// Hardware breakpoint
    Breakpoint,
    /// Power PMU
    Power,
    /// Memory controller
    MemoryController,
    /// Cache controller
    CacheController,
    /// Interconnect
    Interconnect,
    /// Unknown
    Unknown,
}

impl PmuType {
    /// Get type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Uncore => "uncore",
            Self::Software => "software",
            Self::Tracepoint => "tracepoint",
            Self::RawHardware => "raw_hardware",
            Self::Breakpoint => "breakpoint",
            Self::Power => "power",
            Self::MemoryController => "memory_controller",
            Self::CacheController => "cache_controller",
            Self::Interconnect => "interconnect",
            Self::Unknown => "unknown",
        }
    }

    /// Is hardware PMU
    pub fn is_hardware(&self) -> bool {
        matches!(
            self,
            Self::Core
                | Self::Uncore
                | Self::RawHardware
                | Self::Power
                | Self::MemoryController
                | Self::CacheController
                | Self::Interconnect
        )
    }
}

/// PMU capabilities
#[derive(Debug, Clone, Default)]
pub struct PmuCapabilities {
    /// Number of general purpose counters
    pub num_counters: u32,
    /// Number of fixed counters
    pub num_fixed_counters: u32,
    /// Counter width in bits
    pub counter_width: u8,
    /// Supports sampling
    pub supports_sampling: bool,
    /// Supports period
    pub supports_period: bool,
    /// Supports user space reading
    pub supports_userspace_read: bool,
    /// Supports exclusion filters
    pub supports_exclusion: bool,
    /// PMU version
    pub version: u8,
}

// ============================================================================
// PMU
// ============================================================================

/// PMU (Performance Monitoring Unit)
#[derive(Debug)]
pub struct Pmu {
    /// PMU ID
    pub id: PmuId,
    /// Name
    pub name: String,
    /// Type
    pub pmu_type: PmuType,
    /// Capabilities
    pub capabilities: PmuCapabilities,
    /// Available events
    pub events: Vec<EventId>,
    /// Active counters
    active_counters: AtomicU32,
    /// CPU mask
    pub cpu_mask: Vec<CpuId>,
}

impl Pmu {
    /// Create new PMU
    pub fn new(id: PmuId, name: String, pmu_type: PmuType) -> Self {
        Self {
            id,
            name,
            pmu_type,
            capabilities: PmuCapabilities::default(),
            events: Vec::new(),
            active_counters: AtomicU32::new(0),
            cpu_mask: Vec::new(),
        }
    }

    /// Active counter count
    pub fn active_counters(&self) -> u32 {
        self.active_counters.load(Ordering::Relaxed)
    }

    /// Increment active counters
    pub fn add_counter(&self) {
        self.active_counters.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement active counters
    pub fn remove_counter(&self) {
        self.active_counters.fetch_sub(1, Ordering::Relaxed);
    }

    /// Available counters
    pub fn available_counters(&self) -> u32 {
        let total = self.capabilities.num_counters + self.capabilities.num_fixed_counters;
        total.saturating_sub(self.active_counters())
    }
}
