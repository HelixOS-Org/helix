//! # KASLR - Kernel Address Space Layout Randomization
//!
//! Framework-level KASLR implementation with hardware entropy.

use crate::{PhysAddr, RelocError, RelocResult};

// ============================================================================
// ENTROPY SOURCES
// ============================================================================

/// Entropy quality level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum EntropyQuality {
    /// No entropy available
    None     = 0,
    /// Low quality (TSC-based)
    Low      = 1,
    /// Medium quality (RDRAND)
    Medium   = 2,
    /// High quality (RDSEED)
    High     = 3,
    /// Hardware RNG
    Hardware = 4,
}

/// Entropy source configuration
#[derive(Debug, Clone)]
pub struct EntropyConfig {
    /// Minimum acceptable quality
    pub min_quality: EntropyQuality,
    /// Retry count for hardware sources
    pub retry_count: u32,
    /// Mix multiple sources
    pub mix_sources: bool,
}

impl Default for EntropyConfig {
    fn default() -> Self {
        Self {
            min_quality: EntropyQuality::Low,
            retry_count: 10,
            mix_sources: true,
        }
    }
}

// ============================================================================
// ENTROPY COLLECTION
// ============================================================================

/// Entropy collector for KASLR
pub struct EntropyCollector {
    config: EntropyConfig,
    seed: u64,
    quality: EntropyQuality,
}

impl EntropyCollector {
    /// Create new entropy collector
    pub const fn new(config: EntropyConfig) -> Self {
        Self {
            config,
            seed: 0,
            quality: EntropyQuality::None,
        }
    }

    /// Collect entropy from all available sources
    pub fn collect(&mut self) -> RelocResult<u64> {
        self.seed = 0;
        self.quality = EntropyQuality::None;

        // Try hardware RNG first (highest quality)
        if let Some(entropy) = self.try_rdseed() {
            self.mix_entropy(entropy, EntropyQuality::High);
        }

        // Try RDRAND (good quality)
        if let Some(entropy) = self.try_rdrand() {
            self.mix_entropy(entropy, EntropyQuality::Medium);
        }

        // Try TSC (low quality but always available)
        if let Some(entropy) = self.try_tsc() {
            self.mix_entropy(entropy, EntropyQuality::Low);
        }

        // Check minimum quality
        if self.quality < self.config.min_quality {
            return Err(RelocError::InsufficientEntropy);
        }

        Ok(self.seed)
    }

    /// Try RDSEED instruction
    fn try_rdseed(&self) -> Option<u64> {
        #[cfg(target_arch = "x86_64")]
        {
            for _ in 0..self.config.retry_count {
                if let Some(val) = x86_64_rdseed() {
                    return Some(val);
                }
            }
        }
        None
    }

    /// Try RDRAND instruction
    fn try_rdrand(&self) -> Option<u64> {
        #[cfg(target_arch = "x86_64")]
        {
            for _ in 0..self.config.retry_count {
                if let Some(val) = x86_64_rdrand() {
                    return Some(val);
                }
            }
        }
        None
    }

    /// Try TSC (timestamp counter)
    fn try_tsc(&self) -> Option<u64> {
        #[cfg(target_arch = "x86_64")]
        {
            Some(x86_64_rdtsc())
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            None
        }
    }

    /// Mix entropy into seed
    fn mix_entropy(&mut self, entropy: u64, quality: EntropyQuality) {
        if self.config.mix_sources {
            // Use xorshift-like mixing
            self.seed ^= entropy;
            self.seed = self.seed.wrapping_mul(0x517cc1b727220a95);
            self.seed ^= self.seed >> 33;
        } else if quality > self.quality {
            self.seed = entropy;
        }

        if quality > self.quality {
            self.quality = quality;
        }
    }

    /// Get current entropy quality
    pub fn quality(&self) -> EntropyQuality {
        self.quality
    }
}

// ============================================================================
// x86_64 HARDWARE ENTROPY
// ============================================================================

#[cfg(target_arch = "x86_64")]
fn x86_64_rdseed() -> Option<u64> {
    let value: u64;
    let success: u8;

    // Check if RDSEED is supported
    if !cpuid_rdseed_supported() {
        return None;
    }

    unsafe {
        core::arch::asm!(
            "rdseed {0}",
            "setc {1}",
            out(reg) value,
            out(reg_byte) success,
            options(nomem, nostack)
        );
    }

    if success != 0 { Some(value) } else { None }
}

#[cfg(target_arch = "x86_64")]
fn x86_64_rdrand() -> Option<u64> {
    let value: u64;
    let success: u8;

    // Check if RDRAND is supported
    if !cpuid_rdrand_supported() {
        return None;
    }

    unsafe {
        core::arch::asm!(
            "rdrand {0}",
            "setc {1}",
            out(reg) value,
            out(reg_byte) success,
            options(nomem, nostack)
        );
    }

    if success != 0 { Some(value) } else { None }
}

#[cfg(target_arch = "x86_64")]
fn x86_64_rdtsc() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}

#[cfg(target_arch = "x86_64")]
fn cpuid_rdrand_supported() -> bool {
    // CPUID.01H:ECX.RDRAND[bit 30]
    let ecx: u32;
    unsafe {
        core::arch::asm!(
            "push rbx",
            "mov eax, 1",
            "cpuid",
            "pop rbx",
            out("ecx") ecx,
            out("eax") _,
            out("edx") _,
            options(nomem)
        );
    }
    (ecx & (1 << 30)) != 0
}

#[cfg(target_arch = "x86_64")]
fn cpuid_rdseed_supported() -> bool {
    // CPUID.07H.0H:EBX.RDSEED[bit 18]
    let ebx_result: u32;
    unsafe {
        core::arch::asm!(
            "push rbx",
            "mov eax, 7",
            "xor ecx, ecx",
            "cpuid",
            "mov {0:e}, ebx",
            "pop rbx",
            out(reg) ebx_result,
            out("eax") _,
            out("ecx") _,
            out("edx") _,
            options(nomem)
        );
    }
    (ebx_result & (1 << 18)) != 0
}

// ============================================================================
// KASLR ADDRESS GENERATION
// ============================================================================

/// KASLR configuration
#[derive(Debug, Clone)]
pub struct KaslrConfig {
    /// Entropy configuration
    pub entropy: EntropyConfig,
    /// Minimum alignment (power of 2)
    pub alignment: usize,
    /// Address range start
    pub range_start: u64,
    /// Address range end
    pub range_end: u64,
    /// Number of bits of randomness
    pub entropy_bits: u32,
}

impl Default for KaslrConfig {
    fn default() -> Self {
        Self {
            entropy: EntropyConfig::default(),
            alignment: 0x20_0000, // 2 MiB alignment
            range_start: 0xFFFF_8000_0000_0000,
            range_end: 0xFFFF_FFFF_8000_0000,
            entropy_bits: 20, // ~1M possible positions
        }
    }
}

/// KASLR engine
pub struct Kaslr {
    config: KaslrConfig,
    collector: EntropyCollector,
    slide: i64,
    initialized: bool,
}

impl Kaslr {
    /// Create new KASLR engine
    pub const fn new(config: KaslrConfig) -> Self {
        Self {
            collector: EntropyCollector::new(EntropyConfig {
                min_quality: config.entropy.min_quality,
                retry_count: config.entropy.retry_count,
                mix_sources: config.entropy.mix_sources,
            }),
            config,
            slide: 0,
            initialized: false,
        }
    }

    /// Initialize KASLR and generate slide
    pub fn initialize(&mut self, kernel_size: usize) -> RelocResult<i64> {
        if self.initialized {
            return Ok(self.slide);
        }

        // Collect entropy
        let entropy = self.collector.collect()?;

        // Calculate available range
        let range = self.config.range_end - self.config.range_start;
        let available = range.saturating_sub(kernel_size as u64);

        // Apply alignment mask
        let align_mask = !(self.config.alignment as u64 - 1);
        let aligned_range = available & align_mask;

        if aligned_range == 0 {
            return Err(RelocError::InvalidKernelLayout);
        }

        // Generate offset
        let entropy_mask = (1u64 << self.config.entropy_bits) - 1;
        let raw_offset = entropy & entropy_mask;

        // Scale to available range with alignment
        let positions = aligned_range / self.config.alignment as u64;
        let position = raw_offset % positions;
        let offset = position * self.config.alignment as u64;

        self.slide = offset as i64;
        self.initialized = true;

        Ok(self.slide)
    }

    /// Get current slide value
    pub fn slide(&self) -> i64 {
        self.slide
    }

    /// Get entropy quality
    pub fn entropy_quality(&self) -> EntropyQuality {
        self.collector.quality()
    }

    /// Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Apply slide to physical address
    pub fn apply_to_phys(&self, addr: PhysAddr) -> PhysAddr {
        PhysAddr(addr.0.wrapping_add(self.slide as u64))
    }
}

// ============================================================================
// EARLY BOOT KASLR
// ============================================================================

/// Early boot KASLR (minimal, no allocation)
pub struct EarlyKaslr {
    slide: i64,
    alignment: usize,
}

impl EarlyKaslr {
    /// Create early KASLR with given alignment
    pub const fn new(alignment: usize) -> Self {
        Self {
            slide: 0,
            alignment,
        }
    }

    /// Generate slide using only TSC (always available)
    ///
    /// # Safety
    /// Must be called only once during early boot
    pub unsafe fn generate_slide(&mut self, available_range: u64) -> i64 {
        #[cfg(target_arch = "x86_64")]
        {
            let tsc = x86_64_rdtsc();

            // Simple hash of TSC
            let mut hash = tsc;
            hash ^= hash >> 33;
            hash = hash.wrapping_mul(0xff51afd7ed558ccd);
            hash ^= hash >> 33;

            // Align and fit to range
            let positions = available_range / self.alignment as u64;
            if positions > 0 {
                let position = hash % positions;
                self.slide = (position * self.alignment as u64) as i64;
            }
        }

        self.slide
    }

    /// Get slide
    pub fn slide(&self) -> i64 {
        self.slide
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(all(test, not(target_os = "none")))]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_collector() {
        let mut collector = EntropyCollector::new(EntropyConfig::default());
        let result = collector.collect();

        // TSC should always work on x86_64
        #[cfg(target_arch = "x86_64")]
        {
            assert!(result.is_ok());
            assert!(collector.quality() >= EntropyQuality::Low);
        }
    }

    #[test]
    fn test_kaslr_alignment() {
        let config = KaslrConfig {
            alignment: 0x1000,
            range_start: 0x100000,
            range_end: 0x1000000,
            ..Default::default()
        };

        let mut kaslr = Kaslr::new(config);
        let slide = kaslr.initialize(0x10000);

        if let Ok(s) = slide {
            assert_eq!(s % 0x1000, 0, "Slide must be aligned");
        }
    }
}
