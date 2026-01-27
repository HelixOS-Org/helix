//! # x86_64 Architecture HAL Implementation
//!
//! This module provides the hardware abstraction layer for x86_64 CPUs.
//! It implements GDT, IDT, CPU control, interrupt management, task switching,
//! and syscall support.

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

use crate::{HardwareAbstractionLayer, HalResult, HalError};
use crate::cpu::CpuAbstraction;
use crate::mmu::MmuAbstraction;
use crate::interrupts::InterruptController;
use crate::firmware::FirmwareInterface;

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
