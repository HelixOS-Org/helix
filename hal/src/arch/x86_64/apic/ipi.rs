//! # Inter-Processor Interrupts (IPI)
//!
//! This module provides a unified interface for sending IPIs
//! that works with both xAPIC and x2APIC modes.
//!
//! ## IPI Types
//!
//! - **Fixed**: Regular interrupt with a vector
//! - **Lowest Priority**: Delivered to lowest-priority CPU
//! - **SMI**: System Management Interrupt
//! - **NMI**: Non-Maskable Interrupt
//! - **INIT**: Initialization (used for CPU startup)
//! - **SIPI**: Startup IPI (specifies AP entry point)
//!
//! ## Destination Modes
//!
//! - **Physical**: Target a specific APIC ID
//! - **Logical**: Target a group of CPUs via logical destination
//!
//! ## Common IPI Vectors
//!
//! - 0xFD: Reschedule IPI (trigger scheduler)
//! - 0xFC: TLB shootdown IPI
//! - 0xFB: Stop/Halt IPI
//! - 0xFA: Call function IPI

use core::sync::atomic::{AtomicU32, Ordering};

use super::local::{read_lapic, write_lapic};
use super::registers;
use super::{is_x2apic_enabled, RESCHEDULE_VECTOR, TLB_VECTOR, STOP_VECTOR, CALL_VECTOR};

// =============================================================================
// IPI Destination
// =============================================================================

/// IPI destination specifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpiDestination {
    /// Send to a specific APIC ID
    Single(u32),
    /// Send to self
    Myself,
    /// Broadcast to all CPUs including self
    AllIncludingSelf,
    /// Broadcast to all CPUs excluding self
    AllExcludingSelf,
}

impl IpiDestination {
    /// Get the destination shorthand value
    #[inline]
    pub fn shorthand(&self) -> u8 {
        match self {
            IpiDestination::Single(_) => 0b00,
            IpiDestination::Myself => 0b01,
            IpiDestination::AllIncludingSelf => 0b10,
            IpiDestination::AllExcludingSelf => 0b11,
        }
    }

    /// Get the destination APIC ID (or 0 for broadcast)
    #[inline]
    pub fn apic_id(&self) -> u32 {
        match self {
            IpiDestination::Single(id) => *id,
            _ => 0,
        }
    }
}

// =============================================================================
// IPI Type
// =============================================================================

/// IPI delivery mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IpiDeliveryMode {
    /// Fixed interrupt with vector
    Fixed = 0b000,
    /// Lowest priority delivery
    LowestPriority = 0b001,
    /// System Management Interrupt
    Smi = 0b010,
    /// Non-Maskable Interrupt
    Nmi = 0b100,
    /// INIT signal
    Init = 0b101,
    /// Startup IPI
    Sipi = 0b110,
}

/// IPI trigger mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpiTrigger {
    /// Edge-triggered
    Edge,
    /// Level-triggered
    Level,
}

/// IPI level (for level-triggered)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpiLevel {
    /// De-assert
    Deassert,
    /// Assert
    Assert,
}

// =============================================================================
// IPI Statistics
// =============================================================================

/// IPI send counter
static IPI_SEND_COUNT: AtomicU32 = AtomicU32::new(0);

/// IPI receive counter (per-CPU would be in per_cpu data)
static IPI_RECV_COUNT: AtomicU32 = AtomicU32::new(0);

/// Get the total IPI send count
#[inline]
pub fn ipi_send_count() -> u32 {
    IPI_SEND_COUNT.load(Ordering::Relaxed)
}

/// Get the total IPI receive count
#[inline]
pub fn ipi_recv_count() -> u32 {
    IPI_RECV_COUNT.load(Ordering::Relaxed)
}

/// Increment receive count (called from IPI handler)
#[inline]
pub fn ipi_received() {
    IPI_RECV_COUNT.fetch_add(1, Ordering::Relaxed);
}

// =============================================================================
// Raw IPI Send
// =============================================================================

/// Build ICR value for xAPIC
fn build_icr_xapic(
    vector: u8,
    delivery_mode: IpiDeliveryMode,
    dest_mode_logical: bool,
    level: IpiLevel,
    trigger: IpiTrigger,
    shorthand: u8,
) -> u32 {
    let mut icr: u32 = vector as u32;

    // Delivery mode (bits 8-10)
    icr |= (delivery_mode as u32) << 8;

    // Destination mode (bit 11)
    if dest_mode_logical {
        icr |= 1 << 11;
    }

    // Level (bit 14)
    if level == IpiLevel::Assert {
        icr |= 1 << 14;
    }

    // Trigger mode (bit 15)
    if trigger == IpiTrigger::Level {
        icr |= 1 << 15;
    }

    // Destination shorthand (bits 18-19)
    icr |= (shorthand as u32) << 18;

    icr
}

/// Build ICR value for x2APIC (64-bit)
fn build_icr_x2apic(
    dest: u32,
    vector: u8,
    delivery_mode: IpiDeliveryMode,
    dest_mode_logical: bool,
    level: IpiLevel,
    trigger: IpiTrigger,
    shorthand: u8,
) -> u64 {
    let mut icr: u64 = (dest as u64) << 32;

    icr |= vector as u64;
    icr |= (delivery_mode as u64) << 8;

    if dest_mode_logical {
        icr |= 1 << 11;
    }

    if level == IpiLevel::Assert {
        icr |= 1 << 14;
    }

    if trigger == IpiTrigger::Level {
        icr |= 1 << 15;
    }

    icr |= (shorthand as u64) << 18;

    icr
}

/// Send an IPI in xAPIC mode
unsafe fn send_ipi_xapic(
    dest: u32,
    vector: u8,
    delivery_mode: IpiDeliveryMode,
    shorthand: u8,
) {
    // Wait for previous IPI to complete
    wait_ipi_idle_xapic();

    // Write destination to ICR high
    if shorthand == 0 {
        write_lapic(registers::ICR_HIGH, dest << 24);
    }

    // Write command to ICR low (this triggers the IPI)
    let icr = build_icr_xapic(
        vector,
        delivery_mode,
        false, // Physical destination mode
        IpiLevel::Assert,
        IpiTrigger::Edge,
        shorthand,
    );
    write_lapic(registers::ICR_LOW, icr);

    IPI_SEND_COUNT.fetch_add(1, Ordering::Relaxed);
}

/// Send an IPI in x2APIC mode
unsafe fn send_ipi_x2apic(
    dest: u32,
    vector: u8,
    delivery_mode: IpiDeliveryMode,
    shorthand: u8,
) {
    let icr = build_icr_x2apic(
        dest,
        vector,
        delivery_mode,
        false, // Physical destination mode
        IpiLevel::Assert,
        IpiTrigger::Edge,
        shorthand,
    );

    // x2APIC ICR write
    let low = icr as u32;
    let high = (icr >> 32) as u32;
    core::arch::asm!(
        "wrmsr",
        in("ecx") 0x830u32, // x2APIC ICR MSR
        in("eax") low,
        in("edx") high,
        options(nostack, preserves_flags),
    );

    IPI_SEND_COUNT.fetch_add(1, Ordering::Relaxed);
}

/// Wait for IPI to be delivered (xAPIC only)
unsafe fn wait_ipi_idle_xapic() {
    // Bit 12 = Delivery Status (1 = send pending)
    while read_lapic(registers::ICR_LOW) & (1 << 12) != 0 {
        core::hint::spin_loop();
    }
}

// =============================================================================
// High-Level IPI Interface
// =============================================================================

/// Send a fixed IPI
pub fn send_ipi(dest: IpiDestination, vector: u8) {
    unsafe {
        if is_x2apic_enabled() {
            send_ipi_x2apic(
                dest.apic_id(),
                vector,
                IpiDeliveryMode::Fixed,
                dest.shorthand(),
            );
        } else {
            send_ipi_xapic(
                dest.apic_id(),
                vector,
                IpiDeliveryMode::Fixed,
                dest.shorthand(),
            );
        }
    }
}

/// Send an NMI
pub fn send_nmi(dest: IpiDestination) {
    unsafe {
        if is_x2apic_enabled() {
            send_ipi_x2apic(
                dest.apic_id(),
                0, // Vector ignored for NMI
                IpiDeliveryMode::Nmi,
                dest.shorthand(),
            );
        } else {
            send_ipi_xapic(
                dest.apic_id(),
                0,
                IpiDeliveryMode::Nmi,
                dest.shorthand(),
            );
        }
    }
}

/// Send an INIT IPI
pub fn send_init(dest: IpiDestination) {
    unsafe {
        if is_x2apic_enabled() {
            send_ipi_x2apic(
                dest.apic_id(),
                0,
                IpiDeliveryMode::Init,
                dest.shorthand(),
            );
        } else {
            send_ipi_xapic(
                dest.apic_id(),
                0,
                IpiDeliveryMode::Init,
                dest.shorthand(),
            );
        }
    }
}

/// Send a Startup IPI (SIPI)
///
/// The vector specifies the real-mode entry point address divided by 0x1000.
/// For example, vector 0x10 means entry at physical address 0x10000.
pub fn send_sipi(dest: IpiDestination, vector: u8) {
    unsafe {
        if is_x2apic_enabled() {
            send_ipi_x2apic(
                dest.apic_id(),
                vector,
                IpiDeliveryMode::Sipi,
                dest.shorthand(),
            );
        } else {
            send_ipi_xapic(
                dest.apic_id(),
                vector,
                IpiDeliveryMode::Sipi,
                dest.shorthand(),
            );
        }
    }
}

// =============================================================================
// Common IPI Operations
// =============================================================================

/// Send a reschedule IPI to trigger the scheduler on another CPU
#[inline]
pub fn send_reschedule(cpu_apic_id: u32) {
    send_ipi(IpiDestination::Single(cpu_apic_id), RESCHEDULE_VECTOR);
}

/// Broadcast a reschedule IPI to all other CPUs
#[inline]
pub fn broadcast_reschedule() {
    send_ipi(IpiDestination::AllExcludingSelf, RESCHEDULE_VECTOR);
}

/// Send a TLB shootdown IPI
#[inline]
pub fn send_tlb_shootdown(cpu_apic_id: u32) {
    send_ipi(IpiDestination::Single(cpu_apic_id), TLB_VECTOR);
}

/// Broadcast a TLB shootdown IPI to all other CPUs
#[inline]
pub fn broadcast_tlb_shootdown() {
    send_ipi(IpiDestination::AllExcludingSelf, TLB_VECTOR);
}

/// Send a stop/halt IPI
#[inline]
pub fn send_stop(cpu_apic_id: u32) {
    send_ipi(IpiDestination::Single(cpu_apic_id), STOP_VECTOR);
}

/// Broadcast a stop/halt IPI to all other CPUs
#[inline]
pub fn broadcast_stop() {
    send_ipi(IpiDestination::AllExcludingSelf, STOP_VECTOR);
}

/// Send a call function IPI
#[inline]
pub fn send_call(cpu_apic_id: u32) {
    send_ipi(IpiDestination::Single(cpu_apic_id), CALL_VECTOR);
}

/// Broadcast a call function IPI to all other CPUs
#[inline]
pub fn broadcast_call() {
    send_ipi(IpiDestination::AllExcludingSelf, CALL_VECTOR);
}

// =============================================================================
// AP (Application Processor) Startup Sequence
// =============================================================================

/// Standard delay values for INIT-SIPI-SIPI sequence (in microseconds)
pub mod startup_delays {
    /// Delay after INIT IPI
    pub const AFTER_INIT: u64 = 10_000; // 10ms
    /// Delay between SIPIs
    pub const BETWEEN_SIPI: u64 = 200;  // 200us
}

/// Start an Application Processor using the INIT-SIPI-SIPI sequence
///
/// # Arguments
///
/// * `apic_id` - The APIC ID of the target AP
/// * `startup_vector` - The startup vector (entry address / 0x1000)
/// * `delay_fn` - Function to call for delays (takes microseconds)
///
/// # Safety
///
/// The startup vector must point to valid AP startup code.
pub unsafe fn start_ap<F>(apic_id: u32, startup_vector: u8, mut delay_fn: F)
where
    F: FnMut(u64),
{
    let dest = IpiDestination::Single(apic_id);

    // 1. Send INIT IPI
    send_init(dest);

    // 2. Wait 10ms
    delay_fn(startup_delays::AFTER_INIT);

    // 3. Send first SIPI
    send_sipi(dest, startup_vector);

    // 4. Wait 200us
    delay_fn(startup_delays::BETWEEN_SIPI);

    // 5. Send second SIPI (for compatibility)
    send_sipi(dest, startup_vector);
}

/// Broadcast INIT to all APs
pub fn broadcast_init() {
    send_init(IpiDestination::AllExcludingSelf);
}

/// Broadcast SIPI to all APs
pub fn broadcast_sipi(startup_vector: u8) {
    send_sipi(IpiDestination::AllExcludingSelf, startup_vector);
}

// =============================================================================
// IPI Barrier
// =============================================================================

/// Barrier for IPI synchronization
///
/// Used to synchronize all CPUs at a specific point.
pub struct IpiBarrier {
    /// Number of CPUs expected at barrier
    expected: AtomicU32,
    /// Number of CPUs that have arrived
    arrived: AtomicU32,
    /// Generation counter to handle reuse
    generation: AtomicU32,
}

impl IpiBarrier {
    /// Create a new IPI barrier
    pub const fn new() -> Self {
        Self {
            expected: AtomicU32::new(0),
            arrived: AtomicU32::new(0),
            generation: AtomicU32::new(0),
        }
    }

    /// Initialize the barrier for a given number of CPUs
    pub fn init(&self, num_cpus: u32) {
        self.expected.store(num_cpus, Ordering::SeqCst);
        self.arrived.store(0, Ordering::SeqCst);
        self.generation.fetch_add(1, Ordering::SeqCst);
    }

    /// Wait at the barrier
    ///
    /// Returns true for the last CPU to arrive (the "leader")
    pub fn wait(&self) -> bool {
        let gen = self.generation.load(Ordering::Acquire);
        let expected = self.expected.load(Ordering::Acquire);

        // Increment arrived count
        let prev = self.arrived.fetch_add(1, Ordering::AcqRel);
        let is_leader = prev + 1 == expected;

        if is_leader {
            // Leader: reset the barrier for next use
            self.arrived.store(0, Ordering::Release);
            self.generation.fetch_add(1, Ordering::Release);
        } else {
            // Non-leader: spin until generation changes
            while self.generation.load(Ordering::Acquire) == gen {
                core::hint::spin_loop();
            }
        }

        is_leader
    }
}

impl Default for IpiBarrier {
    fn default() -> Self {
        Self::new()
    }
}
