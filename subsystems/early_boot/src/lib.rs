//! # Helix OS Early Boot Sequence
//!
//! Revolutionary, industrial-grade early boot subsystem supporting x86_64, AArch64, and RISC-V.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────────┐
//! │                    HELIX OS EARLY BOOT SEQUENCE                                  │
//! │                    ═══════════════════════════════                               │
//! │                                                                                  │
//! │  ┌─────────────────────────────────────────────────────────────────────────┐    │
//! │  │                         BOOT STAGES                                      │    │
//! │  │                                                                          │    │
//! │  │   ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐            │    │
//! │  │   │ Stage 0  │──▶│ Stage 1  │──▶│ Stage 2  │──▶│ Stage 3  │            │    │
//! │  │   │ Pre-Init │   │ CPU Init │   │ Memory   │   │ Drivers  │            │    │
//! │  │   └──────────┘   └──────────┘   └──────────┘   └──────────┘            │    │
//! │  │         │              │              │              │                   │    │
//! │  │         ▼              ▼              ▼              ▼                   │    │
//! │  │   ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐            │    │
//! │  │   │ Stage 4  │──▶│ Stage 5  │──▶│ Stage 6  │──▶│ Stage 7  │            │    │
//! │  │   │ Interrupts│  │ Timers   │   │ SMP Init │   │ Handoff  │            │    │
//! │  │   └──────────┘   └──────────┘   └──────────┘   └──────────┘            │    │
//! │  │                                                       │                  │    │
//! │  │                                                       ▼                  │    │
//! │  │                                              ┌──────────────┐            │    │
//! │  │                                              │ MAIN KERNEL  │            │    │
//! │  │                                              └──────────────┘            │    │
//! │  └─────────────────────────────────────────────────────────────────────────┘    │
//! │                                                                                  │
//! │  ┌───────────────────┬───────────────────┬───────────────────┐                 │
//! │  │      x86_64       │      AArch64      │      RISC-V       │                 │
//! │  │                   │                   │                   │                 │
//! │  │ • GDT/IDT         │ • Exception Levels│ • Privilege Modes │                 │
//! │  │ • 4/5-level Paging│ • 4-level MMU     │ • Sv39/48/57      │                 │
//! │  │ • APIC/IOAPIC     │ • GICv2/v3        │ • PLIC/CLINT      │                 │
//! │  │ • TSC/HPET        │ • Generic Timer   │ • SBI Timer       │                 │
//! │  │ • INIT-SIPI-SIPI  │ • PSCI            │ • SBI HSM         │                 │
//! │  └───────────────────┴───────────────────┴───────────────────┘                 │
//! │                                                                                  │
//! └─────────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Boot Stages
//!
//! | Stage | Name          | Description                                    |
//! |-------|---------------|------------------------------------------------|
//! | 0     | Pre-Init      | Minimal CPU setup, stack, serial output        |
//! | 1     | CPU Init      | Full CPU initialization, features, caches      |
//! | 2     | Memory Init   | Physical memory, paging, kernel mapping        |
//! | 3     | Driver Init   | Early console, framebuffer, essential drivers  |
//! | 4     | Interrupt Init| IDT/GDT, exception handlers, IRQ routing       |
//! | 5     | Timer Init    | System timers, calibration, tick setup         |
//! | 6     | SMP Init      | Secondary CPU startup, per-CPU data            |
//! | 7     | Handoff       | KASLR, final relocation, kernel entry          |
//!
//! ## Usage
//!
//! ```rust,ignore
//! use helix_early_boot::{BootSequence, BootConfig, BootInfo};
//!
//! #[no_mangle]
//! pub extern "C" fn _start(boot_info: *const BootInfo) -> ! {
//!     let config = BootConfig::default();
//!     let mut sequence = BootSequence::new(config);
//!
//!     unsafe {
//!         sequence.execute(boot_info);
//!     }
//!
//!     // Never returns - transfers to main kernel
//! }
//! ```

#![no_std]
#![feature(asm_const)]
#![feature(naked_functions)]
#![feature(core_intrinsics)]
#![feature(const_mut_refs)]
#![feature(allocator_api)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(clippy::missing_safety_doc)]

// =============================================================================
// EXTERNAL CRATES
// =============================================================================

extern crate alloc;

use bitflags::bitflags;
use spin::{Mutex, RwLock};

// =============================================================================
// MODULE DECLARATIONS
// =============================================================================

/// Core boot abstractions and traits
pub mod core;

/// Boot stages and sequencing
pub mod stages;

/// Boot information structures
pub mod info;

/// Early memory management
pub mod memory;

/// Early console and output
pub mod console;

/// Architecture-specific implementations
pub mod arch;

/// Boot protocols (Limine, Multiboot2, UEFI)
pub mod protocols;

/// Early drivers
pub mod drivers;

/// Boot handoff and KASLR
pub mod handoff;

/// Debug and diagnostic facilities
pub mod debug;

/// Error handling
pub mod error;

// =============================================================================
// RE-EXPORTS
// =============================================================================

pub use crate::console::{
    ConsoleColor, ConsoleWriter, EarlyConsole, FramebufferConsole, SerialConsole,
};
pub use crate::core::{
    BootContext, BootHooks, BootStage, BootState, CpuState, InterruptState, MemoryState,
};
pub use crate::error::{BootError, BootResult};
pub use crate::handoff::{
    BootHandoff, HandoffState, Kaslr, KaslrConfig, KernelStack, HHDM_BASE, KERNEL_VIRT_BASE,
};
pub use crate::info::{
    AcpiInfo, BootInfo, BootInfoBuilder, FramebufferInfo, MemoryMapEntry, MemoryType, SmbiosInfo,
};
pub use crate::memory::{
    BootAllocator, EarlyAllocator, PageTableSetup, PhysicalMemoryMap, VirtualMapping,
};
pub use crate::stages::{
    BootSequence, CpuInitStage, DriverInitStage, HandoffStage, InterruptInitStage, MemoryInitStage,
    PreInitStage, SmpInitStage, StageExecutor, StageResult, TimerInitStage,
};

// =============================================================================
// GLOBAL STATE
// =============================================================================

/// Global boot state - protected by spinlock for early boot safety
static BOOT_STATE: Mutex<BootState> = Mutex::new(BootState::new());

/// Boot configuration - read-mostly, write rarely
static BOOT_CONFIG: RwLock<BootConfig> = RwLock::new(BootConfig::new());

/// Early console for boot messages
static EARLY_CONSOLE: Mutex<Option<EarlyConsole>> = Mutex::new(None);

// =============================================================================
// BOOT CONFIGURATION
// =============================================================================

/// Boot configuration options
#[derive(Debug, Clone)]
pub struct BootConfig {
    /// Enable KASLR (Kernel Address Space Layout Randomization)
    pub kaslr_enabled: bool,

    /// KASLR entropy bits (typically 8-16)
    pub kaslr_entropy_bits: u8,

    /// Enable SMP (Symmetric Multi-Processing)
    pub smp_enabled: bool,

    /// Maximum number of CPUs to initialize
    pub max_cpus: usize,

    /// Enable early serial console
    pub serial_enabled: bool,

    /// Serial port configuration
    pub serial_port: SerialConfig,

    /// Enable framebuffer console
    pub framebuffer_enabled: bool,

    /// Target memory mode
    pub memory_mode: MemoryMode,

    /// Enable verbose boot logging
    pub verbose: bool,

    /// Enable debug features
    pub debug: bool,

    /// Boot timeout in milliseconds (0 = no timeout)
    pub timeout_ms: u32,

    /// Kernel load address (0 = use default)
    pub kernel_load_addr: u64,

    /// Kernel virtual base address
    pub kernel_virt_base: u64,

    /// Physical memory offset for HHDM
    pub hhdm_offset: u64,
}

impl BootConfig {
    /// Create a new default configuration
    pub const fn new() -> Self {
        Self {
            kaslr_enabled: true,
            kaslr_entropy_bits: 12,
            smp_enabled: true,
            max_cpus: 256,
            serial_enabled: true,
            serial_port: SerialConfig::default_com1(),
            framebuffer_enabled: true,
            memory_mode: MemoryMode::FourLevel,
            verbose: false,
            debug: cfg!(debug_assertions),
            timeout_ms: 0,
            kernel_load_addr: 0,
            kernel_virt_base: 0xFFFF_FFFF_8000_0000,
            hhdm_offset: 0xFFFF_8000_0000_0000,
        }
    }

    /// Create configuration for QEMU testing
    pub const fn qemu() -> Self {
        Self {
            kaslr_enabled: false,
            kaslr_entropy_bits: 0,
            smp_enabled: true,
            max_cpus: 16,
            serial_enabled: true,
            serial_port: SerialConfig::default_com1(),
            framebuffer_enabled: true,
            memory_mode: MemoryMode::FourLevel,
            verbose: true,
            debug: true,
            timeout_ms: 0,
            kernel_load_addr: 0,
            kernel_virt_base: 0xFFFF_FFFF_8000_0000,
            hhdm_offset: 0xFFFF_8000_0000_0000,
        }
    }

    /// Create minimal configuration for fast boot
    pub const fn minimal() -> Self {
        Self {
            kaslr_enabled: false,
            kaslr_entropy_bits: 0,
            smp_enabled: false,
            max_cpus: 1,
            serial_enabled: true,
            serial_port: SerialConfig::default_com1(),
            framebuffer_enabled: false,
            memory_mode: MemoryMode::FourLevel,
            verbose: false,
            debug: false,
            timeout_ms: 0,
            kernel_load_addr: 0,
            kernel_virt_base: 0xFFFF_FFFF_8000_0000,
            hhdm_offset: 0xFFFF_8000_0000_0000,
        }
    }
}

impl Default for BootConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Serial port configuration
#[derive(Debug, Clone, Copy)]
pub struct SerialConfig {
    /// I/O port address (x86) or MMIO base (ARM/RISC-V)
    pub base: u64,
    /// Baud rate
    pub baud_rate: u32,
    /// Data bits (5, 6, 7, or 8)
    pub data_bits: u8,
    /// Stop bits (1 or 2)
    pub stop_bits: u8,
    /// Parity mode
    pub parity: Parity,
}

impl SerialConfig {
    /// Default COM1 configuration for x86_64
    pub const fn default_com1() -> Self {
        Self {
            base: 0x3F8,
            baud_rate: 115200,
            data_bits: 8,
            stop_bits: 1,
            parity: Parity::None,
        }
    }

    /// Default UART0 configuration for ARM (QEMU virt)
    pub const fn default_arm_uart0() -> Self {
        Self {
            base: 0x0900_0000,
            baud_rate: 115200,
            data_bits: 8,
            stop_bits: 1,
            parity: Parity::None,
        }
    }

    /// Default UART configuration for RISC-V (QEMU virt)
    pub const fn default_riscv_uart() -> Self {
        Self {
            base: 0x1000_0000,
            baud_rate: 115200,
            data_bits: 8,
            stop_bits: 1,
            parity: Parity::None,
        }
    }
}

/// Serial parity mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parity {
    None,
    Odd,
    Even,
    Mark,
    Space,
}

/// Memory/paging mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryMode {
    /// 4-level paging (48-bit VA on x86_64, standard on ARM64)
    FourLevel,
    /// 5-level paging (57-bit VA on x86_64 with LA57)
    FiveLevel,
    /// Sv39 for RISC-V (39-bit VA, 3-level)
    Sv39,
    /// Sv48 for RISC-V (48-bit VA, 4-level)
    Sv48,
    /// Sv57 for RISC-V (57-bit VA, 5-level)
    Sv57,
}

// =============================================================================
// BOOT FLAGS
// =============================================================================

bitflags! {
    /// Boot capability flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct BootCapabilities: u64 {
        /// UEFI boot services available
        const UEFI = 1 << 0;
        /// ACPI tables available
        const ACPI = 1 << 1;
        /// Device Tree available
        const DEVICE_TREE = 1 << 2;
        /// SMBIOS tables available
        const SMBIOS = 1 << 3;
        /// Framebuffer available
        const FRAMEBUFFER = 1 << 4;
        /// Serial console available
        const SERIAL = 1 << 5;
        /// SMP capable
        const SMP = 1 << 6;
        /// KASLR capable
        const KASLR = 1 << 7;
        /// Hardware RNG available
        const HWRNG = 1 << 8;
        /// TPM available
        const TPM = 1 << 9;
        /// Secure Boot active
        const SECURE_BOOT = 1 << 10;
        /// Hypervisor present
        const HYPERVISOR = 1 << 11;
        /// 5-level paging capable (x86_64 LA57)
        const LA57 = 1 << 12;
        /// 64KB granule capable (ARM64)
        const GRANULE_64K = 1 << 13;
        /// 16KB granule capable (ARM64)
        const GRANULE_16K = 1 << 14;
        /// SVE capable (ARM64)
        const SVE = 1 << 15;
        /// SME capable (ARM64)
        const SME = 1 << 16;
        /// Vector extension capable (RISC-V)
        const VECTOR = 1 << 17;
        /// Hypervisor extension (RISC-V)
        const HYPERVISOR_EXT = 1 << 18;
    }
}

bitflags! {
    /// Boot status flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct BootStatus: u32 {
        /// Pre-init complete
        const PRE_INIT = 1 << 0;
        /// CPU initialized
        const CPU_INIT = 1 << 1;
        /// Memory initialized
        const MEMORY_INIT = 1 << 2;
        /// Drivers initialized
        const DRIVERS_INIT = 1 << 3;
        /// Interrupts initialized
        const INTERRUPTS_INIT = 1 << 4;
        /// Timers initialized
        const TIMERS_INIT = 1 << 5;
        /// SMP initialized
        const SMP_INIT = 1 << 6;
        /// Handoff complete
        const HANDOFF = 1 << 7;
        /// Error occurred
        const ERROR = 1 << 31;
    }
}

// =============================================================================
// ARCHITECTURE DETECTION
// =============================================================================

/// Target architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    /// x86-64 / AMD64
    X86_64,
    /// ARM 64-bit / AArch64
    AArch64,
    /// RISC-V 64-bit
    RiscV64,
    /// Unknown architecture
    Unknown,
}

impl Architecture {
    /// Get the current architecture at compile time
    pub const fn current() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            Self::X86_64
        }

        #[cfg(target_arch = "aarch64")]
        {
            Self::AArch64
        }

        #[cfg(target_arch = "riscv64")]
        {
            Self::RiscV64
        }

        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "riscv64"
        )))]
        {
            Self::Unknown
        }
    }

    /// Get architecture name
    pub const fn name(self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64",
            Self::AArch64 => "aarch64",
            Self::RiscV64 => "riscv64",
            Self::Unknown => "unknown",
        }
    }

    /// Get page size for this architecture
    pub const fn page_size(self) -> usize {
        match self {
            Self::X86_64 => 4096,
            Self::AArch64 => 4096, // Default, can be 16K or 64K
            Self::RiscV64 => 4096,
            Self::Unknown => 4096,
        }
    }
}

// =============================================================================
// MAIN ENTRY POINT
// =============================================================================

/// Main early boot entry point
///
/// # Safety
///
/// This function must only be called once from the bootloader/startup code.
/// The `boot_info` pointer must be valid and properly aligned.
#[no_mangle]
pub unsafe extern "C" fn helix_early_boot(boot_info: *const info::BootInfo) -> ! {
    // Initialize boot state
    {
        let mut state = BOOT_STATE.lock();
        state.set_stage(BootStage::PreInit);
        state.set_architecture(Architecture::current());
    }

    // Get configuration
    let config = BOOT_CONFIG.read().clone();

    // Create and execute boot sequence
    let mut sequence = BootSequence::new(config);

    match sequence.execute(boot_info) {
        Ok(entry_point) => {
            // Transfer to main kernel
            let kernel_entry: extern "C" fn(*const info::BootInfo) -> ! =
                core::mem::transmute(entry_point);
            kernel_entry(boot_info);
        },
        Err(e) => {
            // Boot failed - try to output error and halt
            boot_panic(&e);
        },
    }
}

/// Boot panic handler
fn boot_panic(error: &BootError) -> ! {
    // Try to output to serial if available
    if let Some(ref mut console) = *EARLY_CONSOLE.lock() {
        use core::fmt::Write;
        let _ = writeln!(console, "\n!!! BOOT PANIC !!!");
        let _ = writeln!(console, "Error: {:?}", error);
        let _ = writeln!(console, "Stage: {:?}", BOOT_STATE.lock().current_stage());
    }

    // Halt the system
    arch::halt_forever();
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Get current boot stage
pub fn current_stage() -> BootStage {
    BOOT_STATE.lock().current_stage()
}

/// Get boot status flags
pub fn boot_status() -> BootStatus {
    BOOT_STATE.lock().status()
}

/// Check if a specific boot stage is complete
pub fn stage_complete(stage: BootStage) -> bool {
    BOOT_STATE.lock().stage_complete(stage)
}

/// Log a boot message
pub fn boot_log(message: &str) {
    if let Some(ref mut console) = *EARLY_CONSOLE.lock() {
        use core::fmt::Write;
        let _ = writeln!(console, "[BOOT] {}", message);
    }
}

/// Log a boot message with formatting
#[macro_export]
macro_rules! boot_log {
    ($($arg:tt)*) => {
        $crate::boot_log(&alloc::format!($($arg)*))
    };
}

/// Print boot progress
pub fn boot_progress(stage: &str, current: usize, total: usize) {
    if let Some(ref mut console) = *EARLY_CONSOLE.lock() {
        use core::fmt::Write;
        let percent = (current * 100) / total.max(1);
        let _ = write!(console, "\r[{:>3}%] {}", percent, stage);
    }
}

// =============================================================================
// VERSION INFORMATION
// =============================================================================

/// Early boot subsystem version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build timestamp
pub const BUILD_TIME: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " built on ",
    // Would use build.rs for actual timestamp
    "2026-01-29"
);

/// Git commit (if available)
pub const GIT_COMMIT: Option<&str> = option_env!("GIT_COMMIT");
