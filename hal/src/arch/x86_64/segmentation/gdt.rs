//! # Global Descriptor Table (GDT)
//!
//! 64-bit GDT management for x86_64.
//!
//! ## Overview
//!
//! In 64-bit mode, segmentation is mostly disabled, but we still need:
//!
//! - A valid code segment for CS (with L=1 for long mode)
//! - A valid data segment for SS/DS/ES (mostly ignored but must exist)
//! - A TSS segment for privilege level switching and IST
//!
//! ## Segment Layout
//!
//! ```text
//! Index  Selector   Segment               Access
//! ─────────────────────────────────────────────────
//! 0      0x00       Null                  -
//! 1      0x08       Kernel Code (64-bit)  P=1 DPL=0 C=1 L=1
//! 2      0x10       Kernel Data           P=1 DPL=0 W=1
//! 3      0x18       User Data             P=1 DPL=3 W=1
//! 4      0x20       User Code (64-bit)    P=1 DPL=3 C=1 L=1
//! 5-6    0x28       TSS (16 bytes)        System descriptor
//! ```
//!
//! Note: User data comes before user code for SYSRET compatibility.

use core::mem::size_of;

use super::tss::{Tss, TssEntry, TSS_SIZE};

// =============================================================================
// CONSTANTS
// =============================================================================

/// Number of standard GDT entries (null + 4 segments + TSS(2))
pub const GDT_ENTRY_COUNT: usize = 7;

/// Size of a single GDT entry (8 bytes)
pub const GDT_ENTRY_SIZE: usize = 8;

// =============================================================================
// DESCRIPTOR FLAGS
// =============================================================================

bitflags::bitflags! {
    /// GDT Descriptor Flags (upper 8 bits of entry)
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DescriptorFlags: u8 {
        /// Granularity (0=byte, 1=4KB pages)
        const GRANULARITY = 1 << 7;
        /// Size (0=16-bit, 1=32-bit) - must be 0 for 64-bit code
        const SIZE_32 = 1 << 6;
        /// Long mode (1=64-bit code segment)
        const LONG_MODE = 1 << 5;
        /// Available for system use
        const AVAILABLE = 1 << 4;
    }
}

// =============================================================================
// ACCESS BYTE
// =============================================================================

bitflags::bitflags! {
    /// GDT Access Byte
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct AccessByte: u8 {
        /// Present bit - must be 1 for valid segments
        const PRESENT = 1 << 7;
        /// Privilege level bit 0
        const DPL_0 = 1 << 5;
        /// Privilege level bit 1
        const DPL_1 = 1 << 6;
        /// Descriptor type (0=system, 1=code/data)
        const DESCRIPTOR_TYPE = 1 << 4;
        /// Executable (1=code, 0=data)
        const EXECUTABLE = 1 << 3;
        /// Direction/Conforming
        /// - Data: 0=grows up, 1=grows down
        /// - Code: 0=non-conforming, 1=conforming
        const DIRECTION_CONFORMING = 1 << 2;
        /// Readable (code) / Writable (data)
        const READABLE_WRITABLE = 1 << 1;
        /// Accessed (set by CPU)
        const ACCESSED = 1 << 0;
    }
}

impl AccessByte {
    /// Ring 0 (kernel mode)
    pub const RING0: Self = Self::empty();
    /// Ring 3 (user mode)
    pub const RING3: Self = Self::DPL_0.union(Self::DPL_1);

    /// Set DPL
    pub const fn with_dpl(mut self, dpl: u8) -> Self {
        self.0 .0 = (self.0 .0 & !0x60) | ((dpl & 3) << 5);
        self
    }
}

// =============================================================================
// DESCRIPTOR TYPE
// =============================================================================

/// Segment descriptor type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptorType {
    /// Null descriptor
    Null,
    /// 64-bit code segment
    Code64,
    /// Data segment (readable/writable)
    Data,
    /// TSS (available)
    TssAvailable,
    /// TSS (busy)
    TssBusy,
}

// =============================================================================
// GDT ENTRY
// =============================================================================

/// A single 8-byte GDT entry
#[repr(C, packed)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct GdtEntry {
    data: u64,
}

impl GdtEntry {
    /// Create a null descriptor
    pub const fn null() -> Self {
        Self { data: 0 }
    }

    /// Create a kernel code segment (64-bit)
    pub const fn kernel_code() -> Self {
        // Access: Present, DPL=0, Code segment, Executable, Readable
        // Flags: Long mode (64-bit)
        let access = AccessByte::PRESENT.bits()
            | AccessByte::DESCRIPTOR_TYPE.bits()
            | AccessByte::EXECUTABLE.bits()
            | AccessByte::READABLE_WRITABLE.bits();
        let flags = DescriptorFlags::LONG_MODE.bits();
        Self::from_parts(0, 0xFFFFF, access, flags)
    }

    /// Create a kernel data segment
    pub const fn kernel_data() -> Self {
        // Access: Present, DPL=0, Data segment, Writable
        let access = AccessByte::PRESENT.bits()
            | AccessByte::DESCRIPTOR_TYPE.bits()
            | AccessByte::READABLE_WRITABLE.bits();
        let flags = DescriptorFlags::GRANULARITY.bits();
        Self::from_parts(0, 0xFFFFF, access, flags)
    }

    /// Create a user code segment (64-bit)
    pub const fn user_code() -> Self {
        // Access: Present, DPL=3, Code segment, Executable, Readable
        // Flags: Long mode (64-bit)
        let access = AccessByte::PRESENT.bits()
            | AccessByte::DPL_0.bits()
            | AccessByte::DPL_1.bits()
            | AccessByte::DESCRIPTOR_TYPE.bits()
            | AccessByte::EXECUTABLE.bits()
            | AccessByte::READABLE_WRITABLE.bits();
        let flags = DescriptorFlags::LONG_MODE.bits();
        Self::from_parts(0, 0xFFFFF, access, flags)
    }

    /// Create a user data segment
    pub const fn user_data() -> Self {
        // Access: Present, DPL=3, Data segment, Writable
        let access = AccessByte::PRESENT.bits()
            | AccessByte::DPL_0.bits()
            | AccessByte::DPL_1.bits()
            | AccessByte::DESCRIPTOR_TYPE.bits()
            | AccessByte::READABLE_WRITABLE.bits();
        let flags = DescriptorFlags::GRANULARITY.bits();
        Self::from_parts(0, 0xFFFFF, access, flags)
    }

    /// Create from individual parts
    const fn from_parts(base: u32, limit: u32, access: u8, flags: u8) -> Self {
        let mut data: u64 = 0;

        // Limit bits 0-15
        data |= (limit & 0xFFFF) as u64;
        // Base bits 0-15
        data |= ((base & 0xFFFF) as u64) << 16;
        // Base bits 16-23
        data |= (((base >> 16) & 0xFF) as u64) << 32;
        // Access byte
        data |= (access as u64) << 40;
        // Limit bits 16-19 + Flags
        data |= (((limit >> 16) & 0x0F) as u64) << 48;
        data |= ((flags & 0xF0) as u64) << 48;
        // Base bits 24-31
        data |= (((base >> 24) & 0xFF) as u64) << 56;

        Self { data }
    }

    /// Get the raw u64 value
    pub const fn raw(self) -> u64 {
        self.data
    }

    /// Check if this is a null descriptor
    pub const fn is_null(self) -> bool {
        self.data == 0
    }
}

impl core::fmt::Debug for GdtEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "GdtEntry({:#018x})", self.data)
    }
}

// =============================================================================
// GDT DESCRIPTOR (for LGDT)
// =============================================================================

/// GDT Descriptor for LGDT instruction
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct GdtDescriptor {
    /// Size of GDT - 1
    pub limit: u16,
    /// Base address of GDT
    pub base: u64,
}

impl GdtDescriptor {
    /// Create a new GDT descriptor
    pub const fn new(base: u64, size: u16) -> Self {
        Self {
            limit: size - 1,
            base,
        }
    }
}

impl core::fmt::Debug for GdtDescriptor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("GdtDescriptor")
            .field("limit", &self.limit)
            .field("base", &format_args!("{:#018x}", self.base))
            .finish()
    }
}

// =============================================================================
// GDT TABLE
// =============================================================================

/// Global Descriptor Table
///
/// This is a complete GDT with standard segments and TSS.
#[repr(C, align(8))]
pub struct Gdt {
    /// Null descriptor
    pub null: GdtEntry,
    /// Kernel code segment (ring 0, 64-bit)
    pub kernel_code: GdtEntry,
    /// Kernel data segment (ring 0)
    pub kernel_data: GdtEntry,
    /// User data segment (ring 3)
    /// Note: Comes before user code for SYSRET compatibility
    pub user_data: GdtEntry,
    /// User code segment (ring 3, 64-bit)
    pub user_code: GdtEntry,
    /// TSS entry (16 bytes = 2 GDT entries)
    pub tss: TssEntry,
}

impl Gdt {
    /// Create a new GDT with standard segments
    pub const fn new() -> Self {
        Self {
            null: GdtEntry::null(),
            kernel_code: GdtEntry::kernel_code(),
            kernel_data: GdtEntry::kernel_data(),
            user_data: GdtEntry::user_data(),
            user_code: GdtEntry::user_code(),
            tss: TssEntry::null(),
        }
    }

    /// Set the TSS entry
    pub fn set_tss(&mut self, tss: *const Tss) {
        self.tss = TssEntry::from_tss(tss);
    }

    /// Get the GDT descriptor for LGDT
    pub fn descriptor(&self) -> GdtDescriptor {
        GdtDescriptor::new(self as *const Self as u64, size_of::<Self>() as u16)
    }

    /// Load this GDT
    ///
    /// # Safety
    /// This replaces the current GDT and reloads segment registers.
    pub unsafe fn load(&self) {
        let desc = self.descriptor();
        core::arch::asm!(
            "lgdt [{}]",
            in(reg) &desc,
            options(preserves_flags)
        );
    }

    /// Load and reload segment registers
    ///
    /// # Safety
    /// This replaces the current GDT and all segment registers.
    pub unsafe fn load_and_reload_segments(&self) {
        self.load();

        // Reload CS with a far return
        core::arch::asm!(
            "push {kcs}",
            "lea {tmp}, [rip + 1f]",
            "push {tmp}",
            "retfq",
            "1:",
            kcs = in(reg) super::KERNEL_CS.raw() as u64,
            tmp = lateout(reg) _,
            options(preserves_flags)
        );

        // Reload data segments
        core::arch::asm!(
            "mov ds, {kds:x}",
            "mov es, {kds:x}",
            "mov ss, {kds:x}",
            "xor {zero:e}, {zero:e}",
            "mov fs, {zero:x}",
            "mov gs, {zero:x}",
            kds = in(reg) super::KERNEL_DS.raw(),
            zero = lateout(reg) _,
            options(preserves_flags)
        );
    }

    /// Clear TSS busy flag
    ///
    /// Required before loading TSS with LTR.
    pub fn clear_tss_busy(&mut self) {
        self.tss.clear_busy();
    }
}

impl Default for Gdt {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Debug for Gdt {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Gdt")
            .field("null", &self.null)
            .field("kernel_code", &self.kernel_code)
            .field("kernel_data", &self.kernel_data)
            .field("user_data", &self.user_data)
            .field("user_code", &self.user_code)
            .field("tss", &self.tss)
            .finish()
    }
}

// Verify GDT size at compile time
// 5 * 8 bytes + 16 bytes TSS = 56 bytes
static_assertions::const_assert_eq!(size_of::<Gdt>(), 56);

// =============================================================================
// GDT OPERATIONS
// =============================================================================

/// Store GDT descriptor
pub fn sgdt() -> GdtDescriptor {
    let mut desc = GdtDescriptor { limit: 0, base: 0 };
    unsafe {
        core::arch::asm!(
            "sgdt [{}]",
            in(reg) &mut desc,
            options(nostack, preserves_flags)
        );
    }
    desc
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gdt_entry_size() {
        assert_eq!(size_of::<GdtEntry>(), 8);
    }

    #[test]
    fn test_gdt_size() {
        assert_eq!(size_of::<Gdt>(), 56);
    }

    #[test]
    fn test_null_entry() {
        let entry = GdtEntry::null();
        assert!(entry.is_null());
        assert_eq!(entry.raw(), 0);
    }

    #[test]
    fn test_kernel_code_segment() {
        let entry = GdtEntry::kernel_code();
        let raw = entry.raw();

        // Check L bit (long mode) is set
        assert!((raw >> 53) & 1 == 1);
        // Check D bit (32-bit) is NOT set
        assert!((raw >> 54) & 1 == 0);
        // Check present
        assert!((raw >> 47) & 1 == 1);
        // Check executable
        assert!((raw >> 43) & 1 == 1);
    }

    #[test]
    fn test_user_segments() {
        let code = GdtEntry::user_code();
        let data = GdtEntry::user_data();

        // Check DPL=3 for user code
        let dpl = ((code.raw() >> 45) & 3) as u8;
        assert_eq!(dpl, 3);

        // Check DPL=3 for user data
        let dpl = ((data.raw() >> 45) & 3) as u8;
        assert_eq!(dpl, 3);
    }

    #[test]
    fn test_gdt_new() {
        let gdt = Gdt::new();
        assert!(gdt.null.is_null());
        assert!(!gdt.kernel_code.is_null());
        assert!(!gdt.kernel_data.is_null());
        assert!(!gdt.user_code.is_null());
        assert!(!gdt.user_data.is_null());
    }
}
