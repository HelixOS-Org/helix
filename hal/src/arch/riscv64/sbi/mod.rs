//! # Supervisor Binary Interface (SBI) Framework
//!
//! Complete SBI interface for RISC-V S-mode kernels.
//!
//! The SBI provides a standard interface between the supervisor (S-mode)
//! and the machine (M-mode) firmware (e.g., OpenSBI, RustSBI).
//!
//! ## Submodules
//!
//! - `base`: Base extension (required)
//! - `extensions`: Extension detection and management
//! - `hsm`: Hart State Management
//! - `ipi`: Inter-Processor Interrupts
//! - `rfence`: Remote Fence operations
//! - `timer`: Timer extension
//! - `pmu`: Performance Monitoring Unit

pub mod base;
pub mod extensions;
pub mod hsm;
pub mod ipi;
pub mod rfence;
pub mod timer;
pub mod pmu;

// Re-export commonly used items
pub use base::{SbiRet, sbi_call, get_spec_version, get_impl_id, get_impl_version};
pub use extensions::{probe_extension, Extension};
pub use hsm::{hart_start, hart_stop, hart_get_status, HartState};

// ============================================================================
// SBI Constants
// ============================================================================

/// SBI specification version this implementation targets
pub const SBI_SPEC_VERSION_MAJOR: u32 = 2;
pub const SBI_SPEC_VERSION_MINOR: u32 = 0;

// ============================================================================
// SBI Error Codes
// ============================================================================

/// SBI error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
pub enum SbiError {
    /// Success
    Success = 0,
    /// Failed
    Failed = -1,
    /// Not supported
    NotSupported = -2,
    /// Invalid parameter
    InvalidParam = -3,
    /// Denied
    Denied = -4,
    /// Invalid address
    InvalidAddress = -5,
    /// Already available
    AlreadyAvailable = -6,
    /// Already started
    AlreadyStarted = -7,
    /// Already stopped
    AlreadyStopped = -8,
    /// No shared memory
    NoShmem = -9,
}

impl SbiError {
    /// Convert from raw error code
    pub fn from_raw(value: i64) -> Self {
        match value {
            0 => Self::Success,
            -1 => Self::Failed,
            -2 => Self::NotSupported,
            -3 => Self::InvalidParam,
            -4 => Self::Denied,
            -5 => Self::InvalidAddress,
            -6 => Self::AlreadyAvailable,
            -7 => Self::AlreadyStarted,
            -8 => Self::AlreadyStopped,
            -9 => Self::NoShmem,
            _ => Self::Failed,
        }
    }

    /// Check if success
    pub fn is_success(self) -> bool {
        matches!(self, Self::Success)
    }

    /// Convert to Result
    pub fn as_result(self) -> Result<(), Self> {
        if self.is_success() {
            Ok(())
        } else {
            Err(self)
        }
    }
}

impl core::fmt::Display for SbiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Success => write!(f, "Success"),
            Self::Failed => write!(f, "Operation failed"),
            Self::NotSupported => write!(f, "Not supported"),
            Self::InvalidParam => write!(f, "Invalid parameter"),
            Self::Denied => write!(f, "Access denied"),
            Self::InvalidAddress => write!(f, "Invalid address"),
            Self::AlreadyAvailable => write!(f, "Already available"),
            Self::AlreadyStarted => write!(f, "Already started"),
            Self::AlreadyStopped => write!(f, "Already stopped"),
            Self::NoShmem => write!(f, "No shared memory"),
        }
    }
}

// ============================================================================
// SBI Extension IDs
// ============================================================================

/// SBI extension IDs
pub mod eid {
    /// Legacy Set Timer
    pub const LEGACY_SET_TIMER: usize = 0x00;
    /// Legacy Console Putchar
    pub const LEGACY_CONSOLE_PUTCHAR: usize = 0x01;
    /// Legacy Console Getchar
    pub const LEGACY_CONSOLE_GETCHAR: usize = 0x02;
    /// Legacy Clear IPI
    pub const LEGACY_CLEAR_IPI: usize = 0x03;
    /// Legacy Send IPI
    pub const LEGACY_SEND_IPI: usize = 0x04;
    /// Legacy Remote FENCE.I
    pub const LEGACY_REMOTE_FENCE_I: usize = 0x05;
    /// Legacy Remote SFENCE.VMA
    pub const LEGACY_REMOTE_SFENCE_VMA: usize = 0x06;
    /// Legacy Remote SFENCE.VMA with ASID
    pub const LEGACY_REMOTE_SFENCE_VMA_ASID: usize = 0x07;
    /// Legacy Shutdown
    pub const LEGACY_SHUTDOWN: usize = 0x08;

    /// Base Extension
    pub const BASE: usize = 0x10;
    /// Timer Extension
    pub const TIME: usize = 0x54494D45;
    /// IPI Extension
    pub const IPI: usize = 0x735049;
    /// RFENCE Extension
    pub const RFENCE: usize = 0x52464E43;
    /// Hart State Management Extension
    pub const HSM: usize = 0x48534D;
    /// System Reset Extension
    pub const SRST: usize = 0x53525354;
    /// Performance Monitoring Unit Extension
    pub const PMU: usize = 0x504D55;
    /// Debug Console Extension
    pub const DBCN: usize = 0x4442434E;
    /// System Suspend Extension
    pub const SUSP: usize = 0x53555350;
    /// CPPC Extension
    pub const CPPC: usize = 0x43505043;
    /// Nested Acceleration Extension
    pub const NACL: usize = 0x4E41434C;
    /// Steal-time Accounting Extension
    pub const STA: usize = 0x535441;
}

// ============================================================================
// SBI Function IDs
// ============================================================================

/// Base extension function IDs
pub mod base_fid {
    pub const GET_SPEC_VERSION: usize = 0;
    pub const GET_IMPL_ID: usize = 1;
    pub const GET_IMPL_VERSION: usize = 2;
    pub const PROBE_EXTENSION: usize = 3;
    pub const GET_MVENDORID: usize = 4;
    pub const GET_MARCHID: usize = 5;
    pub const GET_MIMPID: usize = 6;
}

/// HSM extension function IDs
pub mod hsm_fid {
    pub const HART_START: usize = 0;
    pub const HART_STOP: usize = 1;
    pub const HART_GET_STATUS: usize = 2;
    pub const HART_SUSPEND: usize = 3;
}

/// RFENCE extension function IDs
pub mod rfence_fid {
    pub const REMOTE_FENCE_I: usize = 0;
    pub const REMOTE_SFENCE_VMA: usize = 1;
    pub const REMOTE_SFENCE_VMA_ASID: usize = 2;
    pub const REMOTE_HFENCE_GVMA_VMID: usize = 3;
    pub const REMOTE_HFENCE_GVMA: usize = 4;
    pub const REMOTE_HFENCE_VVMA_ASID: usize = 5;
    pub const REMOTE_HFENCE_VVMA: usize = 6;
}

/// SRST extension function IDs
pub mod srst_fid {
    pub const SYSTEM_RESET: usize = 0;
}

/// PMU extension function IDs
pub mod pmu_fid {
    pub const NUM_COUNTERS: usize = 0;
    pub const COUNTER_GET_INFO: usize = 1;
    pub const COUNTER_CFG_MATCH: usize = 2;
    pub const COUNTER_START: usize = 3;
    pub const COUNTER_STOP: usize = 4;
    pub const COUNTER_FW_READ: usize = 5;
    pub const COUNTER_FW_READ_HI: usize = 6;
}

// ============================================================================
// SBI Implementation IDs
// ============================================================================

/// Known SBI implementation IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum SbiImplId {
    /// Berkeley Boot Loader
    Bbl = 0,
    /// OpenSBI
    OpenSbi = 1,
    /// Xvisor
    Xvisor = 2,
    /// KVM
    Kvm = 3,
    /// RustSBI
    RustSbi = 4,
    /// Diosix
    Diosix = 5,
    /// Coffer
    Coffer = 6,
    /// Xen
    Xen = 7,
    /// PolarFire Hart Software Services
    PolarfireHss = 8,
    /// Unknown implementation
    Unknown(u64),
}

impl SbiImplId {
    /// Convert from raw value
    pub fn from_raw(value: u64) -> Self {
        match value {
            0 => Self::Bbl,
            1 => Self::OpenSbi,
            2 => Self::Xvisor,
            3 => Self::Kvm,
            4 => Self::RustSbi,
            5 => Self::Diosix,
            6 => Self::Coffer,
            7 => Self::Xen,
            8 => Self::PolarfireHss,
            v => Self::Unknown(v),
        }
    }

    /// Get the name of the implementation
    pub fn name(self) -> &'static str {
        match self {
            Self::Bbl => "BBL",
            Self::OpenSbi => "OpenSBI",
            Self::Xvisor => "Xvisor",
            Self::Kvm => "KVM",
            Self::RustSbi => "RustSBI",
            Self::Diosix => "Diosix",
            Self::Coffer => "Coffer",
            Self::Xen => "Xen",
            Self::PolarfireHss => "PolarFire HSS",
            Self::Unknown(_) => "Unknown",
        }
    }
}

// ============================================================================
// SBI Information
// ============================================================================

/// SBI firmware information
#[derive(Debug, Clone)]
pub struct SbiInfo {
    /// Spec version
    pub spec_version: (u32, u32),
    /// Implementation ID
    pub impl_id: SbiImplId,
    /// Implementation version
    pub impl_version: u64,
    /// Machine vendor ID
    pub mvendorid: u64,
    /// Machine architecture ID
    pub marchid: u64,
    /// Machine implementation ID
    pub mimpid: u64,
    /// Available extensions
    pub extensions: Extensions,
}

impl SbiInfo {
    /// Query SBI information
    pub fn query() -> Self {
        let spec_version = base::get_spec_version();
        let impl_id = SbiImplId::from_raw(base::get_impl_id());
        let impl_version = base::get_impl_version();
        let mvendorid = base::get_mvendorid();
        let marchid = base::get_marchid();
        let mimpid = base::get_mimpid();
        let extensions = Extensions::probe_all();

        Self {
            spec_version,
            impl_id,
            impl_version,
            mvendorid,
            marchid,
            mimpid,
            extensions,
        }
    }
}

/// Available SBI extensions
#[derive(Debug, Clone, Default)]
pub struct Extensions {
    pub time: bool,
    pub ipi: bool,
    pub rfence: bool,
    pub hsm: bool,
    pub srst: bool,
    pub pmu: bool,
    pub dbcn: bool,
    pub susp: bool,
    pub cppc: bool,
    pub nacl: bool,
    pub sta: bool,
}

impl Extensions {
    /// Probe all known extensions
    pub fn probe_all() -> Self {
        Self {
            time: extensions::probe_extension(eid::TIME),
            ipi: extensions::probe_extension(eid::IPI),
            rfence: extensions::probe_extension(eid::RFENCE),
            hsm: extensions::probe_extension(eid::HSM),
            srst: extensions::probe_extension(eid::SRST),
            pmu: extensions::probe_extension(eid::PMU),
            dbcn: extensions::probe_extension(eid::DBCN),
            susp: extensions::probe_extension(eid::SUSP),
            cppc: extensions::probe_extension(eid::CPPC),
            nacl: extensions::probe_extension(eid::NACL),
            sta: extensions::probe_extension(eid::STA),
        }
    }
}

// ============================================================================
// Legacy SBI Interface
// ============================================================================

/// Legacy SBI console putchar
///
/// Note: This is deprecated, use DBCN extension instead.
pub fn legacy_console_putchar(ch: u8) {
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") eid::LEGACY_CONSOLE_PUTCHAR,
            in("a0") ch as usize,
            options(nostack)
        );
    }
}

/// Legacy SBI console getchar
///
/// Returns the character read, or -1 if no character available.
/// Note: This is deprecated, use DBCN extension instead.
pub fn legacy_console_getchar() -> i64 {
    let ret: i64;
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") eid::LEGACY_CONSOLE_GETCHAR,
            lateout("a0") ret,
            options(nostack)
        );
    }
    ret
}

/// Legacy SBI shutdown
///
/// Note: This is deprecated, use SRST extension instead.
pub fn legacy_shutdown() -> ! {
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") eid::LEGACY_SHUTDOWN,
            options(nostack, noreturn)
        );
    }
}

// ============================================================================
// System Reset Extension
// ============================================================================

/// Reset type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ResetType {
    /// Shutdown
    Shutdown = 0x0000_0000,
    /// Cold reboot
    ColdReboot = 0x0000_0001,
    /// Warm reboot
    WarmReboot = 0x0000_0002,
}

/// Reset reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ResetReason {
    /// No reason
    NoReason = 0x0000_0000,
    /// System failure
    SystemFailure = 0x0000_0001,
}

/// Perform a system reset
pub fn system_reset(reset_type: ResetType, reset_reason: ResetReason) -> ! {
    base::sbi_call_2(eid::SRST, srst_fid::SYSTEM_RESET, reset_type as usize, reset_reason as usize);

    // Should not return, but just in case
    loop {
        unsafe { core::arch::asm!("wfi", options(nomem, nostack)) };
    }
}

/// Shutdown the system
pub fn shutdown() -> ! {
    system_reset(ResetType::Shutdown, ResetReason::NoReason)
}

/// Reboot the system (cold)
pub fn reboot() -> ! {
    system_reset(ResetType::ColdReboot, ResetReason::NoReason)
}

// ============================================================================
// Debug Console Extension
// ============================================================================

/// Write to debug console
pub fn debug_console_write(bytes: &[u8]) -> Result<usize, SbiError> {
    let ret = base::sbi_call_3(
        eid::DBCN,
        0, // write
        bytes.len(),
        bytes.as_ptr() as usize,
        0,
    );

    if ret.error == 0 {
        Ok(ret.value as usize)
    } else {
        Err(SbiError::from_raw(ret.error))
    }
}

/// Write a byte to debug console
pub fn debug_console_write_byte(byte: u8) -> Result<(), SbiError> {
    let ret = base::sbi_call_1(eid::DBCN, 2, byte as usize);
    SbiError::from_raw(ret.error).as_result()
}

/// Read from debug console
pub fn debug_console_read(buf: &mut [u8]) -> Result<usize, SbiError> {
    let ret = base::sbi_call_3(
        eid::DBCN,
        1, // read
        buf.len(),
        buf.as_mut_ptr() as usize,
        0,
    );

    if ret.error == 0 {
        Ok(ret.value as usize)
    } else {
        Err(SbiError::from_raw(ret.error))
    }
}

// ============================================================================
// Initialization
// ============================================================================

use core::sync::atomic::{AtomicBool, Ordering};

/// SBI initialized flag
static SBI_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Cached SBI info
static mut SBI_INFO: Option<SbiInfo> = None;

/// Initialize SBI interface
pub fn init() {
    if SBI_INITIALIZED.load(Ordering::Acquire) {
        return;
    }

    // Query SBI information
    let info = SbiInfo::query();

    unsafe {
        SBI_INFO = Some(info);
    }

    SBI_INITIALIZED.store(true, Ordering::Release);
}

/// Get SBI information
pub fn get_info() -> Option<&'static SbiInfo> {
    if !SBI_INITIALIZED.load(Ordering::Acquire) {
        return None;
    }
    unsafe { SBI_INFO.as_ref() }
}

/// Check if SBI is initialized
pub fn is_initialized() -> bool {
    SBI_INITIALIZED.load(Ordering::Acquire)
}
