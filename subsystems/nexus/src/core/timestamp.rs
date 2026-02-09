//! High-precision timestamps for NEXUS.

/// High-precision timestamp for NEXUS events
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NexusTimestamp(u64);

impl NexusTimestamp {
    /// Create a new timestamp
    #[inline(always)]
    pub const fn new(ticks: u64) -> Self {
        Self(ticks)
    }

    /// Create from raw ticks (alias for new)
    #[inline(always)]
    pub const fn from_ticks(ticks: u64) -> Self {
        Self(ticks)
    }

    /// Get raw ticks
    #[inline(always)]
    pub const fn ticks(&self) -> u64 {
        self.0
    }

    /// Get raw value (alias for ticks)
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }

    /// Get current timestamp (platform-specific)
    #[inline]
    pub fn now() -> Self {
        Self::new(Self::read_tsc())
    }

    /// Read TSC or equivalent
    #[cfg(target_arch = "x86_64")]
    #[inline]
    fn read_tsc() -> u64 {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::x86_64::_rdtsc()
        }
        #[cfg(not(target_arch = "x86_64"))]
        0
    }

    #[cfg(target_arch = "aarch64")]
    #[inline]
    fn read_tsc() -> u64 {
        let cnt: u64;
        unsafe {
            core::arch::asm!("mrs {}, cntvct_el0", out(reg) cnt);
        }
        cnt
    }

    #[cfg(target_arch = "riscv64")]
    #[inline]
    fn read_tsc() -> u64 {
        let cnt: u64;
        unsafe {
            core::arch::asm!("rdtime {}", out(reg) cnt);
        }
        cnt
    }

    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64"
    )))]
    #[inline]
    fn read_tsc() -> u64 {
        0
    }

    /// Duration since another timestamp
    #[inline(always)]
    pub fn duration_since(&self, earlier: Self) -> u64 {
        self.0.saturating_sub(earlier.0)
    }

    /// Convert to nanoseconds (approximate)
    #[inline(always)]
    pub fn to_nanos(&self, frequency_ghz: f64) -> u64 {
        (self.0 as f64 / frequency_ghz) as u64
    }
}

impl Default for NexusTimestamp {
    fn default() -> Self {
        Self::now()
    }
}
