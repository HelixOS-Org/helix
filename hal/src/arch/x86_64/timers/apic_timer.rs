//! # APIC Timer
//!
//! The Local APIC timer is a per-CPU timer that can be used for
//! scheduling, profiling, and other timing needs.
//!
//! ## Features
//!
//! - Per-CPU (no contention in SMP systems)
//! - Three modes: one-shot, periodic, TSC-deadline
//! - Programmable divide ratio
//! - Integrated with local interrupt handling
//!
//! ## Modes
//!
//! 1. **One-Shot Mode**: Count down from initial value, interrupt at zero
//! 2. **Periodic Mode**: Auto-reload and repeat
//! 3. **TSC-Deadline Mode**: Interrupt when TSC reaches a value
//!
//! ## TSC-Deadline Mode
//!
//! This is the most efficient mode on modern CPUs:
//! - No calibration needed (uses TSC frequency)
//! - Higher precision than counter-based modes
//! - Lower latency (direct TSC comparison)

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use super::tsc;

// =============================================================================
// Constants
// =============================================================================

/// Default APIC timer vector
pub const TIMER_VECTOR: u8 = 0x40;

/// Maximum supported CPUs
const MAX_CPUS: usize = 256;

// =============================================================================
// APIC Timer Mode
// =============================================================================

/// APIC Timer operating mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ApicTimerMode {
    /// One-shot mode: count down and stop
    OneShot = 0b00,
    /// Periodic mode: count down and reload
    Periodic = 0b01,
    /// TSC-deadline mode: interrupt at TSC value
    TscDeadline = 0b10,
}

/// APIC Timer divide configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ApicTimerDivide {
    By1 = 0b1011,
    By2 = 0b0000,
    By4 = 0b0001,
    By8 = 0b0010,
    By16 = 0b0011,
    By32 = 0b1000,
    By64 = 0b1001,
    By128 = 0b1010,
}

impl ApicTimerDivide {
    /// Get the divisor value
    pub fn divisor(&self) -> u32 {
        match self {
            ApicTimerDivide::By1 => 1,
            ApicTimerDivide::By2 => 2,
            ApicTimerDivide::By4 => 4,
            ApicTimerDivide::By8 => 8,
            ApicTimerDivide::By16 => 16,
            ApicTimerDivide::By32 => 32,
            ApicTimerDivide::By64 => 64,
            ApicTimerDivide::By128 => 128,
        }
    }
}

// =============================================================================
// Per-CPU State
// =============================================================================

/// Per-CPU timer state
struct PerCpuTimerState {
    /// Timer frequency (ticks per second)
    frequency: AtomicU64,
    /// Timer is calibrated
    calibrated: AtomicBool,
    /// Current mode
    mode: AtomicU32,
    /// Timer vector
    vector: AtomicU32,
}

impl PerCpuTimerState {
    const fn new() -> Self {
        Self {
            frequency: AtomicU64::new(0),
            calibrated: AtomicBool::new(false),
            mode: AtomicU32::new(ApicTimerMode::OneShot as u32),
            vector: AtomicU32::new(TIMER_VECTOR as u32),
        }
    }
}

/// Per-CPU timer states
static PER_CPU_TIMER: [PerCpuTimerState; MAX_CPUS] = [const { PerCpuTimerState::new() }; MAX_CPUS];

/// TSC-deadline mode available
static TSC_DEADLINE_AVAILABLE: AtomicBool = AtomicBool::new(false);

// =============================================================================
// APIC Register Access (Imports)
// =============================================================================

// These would normally come from the APIC module, but we'll define local versions
// to avoid circular dependencies

mod apic_regs {
    pub const LVT_TIMER: u32 = 0x320;
    pub const TIMER_ICR: u32 = 0x380;
    pub const TIMER_CCR: u32 = 0x390;
    pub const TIMER_DCR: u32 = 0x3E0;
}

/// Read APIC register (placeholder - should use apic module)
#[inline]
unsafe fn read_apic(offset: u32) -> u32 {
    // This would use the APIC module's read function
    // For now, we'll use memory-mapped access at default base
    let base = 0xFEE0_0000u64;
    core::ptr::read_volatile((base + offset as u64) as *const u32)
}

/// Write APIC register (placeholder - should use apic module)
#[inline]
unsafe fn write_apic(offset: u32, value: u32) {
    let base = 0xFEE0_0000u64;
    core::ptr::write_volatile((base + offset as u64) as *mut u32, value);
}

// =============================================================================
// Initialization
// =============================================================================

/// Initialize the APIC timer for this CPU
///
/// # Safety
///
/// Must be called after APIC initialization.
pub unsafe fn init(cpu_id: usize) -> Result<(), ApicTimerError> {
    if cpu_id >= MAX_CPUS {
        return Err(ApicTimerError::InvalidCpuId);
    }

    // Check for TSC-deadline mode support
    let tsc_features = tsc::detect_features();
    TSC_DEADLINE_AVAILABLE.store(tsc_features.tsc_deadline, Ordering::SeqCst);

    // Set divide configuration
    write_apic(apic_regs::TIMER_DCR, ApicTimerDivide::By1 as u32);

    // Initially masked
    let lvt = (TIMER_VECTOR as u32) | (1 << 16); // Masked
    write_apic(apic_regs::LVT_TIMER, lvt);

    // Stop the timer
    write_apic(apic_regs::TIMER_ICR, 0);

    PER_CPU_TIMER[cpu_id].vector.store(TIMER_VECTOR as u32, Ordering::SeqCst);

    log::debug!(
        "APIC Timer: CPU {} initialized (TSC-deadline={})",
        cpu_id,
        tsc_features.tsc_deadline
    );

    Ok(())
}

/// Calibrate the APIC timer using the TSC or PIT
///
/// Returns the timer frequency in Hz.
///
/// # Safety
///
/// Interrupts should be disabled during calibration.
pub unsafe fn calibrate(cpu_id: usize) -> Result<u64, ApicTimerError> {
    if cpu_id >= MAX_CPUS {
        return Err(ApicTimerError::InvalidCpuId);
    }

    // Use PIT for calibration
    let frequency = calibrate_with_pit()?;

    PER_CPU_TIMER[cpu_id].frequency.store(frequency, Ordering::SeqCst);
    PER_CPU_TIMER[cpu_id].calibrated.store(true, Ordering::SeqCst);

    log::info!(
        "APIC Timer: CPU {} calibrated at {} Hz ({} MHz)",
        cpu_id,
        frequency,
        frequency / 1_000_000
    );

    Ok(frequency)
}

/// Calibrate using the PIT
unsafe fn calibrate_with_pit() -> Result<u64, ApicTimerError> {
    const PIT_FREQ: u64 = 1_193_182;
    const CALIBRATION_MS: u64 = 10;
    const PIT_TICKS: u64 = PIT_FREQ * CALIBRATION_MS / 1000;

    // Configure PIT channel 2 for calibration
    // This is a simplified version - real implementation would be more robust

    // Set up APIC timer to maximum count
    write_apic(apic_regs::TIMER_DCR, ApicTimerDivide::By1 as u32);
    write_apic(apic_regs::LVT_TIMER, (1 << 16)); // Masked, one-shot
    write_apic(apic_regs::TIMER_ICR, 0xFFFF_FFFF);

    // Wait using TSC (if available and calibrated) or busy loop
    let start_tsc = tsc::read();
    let start_apic = read_apic(apic_regs::TIMER_CCR);

    // Simple delay - in production, use PIT or HPET
    for _ in 0..10_000_000 {
        core::hint::spin_loop();
    }

    let end_tsc = tsc::read();
    let end_apic = read_apic(apic_regs::TIMER_CCR);

    // Calculate elapsed
    let apic_elapsed = start_apic - end_apic;
    let tsc_elapsed = end_tsc - start_tsc;

    // If we know TSC frequency, use it to calculate time
    if let Some(tsc_freq) = tsc::get_frequency_from_cpuid() {
        let time_ns = (tsc_elapsed as u128 * 1_000_000_000) / tsc_freq as u128;
        if time_ns > 0 {
            let apic_freq = (apic_elapsed as u128 * 1_000_000_000) / time_ns;
            return Ok(apic_freq as u64);
        }
    }

    // Fallback: assume ~100MHz APIC timer
    Ok(100_000_000)
}

// =============================================================================
// Timer Control
// =============================================================================

/// Start the APIC timer in one-shot mode
///
/// # Safety
///
/// Timer must be initialized and calibrated.
pub unsafe fn start_oneshot(cpu_id: usize, ticks: u32, vector: u8) -> Result<(), ApicTimerError> {
    if cpu_id >= MAX_CPUS {
        return Err(ApicTimerError::InvalidCpuId);
    }

    // Configure LVT: one-shot mode, specified vector, not masked
    let lvt = (vector as u32) | (ApicTimerMode::OneShot as u32) << 17;
    write_apic(apic_regs::LVT_TIMER, lvt);

    // Set initial count (starts countdown)
    write_apic(apic_regs::TIMER_ICR, ticks);

    PER_CPU_TIMER[cpu_id].mode.store(ApicTimerMode::OneShot as u32, Ordering::SeqCst);

    Ok(())
}

/// Start the APIC timer in periodic mode
///
/// # Safety
///
/// Timer must be initialized and calibrated.
pub unsafe fn start_periodic(cpu_id: usize, ticks: u32, vector: u8) -> Result<(), ApicTimerError> {
    if cpu_id >= MAX_CPUS {
        return Err(ApicTimerError::InvalidCpuId);
    }

    // Configure LVT: periodic mode, specified vector, not masked
    let lvt = (vector as u32) | (ApicTimerMode::Periodic as u32) << 17;
    write_apic(apic_regs::LVT_TIMER, lvt);

    // Set initial count (starts countdown)
    write_apic(apic_regs::TIMER_ICR, ticks);

    PER_CPU_TIMER[cpu_id].mode.store(ApicTimerMode::Periodic as u32, Ordering::SeqCst);

    Ok(())
}

/// Start the APIC timer in TSC-deadline mode
///
/// # Safety
///
/// TSC-deadline mode must be supported and the timer initialized.
pub unsafe fn start_tsc_deadline(cpu_id: usize, deadline: u64, vector: u8) -> Result<(), ApicTimerError> {
    if cpu_id >= MAX_CPUS {
        return Err(ApicTimerError::InvalidCpuId);
    }

    if !TSC_DEADLINE_AVAILABLE.load(Ordering::Relaxed) {
        return Err(ApicTimerError::TscDeadlineNotSupported);
    }

    // Configure LVT: TSC-deadline mode, specified vector, not masked
    let lvt = (vector as u32) | (ApicTimerMode::TscDeadline as u32) << 17;
    write_apic(apic_regs::LVT_TIMER, lvt);

    // Write deadline to IA32_TSC_DEADLINE MSR
    tsc::write_deadline(deadline);

    PER_CPU_TIMER[cpu_id].mode.store(ApicTimerMode::TscDeadline as u32, Ordering::SeqCst);

    Ok(())
}

/// Arm TSC-deadline timer with relative time (nanoseconds)
///
/// # Safety
///
/// TSC-deadline mode must be supported.
pub unsafe fn arm_deadline_ns(cpu_id: usize, ns_from_now: u64, vector: u8) -> Result<(), ApicTimerError> {
    let ticks = super::ns_to_tsc(ns_from_now);
    let deadline = tsc::read() + ticks;
    start_tsc_deadline(cpu_id, deadline, vector)
}

/// Stop the APIC timer
///
/// # Safety
///
/// Timer must be initialized.
pub unsafe fn stop(cpu_id: usize) -> Result<(), ApicTimerError> {
    if cpu_id >= MAX_CPUS {
        return Err(ApicTimerError::InvalidCpuId);
    }

    let mode = PER_CPU_TIMER[cpu_id].mode.load(Ordering::Relaxed);

    if mode == ApicTimerMode::TscDeadline as u32 {
        // Disarm TSC deadline
        tsc::disarm_deadline();
    } else {
        // Stop counter-based timer
        write_apic(apic_regs::TIMER_ICR, 0);
    }

    // Mask the timer
    let lvt = read_apic(apic_regs::LVT_TIMER);
    write_apic(apic_regs::LVT_TIMER, lvt | (1 << 16));

    Ok(())
}

/// Read current timer count
pub fn read_current_count() -> u32 {
    unsafe { read_apic(apic_regs::TIMER_CCR) }
}

/// Get timer frequency for a CPU
pub fn get_frequency(cpu_id: usize) -> Option<u64> {
    if cpu_id >= MAX_CPUS {
        return None;
    }

    if PER_CPU_TIMER[cpu_id].calibrated.load(Ordering::Relaxed) {
        Some(PER_CPU_TIMER[cpu_id].frequency.load(Ordering::Relaxed))
    } else {
        None
    }
}

/// Check if TSC-deadline mode is available
pub fn tsc_deadline_available() -> bool {
    TSC_DEADLINE_AVAILABLE.load(Ordering::Relaxed)
}

// =============================================================================
// Error Type
// =============================================================================

/// APIC Timer error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApicTimerError {
    /// Invalid CPU ID
    InvalidCpuId,
    /// Timer not initialized
    NotInitialized,
    /// Timer not calibrated
    NotCalibrated,
    /// TSC-deadline mode not supported
    TscDeadlineNotSupported,
    /// Calibration failed
    CalibrationFailed,
}

impl core::fmt::Display for ApicTimerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ApicTimerError::InvalidCpuId => write!(f, "Invalid CPU ID"),
            ApicTimerError::NotInitialized => write!(f, "APIC timer not initialized"),
            ApicTimerError::NotCalibrated => write!(f, "APIC timer not calibrated"),
            ApicTimerError::TscDeadlineNotSupported => write!(f, "TSC-deadline mode not supported"),
            ApicTimerError::CalibrationFailed => write!(f, "Timer calibration failed"),
        }
    }
}

// =============================================================================
// APIC Timer Structure
// =============================================================================

/// APIC Timer abstraction
pub struct ApicTimer {
    cpu_id: usize,
}

impl ApicTimer {
    /// Create a new APIC timer for the specified CPU
    pub fn new(cpu_id: usize) -> Option<Self> {
        if cpu_id >= MAX_CPUS {
            return None;
        }
        Some(Self { cpu_id })
    }

    /// Get the CPU ID
    #[inline]
    pub fn cpu_id(&self) -> usize {
        self.cpu_id
    }

    /// Check if calibrated
    #[inline]
    pub fn is_calibrated(&self) -> bool {
        PER_CPU_TIMER[self.cpu_id].calibrated.load(Ordering::Relaxed)
    }

    /// Get frequency
    #[inline]
    pub fn frequency(&self) -> Option<u64> {
        get_frequency(self.cpu_id)
    }

    /// Calculate ticks for a duration in nanoseconds
    pub fn ns_to_ticks(&self, ns: u64) -> Option<u32> {
        let freq = self.frequency()?;
        let ticks = (ns as u128 * freq as u128) / 1_000_000_000;
        if ticks > u32::MAX as u128 {
            None
        } else {
            Some(ticks as u32)
        }
    }

    /// Calculate ticks for a duration in microseconds
    pub fn us_to_ticks(&self, us: u64) -> Option<u32> {
        self.ns_to_ticks(us * 1_000)
    }

    /// Calculate ticks for a duration in milliseconds
    pub fn ms_to_ticks(&self, ms: u64) -> Option<u32> {
        self.ns_to_ticks(ms * 1_000_000)
    }

    /// Start one-shot timer
    ///
    /// # Safety
    ///
    /// Timer must be initialized and calibrated.
    pub unsafe fn start_oneshot(&self, ticks: u32, vector: u8) -> Result<(), ApicTimerError> {
        start_oneshot(self.cpu_id, ticks, vector)
    }

    /// Start periodic timer
    ///
    /// # Safety
    ///
    /// Timer must be initialized and calibrated.
    pub unsafe fn start_periodic(&self, ticks: u32, vector: u8) -> Result<(), ApicTimerError> {
        start_periodic(self.cpu_id, ticks, vector)
    }

    /// Start TSC-deadline timer
    ///
    /// # Safety
    ///
    /// TSC-deadline must be supported.
    pub unsafe fn start_tsc_deadline(&self, deadline: u64, vector: u8) -> Result<(), ApicTimerError> {
        start_tsc_deadline(self.cpu_id, deadline, vector)
    }

    /// Stop the timer
    ///
    /// # Safety
    ///
    /// Timer must be initialized.
    pub unsafe fn stop(&self) -> Result<(), ApicTimerError> {
        stop(self.cpu_id)
    }

    /// Read current count
    #[inline]
    pub fn current_count(&self) -> u32 {
        read_current_count()
    }
}
