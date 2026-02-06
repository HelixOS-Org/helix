//! # RISC-V SBI (Supervisor Binary Interface)
//!
//! Interface to SBI firmware for supervisor mode operations.

use core::sync::atomic::{AtomicU64, Ordering};

use super::*;
use crate::core::BootContext;
use crate::error::BootResult;

// =============================================================================
// SBI EXTENSIONS
// =============================================================================

/// Legacy extensions (deprecated in SBI v0.2+)
pub mod legacy {
    pub const SET_TIMER: u64 = 0x0;
    pub const CONSOLE_PUTCHAR: u64 = 0x1;
    pub const CONSOLE_GETCHAR: u64 = 0x2;
    pub const CLEAR_IPI: u64 = 0x3;
    pub const SEND_IPI: u64 = 0x4;
    pub const REMOTE_FENCE_I: u64 = 0x5;
    pub const REMOTE_SFENCE_VMA: u64 = 0x6;
    pub const REMOTE_SFENCE_VMA_ASID: u64 = 0x7;
    pub const SHUTDOWN: u64 = 0x8;
}

/// Base extension (EID = 0x10)
pub const EXT_BASE: u64 = 0x10;
/// Timer extension (EID = 0x54494D45, "TIME")
pub const EXT_TIME: u64 = 0x54494D45;
/// IPI extension (EID = 0x735049, "sPI")
pub const EXT_IPI: u64 = 0x735049;
/// RFENCE extension (EID = 0x52464E43, "RFNC")
pub const EXT_RFENCE: u64 = 0x52464E43;
/// Hart State Management (EID = 0x48534D, "HSM")
pub const EXT_HSM: u64 = 0x48534D;
/// System Reset (EID = 0x53525354, "SRST")
pub const EXT_SRST: u64 = 0x53525354;
/// PMU extension (EID = 0x504D55, "PMU")
pub const EXT_PMU: u64 = 0x504D55;
/// Debug Console (EID = 0x4442434E, "DBCN")
pub const EXT_DBCN: u64 = 0x4442434E;
/// SUSP extension (EID = 0x53555350, "SUSP")
pub const EXT_SUSP: u64 = 0x53555350;
/// CPPC extension (EID = 0x43505043, "CPPC")
pub const EXT_CPPC: u64 = 0x43505043;

// =============================================================================
// BASE EXTENSION FUNCTION IDs
// =============================================================================

pub mod base {
    pub const GET_SPEC_VERSION: u64 = 0;
    pub const GET_IMPL_ID: u64 = 1;
    pub const GET_IMPL_VERSION: u64 = 2;
    pub const PROBE_EXT: u64 = 3;
    pub const GET_MVENDORID: u64 = 4;
    pub const GET_MARCHID: u64 = 5;
    pub const GET_MIMPID: u64 = 6;
}

// =============================================================================
// HSM EXTENSION FUNCTION IDs
// =============================================================================

pub mod hsm {
    pub const HART_START: u64 = 0;
    pub const HART_STOP: u64 = 1;
    pub const HART_GET_STATUS: u64 = 2;
    pub const HART_SUSPEND: u64 = 3;
}

/// Hart states
pub mod hart_state {
    pub const STARTED: u64 = 0;
    pub const STOPPED: u64 = 1;
    pub const START_PENDING: u64 = 2;
    pub const STOP_PENDING: u64 = 3;
    pub const SUSPENDED: u64 = 4;
    pub const SUSPEND_PENDING: u64 = 5;
    pub const RESUME_PENDING: u64 = 6;
}

// =============================================================================
// SRST EXTENSION FUNCTION IDs
// =============================================================================

pub mod srst {
    pub const SYSTEM_RESET: u64 = 0;

    /// Reset types
    pub const RESET_TYPE_SHUTDOWN: u32 = 0;
    pub const RESET_TYPE_COLD_REBOOT: u32 = 1;
    pub const RESET_TYPE_WARM_REBOOT: u32 = 2;

    /// Reset reasons
    pub const RESET_REASON_NONE: u32 = 0;
    pub const RESET_REASON_SYSTEM_FAILURE: u32 = 1;
}

// =============================================================================
// SBI RETURN VALUES
// =============================================================================

/// SBI return error codes
pub mod error {
    pub const SUCCESS: i64 = 0;
    pub const ERR_FAILED: i64 = -1;
    pub const ERR_NOT_SUPPORTED: i64 = -2;
    pub const ERR_INVALID_PARAM: i64 = -3;
    pub const ERR_DENIED: i64 = -4;
    pub const ERR_INVALID_ADDRESS: i64 = -5;
    pub const ERR_ALREADY_AVAILABLE: i64 = -6;
    pub const ERR_ALREADY_STARTED: i64 = -7;
    pub const ERR_ALREADY_STOPPED: i64 = -8;
}

// =============================================================================
// SBI CALL
// =============================================================================

/// SBI return structure
#[derive(Debug, Clone, Copy)]
pub struct SbiRet {
    pub error: i64,
    pub value: u64,
}

impl SbiRet {
    /// Check if call succeeded
    pub fn is_ok(&self) -> bool {
        self.error == error::SUCCESS
    }

    /// Check if call failed
    pub fn is_err(&self) -> bool {
        self.error != error::SUCCESS
    }
}

/// Make SBI call (new calling convention)
#[inline]
pub fn sbi_call(
    ext: u64,
    fid: u64,
    arg0: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
) -> SbiRet {
    let error: i64;
    let value: u64;

    unsafe {
        core::arch::asm!(
            "ecall",
            in("a0") arg0,
            in("a1") arg1,
            in("a2") arg2,
            in("a3") arg3,
            in("a4") arg4,
            in("a5") arg5,
            in("a6") fid,
            in("a7") ext,
            lateout("a0") error,
            lateout("a1") value,
        );
    }

    SbiRet { error, value }
}

/// Make SBI call with 0 arguments
#[inline]
pub fn sbi_call_0(ext: u64, fid: u64) -> SbiRet {
    sbi_call(ext, fid, 0, 0, 0, 0, 0, 0)
}

/// Make SBI call with 1 argument
#[inline]
pub fn sbi_call_1(ext: u64, fid: u64, arg0: u64) -> SbiRet {
    sbi_call(ext, fid, arg0, 0, 0, 0, 0, 0)
}

/// Make SBI call with 2 arguments
#[inline]
pub fn sbi_call_2(ext: u64, fid: u64, arg0: u64, arg1: u64) -> SbiRet {
    sbi_call(ext, fid, arg0, arg1, 0, 0, 0, 0)
}

/// Make SBI call with 3 arguments
#[inline]
pub fn sbi_call_3(ext: u64, fid: u64, arg0: u64, arg1: u64, arg2: u64) -> SbiRet {
    sbi_call(ext, fid, arg0, arg1, arg2, 0, 0, 0)
}

/// Make legacy SBI call
#[inline]
pub fn sbi_legacy_call(ext: u64, arg0: u64) -> i64 {
    let ret: i64;
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a0") arg0,
            in("a7") ext,
            lateout("a0") ret,
        );
    }
    ret
}

// =============================================================================
// BASE EXTENSION
// =============================================================================

/// SBI version
static SBI_VERSION: AtomicU64 = AtomicU64::new(0);
/// SBI implementation ID
static SBI_IMPL_ID: AtomicU64 = AtomicU64::new(0);

/// Get SBI specification version
pub fn get_spec_version() -> (u32, u32) {
    let ret = sbi_call_0(EXT_BASE, base::GET_SPEC_VERSION);
    if ret.is_ok() {
        let major = ((ret.value >> 24) & 0x7F) as u32;
        let minor = (ret.value & 0xFFFFFF) as u32;
        (major, minor)
    } else {
        (0, 1) // Assume legacy SBI
    }
}

/// Get SBI implementation ID
pub fn get_impl_id() -> u64 {
    let ret = sbi_call_0(EXT_BASE, base::GET_IMPL_ID);
    if ret.is_ok() {
        ret.value
    } else {
        0
    }
}

/// Get SBI implementation version
pub fn get_impl_version() -> u64 {
    let ret = sbi_call_0(EXT_BASE, base::GET_IMPL_VERSION);
    if ret.is_ok() {
        ret.value
    } else {
        0
    }
}

/// Probe if extension is available
pub fn probe_extension(ext: u64) -> bool {
    let ret = sbi_call_1(EXT_BASE, base::PROBE_EXT, ext);
    ret.is_ok() && ret.value != 0
}

/// Get machine vendor ID
pub fn get_mvendorid_sbi() -> u64 {
    let ret = sbi_call_0(EXT_BASE, base::GET_MVENDORID);
    if ret.is_ok() {
        ret.value
    } else {
        0
    }
}

/// Get machine architecture ID
pub fn get_marchid_sbi() -> u64 {
    let ret = sbi_call_0(EXT_BASE, base::GET_MARCHID);
    if ret.is_ok() {
        ret.value
    } else {
        0
    }
}

/// Get machine implementation ID
pub fn get_mimpid_sbi() -> u64 {
    let ret = sbi_call_0(EXT_BASE, base::GET_MIMPID);
    if ret.is_ok() {
        ret.value
    } else {
        0
    }
}

// =============================================================================
// TIMER EXTENSION
// =============================================================================

/// Set timer (TIME extension)
pub fn set_timer(stime_value: u64) -> SbiRet {
    if probe_extension(EXT_TIME) {
        sbi_call_1(EXT_TIME, 0, stime_value)
    } else {
        // Legacy call
        sbi_legacy_call(legacy::SET_TIMER, stime_value);
        SbiRet { error: 0, value: 0 }
    }
}

// =============================================================================
// IPI EXTENSION
// =============================================================================

/// Send IPI to hart mask
pub fn send_ipi(hart_mask: u64, hart_mask_base: u64) -> SbiRet {
    if probe_extension(EXT_IPI) {
        sbi_call_2(EXT_IPI, 0, hart_mask, hart_mask_base)
    } else {
        // Legacy call (expects pointer to hart mask)
        sbi_legacy_call(legacy::SEND_IPI, &hart_mask as *const u64 as u64);
        SbiRet { error: 0, value: 0 }
    }
}

// =============================================================================
// RFENCE EXTENSION
// =============================================================================

pub mod rfence {
    use super::*;

    pub const REMOTE_FENCE_I: u64 = 0;
    pub const REMOTE_SFENCE_VMA: u64 = 1;
    pub const REMOTE_SFENCE_VMA_ASID: u64 = 2;
    pub const REMOTE_HFENCE_GVMA_VMID: u64 = 3;
    pub const REMOTE_HFENCE_GVMA: u64 = 4;
    pub const REMOTE_HFENCE_VVMA_ASID: u64 = 5;
    pub const REMOTE_HFENCE_VVMA: u64 = 6;

    /// Remote fence.i
    pub fn fence_i(hart_mask: u64, hart_mask_base: u64) -> SbiRet {
        if probe_extension(EXT_RFENCE) {
            sbi_call_2(EXT_RFENCE, REMOTE_FENCE_I, hart_mask, hart_mask_base)
        } else {
            sbi_legacy_call(legacy::REMOTE_FENCE_I, &hart_mask as *const u64 as u64);
            SbiRet { error: 0, value: 0 }
        }
    }

    /// Remote sfence.vma
    pub fn sfence_vma(hart_mask: u64, hart_mask_base: u64, start_addr: u64, size: u64) -> SbiRet {
        if probe_extension(EXT_RFENCE) {
            sbi_call(
                EXT_RFENCE,
                REMOTE_SFENCE_VMA,
                hart_mask,
                hart_mask_base,
                start_addr,
                size,
                0,
                0,
            )
        } else {
            sbi_legacy_call(legacy::REMOTE_SFENCE_VMA, &hart_mask as *const u64 as u64);
            SbiRet { error: 0, value: 0 }
        }
    }

    /// Remote sfence.vma with ASID
    pub fn sfence_vma_asid(
        hart_mask: u64,
        hart_mask_base: u64,
        start_addr: u64,
        size: u64,
        asid: u64,
    ) -> SbiRet {
        if probe_extension(EXT_RFENCE) {
            sbi_call(
                EXT_RFENCE,
                REMOTE_SFENCE_VMA_ASID,
                hart_mask,
                hart_mask_base,
                start_addr,
                size,
                asid,
                0,
            )
        } else {
            sbi_legacy_call(
                legacy::REMOTE_SFENCE_VMA_ASID,
                &hart_mask as *const u64 as u64,
            );
            SbiRet { error: 0, value: 0 }
        }
    }
}

// =============================================================================
// HSM EXTENSION (Hart State Management)
// =============================================================================

/// Start a hart
pub fn hart_start(hartid: u64, start_addr: u64, opaque: u64) -> SbiRet {
    sbi_call_3(EXT_HSM, hsm::HART_START, hartid, start_addr, opaque)
}

/// Stop current hart
pub fn hart_stop() -> SbiRet {
    sbi_call_0(EXT_HSM, hsm::HART_STOP)
}

/// Get hart status
pub fn hart_get_status(hartid: u64) -> SbiRet {
    sbi_call_1(EXT_HSM, hsm::HART_GET_STATUS, hartid)
}

/// Suspend hart
pub fn hart_suspend(suspend_type: u32, resume_addr: u64, opaque: u64) -> SbiRet {
    sbi_call_3(
        EXT_HSM,
        hsm::HART_SUSPEND,
        suspend_type as u64,
        resume_addr,
        opaque,
    )
}

// =============================================================================
// SRST EXTENSION (System Reset)
// =============================================================================

/// System reset
pub fn system_reset(reset_type: u32, reset_reason: u32) -> SbiRet {
    sbi_call_2(
        EXT_SRST,
        srst::SYSTEM_RESET,
        reset_type as u64,
        reset_reason as u64,
    )
}

/// Shutdown system
pub fn shutdown() -> ! {
    if probe_extension(EXT_SRST) {
        system_reset(srst::RESET_TYPE_SHUTDOWN, srst::RESET_REASON_NONE);
    } else {
        sbi_legacy_call(legacy::SHUTDOWN, 0);
    }

    // Should not reach here
    loop {
        wfi();
    }
}

/// Reboot system
pub fn reboot() -> ! {
    if probe_extension(EXT_SRST) {
        system_reset(srst::RESET_TYPE_COLD_REBOOT, srst::RESET_REASON_NONE);
    }

    // Fallback - should not reach here
    loop {
        wfi();
    }
}

// =============================================================================
// CONSOLE (Legacy / Debug Console)
// =============================================================================

/// Put character to console (legacy)
pub fn console_putchar(ch: u8) {
    if probe_extension(EXT_DBCN) {
        // Use debug console extension
        sbi_call_3(EXT_DBCN, 0, 1, &ch as *const u8 as u64, 0);
    } else {
        // Legacy call
        sbi_legacy_call(legacy::CONSOLE_PUTCHAR, ch as u64);
    }
}

/// Get character from console (legacy)
pub fn console_getchar() -> Option<u8> {
    let ret = sbi_legacy_call(legacy::CONSOLE_GETCHAR, 0);
    if ret >= 0 {
        Some(ret as u8)
    } else {
        None
    }
}

/// Write string to console
pub fn console_write(s: &str) {
    for byte in s.bytes() {
        console_putchar(byte);
    }
}

/// Write string with newline
pub fn console_writeln(s: &str) {
    console_write(s);
    console_putchar(b'\r');
    console_putchar(b'\n');
}

// =============================================================================
// SMP BOOT
// =============================================================================

/// Boot secondary harts
///
/// # Safety
///
/// The caller must ensure the target CPU exists and the entry point is valid.
pub unsafe fn boot_secondary_harts(start_addr: u64, num_harts: u64) -> u64 {
    let mut started = 0u64;

    if !probe_extension(EXT_HSM) {
        return 0;
    }

    let boot_hartid = get_hart_id();

    for hartid in 0..num_harts {
        if hartid == boot_hartid {
            continue;
        }

        // Check if hart exists and is stopped
        let status = hart_get_status(hartid);
        if status.is_err() {
            continue;
        }

        if status.value == hart_state::STOPPED {
            let ret = hart_start(hartid, start_addr, hartid);
            if ret.is_ok() {
                started += 1;
            }
        }
    }

    started
}

/// Wait for hart to come online
pub fn wait_for_hart(hartid: u64, timeout_us: u64) -> bool {
    let start = super::clint::get_time_us();

    loop {
        let status = hart_get_status(hartid);
        if status.is_ok() && status.value == hart_state::STARTED {
            return true;
        }

        if super::clint::get_time_us() - start > timeout_us {
            return false;
        }

        core::hint::spin_loop();
    }
}

// =============================================================================
// SBI IMPLEMENTATION NAMES
// =============================================================================

/// Get SBI implementation name
pub fn get_impl_name(impl_id: u64) -> &'static str {
    match impl_id {
        0 => "Berkeley Boot Loader (BBL)",
        1 => "OpenSBI",
        2 => "Xvisor",
        3 => "KVM",
        4 => "RustSBI",
        5 => "Diosix",
        6 => "Coffer",
        _ => "Unknown",
    }
}

// =============================================================================
// INITIALIZATION
// =============================================================================

/// Initialize SBI
///
/// # Safety
///
/// The caller must ensure system is in a valid state for initialization.
pub unsafe fn init(ctx: &mut BootContext) -> BootResult<()> {
    // Get SBI version
    let (major, minor) = get_spec_version();
    let version = ((major as u64) << 32) | (minor as u64);
    SBI_VERSION.store(version, Ordering::SeqCst);

    // Get implementation info
    let impl_id = get_impl_id();
    let impl_version = get_impl_version();
    SBI_IMPL_ID.store(impl_id, Ordering::SeqCst);

    // Store in context
    ctx.arch_data.riscv.sbi_version = version;
    ctx.arch_data.riscv.sbi_impl_id = impl_id;
    ctx.arch_data.riscv.sbi_impl_version = impl_version;

    // Check available extensions
    ctx.arch_data.riscv.sbi_ext_time = probe_extension(EXT_TIME);
    ctx.arch_data.riscv.sbi_ext_ipi = probe_extension(EXT_IPI);
    ctx.arch_data.riscv.sbi_ext_rfence = probe_extension(EXT_RFENCE);
    ctx.arch_data.riscv.sbi_ext_hsm = probe_extension(EXT_HSM);
    ctx.arch_data.riscv.sbi_ext_srst = probe_extension(EXT_SRST);

    Ok(())
}
