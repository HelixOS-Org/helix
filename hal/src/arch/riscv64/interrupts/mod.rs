//! # RISC-V Interrupt Controller Framework
//!
//! This module provides interrupt controller support for RISC-V systems.
//!
//! ## Submodules
//!
//! - `clint`: Core Local Interruptor (timer and software interrupts)
//! - `plic`: Platform-Level Interrupt Controller (external interrupts)
//! - `irq`: IRQ management and dispatch

pub mod clint;
pub mod plic;
pub mod irq;

// Re-export commonly used items
pub use clint::{Clint, set_timer, clear_software_interrupt, send_software_interrupt};
pub use plic::{Plic, PlicContext, claim_interrupt, complete_interrupt};
pub use irq::{IrqHandler, register_irq_handler, dispatch_interrupt};

use super::core::csr::{self, interrupt};

// ============================================================================
// Interrupt Constants
// ============================================================================

/// Maximum number of external interrupts
pub const MAX_EXTERNAL_IRQS: usize = 1024;

/// Maximum number of harts
pub const MAX_HARTS: usize = 256;

// ============================================================================
// Interrupt Types
// ============================================================================

/// RISC-V interrupt types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptType {
    /// Software interrupt (IPI)
    Software,
    /// Timer interrupt
    Timer,
    /// External interrupt (from PLIC)
    External,
}

impl InterruptType {
    /// Get the SIE/SIP bit for this interrupt type
    pub const fn sie_bit(self) -> u64 {
        match self {
            Self::Software => interrupt::SSIP,
            Self::Timer => interrupt::STIP,
            Self::External => interrupt::SEIP,
        }
    }

    /// Get the interrupt cause code
    pub const fn cause_code(self) -> u64 {
        match self {
            Self::Software => csr::irq_cause::SUPERVISOR_SOFTWARE,
            Self::Timer => csr::irq_cause::SUPERVISOR_TIMER,
            Self::External => csr::irq_cause::SUPERVISOR_EXTERNAL,
        }
    }
}

// ============================================================================
// Interrupt Control
// ============================================================================

/// Enable a specific interrupt type in SIE
#[inline]
pub fn enable_interrupt(int_type: InterruptType) {
    csr::enable_sie(int_type.sie_bit());
}

/// Disable a specific interrupt type in SIE
#[inline]
pub fn disable_interrupt(int_type: InterruptType) {
    csr::disable_sie(int_type.sie_bit());
}

/// Check if an interrupt type is enabled
#[inline]
pub fn is_interrupt_enabled(int_type: InterruptType) -> bool {
    csr::read_sie() & int_type.sie_bit() != 0
}

/// Check if an interrupt is pending
#[inline]
pub fn is_interrupt_pending(int_type: InterruptType) -> bool {
    csr::read_sip() & int_type.sie_bit() != 0
}

/// Enable all supervisor interrupts (SIE, STIE, SEIE)
pub fn enable_all_interrupts() {
    csr::enable_sie(interrupt::S_ALL);
}

/// Disable all supervisor interrupts
pub fn disable_all_interrupts() {
    csr::disable_sie(interrupt::S_ALL);
}

/// Enable global interrupts (sstatus.SIE)
#[inline]
pub fn enable_global_interrupts() {
    csr::enable_interrupts();
}

/// Disable global interrupts (sstatus.SIE)
#[inline]
pub fn disable_global_interrupts() {
    csr::disable_interrupts();
}

/// Disable global interrupts and return previous state
#[inline]
pub fn disable_global_interrupts_save() -> bool {
    csr::disable_interrupts_save()
}

/// Restore global interrupt state
#[inline]
pub fn restore_global_interrupts(was_enabled: bool) {
    csr::restore_interrupts(was_enabled);
}

// ============================================================================
// Interrupt Controller Configuration
// ============================================================================

/// Interrupt controller configuration
#[derive(Debug, Clone, Copy)]
pub struct InterruptConfig {
    /// CLINT base address
    pub clint_base: usize,
    /// PLIC base address
    pub plic_base: usize,
    /// Number of external interrupt sources
    pub num_sources: usize,
    /// Number of contexts (2 per hart for M and S mode)
    pub num_contexts: usize,
}

impl InterruptConfig {
    /// QEMU virt machine defaults
    pub const QEMU_VIRT: Self = Self {
        clint_base: 0x0200_0000,
        plic_base: 0x0C00_0000,
        num_sources: 127,
        num_contexts: 32, // 16 harts * 2 contexts
    };

    /// SiFive U74 defaults
    pub const SIFIVE_U74: Self = Self {
        clint_base: 0x0200_0000,
        plic_base: 0x0C00_0000,
        num_sources: 127,
        num_contexts: 10, // 5 harts * 2 contexts
    };
}

/// Global interrupt configuration
static mut INTERRUPT_CONFIG: InterruptConfig = InterruptConfig::QEMU_VIRT;

/// Set the interrupt controller configuration
///
/// # Safety
/// Must be called before any interrupt controller access.
pub unsafe fn set_interrupt_config(config: InterruptConfig) {
    INTERRUPT_CONFIG = config;
}

/// Get the current interrupt configuration
pub fn get_interrupt_config() -> InterruptConfig {
    unsafe { INTERRUPT_CONFIG }
}

// ============================================================================
// Initialization
// ============================================================================

/// Initialize the interrupt subsystem
///
/// # Safety
/// Must be called with valid CLINT/PLIC addresses.
pub unsafe fn init(config: InterruptConfig, hart_id: usize) {
    set_interrupt_config(config);

    // Initialize CLINT for this hart
    clint::init(config.clint_base, hart_id);

    // Initialize PLIC for this hart's S-mode context
    let context = plic::hart_to_smode_context(hart_id);
    plic::init(config.plic_base, context);

    // Enable supervisor interrupts in SIE
    enable_all_interrupts();
}

/// Initialize secondary hart interrupts
///
/// # Safety
/// Must be called on secondary hart with proper timing.
pub unsafe fn init_secondary(hart_id: usize) {
    let config = get_interrupt_config();

    // Initialize CLINT for this hart
    clint::init(config.clint_base, hart_id);

    // Initialize PLIC for this hart's S-mode context
    let context = plic::hart_to_smode_context(hart_id);
    plic::init_context(config.plic_base, context);

    // Enable supervisor interrupts in SIE
    enable_all_interrupts();
}

// ============================================================================
// Interrupt Statistics
// ============================================================================

/// Interrupt statistics
#[derive(Debug, Default, Clone)]
pub struct InterruptStats {
    /// Number of timer interrupts
    pub timer_count: u64,
    /// Number of software interrupts (IPI)
    pub software_count: u64,
    /// Number of external interrupts
    pub external_count: u64,
    /// Per-IRQ counts (first 64 IRQs)
    pub irq_counts: [u64; 64],
}

impl InterruptStats {
    /// Create new stats
    pub const fn new() -> Self {
        Self {
            timer_count: 0,
            software_count: 0,
            external_count: 0,
            irq_counts: [0; 64],
        }
    }

    /// Record a timer interrupt
    pub fn record_timer(&mut self) {
        self.timer_count += 1;
    }

    /// Record a software interrupt
    pub fn record_software(&mut self) {
        self.software_count += 1;
    }

    /// Record an external interrupt
    pub fn record_external(&mut self, irq: usize) {
        self.external_count += 1;
        if irq < 64 {
            self.irq_counts[irq] += 1;
        }
    }

    /// Get total interrupt count
    pub fn total(&self) -> u64 {
        self.timer_count + self.software_count + self.external_count
    }
}
