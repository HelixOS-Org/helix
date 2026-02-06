//! # RISC-V CPU Initialization
//!
//! CPU feature detection and initialization for RISC-V.

use core::sync::atomic::{AtomicU64, Ordering};

use super::*;
use crate::core::BootContext;
use crate::error::{BootError, BootResult};

// =============================================================================
// CPU INFO
// =============================================================================

/// Detected XLEN
static XLEN: AtomicU64 = AtomicU64::new(64);
/// ISA extensions
static ISA_EXTENSIONS: AtomicU64 = AtomicU64::new(0);
/// Number of harts
static NUM_HARTS: AtomicU64 = AtomicU64::new(1);

// =============================================================================
// VENDOR IDs
// =============================================================================

/// No vendor ID
pub const VENDOR_NONE: u64 = 0;
/// SiFive
pub const VENDOR_SIFIVE: u64 = 0x489;
/// Western Digital
pub const VENDOR_WD: u64 = 0x4E4;
/// ANDES
pub const VENDOR_ANDES: u64 = 0x31E;
/// T-HEAD
pub const VENDOR_THEAD: u64 = 0x5B7;

// =============================================================================
// ARCHITECTURE IDs
// =============================================================================

/// SiFive E Series
pub const ARCH_SIFIVE_E: u64 = 0x8000_0001;
/// SiFive S Series
pub const ARCH_SIFIVE_S: u64 = 0x8000_0002;
/// SiFive U Series
pub const ARCH_SIFIVE_U: u64 = 0x8000_0003;

// =============================================================================
// CPU FEATURES
// =============================================================================

/// CPU feature flags
#[derive(Debug, Clone, Copy, Default)]
pub struct CpuFeatures {
    /// XLEN (32 or 64)
    pub xlen: u8,
    /// Atomic extension
    pub has_atomic: bool,
    /// Compressed extension
    pub has_compressed: bool,
    /// Single-precision FP
    pub has_float: bool,
    /// Double-precision FP
    pub has_double: bool,
    /// Multiply/divide
    pub has_mul: bool,
    /// Supervisor mode
    pub has_supervisor: bool,
    /// User mode
    pub has_user: bool,
    /// Vector extension
    pub has_vector: bool,
    /// Hypervisor extension
    pub has_hypervisor: bool,
    /// Bit manipulation
    pub has_bitmanip: bool,
    /// Crypto extension
    pub has_crypto: bool,
    /// User interrupts
    pub has_user_int: bool,
    /// Svpbmt extension
    pub has_svpbmt: bool,
    /// Zicsr extension (implied by I)
    pub has_zicsr: bool,
    /// Zifencei extension
    pub has_zifencei: bool,
    /// Vendor ID
    pub vendor_id: u64,
    /// Architecture ID
    pub arch_id: u64,
    /// Implementation ID
    pub impl_id: u64,
    /// Hart ID
    pub hart_id: u64,
}

/// Detect CPU features from MISA
pub fn detect_features() -> CpuFeatures {
    let misa = read_misa();
    let vendor = read_mvendorid();
    let arch = read_marchid();
    let impid = read_mimpid();
    let hartid = read_mhartid();

    // Get XLEN from MISA[63:62] or MXL
    let xlen = match (misa >> 62) & 3 {
        1 => 32,
        2 => 64,
        3 => 128,
        _ => 64, // Default to 64
    };

    CpuFeatures {
        xlen: xlen as u8,
        has_atomic: misa & ISA_A != 0,
        has_compressed: misa & ISA_C != 0,
        has_float: misa & ISA_F != 0,
        has_double: misa & ISA_D != 0,
        has_mul: misa & ISA_M != 0,
        has_supervisor: misa & ISA_S != 0,
        has_user: misa & ISA_U != 0,
        has_vector: misa & ISA_V != 0,
        has_hypervisor: misa & ISA_H != 0,
        has_bitmanip: misa & ISA_B != 0,
        has_crypto: false, // Not in MISA, check extensions
        has_user_int: misa & ISA_N != 0,
        has_svpbmt: false,  // Need to check extension registers
        has_zicsr: true,    // Implied by I
        has_zifencei: true, // Assumed for now
        vendor_id: vendor,
        arch_id: arch,
        impl_id: impid,
        hart_id: hartid,
    }
}

/// Get ISA string
pub fn get_isa_string(features: &CpuFeatures) -> &'static str {
    match features.xlen {
        32 => {
            if features.has_mul
                && features.has_atomic
                && features.has_float
                && features.has_double
                && features.has_compressed
            {
                "rv32gc"
            } else if features.has_mul && features.has_atomic && features.has_compressed {
                "rv32imac"
            } else if features.has_mul && features.has_atomic {
                "rv32ima"
            } else if features.has_mul {
                "rv32im"
            } else {
                "rv32i"
            }
        },
        64 => {
            if features.has_mul
                && features.has_atomic
                && features.has_float
                && features.has_double
                && features.has_compressed
            {
                "rv64gc"
            } else if features.has_mul && features.has_atomic && features.has_compressed {
                "rv64imac"
            } else if features.has_mul && features.has_atomic {
                "rv64ima"
            } else if features.has_mul {
                "rv64im"
            } else {
                "rv64i"
            }
        },
        _ => "rv???",
    }
}

// =============================================================================
// PMP (Physical Memory Protection)
// =============================================================================

/// PMP configuration bits
pub mod pmp {
    /// Read permission
    pub const R: u8 = 1 << 0;
    /// Write permission
    pub const W: u8 = 1 << 1;
    /// Execute permission
    pub const X: u8 = 1 << 2;
    /// Address matching mode: OFF
    pub const A_OFF: u8 = 0 << 3;
    /// Address matching mode: TOR (Top of Range)
    pub const A_TOR: u8 = 1 << 3;
    /// Address matching mode: NA4 (Naturally aligned 4-byte)
    pub const A_NA4: u8 = 2 << 3;
    /// Address matching mode: NAPOT (Naturally aligned power of 2)
    pub const A_NAPOT: u8 = 3 << 3;
    /// Lock bit
    pub const L: u8 = 1 << 7;
}

/// Read pmpcfg0
#[inline]
fn read_pmpcfg0() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("csrr {}, pmpcfg0", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Write pmpcfg0
#[inline]
fn write_pmpcfg0(value: u64) {
    unsafe {
        core::arch::asm!("csrw pmpcfg0, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read pmpaddr0
#[inline]
fn read_pmpaddr0() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("csrr {}, pmpaddr0", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Write pmpaddr0
#[inline]
fn write_pmpaddr0(value: u64) {
    unsafe {
        core::arch::asm!("csrw pmpaddr0, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Configure PMP entry 0
///
/// # Safety
///
/// The caller must ensure the hardware supports this configuration.
pub unsafe fn configure_pmp0(addr: u64, cfg: u8) {
    // NAPOT address encoding: addr[33:2] = base[33:2] | ((size/2 - 1) >> 2)
    write_pmpaddr0(addr >> 2);

    let pmpcfg = read_pmpcfg0();
    write_pmpcfg0((pmpcfg & !0xFF) | (cfg as u64));
}

/// Allow all memory access from S/U mode (for early boot)
///
/// # Safety
///
/// The caller must ensure all safety invariants are upheld.
pub unsafe fn pmp_allow_all() {
    // Set pmpaddr0 to cover all memory (NAPOT with all 1s = full range)
    write_pmpaddr0(u64::MAX >> 10);

    // Configure as NAPOT with RWX
    let cfg = pmp::R | pmp::W | pmp::X | pmp::A_NAPOT;
    write_pmpcfg0(cfg as u64);
}

// =============================================================================
// MACHINE MODE INITIALIZATION
// =============================================================================

/// Initialize machine mode
///
/// # Safety
///
/// The caller must ensure system is in a valid state for initialization.
pub unsafe fn init_machine_mode() {
    // Setup exception delegation
    // Delegate most exceptions to S-mode
    let medeleg = (1 << EXCEPTION_INST_MISALIGNED)
        | (1 << EXCEPTION_BREAKPOINT)
        | (1 << EXCEPTION_ECALL_U)
        | (1 << EXCEPTION_INST_PAGE_FAULT)
        | (1 << EXCEPTION_LOAD_PAGE_FAULT)
        | (1 << EXCEPTION_STORE_PAGE_FAULT);

    core::arch::asm!("csrw medeleg, {}", in(reg) medeleg, options(nomem, nostack));

    // Delegate interrupts to S-mode
    let mideleg = SIE_SSIE | SIE_STIE | SIE_SEIE;
    core::arch::asm!("csrw mideleg, {}", in(reg) mideleg, options(nomem, nostack));

    // Enable counter access from S-mode
    core::arch::asm!("csrw mcounteren, {}", in(reg) 0xFFFF_FFFFu64, options(nomem, nostack));

    // Setup PMP to allow S-mode access to all memory
    pmp_allow_all();
}

/// Drop from M-mode to S-mode
///
/// # Safety
///
/// The caller must ensure the target exception level is properly configured.
pub unsafe fn drop_to_supervisor(entry: u64, hartid: u64, dtb: u64) -> ! {
    // Set MEPC to entry point
    write_mepc(entry);

    // Setup MSTATUS:
    // - MPP = S-mode (01)
    // - MPIE = 1 (enable interrupts on mret)
    // - MXR = 1 (allow read of execute-only pages)
    // - SUM = 1 (allow supervisor access to user pages)
    let mstatus = read_mstatus();
    let mstatus = (mstatus & !MSTATUS_MPP_MASK) | MSTATUS_MPP_S | MSTATUS_MPIE;
    write_mstatus(mstatus);

    // Set arguments (a0 = hartid, a1 = dtb)
    core::arch::asm!(
        "mv a0, {0}",
        "mv a1, {1}",
        "mret",
        in(reg) hartid,
        in(reg) dtb,
        options(noreturn)
    );
}

// =============================================================================
// SUPERVISOR MODE INITIALIZATION
// =============================================================================

/// Initialize supervisor mode
///
/// # Safety
///
/// The caller must ensure system is in a valid state for initialization.
pub unsafe fn init_supervisor_mode() {
    // Enable counter access from U-mode
    core::arch::asm!("csrw scounteren, {}", in(reg) 0xFFFF_FFFFu64, options(nomem, nostack));

    // Clear SSTATUS.SIE (disable interrupts initially)
    let sstatus = read_sstatus() & !SSTATUS_SIE;
    write_sstatus(sstatus);

    // Clear any pending interrupts
    core::arch::asm!("csrw sip, zero", options(nomem, nostack));
}

// =============================================================================
// TRAP HANDLING
// =============================================================================

/// Trap frame saved on trap entry
#[repr(C)]
pub struct TrapFrame {
    /// General purpose registers x1-x31
    pub regs: [u64; 31],
    /// Supervisor status
    pub sstatus: u64,
    /// Exception PC
    pub sepc: u64,
    /// Trap cause
    pub scause: u64,
    /// Trap value
    pub stval: u64,
}

/// Machine trap vector
#[naked]
pub unsafe extern "C" fn machine_trap_vector() {
    core::arch::asm!(
        // Save registers
        "csrw mscratch, sp",
        "addi sp, sp, -256",
        // Save x1-x31
        "sd x1,   8(sp)",
        "sd x2,  16(sp)",
        "sd x3,  24(sp)",
        "sd x4,  32(sp)",
        "sd x5,  40(sp)",
        "sd x6,  48(sp)",
        "sd x7,  56(sp)",
        "sd x8,  64(sp)",
        "sd x9,  72(sp)",
        "sd x10, 80(sp)",
        "sd x11, 88(sp)",
        "sd x12, 96(sp)",
        "sd x13, 104(sp)",
        "sd x14, 112(sp)",
        "sd x15, 120(sp)",
        "sd x16, 128(sp)",
        "sd x17, 136(sp)",
        "sd x18, 144(sp)",
        "sd x19, 152(sp)",
        "sd x20, 160(sp)",
        "sd x21, 168(sp)",
        "sd x22, 176(sp)",
        "sd x23, 184(sp)",
        "sd x24, 192(sp)",
        "sd x25, 200(sp)",
        "sd x26, 208(sp)",
        "sd x27, 216(sp)",
        "sd x28, 224(sp)",
        "sd x29, 232(sp)",
        "sd x30, 240(sp)",
        "sd x31, 248(sp)",
        // Save CSRs
        "csrr t0, mstatus",
        "sd t0, 0(sp)",
        // Call handler
        "mv a0, sp",
        "call machine_trap_handler",
        // Restore CSRs
        "ld t0, 0(sp)",
        "csrw mstatus, t0",
        // Restore x1-x31
        "ld x1,   8(sp)",
        "ld x2,  16(sp)",
        "ld x3,  24(sp)",
        "ld x4,  32(sp)",
        "ld x5,  40(sp)",
        "ld x6,  48(sp)",
        "ld x7,  56(sp)",
        "ld x8,  64(sp)",
        "ld x9,  72(sp)",
        "ld x10, 80(sp)",
        "ld x11, 88(sp)",
        "ld x12, 96(sp)",
        "ld x13, 104(sp)",
        "ld x14, 112(sp)",
        "ld x15, 120(sp)",
        "ld x16, 128(sp)",
        "ld x17, 136(sp)",
        "ld x18, 144(sp)",
        "ld x19, 152(sp)",
        "ld x20, 160(sp)",
        "ld x21, 168(sp)",
        "ld x22, 176(sp)",
        "ld x23, 184(sp)",
        "ld x24, 192(sp)",
        "ld x25, 200(sp)",
        "ld x26, 208(sp)",
        "ld x27, 216(sp)",
        "ld x28, 224(sp)",
        "ld x29, 232(sp)",
        "ld x30, 240(sp)",
        "ld x31, 248(sp)",
        // Restore sp and return
        "addi sp, sp, 256",
        "mret",
        options(noreturn)
    );
}

/// Supervisor trap vector
#[naked]
pub unsafe extern "C" fn supervisor_trap_vector() {
    core::arch::asm!(
        // Save registers
        "csrw sscratch, sp",
        "addi sp, sp, -256",
        // Save all registers (same as machine mode)
        "sd x1,   8(sp)",
        "sd x2,  16(sp)",
        "sd x3,  24(sp)",
        "sd x4,  32(sp)",
        "sd x5,  40(sp)",
        "sd x6,  48(sp)",
        "sd x7,  56(sp)",
        "sd x8,  64(sp)",
        "sd x9,  72(sp)",
        "sd x10, 80(sp)",
        "sd x11, 88(sp)",
        "sd x12, 96(sp)",
        "sd x13, 104(sp)",
        "sd x14, 112(sp)",
        "sd x15, 120(sp)",
        "sd x16, 128(sp)",
        "sd x17, 136(sp)",
        "sd x18, 144(sp)",
        "sd x19, 152(sp)",
        "sd x20, 160(sp)",
        "sd x21, 168(sp)",
        "sd x22, 176(sp)",
        "sd x23, 184(sp)",
        "sd x24, 192(sp)",
        "sd x25, 200(sp)",
        "sd x26, 208(sp)",
        "sd x27, 216(sp)",
        "sd x28, 224(sp)",
        "sd x29, 232(sp)",
        "sd x30, 240(sp)",
        "sd x31, 248(sp)",
        // Save sstatus
        "csrr t0, sstatus",
        "sd t0, 0(sp)",
        // Call handler
        "mv a0, sp",
        "call supervisor_trap_handler",
        // Restore sstatus
        "ld t0, 0(sp)",
        "csrw sstatus, t0",
        // Restore all registers
        "ld x1,   8(sp)",
        "ld x2,  16(sp)",
        "ld x3,  24(sp)",
        "ld x4,  32(sp)",
        "ld x5,  40(sp)",
        "ld x6,  48(sp)",
        "ld x7,  56(sp)",
        "ld x8,  64(sp)",
        "ld x9,  72(sp)",
        "ld x10, 80(sp)",
        "ld x11, 88(sp)",
        "ld x12, 96(sp)",
        "ld x13, 104(sp)",
        "ld x14, 112(sp)",
        "ld x15, 120(sp)",
        "ld x16, 128(sp)",
        "ld x17, 136(sp)",
        "ld x18, 144(sp)",
        "ld x19, 152(sp)",
        "ld x20, 160(sp)",
        "ld x21, 168(sp)",
        "ld x22, 176(sp)",
        "ld x23, 184(sp)",
        "ld x24, 192(sp)",
        "ld x25, 200(sp)",
        "ld x26, 208(sp)",
        "ld x27, 216(sp)",
        "ld x28, 224(sp)",
        "ld x29, 232(sp)",
        "ld x30, 240(sp)",
        "ld x31, 248(sp)",
        "addi sp, sp, 256",
        "sret",
        options(noreturn)
    );
}

/// Machine trap handler
#[no_mangle]
pub extern "C" fn machine_trap_handler(frame: &mut TrapFrame) {
    let cause = read_mcause();
    let is_interrupt = cause & (1 << 63) != 0;
    let code = cause & 0x7FF;

    if is_interrupt {
        match code {
            7 => {
                // Machine timer interrupt
                // Clear timer pending (done by CLINT)
            },
            11 => {
                // Machine external interrupt
                // Handle PLIC
            },
            3 => {
                // Machine software interrupt
            },
            _ => {
                // Unknown interrupt
            },
        }
    } else {
        // Exception
        match code {
            11 => {
                // ECALL from M-mode - should not happen
            },
            _ => {
                // Other exception - panic
            },
        }
    }
}

/// Supervisor trap handler
#[no_mangle]
pub extern "C" fn supervisor_trap_handler(frame: &mut TrapFrame) {
    let cause = read_scause();
    let is_interrupt = cause & (1 << 63) != 0;
    let code = cause & 0x7FF;

    if is_interrupt {
        match code {
            5 => {
                // Supervisor timer interrupt
            },
            9 => {
                // Supervisor external interrupt
            },
            1 => {
                // Supervisor software interrupt
            },
            _ => {},
        }
    } else {
        // Exception
        match code {
            8 => {
                // ECALL from U-mode (syscall)
            },
            12 => {
                // Instruction page fault
            },
            13 => {
                // Load page fault
            },
            15 => {
                // Store page fault
            },
            _ => {
                // Other exception
            },
        }
    }
}

// =============================================================================
// FPU INITIALIZATION
// =============================================================================

/// Enable FPU
///
/// # Safety
///
/// The caller must ensure the CPU supports these features.
pub unsafe fn enable_fpu() {
    // Set MSTATUS.FS to Initial
    let mstatus = read_mstatus();
    write_mstatus((mstatus & !MSTATUS_FS) | MSTATUS_FS_INITIAL);

    // Write to fcsr to set it to initial state
    core::arch::asm!("fscsr zero", options(nomem, nostack));
}

/// Disable FPU
///
/// # Safety
///
/// The caller must ensure disabling this feature won't cause system instability.
pub unsafe fn disable_fpu() {
    let mstatus = read_mstatus();
    write_mstatus(mstatus & !MSTATUS_FS);
}

// =============================================================================
// VECTOR EXTENSION
// =============================================================================

/// Enable vector extension
///
/// # Safety
///
/// The caller must ensure the system is ready for this feature to be enabled.
pub unsafe fn enable_vector() {
    let mstatus = read_mstatus();
    write_mstatus(mstatus | MSTATUS_VS);
}

/// Disable vector extension
///
/// # Safety
///
/// The caller must ensure disabling this feature won't cause system instability.
pub unsafe fn disable_vector() {
    let mstatus = read_mstatus();
    write_mstatus(mstatus & !MSTATUS_VS);
}

// =============================================================================
// INITIALIZATION
// =============================================================================

/// Initialize CPU
///
/// # Safety
///
/// The caller must ensure system is in a valid state for initialization.
pub unsafe fn init(ctx: &mut BootContext) -> BootResult<()> {
    // Detect features
    let features = detect_features();

    // Store in context
    ctx.arch_data.riscv.hart_id = features.hart_id;
    ctx.arch_data.riscv.vendor_id = features.vendor_id;
    ctx.arch_data.riscv.arch_id = features.arch_id;
    ctx.arch_data.riscv.impl_id = features.impl_id;
    ctx.arch_data.riscv.isa_extensions = read_misa();

    // Store global state
    XLEN.store(features.xlen as u64, Ordering::SeqCst);
    ISA_EXTENSIONS.store(read_misa(), Ordering::SeqCst);

    // Enable FPU if available
    if features.has_float || features.has_double {
        enable_fpu();
    }

    // Enable vector if available
    if features.has_vector {
        enable_vector();
    }

    // Setup trap vectors
    write_mtvec(machine_trap_vector as u64);
    write_stvec(supervisor_trap_vector as u64);

    Ok(())
}
