//! # RISC-V Privilege Modes
//!
//! This module defines the three RISC-V privilege modes and provides
//! utilities for mode transitions.
//!
//! ## Privilege Modes
//!
//! ```text
//! +-------+--------+-----+------------------------------------------+
//! | Level | Name   | Abb | Description                              |
//! +-------+--------+-----+------------------------------------------+
//! |   0   | User   |  U  | Unprivileged application code            |
//! |   1   | Super  |  S  | Operating system kernel                  |
//! |   2   | (Hyp)  |  H  | Hypervisor (H extension, optional)       |
//! |   3   | Machine|  M  | Firmware / Bare metal                    |
//! +-------+--------+-----+------------------------------------------+
//! ```
//!
//! ## Mode Transitions
//!
//! - **ECALL**: U→S→M (exception, goes to higher privilege)
//! - **xRET**: Returns to previous mode (M→S→U)
//! - **Trap**: Enters higher privilege mode on exception/interrupt

use super::super::core::csr::{self, status};

// ============================================================================
// Privilege Mode Definitions
// ============================================================================

/// RISC-V privilege modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum PrivilegeMode {
    /// User mode (unprivileged)
    User = 0,
    /// Supervisor mode (kernel)
    Supervisor = 1,
    /// Hypervisor mode (H extension)
    #[allow(dead_code)]
    Hypervisor = 2,
    /// Machine mode (firmware)
    Machine = 3,
}

impl PrivilegeMode {
    /// Create from raw value
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::User),
            1 => Some(Self::Supervisor),
            2 => Some(Self::Hypervisor),
            3 => Some(Self::Machine),
            _ => None,
        }
    }

    /// Convert to raw value
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Check if this mode is privileged (S or above)
    pub const fn is_privileged(self) -> bool {
        matches!(self, Self::Supervisor | Self::Hypervisor | Self::Machine)
    }

    /// Check if this mode can access supervisor resources
    pub const fn can_access_supervisor(self) -> bool {
        matches!(self, Self::Supervisor | Self::Hypervisor | Self::Machine)
    }

    /// Check if this mode can access machine resources
    pub const fn can_access_machine(self) -> bool {
        matches!(self, Self::Machine)
    }

    /// Get human-readable name
    pub const fn name(self) -> &'static str {
        match self {
            Self::User => "User",
            Self::Supervisor => "Supervisor",
            Self::Hypervisor => "Hypervisor",
            Self::Machine => "Machine",
        }
    }

    /// Get short name
    pub const fn short_name(self) -> &'static str {
        match self {
            Self::User => "U",
            Self::Supervisor => "S",
            Self::Hypervisor => "H",
            Self::Machine => "M",
        }
    }
}

impl Default for PrivilegeMode {
    fn default() -> Self {
        Self::User
    }
}

// ============================================================================
// Mode Detection
// ============================================================================

/// Get current privilege mode
///
/// Note: This can only reliably detect S vs U mode from S-mode.
/// We cannot read M-mode state from S-mode.
///
/// Returns Supervisor if we're in S-mode, or User if SPP indicates
/// we came from U-mode.
pub fn get_current_mode() -> PrivilegeMode {
    // We're always in S-mode in the kernel
    // Check SPP to see what mode was active before the last trap
    PrivilegeMode::Supervisor
}

/// Get the previous privilege mode (from sstatus.SPP)
pub fn get_previous_mode() -> PrivilegeMode {
    let sstatus = csr::read_sstatus();
    if sstatus & status::SPP != 0 {
        PrivilegeMode::Supervisor
    } else {
        PrivilegeMode::User
    }
}

/// Set the return privilege mode (sstatus.SPP)
pub fn set_return_mode(mode: PrivilegeMode) {
    match mode {
        PrivilegeMode::User => {
            csr::clear_sstatus(status::SPP);
        }
        PrivilegeMode::Supervisor => {
            csr::set_sstatus(status::SPP);
        }
        _ => {
            // Can only return to U or S from S-mode
            panic!("Cannot return to {:?} from S-mode", mode);
        }
    }
}

// ============================================================================
// Mode Transitions
// ============================================================================

/// Prepare for return to user mode
///
/// Sets up sstatus for returning to user mode via SRET.
pub fn prepare_return_to_user() {
    // Clear SPP (return to U-mode)
    csr::clear_sstatus(status::SPP);
    // Set SPIE (enable interrupts on return)
    csr::set_sstatus(status::SPIE);
}

/// Prepare for return to supervisor mode
///
/// Sets up sstatus for returning to supervisor mode via SRET.
pub fn prepare_return_to_supervisor(interrupts_enabled: bool) {
    // Set SPP (return to S-mode)
    csr::set_sstatus(status::SPP);
    if interrupts_enabled {
        csr::set_sstatus(status::SPIE);
    } else {
        csr::clear_sstatus(status::SPIE);
    }
}

/// Execute SRET instruction to return from trap
///
/// # Safety
/// - sepc must be set to a valid return address
/// - sstatus must be properly configured
/// - Stack and registers must be properly restored
#[inline(never)]
pub unsafe fn sret() -> ! {
    core::arch::asm!("sret", options(noreturn));
}

/// Execute ECALL instruction to call into higher privilege
///
/// Returns the error code from a0.
#[inline]
pub fn ecall() -> i64 {
    let error: i64;
    unsafe {
        core::arch::asm!(
            "ecall",
            lateout("a0") error,
            options(nostack)
        );
    }
    error
}

// ============================================================================
// Status Register Helpers
// ============================================================================

/// Floating-point extension state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExtensionState {
    /// Extension is off (unusable)
    Off = 0,
    /// Extension is in initial state
    Initial = 1,
    /// Extension state is clean (not modified)
    Clean = 2,
    /// Extension state is dirty (modified)
    Dirty = 3,
}

impl ExtensionState {
    pub const fn from_u8(value: u8) -> Self {
        match value & 0b11 {
            0 => Self::Off,
            1 => Self::Initial,
            2 => Self::Clean,
            _ => Self::Dirty,
        }
    }
}

/// Get floating-point unit state
pub fn get_fp_state() -> ExtensionState {
    let sstatus = csr::read_sstatus();
    let fs = ((sstatus & status::FS_MASK) >> status::FS_SHIFT) as u8;
    ExtensionState::from_u8(fs)
}

/// Set floating-point unit state
pub fn set_fp_state(state: ExtensionState) {
    let sstatus = csr::read_sstatus();
    let new_sstatus = (sstatus & !status::FS_MASK) | ((state as u64) << status::FS_SHIFT);
    csr::write_sstatus(new_sstatus);
}

/// Enable floating-point unit
pub fn enable_fp() {
    set_fp_state(ExtensionState::Initial);
}

/// Disable floating-point unit
pub fn disable_fp() {
    set_fp_state(ExtensionState::Off);
}

/// Get vector extension state (V extension)
#[cfg(feature = "vector")]
pub fn get_vector_state() -> ExtensionState {
    let sstatus = csr::read_sstatus();
    let vs = ((sstatus & status::VS_MASK) >> status::VS_SHIFT) as u8;
    ExtensionState::from_u8(vs)
}

/// Set vector extension state
#[cfg(feature = "vector")]
pub fn set_vector_state(state: ExtensionState) {
    let sstatus = csr::read_sstatus();
    let new_sstatus = (sstatus & !status::VS_MASK) | ((state as u64) << status::VS_SHIFT);
    csr::write_sstatus(new_sstatus);
}

// ============================================================================
// Memory Access Control
// ============================================================================

/// Enable Supervisor User Memory access (SUM)
///
/// When set, S-mode can access U-mode memory pages.
/// This should be set temporarily when copying to/from user space.
pub fn enable_sum() {
    csr::set_sstatus(status::SUM);
}

/// Disable Supervisor User Memory access
pub fn disable_sum() {
    csr::clear_sstatus(status::SUM);
}

/// Check if SUM is enabled
pub fn is_sum_enabled() -> bool {
    csr::read_sstatus() & status::SUM != 0
}

/// Execute a closure with SUM enabled
///
/// SUM is restored to its previous state after the closure returns.
pub fn with_sum<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let was_enabled = is_sum_enabled();
    enable_sum();
    let result = f();
    if !was_enabled {
        disable_sum();
    }
    result
}

/// Enable Make eXecutable Readable (MXR)
///
/// When set, loads from pages marked executable are permitted.
pub fn enable_mxr() {
    csr::set_sstatus(status::MXR);
}

/// Disable Make eXecutable Readable
pub fn disable_mxr() {
    csr::clear_sstatus(status::MXR);
}

// ============================================================================
// Interrupt Control in Status Register
// ============================================================================

/// Check if supervisor interrupts are enabled (sstatus.SIE)
pub fn interrupts_enabled() -> bool {
    csr::interrupts_enabled()
}

/// Enable supervisor interrupts
pub fn enable_interrupts() {
    csr::enable_interrupts();
}

/// Disable supervisor interrupts
pub fn disable_interrupts() {
    csr::disable_interrupts();
}

/// Disable interrupts and return previous state
pub fn local_irq_save() -> bool {
    csr::disable_interrupts_save()
}

/// Restore interrupt state
pub fn local_irq_restore(was_enabled: bool) {
    csr::restore_interrupts(was_enabled);
}

/// Execute closure with interrupts disabled
pub fn with_interrupts_disabled<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let was_enabled = local_irq_save();
    let result = f();
    local_irq_restore(was_enabled);
    result
}

// ============================================================================
// Trap Configuration
// ============================================================================

/// Configure sstatus for initial kernel entry
pub fn init_supervisor_mode() {
    // Start with interrupts disabled
    csr::clear_sstatus(status::SIE);

    // Disable FPU initially (enable lazily on first use)
    set_fp_state(ExtensionState::Off);

    // Don't allow S-mode to access U-mode memory by default
    csr::clear_sstatus(status::SUM);

    // Don't make executable pages readable by default
    csr::clear_sstatus(status::MXR);
}

/// Get the complete sstatus value
pub fn get_sstatus() -> u64 {
    csr::read_sstatus()
}

/// Set the complete sstatus value
pub fn set_sstatus(value: u64) {
    csr::write_sstatus(value);
}
