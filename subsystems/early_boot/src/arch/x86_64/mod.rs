//! # x86_64 Early Boot Implementation
//!
//! Complete x86_64 early boot sequence implementation including:
//! - GDT/TSS setup
//! - IDT and exception handlers
//! - Paging (4-level and 5-level)
//! - APIC/IOAPIC initialization
//! - Timer calibration (TSC, HPET, APIC)
//! - SMP startup (INIT-SIPI-SIPI)
//!
//! ## x86_64 Boot Flow
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                        x86_64 EARLY BOOT FLOW                                │
//! │                                                                              │
//! │   ┌──────────────┐                                                          │
//! │   │ From         │                                                          │
//! │   │ Bootloader   │                                                          │
//! │   └──────┬───────┘                                                          │
//! │          │                                                                   │
//! │          ▼                                                                   │
//! │   ┌──────────────┐   Already in 64-bit mode, paging enabled                │
//! │   │  Pre-Init    │   - Validate long mode                                   │
//! │   │              │   - Set up temporary stack                               │
//! │   │              │   - Init serial (COM1)                                   │
//! │   └──────┬───────┘                                                          │
//! │          │                                                                   │
//! │          ▼                                                                   │
//! │   ┌──────────────┐                                                          │
//! │   │  CPU Init    │   - CPUID feature detection                              │
//! │   │              │   - Enable: SSE, AVX, XSAVE, FSGSBASE                    │
//! │   │              │   - Set up CR0, CR4, EFER                                │
//! │   │              │   - Initialize GDT with TSS                              │
//! │   └──────┬───────┘                                                          │
//! │          │                                                                   │
//! │          ▼                                                                   │
//! │   ┌──────────────┐                                                          │
//! │   │ Memory Init  │   - Process E820/UEFI memory map                        │
//! │   │              │   - Build new page tables (4 or 5 level)                 │
//! │   │              │   - Map kernel, HHDM, APIC, I/O                          │
//! │   │              │   - Switch to new page tables                            │
//! │   └──────┬───────┘                                                          │
//! │          │                                                                   │
//! │          ▼                                                                   │
//! │   ┌──────────────┐                                                          │
//! │   │  Interrupt   │   - Set up IDT (256 entries)                             │
//! │   │  Init        │   - Install exception handlers                           │
//! │   │              │   - Initialize Local APIC (xAPIC or x2APIC)              │
//! │   │              │   - Configure I/O APIC                                    │
//! │   └──────┬───────┘                                                          │
//! │          │                                                                   │
//! │          ▼                                                                   │
//! │   ┌──────────────┐                                                          │
//! │   │ Timer Init   │   - Detect TSC frequency (CPUID or calibration)          │
//! │   │              │   - Initialize HPET if available                         │
//! │   │              │   - Configure APIC timer                                 │
//! │   │              │   - Set up tick rate                                     │
//! │   └──────┬───────┘                                                          │
//! │          │                                                                   │
//! │          ▼                                                                   │
//! │   ┌──────────────┐                                                          │
//! │   │  SMP Init    │   - Parse MADT for CPU info                              │
//! │   │              │   - Allocate per-CPU stacks                              │
//! │   │              │   - Send INIT-SIPI-SIPI to APs                           │
//! │   │              │   - Wait for AP startup                                  │
//! │   └──────┬───────┘                                                          │
//! │          │                                                                   │
//! │          ▼                                                                   │
//! │   ┌──────────────┐                                                          │
//! │   │  Handoff     │   - Apply KASLR (if enabled)                             │
//! │   │              │   - Final memory layout                                   │
//! │   │              │   - Jump to kernel entry                                  │
//! │   └──────────────┘                                                          │
//! │                                                                              │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```

use crate::core::{
    BootContext, CpuFeatures, CpuState, InterruptControllerType, PagingMode, SmpStartupMethod,
    TimerType,
};
use crate::error::{BootError, BootResult};
use crate::SerialConfig;

// =============================================================================
// SUBMODULES
// =============================================================================

mod apic;
mod cpu;
mod gdt;
mod idt;
mod paging;
mod serial;
mod smp;
mod timers;

pub use apic::*;
pub use cpu::*;
pub use gdt::*;
pub use idt::*;
pub use paging::*;
pub use serial::*;
pub use smp::*;
pub use timers::*;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Kernel code segment selector
pub const KERNEL_CS: u16 = 0x08;
/// Kernel data segment selector
pub const KERNEL_DS: u16 = 0x10;
/// User code segment selector (32-bit compat)
pub const USER_CS32: u16 = 0x18;
/// User data segment selector
pub const USER_DS: u16 = 0x20;
/// User code segment selector (64-bit)
pub const USER_CS: u16 = 0x28;
/// TSS segment selector
pub const TSS_SEL: u16 = 0x30;

/// Local APIC base address (default)
pub const LAPIC_BASE: u64 = 0xFEE0_0000;

/// I/O APIC base address (default)
pub const IOAPIC_BASE: u64 = 0xFEC0_0000;

/// Page size
pub const PAGE_SIZE: usize = 4096;
/// Large page size (2MB)
pub const LARGE_PAGE_SIZE: usize = 2 * 1024 * 1024;
/// Huge page size (1GB)
pub const HUGE_PAGE_SIZE: usize = 1024 * 1024 * 1024;

/// HHDM base address (higher half direct map)
pub const HHDM_BASE: u64 = 0xFFFF_8000_0000_0000;

/// Kernel base address
pub const KERNEL_BASE: u64 = 0xFFFF_FFFF_8000_0000;

// =============================================================================
// MSR ADDRESSES
// =============================================================================

/// Extended Feature Enable Register
pub const MSR_EFER: u32 = 0xC000_0080;
/// FS base address
pub const MSR_FS_BASE: u32 = 0xC000_0100;
/// GS base address
pub const MSR_GS_BASE: u32 = 0xC000_0101;
/// Kernel GS base (for SWAPGS)
pub const MSR_KERNEL_GS_BASE: u32 = 0xC000_0102;
/// APIC base address
pub const MSR_APIC_BASE: u32 = 0x0000_001B;
/// TSC frequency (if available)
pub const MSR_PLATFORM_INFO: u32 = 0x0000_00CE;
/// PAT (Page Attribute Table)
pub const MSR_PAT: u32 = 0x0000_0277;
/// x2APIC ID
pub const MSR_X2APIC_ID: u32 = 0x0000_0802;
/// STAR (for SYSCALL)
pub const MSR_STAR: u32 = 0xC000_0081;
/// LSTAR (SYSCALL target in 64-bit mode)
pub const MSR_LSTAR: u32 = 0xC000_0082;
/// CSTAR (SYSCALL target in compat mode)
pub const MSR_CSTAR: u32 = 0xC000_0083;
/// SFMASK (SYSCALL flag mask)
pub const MSR_SFMASK: u32 = 0xC000_0084;

// =============================================================================
// EFER FLAGS
// =============================================================================

/// System Call Enable
pub const EFER_SCE: u64 = 1 << 0;
/// Long Mode Enable
pub const EFER_LME: u64 = 1 << 8;
/// Long Mode Active
pub const EFER_LMA: u64 = 1 << 10;
/// No-Execute Enable
pub const EFER_NXE: u64 = 1 << 11;
/// Secure Virtual Machine Enable
pub const EFER_SVME: u64 = 1 << 12;
/// Long Mode Segment Limit Enable
pub const EFER_LMSLE: u64 = 1 << 13;
/// Fast FXSAVE/FXRSTOR
pub const EFER_FFXSR: u64 = 1 << 14;
/// Translation Cache Extension
pub const EFER_TCE: u64 = 1 << 15;

// =============================================================================
// CR0 FLAGS
// =============================================================================

/// Protection Enable
pub const CR0_PE: u64 = 1 << 0;
/// Monitor Coprocessor
pub const CR0_MP: u64 = 1 << 1;
/// Emulation
pub const CR0_EM: u64 = 1 << 2;
/// Task Switched
pub const CR0_TS: u64 = 1 << 3;
/// Extension Type
pub const CR0_ET: u64 = 1 << 4;
/// Numeric Error
pub const CR0_NE: u64 = 1 << 5;
/// Write Protect
pub const CR0_WP: u64 = 1 << 16;
/// Alignment Mask
pub const CR0_AM: u64 = 1 << 18;
/// Not Write-through
pub const CR0_NW: u64 = 1 << 29;
/// Cache Disable
pub const CR0_CD: u64 = 1 << 30;
/// Paging
pub const CR0_PG: u64 = 1 << 31;

// =============================================================================
// CR4 FLAGS
// =============================================================================

/// Virtual-8086 Mode Extensions
pub const CR4_VME: u64 = 1 << 0;
/// Protected-Mode Virtual Interrupts
pub const CR4_PVI: u64 = 1 << 1;
/// Time Stamp Disable
pub const CR4_TSD: u64 = 1 << 2;
/// Debugging Extensions
pub const CR4_DE: u64 = 1 << 3;
/// Page Size Extensions
pub const CR4_PSE: u64 = 1 << 4;
/// Physical Address Extension
pub const CR4_PAE: u64 = 1 << 5;
/// Machine Check Exception
pub const CR4_MCE: u64 = 1 << 6;
/// Page Global Enable
pub const CR4_PGE: u64 = 1 << 7;
/// Performance Monitoring Counter Enable
pub const CR4_PCE: u64 = 1 << 8;
/// OS FXSAVE/FXRSTOR Support
pub const CR4_OSFXSR: u64 = 1 << 9;
/// OS Unmasked Exception Support
pub const CR4_OSXMMEXCPT: u64 = 1 << 10;
/// User-Mode Instruction Prevention
pub const CR4_UMIP: u64 = 1 << 11;
/// 57-bit Linear Addresses (5-level paging)
pub const CR4_LA57: u64 = 1 << 12;
/// VMX Enable
pub const CR4_VMXE: u64 = 1 << 13;
/// SMX Enable
pub const CR4_SMXE: u64 = 1 << 14;
/// FSGSBASE Enable
pub const CR4_FSGSBASE: u64 = 1 << 16;
/// PCID Enable
pub const CR4_PCIDE: u64 = 1 << 17;
/// XSAVE Enable
pub const CR4_OSXSAVE: u64 = 1 << 18;
/// SMEP Enable
pub const CR4_SMEP: u64 = 1 << 20;
/// SMAP Enable
pub const CR4_SMAP: u64 = 1 << 21;
/// Protection Key Enable
pub const CR4_PKE: u64 = 1 << 22;
/// Control-flow Enforcement Technology
pub const CR4_CET: u64 = 1 << 23;
/// Protection Keys for Supervisor-mode
pub const CR4_PKS: u64 = 1 << 24;

// =============================================================================
// MAIN BOOT FUNCTIONS
// =============================================================================

/// Pre-initialization for x86_64
///
/// # Safety
/// Must be called early in boot, in 64-bit long mode.
pub unsafe fn pre_init(ctx: &mut BootContext) -> BootResult<()> {
    // Verify we're in 64-bit long mode
    let efer = rdmsr(MSR_EFER);
    if (efer & EFER_LMA) == 0 {
        return Err(BootError::CpuInitFailed("not in long mode"));
    }

    // Verify paging is enabled
    let cr0 = read_cr0();
    if (cr0 & CR0_PG) == 0 {
        return Err(BootError::CpuInitFailed("paging not enabled"));
    }

    // Store initial CR values
    ctx.arch_data.x86.cr0 = cr0;
    ctx.arch_data.x86.cr3 = read_cr3();
    ctx.arch_data.x86.cr4 = read_cr4();
    ctx.arch_data.x86.efer = efer;

    // Check for LA57 (5-level paging)
    ctx.arch_data.x86.x2apic_enabled =
        (efer & EFER_LME) != 0 && has_cpuid_feature(7, 0, CpuidReg::Ecx, 16);

    Ok(())
}

/// Initialize serial port
pub unsafe fn init_serial(config: &SerialConfig) -> BootResult<()> {
    serial::init_serial_port(config.base as u16, config.baud_rate)
}

/// Detect CPU features
pub unsafe fn detect_cpu_features(state: &mut CpuState) -> BootResult<()> {
    cpu::detect_features(state)
}

/// Initialize FPU and SIMD
pub unsafe fn init_fpu() -> BootResult<()> {
    cpu::init_fpu_simd()
}

/// Full CPU initialization
pub unsafe fn cpu_init(ctx: &mut BootContext) -> BootResult<()> {
    // Set up GDT with TSS
    gdt::init_gdt(ctx)?;

    // Configure control registers
    cpu::configure_control_registers(ctx)?;

    // Enable syscall/sysret
    cpu::enable_syscall()?;

    Ok(())
}

/// Set up page tables
pub unsafe fn setup_page_tables(ctx: &mut BootContext) -> BootResult<()> {
    paging::setup_page_tables(ctx)
}

/// Initialize platform drivers
pub unsafe fn init_platform_drivers(_ctx: &mut BootContext) -> BootResult<()> {
    // x86 doesn't have many platform-specific early drivers
    Ok(())
}

/// Initialize interrupts
pub unsafe fn init_interrupts(ctx: &mut BootContext) -> BootResult<()> {
    // Set up IDT
    idt::init_idt(ctx)?;

    // Initialize APIC
    apic::init_apic(ctx)?;

    Ok(())
}

/// Initialize timers
pub unsafe fn init_timers(ctx: &mut BootContext) -> BootResult<()> {
    timers::init_timers(ctx)
}

/// Initialize SMP
pub unsafe fn init_smp(ctx: &mut BootContext) -> BootResult<()> {
    smp::init_smp(ctx)
}

/// Apply KASLR
pub unsafe fn apply_kaslr(ctx: &mut BootContext) -> BootResult<()> {
    // Generate random offset using RDRAND/RDSEED if available
    let offset = if has_cpuid_feature(7, 0, CpuidReg::Ebx, 18) {
        // RDSEED available
        let mut val: u64;
        core::arch::asm!(
            "2: rdseed {0:r}",
            "jnc 2b",
            out(reg) val,
            options(nomem)
        );
        val & 0x0000_000F_FFFF_F000 // 1GB alignment mask, 36-bit range
    } else if has_cpuid_feature(1, 0, CpuidReg::Ecx, 30) {
        // RDRAND available
        let mut val: u64;
        core::arch::asm!(
            "2: rdrand {0:r}",
            "jnc 2b",
            out(reg) val,
            options(nomem)
        );
        val & 0x0000_000F_FFFF_F000
    } else {
        // Fall back to TSC
        let tsc = rdtsc();
        (tsc ^ (tsc >> 17)) & 0x0000_000F_FFFF_F000
    };

    // Limit to configured entropy bits
    let mask = ((1u64 << ctx.config.kaslr_entropy_bits) - 1) << 12;
    let _kaslr_offset = offset & mask;

    // Note: Actual relocation would be done here
    // For now, we just compute the offset

    Ok(())
}

/// Prepare for kernel handoff
pub unsafe fn prepare_handoff(ctx: &mut BootContext) -> BootResult<()> {
    // Final CR3 should be our new page tables
    ctx.arch_data.x86.cr3 = read_cr3();

    // Make sure interrupts are disabled for handoff
    core::arch::asm!("cli", options(nomem, preserves_flags));

    Ok(())
}

/// Read timestamp counter
#[inline(always)]
pub fn read_timestamp() -> u64 {
    rdtsc()
}

/// Halt forever
pub fn halt_forever() -> ! {
    unsafe {
        core::arch::asm!("cli", options(nomem, preserves_flags));
        loop {
            core::arch::asm!("hlt", options(nomem, preserves_flags));
        }
    }
}

// =============================================================================
// LOW-LEVEL CPU ACCESS
// =============================================================================

/// Read MSR
#[inline(always)]
pub unsafe fn rdmsr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    core::arch::asm!(
        "rdmsr",
        in("ecx") msr,
        out("eax") low,
        out("edx") high,
        options(nomem, preserves_flags)
    );
    ((high as u64) << 32) | (low as u64)
}

/// Write MSR
#[inline(always)]
pub unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    core::arch::asm!(
        "wrmsr",
        in("ecx") msr,
        in("eax") low,
        in("edx") high,
        options(nomem, preserves_flags)
    );
}

/// Read TSC
#[inline(always)]
pub fn rdtsc() -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        core::arch::asm!(
            "rdtsc",
            out("eax") low,
            out("edx") high,
            options(nomem, preserves_flags)
        );
    }
    ((high as u64) << 32) | (low as u64)
}

/// Read TSC with processor ID
#[inline(always)]
pub fn rdtscp() -> (u64, u32) {
    let low: u32;
    let high: u32;
    let aux: u32;
    unsafe {
        core::arch::asm!(
            "rdtscp",
            out("eax") low,
            out("edx") high,
            out("ecx") aux,
            options(nomem, preserves_flags)
        );
    }
    (((high as u64) << 32) | (low as u64), aux)
}

/// Read CR0
#[inline(always)]
pub fn read_cr0() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mov {}, cr0",
            out(reg) value,
            options(nomem, preserves_flags)
        );
    }
    value
}

/// Write CR0
#[inline(always)]
pub unsafe fn write_cr0(value: u64) {
    core::arch::asm!(
        "mov cr0, {}",
        in(reg) value,
        options(nomem, preserves_flags)
    );
}

/// Read CR2 (page fault linear address)
#[inline(always)]
pub fn read_cr2() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mov {}, cr2",
            out(reg) value,
            options(nomem, preserves_flags)
        );
    }
    value
}

/// Read CR3
#[inline(always)]
pub fn read_cr3() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mov {}, cr3",
            out(reg) value,
            options(nomem, preserves_flags)
        );
    }
    value
}

/// Write CR3
#[inline(always)]
pub unsafe fn write_cr3(value: u64) {
    core::arch::asm!(
        "mov cr3, {}",
        in(reg) value,
        options(nomem, preserves_flags)
    );
}

/// Read CR4
#[inline(always)]
pub fn read_cr4() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mov {}, cr4",
            out(reg) value,
            options(nomem, preserves_flags)
        );
    }
    value
}

/// Write CR4
#[inline(always)]
pub unsafe fn write_cr4(value: u64) {
    core::arch::asm!(
        "mov cr4, {}",
        in(reg) value,
        options(nomem, preserves_flags)
    );
}

/// Read XCR0 (extended control register)
#[inline(always)]
pub unsafe fn xgetbv(xcr: u32) -> u64 {
    let low: u32;
    let high: u32;
    core::arch::asm!(
        "xgetbv",
        in("ecx") xcr,
        out("eax") low,
        out("edx") high,
        options(nomem, preserves_flags)
    );
    ((high as u64) << 32) | (low as u64)
}

/// Write XCR0
#[inline(always)]
pub unsafe fn xsetbv(xcr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    core::arch::asm!(
        "xsetbv",
        in("ecx") xcr,
        in("eax") low,
        in("edx") high,
        options(nomem, preserves_flags)
    );
}

/// Invalidate TLB for a single page
#[inline(always)]
pub unsafe fn invlpg(addr: u64) {
    core::arch::asm!(
        "invlpg [{}]",
        in(reg) addr,
        options(preserves_flags)
    );
}

/// Flush entire TLB
#[inline(always)]
pub unsafe fn flush_tlb() {
    let cr3 = read_cr3();
    write_cr3(cr3);
}

/// Output byte to I/O port
#[inline(always)]
pub unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nomem, preserves_flags)
    );
}

/// Input byte from I/O port
#[inline(always)]
pub unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    core::arch::asm!(
        "in al, dx",
        in("dx") port,
        out("al") value,
        options(nomem, preserves_flags)
    );
    value
}

/// Output word to I/O port
#[inline(always)]
pub unsafe fn outw(port: u16, value: u16) {
    core::arch::asm!(
        "out dx, ax",
        in("dx") port,
        in("ax") value,
        options(nomem, preserves_flags)
    );
}

/// Input word from I/O port
#[inline(always)]
pub unsafe fn inw(port: u16) -> u16 {
    let value: u16;
    core::arch::asm!(
        "in ax, dx",
        in("dx") port,
        out("ax") value,
        options(nomem, preserves_flags)
    );
    value
}

/// Output dword to I/O port
#[inline(always)]
pub unsafe fn outl(port: u16, value: u32) {
    core::arch::asm!(
        "out dx, eax",
        in("dx") port,
        in("eax") value,
        options(nomem, preserves_flags)
    );
}

/// Input dword from I/O port
#[inline(always)]
pub unsafe fn inl(port: u16) -> u32 {
    let value: u32;
    core::arch::asm!(
        "in eax, dx",
        in("dx") port,
        out("eax") value,
        options(nomem, preserves_flags)
    );
    value
}

/// Small delay using I/O port
#[inline(always)]
pub unsafe fn io_delay() {
    // Write to unused port for ~1µs delay
    outb(0x80, 0);
}

// =============================================================================
// CPUID HELPERS
// =============================================================================

/// CPUID register selection
#[derive(Debug, Clone, Copy)]
pub enum CpuidReg {
    Eax,
    Ebx,
    Ecx,
    Edx,
}

/// Execute CPUID instruction
pub fn cpuid(leaf: u32, subleaf: u32) -> (u32, u32, u32, u32) {
    let eax: u32;
    let ebx: u32;
    let ecx: u32;
    let edx: u32;

    unsafe {
        core::arch::asm!(
            "cpuid",
            inout("eax") leaf => eax,
            inout("ecx") subleaf => ecx,
            out("ebx") ebx,
            out("edx") edx,
            options(nomem, preserves_flags)
        );
    }

    (eax, ebx, ecx, edx)
}

/// Check if a specific CPUID feature is present
pub fn has_cpuid_feature(leaf: u32, subleaf: u32, reg: CpuidReg, bit: u8) -> bool {
    let (eax, ebx, ecx, edx) = cpuid(leaf, subleaf);
    let value = match reg {
        CpuidReg::Eax => eax,
        CpuidReg::Ebx => ebx,
        CpuidReg::Ecx => ecx,
        CpuidReg::Edx => edx,
    };
    (value & (1 << bit)) != 0
}

/// Get maximum CPUID leaf
pub fn cpuid_max_leaf() -> u32 {
    let (eax, _, _, _) = cpuid(0, 0);
    eax
}

/// Get maximum extended CPUID leaf
pub fn cpuid_max_extended_leaf() -> u32 {
    let (eax, _, _, _) = cpuid(0x8000_0000, 0);
    eax
}
