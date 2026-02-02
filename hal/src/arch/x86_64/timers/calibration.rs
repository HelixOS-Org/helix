//! # Timer Calibration
//!
//! This module provides timer calibration routines for accurately
//! determining timer frequencies.
//!
//! ## Calibration Methods
//!
//! 1. **CPUID**: Direct frequency from CPUID (most accurate, if available)
//! 2. **PIT**: Use legacy PIT as reference (always available)
//! 3. **HPET**: Use HPET as reference (if available)
//! 4. **ACPI PM Timer**: Use ACPI PM Timer (if available)
//!
//! ## Calibration Process
//!
//! For TSC calibration:
//! 1. Read reference timer
//! 2. Read TSC
//! 3. Wait for reference to elapse
//! 4. Read TSC again
//! 5. Calculate TSC frequency from elapsed ticks

use super::{hpet, pit, tsc, TimerError};

// =============================================================================
// Calibration Methods
// =============================================================================

/// Calibration method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationMethod {
    /// Use CPUID to get frequency
    Cpuid,
    /// Use PIT as reference
    Pit,
    /// Use HPET as reference
    Hpet,
    /// Use ACPI PM Timer as reference
    AcpiPm,
}

/// Calibration result
#[derive(Debug, Clone, Copy)]
pub struct CalibrationResult {
    /// Calibrated frequency in Hz
    pub frequency: u64,
    /// Calibration method used
    pub method: CalibrationMethod,
    /// Estimated error in parts per million
    pub error_ppm: u32,
}

// =============================================================================
// TSC Calibration
// =============================================================================

/// Calibrate TSC using the best available method
pub fn calibrate_tsc() -> Result<CalibrationResult, TimerError> {
    // Try CPUID first
    if let Some(freq) = tsc::get_frequency_from_cpuid() {
        return Ok(CalibrationResult {
            frequency: freq,
            method: CalibrationMethod::Cpuid,
            error_ppm: 0, // CPUID is exact
        });
    }

    // Try HPET
    if hpet::is_available() {
        if let Ok(result) = calibrate_tsc_with_hpet() {
            return Ok(result);
        }
    }

    // Fall back to PIT
    calibrate_tsc_with_pit()
}

/// Calibrate TSC using PIT as reference
pub fn calibrate_tsc_with_pit() -> Result<CalibrationResult, TimerError> {
    // Calibration parameters
    const CALIBRATION_MS: u64 = 50;
    const PIT_TICKS: u64 = pit::PIT_FREQUENCY * CALIBRATION_MS / 1000;

    unsafe {
        // Initialize PIT if needed
        pit::init();

        // Perform multiple calibrations and average
        let mut total_freq: u64 = 0;
        const ITERATIONS: u64 = 3;

        for _ in 0..ITERATIONS {
            let freq = calibrate_tsc_single_pit(PIT_TICKS)?;
            total_freq += freq;
        }

        let avg_freq = total_freq / ITERATIONS;

        Ok(CalibrationResult {
            frequency: avg_freq,
            method: CalibrationMethod::Pit,
            error_ppm: 100, // PIT has ~100 ppm accuracy
        })
    }
}

/// Single TSC calibration iteration using PIT
unsafe fn calibrate_tsc_single_pit(pit_ticks: u64) -> Result<u64, TimerError> {
    // Set up PIT channel 2 for one-shot
    let ticks_u16 = if pit_ticks > 65535 {
        65535u16
    } else {
        pit_ticks as u16
    };

    // Configure channel 2 in mode 0 (interrupt on terminal count)
    let command =
        ((pit::PitChannel::Channel2 as u8) << 6) | ((pit::PitAccess::LowHigh as u8) << 4) | 0; // Mode 0

    outb(0x43, command);

    // Write count
    outb(0x42, ticks_u16 as u8);
    outb(0x42, (ticks_u16 >> 8) as u8);

    // Enable gate
    let port_b = inb(0x61);
    outb(0x61, (port_b & 0xFC) | 0x01);

    // Read starting TSC (serialized)
    core::arch::asm!("mfence", options(nostack, preserves_flags));
    let start_tsc = tsc::read();

    // Wait for output to go high
    while inb(0x61) & 0x20 == 0 {
        core::hint::spin_loop();
    }

    // Read ending TSC
    let end_tsc = tsc::read();
    core::arch::asm!("mfence", options(nostack, preserves_flags));

    // Disable gate
    outb(0x61, port_b & 0xFC);

    // Calculate TSC frequency
    let tsc_elapsed = end_tsc - start_tsc;

    // TSC frequency = (TSC elapsed * PIT_FREQUENCY) / PIT ticks
    let frequency = (tsc_elapsed as u128 * pit::PIT_FREQUENCY as u128 / ticks_u16 as u128) as u64;

    if frequency == 0 {
        return Err(TimerError::CalibrationFailed);
    }

    Ok(frequency)
}

/// Calibrate TSC using HPET as reference
pub fn calibrate_tsc_with_hpet() -> Result<CalibrationResult, TimerError> {
    if !hpet::is_available() {
        return Err(TimerError::NotAvailable);
    }

    // Calibration for 10ms
    const CALIBRATION_NS: u64 = 10_000_000;
    let hpet_ticks = hpet::ns_to_ticks(CALIBRATION_NS);

    if hpet_ticks == 0 {
        return Err(TimerError::CalibrationFailed);
    }

    // Read starting values
    let start_hpet = hpet::read_counter();
    let start_tsc = tsc::read_serialized();

    // Wait for HPET ticks to elapse
    while hpet::read_counter().wrapping_sub(start_hpet) < hpet_ticks {
        core::hint::spin_loop();
    }

    // Read ending values
    let end_tsc = tsc::read_serialized();
    let end_hpet = hpet::read_counter();

    // Calculate TSC frequency
    let tsc_elapsed = end_tsc - start_tsc;
    let hpet_elapsed = end_hpet.wrapping_sub(start_hpet);

    // Convert HPET elapsed to nanoseconds
    let elapsed_ns = hpet::ticks_to_ns(hpet_elapsed);

    if elapsed_ns == 0 {
        return Err(TimerError::CalibrationFailed);
    }

    // TSC frequency = (TSC elapsed * 1e9) / elapsed_ns
    let frequency = (tsc_elapsed as u128 * 1_000_000_000 / elapsed_ns as u128) as u64;

    Ok(CalibrationResult {
        frequency,
        method: CalibrationMethod::Hpet,
        error_ppm: 10, // HPET is more accurate than PIT
    })
}

// =============================================================================
// APIC Timer Calibration
// =============================================================================

/// Calibrate APIC timer using TSC
pub fn calibrate_apic_with_tsc(tsc_frequency: u64) -> Result<CalibrationResult, TimerError> {
    if tsc_frequency == 0 {
        return Err(TimerError::CalibrationFailed);
    }

    unsafe {
        // Configure APIC timer for maximum count
        const APIC_LVT_TIMER: u64 = 0xFEE0_0320;
        const APIC_TIMER_ICR: u64 = 0xFEE0_0380;
        const APIC_TIMER_CCR: u64 = 0xFEE0_0390;
        const APIC_TIMER_DCR: u64 = 0xFEE0_03E0;

        // Set divide by 1
        core::ptr::write_volatile(APIC_TIMER_DCR as *mut u32, 0b1011);

        // Mask timer, one-shot mode
        core::ptr::write_volatile(APIC_LVT_TIMER as *mut u32, 1 << 16);

        // Set initial count
        core::ptr::write_volatile(APIC_TIMER_ICR as *mut u32, 0xFFFF_FFFF);

        // Measure for 10ms worth of TSC ticks
        let calibration_tsc_ticks = tsc_frequency / 100; // 10ms

        let start_tsc = tsc::read();
        let start_apic = core::ptr::read_volatile(APIC_TIMER_CCR as *const u32);

        // Wait for TSC ticks
        while tsc::read() - start_tsc < calibration_tsc_ticks {
            core::hint::spin_loop();
        }

        let end_apic = core::ptr::read_volatile(APIC_TIMER_CCR as *const u32);
        let end_tsc = tsc::read();

        // Stop timer
        core::ptr::write_volatile(APIC_TIMER_ICR as *mut u32, 0);

        // Calculate APIC frequency
        let apic_elapsed = start_apic.wrapping_sub(end_apic);
        let tsc_elapsed = end_tsc - start_tsc;

        // APIC freq = (APIC elapsed * TSC freq) / TSC elapsed
        let frequency = (apic_elapsed as u128 * tsc_frequency as u128 / tsc_elapsed as u128) as u64;

        Ok(CalibrationResult {
            frequency,
            method: CalibrationMethod::Cpuid, // Actually TSC-based
            error_ppm: 50,
        })
    }
}

/// Calibrate APIC timer using PIT
pub fn calibrate_apic_with_pit() -> Result<CalibrationResult, TimerError> {
    const CALIBRATION_MS: u64 = 10;
    let pit_ticks = pit::PIT_FREQUENCY * CALIBRATION_MS / 1000;
    let pit_ticks_u16 = if pit_ticks > 65535 {
        65535u16
    } else {
        pit_ticks as u16
    };

    unsafe {
        // Configure APIC timer
        const APIC_LVT_TIMER: u64 = 0xFEE0_0320;
        const APIC_TIMER_ICR: u64 = 0xFEE0_0380;
        const APIC_TIMER_CCR: u64 = 0xFEE0_0390;
        const APIC_TIMER_DCR: u64 = 0xFEE0_03E0;

        // Set divide by 1
        core::ptr::write_volatile(APIC_TIMER_DCR as *mut u32, 0b1011);

        // Mask timer, one-shot mode
        core::ptr::write_volatile(APIC_LVT_TIMER as *mut u32, 1 << 16);

        // Set initial count
        core::ptr::write_volatile(APIC_TIMER_ICR as *mut u32, 0xFFFF_FFFF);

        // Use PIT for timing
        let command = (2 << 6) | (3 << 4) | 0; // Channel 2, low/high, mode 0
        outb(0x43, command);
        outb(0x42, pit_ticks_u16 as u8);
        outb(0x42, (pit_ticks_u16 >> 8) as u8);

        // Enable gate
        let port_b = inb(0x61);
        outb(0x61, (port_b & 0xFC) | 0x01);

        let start_apic = core::ptr::read_volatile(APIC_TIMER_CCR as *const u32);

        // Wait for PIT
        while inb(0x61) & 0x20 == 0 {
            core::hint::spin_loop();
        }

        let end_apic = core::ptr::read_volatile(APIC_TIMER_CCR as *const u32);

        // Disable gate
        outb(0x61, port_b & 0xFC);

        // Stop timer
        core::ptr::write_volatile(APIC_TIMER_ICR as *mut u32, 0);

        // Calculate frequency
        let apic_elapsed = start_apic.wrapping_sub(end_apic);
        let frequency =
            (apic_elapsed as u128 * pit::PIT_FREQUENCY as u128 / pit_ticks_u16 as u128) as u64;

        Ok(CalibrationResult {
            frequency,
            method: CalibrationMethod::Pit,
            error_ppm: 100,
        })
    }
}

// =============================================================================
// Port I/O Helpers
// =============================================================================

#[inline]
unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nostack, nomem, preserves_flags),
    );
}

#[inline]
unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    core::arch::asm!(
        "in al, dx",
        in("dx") port,
        out("al") value,
        options(nostack, nomem, preserves_flags),
    );
    value
}

// =============================================================================
// Calibration Verification
// =============================================================================

/// Verify calibration by comparing against another source
pub fn verify_calibration(frequency: u64, method: CalibrationMethod) -> Option<i64> {
    // Try to get a second opinion
    let reference = match method {
        CalibrationMethod::Cpuid => {
            // If we used CPUID, try PIT verification
            calibrate_tsc_with_pit().ok()?.frequency
        },
        CalibrationMethod::Pit => {
            // If we used PIT, try CPUID
            tsc::get_frequency_from_cpuid()?
        },
        CalibrationMethod::Hpet => {
            // Compare against CPUID or PIT
            tsc::get_frequency_from_cpuid()
                .or_else(|| calibrate_tsc_with_pit().ok().map(|r| r.frequency))?
        },
        CalibrationMethod::AcpiPm => tsc::get_frequency_from_cpuid()?,
    };

    // Calculate difference in PPM
    let diff = frequency as i64 - reference as i64;
    let ppm = (diff * 1_000_000) / reference as i64;

    Some(ppm)
}

// =============================================================================
// Quick Delay (Before Full Initialization)
// =============================================================================

/// Quick delay using TSC (for early boot, before calibration)
///
/// Uses an estimated frequency.
pub fn quick_delay_ns(ns: u64) {
    // Assume ~2 GHz if not calibrated
    const ESTIMATED_FREQ: u64 = 2_000_000_000;

    let freq = super::tsc_frequency();
    let freq = if freq > 0 { freq } else { ESTIMATED_FREQ };

    let target_ticks = (ns as u128 * freq as u128 / 1_000_000_000) as u64;
    let start = tsc::read();

    while tsc::read() - start < target_ticks {
        core::hint::spin_loop();
    }
}

/// Quick delay in microseconds
#[inline]
pub fn quick_delay_us(us: u64) {
    quick_delay_ns(us * 1_000);
}

/// Quick delay in milliseconds
#[inline]
pub fn quick_delay_ms(ms: u64) {
    quick_delay_ns(ms * 1_000_000);
}
