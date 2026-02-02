//! # PSCI (Power State Coordination Interface)
//!
//! This module provides an implementation of the ARM Power State Coordination
//! Interface, which is the standard method for CPU power management on AArch64
//! systems.
//!
//! ## PSCI Overview
//!
//! PSCI provides a standard interface for:
//! - Powering CPUs on and off
//! - Suspending CPUs to low-power states
//! - System reset and shutdown
//! - CPU feature queries
//!
//! ## Conduit Methods
//!
//! PSCI can be invoked via two mechanisms:
//!
//! - **SMC (Secure Monitor Call)**: Traps to EL3 (Secure World)
//! - **HVC (Hypervisor Call)**: Traps to EL2 (Hypervisor)
//!
//! The conduit method is typically specified in the device tree or ACPI.
//!
//! ## Function IDs
//!
//! PSCI uses specific function IDs for each operation:
//!
//! | Function           | 32-bit ID    | 64-bit ID    |
//! |--------------------|--------------|--------------|
//! | PSCI_VERSION       | 0x8400_0000  | -            |
//! | CPU_SUSPEND        | 0x8400_0001  | 0xC400_0001  |
//! | CPU_OFF            | 0x8400_0002  | -            |
//! | CPU_ON             | 0x8400_0003  | 0xC400_0003  |
//! | AFFINITY_INFO      | 0x8400_0004  | 0xC400_0004  |
//! | SYSTEM_OFF         | 0x8400_0008  | -            |
//! | SYSTEM_RESET       | 0x8400_0009  | -            |
//! | PSCI_FEATURES      | 0x8400_000A  | -            |
//!
//! ## Usage
//!
//! ```ignore
//! use hal::arch::aarch64::smp::psci::{Psci, PsciConduit};
//!
//! let psci = Psci::new(PsciConduit::Smc);
//!
//! // Check PSCI version
//! let version = psci.version();
//! println!("PSCI version: {}.{}", version.major(), version.minor());
//!
//! // Start a secondary CPU
//! let mpidr = Mpidr::from_affinity(0, 0, 0, 1);
//! psci.cpu_on(mpidr, entry_point, context)?;
//! ```

use core::arch::asm;

use super::Mpidr;

// ============================================================================
// PSCI Function IDs
// ============================================================================

/// PSCI version query (32-bit)
pub const PSCI_VERSION: u32 = 0x8400_0000;

/// CPU suspend (32-bit)
pub const PSCI_CPU_SUSPEND_32: u32 = 0x8400_0001;

/// CPU suspend (64-bit)
pub const PSCI_CPU_SUSPEND_64: u64 = 0xC400_0001;

/// CPU off
pub const PSCI_CPU_OFF: u32 = 0x8400_0002;

/// CPU on (32-bit)
pub const PSCI_CPU_ON_32: u32 = 0x8400_0003;

/// CPU on (64-bit)
pub const PSCI_CPU_ON_64: u64 = 0xC400_0003;

/// Affinity info (32-bit)
pub const PSCI_AFFINITY_INFO_32: u32 = 0x8400_0004;

/// Affinity info (64-bit)
pub const PSCI_AFFINITY_INFO_64: u64 = 0xC400_0004;

/// Migrate
pub const PSCI_MIGRATE: u32 = 0x8400_0005;

/// Migrate info type
pub const PSCI_MIGRATE_INFO_TYPE: u32 = 0x8400_0006;

/// Migrate info up CPU
pub const PSCI_MIGRATE_INFO_UP_CPU: u32 = 0x8400_0007;

/// System off
pub const PSCI_SYSTEM_OFF: u32 = 0x8400_0008;

/// System reset
pub const PSCI_SYSTEM_RESET: u32 = 0x8400_0009;

/// PSCI features query
pub const PSCI_FEATURES: u32 = 0x8400_000A;

/// CPU freeze (PSCI 1.0)
pub const PSCI_CPU_FREEZE: u32 = 0x8400_000B;

/// CPU default suspend (PSCI 1.0)
pub const PSCI_CPU_DEFAULT_SUSPEND_32: u32 = 0x8400_000C;
pub const PSCI_CPU_DEFAULT_SUSPEND_64: u64 = 0xC400_000C;

/// Node HW state (PSCI 1.0)
pub const PSCI_NODE_HW_STATE_32: u32 = 0x8400_000D;
pub const PSCI_NODE_HW_STATE_64: u64 = 0xC400_000D;

/// System suspend (PSCI 1.0)
pub const PSCI_SYSTEM_SUSPEND_32: u32 = 0x8400_000E;
pub const PSCI_SYSTEM_SUSPEND_64: u64 = 0xC400_000E;

/// Set suspend mode (PSCI 1.0)
pub const PSCI_SET_SUSPEND_MODE: u32 = 0x8400_000F;

/// Stat residency (PSCI 1.0)
pub const PSCI_STAT_RESIDENCY_32: u32 = 0x8400_0010;
pub const PSCI_STAT_RESIDENCY_64: u64 = 0xC400_0010;

/// Stat count (PSCI 1.0)
pub const PSCI_STAT_COUNT_32: u32 = 0x8400_0011;
pub const PSCI_STAT_COUNT_64: u64 = 0xC400_0011;

/// System reset 2 (PSCI 1.1)
pub const PSCI_SYSTEM_RESET2_32: u32 = 0x8400_0012;
pub const PSCI_SYSTEM_RESET2_64: u64 = 0xC400_0012;

/// Memory protect (PSCI 1.1)
pub const PSCI_MEM_PROTECT: u32 = 0x8400_0013;

/// Memory protect check range (PSCI 1.1)
pub const PSCI_MEM_PROTECT_CHECK_RANGE_32: u32 = 0x8400_0014;
pub const PSCI_MEM_PROTECT_CHECK_RANGE_64: u64 = 0xC400_0014;

// ============================================================================
// PSCI Error Codes
// ============================================================================

/// PSCI success
pub const PSCI_SUCCESS: i32 = 0;

/// PSCI not supported
pub const PSCI_NOT_SUPPORTED: i32 = -1;

/// PSCI invalid parameters
pub const PSCI_INVALID_PARAMS: i32 = -2;

/// PSCI denied
pub const PSCI_DENIED: i32 = -3;

/// PSCI already on
pub const PSCI_ALREADY_ON: i32 = -4;

/// PSCI on pending
pub const PSCI_ON_PENDING: i32 = -5;

/// PSCI internal failure
pub const PSCI_INTERNAL_FAILURE: i32 = -6;

/// PSCI not present
pub const PSCI_NOT_PRESENT: i32 = -7;

/// PSCI disabled
pub const PSCI_DISABLED: i32 = -8;

/// PSCI invalid address
pub const PSCI_INVALID_ADDRESS: i32 = -9;

// ============================================================================
// PSCI Types
// ============================================================================

/// PSCI conduit method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PsciConduit {
    /// Secure Monitor Call (to EL3)
    Smc,
    /// Hypervisor Call (to EL2)
    Hvc,
}

impl Default for PsciConduit {
    fn default() -> Self {
        // SMC is more common
        PsciConduit::Smc
    }
}

/// PSCI version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PsciVersion(u32);

impl PsciVersion {
    /// Create from raw value
    pub const fn from_raw(value: u32) -> Self {
        Self(value)
    }

    /// Get major version
    pub const fn major(self) -> u16 {
        (self.0 >> 16) as u16
    }

    /// Get minor version
    pub const fn minor(self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }

    /// PSCI 0.1
    pub const V0_1: Self = Self(0x0000_0001);

    /// PSCI 0.2
    pub const V0_2: Self = Self(0x0000_0002);

    /// PSCI 1.0
    pub const V1_0: Self = Self(0x0001_0000);

    /// PSCI 1.1
    pub const V1_1: Self = Self(0x0001_0001);
}

impl core::fmt::Display for PsciVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}", self.major(), self.minor())
    }
}

/// Affinity info state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum AffinityState {
    /// CPU is on
    On        = 0,
    /// CPU is off
    Off       = 1,
    /// CPU is transitioning
    OnPending = 2,
}

impl AffinityState {
    /// Create from PSCI return value
    pub fn from_result(value: i32) -> Option<Self> {
        match value {
            0 => Some(AffinityState::On),
            1 => Some(AffinityState::Off),
            2 => Some(AffinityState::OnPending),
            _ => None,
        }
    }
}

/// PSCI error type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PsciError {
    /// Function not supported
    NotSupported,
    /// Invalid parameters
    InvalidParams,
    /// Operation denied
    Denied,
    /// CPU already on
    AlreadyOn,
    /// CPU on pending
    OnPending,
    /// Internal failure
    InternalFailure,
    /// CPU not present
    NotPresent,
    /// Function disabled
    Disabled,
    /// Invalid address
    InvalidAddress,
    /// Unknown error
    Unknown(i32),
}

impl PsciError {
    /// Create from PSCI return value
    pub fn from_result(value: i32) -> Option<Self> {
        match value {
            PSCI_SUCCESS => None,
            PSCI_NOT_SUPPORTED => Some(PsciError::NotSupported),
            PSCI_INVALID_PARAMS => Some(PsciError::InvalidParams),
            PSCI_DENIED => Some(PsciError::Denied),
            PSCI_ALREADY_ON => Some(PsciError::AlreadyOn),
            PSCI_ON_PENDING => Some(PsciError::OnPending),
            PSCI_INTERNAL_FAILURE => Some(PsciError::InternalFailure),
            PSCI_NOT_PRESENT => Some(PsciError::NotPresent),
            PSCI_DISABLED => Some(PsciError::Disabled),
            PSCI_INVALID_ADDRESS => Some(PsciError::InvalidAddress),
            _ => Some(PsciError::Unknown(value)),
        }
    }
}

// ============================================================================
// Low-Level Conduit Calls
// ============================================================================

/// Make an SMC call with 4 arguments
#[inline]
pub fn smc64(func_id: u64, arg1: u64, arg2: u64, arg3: u64) -> i64 {
    let result: i64;
    unsafe {
        asm!(
            "smc #0",
            inout("x0") func_id => result,
            inout("x1") arg1 => _,
            inout("x2") arg2 => _,
            inout("x3") arg3 => _,
            // Clobber x4-x17 as per SMCCC
            out("x4") _,
            out("x5") _,
            out("x6") _,
            out("x7") _,
            out("x8") _,
            out("x9") _,
            out("x10") _,
            out("x11") _,
            out("x12") _,
            out("x13") _,
            out("x14") _,
            out("x15") _,
            out("x16") _,
            out("x17") _,
            options(nomem, nostack),
        );
    }
    result
}

/// Make an SMC call with 32-bit function ID
#[inline]
pub fn smc32(func_id: u32, arg1: u64, arg2: u64, arg3: u64) -> i32 {
    smc64(func_id as u64, arg1, arg2, arg3) as i32
}

/// Make an HVC call with 4 arguments
#[inline]
pub fn hvc64(func_id: u64, arg1: u64, arg2: u64, arg3: u64) -> i64 {
    let result: i64;
    unsafe {
        asm!(
            "hvc #0",
            inout("x0") func_id => result,
            inout("x1") arg1 => _,
            inout("x2") arg2 => _,
            inout("x3") arg3 => _,
            out("x4") _,
            out("x5") _,
            out("x6") _,
            out("x7") _,
            out("x8") _,
            out("x9") _,
            out("x10") _,
            out("x11") _,
            out("x12") _,
            out("x13") _,
            out("x14") _,
            out("x15") _,
            out("x16") _,
            out("x17") _,
            options(nomem, nostack),
        );
    }
    result
}

/// Make an HVC call with 32-bit function ID
#[inline]
pub fn hvc32(func_id: u32, arg1: u64, arg2: u64, arg3: u64) -> i32 {
    hvc64(func_id as u64, arg1, arg2, arg3) as i32
}

// ============================================================================
// PSCI Interface
// ============================================================================

/// PSCI interface
#[derive(Debug, Clone, Copy)]
pub struct Psci {
    conduit: PsciConduit,
}

impl Psci {
    /// Create a new PSCI interface with the specified conduit
    pub const fn new(conduit: PsciConduit) -> Self {
        Self { conduit }
    }

    /// Create with SMC conduit
    pub const fn smc() -> Self {
        Self::new(PsciConduit::Smc)
    }

    /// Create with HVC conduit
    pub const fn hvc() -> Self {
        Self::new(PsciConduit::Hvc)
    }

    /// Get the conduit method
    pub const fn conduit(&self) -> PsciConduit {
        self.conduit
    }

    /// Make a 32-bit PSCI call
    #[inline]
    fn call32(&self, func_id: u32, arg1: u64, arg2: u64, arg3: u64) -> i32 {
        match self.conduit {
            PsciConduit::Smc => smc32(func_id, arg1, arg2, arg3),
            PsciConduit::Hvc => hvc32(func_id, arg1, arg2, arg3),
        }
    }

    /// Make a 64-bit PSCI call
    #[inline]
    fn call64(&self, func_id: u64, arg1: u64, arg2: u64, arg3: u64) -> i64 {
        match self.conduit {
            PsciConduit::Smc => smc64(func_id, arg1, arg2, arg3),
            PsciConduit::Hvc => hvc64(func_id, arg1, arg2, arg3),
        }
    }

    /// Convert result to Result type
    fn check_result(result: i32) -> Result<(), PsciError> {
        match PsciError::from_result(result) {
            None => Ok(()),
            Some(e) => Err(e),
        }
    }

    // ========================================================================
    // PSCI Functions
    // ========================================================================

    /// Get PSCI version
    pub fn version(&self) -> PsciVersion {
        let result = self.call32(PSCI_VERSION, 0, 0, 0);
        PsciVersion::from_raw(result as u32)
    }

    /// Check if a PSCI function is supported
    pub fn is_supported(&self, func_id: u32) -> bool {
        let result = self.call32(PSCI_FEATURES, func_id as u64, 0, 0);
        result >= 0
    }

    /// Get CPU affinity info
    pub fn affinity_info(&self, mpidr: Mpidr, level: u32) -> Result<AffinityState, PsciError> {
        let result = self.call64(
            PSCI_AFFINITY_INFO_64,
            mpidr.psci_affinity(),
            level as u64,
            0,
        ) as i32;

        if result < 0 {
            Err(PsciError::from_result(result).unwrap())
        } else {
            AffinityState::from_result(result).ok_or(PsciError::Unknown(result))
        }
    }

    /// Power on a CPU
    ///
    /// # Arguments
    ///
    /// - `mpidr`: Target CPU's MPIDR affinity
    /// - `entry_point`: Physical address of entry point
    /// - `context_id`: Context ID passed to the CPU in x0
    pub fn cpu_on(&self, mpidr: Mpidr, entry_point: u64, context_id: u64) -> Result<(), PsciError> {
        let result = self.call64(
            PSCI_CPU_ON_64,
            mpidr.psci_affinity(),
            entry_point,
            context_id,
        ) as i32;

        Self::check_result(result)
    }

    /// Power off the current CPU
    ///
    /// This function does not return on success.
    pub fn cpu_off(&self) -> Result<(), PsciError> {
        let result = self.call32(PSCI_CPU_OFF, 0, 0, 0);
        Self::check_result(result)
    }

    /// Suspend the current CPU
    ///
    /// # Arguments
    ///
    /// - `power_state`: Power state to enter
    /// - `entry_point`: Resume entry point
    /// - `context_id`: Context ID passed on resume
    pub fn cpu_suspend(
        &self,
        power_state: u64,
        entry_point: u64,
        context_id: u64,
    ) -> Result<(), PsciError> {
        let result = self.call64(PSCI_CPU_SUSPEND_64, power_state, entry_point, context_id) as i32;
        Self::check_result(result)
    }

    /// Shut down the system
    ///
    /// This function does not return.
    pub fn system_off(&self) -> ! {
        self.call32(PSCI_SYSTEM_OFF, 0, 0, 0);

        // Should not return, but loop just in case
        loop {
            unsafe { asm!("wfi", options(nomem, nostack)) };
        }
    }

    /// Reset the system
    ///
    /// This function does not return.
    pub fn system_reset(&self) -> ! {
        self.call32(PSCI_SYSTEM_RESET, 0, 0, 0);

        // Should not return, but loop just in case
        loop {
            unsafe { asm!("wfi", options(nomem, nostack)) };
        }
    }

    /// System suspend (PSCI 1.0+)
    pub fn system_suspend(&self, entry_point: u64, context_id: u64) -> Result<(), PsciError> {
        let result = self.call64(PSCI_SYSTEM_SUSPEND_64, entry_point, context_id, 0) as i32;
        Self::check_result(result)
    }

    /// Get node hardware state (PSCI 1.0+)
    pub fn node_hw_state(&self, mpidr: Mpidr, level: u32) -> Result<u32, PsciError> {
        let result = self.call64(
            PSCI_NODE_HW_STATE_64,
            mpidr.psci_affinity(),
            level as u64,
            0,
        ) as i32;

        if result < 0 {
            Err(PsciError::from_result(result).unwrap())
        } else {
            Ok(result as u32)
        }
    }
}

impl Default for Psci {
    fn default() -> Self {
        Self::smc()
    }
}

// ============================================================================
// Global PSCI Instance
// ============================================================================

/// Global PSCI instance (initialized at boot)
static mut PSCI: Psci = Psci::smc();

/// Initialize the global PSCI interface
///
/// # Safety
///
/// Must be called once during early boot, before any other PSCI calls.
pub unsafe fn init_psci(conduit: PsciConduit) {
    PSCI = Psci::new(conduit);
}

/// Get the global PSCI interface
pub fn psci() -> &'static Psci {
    unsafe { &PSCI }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Get PSCI version
pub fn psci_version() -> PsciVersion {
    psci().version()
}

/// Power on a secondary CPU
pub fn cpu_on(mpidr: Mpidr, entry_point: u64, context_id: u64) -> Result<(), PsciError> {
    psci().cpu_on(mpidr, entry_point, context_id)
}

/// Power off the current CPU
pub fn cpu_off() -> Result<(), PsciError> {
    psci().cpu_off()
}

/// Reset the system
pub fn system_reset() -> ! {
    psci().system_reset()
}

/// Shut down the system
pub fn system_off() -> ! {
    psci().system_off()
}

/// Get CPU affinity state
pub fn affinity_info(mpidr: Mpidr, level: u32) -> Result<AffinityState, PsciError> {
    psci().affinity_info(mpidr, level)
}
