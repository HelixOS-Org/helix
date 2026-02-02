//! # Time Stamp Counter (TSC)
//!
//! The TSC is a 64-bit register present on all x86_64 processors that
//! counts processor cycles. On modern CPUs with invariant TSC, it
//! provides a reliable, high-resolution time source.
//!
//! ## TSC Variants
//!
//! 1. **Non-invariant TSC** (older CPUs)
//!    - Frequency changes with CPU frequency
//!    - Not suitable for timekeeping
//!
//! 2. **Constant TSC**
//!    - Fixed frequency even during power states
//!    - Stops during some sleep states
//!
//! 3. **Invariant TSC** (modern CPUs)
//!    - Constant rate at all ACPI P/C states
//!    - Synchronized across all cores
//!    - Best for timekeeping
//!
//! ## RDTSCP
//!
//! The RDTSCP instruction also returns the processor ID, which is useful
//! for determining which CPU's TSC was read (important for older SMP systems).

use core::sync::atomic::{AtomicU64, Ordering};

// =============================================================================
// Constants
// =============================================================================

/// CPUID leaf for TSC information
const CPUID_TSC_INFO: u32 = 0x15;

/// CPUID leaf for processor frequency info
const CPUID_FREQ_INFO: u32 = 0x16;

/// IA32_TSC_AUX MSR (processor ID for RDTSCP)
const IA32_TSC_AUX: u32 = 0xC0000103;

// =============================================================================
// TSC Features
// =============================================================================

/// TSC feature flags
#[derive(Debug, Clone, Copy)]
pub struct TscFeatures {
    /// TSC is available
    pub available: bool,
    /// TSC is invariant (constant rate)
    pub invariant: bool,
    /// RDTSCP instruction available
    pub rdtscp: bool,
    /// TSC deadline mode available (for APIC)
    pub tsc_deadline: bool,
    /// TSC adjust MSR available
    pub tsc_adjust: bool,
}

impl TscFeatures {
    /// Detect TSC features from CPUID
    pub fn detect() -> Self {
        let mut features = Self {
            available: false,
            invariant: false,
            rdtscp: false,
            tsc_deadline: false,
            tsc_adjust: false,
        };

        // Check basic TSC support (CPUID.01H:EDX.TSC[bit 4])
        let (eax, _, ecx, edx) = cpuid(1);
        features.available = edx & (1 << 4) != 0;
        features.tsc_deadline = ecx & (1 << 24) != 0;

        if !features.available {
            return features;
        }

        // Check for RDTSCP (CPUID.80000001H:EDX[bit 27])
        if cpuid_max_extended() >= 0x80000001 {
            let (_, _, _, edx) = cpuid(0x80000001);
            features.rdtscp = edx & (1 << 27) != 0;
        }

        // Check for invariant TSC (CPUID.80000007H:EDX[bit 8])
        if cpuid_max_extended() >= 0x80000007 {
            let (_, _, _, edx) = cpuid(0x80000007);
            features.invariant = edx & (1 << 8) != 0;
        }

        // Check for TSC_ADJUST MSR (CPUID.07H:EBX[bit 1])
        if eax >= 7 {
            let (_, ebx, _, _) = cpuid_subleaf(7, 0);
            features.tsc_adjust = ebx & (1 << 1) != 0;
        }

        features
    }
}

/// Detect TSC features
pub fn detect_features() -> TscFeatures {
    TscFeatures::detect()
}

// =============================================================================
// CPUID Helpers
// =============================================================================

fn cpuid(leaf: u32) -> (u32, u32, u32, u32) {
    let (eax, ebx, ecx, edx): (u32, u32, u32, u32);
    unsafe {
        core::arch::asm!(
            "cpuid",
            inout("eax") leaf => eax,
            out("ebx") ebx,
            out("ecx") ecx,
            out("edx") edx,
            options(nostack, preserves_flags),
        );
    }
    (eax, ebx, ecx, edx)
}

fn cpuid_subleaf(leaf: u32, subleaf: u32) -> (u32, u32, u32, u32) {
    let (eax, ebx, ecx, edx): (u32, u32, u32, u32);
    unsafe {
        core::arch::asm!(
            "cpuid",
            inout("eax") leaf => eax,
            out("ebx") ebx,
            inout("ecx") subleaf => ecx,
            out("edx") edx,
            options(nostack, preserves_flags),
        );
    }
    (eax, ebx, ecx, edx)
}

fn cpuid_max_extended() -> u32 {
    let (eax, _, _, _) = cpuid(0x80000000);
    eax
}

// =============================================================================
// TSC Reading
// =============================================================================

/// Read the Time Stamp Counter
#[inline]
pub fn read() -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        core::arch::asm!(
            "rdtsc",
            out("eax") low,
            out("edx") high,
            options(nostack, nomem, preserves_flags),
        );
    }
    ((high as u64) << 32) | (low as u64)
}

/// Read the Time Stamp Counter with serialization
///
/// Uses LFENCE to ensure the RDTSC is not reordered.
#[inline]
pub fn read_serialized() -> u64 {
    unsafe {
        core::arch::asm!("lfence", options(nostack, preserves_flags));
    }
    let result = read();
    unsafe {
        core::arch::asm!("lfence", options(nostack, preserves_flags));
    }
    result
}

/// Read the Time Stamp Counter and processor ID
///
/// Returns (tsc, processor_id).
/// The processor ID is the value set in IA32_TSC_AUX MSR.
#[inline]
pub fn read_with_id() -> (u64, u32) {
    let low: u32;
    let high: u32;
    let aux: u32;
    unsafe {
        core::arch::asm!(
            "rdtscp",
            out("eax") low,
            out("edx") high,
            out("ecx") aux,
            options(nostack, nomem, preserves_flags),
        );
    }
    (((high as u64) << 32) | (low as u64), aux)
}

/// Read TSC with serialization using RDTSCP if available
#[inline]
pub fn read_best() -> u64 {
    static RDTSCP_AVAILABLE: AtomicU64 = AtomicU64::new(2); // 2 = unknown

    match RDTSCP_AVAILABLE.load(Ordering::Relaxed) {
        0 => read_serialized(),
        1 => read_with_id().0,
        _ => {
            // Check once
            let features = TscFeatures::detect();
            if features.rdtscp {
                RDTSCP_AVAILABLE.store(1, Ordering::Relaxed);
                read_with_id().0
            } else {
                RDTSCP_AVAILABLE.store(0, Ordering::Relaxed);
                read_serialized()
            }
        },
    }
}

// =============================================================================
// TSC Frequency Detection
// =============================================================================

/// Get TSC frequency from CPUID (if available)
///
/// This uses CPUID leaf 0x15 (TSC/Core Crystal Clock)
/// and optionally 0x16 (Processor Frequency Information).
pub fn get_frequency_from_cpuid() -> Option<u64> {
    let (max_leaf, _, _, _) = cpuid(0);

    if max_leaf >= CPUID_TSC_INFO {
        let (eax, ebx, ecx, _) = cpuid(CPUID_TSC_INFO);

        // EAX = denominator of TSC/crystal ratio
        // EBX = numerator of TSC/crystal ratio
        // ECX = crystal frequency in Hz (if non-zero)

        if eax != 0 && ebx != 0 {
            let crystal_freq = if ecx != 0 {
                ecx as u64
            } else {
                // Try to determine crystal frequency from processor family
                // This is a fallback for processors that don't report it
                get_default_crystal_frequency()
            };

            if crystal_freq != 0 {
                // TSC frequency = (crystal * numerator) / denominator
                let tsc_freq = (crystal_freq * ebx as u64) / eax as u64;
                return Some(tsc_freq);
            }
        }
    }

    // Try CPUID.16H for base frequency
    if max_leaf >= CPUID_FREQ_INFO {
        let (eax, _, _, _) = cpuid(CPUID_FREQ_INFO);

        // EAX = Processor Base Frequency (in MHz)
        if eax != 0 {
            // Note: This is base frequency, not TSC frequency
            // For invariant TSC, they're usually the same
            return Some((eax as u64) * 1_000_000);
        }
    }

    None
}

/// Get default crystal frequency for known processor families
fn get_default_crystal_frequency() -> u64 {
    // Read processor signature
    let (_, _, _, _) = cpuid(1);

    // TODO: Add processor family detection
    // Common values:
    // - Skylake/Kaby Lake desktop: 24 MHz
    // - Skylake-X: 25 MHz
    // - Atom Goldmont: 19.2 MHz

    // Return 0 to indicate unknown
    0
}

// =============================================================================
// TSC Struct
// =============================================================================

/// TSC wrapper for timing operations
pub struct Tsc {
    /// Cached frequency (0 = not calibrated)
    frequency: u64,
}

impl Tsc {
    /// Create a new TSC instance
    pub const fn new() -> Self {
        Self { frequency: 0 }
    }

    /// Create a calibrated TSC instance
    pub fn calibrated(frequency: u64) -> Self {
        Self { frequency }
    }

    /// Get the TSC frequency
    #[inline]
    pub fn frequency(&self) -> u64 {
        self.frequency
    }

    /// Set the TSC frequency (after calibration)
    pub fn set_frequency(&mut self, frequency: u64) {
        self.frequency = frequency;
    }

    /// Read the current TSC value
    #[inline]
    pub fn read(&self) -> u64 {
        read()
    }

    /// Read with serialization
    #[inline]
    pub fn read_serialized(&self) -> u64 {
        read_serialized()
    }

    /// Convert TSC ticks to nanoseconds
    #[inline]
    pub fn ticks_to_ns(&self, ticks: u64) -> u64 {
        if self.frequency > 0 {
            ((ticks as u128 * 1_000_000_000) / self.frequency as u128) as u64
        } else {
            0
        }
    }

    /// Convert nanoseconds to TSC ticks
    #[inline]
    pub fn ns_to_ticks(&self, ns: u64) -> u64 {
        if self.frequency > 0 {
            ((ns as u128 * self.frequency as u128) / 1_000_000_000) as u64
        } else {
            0
        }
    }

    /// Delay for specified nanoseconds
    pub fn delay_ns(&self, ns: u64) {
        let ticks = self.ns_to_ticks(ns);
        let start = self.read();
        while self.read() - start < ticks {
            core::hint::spin_loop();
        }
    }

    /// Delay for specified microseconds
    #[inline]
    pub fn delay_us(&self, us: u64) {
        self.delay_ns(us * 1_000);
    }

    /// Delay for specified milliseconds
    #[inline]
    pub fn delay_ms(&self, ms: u64) {
        self.delay_ns(ms * 1_000_000);
    }
}

impl Default for Tsc {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// TSC Synchronization (for SMP)
// =============================================================================

/// TSC offset for synchronization
static TSC_OFFSET: AtomicU64 = AtomicU64::new(0);

/// Set TSC offset for this CPU
///
/// # Safety
///
/// Must be called during AP initialization.
pub unsafe fn set_offset(offset: u64) {
    TSC_OFFSET.store(offset, Ordering::SeqCst);
}

/// Get synchronized TSC value
#[inline]
pub fn read_synchronized() -> u64 {
    read().wrapping_add(TSC_OFFSET.load(Ordering::Relaxed))
}

/// Set the IA32_TSC_AUX MSR (processor ID for RDTSCP)
///
/// # Safety
///
/// Must be called during CPU initialization.
pub unsafe fn set_aux(value: u32) {
    core::arch::asm!(
        "wrmsr",
        in("ecx") IA32_TSC_AUX,
        in("eax") value,
        in("edx") 0u32,
        options(nostack, preserves_flags),
    );
}

// =============================================================================
// TSC Deadline (for APIC Timer)
// =============================================================================

/// IA32_TSC_DEADLINE MSR
const IA32_TSC_DEADLINE: u32 = 0x6E0;

/// Write to TSC deadline MSR
///
/// When the TSC reaches this value, a timer interrupt is generated.
///
/// # Safety
///
/// APIC timer must be configured in TSC-deadline mode.
pub unsafe fn write_deadline(deadline: u64) {
    let low = deadline as u32;
    let high = (deadline >> 32) as u32;
    core::arch::asm!(
        "wrmsr",
        in("ecx") IA32_TSC_DEADLINE,
        in("eax") low,
        in("edx") high,
        options(nostack, preserves_flags),
    );
}

/// Arm a TSC deadline relative to current time
///
/// # Safety
///
/// APIC timer must be configured in TSC-deadline mode.
#[inline]
pub unsafe fn arm_deadline_relative(ticks: u64) {
    let deadline = read() + ticks;
    write_deadline(deadline);
}

/// Disarm TSC deadline timer
///
/// # Safety
///
/// APIC timer must be configured in TSC-deadline mode.
#[inline]
pub unsafe fn disarm_deadline() {
    write_deadline(0);
}
