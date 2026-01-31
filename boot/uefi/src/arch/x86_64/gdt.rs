//! x86_64 Global Descriptor Table (GDT)
//!
//! Complete GDT implementation for long mode.

use core::mem::size_of;

// =============================================================================
// SEGMENT SELECTORS
// =============================================================================

/// Segment selector indices
pub mod selectors {
    /// Null descriptor
    pub const NULL: u16 = 0x00;
    /// Kernel code segment (64-bit)
    pub const KERNEL_CODE: u16 = 0x08;
    /// Kernel data segment
    pub const KERNEL_DATA: u16 = 0x10;
    /// User code segment (32-bit, for compatibility mode)
    pub const USER_CODE32: u16 = 0x18;
    /// User data segment
    pub const USER_DATA: u16 = 0x20;
    /// User code segment (64-bit)
    pub const USER_CODE64: u16 = 0x28;
    /// TSS descriptor (takes 2 entries)
    pub const TSS: u16 = 0x30;

    /// Get selector with RPL
    pub const fn with_rpl(selector: u16, rpl: u16) -> u16 {
        (selector & !3) | (rpl & 3)
    }

    /// Kernel RPL
    pub const fn kernel(selector: u16) -> u16 {
        with_rpl(selector, 0)
    }

    /// User RPL
    pub const fn user(selector: u16) -> u16 {
        with_rpl(selector, 3)
    }
}

// =============================================================================
// SEGMENT DESCRIPTOR
// =============================================================================

/// Segment descriptor access byte flags
pub mod access {
    /// Segment is accessed
    pub const ACCESSED: u8 = 1 << 0;
    /// Readable (code) or Writable (data)
    pub const RW: u8 = 1 << 1;
    /// Direction/Conforming
    pub const DC: u8 = 1 << 2;
    /// Executable (code segment)
    pub const EXECUTABLE: u8 = 1 << 3;
    /// Code/Data segment (not system)
    pub const CODE_DATA: u8 = 1 << 4;
    /// DPL bit 0
    pub const DPL_0: u8 = 0 << 5;
    /// DPL bit 1 (ring 1)
    pub const DPL_1: u8 = 1 << 5;
    /// DPL bit 2 (ring 2)
    pub const DPL_2: u8 = 2 << 5;
    /// DPL bit 3 (ring 3)
    pub const DPL_3: u8 = 3 << 5;
    /// Segment present
    pub const PRESENT: u8 = 1 << 7;

    /// Kernel code segment
    pub const KERNEL_CODE: u8 = PRESENT | CODE_DATA | EXECUTABLE | RW;
    /// Kernel data segment
    pub const KERNEL_DATA: u8 = PRESENT | CODE_DATA | RW;
    /// User code segment
    pub const USER_CODE: u8 = PRESENT | CODE_DATA | EXECUTABLE | RW | DPL_3;
    /// User data segment
    pub const USER_DATA: u8 = PRESENT | CODE_DATA | RW | DPL_3;
}

/// Segment descriptor flags byte
pub mod flags {
    /// Long mode (64-bit code)
    pub const LONG_MODE: u8 = 1 << 1;
    /// Size bit (32-bit if set, 16-bit if clear)
    pub const SIZE_32: u8 = 1 << 2;
    /// Granularity (4KB if set, byte if clear)
    pub const GRANULARITY: u8 = 1 << 3;

    /// 64-bit code segment flags
    pub const CODE64: u8 = LONG_MODE | GRANULARITY;
    /// 32-bit code segment flags
    pub const CODE32: u8 = SIZE_32 | GRANULARITY;
    /// Data segment flags
    pub const DATA: u8 = GRANULARITY;
}

/// Segment descriptor (8 bytes)
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SegmentDescriptor {
    /// Limit bits 0-15
    limit_low: u16,
    /// Base bits 0-15
    base_low: u16,
    /// Base bits 16-23
    base_middle: u8,
    /// Access byte
    access: u8,
    /// Limit bits 16-19 and flags
    limit_flags: u8,
    /// Base bits 24-31
    base_high: u8,
}

impl SegmentDescriptor {
    /// Create null descriptor
    pub const fn null() -> Self {
        Self {
            limit_low: 0,
            base_low: 0,
            base_middle: 0,
            access: 0,
            limit_flags: 0,
            base_high: 0,
        }
    }

    /// Create new segment descriptor
    pub const fn new(base: u32, limit: u32, access_byte: u8, flags_byte: u8) -> Self {
        Self {
            limit_low: limit as u16,
            base_low: base as u16,
            base_middle: (base >> 16) as u8,
            access: access_byte,
            limit_flags: ((limit >> 16) as u8 & 0x0F) | (flags_byte << 4),
            base_high: (base >> 24) as u8,
        }
    }

    /// Create 64-bit kernel code descriptor
    pub const fn kernel_code64() -> Self {
        Self::new(0, 0xFFFFF, access::KERNEL_CODE, flags::CODE64)
    }

    /// Create kernel data descriptor
    pub const fn kernel_data() -> Self {
        Self::new(0, 0xFFFFF, access::KERNEL_DATA, flags::DATA)
    }

    /// Create 32-bit user code descriptor
    pub const fn user_code32() -> Self {
        Self::new(0, 0xFFFFF, access::USER_CODE, flags::CODE32)
    }

    /// Create user data descriptor
    pub const fn user_data() -> Self {
        Self::new(0, 0xFFFFF, access::USER_DATA, flags::DATA)
    }

    /// Create 64-bit user code descriptor
    pub const fn user_code64() -> Self {
        Self::new(0, 0xFFFFF, access::USER_CODE, flags::CODE64)
    }

    /// Get base address
    pub fn base(&self) -> u32 {
        (self.base_low as u32) |
        ((self.base_middle as u32) << 16) |
        ((self.base_high as u32) << 24)
    }

    /// Get limit
    pub fn limit(&self) -> u32 {
        (self.limit_low as u32) |
        (((self.limit_flags & 0x0F) as u32) << 16)
    }

    /// Check if present
    pub fn is_present(&self) -> bool {
        (self.access & access::PRESENT) != 0
    }

    /// Get DPL
    pub fn dpl(&self) -> u8 {
        (self.access >> 5) & 3
    }
}

// =============================================================================
// TSS DESCRIPTOR (64-bit)
// =============================================================================

/// System segment types
pub mod system_type {
    /// Available 64-bit TSS
    pub const TSS64_AVAILABLE: u8 = 0x9;
    /// Busy 64-bit TSS
    pub const TSS64_BUSY: u8 = 0xB;
    /// LDT
    pub const LDT: u8 = 0x2;
    /// Call gate
    pub const CALL_GATE: u8 = 0xC;
    /// Interrupt gate
    pub const INTERRUPT_GATE: u8 = 0xE;
    /// Trap gate
    pub const TRAP_GATE: u8 = 0xF;
}

/// TSS descriptor (16 bytes in 64-bit mode)
#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct TssDescriptor {
    /// Low 8 bytes (like SegmentDescriptor)
    low: SegmentDescriptor,
    /// Base bits 32-63
    base_upper: u32,
    /// Reserved (must be zero)
    reserved: u32,
}

impl TssDescriptor {
    /// Create new TSS descriptor
    pub fn new(base: u64, limit: u32) -> Self {
        let base32 = base as u32;

        Self {
            low: SegmentDescriptor {
                limit_low: limit as u16,
                base_low: base32 as u16,
                base_middle: (base32 >> 16) as u8,
                access: access::PRESENT | system_type::TSS64_AVAILABLE,
                limit_flags: ((limit >> 16) as u8 & 0x0F),
                base_high: (base32 >> 24) as u8,
            },
            base_upper: (base >> 32) as u32,
            reserved: 0,
        }
    }

    /// Get base address
    pub fn base(&self) -> u64 {
        (self.low.base() as u64) | ((self.base_upper as u64) << 32)
    }

    /// Get limit
    pub fn limit(&self) -> u32 {
        self.low.limit()
    }
}

// =============================================================================
// TASK STATE SEGMENT (64-bit)
// =============================================================================

/// 64-bit Task State Segment
#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct TaskStateSegment {
    /// Reserved
    reserved1: u32,
    /// Privilege stack table (RSP0, RSP1, RSP2)
    pub privilege_stack_table: [u64; 3],
    /// Reserved
    reserved2: u64,
    /// Interrupt stack table (IST1-IST7)
    pub interrupt_stack_table: [u64; 7],
    /// Reserved
    reserved3: u64,
    /// Reserved
    reserved4: u16,
    /// I/O map base address
    pub iomap_base: u16,
}

impl TaskStateSegment {
    /// TSS size
    pub const SIZE: usize = size_of::<Self>();

    /// Create new TSS
    pub const fn new() -> Self {
        Self {
            reserved1: 0,
            privilege_stack_table: [0; 3],
            reserved2: 0,
            interrupt_stack_table: [0; 7],
            reserved3: 0,
            reserved4: 0,
            iomap_base: Self::SIZE as u16, // No I/O bitmap
        }
    }

    /// Set RSP0 (kernel stack for ring 0 transitions)
    pub fn set_rsp0(&mut self, rsp: u64) {
        self.privilege_stack_table[0] = rsp;
    }

    /// Get RSP0
    pub fn rsp0(&self) -> u64 {
        self.privilege_stack_table[0]
    }

    /// Set IST entry (1-7)
    pub fn set_ist(&mut self, index: usize, stack: u64) {
        if index >= 1 && index <= 7 {
            self.interrupt_stack_table[index - 1] = stack;
        }
    }

    /// Get IST entry (1-7)
    pub fn ist(&self, index: usize) -> u64 {
        if index >= 1 && index <= 7 {
            self.interrupt_stack_table[index - 1]
        } else {
            0
        }
    }
}

impl Default for TaskStateSegment {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// GDT STRUCTURE
// =============================================================================

/// Number of GDT entries
pub const GDT_ENTRIES: usize = 8; // Including TSS (which takes 2 slots)

/// Global Descriptor Table
#[repr(C, align(8))]
pub struct Gdt {
    /// Descriptors
    entries: [SegmentDescriptor; GDT_ENTRIES],
}

impl Gdt {
    /// Create new GDT with default entries
    pub const fn new() -> Self {
        Self {
            entries: [
                SegmentDescriptor::null(),        // 0x00: Null
                SegmentDescriptor::kernel_code64(), // 0x08: Kernel Code (64-bit)
                SegmentDescriptor::kernel_data(),   // 0x10: Kernel Data
                SegmentDescriptor::user_code32(),   // 0x18: User Code (32-bit)
                SegmentDescriptor::user_data(),     // 0x20: User Data
                SegmentDescriptor::user_code64(),   // 0x28: User Code (64-bit)
                SegmentDescriptor::null(),        // 0x30: TSS low (placeholder)
                SegmentDescriptor::null(),        // 0x38: TSS high (placeholder)
            ],
        }
    }

    /// Set TSS descriptor
    pub fn set_tss(&mut self, tss_addr: u64) {
        let tss_desc = TssDescriptor::new(tss_addr, TaskStateSegment::SIZE as u32 - 1);

        // TSS takes 2 entries in 64-bit mode
        // Safety: We're treating two SegmentDescriptors as one TssDescriptor
        unsafe {
            let ptr = &mut self.entries[6] as *mut SegmentDescriptor as *mut TssDescriptor;
            *ptr = tss_desc;
        }
    }

    /// Get pointer to GDT
    pub fn pointer(&self) -> GdtPointer {
        GdtPointer {
            limit: (size_of::<Self>() - 1) as u16,
            base: self as *const _ as u64,
        }
    }

    /// Load this GDT
    pub unsafe fn load(&self) {
        let ptr = self.pointer();
        load_gdt(&ptr);
    }
}

impl Default for Gdt {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// GDT POINTER
// =============================================================================

/// GDT pointer for LGDT instruction
#[repr(C, packed)]
pub struct GdtPointer {
    /// Size of GDT - 1
    pub limit: u16,
    /// Base address of GDT
    pub base: u64,
}

/// Load GDT
#[inline]
pub unsafe fn load_gdt(gdt: &GdtPointer) {
    core::arch::asm!(
        "lgdt [{}]",
        in(reg) gdt,
        options(nostack, preserves_flags),
    );
}

/// Store GDT
#[inline]
pub fn store_gdt() -> GdtPointer {
    let mut gdt = GdtPointer { limit: 0, base: 0 };
    unsafe {
        core::arch::asm!(
            "sgdt [{}]",
            in(reg) &mut gdt,
            options(nostack, preserves_flags),
        );
    }
    gdt
}

/// Load code segment
#[inline]
pub unsafe fn load_cs(sel: u16) {
    // Use far return to load CS
    core::arch::asm!(
        "push {sel}",
        "lea {tmp}, [rip + 2f]",
        "push {tmp}",
        "retfq",
        "2:",
        sel = in(reg) sel as u64,
        tmp = lateout(reg) _,
        options(preserves_flags),
    );
}

/// Load data segments
#[inline]
pub unsafe fn load_ds(sel: u16) {
    core::arch::asm!("mov ds, {0:x}", in(reg) sel, options(nostack, preserves_flags));
}

#[inline]
pub unsafe fn load_es(sel: u16) {
    core::arch::asm!("mov es, {0:x}", in(reg) sel, options(nostack, preserves_flags));
}

#[inline]
pub unsafe fn load_fs(sel: u16) {
    core::arch::asm!("mov fs, {0:x}", in(reg) sel, options(nostack, preserves_flags));
}

#[inline]
pub unsafe fn load_gs(sel: u16) {
    core::arch::asm!("mov gs, {0:x}", in(reg) sel, options(nostack, preserves_flags));
}

#[inline]
pub unsafe fn load_ss(sel: u16) {
    core::arch::asm!("mov ss, {0:x}", in(reg) sel, options(nostack, preserves_flags));
}

/// Load TSS
#[inline]
pub unsafe fn load_tss(sel: u16) {
    core::arch::asm!("ltr {0:x}", in(reg) sel, options(nostack, preserves_flags));
}

/// Reload all segment registers after loading GDT
pub unsafe fn reload_segments(code_sel: u16, data_sel: u16) {
    load_cs(code_sel);
    load_ds(data_sel);
    load_es(data_sel);
    load_fs(data_sel);
    load_gs(data_sel);
    load_ss(data_sel);
}

// =============================================================================
// STATIC GDT
// =============================================================================

/// Static GDT instance
static mut STATIC_GDT: Gdt = Gdt::new();

/// Static TSS instance
static mut STATIC_TSS: TaskStateSegment = TaskStateSegment::new();

/// Initialize static GDT and TSS
pub unsafe fn init_static() {
    // Set up TSS in GDT
    let tss_addr = &STATIC_TSS as *const _ as u64;
    STATIC_GDT.set_tss(tss_addr);

    // Load GDT
    STATIC_GDT.load();

    // Reload segment registers
    reload_segments(selectors::KERNEL_CODE, selectors::KERNEL_DATA);

    // Load TSS
    load_tss(selectors::TSS);
}

/// Set kernel stack in TSS
pub unsafe fn set_kernel_stack(stack: u64) {
    STATIC_TSS.set_rsp0(stack);
}

/// Set interrupt stack in TSS
pub unsafe fn set_interrupt_stack(ist_index: usize, stack: u64) {
    STATIC_TSS.set_ist(ist_index, stack);
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_descriptor_size() {
        assert_eq!(size_of::<SegmentDescriptor>(), 8);
    }

    #[test]
    fn test_tss_descriptor_size() {
        assert_eq!(size_of::<TssDescriptor>(), 16);
    }

    #[test]
    fn test_tss_size() {
        assert_eq!(TaskStateSegment::SIZE, 104);
    }

    #[test]
    fn test_null_descriptor() {
        let null = SegmentDescriptor::null();
        assert_eq!(null.base(), 0);
        assert_eq!(null.limit(), 0);
        assert!(!null.is_present());
    }

    #[test]
    fn test_kernel_code_descriptor() {
        let desc = SegmentDescriptor::kernel_code64();
        assert!(desc.is_present());
        assert_eq!(desc.dpl(), 0);
        assert_eq!(desc.limit(), 0xFFFFF);
    }

    #[test]
    fn test_user_code_descriptor() {
        let desc = SegmentDescriptor::user_code64();
        assert!(desc.is_present());
        assert_eq!(desc.dpl(), 3);
    }

    #[test]
    fn test_selectors() {
        assert_eq!(selectors::kernel(selectors::KERNEL_CODE), 0x08);
        assert_eq!(selectors::user(selectors::USER_CODE64), 0x2B);
    }

    #[test]
    fn test_tss_creation() {
        let tss = TaskStateSegment::new();
        assert_eq!(tss.rsp0(), 0);
        assert_eq!(tss.iomap_base, TaskStateSegment::SIZE as u16);
    }
}
