//! # SBI Extension Management
//!
//! Utilities for probing and managing SBI extensions.

use super::{eid, base_fid};
use super::base::{sbi_call_1, SbiRet};
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// Extension Enum
// ============================================================================

/// SBI extension identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Extension {
    /// Base extension (always available)
    Base,
    /// Timer extension
    Timer,
    /// IPI extension
    Ipi,
    /// Remote fence extension
    RFence,
    /// Hart State Management extension
    Hsm,
    /// System Reset extension
    Srst,
    /// Performance Monitoring Unit extension
    Pmu,
    /// Debug Console extension
    Dbcn,
    /// System Suspend extension
    Susp,
    /// CPPC extension
    Cppc,
    /// Nested Acceleration extension
    Nacl,
    /// Steal-time Accounting extension
    Sta,
    /// Custom extension with ID
    Custom(usize),
}

impl Extension {
    /// Get the extension ID
    pub fn id(self) -> usize {
        match self {
            Self::Base => eid::BASE,
            Self::Timer => eid::TIME,
            Self::Ipi => eid::IPI,
            Self::RFence => eid::RFENCE,
            Self::Hsm => eid::HSM,
            Self::Srst => eid::SRST,
            Self::Pmu => eid::PMU,
            Self::Dbcn => eid::DBCN,
            Self::Susp => eid::SUSP,
            Self::Cppc => eid::CPPC,
            Self::Nacl => eid::NACL,
            Self::Sta => eid::STA,
            Self::Custom(id) => id,
        }
    }

    /// Get extension from ID
    pub fn from_id(id: usize) -> Self {
        match id {
            eid::BASE => Self::Base,
            eid::TIME => Self::Timer,
            eid::IPI => Self::Ipi,
            eid::RFENCE => Self::RFence,
            eid::HSM => Self::Hsm,
            eid::SRST => Self::Srst,
            eid::PMU => Self::Pmu,
            eid::DBCN => Self::Dbcn,
            eid::SUSP => Self::Susp,
            eid::CPPC => Self::Cppc,
            eid::NACL => Self::Nacl,
            eid::STA => Self::Sta,
            id => Self::Custom(id),
        }
    }

    /// Get extension name
    pub fn name(self) -> &'static str {
        match self {
            Self::Base => "Base",
            Self::Timer => "Timer",
            Self::Ipi => "IPI",
            Self::RFence => "Remote Fence",
            Self::Hsm => "Hart State Management",
            Self::Srst => "System Reset",
            Self::Pmu => "Performance Monitoring",
            Self::Dbcn => "Debug Console",
            Self::Susp => "System Suspend",
            Self::Cppc => "CPPC",
            Self::Nacl => "Nested Acceleration",
            Self::Sta => "Steal-time Accounting",
            Self::Custom(_) => "Custom",
        }
    }

    /// All standard extensions
    pub const ALL: &'static [Extension] = &[
        Self::Base,
        Self::Timer,
        Self::Ipi,
        Self::RFence,
        Self::Hsm,
        Self::Srst,
        Self::Pmu,
        Self::Dbcn,
        Self::Susp,
        Self::Cppc,
        Self::Nacl,
        Self::Sta,
    ];
}

// ============================================================================
// Extension Probing
// ============================================================================

/// Probe if an extension is available
pub fn probe_extension(extension_id: usize) -> bool {
    let ret = sbi_call_1(eid::BASE, base_fid::PROBE_EXTENSION, extension_id);
    ret.value != 0
}

/// Probe an extension by enum
pub fn probe(extension: Extension) -> bool {
    probe_extension(extension.id())
}

/// Get extension value (for extensions that return more than availability)
pub fn probe_extension_value(extension_id: usize) -> i64 {
    let ret = sbi_call_1(eid::BASE, base_fid::PROBE_EXTENSION, extension_id);
    ret.value
}

// ============================================================================
// Cached Extension Availability
// ============================================================================

/// Cached extension availability bitmap
static EXTENSION_CACHE: AtomicU64 = AtomicU64::new(0);

/// Cache initialized flag
static CACHE_INITIALIZED: AtomicU64 = AtomicU64::new(0);

/// Extension bit positions in cache
mod ext_bit {
    pub const BASE: u64 = 1 << 0;
    pub const TIMER: u64 = 1 << 1;
    pub const IPI: u64 = 1 << 2;
    pub const RFENCE: u64 = 1 << 3;
    pub const HSM: u64 = 1 << 4;
    pub const SRST: u64 = 1 << 5;
    pub const PMU: u64 = 1 << 6;
    pub const DBCN: u64 = 1 << 7;
    pub const SUSP: u64 = 1 << 8;
    pub const CPPC: u64 = 1 << 9;
    pub const NACL: u64 = 1 << 10;
    pub const STA: u64 = 1 << 11;
}

/// Initialize extension cache
pub fn init_extension_cache() {
    let mut cache = ext_bit::BASE; // Base is always available

    if probe_extension(eid::TIME) {
        cache |= ext_bit::TIMER;
    }
    if probe_extension(eid::IPI) {
        cache |= ext_bit::IPI;
    }
    if probe_extension(eid::RFENCE) {
        cache |= ext_bit::RFENCE;
    }
    if probe_extension(eid::HSM) {
        cache |= ext_bit::HSM;
    }
    if probe_extension(eid::SRST) {
        cache |= ext_bit::SRST;
    }
    if probe_extension(eid::PMU) {
        cache |= ext_bit::PMU;
    }
    if probe_extension(eid::DBCN) {
        cache |= ext_bit::DBCN;
    }
    if probe_extension(eid::SUSP) {
        cache |= ext_bit::SUSP;
    }
    if probe_extension(eid::CPPC) {
        cache |= ext_bit::CPPC;
    }
    if probe_extension(eid::NACL) {
        cache |= ext_bit::NACL;
    }
    if probe_extension(eid::STA) {
        cache |= ext_bit::STA;
    }

    EXTENSION_CACHE.store(cache, Ordering::SeqCst);
    CACHE_INITIALIZED.store(1, Ordering::SeqCst);
}

/// Check if an extension is available (uses cache if available)
pub fn is_available(extension: Extension) -> bool {
    // If cache not initialized, probe directly
    if CACHE_INITIALIZED.load(Ordering::Acquire) == 0 {
        return probe(extension);
    }

    let cache = EXTENSION_CACHE.load(Ordering::Relaxed);
    let bit = match extension {
        Extension::Base => ext_bit::BASE,
        Extension::Timer => ext_bit::TIMER,
        Extension::Ipi => ext_bit::IPI,
        Extension::RFence => ext_bit::RFENCE,
        Extension::Hsm => ext_bit::HSM,
        Extension::Srst => ext_bit::SRST,
        Extension::Pmu => ext_bit::PMU,
        Extension::Dbcn => ext_bit::DBCN,
        Extension::Susp => ext_bit::SUSP,
        Extension::Cppc => ext_bit::CPPC,
        Extension::Nacl => ext_bit::NACL,
        Extension::Sta => ext_bit::STA,
        Extension::Custom(_) => return probe(extension), // Always probe custom
    };

    cache & bit != 0
}

// ============================================================================
// Extension Requirements
// ============================================================================

/// Check if all required extensions are available
pub fn check_requirements(required: &[Extension]) -> Result<(), &'static [Extension]> {
    for ext in required {
        if !is_available(*ext) {
            return Err(required);
        }
    }
    Ok(())
}

/// Minimum required extensions for basic operation
pub const MINIMUM_REQUIRED: &[Extension] = &[
    Extension::Base,
    Extension::Timer,
    Extension::Ipi,
    Extension::RFence,
];

/// Check minimum requirements
pub fn check_minimum_requirements() -> bool {
    for ext in MINIMUM_REQUIRED {
        if !probe(*ext) {
            return false;
        }
    }
    true
}

// ============================================================================
// Extension Information
// ============================================================================

/// Get list of available extensions
pub fn available_extensions() -> alloc::vec::Vec<Extension> {
    let mut available = alloc::vec::Vec::new();

    for ext in Extension::ALL {
        if is_available(*ext) {
            available.push(*ext);
        }
    }

    available
}

extern crate alloc;

/// Extension availability summary
#[derive(Debug, Clone)]
pub struct ExtensionSummary {
    pub available: alloc::vec::Vec<Extension>,
    pub missing: alloc::vec::Vec<Extension>,
}

impl ExtensionSummary {
    /// Create a summary of extension availability
    pub fn new() -> Self {
        let mut available = alloc::vec::Vec::new();
        let mut missing = alloc::vec::Vec::new();

        for ext in Extension::ALL {
            if is_available(*ext) {
                available.push(*ext);
            } else {
                missing.push(*ext);
            }
        }

        Self { available, missing }
    }
}

impl Default for ExtensionSummary {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Extension Feature Flags
// ============================================================================

/// Feature flags derived from available extensions
#[derive(Debug, Clone, Copy, Default)]
pub struct SbiFeatures {
    /// Can manage hart state
    pub can_manage_harts: bool,
    /// Can send IPIs
    pub can_send_ipi: bool,
    /// Can do remote fences
    pub can_remote_fence: bool,
    /// Can reset system
    pub can_system_reset: bool,
    /// Can suspend system
    pub can_system_suspend: bool,
    /// Has PMU support
    pub has_pmu: bool,
    /// Has debug console
    pub has_debug_console: bool,
    /// Has CPPC support
    pub has_cppc: bool,
    /// Has nested acceleration
    pub has_nested_accel: bool,
    /// Has steal-time accounting
    pub has_steal_time: bool,
}

impl SbiFeatures {
    /// Detect features from available extensions
    pub fn detect() -> Self {
        Self {
            can_manage_harts: is_available(Extension::Hsm),
            can_send_ipi: is_available(Extension::Ipi),
            can_remote_fence: is_available(Extension::RFence),
            can_system_reset: is_available(Extension::Srst),
            can_system_suspend: is_available(Extension::Susp),
            has_pmu: is_available(Extension::Pmu),
            has_debug_console: is_available(Extension::Dbcn),
            has_cppc: is_available(Extension::Cppc),
            has_nested_accel: is_available(Extension::Nacl),
            has_steal_time: is_available(Extension::Sta),
        }
    }
}
