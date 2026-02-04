//! # IDT Management
//!
//! This module provides the actual IDT structure and loading functions.
//!
//! ## IDT Structure
//!
//! The IDT is a 256-entry table of gate descriptors. Each entry is 16 bytes,
//! making the total IDT size 4096 bytes (4KB).

use core::mem::size_of;

use super::entries::{GateOptions, GateType, IdtEntry};
use super::vectors::ExceptionVector;
use super::{handlers, segmentation};

// =============================================================================
// IDT Descriptor (for LIDT instruction)
// =============================================================================

/// IDT Descriptor
///
/// This structure is loaded into the IDTR register using the LIDT instruction.
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct IdtDescriptor {
    /// Limit (size of IDT in bytes - 1)
    limit: u16,
    /// Base address of the IDT
    base: u64,
}

impl IdtDescriptor {
    /// Create a new IDT descriptor
    #[inline]
    pub const fn new(base: u64, size: u16) -> Self {
        Self {
            limit: size - 1,
            base,
        }
    }

    /// Get the base address
    #[inline]
    pub const fn base(&self) -> u64 {
        self.base
    }

    /// Get the limit
    #[inline]
    pub const fn limit(&self) -> u16 {
        self.limit
    }
}

// =============================================================================
// IDT Structure
// =============================================================================

/// Number of IDT entries
pub const IDT_SIZE: usize = 256;

/// IDT entry size in bytes
pub const IDT_ENTRY_SIZE: usize = 16;

/// Total IDT size in bytes
pub const IDT_TOTAL_SIZE: usize = IDT_SIZE * IDT_ENTRY_SIZE;

/// The Interrupt Descriptor Table
///
/// This is a 256-entry table of gate descriptors.
#[repr(C, align(16))]
pub struct Idt {
    /// IDT entries
    entries: [IdtEntry; IDT_SIZE],
}

impl Idt {
    /// Create a new empty IDT
    pub const fn new() -> Self {
        Self {
            entries: [IdtEntry::empty(); IDT_SIZE],
        }
    }

    /// Get a reference to an entry
    #[inline]
    pub fn get(&self, vector: u8) -> &IdtEntry {
        &self.entries[vector as usize]
    }

    /// Get a mutable reference to an entry
    #[inline]
    pub fn get_mut(&mut self, vector: u8) -> &mut IdtEntry {
        &mut self.entries[vector as usize]
    }

    /// Set an entry
    #[inline]
    pub fn set(&mut self, vector: u8, entry: IdtEntry) {
        self.entries[vector as usize] = entry;
    }

    /// Set a handler for a vector
    #[inline]
    pub fn set_handler(&mut self, vector: u8, handler: u64, selector: u16, options: GateOptions) {
        self.entries[vector as usize] = IdtEntry::new(handler, selector, options);
    }

    /// Set an interrupt gate handler
    #[inline]
    pub fn set_interrupt_handler(&mut self, vector: u8, handler: u64, selector: u16) {
        self.entries[vector as usize] = IdtEntry::interrupt(handler, selector);
    }

    /// Set a trap gate handler
    #[inline]
    pub fn set_trap_handler(&mut self, vector: u8, handler: u64, selector: u16) {
        self.entries[vector as usize] = IdtEntry::trap(handler, selector);
    }

    /// Set an interrupt gate handler with IST
    #[inline]
    pub fn set_interrupt_handler_with_ist(
        &mut self,
        vector: u8,
        handler: u64,
        selector: u16,
        ist: u8,
    ) {
        self.entries[vector as usize] = IdtEntry::interrupt_with_ist(handler, selector, ist);
    }

    /// Create the IDT descriptor for LIDT
    #[inline]
    pub fn descriptor(&self) -> IdtDescriptor {
        IdtDescriptor::new(
            self.entries.as_ptr() as u64,
            (IDT_SIZE * size_of::<IdtEntry>()) as u16,
        )
    }

    /// Load this IDT into the CPU
    ///
    /// # Safety
    ///
    /// The IDT must remain valid for as long as it's loaded.
    #[inline]
    pub unsafe fn load(&self) {
        let desc = self.descriptor();
        unsafe {
            core::arch::asm!(
                "lidt [{}]",
                in(reg) &desc,
                options(nostack, preserves_flags),
            );
        }
    }
}

impl Default for Idt {
    fn default() -> Self {
        Self::new()
    }
}

impl core::ops::Index<u8> for Idt {
    type Output = IdtEntry;

    fn index(&self, index: u8) -> &Self::Output {
        &self.entries[index as usize]
    }
}

impl core::ops::IndexMut<u8> for Idt {
    fn index_mut(&mut self, index: u8) -> &mut Self::Output {
        &mut self.entries[index as usize]
    }
}

// =============================================================================
// Static IDT
// =============================================================================

use core::cell::UnsafeCell;

/// Wrapper for CPU-local static data that requires interior mutability.
///
/// # Safety
///
/// This type is only safe to use for data that is:
/// 1. Initialized once during boot before other CPUs start
/// 2. Only accessed by a single CPU at a time (per-CPU data)
/// 3. Or protected by external synchronization
struct CpuStatic<T>(UnsafeCell<T>);

impl<T> CpuStatic<T> {
    /// Create a new CpuStatic with the given value.
    const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    /// Get a reference to the inner value.
    ///
    /// # Safety
    ///
    /// Caller must ensure no mutable references exist.
    unsafe fn get(&self) -> &T {
        unsafe { &*self.0.get() }
    }

    /// Get a mutable reference to the inner value.
    ///
    /// # Safety
    ///
    /// Caller must ensure exclusive access.
    #[allow(clippy::mut_from_ref)]
    unsafe fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.0.get() }
    }
}

// SAFETY: IDT is initialized during boot and accessed per-CPU.
// Each CPU loads its own view, and modifications are synchronized
// by the boot sequence (single-threaded until APs are started).
unsafe impl<T> Sync for CpuStatic<T> {}

/// The global IDT instance
///
/// This is shared across all CPUs.
static IDT: CpuStatic<Idt> = CpuStatic::new(Idt::new());

/// Get a reference to the global IDT
///
/// # Safety
///
/// The caller must ensure exclusive access if modifying.
pub unsafe fn get_idt() -> &'static Idt {
    unsafe { IDT.get() }
}

/// Get a mutable reference to the global IDT
///
/// # Safety
///
/// The caller must ensure exclusive access.
pub unsafe fn get_idt_mut() -> &'static mut Idt {
    unsafe { IDT.get_mut() }
}

// =============================================================================
// IDT Initialization
// =============================================================================

/// Initialize the IDT with default handlers
///
/// # Safety
///
/// This must only be called once during early boot.
pub unsafe fn init_idt() {
    let idt = unsafe { get_idt_mut() };
    let kernel_cs = segmentation::selectors::KERNEL_CS.raw();

    // Set up exception handlers (0x00-0x1F)
    setup_exception_handlers(idt, kernel_cs);

    // Set up default handlers for all other vectors
    for vector in 0x20u8..=0xFF {
        idt.set_interrupt_handler(
            vector,
            handlers::default_interrupt_handler as u64,
            kernel_cs,
        );
    }

    // Set up system call handler (if using INT 0x80)
    idt.set(
        0x80,
        IdtEntry::user_callable(
            handlers::default_interrupt_handler as u64,
            kernel_cs,
            GateType::Trap,
        ),
    );

    // Set up spurious interrupt handler
    idt.set_interrupt_handler(0xFF, handlers::spurious_interrupt_handler as u64, kernel_cs);

    log::debug!("IDT: Exception handlers configured");
}

/// Set up all exception handlers
fn setup_exception_handlers(idt: &mut Idt, selector: u16) {
    // #DE - Divide Error
    idt.set_interrupt_handler(
        ExceptionVector::DivideError as u8,
        handlers::divide_error_handler as u64,
        selector,
    );

    // #DB - Debug (use IST for safety during single-stepping)
    idt.set_interrupt_handler_with_ist(
        ExceptionVector::Debug as u8,
        handlers::debug_handler as u64,
        selector,
        ExceptionVector::Debug.recommended_ist(),
    );

    // NMI (use IST)
    idt.set_interrupt_handler_with_ist(
        ExceptionVector::NonMaskableInterrupt as u8,
        handlers::nmi_handler as u64,
        selector,
        ExceptionVector::NonMaskableInterrupt.recommended_ist(),
    );

    // #BP - Breakpoint (trap gate for debugging)
    idt.set(
        ExceptionVector::Breakpoint as u8,
        IdtEntry::user_callable(
            handlers::breakpoint_handler as u64,
            selector,
            GateType::Trap,
        ),
    );

    // #OF - Overflow
    idt.set_interrupt_handler(
        ExceptionVector::Overflow as u8,
        handlers::overflow_handler as u64,
        selector,
    );

    // #BR - Bound Range
    idt.set_interrupt_handler(
        ExceptionVector::BoundRangeExceeded as u8,
        handlers::bound_range_handler as u64,
        selector,
    );

    // #UD - Invalid Opcode
    idt.set_interrupt_handler(
        ExceptionVector::InvalidOpcode as u8,
        handlers::invalid_opcode_handler as u64,
        selector,
    );

    // #NM - Device Not Available
    idt.set_interrupt_handler(
        ExceptionVector::DeviceNotAvailable as u8,
        handlers::device_not_available_handler as u64,
        selector,
    );

    // #DF - Double Fault (MUST use IST!)
    idt.set_interrupt_handler_with_ist(
        ExceptionVector::DoubleFault as u8,
        handlers::double_fault_handler as u64,
        selector,
        ExceptionVector::DoubleFault.recommended_ist(),
    );

    // #TS - Invalid TSS
    idt.set_interrupt_handler(
        ExceptionVector::InvalidTss as u8,
        handlers::invalid_tss_handler as u64,
        selector,
    );

    // #NP - Segment Not Present
    idt.set_interrupt_handler(
        ExceptionVector::SegmentNotPresent as u8,
        handlers::segment_not_present_handler as u64,
        selector,
    );

    // #SS - Stack Segment Fault
    idt.set_interrupt_handler(
        ExceptionVector::StackSegmentFault as u8,
        handlers::stack_segment_handler as u64,
        selector,
    );

    // #GP - General Protection Fault
    idt.set_interrupt_handler(
        ExceptionVector::GeneralProtection as u8,
        handlers::general_protection_handler as u64,
        selector,
    );

    // #PF - Page Fault
    idt.set_interrupt_handler(
        ExceptionVector::PageFault as u8,
        handlers::page_fault_handler as u64,
        selector,
    );

    // #MF - x87 FPU Error
    idt.set_interrupt_handler(
        ExceptionVector::X87FloatingPoint as u8,
        handlers::x87_fpu_handler as u64,
        selector,
    );

    // #AC - Alignment Check
    idt.set_interrupt_handler(
        ExceptionVector::AlignmentCheck as u8,
        handlers::alignment_check_handler as u64,
        selector,
    );

    // #MC - Machine Check (use IST!)
    idt.set_interrupt_handler_with_ist(
        ExceptionVector::MachineCheck as u8,
        handlers::machine_check_handler as u64,
        selector,
        ExceptionVector::MachineCheck.recommended_ist(),
    );

    // #XM - SIMD Floating-Point
    idt.set_interrupt_handler(
        ExceptionVector::SimdFloatingPoint as u8,
        handlers::simd_floating_point_handler as u64,
        selector,
    );

    // #VE - Virtualization Exception
    idt.set_interrupt_handler(
        ExceptionVector::VirtualizationException as u8,
        handlers::virtualization_handler as u64,
        selector,
    );

    // #CP - Control Protection
    idt.set_interrupt_handler(
        ExceptionVector::ControlProtection as u8,
        handlers::control_protection_handler as u64,
        selector,
    );

    // #SX - Security Exception
    idt.set_interrupt_handler(
        ExceptionVector::SecurityException as u8,
        handlers::security_handler as u64,
        selector,
    );
}

/// Load the global IDT into the CPU
///
/// # Safety
///
/// The IDT must be properly initialized.
pub unsafe fn load_idt() {
    let idt = unsafe { get_idt() };
    unsafe {
        idt.load();
    }
}

/// Set a handler for a specific vector
///
/// # Safety
///
/// The handler must be a valid interrupt handler function.
pub unsafe fn set_handler(vector: u8, handler: usize, gate_type: GateType) {
    let idt = unsafe { get_idt_mut() };
    let selector = segmentation::selectors::KERNEL_CS.raw();

    idt.set(
        vector,
        IdtEntry::new(handler as u64, selector, GateOptions::from_type(gate_type)),
    );
}

/// Set an exception handler with IST
///
/// # Safety
///
/// The handler must be a valid exception handler function.
pub unsafe fn set_exception_handler(vector: u8, handler: usize, ist: u8) {
    let idt = unsafe { get_idt_mut() };
    let selector = segmentation::selectors::KERNEL_CS.raw();

    idt.set(
        vector,
        IdtEntry::new(
            handler as u64,
            selector,
            GateOptions::new_interrupt().with_ist(ist),
        ),
    );
}

// =============================================================================
// Compile-time Assertions
// =============================================================================

const _: () = {
    // Verify sizes
    assert!(size_of::<IdtDescriptor>() == 10);
    assert!(size_of::<Idt>() == IDT_TOTAL_SIZE);
    assert!(IDT_TOTAL_SIZE == 4096);
};
