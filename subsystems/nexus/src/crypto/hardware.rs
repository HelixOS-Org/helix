//! Hardware crypto detection.

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

// ============================================================================
// HARDWARE CRYPTO FEATURES
// ============================================================================

/// Hardware crypto feature
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HwCryptoFeature {
    /// AES-NI (x86)
    AesNi,
    /// PCLMUL (x86)
    Pclmul,
    /// SHA extensions (x86)
    ShaNi,
    /// AVX
    Avx,
    /// AVX2
    Avx2,
    /// AVX512
    Avx512,
    /// ARM Crypto extensions
    ArmCe,
    /// ARM NEON
    ArmNeon,
    /// ARM SHA extensions
    ArmSha,
    /// RISC-V Crypto
    RiscvCrypto,
    /// TPM
    Tpm,
    /// Hardware RNG
    HwRng,
}

impl HwCryptoFeature {
    /// Get feature name
    pub fn name(&self) -> &'static str {
        match self {
            Self::AesNi => "aes-ni",
            Self::Pclmul => "pclmul",
            Self::ShaNi => "sha-ni",
            Self::Avx => "avx",
            Self::Avx2 => "avx2",
            Self::Avx512 => "avx512",
            Self::ArmCe => "arm-ce",
            Self::ArmNeon => "arm-neon",
            Self::ArmSha => "arm-sha",
            Self::RiscvCrypto => "riscv-crypto",
            Self::Tpm => "tpm",
            Self::HwRng => "hwrng",
        }
    }
}

// ============================================================================
// HARDWARE CRYPTO DETECTOR
// ============================================================================

/// Hardware crypto detector
pub struct HwCryptoDetector {
    /// Detected features
    features: Vec<HwCryptoFeature>,
    /// Checked
    checked: AtomicBool,
}

impl HwCryptoDetector {
    /// Create new detector
    pub fn new() -> Self {
        Self {
            features: Vec::new(),
            checked: AtomicBool::new(false),
        }
    }

    /// Add detected feature
    #[inline]
    pub fn add_feature(&mut self, feature: HwCryptoFeature) {
        if !self.features.contains(&feature) {
            self.features.push(feature);
        }
    }

    /// Check if feature is available
    #[inline(always)]
    pub fn has_feature(&self, feature: HwCryptoFeature) -> bool {
        self.features.contains(&feature)
    }

    /// Has AES acceleration
    #[inline(always)]
    pub fn has_aes_acceleration(&self) -> bool {
        self.has_feature(HwCryptoFeature::AesNi) || self.has_feature(HwCryptoFeature::ArmCe)
    }

    /// Has SHA acceleration
    #[inline(always)]
    pub fn has_sha_acceleration(&self) -> bool {
        self.has_feature(HwCryptoFeature::ShaNi) || self.has_feature(HwCryptoFeature::ArmSha)
    }

    /// Get all features
    #[inline(always)]
    pub fn features(&self) -> &[HwCryptoFeature] {
        &self.features
    }

    /// Mark as checked
    #[inline(always)]
    pub fn mark_checked(&self) {
        self.checked.store(true, Ordering::Relaxed);
    }

    /// Is checked
    #[inline(always)]
    pub fn is_checked(&self) -> bool {
        self.checked.load(Ordering::Relaxed)
    }
}

impl Default for HwCryptoDetector {
    fn default() -> Self {
        Self::new()
    }
}
