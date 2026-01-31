//! x86_64 Architecture Support
//!
//! Platform-specific code for AMD64/Intel 64.

pub mod cpu;
pub mod gdt;
pub mod idt;
pub mod paging;
pub mod apic;

use crate::error::Result;
use crate::arch::{CpuFeatures, MemoryModel, ArchOperations};
use crate::raw::types::*;

// =============================================================================
// EARLY INITIALIZATION
// =============================================================================

/// Early platform initialization
pub fn early_init() -> Result<()> {
    // Disable interrupts during early init
    unsafe { core::arch::asm!("cli", options(nomem, nostack)); }

    // TODO: Set up minimal GDT if needed
    // TODO: Check CPU compatibility

    Ok(())
}

/// Initialize CPU
pub fn init_cpu(features: &CpuFeatures) -> Result<()> {
    // Enable required CPU features
    cpu::enable_features(features)?;

    Ok(())
}

/// Initialize memory subsystem
pub fn init_memory(_model: &MemoryModel) -> Result<()> {
    // Memory initialization is handled by the paging module
    Ok(())
}

/// Initialize interrupts
pub fn init_interrupts() -> Result<()> {
    // IDT setup is handled by the idt module
    Ok(())
}

// =============================================================================
// CPU ID
// =============================================================================

/// Execute CPUID instruction
/// Note: RBX is preserved manually as it's used by LLVM on UEFI targets
#[inline]
pub fn cpuid(leaf: u32, subleaf: u32) -> CpuidResult {
    let mut result = CpuidResult::default();

    unsafe {
        // Save RBX, execute cpuid, restore RBX (RBX is reserved by LLVM on UEFI)
        core::arch::asm!(
            "push rbx",
            "cpuid",
            "mov {ebx_out:e}, ebx",
            "pop rbx",
            inout("eax") leaf => result.eax,
            inout("ecx") subleaf => result.ecx,
            ebx_out = out(reg) result.ebx,
            lateout("edx") result.edx,
            options(nostack),
        );
    }

    result
}

/// CPUID result
#[derive(Debug, Clone, Copy, Default)]
pub struct CpuidResult {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
}

impl CpuidResult {
    /// Check if bit is set in EAX
    pub fn eax_bit(&self, bit: u32) -> bool {
        (self.eax & (1 << bit)) != 0
    }

    /// Check if bit is set in EBX
    pub fn ebx_bit(&self, bit: u32) -> bool {
        (self.ebx & (1 << bit)) != 0
    }

    /// Check if bit is set in ECX
    pub fn ecx_bit(&self, bit: u32) -> bool {
        (self.ecx & (1 << bit)) != 0
    }

    /// Check if bit is set in EDX
    pub fn edx_bit(&self, bit: u32) -> bool {
        (self.edx & (1 << bit)) != 0
    }
}

// =============================================================================
// MSR ACCESS
// =============================================================================

/// Read MSR (Model Specific Register)
#[inline]
pub unsafe fn rdmsr(msr: u32) -> u64 {
    let (low, high): (u32, u32);
    core::arch::asm!(
        "rdmsr",
        in("ecx") msr,
        out("eax") low,
        out("edx") high,
        options(nomem, nostack, preserves_flags),
    );
    ((high as u64) << 32) | (low as u64)
}

/// Write MSR (Model Specific Register)
#[inline]
pub unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    core::arch::asm!(
        "wrmsr",
        in("ecx") msr,
        in("eax") low,
        in("edx") high,
        options(nomem, nostack, preserves_flags),
    );
}

/// Common MSR addresses
pub mod msr {
    /// IA32_APIC_BASE
    pub const IA32_APIC_BASE: u32 = 0x1B;
    /// IA32_FEATURE_CONTROL
    pub const IA32_FEATURE_CONTROL: u32 = 0x3A;
    /// IA32_SYSENTER_CS
    pub const IA32_SYSENTER_CS: u32 = 0x174;
    /// IA32_SYSENTER_ESP
    pub const IA32_SYSENTER_ESP: u32 = 0x175;
    /// IA32_SYSENTER_EIP
    pub const IA32_SYSENTER_EIP: u32 = 0x176;
    /// IA32_PAT
    pub const IA32_PAT: u32 = 0x277;
    /// IA32_EFER
    pub const IA32_EFER: u32 = 0xC0000080;
    /// IA32_STAR
    pub const IA32_STAR: u32 = 0xC0000081;
    /// IA32_LSTAR
    pub const IA32_LSTAR: u32 = 0xC0000082;
    /// IA32_CSTAR
    pub const IA32_CSTAR: u32 = 0xC0000083;
    /// IA32_FMASK
    pub const IA32_FMASK: u32 = 0xC0000084;
    /// IA32_FS_BASE
    pub const IA32_FS_BASE: u32 = 0xC0000100;
    /// IA32_GS_BASE
    pub const IA32_GS_BASE: u32 = 0xC0000101;
    /// IA32_KERNEL_GS_BASE
    pub const IA32_KERNEL_GS_BASE: u32 = 0xC0000102;
    /// IA32_TSC_AUX
    pub const IA32_TSC_AUX: u32 = 0xC0000103;
}

// =============================================================================
// CONTROL REGISTERS
// =============================================================================

/// Read CR0
#[inline]
pub fn read_cr0() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("mov {}, cr0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write CR0
#[inline]
pub unsafe fn write_cr0(value: u64) {
    core::arch::asm!("mov cr0, {}", in(reg) value, options(nomem, nostack, preserves_flags));
}

/// Read CR2 (page fault address)
#[inline]
pub fn read_cr2() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("mov {}, cr2", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Read CR3 (page table base)
#[inline]
pub fn read_cr3() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("mov {}, cr3", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write CR3 (page table base)
#[inline]
pub unsafe fn write_cr3(value: u64) {
    core::arch::asm!("mov cr3, {}", in(reg) value, options(nomem, nostack, preserves_flags));
}

/// Read CR4
#[inline]
pub fn read_cr4() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("mov {}, cr4", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write CR4
#[inline]
pub unsafe fn write_cr4(value: u64) {
    core::arch::asm!("mov cr4, {}", in(reg) value, options(nomem, nostack, preserves_flags));
}

/// CR0 bits
pub mod cr0 {
    pub const PE: u64 = 1 << 0;  // Protected Mode Enable
    pub const MP: u64 = 1 << 1;  // Monitor Co-Processor
    pub const EM: u64 = 1 << 2;  // Emulation
    pub const TS: u64 = 1 << 3;  // Task Switched
    pub const ET: u64 = 1 << 4;  // Extension Type
    pub const NE: u64 = 1 << 5;  // Numeric Error
    pub const WP: u64 = 1 << 16; // Write Protect
    pub const AM: u64 = 1 << 18; // Alignment Mask
    pub const NW: u64 = 1 << 29; // Not Write-Through
    pub const CD: u64 = 1 << 30; // Cache Disable
    pub const PG: u64 = 1 << 31; // Paging
}

/// CR4 bits
pub mod cr4 {
    pub const VME: u64 = 1 << 0;        // Virtual 8086 Mode Extensions
    pub const PVI: u64 = 1 << 1;        // Protected-mode Virtual Interrupts
    pub const TSD: u64 = 1 << 2;        // Time Stamp Disable
    pub const DE: u64 = 1 << 3;         // Debugging Extensions
    pub const PSE: u64 = 1 << 4;        // Page Size Extension
    pub const PAE: u64 = 1 << 5;        // Physical Address Extension
    pub const MCE: u64 = 1 << 6;        // Machine Check Exception
    pub const PGE: u64 = 1 << 7;        // Page Global Enable
    pub const PCE: u64 = 1 << 8;        // Performance-Monitoring Counter Enable
    pub const OSFXSR: u64 = 1 << 9;     // FXSAVE/FXRSTOR Support
    pub const OSXMMEXCPT: u64 = 1 << 10; // Unmasked SIMD Exceptions
    pub const UMIP: u64 = 1 << 11;      // User-Mode Instruction Prevention
    pub const LA57: u64 = 1 << 12;      // 5-Level Paging
    pub const VMXE: u64 = 1 << 13;      // Virtual Machine Extensions Enable
    pub const SMXE: u64 = 1 << 14;      // Safer Mode Extensions Enable
    pub const FSGSBASE: u64 = 1 << 16;  // FSGSBASE Instructions Enable
    pub const PCIDE: u64 = 1 << 17;     // PCID Enable
    pub const OSXSAVE: u64 = 1 << 18;   // XSAVE Enable
    pub const KL: u64 = 1 << 19;        // Key Locker Enable
    pub const SMEP: u64 = 1 << 20;      // SMEP Enable
    pub const SMAP: u64 = 1 << 21;      // SMAP Enable
    pub const PKE: u64 = 1 << 22;       // PKU Enable
    pub const CET: u64 = 1 << 23;       // Control-flow Enforcement
    pub const PKS: u64 = 1 << 24;       // Protection Keys for Supervisor
}

// =============================================================================
// EFER (Extended Feature Enable Register)
// =============================================================================

/// EFER bits
pub mod efer {
    /// System Call Extensions
    pub const SCE: u64 = 1 << 0;
    /// Long Mode Enable
    pub const LME: u64 = 1 << 8;
    /// Long Mode Active
    pub const LMA: u64 = 1 << 10;
    /// No-Execute Enable
    pub const NXE: u64 = 1 << 11;
    /// Secure Virtual Machine Enable
    pub const SVME: u64 = 1 << 12;
    /// Long Mode Segment Limit Enable
    pub const LMSLE: u64 = 1 << 13;
    /// Fast FXSAVE/FXRSTOR
    pub const FFXSR: u64 = 1 << 14;
    /// Translation Cache Extension
    pub const TCE: u64 = 1 << 15;
}

/// Read EFER
#[inline]
pub fn read_efer() -> u64 {
    unsafe { rdmsr(msr::IA32_EFER) }
}

/// Write EFER
#[inline]
pub unsafe fn write_efer(value: u64) {
    wrmsr(msr::IA32_EFER, value);
}

// =============================================================================
// INTERRUPTS
// =============================================================================

/// Enable interrupts
#[inline]
pub fn enable_interrupts() {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack));
    }
}

/// Disable interrupts
#[inline]
pub fn disable_interrupts() {
    unsafe {
        core::arch::asm!("cli", options(nomem, nostack));
    }
}

/// Check if interrupts are enabled
#[inline]
pub fn interrupts_enabled() -> bool {
    let flags: u64;
    unsafe {
        core::arch::asm!(
            "pushfq; pop {}",
            out(reg) flags,
            options(nomem, preserves_flags),
        );
    }
    (flags & (1 << 9)) != 0
}

/// Execute with interrupts disabled
#[inline]
pub fn without_interrupts<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let enabled = interrupts_enabled();
    disable_interrupts();
    let result = f();
    if enabled {
        enable_interrupts();
    }
    result
}

// =============================================================================
// TLB
// =============================================================================

/// Invalidate TLB entry
#[inline]
pub fn invlpg(addr: VirtualAddress) {
    unsafe {
        core::arch::asm!("invlpg [{}]", in(reg) addr.0 as usize, options(nostack, preserves_flags));
    }
}

/// Flush entire TLB by reloading CR3
#[inline]
pub fn flush_tlb() {
    unsafe {
        write_cr3(read_cr3());
    }
}

/// Flush TLB with PCID
#[inline]
pub unsafe fn flush_tlb_pcid(pcid: u16, all: bool) {
    let descriptor: u128;
    let invpcid_type: u64;

    if all {
        // Type 2: Invalidate all including global
        invpcid_type = 2;
        descriptor = 0;
    } else {
        // Type 1: Invalidate single PCID
        invpcid_type = 1;
        descriptor = pcid as u128;
    }

    core::arch::asm!(
        "invpcid {}, [{}]",
        in(reg) invpcid_type,
        in(reg) &descriptor,
        options(nostack, preserves_flags),
    );
}

// =============================================================================
// TIMESTAMPS
// =============================================================================

/// Read timestamp counter (RDTSC)
#[inline]
pub fn rdtsc() -> u64 {
    let low: u32;
    let high: u32;

    unsafe {
        core::arch::asm!(
            "rdtsc",
            out("eax") low,
            out("edx") high,
            options(nomem, nostack, preserves_flags),
        );
    }

    ((high as u64) << 32) | (low as u64)
}

/// Read timestamp counter with processor ID (RDTSCP)
#[inline]
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
            options(nomem, nostack, preserves_flags),
        );
    }

    (((high as u64) << 32) | (low as u64), aux)
}

// =============================================================================
// MEMORY BARRIERS
// =============================================================================

/// Full memory barrier (MFENCE)
#[inline]
pub fn mfence() {
    unsafe {
        core::arch::asm!("mfence", options(nostack, preserves_flags));
    }
}

/// Load fence (LFENCE)
#[inline]
pub fn lfence() {
    unsafe {
        core::arch::asm!("lfence", options(nostack, preserves_flags));
    }
}

/// Store fence (SFENCE)
#[inline]
pub fn sfence() {
    unsafe {
        core::arch::asm!("sfence", options(nostack, preserves_flags));
    }
}

// =============================================================================
// SPECIAL INSTRUCTIONS
// =============================================================================

/// Halt CPU (HLT)
#[inline]
pub fn hlt() {
    unsafe {
        core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
    }
}

/// Halt loop
#[inline]
pub fn halt() -> ! {
    loop {
        disable_interrupts();
        hlt();
    }
}

/// Pause instruction (for spin loops)
#[inline]
pub fn pause() {
    unsafe {
        core::arch::asm!("pause", options(nomem, nostack, preserves_flags));
    }
}

/// No operation
#[inline]
pub fn nop() {
    unsafe {
        core::arch::asm!("nop", options(nomem, nostack, preserves_flags));
    }
}

/// Breakpoint
#[inline]
pub fn breakpoint() {
    unsafe {
        core::arch::asm!("int3", options(nomem, nostack));
    }
}

/// Software interrupt
/// Note: Only supports fixed vector values due to inline asm constraints
#[inline]
pub unsafe fn software_interrupt(_vector: u8) {
    // Cannot use runtime value in const - use fixed interrupts instead
    // For dynamic interrupts, use interrupt descriptor table
    core::arch::asm!("int 0x80", options(nomem, nostack));
}

// =============================================================================
// RANDOM NUMBER GENERATION
// =============================================================================

/// Read random number (RDRAND)
pub fn rdrand() -> Option<u64> {
    let mut value: u64;
    let success: u8;

    unsafe {
        core::arch::asm!(
            "rdrand {}",
            "setc {}",
            out(reg) value,
            out(reg_byte) success,
            options(nomem, nostack),
        );
    }

    if success != 0 {
        Some(value)
    } else {
        None
    }
}

/// Read seed (RDSEED)
pub fn rdseed() -> Option<u64> {
    let mut value: u64;
    let success: u8;

    unsafe {
        core::arch::asm!(
            "rdseed {}",
            "setc {}",
            out(reg) value,
            out(reg_byte) success,
            options(nomem, nostack),
        );
    }

    if success != 0 {
        Some(value)
    } else {
        None
    }
}

// =============================================================================
// PORT I/O
// =============================================================================

/// Read byte from port
#[inline]
pub unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    core::arch::asm!(
        "in al, dx",
        in("dx") port,
        out("al") value,
        options(nomem, nostack, preserves_flags),
    );
    value
}

/// Write byte to port
#[inline]
pub unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nomem, nostack, preserves_flags),
    );
}

/// Read word from port
#[inline]
pub unsafe fn inw(port: u16) -> u16 {
    let value: u16;
    core::arch::asm!(
        "in ax, dx",
        in("dx") port,
        out("ax") value,
        options(nomem, nostack, preserves_flags),
    );
    value
}

/// Write word to port
#[inline]
pub unsafe fn outw(port: u16, value: u16) {
    core::arch::asm!(
        "out dx, ax",
        in("dx") port,
        in("ax") value,
        options(nomem, nostack, preserves_flags),
    );
}

/// Read dword from port
#[inline]
pub unsafe fn inl(port: u16) -> u32 {
    let value: u32;
    core::arch::asm!(
        "in eax, dx",
        in("dx") port,
        out("eax") value,
        options(nomem, nostack, preserves_flags),
    );
    value
}

/// Write dword to port
#[inline]
pub unsafe fn outl(port: u16, value: u32) {
    core::arch::asm!(
        "out dx, eax",
        in("dx") port,
        in("eax") value,
        options(nomem, nostack, preserves_flags),
    );
}

/// I/O wait (for slow devices)
#[inline]
pub fn io_wait() {
    unsafe {
        outb(0x80, 0);
    }
}

// =============================================================================
// ARCH OPERATIONS IMPLEMENTATION
// =============================================================================

/// x86_64 architecture operations
pub struct X86_64Ops;

impl ArchOperations for X86_64Ops {
    fn init() -> Result<()> {
        early_init()
    }

    fn current_cpu_id() -> u32 {
        // Use CPUID to get APIC ID
        let result = cpuid(1, 0);
        (result.ebx >> 24) as u32
    }

    fn halt() -> ! {
        halt()
    }

    fn enable_interrupts() {
        enable_interrupts()
    }

    fn disable_interrupts() {
        disable_interrupts()
    }

    fn interrupts_enabled() -> bool {
        interrupts_enabled()
    }

    fn read_timestamp() -> u64 {
        rdtsc()
    }

    fn invalidate_tlb_entry(addr: VirtualAddress) {
        invlpg(addr)
    }

    fn invalidate_tlb_all() {
        flush_tlb()
    }

    fn memory_barrier() {
        mfence()
    }

    fn read_barrier() {
        lfence()
    }

    fn write_barrier() {
        sfence()
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpuid() {
        let result = cpuid(0, 0);
        assert!(result.eax > 0); // Max supported leaf
    }

    #[test]
    fn test_control_registers() {
        let cr0 = read_cr0();
        assert!((cr0 & cr0::PE) != 0); // Protected mode
        assert!((cr0 & cr0::PG) != 0); // Paging enabled
    }

    #[test]
    fn test_timestamp() {
        let t1 = rdtsc();
        let t2 = rdtsc();
        assert!(t2 >= t1);
    }
}
