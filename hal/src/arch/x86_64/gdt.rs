//! # Global Descriptor Table (GDT)
//!
//! The GDT defines memory segments for x86_64. In long mode, segmentation is
//! mostly disabled, but we still need a valid GDT for:
//! - Code segment (CS) for kernel mode
//! - Data segment (DS/SS) for kernel mode
//! - Code/data segments for user mode
//! - Task State Segment (TSS) for interrupt stack switching
//!
//! ## Memory Layout
//! ```text
//! GDT Entry Layout (8 bytes each, except TSS which is 16 bytes):
//! ┌────────────────────────────────────────────────────┐
//! │ 0x00: Null descriptor (required)                   │
//! │ 0x08: Kernel code segment (64-bit)                 │
//! │ 0x10: Kernel data segment                          │
//! │ 0x18: User code segment (64-bit)                   │
//! │ 0x20: User data segment                            │
//! │ 0x28: TSS descriptor (16 bytes)                    │
//! └────────────────────────────────────────────────────┘
//! ```

use core::arch::asm;
use core::mem::size_of;

/// Kernel code segment selector
pub const KERNEL_CODE_SELECTOR: u16 = 0x08;
/// Kernel data segment selector
pub const KERNEL_DATA_SELECTOR: u16 = 0x10;
/// User data segment selector (with RPL 3) - must be before user code for SYSRET
pub const USER_DATA_SELECTOR: u16 = 0x18 | 3;
/// User code segment selector (with RPL 3)
pub const USER_CODE_SELECTOR: u16 = 0x20 | 3;
/// TSS segment selector
pub const TSS_SELECTOR: u16 = 0x28;

/// Number of IST entries (Interrupt Stack Table)
const IST_ENTRIES: usize = 7;

/// Size of each interrupt stack (16KB)
const INTERRUPT_STACK_SIZE: usize = 16 * 1024;

/// Task State Segment
#[repr(C, packed)]
pub struct TaskStateSegment {
    reserved_0: u32,
    /// Privilege Stack Table (RSP for ring 0, 1, 2)
    pub privilege_stack_table: [u64; 3],
    reserved_1: u64,
    /// Interrupt Stack Table (for IST1-IST7)
    pub interrupt_stack_table: [u64; IST_ENTRIES],
    reserved_2: u64,
    reserved_3: u16,
    /// I/O Map Base Address
    pub iomap_base: u16,
}

impl TaskStateSegment {
    /// Create a new TSS with zeroed stacks
    pub const fn new() -> Self {
        Self {
            reserved_0: 0,
            privilege_stack_table: [0; 3],
            reserved_1: 0,
            interrupt_stack_table: [0; IST_ENTRIES],
            reserved_2: 0,
            reserved_3: 0,
            iomap_base: size_of::<Self>() as u16,
        }
    }
}

/// GDT entry (8 bytes)
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct GdtEntry {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    flags_limit_high: u8,
    base_high: u8,
}

impl GdtEntry {
    /// Create a null entry
    pub const fn null() -> Self {
        Self {
            limit_low: 0,
            base_low: 0,
            base_middle: 0,
            access: 0,
            flags_limit_high: 0,
            base_high: 0,
        }
    }

    /// Create a 64-bit code segment
    pub const fn kernel_code_segment() -> Self {
        Self {
            limit_low: 0xFFFF,
            base_low: 0,
            base_middle: 0,
            // Present | DPL 0 | Code | Executable | Readable
            access: 0b1001_1010,
            // Long mode | 4KB granularity | limit high
            flags_limit_high: 0b1010_1111,
            base_high: 0,
        }
    }

    /// Create a kernel data segment
    pub const fn kernel_data_segment() -> Self {
        Self {
            limit_low: 0xFFFF,
            base_low: 0,
            base_middle: 0,
            // Present | DPL 0 | Data | Writable
            access: 0b1001_0010,
            // 4KB granularity | limit high
            flags_limit_high: 0b1100_1111,
            base_high: 0,
        }
    }

    /// Create a 64-bit user code segment
    pub const fn user_code_segment() -> Self {
        Self {
            limit_low: 0xFFFF,
            base_low: 0,
            base_middle: 0,
            // Present | DPL 3 | Code | Executable | Readable
            access: 0b1111_1010,
            // Long mode | 4KB granularity | limit high
            flags_limit_high: 0b1010_1111,
            base_high: 0,
        }
    }

    /// Create a user data segment
    pub const fn user_data_segment() -> Self {
        Self {
            limit_low: 0xFFFF,
            base_low: 0,
            base_middle: 0,
            // Present | DPL 3 | Data | Writable
            access: 0b1111_0010,
            // 4KB granularity | limit high
            flags_limit_high: 0b1100_1111,
            base_high: 0,
        }
    }
}

/// TSS descriptor (16 bytes for 64-bit mode)
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct TssDescriptor {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    flags_limit_high: u8,
    base_high: u8,
    base_upper: u32,
    reserved: u32,
}

impl TssDescriptor {
    /// Create a TSS descriptor from a TSS address
    pub const fn new(tss_addr: u64, size: u16) -> Self {
        Self {
            limit_low: size,
            base_low: tss_addr as u16,
            base_middle: (tss_addr >> 16) as u8,
            // Present | DPL 0 | TSS Available (0x89)
            access: 0x89,
            flags_limit_high: 0,
            base_high: (tss_addr >> 24) as u8,
            base_upper: (tss_addr >> 32) as u32,
            reserved: 0,
        }
    }
}

/// GDT Pointer structure for LGDT instruction
#[repr(C, packed)]
pub struct GdtPointer {
    /// Size of GDT - 1
    pub limit: u16,
    /// Base address of GDT
    pub base: u64,
}

/// The Global Descriptor Table
#[repr(C, align(16))]
pub struct Gdt {
    entries: [GdtEntry; 5],
    tss_entry: TssDescriptor,
}

impl Gdt {
    /// Create a new GDT
    pub const fn new() -> Self {
        Self {
            entries: [
                GdtEntry::null(),                // 0x00: Null
                GdtEntry::kernel_code_segment(), // 0x08: Kernel code
                GdtEntry::kernel_data_segment(), // 0x10: Kernel data
                GdtEntry::user_data_segment(),   // 0x18: User data (must be before user code for SYSRET)
                GdtEntry::user_code_segment(),   // 0x20: User code
            ],
            tss_entry: TssDescriptor::new(0, 0), // Will be set during init
        }
    }

    /// Set the TSS descriptor
    pub fn set_tss(&mut self, tss: &TaskStateSegment) {
        let tss_addr = tss as *const _ as u64;
        let tss_size = (size_of::<TaskStateSegment>() - 1) as u16;
        self.tss_entry = TssDescriptor::new(tss_addr, tss_size);
    }

    /// Get a pointer for the LGDT instruction
    pub fn pointer(&self) -> GdtPointer {
        GdtPointer {
            limit: (size_of::<Self>() - 1) as u16,
            base: self as *const _ as u64,
        }
    }
}

// =============================================================================
// Global State
// =============================================================================

/// Static GDT - must be static because CPU references it
static mut GDT: Gdt = Gdt::new();

/// Static TSS
static mut TSS: TaskStateSegment = TaskStateSegment::new();

/// Interrupt stack for double faults (IST1)
#[repr(align(16))]
struct InterruptStack([u8; INTERRUPT_STACK_SIZE]);

static mut DOUBLE_FAULT_STACK: InterruptStack = InterruptStack([0; INTERRUPT_STACK_SIZE]);
static mut PAGE_FAULT_STACK: InterruptStack = InterruptStack([0; INTERRUPT_STACK_SIZE]);

// =============================================================================
// Initialization
// =============================================================================

/// Kernel stack for Ring 3 -> Ring 0 transitions
#[repr(align(16))]
struct KernelRing0Stack([u8; 32 * 1024]); // 32KB kernel stack

static mut KERNEL_RING0_STACK: KernelRing0Stack = KernelRing0Stack([0; 32 * 1024]);

/// Initialize the GDT and TSS
/// 
/// # Safety
/// Must be called only once during early boot.
pub unsafe fn init() {
    unsafe {
        // Set up interrupt stacks in TSS
        let double_fault_stack_end = DOUBLE_FAULT_STACK.0.as_ptr() as u64 
            + INTERRUPT_STACK_SIZE as u64;
        let page_fault_stack_end = PAGE_FAULT_STACK.0.as_ptr() as u64 
            + INTERRUPT_STACK_SIZE as u64;
        
        TSS.interrupt_stack_table[0] = double_fault_stack_end; // IST1 for double fault
        TSS.interrupt_stack_table[1] = page_fault_stack_end;   // IST2 for page fault
        
        // Set RSP0 - the kernel stack used when transitioning from Ring 3 to Ring 0
        let kernel_stack_top = KERNEL_RING0_STACK.0.as_ptr() as u64 + 32 * 1024;
        TSS.privilege_stack_table[0] = kernel_stack_top; // RSP0
        
        // Set the TSS in the GDT
        GDT.set_tss(&TSS);
        
        // Load the GDT
        let gdt_ptr = GDT.pointer();
        asm!(
            "lgdt [{}]",
            in(reg) &gdt_ptr,
            options(readonly, nostack, preserves_flags)
        );
        
        // Reload segment registers
        // CS is loaded via a far return
        asm!(
            "push {sel}",           // Push code segment selector
            "lea {tmp}, [rip + 2f]", // Get address of label 2
            "push {tmp}",           // Push return address
            "retfq",                // Far return to reload CS
            "2:",
            sel = in(reg) KERNEL_CODE_SELECTOR as u64,
            tmp = lateout(reg) _,
            options(preserves_flags),
        );
        
        // Load other segment registers
        asm!(
            "mov ds, {0:x}",
            "mov es, {0:x}",
            "mov fs, {0:x}",
            "mov gs, {0:x}",
            "mov ss, {0:x}",
            in(reg) KERNEL_DATA_SELECTOR,
            options(nostack, preserves_flags),
        );
        
        // Load TSS
        asm!(
            "ltr {0:x}",
            in(reg) TSS_SELECTOR,
            options(nostack, preserves_flags),
        );
        
        log::debug!("GDT initialized at {:p}", &GDT);
        log::debug!("TSS initialized with RSP0={:#x}", kernel_stack_top);
    }
}

/// Update RSP0 in TSS (for task switching with different kernel stacks)
/// 
/// # Safety
/// Must be called with a valid kernel stack pointer.
pub unsafe fn set_kernel_stack(stack_top: u64) {
    unsafe {
        TSS.privilege_stack_table[0] = stack_top;
    }
}

/// Get the kernel code segment selector
pub fn kernel_code_selector() -> u16 {
    KERNEL_CODE_SELECTOR
}

/// Get the kernel data segment selector
pub fn kernel_data_selector() -> u16 {
    KERNEL_DATA_SELECTOR
}
