//! # KASLR - Kernel Address Space Layout Randomization
//!
//! This module provides secure, hardware-backed randomization for kernel load addresses.
//! KASLR is a security mechanism that makes exploitation more difficult by loading the
//! kernel at a random virtual address on each boot.
//!
//! ## Security Model
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                      KASLR SECURITY LAYERS                          │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │  Layer 1: Hardware Entropy (RDSEED/RDRAND)                         │
//! │  Layer 2: Firmware Entropy (UEFI RNG Protocol)                     │
//! │  Layer 3: Time-based Entropy (TSC jitter)                          │
//! │  Layer 4: Mixed Sources (combine all available)                    │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Address Space Layout
//!
//! The kernel can be loaded anywhere within the KASLR region:
//!
//! ```text
//! 0xFFFF_8000_0000_0000  ┌────────────────────┐
//!                        │  Direct Map        │
//! 0xFFFF_8800_0000_0000  ├────────────────────┤
//!                        │  KASLR Region      │ ◄── Kernel loaded here
//!                        │  (1GB, 2MB aligned)│     (256K possible slots)
//! 0xFFFF_C000_0000_0000  ├────────────────────┤
//!                        │  vmalloc/modules   │
//! 0xFFFF_FFFF_FFFF_FFFF  └────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,no_run
//! use hal::kaslr::{KaslrConfig, generate_kaslr_offset};
//!
//! let config = KaslrConfig::default();
//! let kernel_size = 0x200000; // 2MB
//!
//! let load_address = generate_kaslr_offset(&config, kernel_size)?;
//! println!("Loading kernel at: 0x{:016x}", load_address);
//! ```

#![allow(dead_code)]

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

// ============================================================================
// CONFIGURATION
// ============================================================================

/// KASLR configuration parameters
#[derive(Debug, Clone)]
pub struct KaslrConfig {
    /// Minimum kernel virtual address
    pub min_address: u64,
    /// Maximum kernel virtual address (exclusive)
    pub max_address: u64,
    /// Required alignment (2MB for huge pages)
    pub alignment: u64,
    /// Bits of entropy (higher = more randomness)
    pub entropy_bits: u8,
    /// Physical memory offset for direct mapping
    pub phys_offset: u64,
    /// Enable KASLR (can be disabled for debugging)
    pub enabled: bool,
}

impl Default for KaslrConfig {
    fn default() -> Self {
        Self {
            // Higher-half kernel region
            min_address: 0xFFFF_FFFF_8000_0000, // -2GB
            max_address: 0xFFFF_FFFF_C000_0000, // -1GB (1GB range)
            alignment: 0x20_0000,                // 2MB alignment
            entropy_bits: 18,                    // ~256K possible positions
            phys_offset: 0xFFFF_8000_0000_0000,  // Physical memory direct map
            enabled: true,
        }
    }
}

impl KaslrConfig {
    /// Create configuration for minimal kernel (smaller range)
    pub fn minimal() -> Self {
        Self {
            min_address: 0xFFFF_FFFF_8000_0000,
            max_address: 0xFFFF_FFFF_9000_0000, // 256MB range
            alignment: 0x20_0000,
            entropy_bits: 12, // ~4K positions
            ..Default::default()
        }
    }

    /// Create configuration with custom range
    pub fn with_range(min: u64, max: u64) -> Self {
        Self {
            min_address: min,
            max_address: max,
            ..Default::default()
        }
    }

    /// Disable KASLR (for debugging)
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Calculate number of possible slots
    pub fn num_slots(&self, kernel_size: u64) -> u64 {
        let usable_range = self.max_address.saturating_sub(self.min_address + kernel_size);
        usable_range / self.alignment
    }

    /// Calculate effective entropy bits
    pub fn effective_entropy(&self, kernel_size: u64) -> u8 {
        let slots = self.num_slots(kernel_size);
        if slots == 0 {
            return 0;
        }
        // log2(slots), capped at configured entropy_bits
        let bits = 64 - slots.leading_zeros();
        core::cmp::min(bits as u8, self.entropy_bits)
    }
}

// ============================================================================
// ENTROPY SOURCES
// ============================================================================

/// Available entropy sources
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntropySource {
    /// RDSEED instruction (hardware true RNG) - Best quality
    Rdseed,
    /// RDRAND instruction (hardware PRNG) - Good quality
    Rdrand,
    /// UEFI RNG Protocol - Firmware provided
    UefiRng,
    /// TSC (Time Stamp Counter) - Fallback, lower quality
    Tsc,
    /// Combined sources (mix all available)
    Mixed,
    /// Fixed value (for debugging/testing only)
    Fixed(u64),
}

/// Entropy quality rating
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntropyQuality {
    /// Unsuitable for security
    None = 0,
    /// Weak entropy (TSC, predictable)
    Weak = 1,
    /// Moderate entropy (firmware RNG)
    Moderate = 2,
    /// Strong entropy (RDRAND)
    Strong = 3,
    /// Cryptographic entropy (RDSEED)
    Cryptographic = 4,
}

impl EntropySource {
    /// Get the quality rating of this source
    pub fn quality(&self) -> EntropyQuality {
        match self {
            Self::Rdseed => EntropyQuality::Cryptographic,
            Self::Rdrand => EntropyQuality::Strong,
            Self::UefiRng => EntropyQuality::Moderate,
            Self::Tsc => EntropyQuality::Weak,
            Self::Mixed => EntropyQuality::Strong,
            Self::Fixed(_) => EntropyQuality::None,
        }
    }
}

// ============================================================================
// HARDWARE ENTROPY (x86_64)
// ============================================================================

/// Check if RDRAND is supported
#[cfg(target_arch = "x86_64")]
pub fn rdrand_supported() -> bool {
    // CPUID.01H:ECX.RDRAND[bit 30]
    let cpuid = unsafe { core::arch::x86_64::__cpuid(1) };
    (cpuid.ecx & (1 << 30)) != 0
}

/// Check if RDSEED is supported
#[cfg(target_arch = "x86_64")]
pub fn rdseed_supported() -> bool {
    // CPUID.(EAX=07H, ECX=0H):EBX.RDSEED[bit 18]
    let cpuid = unsafe { core::arch::x86_64::__cpuid_count(7, 0) };
    (cpuid.ebx & (1 << 18)) != 0
}

/// Get 64-bit random value from RDSEED (true RNG)
///
/// Returns `None` if RDSEED is not supported or fails.
#[cfg(target_arch = "x86_64")]
pub fn rdseed64() -> Option<u64> {
    if !rdseed_supported() {
        return None;
    }

    // Retry up to 10 times (RDSEED can fail under high demand)
    for _ in 0..10 {
        let value: u64;
        let success: u8;

        unsafe {
            core::arch::asm!(
                "rdseed {0}",
                "setc {1}",
                out(reg) value,
                out(reg_byte) success,
                options(nomem, nostack)
            );
        }

        if success != 0 {
            return Some(value);
        }

        // Small delay between retries
        core::hint::spin_loop();
    }

    None
}

/// Get 64-bit random value from RDRAND (PRNG)
///
/// Returns `None` if RDRAND is not supported or fails.
#[cfg(target_arch = "x86_64")]
pub fn rdrand64() -> Option<u64> {
    if !rdrand_supported() {
        return None;
    }

    // Retry up to 10 times
    for _ in 0..10 {
        let value: u64;
        let success: u8;

        unsafe {
            core::arch::asm!(
                "rdrand {0}",
                "setc {1}",
                out(reg) value,
                out(reg_byte) success,
                options(nomem, nostack)
            );
        }

        if success != 0 {
            return Some(value);
        }

        core::hint::spin_loop();
    }

    None
}

/// Read TSC (Time Stamp Counter)
///
/// Low-quality entropy, but always available.
#[cfg(target_arch = "x86_64")]
pub fn rdtsc() -> u64 {
    let lo: u32;
    let hi: u32;

    unsafe {
        core::arch::asm!(
            "rdtsc",
            out("eax") lo,
            out("edx") hi,
            options(nomem, nostack)
        );
    }

    ((hi as u64) << 32) | (lo as u64)
}

/// Read TSC with more precision (includes RDTSCP)
#[cfg(target_arch = "x86_64")]
pub fn rdtscp() -> (u64, u32) {
    let lo: u32;
    let hi: u32;
    let aux: u32;

    unsafe {
        core::arch::asm!(
            "rdtscp",
            out("eax") lo,
            out("edx") hi,
            out("ecx") aux,
            options(nomem, nostack)
        );
    }

    (((hi as u64) << 32) | (lo as u64), aux)
}

// Stubs for non-x86_64
#[cfg(not(target_arch = "x86_64"))]
pub fn rdrand_supported() -> bool {
    false
}
#[cfg(not(target_arch = "x86_64"))]
pub fn rdseed_supported() -> bool {
    false
}
#[cfg(not(target_arch = "x86_64"))]
pub fn rdseed64() -> Option<u64> {
    None
}
#[cfg(not(target_arch = "x86_64"))]
pub fn rdrand64() -> Option<u64> {
    None
}
#[cfg(not(target_arch = "x86_64"))]
pub fn rdtsc() -> u64 {
    0
}
#[cfg(not(target_arch = "x86_64"))]
pub fn rdtscp() -> (u64, u32) {
    (0, 0)
}

// ============================================================================
// ENTROPY MIXING
// ============================================================================

/// Simple entropy mixer using XORshift
struct EntropyMixer {
    state: u64,
}

impl EntropyMixer {
    /// Create mixer with initial seed
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x853c49e6748fea9b } else { seed },
        }
    }

    /// Mix in additional entropy
    fn mix(&mut self, entropy: u64) {
        self.state ^= entropy;
        self.state = self.state.wrapping_mul(0x2545F4914F6CDD1D);
        self.state ^= self.state >> 27;
    }

    /// Get final mixed value
    fn finalize(mut self) -> u64 {
        // Final mixing steps
        self.state ^= self.state >> 33;
        self.state = self.state.wrapping_mul(0xFF51AFD7ED558CCD);
        self.state ^= self.state >> 33;
        self.state = self.state.wrapping_mul(0xC4CEB9FE1A85EC53);
        self.state ^= self.state >> 33;
        self.state
    }
}

/// Collect entropy from all available sources
pub fn collect_entropy() -> (u64, EntropyQuality) {
    let mut mixer = EntropyMixer::new(0);
    let mut best_quality = EntropyQuality::None;

    // Try RDSEED first (best quality)
    if let Some(val) = rdseed64() {
        mixer.mix(val);
        best_quality = EntropyQuality::Cryptographic;
    }

    // Try RDRAND
    if let Some(val) = rdrand64() {
        mixer.mix(val);
        if best_quality < EntropyQuality::Strong {
            best_quality = EntropyQuality::Strong;
        }
    }

    // Always add TSC for additional mixing
    let tsc = rdtsc();
    mixer.mix(tsc);

    // If we got nothing good, at least use TSC
    if best_quality < EntropyQuality::Weak {
        best_quality = EntropyQuality::Weak;
    }

    (mixer.finalize(), best_quality)
}

/// Get entropy from a specific source
pub fn get_entropy(source: EntropySource) -> Option<u64> {
    match source {
        EntropySource::Rdseed => rdseed64(),
        EntropySource::Rdrand => rdrand64(),
        EntropySource::UefiRng => None, // Requires UEFI runtime
        EntropySource::Tsc => Some(rdtsc()),
        EntropySource::Mixed => Some(collect_entropy().0),
        EntropySource::Fixed(v) => Some(v),
    }
}

// ============================================================================
// KASLR GENERATION
// ============================================================================

/// Result type for KASLR operations
pub type KaslrResult<T> = Result<T, KaslrError>;

/// KASLR errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KaslrError {
    /// KASLR is disabled
    Disabled,
    /// Kernel too large for KASLR range
    KernelTooLarge,
    /// No entropy available
    NoEntropy,
    /// Insufficient entropy quality
    LowEntropy(EntropyQuality),
    /// Invalid configuration
    InvalidConfig,
    /// Alignment error
    AlignmentError,
}

/// KASLR result with details
#[derive(Debug, Clone)]
pub struct KaslrResult2 {
    /// Final load address
    pub load_address: u64,
    /// Slide from default address
    pub slide: i64,
    /// Entropy source used
    pub source: EntropySource,
    /// Entropy quality achieved
    pub quality: EntropyQuality,
    /// Random value used
    pub random_value: u64,
    /// Slot index selected
    pub slot_index: u64,
    /// Total available slots
    pub total_slots: u64,
}

/// Generate KASLR offset for kernel
///
/// Calculates a randomized load address for the kernel within the
/// configured KASLR region.
///
/// # Arguments
///
/// * `config` - KASLR configuration parameters
/// * `kernel_size` - Size of the kernel in bytes
///
/// # Returns
///
/// The randomized virtual load address on success
pub fn generate_kaslr_offset(config: &KaslrConfig, kernel_size: u64) -> KaslrResult<u64> {
    if !config.enabled {
        return Err(KaslrError::Disabled);
    }

    // Validate configuration
    if config.max_address <= config.min_address {
        return Err(KaslrError::InvalidConfig);
    }

    if config.alignment == 0 || !config.alignment.is_power_of_two() {
        return Err(KaslrError::AlignmentError);
    }

    // Align kernel size up
    let aligned_size = (kernel_size + config.alignment - 1) & !(config.alignment - 1);

    // Calculate usable range
    let range = config.max_address.saturating_sub(config.min_address);
    if aligned_size >= range {
        return Err(KaslrError::KernelTooLarge);
    }

    // Calculate number of slots
    let num_slots = (range - aligned_size) / config.alignment;
    if num_slots == 0 {
        return Err(KaslrError::KernelTooLarge);
    }

    // Get entropy
    let (random, quality) = collect_entropy();
    if quality < EntropyQuality::Weak {
        return Err(KaslrError::NoEntropy);
    }

    // Select slot
    let slot = random % num_slots;

    // Calculate final address
    let load_address = config.min_address + slot * config.alignment;

    // Verify alignment
    debug_assert!(load_address % config.alignment == 0);
    debug_assert!(load_address >= config.min_address);
    debug_assert!(load_address + aligned_size <= config.max_address);

    Ok(load_address)
}

/// Generate KASLR offset with detailed results
pub fn generate_kaslr_offset_detailed(
    config: &KaslrConfig,
    kernel_size: u64,
    default_address: u64,
) -> KaslrResult<KaslrResult2> {
    if !config.enabled {
        // Return default address with no slide
        return Ok(KaslrResult2 {
            load_address: default_address,
            slide: 0,
            source: EntropySource::Fixed(0),
            quality: EntropyQuality::None,
            random_value: 0,
            slot_index: 0,
            total_slots: 1,
        });
    }

    // Align kernel size
    let aligned_size = (kernel_size + config.alignment - 1) & !(config.alignment - 1);

    // Calculate range
    let range = config.max_address.saturating_sub(config.min_address);
    if aligned_size >= range {
        return Err(KaslrError::KernelTooLarge);
    }

    let num_slots = (range - aligned_size) / config.alignment;
    if num_slots == 0 {
        return Err(KaslrError::KernelTooLarge);
    }

    // Determine best entropy source
    let (random, source, quality) = if let Some(val) = rdseed64() {
        (val, EntropySource::Rdseed, EntropyQuality::Cryptographic)
    } else if let Some(val) = rdrand64() {
        (val, EntropySource::Rdrand, EntropyQuality::Strong)
    } else {
        let tsc = rdtsc();
        (tsc, EntropySource::Tsc, EntropyQuality::Weak)
    };

    // Select slot
    let slot = random % num_slots;
    let load_address = config.min_address + slot * config.alignment;
    let slide = (load_address as i128 - default_address as i128) as i64;

    Ok(KaslrResult2 {
        load_address,
        slide,
        source,
        quality,
        random_value: random,
        slot_index: slot,
        total_slots: num_slots,
    })
}

// ============================================================================
// GLOBAL STATE
// ============================================================================

/// Global KASLR state (initialized once at boot)
static KASLR_INITIALIZED: AtomicBool = AtomicBool::new(false);
static KASLR_ENABLED: AtomicBool = AtomicBool::new(false);
static KASLR_SLIDE: AtomicU64 = AtomicU64::new(0);
static KASLR_BASE: AtomicU64 = AtomicU64::new(0);

/// Initialize global KASLR state
///
/// This should be called once during early boot.
///
/// # Safety
///
/// Must only be called once, before any other KASLR functions.
pub fn init_kaslr(base_address: u64, slide: i64) {
    if KASLR_INITIALIZED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        KASLR_BASE.store(base_address, Ordering::SeqCst);
        KASLR_SLIDE.store(slide as u64, Ordering::SeqCst);
        KASLR_ENABLED.store(slide != 0, Ordering::SeqCst);
    }
}

/// Check if KASLR has been initialized
pub fn kaslr_initialized() -> bool {
    KASLR_INITIALIZED.load(Ordering::SeqCst)
}

/// Check if KASLR is active (slide != 0)
pub fn kaslr_active() -> bool {
    KASLR_ENABLED.load(Ordering::SeqCst)
}

/// Get the current KASLR slide
pub fn get_kaslr_slide() -> i64 {
    KASLR_SLIDE.load(Ordering::SeqCst) as i64
}

/// Get the KASLR base address
pub fn get_kaslr_base() -> u64 {
    KASLR_BASE.load(Ordering::SeqCst)
}

/// Translate a linked address to a runtime address
#[inline]
pub fn kaslr_translate(linked_addr: u64) -> u64 {
    let slide = get_kaslr_slide();
    (linked_addr as i128 + slide as i128) as u64
}

// ============================================================================
// BOOT PARAMETER
// ============================================================================

/// KASLR boot parameter for kernel command line
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KaslrBootParam {
    /// KASLR enabled (default)
    Enabled,
    /// KASLR disabled (nokaslr)
    Disabled,
    /// Use specific slide value
    FixedSlide(i64),
}

impl KaslrBootParam {
    /// Parse from kernel command line
    pub fn from_cmdline(cmdline: &str) -> Self {
        if cmdline.contains("nokaslr") {
            Self::Disabled
        } else if let Some(pos) = cmdline.find("kaslr_slide=") {
            // Parse kaslr_slide=0x1000 format
            let rest = &cmdline[pos + 12..];
            let end = rest.find(' ').unwrap_or(rest.len());
            let value_str = &rest[..end];

            let value = if value_str.starts_with("0x") || value_str.starts_with("0X") {
                i64::from_str_radix(&value_str[2..], 16).unwrap_or(0)
            } else {
                value_str.parse().unwrap_or(0)
            };

            if value != 0 {
                Self::FixedSlide(value)
            } else {
                Self::Enabled
            }
        } else {
            Self::Enabled
        }
    }
}

// ============================================================================
// DIAGNOSTICS
// ============================================================================

/// KASLR diagnostic information
#[derive(Debug, Clone)]
pub struct KaslrDiagnostics {
    pub rdrand_available: bool,
    pub rdseed_available: bool,
    pub entropy_quality: EntropyQuality,
    pub sample_random: u64,
    pub kaslr_active: bool,
    pub current_slide: i64,
    pub current_base: u64,
}

impl KaslrDiagnostics {
    /// Collect diagnostic information
    pub fn collect() -> Self {
        let (sample, quality) = collect_entropy();

        Self {
            rdrand_available: rdrand_supported(),
            rdseed_available: rdseed_supported(),
            entropy_quality: quality,
            sample_random: sample,
            kaslr_active: kaslr_active(),
            current_slide: get_kaslr_slide(),
            current_base: get_kaslr_base(),
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = KaslrConfig::default();
        assert!(config.enabled);
        assert!(config.alignment.is_power_of_two());
        assert!(config.max_address > config.min_address);
    }

    #[test]
    fn test_slot_calculation() {
        let config = KaslrConfig::default();
        let kernel_size = 0x200000; // 2MB
        let slots = config.num_slots(kernel_size);
        assert!(slots > 0);
    }

    #[test]
    fn test_effective_entropy() {
        let config = KaslrConfig::default();
        let bits = config.effective_entropy(0x200000);
        assert!(bits > 0);
        assert!(bits <= config.entropy_bits);
    }

    #[test]
    fn test_entropy_quality_order() {
        assert!(EntropyQuality::Cryptographic > EntropyQuality::Strong);
        assert!(EntropyQuality::Strong > EntropyQuality::Moderate);
        assert!(EntropyQuality::Moderate > EntropyQuality::Weak);
        assert!(EntropyQuality::Weak > EntropyQuality::None);
    }

    #[test]
    fn test_mixer() {
        let mut mixer = EntropyMixer::new(12345);
        mixer.mix(0xDEADBEEF);
        mixer.mix(0xCAFEBABE);
        let result = mixer.finalize();
        assert_ne!(result, 0);
        assert_ne!(result, 12345);
    }

    #[test]
    fn test_boot_param_parsing() {
        assert_eq!(
            KaslrBootParam::from_cmdline("nokaslr quiet"),
            KaslrBootParam::Disabled
        );
        assert_eq!(
            KaslrBootParam::from_cmdline("quiet"),
            KaslrBootParam::Enabled
        );
        assert_eq!(
            KaslrBootParam::from_cmdline("kaslr_slide=0x1000"),
            KaslrBootParam::FixedSlide(0x1000)
        );
    }

    #[test]
    fn test_kaslr_disabled() {
        let config = KaslrConfig::disabled();
        assert!(!config.enabled);
        let result = generate_kaslr_offset(&config, 0x200000);
        assert!(matches!(result, Err(KaslrError::Disabled)));
    }

    #[test]
    fn test_kernel_too_large() {
        let config = KaslrConfig {
            min_address: 0x1000,
            max_address: 0x2000,
            ..Default::default()
        };
        let result = generate_kaslr_offset(&config, 0x10000);
        assert!(matches!(result, Err(KaslrError::KernelTooLarge)));
    }
}
