//! # x86_64 APIC Framework
//!
//! This module provides an industrial-grade implementation of the Advanced
//! Programmable Interrupt Controller (APIC) subsystem, including Local APIC,
//! I/O APIC, and x2APIC support.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         APIC Architecture                                │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │   ┌─────────────┐     ┌─────────────┐     ┌─────────────┐               │
//! │   │   CPU 0     │     │   CPU 1     │     │   CPU n     │               │
//! │   │ ┌─────────┐ │     │ ┌─────────┐ │     │ ┌─────────┐ │               │
//! │   │ │Local    │ │     │ │Local    │ │     │ │Local    │ │               │
//! │   │ │APIC     │ │     │ │APIC     │ │     │ │APIC     │ │               │
//! │   │ │         │ │     │ │         │ │     │ │         │ │               │
//! │   │ │ Timer   │ │     │ │ Timer   │ │     │ │ Timer   │ │               │
//! │   │ │ LVT     │ │     │ │ LVT     │ │     │ │ LVT     │ │               │
//! │   │ │ IPI     │ │     │ │ IPI     │ │     │ │ IPI     │ │               │
//! │   │ └────┬────┘ │     │ └────┬────┘ │     │ └────┬────┘ │               │
//! │   └──────┼──────┘     └──────┼──────┘     └──────┼──────┘               │
//! │          │                   │                   │                       │
//! │          └───────────────────┼───────────────────┘                       │
//! │                              │                                           │
//! │                    ┌─────────┴─────────┐                                │
//! │                    │    APIC Bus       │                                │
//! │                    └─────────┬─────────┘                                │
//! │                              │                                           │
//! │                    ┌─────────┴─────────┐                                │
//! │                    │    I/O APIC       │                                │
//! │                    │                   │                                │
//! │                    │  IRQ 0-23         │                                │
//! │                    │  Redirection      │                                │
//! │                    └─────────┬─────────┘                                │
//! │                              │                                           │
//! │          ┌───────────────────┼───────────────────┐                      │
//! │          │                   │                   │                       │
//! │   ┌──────┴──────┐     ┌──────┴──────┐     ┌──────┴──────┐               │
//! │   │   Device    │     │   Device    │     │   Device    │               │
//! │   │   (IRQ)     │     │   (IRQ)     │     │   (MSI)     │               │
//! │   └─────────────┘     └─────────────┘     └─────────────┘               │
//! │                                                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Features
//!
//! - xAPIC (memory-mapped) support
//! - x2APIC (MSR-based) support with automatic detection
//! - I/O APIC for external interrupt routing
//! - IPI (Inter-Processor Interrupt) support
//! - APIC Timer with one-shot and periodic modes
//! - MSI/MSI-X support for PCIe devices
//! - NMI and SMI handling
//!
//! ## Usage
//!
//! ```rust,ignore
//! use hal::arch::x86_64::apic;
//!
//! // Initialize the APIC subsystem
//! unsafe { apic::init(); }
//!
//! // Send an IPI to another processor
//! apic::send_ipi(target_cpu, apic::IpiVector::Reschedule);
//!
//! // Configure and start the APIC timer
//! apic::timer::start_periodic(1_000_000); // 1ms interval
//! ```

#![allow(dead_code)]

pub mod ioapic;
pub mod ipi;
pub mod local;
pub mod msi;
pub mod x2apic;

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

pub use ioapic::{DeliveryMode, DestinationMode, IoApic, Polarity, RedirectionEntry, TriggerMode};
pub use ipi::{IpiBarrier, IpiDeliveryMode, IpiDestination};
pub use local::{ApicError, LocalApic, LocalApicMode, LvtEntry, TimerDivide, TimerMode};
pub use msi::{MsiAddress, MsiData, MsiDeliveryMode, MsiMessage, MsixTableEntry};
pub use x2apic::X2Apic;

// =============================================================================
// Constants
// =============================================================================

/// Default Local APIC base address
pub const LAPIC_BASE_DEFAULT: u64 = 0xFEE0_0000;

/// Default I/O APIC base address
pub const IOAPIC_BASE_DEFAULT: u64 = 0xFEC0_0000;

/// APIC spurious vector
pub const SPURIOUS_VECTOR: u8 = 0xFF;

/// APIC error vector
pub const ERROR_VECTOR: u8 = 0xFE;

/// APIC timer vector (default)
pub const TIMER_VECTOR: u8 = 0x40;

/// Reschedule IPI vector
pub const RESCHEDULE_VECTOR: u8 = 0xFD;

/// TLB shootdown IPI vector
pub const TLB_VECTOR: u8 = 0xFC;

/// Stop/halt IPI vector
pub const STOP_VECTOR: u8 = 0xFB;

/// Call function IPI vector
pub const CALL_VECTOR: u8 = 0xFA;

/// Maximum number of I/O APICs
pub const MAX_IOAPICS: usize = 8;

/// Maximum number of CPUs (Local APICs)
pub const MAX_CPUS: usize = 256;

// =============================================================================
// APIC State
// =============================================================================

static APIC_INITIALIZED: AtomicBool = AtomicBool::new(false);
static X2APIC_ENABLED: AtomicBool = AtomicBool::new(false);
static BSP_APIC_ID: AtomicU32 = AtomicU32::new(0);
static LAPIC_BASE: core::sync::atomic::AtomicU64 =
    core::sync::atomic::AtomicU64::new(LAPIC_BASE_DEFAULT);

// =============================================================================
// Public Interface
// =============================================================================

/// Initialize the APIC subsystem for the BSP
///
/// This function:
/// 1. Detects APIC presence and capabilities
/// 2. Enables x2APIC if available and preferred
/// 3. Sets up the Local APIC
/// 4. Configures the spurious interrupt vector
///
/// # Safety
///
/// This must be called exactly once during early boot on the BSP,
/// before enabling interrupts.
pub unsafe fn init() -> Result<(), ApicError> {
    if APIC_INITIALIZED.swap(true, Ordering::SeqCst) {
        return Err(ApicError::AlreadyInitialized);
    }

    // Check APIC presence
    if !is_apic_present() {
        APIC_INITIALIZED.store(false, Ordering::SeqCst);
        return Err(ApicError::NotPresent);
    }

    // Detect x2APIC support
    let x2apic_supported = is_x2apic_supported();

    // Try to enable x2APIC if supported
    if x2apic_supported {
        if unsafe { enable_x2apic() } {
            X2APIC_ENABLED.store(true, Ordering::SeqCst);
            log::info!("APIC: x2APIC mode enabled");
        } else {
            log::warn!("APIC: x2APIC supported but enable failed, falling back to xAPIC");
        }
    }

    // Get the LAPIC base address
    let base = get_lapic_base();
    LAPIC_BASE.store(base, Ordering::SeqCst);

    // Initialize the Local APIC
    unsafe { local::init(base)? };

    // Store BSP APIC ID
    let bsp_id = local::get_apic_id();
    BSP_APIC_ID.store(bsp_id, Ordering::SeqCst);

    log::info!(
        "APIC: Initialized (BSP ID={}, base={:#x}, x2APIC={})",
        bsp_id,
        base,
        X2APIC_ENABLED.load(Ordering::Relaxed)
    );

    Ok(())
}

/// Initialize the Local APIC for an Application Processor
///
/// # Safety
///
/// Must be called after `init()` has completed on the BSP.
pub unsafe fn init_for_ap() -> Result<(), ApicError> {
    if !APIC_INITIALIZED.load(Ordering::Acquire) {
        return Err(ApicError::NotInitialized);
    }

    let base = LAPIC_BASE.load(Ordering::Acquire);
    unsafe { local::init(base)? };

    let ap_id = local::get_apic_id();
    log::debug!("APIC: AP {} initialized", ap_id);

    Ok(())
}

/// Check if APIC is present
#[inline]
pub fn is_apic_present() -> bool {
    // CPUID.01H:EDX.APIC[bit 9]
    let edx: u32;
    unsafe {
        // Save rbx which is reserved by LLVM
        let rbx: u64;
        core::arch::asm!(
            "mov {0}, rbx",
            "mov eax, 1",
            "cpuid",
            "mov rbx, {0}",
            out(reg) rbx,
            out("edx") edx,
            out("eax") _,
            out("ecx") _,
            options(nostack, preserves_flags),
        );
        let _ = rbx; // Silence unused warning
    }
    edx & (1 << 9) != 0
}

/// Check if x2APIC is supported
#[inline]
pub fn is_x2apic_supported() -> bool {
    // CPUID.01H:ECX.x2APIC[bit 21]
    let ecx: u32;
    unsafe {
        // Save rbx which is reserved by LLVM
        let rbx: u64;
        core::arch::asm!(
            "mov {0}, rbx",
            "mov eax, 1",
            "cpuid",
            "mov rbx, {0}",
            out(reg) rbx,
            out("ecx") ecx,
            out("eax") _,
            out("edx") _,
            options(nostack, preserves_flags),
        );
        let _ = rbx; // Silence unused warning
    }
    ecx & (1 << 21) != 0
}

/// Check if x2APIC is currently enabled
#[inline]
pub fn is_x2apic_enabled() -> bool {
    X2APIC_ENABLED.load(Ordering::Relaxed)
}

/// Get the Local APIC base address from MSR
fn get_lapic_base() -> u64 {
    const IA32_APIC_BASE_MSR: u32 = 0x1B;

    let (low, high): (u32, u32);
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("ecx") IA32_APIC_BASE_MSR,
            out("eax") low,
            out("edx") high,
            options(nostack, preserves_flags),
        );
    }

    (((high as u64) << 32) | (low as u64)) & 0xFFFF_FFFF_FFFF_F000
}

/// Enable x2APIC mode
///
/// Returns true if successful.
unsafe fn enable_x2apic() -> bool {
    const IA32_APIC_BASE_MSR: u32 = 0x1B;
    const APIC_ENABLE: u64 = 1 << 11;
    const X2APIC_ENABLE: u64 = 1 << 10;

    let (low, high): (u32, u32);
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("ecx") IA32_APIC_BASE_MSR,
            out("eax") low,
            out("edx") high,
            options(nostack, preserves_flags),
        );
    }

    let value = ((high as u64) << 32) | (low as u64);
    let new_value = value | APIC_ENABLE | X2APIC_ENABLE;

    let new_low = new_value as u32;
    let new_high = (new_value >> 32) as u32;

    unsafe {
        core::arch::asm!(
            "wrmsr",
            in("ecx") IA32_APIC_BASE_MSR,
            in("eax") new_low,
            in("edx") new_high,
            options(nostack, preserves_flags),
        );
    }

    // Verify
    let (verify_low, verify_high): (u32, u32);
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("ecx") IA32_APIC_BASE_MSR,
            out("eax") verify_low,
            out("edx") verify_high,
            options(nostack, preserves_flags),
        );
    }

    let verify = ((verify_high as u64) << 32) | (verify_low as u64);
    verify & X2APIC_ENABLE != 0
}

/// Get the current CPU's APIC ID
#[inline]
pub fn current_apic_id() -> u32 {
    local::get_apic_id()
}

/// Get the BSP's APIC ID
#[inline]
pub fn bsp_apic_id() -> u32 {
    BSP_APIC_ID.load(Ordering::Relaxed)
}

/// Check if the current CPU is the BSP
#[inline]
pub fn is_bsp() -> bool {
    current_apic_id() == bsp_apic_id()
}

/// Send End-Of-Interrupt signal
///
/// This must be called at the end of every interrupt handler.
#[inline]
pub fn end_of_interrupt() {
    local::send_eoi();
}

/// Send an IPI to another processor
///
/// # Safety
///
/// The target CPU must be valid and running.
#[inline]
pub unsafe fn send_ipi(destination: IpiDestination, vector: u8) {
    unsafe {
        local::send_ipi(destination, vector);
    }
}

/// Broadcast an IPI to all processors except self
///
/// # Safety
///
/// - The Local APIC must be properly initialized.
/// - The vector must be a valid interrupt vector (32-255).
/// - All target CPUs must be capable of receiving interrupts.
#[inline]
pub unsafe fn broadcast_ipi(vector: u8) {
    unsafe {
        local::send_ipi(IpiDestination::AllExcludingSelf, vector);
    }
}

/// Initialize an I/O APIC
///
/// # Safety
///
/// The base address must be valid and mapped.
pub unsafe fn init_ioapic(_id: u8, base: u64, gsi_base: u32) -> Result<(), ApicError> {
    unsafe {
        ioapic::register_ioapic(base, gsi_base)?;
    }
    Ok(())
}

// =============================================================================
// APIC Register Offsets (xAPIC mode)
// =============================================================================

/// Local APIC register offsets (for memory-mapped access)
pub mod registers {
    /// APIC ID Register
    pub const ID: u32 = 0x020;
    /// APIC Version Register
    pub const VERSION: u32 = 0x030;
    /// Task Priority Register
    pub const TPR: u32 = 0x080;
    /// Arbitration Priority Register
    pub const APR: u32 = 0x090;
    /// Processor Priority Register
    pub const PPR: u32 = 0x0A0;
    /// End Of Interrupt Register
    pub const EOI: u32 = 0x0B0;
    /// Remote Read Register
    pub const RRD: u32 = 0x0C0;
    /// Logical Destination Register
    pub const LDR: u32 = 0x0D0;
    /// Destination Format Register
    pub const DFR: u32 = 0x0E0;
    /// Spurious Interrupt Vector Register
    pub const SVR: u32 = 0x0F0;
    /// In-Service Register (8 registers)
    pub const ISR_BASE: u32 = 0x100;
    /// Trigger Mode Register (8 registers)
    pub const TMR_BASE: u32 = 0x180;
    /// Interrupt Request Register (8 registers)
    pub const IRR_BASE: u32 = 0x200;
    /// Error Status Register
    pub const ESR: u32 = 0x280;
    /// LVT CMCI Register
    pub const LVT_CMCI: u32 = 0x2F0;
    /// Interrupt Command Register (low)
    pub const ICR_LOW: u32 = 0x300;
    /// Interrupt Command Register (high)
    pub const ICR_HIGH: u32 = 0x310;
    /// LVT Timer Register
    pub const LVT_TIMER: u32 = 0x320;
    /// LVT Thermal Sensor Register
    pub const LVT_THERMAL: u32 = 0x330;
    /// LVT Performance Counter Register
    pub const LVT_PERF: u32 = 0x340;
    /// LVT LINT0 Register
    pub const LVT_LINT0: u32 = 0x350;
    /// LVT LINT1 Register
    pub const LVT_LINT1: u32 = 0x360;
    /// LVT Error Register
    pub const LVT_ERROR: u32 = 0x370;
    /// Initial Count Register (for timer)
    pub const TIMER_ICR: u32 = 0x380;
    /// Current Count Register (for timer)
    pub const TIMER_CCR: u32 = 0x390;
    /// Divide Configuration Register (for timer)
    pub const TIMER_DCR: u32 = 0x3E0;
}

/// x2APIC MSR offsets
pub mod x2apic_msr {
    /// Base MSR address for x2APIC registers
    pub const BASE: u32 = 0x800;
    /// APIC ID register MSR
    pub const ID: u32 = BASE + 0x02;
    /// APIC version register MSR
    pub const VERSION: u32 = BASE + 0x03;
    /// Task priority register MSR
    pub const TPR: u32 = BASE + 0x08;
    /// Processor priority register MSR
    pub const PPR: u32 = BASE + 0x0A;
    /// End-of-interrupt register MSR
    pub const EOI: u32 = BASE + 0x0B;
    /// Logical destination register MSR
    pub const LDR: u32 = BASE + 0x0D;
    /// Spurious interrupt vector register MSR
    pub const SVR: u32 = BASE + 0x0F;
    /// In-service register base MSR
    pub const ISR_BASE: u32 = BASE + 0x10;
    /// Trigger mode register base MSR
    pub const TMR_BASE: u32 = BASE + 0x18;
    /// Interrupt request register base MSR
    pub const IRR_BASE: u32 = BASE + 0x20;
    /// Error status register MSR
    pub const ESR: u32 = BASE + 0x28;
    /// LVT corrected machine check interrupt MSR
    pub const LVT_CMCI: u32 = BASE + 0x2F;
    /// Interrupt command register MSR (combined high/low in x2APIC)
    pub const ICR: u32 = BASE + 0x30;
    /// LVT timer register MSR
    pub const LVT_TIMER: u32 = BASE + 0x32;
    /// LVT thermal sensor register MSR
    pub const LVT_THERMAL: u32 = BASE + 0x33;
    /// LVT performance monitoring counter MSR
    pub const LVT_PERF: u32 = BASE + 0x34;
    /// LVT LINT0 register MSR
    pub const LVT_LINT0: u32 = BASE + 0x35;
    /// LVT LINT1 register MSR
    pub const LVT_LINT1: u32 = BASE + 0x36;
    /// LVT error register MSR
    pub const LVT_ERROR: u32 = BASE + 0x37;
    /// Timer initial count register MSR
    pub const TIMER_ICR: u32 = BASE + 0x38;
    /// Timer current count register MSR
    pub const TIMER_CCR: u32 = BASE + 0x39;
    /// Timer divide configuration register MSR
    pub const TIMER_DCR: u32 = BASE + 0x3E;
    /// Self IPI register MSR
    pub const SELF_IPI: u32 = BASE + 0x3F;
}

// =============================================================================
// Compile-time Assertions
// =============================================================================

const _: () = {
    // Verify register alignment
    assert!(registers::ID % 16 == 0);
    assert!(registers::EOI % 16 == 0);
    assert!(registers::SVR % 16 == 0);
};
