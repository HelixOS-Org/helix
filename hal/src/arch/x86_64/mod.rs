//! # x86_64 Architecture HAL Implementation
//!
//! This module provides the hardware abstraction layer for x86_64 CPUs.
//! It implements GDT, IDT, CPU control, interrupt management, task switching,
//! and syscall support.
//!
//! ## Module Organization
//!
//! ### Core Framework (NEW - Industrial Grade)
//! - [`core`]: Fundamental CPU primitives
//!   - [`core::cpuid`]: Complete CPUID enumeration
//!   - [`core::msr`]: Model-Specific Registers
//!   - [`core::control_regs`]: CR0-CR4, XCR0
//!   - [`core::features`]: CPU capability detection
//!   - [`core::cache`]: Cache control and prefetch
//!   - [`core::fpu`]: FPU/SSE/AVX state management
//!
//! ### Segmentation Framework (NEW - Industrial Grade)
//! - [`segmentation`]: GDT/TSS for SMP systems
//!   - [`segmentation::selectors`]: Type-safe segment selectors
//!   - [`segmentation::tss`]: 64-bit TSS with IST support
//!   - [`segmentation::gdt`]: GDT management with system descriptors
//!   - [`segmentation::per_cpu`]: Per-CPU GDT/TSS for SMP
//!
//! ### Interrupt Framework (NEW - Industrial Grade)
//! - [`interrupts`]: IDT and interrupt handling for SMP
//!   - [`interrupts::idt`]: IDT management
//!   - [`interrupts::entries`]: Gate descriptors
//!   - [`interrupts::vectors`]: Vector allocation
//!   - [`interrupts::handlers`]: Exception and interrupt handlers
//!   - [`interrupts::frame`]: Interrupt stack frames
//!
//! ### Paging Framework (NEW - Industrial Grade)
//! - [`paging_v2`]: 4/5-level paging with PCID
//!   - [`paging_v2::addresses`]: Physical/Virtual address types
//!   - [`paging_v2::entries`]: Page table entries and flags
//!   - [`paging_v2::table`]: Page table structure
//!   - [`paging_v2::tlb`]: TLB management and PCID
//!   - [`paging_v2::walker`]: Page table walking
//!
//! ### APIC Framework (NEW - Industrial Grade)
//! - [`apic`]: Advanced Programmable Interrupt Controller
//!   - [`apic::local`]: Local APIC (xAPIC mode)
//!   - [`apic::ioapic`]: I/O APIC for external interrupts
//!   - [`apic::x2apic`]: x2APIC MSR-based mode
//!   - [`apic::ipi`]: Inter-Processor Interrupts
//!   - [`apic::msi`]: Message Signaled Interrupts
//!
//! ### Timer Framework (NEW - Industrial Grade)
//! - [`timers`]: Comprehensive timer support
//!   - [`timers::tsc`]: Time Stamp Counter
//!   - [`timers::hpet`]: High Precision Event Timer
//!   - [`timers::apic_timer`]: Per-CPU APIC Timer
//!   - [`timers::pit`]: Legacy PIT (for calibration)
//!   - [`timers::calibration`]: Timer calibration routines
//!
//! ### SMP Framework (NEW - Industrial Grade)
//! - [`smp`]: Symmetric Multi-Processing support
//!   - [`smp::startup`]: AP startup via INIT-SIPI-SIPI
//!   - [`smp::cpu_info`]: CPU topology and enumeration
//!   - [`smp::per_cpu`]: Per-CPU data with GS base
//!   - [`smp::barriers`]: Synchronization primitives
//!
//! ### Legacy Modules (Being Refactored)
//! - [`gdt`]: Global Descriptor Table (DEPRECATED - use segmentation)
//! - [`idt`]: Interrupt Descriptor Table (DEPRECATED - use interrupts)
//! - [`cpu`]: CPU control
//! - [`exceptions`]: Exception handlers (DEPRECATED - use interrupts)
//! - [`paging`]: Page table management (DEPRECATED - use paging_v2)

// =============================================================================
// NEW INDUSTRIAL-GRADE CORE FRAMEWORK
// =============================================================================

pub mod core;
pub mod segmentation;
pub mod interrupts;
pub mod paging_v2;
pub mod apic;
pub mod timers;
pub mod smp;

// =============================================================================
// EXISTING MODULES (Legacy - To Be Refactored)
// =============================================================================

pub mod gdt;
pub mod idt;
pub mod cpu;
pub mod exceptions;
pub mod pic;
pub mod pit;
pub mod task;
pub mod context;
pub mod irq;
pub mod syscall;
pub mod userspace;
pub mod paging;

use crate::HalResult;

/// x86_64 HAL Implementation
pub struct X86_64Hal {
    cpu: cpu::X86_64Cpu,
    // mmu: X86_64Mmu,
    // interrupts: X86_64InterruptController,
    // firmware: X86_64Firmware,
}

impl X86_64Hal {
    /// Create and initialize the x86_64 HAL
    ///
    /// # Safety
    /// This should only be called once during boot.
    pub unsafe fn init() -> HalResult<Self> {
        // Initialize GDT
        unsafe { gdt::init(); }

        // Initialize IDT
        unsafe { idt::init(); }

        Ok(Self {
            cpu: cpu::X86_64Cpu::new(),
        })
    }
}

/// Initialize the x86_64 HAL (full initialization)
///
/// This is a convenience function for early boot.
///
/// # Safety
/// Must be called only once, during early boot, before interrupts are enabled.
pub unsafe fn init() {
    // Core CPU setup
    unsafe {
        gdt::init();
        idt::init();
    }

    log::info!("x86_64 HAL: GDT and IDT initialized");

    // Initialize interrupt controllers
    unsafe {
        pic::init();
        pit::init_default();
    }

    log::info!("x86_64 HAL: PIC and PIT initialized");

    // Set up timer interrupt handler
    unsafe {
        idt::set_handler(
            idt::vectors::TIMER,
            irq::timer_handler as u64,
            idt::IdtEntryOptions::interrupt(),
        );
        idt::set_handler(
            idt::vectors::KEYBOARD,
            irq::keyboard_handler as u64,
            idt::IdtEntryOptions::interrupt(),
        );
        idt::reload();
    }

    // Enable timer IRQ
    pic::enable_irq(pic::Irq::Timer);

    // Initialize syscall support
    unsafe {
        syscall::init();
    }

    log::info!("x86_64 HAL fully initialized");
}
