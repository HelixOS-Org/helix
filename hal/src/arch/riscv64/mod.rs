//! # RISC-V 64-bit Architecture HAL Implementation
//!
//! This module provides the hardware abstraction layer for RISC-V 64-bit (RV64GC) CPUs.
//! It implements privilege levels, MMU, CLINT/PLIC, SMP, and timer support for a complete
//! industrial-grade RISC-V kernel foundation.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                      Helix OS - RISC-V 64 HAL                            │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐       │
//! │  │    Core     │ │ Privilege   │ │     MMU     │ │ Interrupts  │       │
//! │  │  Framework  │ │   Levels    │ │  Framework  │ │CLINT + PLIC │       │
//! │  │             │ │             │ │             │ │             │       │
//! │  │• Registers  │ │• M/S/U mode │ │• Sv39/48/57 │ │• Timer      │       │
//! │  │• CSRs       │ │• Traps      │ │• SATP       │ │• Software   │       │
//! │  │• Features   │ │• Handlers   │ │• ASID       │ │• External   │       │
//! │  │• Barriers   │ │• Syscalls   │ │• TLB        │ │• IRQ        │       │
//! │  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘       │
//! │                                                                          │
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐                       │
//! │  │     SMP     │ │    SBI      │ │   Timers    │                       │
//! │  │  Framework  │ │  Interface  │ │  Framework  │                       │
//! │  │             │ │             │ │             │                       │
//! │  │• Hart ID    │ │• Base       │ │• MTIME      │                       │
//! │  │• Startup    │ │• Timer      │ │• MTIMECMP   │                       │
//! │  │• Per-Hart   │ │• HSM        │ │• Supervisor │                       │
//! │  │• IPI        │ │• IPI        │ │             │                       │
//! │  └─────────────┘ └─────────────┘ └─────────────┘                       │
//! │                                                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Module Organization
//!
//! ### Core Framework
//! - [`core`]: Fundamental CPU primitives
//!   - [`core::registers`]: General purpose registers (x0-x31)
//!   - [`core::csr`]: Control and Status Registers
//!   - [`core::features`]: ISA extension detection
//!   - [`core::cache`]: Cache operations (FENCE.I)
//!   - [`core::barriers`]: Memory barriers (FENCE)
//!
//! ### Privilege Level Framework
//! - [`privilege`]: M/S/U mode handling
//!   - [`privilege::modes`]: Privilege mode definitions
//!   - [`privilege::traps`]: Trap handling
//!   - [`privilege::vectors`]: Trap vector table
//!   - [`privilege::syscall`]: ECALL handling
//!
//! ### MMU Framework
//! - [`mmu`]: Memory Management Unit
//!   - [`mmu::entries`]: Page Table Entry format
//!   - [`mmu::tables`]: Page table management
//!   - [`mmu::tlb`]: TLB operations (SFENCE.VMA)
//!   - [`mmu::asid`]: ASID management
//!   - [`mmu::satp`]: SATP register control
//!
//! ### Interrupt Framework
//! - [`interrupts`]: Interrupt handling
//!   - [`interrupts::clint`]: Core Local Interruptor
//!   - [`interrupts::plic`]: Platform-Level Interrupt Controller
//!   - [`interrupts::irq`]: IRQ management
//!
//! ### SMP Framework
//! - [`smp`]: Symmetric Multi-Processing
//!   - [`smp::hartid`]: Hart ID handling
//!   - [`smp::percpu`]: Per-hart data
//!   - [`smp::startup`]: Secondary hart startup
//!   - [`smp::ipi`]: Inter-Processor Interrupts
//!
//! ### Timer Framework
//! - [`timers`]: Timer support
//!   - [`timers::mtime`]: Machine timer
//!   - [`timers::sstimer`]: Supervisor timer
//!
//! ### SBI Framework
//! - [`sbi`]: Supervisor Binary Interface
//!   - [`sbi::base`]: Base extension
//!   - [`sbi::timer`]: Timer extension
//!   - [`sbi::hsm`]: Hart State Management

#![allow(dead_code)]

// =============================================================================
// CORE FRAMEWORK
// =============================================================================

pub mod core;

// =============================================================================
// PRIVILEGE LEVEL FRAMEWORK
// =============================================================================

pub mod privilege;

// =============================================================================
// MEMORY MANAGEMENT UNIT
// =============================================================================

pub mod mmu;

// =============================================================================
// INTERRUPT FRAMEWORK
// =============================================================================

pub mod interrupts;

// =============================================================================
// SYMMETRIC MULTI-PROCESSING
// =============================================================================

pub mod smp;

// =============================================================================
// TIMER FRAMEWORK
// =============================================================================

pub mod timers;

// =============================================================================
// SBI INTERFACE
// =============================================================================

pub mod sbi;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Maximum number of supported harts
pub const MAX_HARTS: usize = 256;

/// Page size (4KB)
pub const PAGE_SIZE: usize = 4096;

/// Page shift (log2 of page size)
pub const PAGE_SHIFT: usize = 12;

/// Mega page size (2MB for Sv39/48)
pub const MEGA_PAGE_SIZE: usize = 2 * 1024 * 1024;

/// Giga page size (1GB for Sv39/48)
pub const GIGA_PAGE_SIZE: usize = 1024 * 1024 * 1024;

/// Tera page size (512GB for Sv48/57)
pub const TERA_PAGE_SIZE: usize = 512 * 1024 * 1024 * 1024;

/// Kernel virtual address base (higher half for Sv39)
/// Using canonical higher half: 0xFFFF_FFC0_0000_0000
pub const KERNEL_VADDR_BASE: u64 = 0xFFFF_FFC0_0000_0000;

/// User virtual address limit (Sv39)
/// Max user address: 0x0000_003F_FFFF_FFFF (256 GB)
pub const USER_VADDR_LIMIT_SV39: u64 = 0x0000_003F_FFFF_FFFF;

/// User virtual address limit (Sv48)
/// Max user address: 0x0000_7FFF_FFFF_FFFF (128 TB)
pub const USER_VADDR_LIMIT_SV48: u64 = 0x0000_7FFF_FFFF_FFFF;

// =============================================================================
// PLATFORM DEFAULTS (QEMU virt)
// =============================================================================

/// Default CLINT base address (QEMU virt)
pub const CLINT_BASE_QEMU: usize = 0x0200_0000;

/// Default PLIC base address (QEMU virt)
pub const PLIC_BASE_QEMU: usize = 0x0C00_0000;

/// Default UART base address (QEMU virt)
pub const UART_BASE_QEMU: usize = 0x1000_0000;

/// Default memory base (QEMU virt)
pub const DRAM_BASE_QEMU: usize = 0x8000_0000;

// =============================================================================
// RE-EXPORTS
// =============================================================================

// Core framework re-exports
pub use core::{
    registers::GeneralRegisters,
    csr::{read_sstatus, write_sstatus, read_satp, write_satp},
    csr::{read_scause, read_stval, read_sepc},
    features::RiscvFeatures,
    barriers::{fence, fence_i, sfence_vma, sfence_vma_asid},
};

// Privilege framework re-exports
pub use privilege::{
    modes::PrivilegeMode,
    traps::{TrapCause, TrapFrame},
    vectors::set_stvec,
};

// MMU re-exports
pub use mmu::{
    entries::{PageTableEntry, PageFlags},
    tables::PageTable,
    tlb::{tlb_flush_all, tlb_flush_page, tlb_flush_asid},
    asid::AsidManager,
    satp::{SatpMode, Satp},
};

// Interrupt re-exports
pub use interrupts::{
    clint::Clint,
    plic::Plic,
    irq::{IrqHandler, enable_interrupts, disable_interrupts},
};

// SMP re-exports
pub use smp::{
    hartid::{HartId, current_hart_id},
    percpu::PerHartData,
    ipi::send_ipi,
};

// Timer re-exports
pub use timers::{
    mtime::{read_mtime, Timer},
    sstimer::SupervisorTimer,
};

// SBI re-exports
pub use sbi::{
    SbiRet, SbiError,
    base::{sbi_get_spec_version, sbi_probe_extension},
    timer::sbi_set_timer,
    hsm::{sbi_hart_start, sbi_hart_stop, sbi_hart_get_status},
};

// =============================================================================
// HAL IMPLEMENTATION
// =============================================================================

/// RISC-V 64 HAL Implementation
///
/// Main entry point for the RISC-V 64-bit hardware abstraction layer.
pub struct Riscv64Hal {
    /// Initialization state
    initialized: bool,
    /// Detected features
    features: RiscvFeatures,
    /// Boot hart ID
    boot_hart_id: usize,
}

impl Riscv64Hal {
    /// Create a new RISC-V 64 HAL instance
    pub const fn new() -> Self {
        Self {
            initialized: false,
            features: RiscvFeatures::empty(),
            boot_hart_id: 0,
        }
    }

    /// Initialize the HAL
    ///
    /// # Arguments
    /// * `hart_id` - The hart ID of the boot hart
    /// * `dtb_addr` - Device tree blob address (from bootloader)
    ///
    /// # Safety
    /// Must be called once during boot on the boot hart.
    pub unsafe fn init(&mut self, hart_id: usize, _dtb_addr: usize) -> Result<(), &'static str> {
        if self.initialized {
            return Err("HAL already initialized");
        }

        self.boot_hart_id = hart_id;

        // Detect CPU features
        self.features = RiscvFeatures::detect();

        // Initialize per-hart data for boot hart
        smp::percpu::init_boot_hart(hart_id);

        // Set up trap vector
        privilege::vectors::init_stvec();

        self.initialized = true;
        Ok(())
    }

    /// Initialize CLINT
    ///
    /// # Safety
    /// Platform-specific initialization.
    pub unsafe fn init_clint(&self, base: usize) -> Clint {
        Clint::new(base as *mut u8)
    }

    /// Initialize PLIC
    ///
    /// # Safety
    /// Platform-specific initialization.
    pub unsafe fn init_plic(&self, base: usize) -> Plic {
        let plic = Plic::new(base as *mut u8);
        plic.init();
        plic
    }

    /// Get detected features
    pub fn features(&self) -> &RiscvFeatures {
        &self.features
    }

    /// Get boot hart ID
    pub fn boot_hart_id(&self) -> usize {
        self.boot_hart_id
    }

    /// Check if HAL is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get current privilege mode
    pub fn current_privilege(&self) -> PrivilegeMode {
        // In S-mode kernel, we're always in S-mode when this is called
        PrivilegeMode::Supervisor
    }

    /// Get current hart ID
    pub fn current_hart_id(&self) -> usize {
        current_hart_id()
    }
}

impl Default for Riscv64Hal {
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
pub unsafe fn init_qemu_virt(hart_id: usize, dtb_addr: usize) -> Riscv64Hal {
    let mut hal = Riscv64Hal::new();
    hal.init(hart_id, dtb_addr).expect("HAL init failed");

    // Initialize CLINT
    let _clint = hal.init_clint(CLINT_BASE_QEMU);

    // Initialize PLIC
    let _plic = hal.init_plic(PLIC_BASE_QEMU);

    hal
}

/// Initialize for SiFive platforms
///
/// # Safety
/// Platform-specific initialization.
pub unsafe fn init_sifive(hart_id: usize, dtb_addr: usize) -> Riscv64Hal {
    let mut hal = Riscv64Hal::new();
    hal.init(hart_id, dtb_addr).expect("HAL init failed");

    // SiFive uses standard CLINT/PLIC addresses
    let _clint = hal.init_clint(CLINT_BASE_QEMU);
    let _plic = hal.init_plic(PLIC_BASE_QEMU);

    hal
}
