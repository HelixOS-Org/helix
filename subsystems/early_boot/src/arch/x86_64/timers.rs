//! # x86_64 Timer Initialization
//!
//! TSC, HPET, APIC Timer, and PIT timer support.

use core::sync::atomic::{AtomicU64, Ordering};

use super::*;
use crate::core::{BootContext, TimerType};
use crate::error::{BootError, BootResult};

// =============================================================================
// TSC (Time Stamp Counter)
// =============================================================================

/// TSC frequency in Hz
static TSC_FREQUENCY: AtomicU64 = AtomicU64::new(0);

/// TSC features
pub struct TscFeatures {
    /// TSC is available
    pub available: bool,
    /// TSC is invariant (constant rate)
    pub invariant: bool,
    /// TSC deadline mode supported
    pub deadline_mode: bool,
    /// TSC adjust MSR available
    pub adjust_msr: bool,
}

/// Detect TSC features
///
/// # Safety
///
/// The caller must ensure the firmware is accessible.
pub unsafe fn detect_tsc_features() -> TscFeatures {
    let (_, _, _, edx) = cpuid(1, 0);
    let available = (edx & (1 << 4)) != 0;

    let (_, _, ecx, _) = cpuid(1, 0);
    let deadline_mode = (ecx & (1 << 24)) != 0;

    // Check invariant TSC
    let (eax_max, _, _, _) = cpuid(0x80000000, 0);
    let invariant = if eax_max >= 0x80000007 {
        let (_, _, _, edx) = cpuid(0x80000007, 0);
        (edx & (1 << 8)) != 0
    } else {
        false
    };

    // Check TSC_ADJUST MSR
    let (_, _, _, edx) = cpuid(7, 0);
    let adjust_msr = (edx & (1 << 1)) != 0;

    TscFeatures {
        available,
        invariant,
        deadline_mode,
        adjust_msr,
    }
}

/// Get TSC frequency directly from CPUID (Intel)
///
/// # Safety
///
/// The caller must ensure the hardware is properly initialized before reading.
pub unsafe fn get_tsc_frequency_cpuid() -> Option<u64> {
    let (eax_max, _, _, _) = cpuid(0, 0);

    if eax_max >= 0x15 {
        // TSC/Crystal ratio (CPUID.15H)
        let (eax, ebx, ecx, _) = cpuid(0x15, 0);

        if eax != 0 && ebx != 0 {
            let crystal_freq = if ecx != 0 {
                ecx as u64
            } else {
                // Assume common crystal frequencies
                // Intel: 24MHz for most, 25MHz for some
                24_000_000
            };

            return Some((crystal_freq * ebx as u64) / eax as u64);
        }
    }

    if eax_max >= 0x16 {
        // Processor frequency info (CPUID.16H)
        let (eax, _, _, _) = cpuid(0x16, 0);
        if eax != 0 {
            return Some((eax as u64) * 1_000_000);
        }
    }

    None
}

/// Calibrate TSC using PIT
///
/// # Safety
///
/// The caller must ensure the timer hardware is accessible and not in use.
pub unsafe fn calibrate_tsc_pit() -> u64 {
    const PIT_FREQUENCY: u64 = 1193182;
    const CALIBRATION_MS: u64 = 50;

    // Configure PIT channel 2 for timing
    const PIT_CH2_DATA: u16 = 0x42;
    const PIT_CMD: u16 = 0x43;
    const PIT_CH2_GATE: u16 = 0x61;

    let pit_count = ((PIT_FREQUENCY * CALIBRATION_MS) / 1000) as u16;

    // Disable gate, set up counter
    let gate = inb(PIT_CH2_GATE);
    outb(PIT_CH2_GATE, gate & 0xFC);
    outb(PIT_CMD, 0xB0); // Channel 2, mode 0, binary
    outb(PIT_CH2_DATA, (pit_count & 0xFF) as u8);
    outb(PIT_CH2_DATA, (pit_count >> 8) as u8);

    // Get start TSC
    let start_tsc = rdtsc();

    // Enable gate
    outb(PIT_CH2_GATE, (gate & 0xFC) | 1);

    // Wait for counter
    while inb(PIT_CH2_GATE) & 0x20 == 0 {
        core::hint::spin_loop();
    }

    // Get end TSC
    let end_tsc = rdtsc();

    // Disable gate
    outb(PIT_CH2_GATE, gate);

    // Calculate frequency
    ((end_tsc - start_tsc) * 1000) / CALIBRATION_MS
}

/// Calibrate TSC using HPET if available
///
/// # Safety
///
/// The caller must ensure the timer hardware is accessible and not in use.
pub unsafe fn calibrate_tsc_hpet() -> Option<u64> {
    let hpet_freq = get_hpet_frequency();
    if hpet_freq == 0 {
        return None;
    }

    const CALIBRATION_TICKS: u64 = 100_000; // ~100ms at 1MHz

    let start_hpet = read_hpet_counter();
    let start_tsc = rdtsc();

    // Wait for HPET ticks
    while read_hpet_counter() - start_hpet < CALIBRATION_TICKS {
        core::hint::spin_loop();
    }

    let end_hpet = read_hpet_counter();
    let end_tsc = rdtsc();

    let hpet_elapsed = end_hpet - start_hpet;
    let tsc_elapsed = end_tsc - start_tsc;

    Some((tsc_elapsed * hpet_freq) / hpet_elapsed)
}

/// Initialize TSC
///
/// # Safety
///
/// The caller must ensure system is in a valid state for initialization.
pub unsafe fn init_tsc(ctx: &mut BootContext) -> BootResult<()> {
    let features = detect_tsc_features();

    if !features.available {
        return Err(BootError::HardwareNotSupported);
    }

    // Try to get frequency from CPUID first
    let frequency = get_tsc_frequency_cpuid()
        .or_else(|| calibrate_tsc_hpet())
        .unwrap_or_else(|| calibrate_tsc_pit());

    TSC_FREQUENCY.store(frequency, Ordering::SeqCst);

    ctx.timer_state.tsc_frequency = frequency;
    ctx.timer_state.timer_types |= TimerType::TSC.bits();

    if features.invariant {
        ctx.timer_state.timer_types |= TimerType::INVARIANT_TSC.bits();
    }

    if features.deadline_mode {
        ctx.timer_state.timer_types |= TimerType::TSC_DEADLINE.bits();
    }

    Ok(())
}

/// Get current TSC frequency
pub fn get_tsc_frequency() -> u64 {
    TSC_FREQUENCY.load(Ordering::SeqCst)
}

/// Convert TSC ticks to nanoseconds
pub fn tsc_to_ns(ticks: u64) -> u64 {
    let freq = get_tsc_frequency();
    if freq == 0 {
        return 0;
    }
    (ticks * 1_000_000_000) / freq
}

/// Convert nanoseconds to TSC ticks
pub fn ns_to_tsc(ns: u64) -> u64 {
    let freq = get_tsc_frequency();
    (ns * freq) / 1_000_000_000
}

// =============================================================================
// HPET (High Precision Event Timer)
// =============================================================================

/// HPET base address (from ACPI)
static HPET_BASE: AtomicU64 = AtomicU64::new(0);

/// HPET frequency
static HPET_FREQUENCY: AtomicU64 = AtomicU64::new(0);

/// HPET register offsets
const HPET_CAP: u64 = 0x00;
const HPET_CFG: u64 = 0x10;
const HPET_STATUS: u64 = 0x20;
const HPET_COUNTER: u64 = 0xF0;
const HPET_TIMER_CFG: u64 = 0x100;
const HPET_TIMER_CMP: u64 = 0x108;

/// HPET capabilities
pub struct HpetCapabilities {
    /// Period in femtoseconds
    pub period_fs: u32,
    /// Number of timers
    pub num_timers: u8,
    /// 64-bit counter support
    pub counter_64bit: bool,
    /// Legacy replacement capable
    pub legacy_capable: bool,
}

/// Read HPET register
unsafe fn hpet_read(offset: u64) -> u64 {
    let base = HPET_BASE.load(Ordering::SeqCst);
    if base == 0 {
        return 0;
    }
    core::ptr::read_volatile((base + offset) as *const u64)
}

/// Write HPET register
unsafe fn hpet_write(offset: u64, value: u64) {
    let base = HPET_BASE.load(Ordering::SeqCst);
    if base == 0 {
        return;
    }
    core::ptr::write_volatile((base + offset) as *mut u64, value);
}

/// Read HPET counter
///
/// # Safety
///
/// The caller must ensure the hardware is properly initialized before reading.
pub unsafe fn read_hpet_counter() -> u64 {
    hpet_read(HPET_COUNTER)
}

/// Get HPET frequency
pub fn get_hpet_frequency() -> u64 {
    HPET_FREQUENCY.load(Ordering::SeqCst)
}

/// Find HPET from ACPI tables
fn find_hpet_base(_ctx: &BootContext) -> Option<u64> {
    // TODO: Parse ACPI HPET table
    // Common default address
    Some(0xFED00000)
}

/// Initialize HPET
///
/// # Safety
///
/// The caller must ensure system is in a valid state for initialization.
pub unsafe fn init_hpet(ctx: &mut BootContext) -> BootResult<()> {
    let base = match find_hpet_base(ctx) {
        Some(b) => b,
        None => return Err(BootError::HardwareNotSupported),
    };

    HPET_BASE.store(base, Ordering::SeqCst);

    // Read capabilities
    let cap = hpet_read(HPET_CAP);
    let period_fs = (cap >> 32) as u32;

    if period_fs == 0 {
        return Err(BootError::HardwareNotSupported);
    }

    // Calculate frequency (10^15 fs = 1 second)
    let frequency = 1_000_000_000_000_000u64 / period_fs as u64;
    HPET_FREQUENCY.store(frequency, Ordering::SeqCst);

    let num_timers = ((cap >> 8) & 0x1F) as u8 + 1;
    let counter_64bit = (cap & (1 << 13)) != 0;
    let legacy_capable = (cap & (1 << 15)) != 0;

    // Stop HPET
    let cfg = hpet_read(HPET_CFG);
    hpet_write(HPET_CFG, cfg & !1);

    // Reset counter
    hpet_write(HPET_COUNTER, 0);

    // Start HPET
    hpet_write(HPET_CFG, 1);

    // Store in context
    ctx.arch_data.x86.hpet_base = base;
    ctx.timer_state.timer_types |= TimerType::HPET.bits();

    Ok(())
}

/// Set up HPET timer for one-shot interrupt
///
/// # Safety
///
/// The caller must ensure the value is valid for the current system state.
pub unsafe fn setup_hpet_oneshot(timer: u8, ns: u64, vector: u8) {
    let frequency = get_hpet_frequency();
    if frequency == 0 {
        return;
    }

    let ticks = (ns * frequency) / 1_000_000_000;
    let timer_offset = HPET_TIMER_CFG + (timer as u64 * 0x20);

    // Configure timer
    let cfg: u64 = (1 << 2) // Enable interrupt
        | ((vector as u64) << 9); // Route to interrupt

    hpet_write(timer_offset, cfg);
    hpet_write(timer_offset + 8, read_hpet_counter() + ticks);
}

// =============================================================================
// PIT (Programmable Interval Timer)
// =============================================================================

/// PIT ports
const PIT_CH0_DATA: u16 = 0x40;
const PIT_CH1_DATA: u16 = 0x41;
const PIT_CH2_DATA: u16 = 0x42;
const PIT_CMD: u16 = 0x43;

/// PIT base frequency
const PIT_FREQUENCY: u64 = 1193182;

/// Initialize PIT for periodic timer
///
/// # Safety
///
/// The caller must ensure system is in a valid state for initialization.
pub unsafe fn init_pit_periodic(frequency_hz: u64) {
    let divisor = PIT_FREQUENCY / frequency_hz;

    // Channel 0, square wave mode
    outb(PIT_CMD, 0x36);
    outb(PIT_CH0_DATA, (divisor & 0xFF) as u8);
    outb(PIT_CH0_DATA, (divisor >> 8) as u8);
}

/// Disable PIT
///
/// # Safety
///
/// The caller must ensure disabling this feature won't cause system instability.
pub unsafe fn disable_pit() {
    // Set to one-shot mode with count 0
    outb(PIT_CMD, 0x30);
    outb(PIT_CH0_DATA, 0);
    outb(PIT_CH0_DATA, 0);
}

/// Busy-wait using PIT
///
/// # Safety
///
/// The caller must ensure the timer is properly calibrated.
pub unsafe fn pit_delay_ms(ms: u64) {
    let count = ((PIT_FREQUENCY * ms) / 1000) as u16;

    // Use channel 2 for delay
    let gate = inb(0x61);
    outb(0x61, gate & 0xFC);

    outb(PIT_CMD, 0xB0);
    outb(PIT_CH2_DATA, (count & 0xFF) as u8);
    outb(PIT_CH2_DATA, (count >> 8) as u8);

    outb(0x61, (gate & 0xFC) | 1);

    while inb(0x61) & 0x20 == 0 {
        core::hint::spin_loop();
    }

    outb(0x61, gate);
}

// =============================================================================
// TIMER INITIALIZATION
// =============================================================================

/// Initialize all timers
///
/// # Safety
///
/// The caller must ensure timer hardware is accessible.
pub unsafe fn init_timers(ctx: &mut BootContext) -> BootResult<()> {
    // Disable legacy PIT
    disable_pit();

    // Initialize TSC
    if let Err(e) = init_tsc(ctx) {
        // TSC is critical, fail if not available
        return Err(e);
    }

    // Try to initialize HPET
    let _ = init_hpet(ctx);

    // Calibrate APIC timer
    let apic_freq = super::apic::calibrate_apic_timer();
    ctx.timer_state.apic_timer_frequency = apic_freq;

    if apic_freq > 0 {
        ctx.timer_state.timer_types |= TimerType::APIC_TIMER.bits();
    }

    // Set up periodic timer (1000 Hz)
    super::apic::setup_apic_timer(1000, 0xFE);

    ctx.timer_state.ticks_per_second = 1000;

    Ok(())
}

// =============================================================================
// DELAY FUNCTIONS
// =============================================================================

/// Busy-wait for microseconds using TSC
///
/// # Safety
///
/// The caller must ensure the timer is properly calibrated.
pub unsafe fn delay_us(us: u64) {
    let freq = get_tsc_frequency();
    if freq == 0 {
        // Fallback to PIT
        pit_delay_ms((us + 999) / 1000);
        return;
    }

    let ticks = (us * freq) / 1_000_000;
    let start = rdtsc();

    while rdtsc() - start < ticks {
        core::hint::spin_loop();
    }
}

/// Busy-wait for milliseconds
///
/// # Safety
///
/// The caller must ensure the timer is properly calibrated.
pub unsafe fn delay_ms(ms: u64) {
    delay_us(ms * 1000);
}

/// Busy-wait for nanoseconds
///
/// # Safety
///
/// The caller must ensure the timer is properly calibrated.
pub unsafe fn delay_ns(ns: u64) {
    let freq = get_tsc_frequency();
    if freq == 0 {
        return;
    }

    let ticks = (ns * freq) / 1_000_000_000;
    if ticks == 0 {
        return;
    }

    let start = rdtsc();
    while rdtsc() - start < ticks {
        core::hint::spin_loop();
    }
}

// =============================================================================
// TIMESTAMP FUNCTIONS
// =============================================================================

/// Get current time in nanoseconds since boot
pub fn get_time_ns() -> u64 {
    let ticks = unsafe { rdtsc() };
    tsc_to_ns(ticks)
}

/// Get current time in microseconds since boot
pub fn get_time_us() -> u64 {
    get_time_ns() / 1000
}

/// Get current time in milliseconds since boot
pub fn get_time_ms() -> u64 {
    get_time_ns() / 1_000_000
}

// =============================================================================
// TSC DEADLINE MODE
// =============================================================================

/// Set TSC deadline for next interrupt
///
/// # Safety
///
/// The caller must ensure timer hardware is properly initialized.
pub unsafe fn set_tsc_deadline(deadline: u64) {
    wrmsr(MSR_TSC_DEADLINE, deadline);
}

/// Enable TSC deadline mode for APIC timer
///
/// # Safety
///
/// The caller must ensure the system is ready for this feature to be enabled.
pub unsafe fn enable_tsc_deadline_mode(vector: u8) {
    // LVT Timer: TSC-deadline mode
    let lvt = (2 << 17) | (vector as u32);
    super::apic::lapic_write(super::apic::LAPIC_LVT_TIMER, lvt);
}

/// Set TSC deadline for interrupt in nanoseconds from now
///
/// # Safety
///
/// The caller must ensure timer hardware is properly initialized.
pub unsafe fn set_deadline_ns(ns: u64) {
    let current = rdtsc();
    let ticks = ns_to_tsc(ns);
    set_tsc_deadline(current + ticks);
}
