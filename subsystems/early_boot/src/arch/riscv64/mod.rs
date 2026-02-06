//! # RISC-V Early Boot Implementation
//!
//! Complete early boot sequence for RISC-V architecture.
//! Supports RV64 with Sv39/Sv48/Sv57 paging, PLIC/CLINT, and SBI.

pub mod clint;
pub mod cpu;
pub mod mmu;
pub mod plic;
pub mod sbi;
pub mod serial;

use crate::core::BootContext;
use crate::error::{BootError, BootResult};

// =============================================================================
// CSR ADDRESSES
// =============================================================================

// Machine mode CSRs (0x300-0x3FF)
/// Machine status register
pub const CSR_MSTATUS: u16 = 0x300;
/// Machine ISA
pub const CSR_MISA: u16 = 0x301;
/// Machine exception delegation
pub const CSR_MEDELEG: u16 = 0x302;
/// Machine interrupt delegation
pub const CSR_MIDELEG: u16 = 0x303;
/// Machine interrupt enable
pub const CSR_MIE: u16 = 0x304;
/// Machine trap-handler base address
pub const CSR_MTVEC: u16 = 0x305;
/// Machine counter enable
pub const CSR_MCOUNTEREN: u16 = 0x306;
/// Machine scratch register
pub const CSR_MSCRATCH: u16 = 0x340;
/// Machine exception program counter
pub const CSR_MEPC: u16 = 0x341;
/// Machine trap cause
pub const CSR_MCAUSE: u16 = 0x342;
/// Machine trap value
pub const CSR_MTVAL: u16 = 0x343;
/// Machine interrupt pending
pub const CSR_MIP: u16 = 0x344;
/// Physical memory protection config 0
pub const CSR_PMPCFG0: u16 = 0x3A0;
/// Physical memory protection address 0
pub const CSR_PMPADDR0: u16 = 0x3B0;
/// Machine hart ID
pub const CSR_MHARTID: u16 = 0xF14;
/// Machine vendor ID
pub const CSR_MVENDORID: u16 = 0xF11;
/// Machine architecture ID
pub const CSR_MARCHID: u16 = 0xF12;
/// Machine implementation ID
pub const CSR_MIMPID: u16 = 0xF13;

// Supervisor mode CSRs (0x100-0x1FF)
/// Supervisor status register
pub const CSR_SSTATUS: u16 = 0x100;
/// Supervisor interrupt enable
pub const CSR_SIE: u16 = 0x104;
/// Supervisor trap-handler base address
pub const CSR_STVEC: u16 = 0x105;
/// Supervisor counter enable
pub const CSR_SCOUNTEREN: u16 = 0x106;
/// Supervisor scratch register
pub const CSR_SSCRATCH: u16 = 0x140;
/// Supervisor exception program counter
pub const CSR_SEPC: u16 = 0x141;
/// Supervisor trap cause
pub const CSR_SCAUSE: u16 = 0x142;
/// Supervisor trap value
pub const CSR_STVAL: u16 = 0x143;
/// Supervisor interrupt pending
pub const CSR_SIP: u16 = 0x144;
/// Supervisor address translation and protection
pub const CSR_SATP: u16 = 0x180;

// User mode CSRs
/// User status (if N extension)
pub const CSR_USTATUS: u16 = 0x000;
/// Cycle counter
pub const CSR_CYCLE: u16 = 0xC00;
/// Timer
pub const CSR_TIME: u16 = 0xC01;
/// Instructions retired
pub const CSR_INSTRET: u16 = 0xC02;

// Hypervisor CSRs (0x600-0x6FF)
/// Hypervisor status
pub const CSR_HSTATUS: u16 = 0x600;
/// Hypervisor exception delegation
pub const CSR_HEDELEG: u16 = 0x602;
/// Hypervisor interrupt delegation
pub const CSR_HIDELEG: u16 = 0x603;
/// Hypervisor interrupt enable
pub const CSR_HIE: u16 = 0x604;
/// Hypervisor guest external interrupt pending
pub const CSR_HGEIP: u16 = 0xE12;

// =============================================================================
// STATUS REGISTER BITS
// =============================================================================

/// Supervisor Interrupt Enable
pub const SSTATUS_SIE: u64 = 1 << 1;
/// Machine Interrupt Enable
pub const MSTATUS_MIE: u64 = 1 << 3;
/// Supervisor Previous Interrupt Enable
pub const SSTATUS_SPIE: u64 = 1 << 5;
/// Machine Previous Interrupt Enable
pub const MSTATUS_MPIE: u64 = 1 << 7;
/// Supervisor Previous Privilege
pub const SSTATUS_SPP: u64 = 1 << 8;
/// Vector extension state
pub const MSTATUS_VS: u64 = 3 << 9;
/// Machine Previous Privilege (2 bits)
pub const MSTATUS_MPP_MASK: u64 = 3 << 11;
/// MPP = User mode
pub const MSTATUS_MPP_U: u64 = 0 << 11;
/// MPP = Supervisor mode
pub const MSTATUS_MPP_S: u64 = 1 << 11;
/// MPP = Machine mode
pub const MSTATUS_MPP_M: u64 = 3 << 11;
/// FPU state
pub const MSTATUS_FS: u64 = 3 << 13;
/// FPU state: off
pub const MSTATUS_FS_OFF: u64 = 0 << 13;
/// FPU state: initial
pub const MSTATUS_FS_INITIAL: u64 = 1 << 13;
/// FPU state: clean
pub const MSTATUS_FS_CLEAN: u64 = 2 << 13;
/// FPU state: dirty
pub const MSTATUS_FS_DIRTY: u64 = 3 << 13;
/// Extension state
pub const MSTATUS_XS: u64 = 3 << 15;
/// Modify Privilege
pub const MSTATUS_MPRV: u64 = 1 << 17;
/// Supervisor User Memory
pub const MSTATUS_SUM: u64 = 1 << 18;
/// Make Executable Readable
pub const MSTATUS_MXR: u64 = 1 << 19;
/// Trap Virtual Memory
pub const MSTATUS_TVM: u64 = 1 << 20;
/// Timeout Wait
pub const MSTATUS_TW: u64 = 1 << 21;
/// Trap SRET
pub const MSTATUS_TSR: u64 = 1 << 22;
/// User XLEN (2 bits)
pub const MSTATUS_UXL: u64 = 3 << 32;
/// Supervisor XLEN (2 bits)
pub const MSTATUS_SXL: u64 = 3 << 34;
/// State Dirty
pub const MSTATUS_SD: u64 = 1 << 63;

// =============================================================================
// INTERRUPT/EXCEPTION BITS
// =============================================================================

// Interrupt causes (bit 63 set)
/// Supervisor software interrupt
pub const INTERRUPT_S_SOFT: u64 = 1;
/// Machine software interrupt
pub const INTERRUPT_M_SOFT: u64 = 3;
/// Supervisor timer interrupt
pub const INTERRUPT_S_TIMER: u64 = 5;
/// Machine timer interrupt
pub const INTERRUPT_M_TIMER: u64 = 7;
/// Supervisor external interrupt
pub const INTERRUPT_S_EXT: u64 = 9;
/// Machine external interrupt
pub const INTERRUPT_M_EXT: u64 = 11;

// Interrupt enable/pending bits
/// Supervisor software interrupt
pub const SIE_SSIE: u64 = 1 << 1;
/// Machine software interrupt
pub const MIE_MSIE: u64 = 1 << 3;
/// Supervisor timer interrupt
pub const SIE_STIE: u64 = 1 << 5;
/// Machine timer interrupt
pub const MIE_MTIE: u64 = 1 << 7;
/// Supervisor external interrupt
pub const SIE_SEIE: u64 = 1 << 9;
/// Machine external interrupt
pub const MIE_MEIE: u64 = 1 << 11;

// Exception causes (bit 63 clear)
/// Instruction address misaligned
pub const EXCEPTION_INST_MISALIGNED: u64 = 0;
/// Instruction access fault
pub const EXCEPTION_INST_ACCESS: u64 = 1;
/// Illegal instruction
pub const EXCEPTION_ILLEGAL_INST: u64 = 2;
/// Breakpoint
pub const EXCEPTION_BREAKPOINT: u64 = 3;
/// Load address misaligned
pub const EXCEPTION_LOAD_MISALIGNED: u64 = 4;
/// Load access fault
pub const EXCEPTION_LOAD_ACCESS: u64 = 5;
/// Store/AMO address misaligned
pub const EXCEPTION_STORE_MISALIGNED: u64 = 6;
/// Store/AMO access fault
pub const EXCEPTION_STORE_ACCESS: u64 = 7;
/// Environment call from U-mode
pub const EXCEPTION_ECALL_U: u64 = 8;
/// Environment call from S-mode
pub const EXCEPTION_ECALL_S: u64 = 9;
/// Environment call from M-mode
pub const EXCEPTION_ECALL_M: u64 = 11;
/// Instruction page fault
pub const EXCEPTION_INST_PAGE_FAULT: u64 = 12;
/// Load page fault
pub const EXCEPTION_LOAD_PAGE_FAULT: u64 = 13;
/// Store/AMO page fault
pub const EXCEPTION_STORE_PAGE_FAULT: u64 = 15;

// =============================================================================
// PRIVILEGE LEVELS
// =============================================================================

/// User mode
pub const PRIV_USER: u8 = 0;
/// Supervisor mode
pub const PRIV_SUPERVISOR: u8 = 1;
/// Reserved
pub const PRIV_RESERVED: u8 = 2;
/// Machine mode
pub const PRIV_MACHINE: u8 = 3;

// =============================================================================
// ISA EXTENSIONS
// =============================================================================

/// Atomic extension
pub const ISA_A: u64 = 1 << 0;
/// Bit manipulation
pub const ISA_B: u64 = 1 << 1;
/// Compressed instructions
pub const ISA_C: u64 = 1 << 2;
/// Double-precision float
pub const ISA_D: u64 = 1 << 3;
/// RV32E base
pub const ISA_E: u64 = 1 << 4;
/// Single-precision float
pub const ISA_F: u64 = 1 << 5;
/// Hypervisor
pub const ISA_H: u64 = 1 << 7;
/// Integer base
pub const ISA_I: u64 = 1 << 8;
/// Integer multiply/divide
pub const ISA_M: u64 = 1 << 12;
/// User-level interrupts
pub const ISA_N: u64 = 1 << 13;
/// Packed SIMD
pub const ISA_P: u64 = 1 << 15;
/// Quad-precision float
pub const ISA_Q: u64 = 1 << 16;
/// Supervisor mode
pub const ISA_S: u64 = 1 << 18;
/// User mode
pub const ISA_U: u64 = 1 << 20;
/// Vector extension
pub const ISA_V: u64 = 1 << 21;

// =============================================================================
// SATP MODES
// =============================================================================

/// Bare (no translation)
pub const SATP_MODE_BARE: u64 = 0;
/// Sv39 (39-bit virtual address)
pub const SATP_MODE_SV39: u64 = 8;
/// Sv48 (48-bit virtual address)
pub const SATP_MODE_SV48: u64 = 9;
/// Sv57 (57-bit virtual address)
pub const SATP_MODE_SV57: u64 = 10;
/// Sv64 (reserved)
pub const SATP_MODE_SV64: u64 = 11;

/// SATP mode shift
pub const SATP_MODE_SHIFT: u64 = 60;
/// SATP ASID shift
pub const SATP_ASID_SHIFT: u64 = 44;
/// SATP PPN mask
pub const SATP_PPN_MASK: u64 = (1 << 44) - 1;

// =============================================================================
// PTE FLAGS
// =============================================================================

/// Page table entry: Valid
pub const PTE_V: u64 = 1 << 0;
/// Page table entry: Readable
pub const PTE_R: u64 = 1 << 1;
/// Page table entry: Writable
pub const PTE_W: u64 = 1 << 2;
/// Page table entry: Executable
pub const PTE_X: u64 = 1 << 3;
/// Page table entry: User accessible
pub const PTE_U: u64 = 1 << 4;
/// Page table entry: Global
pub const PTE_G: u64 = 1 << 5;
/// Page table entry: Accessed
pub const PTE_A: u64 = 1 << 6;
/// Page table entry: Dirty
pub const PTE_D: u64 = 1 << 7;
/// RSW (reserved for software)
pub const PTE_RSW_MASK: u64 = 3 << 8;
/// PPN shift
pub const PTE_PPN_SHIFT: u64 = 10;

// =============================================================================
// PAGE SIZES
// =============================================================================

/// 4KB page (standard)
pub const PAGE_SIZE_4K: u64 = 4096;
/// 2MB megapage
pub const PAGE_SIZE_2M: u64 = 2 * 1024 * 1024;
/// 1GB gigapage
pub const PAGE_SIZE_1G: u64 = 1024 * 1024 * 1024;
/// 512GB terapage (Sv57 only)
pub const PAGE_SIZE_512G: u64 = 512 * 1024 * 1024 * 1024;

// =============================================================================
// CSR ACCESS MACROS
// =============================================================================

/// Read CSR
#[macro_export]
macro_rules! read_csr {
    ($csr:expr) => {{
        let value: u64;
        unsafe {
            core::arch::asm!(
                concat!("csrr {}, ", stringify!($csr)),
                out(reg) value,
                options(nomem, nostack)
            );
        }
        value
    }};
}

/// Write CSR
#[macro_export]
macro_rules! write_csr {
    ($csr:expr, $value:expr) => {{
        unsafe {
            core::arch::asm!(
                concat!("csrw ", stringify!($csr), ", {}"),
                in(reg) $value,
                options(nomem, nostack)
            );
        }
    }};
}

/// Set CSR bits
#[macro_export]
macro_rules! set_csr {
    ($csr:expr, $bits:expr) => {{
        unsafe {
            core::arch::asm!(
                concat!("csrs ", stringify!($csr), ", {}"),
                in(reg) $bits,
                options(nomem, nostack)
            );
        }
    }};
}

/// Clear CSR bits
#[macro_export]
macro_rules! clear_csr {
    ($csr:expr, $bits:expr) => {{
        unsafe {
            core::arch::asm!(
                concat!("csrc ", stringify!($csr), ", {}"),
                in(reg) $bits,
                options(nomem, nostack)
            );
        }
    }};
}

// =============================================================================
// CSR ACCESS FUNCTIONS
// =============================================================================

/// Read mstatus
#[inline]
pub fn read_mstatus() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("csrr {}, mstatus", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Write mstatus
#[inline]
pub fn write_mstatus(value: u64) {
    unsafe {
        core::arch::asm!("csrw mstatus, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read sstatus
#[inline]
pub fn read_sstatus() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("csrr {}, sstatus", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Write sstatus
#[inline]
pub fn write_sstatus(value: u64) {
    unsafe {
        core::arch::asm!("csrw sstatus, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read mhartid
#[inline]
pub fn read_mhartid() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("csrr {}, mhartid", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Read misa
#[inline]
pub fn read_misa() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("csrr {}, misa", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Read mvendorid
#[inline]
pub fn read_mvendorid() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("csrr {}, mvendorid", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Read marchid
#[inline]
pub fn read_marchid() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("csrr {}, marchid", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Read mimpid
#[inline]
pub fn read_mimpid() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("csrr {}, mimpid", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Read mepc
#[inline]
pub fn read_mepc() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("csrr {}, mepc", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Write mepc
#[inline]
pub fn write_mepc(value: u64) {
    unsafe {
        core::arch::asm!("csrw mepc, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read sepc
#[inline]
pub fn read_sepc() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("csrr {}, sepc", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Write sepc
#[inline]
pub fn write_sepc(value: u64) {
    unsafe {
        core::arch::asm!("csrw sepc, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read mcause
#[inline]
pub fn read_mcause() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("csrr {}, mcause", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Read scause
#[inline]
pub fn read_scause() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("csrr {}, scause", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Read mtval
#[inline]
pub fn read_mtval() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("csrr {}, mtval", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Read stval
#[inline]
pub fn read_stval() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("csrr {}, stval", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Read mtvec
#[inline]
pub fn read_mtvec() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("csrr {}, mtvec", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Write mtvec
#[inline]
pub fn write_mtvec(value: u64) {
    unsafe {
        core::arch::asm!("csrw mtvec, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read stvec
#[inline]
pub fn read_stvec() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("csrr {}, stvec", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Write stvec
#[inline]
pub fn write_stvec(value: u64) {
    unsafe {
        core::arch::asm!("csrw stvec, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read satp
#[inline]
pub fn read_satp() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("csrr {}, satp", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Write satp
#[inline]
pub fn write_satp(value: u64) {
    unsafe {
        core::arch::asm!("csrw satp, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read mie
#[inline]
pub fn read_mie() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("csrr {}, mie", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Write mie
#[inline]
pub fn write_mie(value: u64) {
    unsafe {
        core::arch::asm!("csrw mie, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read sie
#[inline]
pub fn read_sie() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("csrr {}, sie", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Write sie
#[inline]
pub fn write_sie(value: u64) {
    unsafe {
        core::arch::asm!("csrw sie, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read mip
#[inline]
pub fn read_mip() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("csrr {}, mip", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Read sip
#[inline]
pub fn read_sip() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("csrr {}, sip", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Read time
#[inline]
pub fn read_time() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("csrr {}, time", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Read cycle
#[inline]
pub fn read_cycle() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("csrr {}, cycle", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Read instret
#[inline]
pub fn read_instret() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("csrr {}, instret", out(reg) value, options(nomem, nostack));
    }
    value
}

// =============================================================================
// MEMORY BARRIERS
// =============================================================================

/// Fence (memory ordering)
#[inline]
pub fn fence() {
    unsafe {
        core::arch::asm!("fence", options(nostack));
    }
}

/// Fence.i (instruction fence)
#[inline]
pub fn fence_i() {
    unsafe {
        core::arch::asm!("fence.i", options(nostack));
    }
}

/// Fence iorw,iorw
#[inline]
pub fn fence_iorw() {
    unsafe {
        core::arch::asm!("fence iorw, iorw", options(nostack));
    }
}

/// SFENCE.VMA (TLB flush)
#[inline]
pub fn sfence_vma() {
    unsafe {
        core::arch::asm!("sfence.vma", options(nostack));
    }
}

/// SFENCE.VMA with address
#[inline]
pub fn sfence_vma_addr(addr: u64) {
    unsafe {
        core::arch::asm!("sfence.vma {}, zero", in(reg) addr, options(nostack));
    }
}

/// SFENCE.VMA with address and ASID
#[inline]
pub fn sfence_vma_asid(addr: u64, asid: u64) {
    unsafe {
        core::arch::asm!("sfence.vma {}, {}", in(reg) addr, in(reg) asid, options(nostack));
    }
}

// =============================================================================
// WAIT INSTRUCTIONS
// =============================================================================

/// Wait for interrupt
#[inline]
pub fn wfi() {
    unsafe {
        core::arch::asm!("wfi", options(nomem, nostack));
    }
}

// =============================================================================
// PRIVILEGE MODE TRANSITIONS
// =============================================================================

/// Execute MRET (return from machine mode)
#[inline]
///
/// # Safety
///
/// The caller must ensure proper CSR state for exception return.
pub unsafe fn mret() -> ! {
    core::arch::asm!("mret", options(noreturn));
}

/// Execute SRET (return from supervisor mode)
#[inline]
///
/// # Safety
///
/// The caller must ensure proper CSR state for exception return.
pub unsafe fn sret() -> ! {
    core::arch::asm!("sret", options(noreturn));
}

// =============================================================================
// INTERRUPT CONTROL
// =============================================================================

/// Enable global interrupts (machine mode)
#[inline]
pub fn enable_interrupts_m() {
    let mstatus = read_mstatus();
    write_mstatus(mstatus | MSTATUS_MIE);
}

/// Disable global interrupts (machine mode)
#[inline]
pub fn disable_interrupts_m() {
    let mstatus = read_mstatus();
    write_mstatus(mstatus & !MSTATUS_MIE);
}

/// Enable global interrupts (supervisor mode)
#[inline]
pub fn enable_interrupts_s() {
    let sstatus = read_sstatus();
    write_sstatus(sstatus | SSTATUS_SIE);
}

/// Disable global interrupts (supervisor mode)
#[inline]
pub fn disable_interrupts_s() {
    let sstatus = read_sstatus();
    write_sstatus(sstatus & !SSTATUS_SIE);
}

/// Check if interrupts are enabled (supervisor mode)
#[inline]
pub fn interrupts_enabled_s() -> bool {
    read_sstatus() & SSTATUS_SIE != 0
}

// =============================================================================
// PRIVILEGE MODE DETECTION
// =============================================================================

/// Get current hart ID
#[inline]
pub fn get_hart_id() -> u64 {
    // In S-mode, use SBI or read from memory
    // In M-mode, read mhartid directly
    read_mhartid()
}

// =============================================================================
// BOOT ENTRY POINTS
// =============================================================================

/// Machine mode entry point
///
/// # Safety
///
/// The caller must ensure proper machine state before calling this entry point.
pub unsafe fn machine_entry(hartid: u64, dtb_addr: u64) -> ! {
    // Primary hart setup
    if hartid == 0 {
        // Setup machine mode
        cpu::init_machine_mode();

        // Drop to supervisor mode
        cpu::drop_to_supervisor(supervisor_entry as u64, hartid, dtb_addr);
    } else {
        // Secondary hart - wait
        loop {
            wfi();
        }
    }
}

/// Supervisor mode entry point
extern "C" fn supervisor_entry(hartid: u64, dtb_addr: u64) -> ! {
    // Supervisor mode initialization
    unsafe {
        // TODO: Full supervisor initialization
    }

    loop {
        wfi();
    }
}

/// Early boot initialization
///
/// # Safety
///
/// The caller must ensure system is in a valid state for initialization.
pub unsafe fn init(ctx: &mut BootContext) -> BootResult<()> {
    // Initialize CPU
    cpu::init(ctx)?;

    // Setup MMU
    mmu::init(ctx)?;

    // Initialize PLIC
    plic::init(ctx)?;

    // Initialize CLINT
    clint::init(ctx)?;

    // Initialize serial
    serial::init(ctx)?;

    Ok(())
}
