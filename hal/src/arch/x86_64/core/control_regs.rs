//! # Control Registers Framework
//!
//! Complete x86_64 control register management.
//!
//! ## Overview
//!
//! Control registers (CR0-CR4, XCR0) control fundamental CPU features:
//!
//! - **CR0**: Protection enable, paging, cache control
//! - **CR2**: Page fault linear address (read-only)
//! - **CR3**: Page table base address (PML4/PML5)
//! - **CR4**: Extensions (PAE, PSE, SMEP, SMAP, LA57, etc.)
//! - **XCR0**: Extended state component bitmap (for XSAVE)
//!
//! ## Usage
//!
//! ```rust,no_run
//! use helix_hal::arch::x86_64::core::control_regs::{Cr0, Cr4};
//!
//! // Read CR0
//! let cr0 = Cr0::read();
//! if cr0.contains(Cr0::PG) {
//!     // Paging is enabled
//! }
//!
//! // Enable SMEP in CR4
//! unsafe {
//!     Cr4::update(|cr4| {
//!         cr4.insert(Cr4::SMEP);
//!     });
//! }
//! ```

use core::arch::asm;

// =============================================================================
// CR0 - Control Register 0
// =============================================================================

bitflags::bitflags! {
    /// CR0 - Control Register 0
    ///
    /// Controls operating mode and states of the processor.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Cr0: u64 {
        /// Protection Enable
        /// When set, enables protected mode.
        const PE = 1 << 0;

        /// Monitor Coprocessor
        /// Controls interaction of WAIT/FWAIT instructions with TS flag.
        const MP = 1 << 1;

        /// Emulation
        /// When set, indicates that processor does not have x87 FPU.
        const EM = 1 << 2;

        /// Task Switched
        /// Allows saving x87 task context upon task switch.
        const TS = 1 << 3;

        /// Extension Type
        /// On 386, indicated whether x87 was 287 or 387.
        /// Reserved on modern processors (always 1).
        const ET = 1 << 4;

        /// Numeric Error
        /// Enables internal x87 floating-point error reporting.
        const NE = 1 << 5;

        /// Write Protect
        /// When set, prevents supervisor-level code from writing to
        /// read-only pages.
        const WP = 1 << 16;

        /// Alignment Mask
        /// Enables alignment checking when CPL = 3 and AC flag set.
        const AM = 1 << 18;

        /// Not Write-through
        /// Globally enables/disables write-through caching.
        const NW = 1 << 29;

        /// Cache Disable
        /// Globally enables/disables the memory caches.
        const CD = 1 << 30;

        /// Paging
        /// Enables paging. Requires PE to be set.
        const PG = 1 << 31;
    }
}

impl Cr0 {
    /// Read CR0 register
    #[inline]
    pub fn read() -> Self {
        let value: u64;
        unsafe {
            asm!("mov {}, cr0", out(reg) value, options(nomem, nostack, preserves_flags));
        }
        Self::from_bits_truncate(value)
    }

    /// Write CR0 register
    ///
    /// # Safety
    /// Incorrect values can crash the system.
    /// - Cannot clear PE when paging is enabled
    /// - Cannot clear PG when protection is disabled
    #[inline]
    pub unsafe fn write(self) {
        unsafe {
            asm!("mov cr0, {}", in(reg) self.bits(), options(nomem, nostack, preserves_flags));
        }
    }

    /// Update CR0 with a closure
    ///
    /// # Safety
    /// See `write`.
    #[inline]
    pub unsafe fn update<F: FnOnce(&mut Self)>(f: F) {
        let mut cr0 = Self::read();
        f(&mut cr0);
        unsafe { cr0.write() };
    }

    /// Get the required bits for 64-bit mode
    pub const fn long_mode_required() -> Self {
        Self::PE.union(Self::PG)
    }

    /// Check if caching is enabled (CD=0 and NW=0)
    pub fn cache_enabled(self) -> bool {
        !self.intersects(Self::CD | Self::NW)
    }

    /// Check if paging is enabled
    pub fn paging_enabled(self) -> bool {
        self.contains(Self::PG)
    }

    /// Check if protected mode is enabled
    pub fn protected_mode(self) -> bool {
        self.contains(Self::PE)
    }

    /// Check if write protection is enabled
    pub fn write_protect(self) -> bool {
        self.contains(Self::WP)
    }
}

// =============================================================================
// CR2 - Page Fault Linear Address
// =============================================================================

/// CR2 - Page Fault Linear Address Register
///
/// Contains the linear address that caused a page fault.
pub struct Cr2;

impl Cr2 {
    /// Read CR2 (page fault address)
    #[inline]
    pub fn read() -> u64 {
        let value: u64;
        unsafe {
            asm!("mov {}, cr2", out(reg) value, options(nomem, nostack, preserves_flags));
        }
        value
    }

    /// Write CR2
    ///
    /// # Safety
    /// CR2 is typically read-only (set by hardware on page faults).
    /// Writing is generally only useful in virtualization contexts.
    #[inline]
    pub unsafe fn write(value: u64) {
        unsafe {
            asm!("mov cr2, {}", in(reg) value, options(nomem, nostack, preserves_flags));
        }
    }
}

// =============================================================================
// CR3 - Page Table Base Address
// =============================================================================

bitflags::bitflags! {
    /// CR3 flags (when PCID is not used)
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Cr3Flags: u64 {
        /// Page-level Write Through
        const PWT = 1 << 3;
        /// Page-level Cache Disable
        const PCD = 1 << 4;
    }
}

/// CR3 - Page Directory Base Register
///
/// Contains the physical address of the page table root (PML4 or PML5)
/// and optional PCID.
#[derive(Debug, Clone, Copy)]
pub struct Cr3 {
    value: u64,
}

impl Cr3 {
    /// Physical address mask (bits 12-51)
    const ADDR_MASK: u64 = 0x000F_FFFF_FFFF_F000;

    /// PCID mask (bits 0-11)
    const PCID_MASK: u64 = 0x0FFF;

    /// No-invalidate bit (bit 63, used with PCID)
    const NO_INVALIDATE: u64 = 1 << 63;

    /// Read CR3 register
    #[inline]
    pub fn read() -> Self {
        let value: u64;
        unsafe {
            asm!("mov {}, cr3", out(reg) value, options(nomem, nostack, preserves_flags));
        }
        Self { value }
    }

    /// Write CR3 register
    ///
    /// # Safety
    /// Must point to a valid page table structure.
    /// Invalidates TLB entries (unless NO_INVALIDATE is set with PCID).
    #[inline]
    pub unsafe fn write(self) {
        unsafe {
            asm!("mov cr3, {}", in(reg) self.value, options(nostack, preserves_flags));
        }
    }

    /// Create a new CR3 value with physical address
    pub fn new(phys_addr: u64) -> Self {
        Self {
            value: phys_addr & Self::ADDR_MASK,
        }
    }

    /// Create a new CR3 value with physical address and PCID
    pub fn with_pcid(phys_addr: u64, pcid: u16) -> Self {
        Self {
            value: (phys_addr & Self::ADDR_MASK) | ((pcid as u64) & Self::PCID_MASK),
        }
    }

    /// Create a new CR3 value with no-invalidate flag
    pub fn with_pcid_no_invalidate(phys_addr: u64, pcid: u16) -> Self {
        Self {
            value: (phys_addr & Self::ADDR_MASK)
                | ((pcid as u64) & Self::PCID_MASK)
                | Self::NO_INVALIDATE,
        }
    }

    /// Get the page table base physical address
    pub fn base_address(self) -> u64 {
        self.value & Self::ADDR_MASK
    }

    /// Get the PCID (if PCID feature is enabled)
    pub fn pcid(self) -> u16 {
        (self.value & Self::PCID_MASK) as u16
    }

    /// Check if no-invalidate flag is set
    pub fn no_invalidate(self) -> bool {
        (self.value & Self::NO_INVALIDATE) != 0
    }

    /// Get flags (PWT, PCD)
    pub fn flags(self) -> Cr3Flags {
        Cr3Flags::from_bits_truncate(self.value)
    }

    /// Set flags
    pub fn set_flags(&mut self, flags: Cr3Flags) {
        self.value = (self.value & !(Cr3Flags::PWT | Cr3Flags::PCD).bits()) | flags.bits();
    }

    /// Reload CR3 (flush TLB)
    ///
    /// # Safety
    /// Flushes TLB, which can affect performance and concurrent memory access.
    #[inline]
    pub unsafe fn reload() {
        let cr3 = Self::read();
        unsafe { cr3.write() };
    }
}

// =============================================================================
// CR4 - Control Register 4
// =============================================================================

bitflags::bitflags! {
    /// CR4 - Control Register 4
    ///
    /// Controls various processor extensions.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Cr4: u64 {
        /// Virtual-8086 Mode Extensions
        const VME = 1 << 0;

        /// Protected-Mode Virtual Interrupts
        const PVI = 1 << 1;

        /// Time Stamp Disable
        /// When set, RDTSC can only be executed in ring 0.
        const TSD = 1 << 2;

        /// Debugging Extensions
        /// Enables I/O breakpoints and DR4/DR5 behavior.
        const DE = 1 << 3;

        /// Page Size Extension
        /// Enables 4MB pages (legacy, PAE uses 2MB).
        const PSE = 1 << 4;

        /// Physical Address Extension
        /// Enables 64-bit page table entries and > 4GB physical memory.
        const PAE = 1 << 5;

        /// Machine Check Enable
        /// Enables machine check exceptions.
        const MCE = 1 << 6;

        /// Page Global Enable
        /// Enables global page feature (pages marked global survive CR3 reload).
        const PGE = 1 << 7;

        /// Performance-Monitoring Counter Enable
        /// Enables RDPMC instruction at any privilege level.
        const PCE = 1 << 8;

        /// Operating System Support for FXSAVE/FXRSTOR
        const OSFXSR = 1 << 9;

        /// Operating System Support for Unmasked SIMD Exceptions
        const OSXMMEXCPT = 1 << 10;

        /// User-Mode Instruction Prevention
        /// Prevents execution of SGDT, SIDT, SLDT, SMSW, STR in user mode.
        const UMIP = 1 << 11;

        /// 57-bit Linear Addresses (5-level paging)
        const LA57 = 1 << 12;

        /// VMX Enable (Intel VT-x)
        const VMXE = 1 << 13;

        /// SMX Enable (Safer Mode Extensions)
        const SMXE = 1 << 14;

        /// FSGSBASE Enable
        /// Enables RDFSBASE, RDGSBASE, WRFSBASE, WRGSBASE instructions.
        const FSGSBASE = 1 << 16;

        /// PCID Enable
        /// Enables Process-Context Identifiers (TLB tagging).
        const PCIDE = 1 << 17;

        /// XSAVE and Processor Extended States Enable
        const OSXSAVE = 1 << 18;

        /// Key Locker Enable
        const KL = 1 << 19;

        /// Supervisor Mode Execution Prevention
        /// Prevents supervisor-mode code from executing user-accessible pages.
        const SMEP = 1 << 20;

        /// Supervisor Mode Access Prevention
        /// Prevents supervisor-mode code from accessing user-accessible pages.
        const SMAP = 1 << 21;

        /// Protection Keys for User-mode pages
        const PKE = 1 << 22;

        /// Control-flow Enforcement Technology
        const CET = 1 << 23;

        /// Protection Keys for Supervisor-mode pages
        const PKS = 1 << 24;

        /// User Interrupts Enable
        const UINTR = 1 << 25;
    }
}

impl Cr4 {
    /// Read CR4 register
    #[inline]
    pub fn read() -> Self {
        let value: u64;
        unsafe {
            asm!("mov {}, cr4", out(reg) value, options(nomem, nostack, preserves_flags));
        }
        Self::from_bits_truncate(value)
    }

    /// Write CR4 register
    ///
    /// # Safety
    /// Incorrect values can crash the system or disable critical features.
    #[inline]
    pub unsafe fn write(self) {
        unsafe {
            asm!("mov cr4, {}", in(reg) self.bits(), options(nomem, nostack, preserves_flags));
        }
    }

    /// Update CR4 with a closure
    ///
    /// # Safety
    /// See `write`.
    #[inline]
    pub unsafe fn update<F: FnOnce(&mut Self)>(f: F) {
        let mut cr4 = Self::read();
        f(&mut cr4);
        unsafe { cr4.write() };
    }

    /// Get the required bits for 64-bit mode with PAE
    pub const fn long_mode_required() -> Self {
        Self::PAE
    }

    /// Check if 5-level paging is enabled
    pub fn la57_enabled(self) -> bool {
        self.contains(Self::LA57)
    }

    /// Check if SMEP is enabled
    pub fn smep_enabled(self) -> bool {
        self.contains(Self::SMEP)
    }

    /// Check if SMAP is enabled
    pub fn smap_enabled(self) -> bool {
        self.contains(Self::SMAP)
    }

    /// Check if PCID is enabled
    pub fn pcid_enabled(self) -> bool {
        self.contains(Self::PCIDE)
    }

    /// Check if global pages are enabled
    pub fn global_pages_enabled(self) -> bool {
        self.contains(Self::PGE)
    }
}

// =============================================================================
// CR8 - Task Priority Register (TPR)
// =============================================================================

/// CR8 - Task Priority Register
///
/// In 64-bit mode, CR8 provides a mechanism for reading/writing the
/// APIC task priority register (bits 7:4 of APIC TPR).
pub struct Cr8;

impl Cr8 {
    /// Read CR8 (task priority)
    #[inline]
    pub fn read() -> u8 {
        let value: u64;
        unsafe {
            asm!("mov {}, cr8", out(reg) value, options(nomem, nostack, preserves_flags));
        }
        (value & 0xF) as u8
    }

    /// Write CR8 (task priority)
    ///
    /// # Safety
    /// Affects interrupt handling. Value should be 0-15.
    #[inline]
    pub unsafe fn write(priority: u8) {
        let value = (priority & 0xF) as u64;
        unsafe {
            asm!("mov cr8, {}", in(reg) value, options(nomem, nostack, preserves_flags));
        }
    }
}

// =============================================================================
// XCR0 - Extended Control Register 0
// =============================================================================

bitflags::bitflags! {
    /// XCR0 - Extended Control Register 0
    ///
    /// Controls which processor states are saved/restored by XSAVE/XRSTOR.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Xcr0: u64 {
        /// x87 FPU state (always set)
        const X87 = 1 << 0;

        /// SSE state (XMM registers)
        const SSE = 1 << 1;

        /// AVX state (YMM registers)
        const AVX = 1 << 2;

        /// MPX BNDREGS state
        const BNDREGS = 1 << 3;

        /// MPX BNDCSR state
        const BNDCSR = 1 << 4;

        /// AVX-512 opmask state (k0-k7)
        const OPMASK = 1 << 5;

        /// AVX-512 ZMM_Hi256 state (upper 256 bits of ZMM0-15)
        const ZMM_HI256 = 1 << 6;

        /// AVX-512 Hi16_ZMM state (ZMM16-31)
        const HI16_ZMM = 1 << 7;

        /// Processor Trace state
        const PT = 1 << 8;

        /// PKRU state (Protection Keys)
        const PKRU = 1 << 9;

        /// PASID state
        const PASID = 1 << 10;

        /// CET_U state (User CET)
        const CET_U = 1 << 11;

        /// CET_S state (Supervisor CET)
        const CET_S = 1 << 12;

        /// HDC state
        const HDC = 1 << 13;

        /// UINTR state
        const UINTR = 1 << 14;

        /// LBR state
        const LBR = 1 << 15;

        /// HWP state
        const HWP = 1 << 16;

        /// AMX TILECFG state
        const TILECFG = 1 << 17;

        /// AMX TILEDATA state
        const TILEDATA = 1 << 18;

        /// APX state
        const APX = 1 << 19;
    }
}

impl Xcr0 {
    /// Read XCR0 using XGETBV
    ///
    /// # Safety
    /// Requires CR4.OSXSAVE to be set.
    #[inline]
    pub unsafe fn read() -> Self {
        let (lo, hi): (u32, u32);
        unsafe {
            asm!(
                "xgetbv",
                in("ecx") 0u32,
                out("eax") lo,
                out("edx") hi,
                options(nomem, nostack, preserves_flags)
            );
        }
        Self::from_bits_truncate(((hi as u64) << 32) | (lo as u64))
    }

    /// Write XCR0 using XSETBV
    ///
    /// # Safety
    /// Requires ring 0 and CR4.OSXSAVE to be set.
    /// Invalid combinations can cause #GP.
    #[inline]
    pub unsafe fn write(self) {
        let value = self.bits();
        let lo = value as u32;
        let hi = (value >> 32) as u32;
        unsafe {
            asm!(
                "xsetbv",
                in("ecx") 0u32,
                in("eax") lo,
                in("edx") hi,
                options(nomem, nostack, preserves_flags)
            );
        }
    }

    /// Update XCR0 with a closure
    ///
    /// # Safety
    /// See `write`.
    #[inline]
    pub unsafe fn update<F: FnOnce(&mut Self)>(f: F) {
        let mut xcr0 = unsafe { Self::read() };
        f(&mut xcr0);
        unsafe { xcr0.write() };
    }

    /// Standard x87 + SSE configuration
    pub const fn x87_sse() -> Self {
        Self::X87.union(Self::SSE)
    }

    /// x87 + SSE + AVX configuration
    pub const fn avx() -> Self {
        Self::X87.union(Self::SSE).union(Self::AVX)
    }

    /// Full AVX-512 configuration
    pub const fn avx512() -> Self {
        Self::X87
            .union(Self::SSE)
            .union(Self::AVX)
            .union(Self::OPMASK)
            .union(Self::ZMM_HI256)
            .union(Self::HI16_ZMM)
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Disable interrupts and return previous state
///
/// # Safety
/// Must re-enable interrupts when done.
#[inline]
pub unsafe fn cli() -> bool {
    let rflags: u64;
    unsafe {
        asm!(
            "pushfq",
            "pop {}",
            "cli",
            out(reg) rflags,
            options(preserves_flags)
        );
    }
    (rflags & (1 << 9)) != 0
}

/// Enable interrupts
///
/// # Safety
/// Should only be called if interrupts were previously enabled.
#[inline]
pub unsafe fn sti() {
    unsafe {
        asm!("sti", options(nomem, nostack));
    }
}

/// Restore interrupt state
///
/// # Safety
/// `enabled` should be the return value from `cli()`.
#[inline]
pub unsafe fn restore_interrupts(enabled: bool) {
    if enabled {
        unsafe { sti() };
    }
}

/// Halt the processor until next interrupt
///
/// # Safety
/// Will resume on next interrupt (if interrupts are enabled).
#[inline]
pub unsafe fn hlt() {
    unsafe {
        asm!("hlt", options(nomem, nostack, preserves_flags));
    }
}

/// Pause hint for spin-wait loops
#[inline]
pub fn pause() {
    unsafe {
        asm!("pause", options(nomem, nostack, preserves_flags));
    }
}

/// Memory fence (MFENCE)
#[inline]
pub fn mfence() {
    unsafe {
        asm!("mfence", options(nostack, preserves_flags));
    }
}

/// Load fence (LFENCE)
#[inline]
pub fn lfence() {
    unsafe {
        asm!("lfence", options(nostack, preserves_flags));
    }
}

/// Store fence (SFENCE)
#[inline]
pub fn sfence() {
    unsafe {
        asm!("sfence", options(nostack, preserves_flags));
    }
}

/// Invalidate all TLB entries
///
/// # Safety
/// Expensive operation, affects all address translations.
#[inline]
pub unsafe fn invlpg_all() {
    unsafe { Cr3::reload() };
}

/// Invalidate TLB entry for a specific virtual address
///
/// # Safety
/// Can affect concurrent memory access.
#[inline]
pub unsafe fn invlpg(addr: u64) {
    unsafe {
        asm!("invlpg [{}]", in(reg) addr, options(nostack, preserves_flags));
    }
}

/// Invalidate TLB entries for PCID
///
/// # Safety
/// Requires INVPCID instruction support.
#[inline]
pub unsafe fn invpcid(pcid: u16, addr: u64, inv_type: u64) {
    let descriptor: [u64; 2] = [pcid as u64, addr];
    unsafe {
        asm!(
            "invpcid {}, [{}]",
            in(reg) inv_type,
            in(reg) &descriptor,
            options(nostack, preserves_flags)
        );
    }
}

/// INVPCID types
pub mod invpcid_type {
    /// Invalidate individual address
    pub const INDIVIDUAL: u64 = 0;
    /// Invalidate single context
    pub const SINGLE_CONTEXT: u64 = 1;
    /// Invalidate all contexts including globals
    pub const ALL_CONTEXTS: u64 = 2;
    /// Invalidate all contexts except globals
    pub const ALL_CONTEXTS_EXCEPT_GLOBAL: u64 = 3;
}

// =============================================================================
// RFLAGS OPERATIONS
// =============================================================================

bitflags::bitflags! {
    /// RFLAGS register
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct RFlags: u64 {
        /// Carry Flag
        const CF = 1 << 0;
        /// Parity Flag
        const PF = 1 << 2;
        /// Auxiliary Carry Flag
        const AF = 1 << 4;
        /// Zero Flag
        const ZF = 1 << 6;
        /// Sign Flag
        const SF = 1 << 7;
        /// Trap Flag
        const TF = 1 << 8;
        /// Interrupt Enable Flag
        const IF = 1 << 9;
        /// Direction Flag
        const DF = 1 << 10;
        /// Overflow Flag
        const OF = 1 << 11;
        /// I/O Privilege Level (bit 0)
        const IOPL0 = 1 << 12;
        /// I/O Privilege Level (bit 1)
        const IOPL1 = 1 << 13;
        /// Nested Task
        const NT = 1 << 14;
        /// Resume Flag
        const RF = 1 << 16;
        /// Virtual-8086 Mode
        const VM = 1 << 17;
        /// Alignment Check / Access Control
        const AC = 1 << 18;
        /// Virtual Interrupt Flag
        const VIF = 1 << 19;
        /// Virtual Interrupt Pending
        const VIP = 1 << 20;
        /// ID Flag (CPUID available)
        const ID = 1 << 21;
    }
}

impl RFlags {
    /// Read current RFLAGS
    #[inline]
    pub fn read() -> Self {
        let value: u64;
        unsafe {
            asm!(
                "pushfq",
                "pop {}",
                out(reg) value,
                options(preserves_flags)
            );
        }
        Self::from_bits_truncate(value)
    }

    /// Write RFLAGS
    ///
    /// # Safety
    /// Can affect interrupt handling, privilege levels, and execution flow.
    #[inline]
    pub unsafe fn write(self) {
        unsafe {
            asm!(
                "push {}",
                "popfq",
                in(reg) self.bits(),
                options(preserves_flags)
            );
        }
    }

    /// Check if interrupts are enabled
    pub fn interrupts_enabled(self) -> bool {
        self.contains(Self::IF)
    }

    /// Get I/O privilege level
    pub fn iopl(self) -> u8 {
        ((self.bits() >> 12) & 3) as u8
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cr0_read() {
        let cr0 = Cr0::read();
        // In 64-bit mode, PE and PG must be set
        assert!(cr0.protected_mode());
        assert!(cr0.paging_enabled());
    }

    #[test]
    fn test_cr3_read() {
        let cr3 = Cr3::read();
        // Page table base should be non-zero and page-aligned
        assert!(cr3.base_address() > 0);
        assert_eq!(cr3.base_address() & 0xFFF, 0);
    }

    #[test]
    fn test_cr4_read() {
        let cr4 = Cr4::read();
        // PAE must be enabled for 64-bit mode
        assert!(cr4.contains(Cr4::PAE));
    }

    #[test]
    fn test_rflags_read() {
        let rflags = RFlags::read();
        // ID flag should be settable on x86_64
        assert!(rflags.bits() & 0x2 == 0x2); // Reserved bit is always 1
    }

    #[test]
    fn test_pause() {
        // Just ensure it doesn't crash
        pause();
    }
}
