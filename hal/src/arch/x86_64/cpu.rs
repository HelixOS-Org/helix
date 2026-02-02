//! # CPU Control
//!
//! Low-level CPU control functions for x86_64.

use core::arch::asm;

/// Disable interrupts (CLI)
///
/// # Safety
/// Disabling interrupts can cause the system to hang if not re-enabled.
#[inline]
pub unsafe fn disable_interrupts() {
    unsafe {
        asm!("cli", options(nomem, nostack, preserves_flags));
    }
}

/// Enable interrupts (STI)
///
/// # Safety
/// Interrupts should only be enabled when the system is ready to handle them.
#[inline]
pub unsafe fn enable_interrupts() {
    unsafe {
        asm!("sti", options(nomem, nostack, preserves_flags));
    }
}

/// Check if interrupts are enabled
#[inline]
pub fn are_interrupts_enabled() -> bool {
    let flags: u64;
    unsafe {
        asm!("pushfq; pop {}", out(reg) flags, options(nomem, preserves_flags));
    }
    (flags & (1 << 9)) != 0 // IF flag is bit 9
}

/// Halt the CPU until the next interrupt
#[inline]
pub fn halt() {
    unsafe {
        asm!("hlt", options(nomem, nostack, preserves_flags));
    }
}

/// Halt and wait for interrupt (enables interrupts then halts)
#[inline]
pub fn halt_with_interrupts() {
    unsafe {
        asm!("sti; hlt", options(nomem, nostack));
    }
}

/// Execute without interrupts
///
/// Disables interrupts, executes the closure, and restores the previous state.
pub fn without_interrupts<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let were_enabled = are_interrupts_enabled();

    if were_enabled {
        unsafe {
            disable_interrupts();
        }
    }

    let result = f();

    if were_enabled {
        unsafe {
            enable_interrupts();
        }
    }

    result
}

/// Read a Model Specific Register (MSR)
///
/// # Safety
/// Reading from an invalid MSR causes a general protection fault.
#[inline]
pub unsafe fn read_msr(msr: u32) -> u64 {
    let (low, high): (u32, u32);
    unsafe {
        asm!(
            "rdmsr",
            in("ecx") msr,
            out("eax") low,
            out("edx") high,
            options(nomem, nostack, preserves_flags)
        );
    }
    ((high as u64) << 32) | (low as u64)
}

/// Write to a Model Specific Register (MSR)
///
/// # Safety
/// Writing to an invalid MSR or with invalid values causes undefined behavior.
#[inline]
pub unsafe fn write_msr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    unsafe {
        asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") low,
            in("edx") high,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Read from an I/O port
///
/// # Safety
/// Reading from invalid ports can cause undefined behavior.
#[inline]
pub unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe {
        asm!("in al, dx", out("al") value, in("dx") port, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write to an I/O port
///
/// # Safety
/// Writing to invalid ports can cause undefined behavior.
#[inline]
pub unsafe fn outb(port: u16, value: u8) {
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
    }
}

/// Read a 16-bit value from an I/O port
///
/// # Safety
/// Reading from invalid ports can cause undefined behavior.
#[inline]
pub unsafe fn inw(port: u16) -> u16 {
    let value: u16;
    unsafe {
        asm!("in ax, dx", out("ax") value, in("dx") port, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write a 16-bit value to an I/O port
///
/// # Safety
/// Writing to invalid ports can cause undefined behavior.
#[inline]
pub unsafe fn outw(port: u16, value: u16) {
    unsafe {
        asm!("out dx, ax", in("dx") port, in("ax") value, options(nomem, nostack, preserves_flags));
    }
}

/// Read a 32-bit value from an I/O port
///
/// # Safety
/// Reading from invalid ports can cause undefined behavior.
#[inline]
pub unsafe fn inl(port: u16) -> u32 {
    let value: u32;
    unsafe {
        asm!("in eax, dx", out("eax") value, in("dx") port, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write a 32-bit value to an I/O port
///
/// # Safety
/// Writing to invalid ports can cause undefined behavior.
#[inline]
pub unsafe fn outl(port: u16, value: u32) {
    unsafe {
        asm!("out dx, eax", in("dx") port, in("eax") value, options(nomem, nostack, preserves_flags));
    }
}

/// Read the current instruction pointer (RIP)
#[inline]
pub fn read_rip() -> u64 {
    let rip: u64;
    unsafe {
        asm!(
            "lea {}, [rip]",
            out(reg) rip,
            options(nomem, nostack, preserves_flags)
        );
    }
    rip
}

/// Read the current stack pointer (RSP)
#[inline]
pub fn read_rsp() -> u64 {
    let rsp: u64;
    unsafe {
        asm!(
            "mov {}, rsp",
            out(reg) rsp,
            options(nomem, nostack, preserves_flags)
        );
    }
    rsp
}

/// Read CR2 (page fault linear address)
#[inline]
pub fn read_cr2() -> u64 {
    let cr2: u64;
    unsafe {
        asm!("mov {}, cr2", out(reg) cr2, options(nomem, nostack, preserves_flags));
    }
    cr2
}

/// Read CR3 (page table base)
#[inline]
pub fn read_cr3() -> u64 {
    let cr3: u64;
    unsafe {
        asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack, preserves_flags));
    }
    cr3
}

/// Write CR3 (page table base) - flushes TLB
///
/// # Safety
/// Writing an invalid page table address will crash the system.
#[inline]
pub unsafe fn write_cr3(value: u64) {
    unsafe {
        asm!("mov cr3, {}", in(reg) value, options(nostack, preserves_flags));
    }
}

/// Invalidate a TLB entry
#[inline]
pub fn invlpg(addr: u64) {
    unsafe {
        asm!("invlpg [{}]", in(reg) addr, options(nostack, preserves_flags));
    }
}

/// x86_64 CPU implementation
pub struct X86_64Cpu {
    /// CPU ID (for SMP)
    id: u32,
}

impl X86_64Cpu {
    /// Create a new CPU abstraction
    pub const fn new() -> Self {
        Self { id: 0 }
    }

    /// Get the CPU ID
    pub fn id(&self) -> u32 {
        self.id
    }
}
