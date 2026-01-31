//! # AArch64 Architecture HAL Implementation
//!
//! This module provides the hardware abstraction layer for AArch64 (ARM64) CPUs.
//! It implements Exception Levels, MMU, GIC, SMP, and timer support for a complete
//! industrial-grade ARM64 kernel foundation.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                      Helix OS - AArch64 HAL                              │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐       │
//! │  │    Core     │ │  Exception  │ │     MMU     │ │     GIC     │       │
//! │  │  Framework  │ │  Levels     │ │  Framework  │ │  Framework  │       │
//! │  │             │ │             │ │             │ │             │       │
//! │  │• Registers  │ │• EL0-EL3    │ │• 4KB/64KB   │ │• GICv2/v3   │       │
//! │  │• Sys Regs   │ │• Vectors    │ │• TTBR0/1    │ │• Distrib.   │       │
//! │  │• Features   │ │• Handlers   │ │• ASID       │ │• Redistrib. │       │
//! │  │• Cache      │ │• Syscalls   │ │• TLB        │ │• Routing    │       │
//! │  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘       │
//! │                                                                          │
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐                       │
//! │  │     SMP     │ │    PSCI     │ │   Timers    │                       │
//! │  │  Framework  │ │   Power     │ │  Framework  │                       │
//! │  │             │ │             │ │             │                       │
//! │  │• CPU enum   │ │• CPU_ON/OFF │ │• Generic    │                       │
//! │  │• Startup    │ │• SYSTEM_*   │ │• Physical   │                       │
//! │  │• Per-CPU    │ │• SMC/HVC    │ │• Virtual    │                       │
//! │  │• Barriers   │ │             │ │• Watchdog   │                       │
//! │  └─────────────┘ └─────────────┘ └─────────────┘                       │
//! │                                                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Module Organization
//!
//! ### Core Framework
//! - [`core`]: Fundamental CPU primitives
//!   - [`core::registers`]: General purpose registers (X0-X30, SP, PC, PSTATE)
//!   - [`core::system_regs`]: System registers (SCTLR, TCR, MAIR, etc.)
//!   - [`core::features`]: CPU feature detection (ID_AA64*)
//!   - [`core::cache`]: Cache maintenance operations
//!   - [`core::barriers`]: Memory barriers (DMB, DSB, ISB)
//!   - [`core::fpu`]: NEON/SVE state management
//!
//! ### Exception Level Framework
//! - [`exception`]: Exception handling for EL0-EL3
//!   - [`exception::el`]: Exception level utilities
//!   - [`exception::vectors`]: Vector table definition
//!   - [`exception::handlers`]: Exception handlers
//!   - [`exception::sync`]: Synchronous exception handling
//!   - [`exception::irq`]: IRQ handling
//!   - [`exception::syscall`]: System call handling
//!   - [`exception::frame`]: Exception frame structure
//!
//! ### MMU Framework
//! - [`mmu`]: Memory Management Unit
//!   - [`mmu::translation_table`]: Translation table management
//!   - [`mmu::entries`]: Page/block descriptor formats
//!   - [`mmu::granule`]: 4KB/16KB/64KB granule support
//!   - [`mmu::asid`]: ASID management
//!   - [`mmu::tlb`]: TLB maintenance
//!   - [`mmu::tcr`]: Translation Control Register
//!   - [`mmu::mair`]: Memory Attribute Indirection
//!
//! ### GIC Framework
//! - [`gic`]: Generic Interrupt Controller
//!   - [`gic::distributor`]: GICD_* registers
//!   - [`gic::redistributor`]: GICR_* registers (GICv3)
//!   - [`gic::cpu_interface`]: GICC_* or ICC_* registers
//!   - [`gic::gicv2`]: GICv2 implementation
//!   - [`gic::gicv3`]: GICv3 implementation
//!   - [`gic::routing`]: Interrupt routing
//!
//! ### SMP Framework
//! - [`smp`]: Symmetric Multi-Processing
//!   - [`smp::cpu_info`]: CPU enumeration and topology
//!   - [`smp::mpidr`]: MPIDR handling
//!   - [`smp::startup`]: Secondary CPU startup
//!   - [`smp::per_cpu`]: Per-CPU data (TPIDR_EL1)
//!   - [`smp::barriers`]: SMP synchronization
//!
//! ### PSCI Framework
//! - [`psci`]: Power State Coordination Interface
//!   - [`psci::conduit`]: SMC/HVC selection
//!   - [`psci::functions`]: PSCI function calls
//!   - [`psci::cpu_ops`]: CPU power operations
//!
//! ### Timer Framework
//! - [`timers`]: ARM Timer support
//!   - [`timers::generic_timer`]: ARM Generic Timer
//!   - [`timers::system_counter`]: System counter access
//!   - [`timers::physical_timer`]: Physical timer (CNTP_*)
//!   - [`timers::virtual_timer`]: Virtual timer (CNTV_*)

#![allow(dead_code)]

// =============================================================================
// CORE FRAMEWORK
// =============================================================================

pub mod core;

// =============================================================================
// EXCEPTION LEVEL FRAMEWORK
// =============================================================================

pub mod exception;

// =============================================================================
// MEMORY MANAGEMENT UNIT
// =============================================================================

pub mod mmu;

// =============================================================================
// GENERIC INTERRUPT CONTROLLER
// =============================================================================

pub mod gic;

// =============================================================================
// SYMMETRIC MULTI-PROCESSING
// =============================================================================

pub mod smp;

// =============================================================================
// TIMER FRAMEWORK
// =============================================================================

pub mod timers;

// NOTE: psci is part of the smp module

// =============================================================================
// CONSTANTS
// =============================================================================

/// Maximum number of supported CPUs
pub const MAX_CPUS: usize = 256;

/// Page size (4KB default)
pub const PAGE_SIZE: usize = 4096;

/// Page shift (log2 of page size)
pub const PAGE_SHIFT: usize = 12;

/// Large page size (2MB)
pub const LARGE_PAGE_SIZE: usize = 2 * 1024 * 1024;

/// Huge page size (1GB)
pub const HUGE_PAGE_SIZE: usize = 1024 * 1024 * 1024;

/// Kernel virtual address base (higher half)
pub const KERNEL_VADDR_BASE: u64 = 0xFFFF_0000_0000_0000;

/// User virtual address limit
pub const USER_VADDR_LIMIT: u64 = 0x0000_FFFF_FFFF_FFFF;

/// Physical address mask (48-bit)
pub const PHYS_ADDR_MASK: u64 = 0x0000_FFFF_FFFF_F000;

// =============================================================================
// RE-EXPORTS
// =============================================================================

// Core framework re-exports
pub use core::{
    registers::GeneralRegisters,
    system_regs::{read_sctlr_el1, write_sctlr_el1, read_tcr_el1, write_tcr_el1},
    features::CpuFeatures,
    cache::{dcache_clean_range, dcache_invalidate_range, dcache_clean_invalidate_range},
    barriers::{dmb, dsb, isb},
    fpu::FpuState,
};

// Exception framework re-exports
pub use exception::{
    el::{current_el, CurrentEl},
    context::ExceptionContext,
    handlers::ExceptionHandler,
};

// MMU re-exports
pub use mmu::{
    entries::{PageTableEntry, PageFlags, MemoryAttributes},
    tables::PageTable,
    tlb::{tlb_invalidate_all, tlb_invalidate_asid, tlb_invalidate_va},
    asid::AsidManager,
};

// GIC re-exports
pub use gic::{
    GicVersion, InterruptType, Gic,
    distributor::Distributor,
    cpu_interface::CpuInterface,
};

// SMP re-exports
pub use smp::{
    CpuState, CpuInfo, CpuTopology,
    mpidr::Mpidr,
    psci::Psci,
    percpu::PerCpuData,
    ipi::{IpiVector, send_reschedule_ipi},
};

// Timer re-exports
pub use timers::{
    Timer, TimerOperations,
    physical::PhysicalTimer,
    virtual_timer::VirtualTimer,
};

// =============================================================================
// HAL IMPLEMENTATION
// =============================================================================

/// AArch64 HAL Implementation
///
/// Main entry point for the AArch64 hardware abstraction layer.
pub struct AArch64Hal {
    /// Initialization state
    initialized: bool,
    /// Timer subsystem
    timer_subsystem: Option<timers::TimerSubsystem>,
}

impl AArch64Hal {
    /// Create a new AArch64 HAL instance
    pub const fn new() -> Self {
        Self {
            initialized: false,
            timer_subsystem: None,
        }
    }

    /// Initialize the HAL
    ///
    /// # Safety
    /// Must be called once during boot on the primary CPU.
    pub unsafe fn init(&mut self) -> Result<(), &'static str> {
        if self.initialized {
            return Err("HAL already initialized");
        }

        // Detect CPU features
        let features = core::features::CpuFeatures::detect();

        // Log detected features (debug)
        let _ = features;

        // Initialize timer subsystem
        self.timer_subsystem = Some(timers::TimerSubsystem::init());

        self.initialized = true;
        Ok(())
    }

    /// Initialize GIC with auto-detection
    ///
    /// # Safety
    /// Must be called after basic system initialization.
    pub unsafe fn init_gic(&self, gicd_base: usize, gicr_or_gicc_base: usize) -> Result<gic::Gic, &'static str> {
        let version = gic::GicVersion::detect(gicd_base as *const u8);
        let gic = gic::Gic::new(version, gicd_base as *mut u8, gicr_or_gicc_base as *mut u8);
        gic.init();
        Ok(gic)
    }

    /// Initialize SMP on the BSP
    ///
    /// # Safety
    /// Must be called on BSP only.
    pub unsafe fn init_smp_bsp(&self) -> Result<(), &'static str> {
        smp::init_bsp();
        Ok(())
    }

    /// Get timer subsystem
    pub fn timer_subsystem(&self) -> Option<&timers::TimerSubsystem> {
        self.timer_subsystem.as_ref()
    }

    /// Check if HAL is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get current exception level
    pub fn current_el(&self) -> u8 {
        exception::el::current_el()
    }

    /// Get current CPU ID
    pub fn current_cpu_id(&self) -> u32 {
        smp::mpidr::Mpidr::current().linear_id()
    }
}

impl Default for AArch64Hal {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// PLATFORM INITIALIZATION HELPERS
// =============================================================================

/// Initialize for QEMU virt platform
///
/// # Safety
/// Platform-specific initialization. Must only be called on QEMU virt.
pub unsafe fn init_qemu_virt() -> AArch64Hal {
    let mut hal = AArch64Hal::new();
    hal.init().expect("HAL init failed");

    // QEMU virt GIC addresses (GICv3)
    // GICD: 0x0800_0000
    // GICR: 0x080A_0000
    let _gic = hal.init_gic(0x0800_0000, 0x080A_0000).expect("GIC init failed");

    hal.init_smp_bsp().expect("SMP init failed");

    hal
}

/// Initialize for Raspberry Pi 4
///
/// # Safety
/// Platform-specific initialization. Must only be called on RPi 4.
pub unsafe fn init_rpi4() -> AArch64Hal {
    let mut hal = AArch64Hal::new();
    hal.init().expect("HAL init failed");

    // RPi 4 GIC addresses (GICv2)
    // GICD: 0xFF84_1000
    // GICC: 0xFF84_2000
    let _gic = hal.init_gic(0xFF84_1000, 0xFF84_2000).expect("GIC init failed");

    hal.init_smp_bsp().expect("SMP init failed");

    hal
}
