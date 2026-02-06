//! Timing utilities for accurate cycle counting
//!
//! Provides platform-specific timing primitives for benchmarking.



// =============================================================================
// Time Stamp Counter (TSC)
// =============================================================================

/// Read the Time Stamp Counter (x86_64)
/// Returns CPU cycles since reset
#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub fn read_tsc() -> u64 {
    let lo: u32;
    let hi: u32;

    // RDTSC instruction reads 64-bit TSC into EDX:EAX
    unsafe {
        core::arch::asm!(
            "rdtsc",
            out("eax") lo,
            out("edx") hi,
            options(nostack, nomem, preserves_flags)
        );
    }

    ((hi as u64) << 32) | (lo as u64)
}

/// Read TSC with serialization (more accurate but slower)
#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub fn read_tsc_serialized() -> u64 {
    let lo: u32;
    let hi: u32;

    // RDTSCP serializes and returns processor ID in ECX
    unsafe {
        core::arch::asm!(
            "rdtscp",
            out("eax") lo,
            out("edx") hi,
            out("ecx") _,
            options(nostack, nomem, preserves_flags)
        );
    }

    ((hi as u64) << 32) | (lo as u64)
}

/// Read TSC with full serialization (CPUID barrier)
#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub fn read_tsc_fenced() -> u64 {
    // CPUID serializes the instruction stream
    // Note: We save/restore rbx manually since LLVM reserves it
    unsafe {
        let _eax: u32;
        let _ecx: u32;
        let _edx: u32;
        core::arch::asm!(
            "push rbx",
            "cpuid",
            "pop rbx",
            inout("eax") 0 => _eax,
            out("ecx") _ecx,
            out("edx") _edx,
            options(nostack, preserves_flags)
        );
    }
    read_tsc()
}

/// ARM64 cycle counter
#[cfg(target_arch = "aarch64")]
#[inline(always)]
pub fn read_tsc() -> u64 {
    let count: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, cntvct_el0",
            out(reg) count,
            options(nostack, nomem)
        );
    }
    count
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
pub fn read_tsc_serialized() -> u64 {
    let count: u64;
    unsafe {
        core::arch::asm!(
            "isb",
            "mrs {}, cntvct_el0",
            out(reg) count,
            options(nostack, nomem)
        );
    }
    count
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
pub fn read_tsc_fenced() -> u64 {
    read_tsc_serialized()
}

/// Fallback for unsupported architectures
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[inline(always)]
pub fn read_tsc() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[inline(always)]
pub fn read_tsc_serialized() -> u64 {
    read_tsc()
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[inline(always)]
pub fn read_tsc_fenced() -> u64 {
    read_tsc()
}

// =============================================================================
// Timing Source Abstraction
// =============================================================================

/// Timing source trait
pub trait TimingSource {
    /// Read current cycle count
    fn read(&self) -> u64;

    /// Frequency in Hz
    fn frequency(&self) -> u64;

    /// Name of timing source
    fn name(&self) -> &'static str;
}

/// CPU cycle counter
pub struct CycleCounter {
    frequency_mhz: u64,
}

impl CycleCounter {
    pub fn new(frequency_mhz: u64) -> Self {
        Self { frequency_mhz }
    }

    pub fn with_auto_detect() -> Self {
        // TODO: Auto-detect CPU frequency
        Self {
            frequency_mhz: 2500,
        }
    }
}

impl TimingSource for CycleCounter {
    fn read(&self) -> u64 {
        read_tsc()
    }

    fn frequency(&self) -> u64 {
        self.frequency_mhz * 1_000_000
    }

    fn name(&self) -> &'static str {
        "TSC"
    }
}

impl Default for CycleCounter {
    fn default() -> Self {
        Self::with_auto_detect()
    }
}

// =============================================================================
// Measurement Helpers
// =============================================================================

/// Measure cycles for a closure
#[inline(always)]
pub fn measure<F, R>(f: F) -> (u64, R)
where
    F: FnOnce() -> R,
{
    let start = read_tsc();
    let result = f();
    let end = read_tsc();
    (end - start, result)
}

/// Measure cycles for a closure (serialized)
#[inline(always)]
pub fn measure_serialized<F, R>(f: F) -> (u64, R)
where
    F: FnOnce() -> R,
{
    let start = read_tsc_fenced();
    let result = f();
    let end = read_tsc_serialized();
    (end - start, result)
}

/// Measure cycles for a closure, returning only cycles
#[inline(always)]
pub fn measure_cycles<F>(f: F) -> u64
where
    F: FnOnce(),
{
    let start = read_tsc();
    f();
    let end = read_tsc();
    end - start
}

/// Measure cycles for a closure with warmup
#[inline(always)]
pub fn measure_with_warmup<F>(warmup: u32, f: F) -> u64
where
    F: Fn(),
{
    // Warmup
    for _ in 0..warmup {
        f();
    }

    // Actual measurement
    let start = read_tsc();
    f();
    read_tsc() - start
}

// =============================================================================
// Time Conversion
// =============================================================================

/// Convert cycles to nanoseconds
#[inline]
pub fn cycles_to_ns(cycles: u64, freq_mhz: u64) -> u64 {
    if freq_mhz == 0 {
        return 0;
    }
    cycles * 1000 / freq_mhz
}

/// Convert cycles to microseconds
#[inline]
pub fn cycles_to_us(cycles: u64, freq_mhz: u64) -> u64 {
    if freq_mhz == 0 {
        return 0;
    }
    cycles / freq_mhz
}

/// Convert cycles to milliseconds
#[inline]
pub fn cycles_to_ms(cycles: u64, freq_mhz: u64) -> u64 {
    if freq_mhz == 0 {
        return 0;
    }
    cycles / (freq_mhz * 1000)
}

/// Convert nanoseconds to cycles
#[inline]
pub fn ns_to_cycles(ns: u64, freq_mhz: u64) -> u64 {
    ns * freq_mhz / 1000
}

// =============================================================================
// Delay Functions
// =============================================================================

/// Busy-wait for specified cycles
#[inline]
pub fn delay_cycles(cycles: u64) {
    let start = read_tsc();
    while read_tsc() - start < cycles {
        core::hint::spin_loop();
    }
}

/// Busy-wait for specified nanoseconds
#[inline]
pub fn delay_ns(ns: u64, freq_mhz: u64) {
    delay_cycles(ns_to_cycles(ns, freq_mhz));
}

/// Busy-wait for specified microseconds
#[inline]
pub fn delay_us(us: u64, freq_mhz: u64) {
    delay_cycles(us * freq_mhz);
}

// =============================================================================
// Stopwatch
// =============================================================================

/// Simple stopwatch for measuring elapsed time
pub struct Stopwatch {
    start: u64,
    freq_mhz: u64,
}

impl Stopwatch {
    /// Start a new stopwatch
    pub fn start(freq_mhz: u64) -> Self {
        Self {
            start: read_tsc(),
            freq_mhz,
        }
    }

    /// Start with default frequency
    pub fn start_default() -> Self {
        Self::start(2500)
    }

    /// Elapsed cycles since start
    pub fn elapsed_cycles(&self) -> u64 {
        read_tsc() - self.start
    }

    /// Elapsed nanoseconds since start
    pub fn elapsed_ns(&self) -> u64 {
        cycles_to_ns(self.elapsed_cycles(), self.freq_mhz)
    }

    /// Elapsed microseconds since start
    pub fn elapsed_us(&self) -> u64 {
        cycles_to_us(self.elapsed_cycles(), self.freq_mhz)
    }

    /// Elapsed milliseconds since start
    pub fn elapsed_ms(&self) -> u64 {
        cycles_to_ms(self.elapsed_cycles(), self.freq_mhz)
    }

    /// Lap: return elapsed and restart
    pub fn lap(&mut self) -> u64 {
        let elapsed = self.elapsed_cycles();
        self.start = read_tsc();
        elapsed
    }

    /// Reset the stopwatch
    pub fn reset(&mut self) {
        self.start = read_tsc();
    }
}

// =============================================================================
// Multi-Sample Timer
// =============================================================================

/// Timer that collects multiple samples
pub struct MultiSampleTimer {
    samples: [u64; 64],
    count: usize,
    freq_mhz: u64,
}

impl MultiSampleTimer {
    pub fn new(freq_mhz: u64) -> Self {
        Self {
            samples: [0; 64],
            count: 0,
            freq_mhz,
        }
    }

    /// Record a sample
    pub fn record(&mut self, cycles: u64) {
        if self.count < 64 {
            self.samples[self.count] = cycles;
            self.count += 1;
        }
    }

    /// Get number of samples
    pub fn count(&self) -> usize {
        self.count
    }

    /// Calculate minimum
    pub fn min(&self) -> u64 {
        self.samples[..self.count]
            .iter()
            .copied()
            .min()
            .unwrap_or(0)
    }

    /// Calculate maximum
    pub fn max(&self) -> u64 {
        self.samples[..self.count]
            .iter()
            .copied()
            .max()
            .unwrap_or(0)
    }

    /// Calculate mean
    pub fn mean(&self) -> u64 {
        if self.count == 0 {
            return 0;
        }
        let sum: u64 = self.samples[..self.count].iter().sum();
        sum / self.count as u64
    }

    /// Reset
    pub fn reset(&mut self) {
        self.count = 0;
    }
}
