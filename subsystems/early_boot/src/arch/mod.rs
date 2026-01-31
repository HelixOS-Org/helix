//! # Architecture Module
//!
//! Provides architecture-specific implementations for the early boot sequence.
//! Each architecture implements the same interface defined here.

use crate::core::{BootContext, CpuState};
use crate::error::{BootError, BootResult};
use crate::SerialConfig;

// =============================================================================
// ARCHITECTURE-SPECIFIC MODULES
// =============================================================================

#[cfg(target_arch = "x86_64")]
pub mod x86_64;

#[cfg(target_arch = "aarch64")]
pub mod aarch64;

#[cfg(target_arch = "riscv64")]
pub mod riscv64;

// =============================================================================
// COMMON ARCHITECTURE INTERFACE
// =============================================================================

// Re-export architecture-specific implementations
#[cfg(target_arch = "aarch64")]
pub use aarch64::*;
#[cfg(target_arch = "riscv64")]
pub use riscv64::*;
#[cfg(target_arch = "x86_64")]
pub use x86_64::*;

// =============================================================================
// FALLBACK IMPLEMENTATIONS (for unsupported architectures)
// =============================================================================

#[cfg(not(any(
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "riscv64"
)))]
mod fallback {
    use super::*;

    pub unsafe fn pre_init(_ctx: &mut BootContext) -> BootResult<()> {
        Err(BootError::NotSupported("unsupported architecture"))
    }

    pub unsafe fn init_serial(_config: &SerialConfig) -> BootResult<()> {
        Err(BootError::NotSupported("serial"))
    }

    pub unsafe fn detect_cpu_features(_state: &mut CpuState) -> BootResult<()> {
        Err(BootError::NotSupported("CPU detection"))
    }

    pub unsafe fn init_fpu() -> BootResult<()> {
        Err(BootError::NotSupported("FPU"))
    }

    pub unsafe fn cpu_init(_ctx: &mut BootContext) -> BootResult<()> {
        Err(BootError::NotSupported("CPU init"))
    }

    pub unsafe fn setup_page_tables(_ctx: &mut BootContext) -> BootResult<()> {
        Err(BootError::NotSupported("paging"))
    }

    pub unsafe fn init_platform_drivers(_ctx: &mut BootContext) -> BootResult<()> {
        Ok(())
    }

    pub unsafe fn init_interrupts(_ctx: &mut BootContext) -> BootResult<()> {
        Err(BootError::NotSupported("interrupts"))
    }

    pub unsafe fn init_timers(_ctx: &mut BootContext) -> BootResult<()> {
        Err(BootError::NotSupported("timers"))
    }

    pub unsafe fn init_smp(_ctx: &mut BootContext) -> BootResult<()> {
        Err(BootError::SmpNotAvailable)
    }

    pub unsafe fn apply_kaslr(_ctx: &mut BootContext) -> BootResult<()> {
        Err(BootError::NotSupported("KASLR"))
    }

    pub unsafe fn prepare_handoff(_ctx: &mut BootContext) -> BootResult<()> {
        Ok(())
    }

    pub fn read_timestamp() -> u64 {
        0
    }

    pub fn halt_forever() -> ! {
        loop {
            core::hint::spin_loop();
        }
    }
}

#[cfg(not(any(
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "riscv64"
)))]
pub use fallback::*;

// =============================================================================
// COMMON UTILITIES
// =============================================================================

/// Memory barrier (full fence)
#[inline(always)]
pub fn memory_barrier() {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("mfence", options(nostack, preserves_flags));
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("dsb sy", options(nostack, preserves_flags));
    }

    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!("fence iorw, iorw", options(nostack, preserves_flags));
    }

    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64"
    )))]
    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
}

/// Instruction barrier (serialize pipeline)
#[inline(always)]
pub fn instruction_barrier() {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        // x86_64 doesn't have a dedicated instruction barrier
        // CPUID serializes but is slow, use LFENCE
        core::arch::asm!("lfence", options(nostack, preserves_flags));
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("isb", options(nostack, preserves_flags));
    }

    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!("fence.i", options(nostack, preserves_flags));
    }
}

/// Disable interrupts and return previous state
#[inline(always)]
pub fn disable_interrupts() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        let flags: u64;
        unsafe {
            core::arch::asm!(
                "pushfq",
                "pop {}",
                "cli",
                out(reg) flags,
                options(nomem, preserves_flags)
            );
        }
        (flags & 0x200) != 0 // IF flag
    }

    #[cfg(target_arch = "aarch64")]
    {
        let daif: u64;
        unsafe {
            core::arch::asm!(
                "mrs {}, daif",
                "msr daifset, #0xf",
                out(reg) daif,
                options(nomem, preserves_flags)
            );
        }
        (daif & 0x80) == 0 // IRQ enabled if I bit is 0
    }

    #[cfg(target_arch = "riscv64")]
    {
        let sstatus: u64;
        unsafe {
            core::arch::asm!(
                "csrrci {}, sstatus, 0x2", // Clear SIE bit
                out(reg) sstatus,
                options(nomem)
            );
        }
        (sstatus & 0x2) != 0 // SIE bit
    }

    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64"
    )))]
    false
}

/// Enable interrupts
#[inline(always)]
pub fn enable_interrupts() {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("sti", options(nomem, preserves_flags));
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("msr daifclr, #0x2", options(nomem, preserves_flags)); // Clear IRQ mask
    }

    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!("csrsi sstatus, 0x2", options(nomem)); // Set SIE bit
    }
}

/// Restore interrupt state
#[inline(always)]
pub fn restore_interrupts(enabled: bool) {
    if enabled {
        enable_interrupts();
    }
}

/// Pause/yield hint (for spin loops)
#[inline(always)]
pub fn spin_hint() {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("pause", options(nomem, nostack, preserves_flags));
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("yield", options(nomem, nostack, preserves_flags));
    }

    #[cfg(target_arch = "riscv64")]
    unsafe {
        // PAUSE hint (if available) or just a NOP
        core::arch::asm!("nop", options(nomem, nostack, preserves_flags));
    }

    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64"
    )))]
    core::hint::spin_loop();
}

/// Get current CPU ID
#[inline(always)]
pub fn current_cpu_id() -> u32 {
    #[cfg(target_arch = "x86_64")]
    {
        // Read APIC ID from CPUID or APIC
        let mut id: u32;
        unsafe {
            // Use CPUID leaf 0x01, EBX[31:24]
            core::arch::asm!(
                "cpuid",
                inout("eax") 1u32 => _,
                out("ebx") id,
                out("ecx") _,
                out("edx") _,
                options(nomem, preserves_flags)
            );
        }
        (id >> 24) & 0xFF
    }

    #[cfg(target_arch = "aarch64")]
    {
        let mpidr: u64;
        unsafe {
            core::arch::asm!(
                "mrs {}, mpidr_el1",
                out(reg) mpidr,
                options(nomem, preserves_flags)
            );
        }
        (mpidr & 0xFF) as u32
    }

    #[cfg(target_arch = "riscv64")]
    {
        let hart_id: u64;
        unsafe {
            core::arch::asm!(
                "mv {}, tp",
                out(reg) hart_id,
                options(nomem, preserves_flags)
            );
        }
        hart_id as u32
    }

    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64"
    )))]
    0
}

/// Check if we're on the BSP (Boot Strap Processor)
pub fn is_bsp() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        // BSP flag in IA32_APIC_BASE MSR
        let apic_base: u64;
        unsafe {
            core::arch::asm!(
                "rdmsr",
                in("ecx") 0x1Bu32, // IA32_APIC_BASE
                out("eax") _,
                out("edx") _,
                lateout("rax") apic_base,
                options(nomem, preserves_flags)
            );
        }
        (apic_base & (1 << 8)) != 0
    }

    #[cfg(target_arch = "aarch64")]
    {
        // Check MPIDR, typically CPU 0 is BSP
        current_cpu_id() == 0
    }

    #[cfg(target_arch = "riscv64")]
    {
        // First hart is typically BSP
        current_cpu_id() == 0
    }

    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64"
    )))]
    true
}
