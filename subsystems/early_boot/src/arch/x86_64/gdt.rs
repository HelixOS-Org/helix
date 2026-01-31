//! # x86_64 GDT and TSS Setup
//!
//! Global Descriptor Table and Task State Segment initialization.

use core::mem::size_of;

use super::*;
use crate::core::BootContext;
use crate::error::{BootError, BootResult};

// =============================================================================
// GDT ENTRY TYPES
// =============================================================================

/// 64-bit GDT entry
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct GdtEntry {
    /// Limit bits 0-15
    pub limit_low: u16,
    /// Base bits 0-15
    pub base_low: u16,
    /// Base bits 16-23
    pub base_mid: u8,
    /// Access byte
    pub access: u8,
    /// Limit bits 16-19 and flags
    pub flags_limit: u8,
    /// Base bits 24-31
    pub base_high: u8,
}

impl GdtEntry {
    /// Create a null entry
    pub const fn null() -> Self {
        Self {
            limit_low: 0,
            base_low: 0,
            base_mid: 0,
            access: 0,
            flags_limit: 0,
            base_high: 0,
        }
    }

    /// Create a 64-bit code segment
    pub const fn code64(dpl: u8) -> Self {
        Self {
            limit_low: 0xFFFF,
            base_low: 0,
            base_mid: 0,
            access: 0x9A | ((dpl & 3) << 5), // Present, executable, readable
            flags_limit: 0xAF,               // Long mode, 4KB granularity
            base_high: 0,
        }
    }

    /// Create a 64-bit data segment
    pub const fn data64(dpl: u8) -> Self {
        Self {
            limit_low: 0xFFFF,
            base_low: 0,
            base_mid: 0,
            access: 0x92 | ((dpl & 3) << 5), // Present, writable
            flags_limit: 0xCF,               // 4KB granularity
            base_high: 0,
        }
    }

    /// Create a 32-bit code segment (for compatibility mode)
    pub const fn code32(dpl: u8) -> Self {
        Self {
            limit_low: 0xFFFF,
            base_low: 0,
            base_mid: 0,
            access: 0x9A | ((dpl & 3) << 5),
            flags_limit: 0xCF, // 32-bit, 4KB granularity
            base_high: 0,
        }
    }
}

/// System segment descriptor (for TSS)
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct SystemDescriptor {
    /// Low 8 bytes
    pub low: GdtEntry,
    /// Base bits 32-63
    pub base_upper: u32,
    /// Reserved
    pub reserved: u32,
}

impl SystemDescriptor {
    /// Create a TSS descriptor
    pub fn tss(base: u64, limit: u32) -> Self {
        Self {
            low: GdtEntry {
                limit_low: (limit & 0xFFFF) as u16,
                base_low: (base & 0xFFFF) as u16,
                base_mid: ((base >> 16) & 0xFF) as u8,
                access: 0x89, // Present, 64-bit TSS available
                flags_limit: ((limit >> 16) & 0x0F) as u8,
                base_high: ((base >> 24) & 0xFF) as u8,
            },
            base_upper: (base >> 32) as u32,
            reserved: 0,
        }
    }
}

// =============================================================================
// TASK STATE SEGMENT
// =============================================================================

/// 64-bit Task State Segment
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Tss {
    /// Reserved
    pub reserved0: u32,
    /// RSP for privilege level 0
    pub rsp0: u64,
    /// RSP for privilege level 1
    pub rsp1: u64,
    /// RSP for privilege level 2
    pub rsp2: u64,
    /// Reserved
    pub reserved1: u64,
    /// Interrupt Stack Table pointers (IST1-IST7)
    pub ist: [u64; 7],
    /// Reserved
    pub reserved2: u64,
    /// Reserved
    pub reserved3: u16,
    /// I/O Map Base Address
    pub iomap_base: u16,
}

impl Tss {
    /// Create a new TSS
    pub const fn new() -> Self {
        Self {
            reserved0: 0,
            rsp0: 0,
            rsp1: 0,
            rsp2: 0,
            reserved1: 0,
            ist: [0; 7],
            reserved2: 0,
            reserved3: 0,
            iomap_base: size_of::<Tss>() as u16,
        }
    }

    /// Set the kernel stack pointer (RSP0)
    pub fn set_kernel_stack(&mut self, stack: u64) {
        self.rsp0 = stack;
    }

    /// Set an Interrupt Stack Table entry
    pub fn set_ist(&mut self, index: usize, stack: u64) {
        if index > 0 && index <= 7 {
            self.ist[index - 1] = stack;
        }
    }
}

// =============================================================================
// GDT POINTER
// =============================================================================

/// GDT/IDT pointer structure
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct DescriptorTablePointer {
    /// Size of the table minus 1
    pub limit: u16,
    /// Linear base address
    pub base: u64,
}

// =============================================================================
// GLOBAL GDT STRUCTURE
// =============================================================================

/// Complete GDT with TSS
#[repr(C, align(16))]
pub struct Gdt {
    /// Null descriptor
    pub null: GdtEntry,
    /// Kernel code segment (selector 0x08)
    pub kernel_code: GdtEntry,
    /// Kernel data segment (selector 0x10)
    pub kernel_data: GdtEntry,
    /// User code segment 32-bit (selector 0x18)
    pub user_code32: GdtEntry,
    /// User data segment (selector 0x20)
    pub user_data: GdtEntry,
    /// User code segment 64-bit (selector 0x28)
    pub user_code64: GdtEntry,
    /// TSS (selector 0x30, takes 2 slots)
    pub tss: SystemDescriptor,
}

impl Gdt {
    /// Create a new GDT
    pub const fn new() -> Self {
        Self {
            null: GdtEntry::null(),
            kernel_code: GdtEntry::code64(0),
            kernel_data: GdtEntry::data64(0),
            user_code32: GdtEntry::code32(3),
            user_data: GdtEntry::data64(3),
            user_code64: GdtEntry::code64(3),
            tss: SystemDescriptor {
                low: GdtEntry::null(),
                base_upper: 0,
                reserved: 0,
            },
        }
    }

    /// Set TSS in the GDT
    pub fn set_tss(&mut self, tss_ptr: &'static Tss) {
        let base = tss_ptr as *const Tss as u64;
        let limit = (size_of::<Tss>() - 1) as u32;
        self.tss = SystemDescriptor::tss(base, limit);
    }

    /// Get GDT pointer for LGDT
    pub fn pointer(&self) -> DescriptorTablePointer {
        DescriptorTablePointer {
            limit: (size_of::<Self>() - 1) as u16,
            base: self as *const _ as u64,
        }
    }
}

// =============================================================================
// STATIC GDT AND TSS
// =============================================================================

/// Static GDT (per-CPU would need an array)
static mut GDT: Gdt = Gdt::new();

/// Static TSS for BSP
static mut BSP_TSS: Tss = Tss::new();

/// Boot stack for kernel
#[repr(C, align(4096))]
struct KernelStack {
    data: [u8; 32768], // 32KB stack
}

static mut KERNEL_STACK: KernelStack = KernelStack { data: [0; 32768] };

/// Interrupt stacks (IST)
#[repr(C, align(4096))]
struct InterruptStack {
    data: [u8; 16384], // 16KB per IST
}

static mut IST_STACKS: [InterruptStack; 7] = [
    InterruptStack { data: [0; 16384] },
    InterruptStack { data: [0; 16384] },
    InterruptStack { data: [0; 16384] },
    InterruptStack { data: [0; 16384] },
    InterruptStack { data: [0; 16384] },
    InterruptStack { data: [0; 16384] },
    InterruptStack { data: [0; 16384] },
];

// =============================================================================
// GDT INITIALIZATION
// =============================================================================

/// Initialize GDT with TSS
pub unsafe fn init_gdt(ctx: &mut BootContext) -> BootResult<()> {
    // Set up kernel stack in TSS
    let kernel_stack_top = (&raw const KERNEL_STACK as u64) + 32768;
    BSP_TSS.set_kernel_stack(kernel_stack_top);

    // Set up IST stacks
    for i in 0..7 {
        let ist_top = (&raw const IST_STACKS[i] as u64) + 16384;
        BSP_TSS.set_ist(i + 1, ist_top);
    }

    // Set TSS in GDT
    GDT.set_tss(&BSP_TSS);

    // Load GDT
    let gdt_ptr = GDT.pointer();
    core::arch::asm!(
        "lgdt [{}]",
        in(reg) &gdt_ptr,
        options(nostack)
    );

    // Reload segment registers
    reload_segments();

    // Load TSS
    load_tss(TSS_SEL);

    // Store in context
    ctx.arch_data.x86.gdt_base = &raw const GDT as u64;
    ctx.arch_data.x86.gdt_limit = gdt_ptr.limit;
    ctx.arch_data.x86.tss_base = &raw const BSP_TSS as u64;

    Ok(())
}

/// Reload segment registers after GDT change
unsafe fn reload_segments() {
    // Reload CS via far return
    core::arch::asm!(
        "push {sel}",
        "lea {tmp}, [rip + 2f]",
        "push {tmp}",
        "retfq",
        "2:",
        sel = in(reg) KERNEL_CS as u64,
        tmp = lateout(reg) _,
        options(preserves_flags)
    );

    // Reload data segments
    core::arch::asm!(
        "mov ds, {sel:x}",
        "mov es, {sel:x}",
        "mov ss, {sel:x}",
        sel = in(reg) KERNEL_DS as u32,
        options(nostack, preserves_flags)
    );

    // Clear FS and GS
    core::arch::asm!(
        "xor eax, eax",
        "mov fs, ax",
        "mov gs, ax",
        out("eax") _,
        options(nostack, preserves_flags)
    );
}

/// Load the Task State Segment
unsafe fn load_tss(selector: u16) {
    core::arch::asm!(
        "ltr {sel:x}",
        sel = in(reg) selector as u32,
        options(nostack, preserves_flags)
    );
}

// =============================================================================
// PER-CPU GDT
// =============================================================================

/// Per-CPU GDT entry
#[repr(C, align(16))]
pub struct PerCpuGdt {
    /// The GDT itself
    pub gdt: Gdt,
    /// The TSS
    pub tss: Tss,
    /// Kernel stack
    kernel_stack: [u8; 32768],
    /// IST stacks
    ist_stacks: [[u8; 16384]; 7],
}

impl PerCpuGdt {
    /// Create a new per-CPU GDT/TSS
    pub const fn new() -> Self {
        Self {
            gdt: Gdt::new(),
            tss: Tss::new(),
            kernel_stack: [0; 32768],
            ist_stacks: [[0; 16384]; 7],
        }
    }

    /// Initialize this per-CPU GDT
    pub unsafe fn init(&mut self) {
        // Set up kernel stack
        let stack_top = self.kernel_stack.as_ptr().add(32768) as u64;
        self.tss.set_kernel_stack(stack_top);

        // Set up IST stacks
        for i in 0..7 {
            let ist_top = self.ist_stacks[i].as_ptr().add(16384) as u64;
            self.tss.set_ist(i + 1, ist_top);
        }

        // Set TSS pointer in GDT
        let tss_base = &self.tss as *const Tss as u64;
        let tss_limit = (size_of::<Tss>() - 1) as u32;
        self.gdt.tss = SystemDescriptor::tss(tss_base, tss_limit);
    }

    /// Load this GDT and TSS for the current CPU
    pub unsafe fn load(&self) {
        let gdt_ptr = self.gdt.pointer();
        core::arch::asm!(
            "lgdt [{}]",
            in(reg) &gdt_ptr,
            options(nostack)
        );

        reload_segments();
        load_tss(TSS_SEL);
    }
}

// =============================================================================
// SEGMENT SELECTOR HELPERS
// =============================================================================

/// Create a segment selector
pub const fn selector(index: u16, ti: u16, rpl: u16) -> u16 {
    (index << 3) | (ti << 2) | rpl
}

/// Get the DPL from a segment selector
pub const fn selector_rpl(sel: u16) -> u16 {
    sel & 3
}

/// Check if selector is for LDT
pub const fn selector_is_ldt(sel: u16) -> bool {
    (sel & 4) != 0
}
