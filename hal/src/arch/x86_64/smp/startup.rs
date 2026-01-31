//! # Application Processor Startup
//!
//! This module handles starting Application Processors (APs) using
//! the INIT-SIPI-SIPI protocol.
//!
//! ## Startup Sequence
//!
//! 1. Prepare trampoline code at low memory (< 1MB)
//! 2. For each AP:
//!    a. Send INIT IPI
//!    b. Wait 10ms
//!    c. Send first SIPI
//!    d. Wait 200µs
//!    e. Send second SIPI (if needed)
//!    f. Wait for AP to respond

use core::sync::atomic::{AtomicU32, AtomicU64, Ordering, fence};

use super::{MAX_CPUS, CPU_STACK_SIZE, AP_TRAMPOLINE_ADDR, SmpError};
use super::cpu_info::{CpuState, register_cpu, get_cpu_info};

// =============================================================================
// Trampoline
// =============================================================================

/// Trampoline data structure passed to APs
#[repr(C, align(4096))]
pub struct TrampolineData {
    /// Magic value for validation
    pub magic: u32,
    /// AP entry point (long mode)
    pub entry_point: u64,
    /// Initial CR3 (page table)
    pub cr3: u64,
    /// GDT pointer (limit + base)
    pub gdt_limit: u16,
    pub _pad1: u16,
    pub gdt_base: u64,
    /// IDT pointer (limit + base)
    pub idt_limit: u16,
    pub _pad2: u16,
    pub idt_base: u64,
    /// Stack pointers for each AP (indexed by APIC ID)
    pub stacks: [u64; MAX_CPUS],
    /// AP ready flag
    pub ap_ready: AtomicU32,
    /// Current AP APIC ID being started
    pub current_ap: AtomicU32,
    /// Total APs started
    pub aps_started: AtomicU32,
    /// Reserved for alignment
    pub _reserved: [u8; 16],
}

/// Trampoline magic value
pub const TRAMPOLINE_MAGIC: u32 = 0x5452_414D; // "TRAM"

impl TrampolineData {
    /// Create new trampoline data
    pub const fn new() -> Self {
        Self {
            magic: TRAMPOLINE_MAGIC,
            entry_point: 0,
            cr3: 0,
            gdt_limit: 0,
            _pad1: 0,
            gdt_base: 0,
            idt_limit: 0,
            _pad2: 0,
            idt_base: 0,
            stacks: [0; MAX_CPUS],
            ap_ready: AtomicU32::new(0),
            current_ap: AtomicU32::new(0xFFFF_FFFF),
            aps_started: AtomicU32::new(0),
            _reserved: [0; 16],
        }
    }
}

impl Default for TrampolineData {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// AP Startup State
// =============================================================================

/// AP startup timeout in microseconds
const AP_STARTUP_TIMEOUT_US: u64 = 200_000; // 200ms

/// INIT IPI wait time in microseconds
const INIT_WAIT_US: u64 = 10_000; // 10ms

/// SIPI wait time in microseconds
const SIPI_WAIT_US: u64 = 200; // 200µs

/// Global trampoline data pointer
static TRAMPOLINE_DATA_PTR: AtomicU64 = AtomicU64::new(0);

/// Set trampoline data pointer
pub fn set_trampoline_data(data: &TrampolineData) {
    TRAMPOLINE_DATA_PTR.store(data as *const _ as u64, Ordering::SeqCst);
}

/// Get trampoline data pointer
pub fn get_trampoline_data() -> Option<&'static TrampolineData> {
    let ptr = TRAMPOLINE_DATA_PTR.load(Ordering::SeqCst);
    if ptr != 0 {
        Some(unsafe { &*(ptr as *const TrampolineData) })
    } else {
        None
    }
}

// =============================================================================
// AP Startup
// =============================================================================

/// Start a single AP
///
/// # Arguments
/// * `apic_id` - APIC ID of the AP to start
/// * `cpu_id` - CPU index for this AP
/// * `local_apic` - Local APIC for sending IPIs
///
/// # Returns
/// * `Ok(())` if AP started successfully
/// * `Err(SmpError)` if startup failed
pub fn start_ap(
    apic_id: u32,
    cpu_id: usize,
) -> Result<(), SmpError> {
    // Register CPU if not already done
    if get_cpu_info(cpu_id).is_none() {
        register_cpu(cpu_id, apic_id, false)?;
    }

    // Get trampoline data
    let trampoline = get_trampoline_data()
        .ok_or(SmpError::TrampolineNotReady)?;

    // Set current AP being started
    trampoline.current_ap.store(apic_id, Ordering::SeqCst);
    fence(Ordering::SeqCst);

    // Update CPU state
    if let Some(info) = get_cpu_info(cpu_id) {
        info.set_state(CpuState::Starting);
    }

    // Calculate SIPI vector (entry address / 4K)
    let sipi_vector = (AP_TRAMPOLINE_ADDR / 0x1000) as u8;

    // Send INIT IPI
    send_init_ipi(apic_id)?;

    // Wait 10ms
    delay_us(INIT_WAIT_US);

    // Send first SIPI
    send_sipi(apic_id, sipi_vector)?;

    // Wait 200µs
    delay_us(SIPI_WAIT_US);

    // Check if AP responded
    if !wait_for_ap_ready(apic_id, 1000) {
        // Send second SIPI
        send_sipi(apic_id, sipi_vector)?;

        // Wait for AP with timeout
        if !wait_for_ap_ready(apic_id, AP_STARTUP_TIMEOUT_US) {
            if let Some(info) = get_cpu_info(cpu_id) {
                info.set_state(CpuState::Error);
            }
            return Err(SmpError::ApStartupTimeout);
        }
    }

    // AP is now running
    if let Some(info) = get_cpu_info(cpu_id) {
        info.set_state(CpuState::Online);
    }

    trampoline.aps_started.fetch_add(1, Ordering::SeqCst);

    log::debug!("AP {} (CPU {}) started successfully", apic_id, cpu_id);

    Ok(())
}

/// Wait for AP to signal ready
fn wait_for_ap_ready(apic_id: u32, timeout_us: u64) -> bool {
    let trampoline = match get_trampoline_data() {
        Some(t) => t,
        None => return false,
    };

    let start = read_tsc();
    let timeout_ticks = us_to_tsc_ticks(timeout_us);

    loop {
        fence(Ordering::SeqCst);

        // Check if this AP signaled ready
        let ready_apic = trampoline.ap_ready.load(Ordering::SeqCst);
        if ready_apic == apic_id {
            // Clear the ready flag
            trampoline.ap_ready.store(0, Ordering::SeqCst);
            return true;
        }

        // Check timeout
        let elapsed = read_tsc().wrapping_sub(start);
        if elapsed > timeout_ticks {
            return false;
        }

        // Brief pause
        core::hint::spin_loop();
    }
}

/// Start all APs in the system
pub fn start_all_aps(ap_list: &[(u32, usize)]) -> Result<usize, SmpError> {
    let mut started = 0usize;

    for &(apic_id, cpu_id) in ap_list {
        match start_ap(apic_id, cpu_id) {
            Ok(()) => {
                started += 1;
            }
            Err(e) => {
                log::warn!("Failed to start AP {} (CPU {}): {:?}", apic_id, cpu_id, e);
            }
        }
    }

    log::info!("Started {}/{} APs", started, ap_list.len());

    Ok(started)
}

// =============================================================================
// IPI Sending
// =============================================================================

/// Send INIT IPI to specified CPU
fn send_init_ipi(apic_id: u32) -> Result<(), SmpError> {
    // Write to ICR to send INIT
    unsafe {
        write_icr(apic_id as u64, 0x00004500); // INIT, level assert
    }

    // Wait for delivery
    wait_for_ipi_delivery()?;

    // De-assert
    unsafe {
        write_icr(apic_id as u64, 0x00008500); // INIT, level de-assert
    }

    wait_for_ipi_delivery()?;

    Ok(())
}

/// Send SIPI to specified CPU
fn send_sipi(apic_id: u32, vector: u8) -> Result<(), SmpError> {
    // SIPI: delivery mode 110 (Startup), vector = page number
    let icr_low = 0x00004600 | (vector as u32);

    unsafe {
        write_icr(apic_id as u64, icr_low);
    }

    wait_for_ipi_delivery()?;

    Ok(())
}

/// Wait for IPI delivery to complete
fn wait_for_ipi_delivery() -> Result<(), SmpError> {
    let start = read_tsc();
    let timeout = us_to_tsc_ticks(1000); // 1ms timeout

    loop {
        // Check delivery status in ICR
        let status = unsafe { read_icr() };

        // Bit 12 is delivery status (0 = idle, 1 = pending)
        if (status & (1 << 12)) == 0 {
            return Ok(());
        }

        let elapsed = read_tsc().wrapping_sub(start);
        if elapsed > timeout {
            return Err(SmpError::IpiDeliveryFailed);
        }

        core::hint::spin_loop();
    }
}

// =============================================================================
// Low-level APIC Access
// =============================================================================

/// Default Local APIC base address
const LAPIC_BASE: u64 = 0xFEE0_0000;

/// ICR Low register offset
const ICR_LOW_OFFSET: u64 = 0x300;

/// ICR High register offset
const ICR_HIGH_OFFSET: u64 = 0x310;

/// Write to ICR registers
unsafe fn write_icr(dest_apic_id: u64, icr_low: u32) {
    let icr_high = (dest_apic_id as u32) << 24;

    // Write high first (destination), then low (triggers send)
    let high_ptr = (LAPIC_BASE + ICR_HIGH_OFFSET) as *mut u32;
    let low_ptr = (LAPIC_BASE + ICR_LOW_OFFSET) as *mut u32;

    core::ptr::write_volatile(high_ptr, icr_high);
    core::ptr::write_volatile(low_ptr, icr_low);
}

/// Read ICR low register
unsafe fn read_icr() -> u32 {
    let low_ptr = (LAPIC_BASE + ICR_LOW_OFFSET) as *const u32;
    core::ptr::read_volatile(low_ptr)
}

// =============================================================================
// Timing Helpers
// =============================================================================

/// Read TSC
#[inline]
fn read_tsc() -> u64 {
    let (lo, hi): (u32, u32);
    unsafe {
        core::arch::asm!(
            "rdtsc",
            out("eax") lo,
            out("edx") hi,
            options(nostack, preserves_flags),
        );
    }
    ((hi as u64) << 32) | (lo as u64)
}

/// TSC frequency (must be calibrated)
static TSC_FREQUENCY_HZ: AtomicU64 = AtomicU64::new(2_000_000_000); // Default 2GHz

/// Set TSC frequency for timing
pub fn set_tsc_frequency(freq_hz: u64) {
    TSC_FREQUENCY_HZ.store(freq_hz, Ordering::SeqCst);
}

/// Convert microseconds to TSC ticks
fn us_to_tsc_ticks(us: u64) -> u64 {
    let freq = TSC_FREQUENCY_HZ.load(Ordering::Relaxed);
    (us * freq) / 1_000_000
}

/// Delay for specified microseconds
fn delay_us(us: u64) {
    let ticks = us_to_tsc_ticks(us);
    let start = read_tsc();

    while read_tsc().wrapping_sub(start) < ticks {
        core::hint::spin_loop();
    }
}

// =============================================================================
// AP Entry Point
// =============================================================================

/// AP entry point type
pub type ApEntryFn = extern "C" fn(cpu_id: u32, apic_id: u32) -> !;

/// Default AP entry point (placeholder)
#[no_mangle]
pub extern "C" fn ap_entry_default(cpu_id: u32, apic_id: u32) -> ! {
    // Signal that this AP is ready
    if let Some(trampoline) = get_trampoline_data() {
        trampoline.ap_ready.store(apic_id, Ordering::SeqCst);
    }

    log::info!("AP {} (APIC {}) entered idle loop", cpu_id, apic_id);

    // Idle loop
    loop {
        unsafe {
            core::arch::asm!("hlt", options(nostack, nomem));
        }
    }
}

// =============================================================================
// Stack Allocation
// =============================================================================

/// Allocate stack for an AP
pub fn allocate_ap_stack(cpu_id: usize) -> Result<u64, SmpError> {
    if cpu_id >= MAX_CPUS {
        return Err(SmpError::InvalidCpuId);
    }

    // In a real implementation, this would allocate from the heap
    // For now, return a placeholder
    // The actual stack allocation must be done by the memory manager

    let trampoline = get_trampoline_data()
        .ok_or(SmpError::TrampolineNotReady)?;

    // Check if stack already allocated
    let existing = trampoline.stacks[cpu_id];
    if existing != 0 {
        return Ok(existing);
    }

    Err(SmpError::StackAllocationFailed)
}

/// Set AP stack pointer
pub fn set_ap_stack(cpu_id: usize, stack_top: u64) -> Result<(), SmpError> {
    if cpu_id >= MAX_CPUS {
        return Err(SmpError::InvalidCpuId);
    }

    // In a real implementation, we'd update trampoline data
    // This is a placeholder - actual implementation depends on memory allocation

    log::debug!("Set stack for CPU {}: {:#x}", cpu_id, stack_top);

    Ok(())
}
