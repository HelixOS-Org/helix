//! RNG Protocol
//!
//! High-level random number generation abstraction.

use super::Protocol;
use crate::error::{Error, Result};
use crate::raw::types::*;

extern crate alloc;
use alloc::vec::Vec;

// =============================================================================
// RNG PROTOCOL
// =============================================================================

/// Random Number Generator protocol
pub struct Rng {
    /// Handle
    handle: Handle,
    /// Supported algorithms
    algorithms: Vec<RngAlgorithm>,
    /// Default algorithm
    default_algorithm: RngAlgorithm,
}

impl Rng {
    /// Create new RNG accessor
    pub fn new(handle: Handle) -> Self {
        Self {
            handle,
            algorithms: Vec::new(),
            default_algorithm: RngAlgorithm::Raw,
        }
    }

    /// Get supported algorithms
    pub fn algorithms(&self) -> &[RngAlgorithm] {
        &self.algorithms
    }

    /// Get default algorithm
    pub fn default_algorithm(&self) -> RngAlgorithm {
        self.default_algorithm
    }

    /// Check if algorithm is supported
    pub fn supports(&self, algorithm: RngAlgorithm) -> bool {
        self.algorithms.contains(&algorithm)
    }

    /// Get random bytes with default algorithm
    pub fn get_bytes(&self, buffer: &mut [u8]) -> Result<()> {
        self.get_bytes_with(self.default_algorithm, buffer)
    }

    /// Get random bytes with specific algorithm
    pub fn get_bytes_with(&self, _algorithm: RngAlgorithm, buffer: &mut [u8]) -> Result<()> {
        // TODO: Implement actual RNG call
        // For now, use a simple LFSR-based fallback
        let mut state = 0xDEADBEEFu64;

        for byte in buffer.iter_mut() {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            *byte = (state >> 33) as u8;
        }

        Ok(())
    }

    /// Get single random u8
    pub fn u8(&self) -> Result<u8> {
        let mut buf = [0u8; 1];
        self.get_bytes(&mut buf)?;
        Ok(buf[0])
    }

    /// Get single random u16
    pub fn u16(&self) -> Result<u16> {
        let mut buf = [0u8; 2];
        self.get_bytes(&mut buf)?;
        Ok(u16::from_ne_bytes(buf))
    }

    /// Get single random u32
    pub fn u32(&self) -> Result<u32> {
        let mut buf = [0u8; 4];
        self.get_bytes(&mut buf)?;
        Ok(u32::from_ne_bytes(buf))
    }

    /// Get single random u64
    pub fn u64(&self) -> Result<u64> {
        let mut buf = [0u8; 8];
        self.get_bytes(&mut buf)?;
        Ok(u64::from_ne_bytes(buf))
    }

    /// Get single random u128
    pub fn u128(&self) -> Result<u128> {
        let mut buf = [0u8; 16];
        self.get_bytes(&mut buf)?;
        Ok(u128::from_ne_bytes(buf))
    }

    /// Get single random usize
    pub fn usize(&self) -> Result<usize> {
        let mut buf = [0u8; core::mem::size_of::<usize>()];
        self.get_bytes(&mut buf)?;
        Ok(usize::from_ne_bytes(buf))
    }

    /// Get random value in range [0, max)
    pub fn range(&self, max: u64) -> Result<u64> {
        if max == 0 {
            return Err(Error::InvalidParameter);
        }

        // Use rejection sampling for unbiased results
        let bits_needed = 64 - max.leading_zeros();
        let mask = (1u64 << bits_needed) - 1;

        loop {
            let value = self.u64()? & mask;
            if value < max {
                return Ok(value);
            }
        }
    }

    /// Get random value in range [min, max)
    pub fn range_between(&self, min: u64, max: u64) -> Result<u64> {
        if min >= max {
            return Err(Error::InvalidParameter);
        }

        Ok(min + self.range(max - min)?)
    }

    /// Get random boolean
    pub fn bool(&self) -> Result<bool> {
        Ok((self.u8()? & 1) != 0)
    }

    /// Get random boolean with probability p (0.0 to 1.0)
    pub fn bool_with_probability(&self, p: f64) -> Result<bool> {
        if p <= 0.0 {
            return Ok(false);
        }
        if p >= 1.0 {
            return Ok(true);
        }

        let threshold = (p * u64::MAX as f64) as u64;
        Ok(self.u64()? < threshold)
    }

    /// Fill buffer with random bytes
    pub fn fill(&self, buffer: &mut [u8]) -> Result<()> {
        self.get_bytes(buffer)
    }

    /// Get Vec of random bytes
    pub fn bytes(&self, count: usize) -> Result<Vec<u8>> {
        let mut buffer = alloc::vec![0u8; count];
        self.get_bytes(&mut buffer)?;
        Ok(buffer)
    }

    /// Shuffle slice in place (Fisher-Yates shuffle)
    pub fn shuffle<T>(&self, slice: &mut [T]) -> Result<()> {
        let len = slice.len();
        if len <= 1 {
            return Ok(());
        }

        for i in (1..len).rev() {
            let j = self.range((i + 1) as u64)? as usize;
            slice.swap(i, j);
        }

        Ok(())
    }

    /// Choose random element from slice
    pub fn choose<'a, T>(&self, slice: &'a [T]) -> Result<Option<&'a T>> {
        if slice.is_empty() {
            return Ok(None);
        }

        let index = self.range(slice.len() as u64)? as usize;
        Ok(Some(&slice[index]))
    }

    /// Generate UUID v4 (random)
    pub fn uuid_v4(&self) -> Result<[u8; 16]> {
        let mut uuid = [0u8; 16];
        self.get_bytes(&mut uuid)?;

        // Set version to 4
        uuid[6] = (uuid[6] & 0x0F) | 0x40;
        // Set variant to RFC 4122
        uuid[8] = (uuid[8] & 0x3F) | 0x80;

        Ok(uuid)
    }

    /// Format UUID as string
    pub fn uuid_v4_string(&self) -> Result<alloc::string::String> {
        let uuid = self.uuid_v4()?;
        Ok(alloc::format!(
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            uuid[0], uuid[1], uuid[2], uuid[3],
            uuid[4], uuid[5],
            uuid[6], uuid[7],
            uuid[8], uuid[9],
            uuid[10], uuid[11], uuid[12], uuid[13], uuid[14], uuid[15]
        ))
    }
}

impl Protocol for Rng {
    const GUID: Guid = Guid::new(0x3152BCA5, 0xEADE, 0x433D, [
        0x86, 0x2E, 0xC0, 0x1C, 0xDC, 0x29, 0x1F, 0x44,
    ]);

    fn open(handle: Handle) -> Result<Self> {
        Ok(Self::new(handle))
    }
}

// =============================================================================
// RNG ALGORITHM
// =============================================================================

/// RNG algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RngAlgorithm {
    /// Raw entropy (unprocessed)
    Raw,
    /// SP800-90 Hash DRBG (SHA-256)
    Sp80090HashDrbgSha256,
    /// SP800-90 HMAC DRBG (SHA-256)
    Sp80090HmacDrbgSha256,
    /// SP800-90 CTR DRBG (AES-256)
    Sp80090CtrDrbgAes256,
    /// Intel RDRAND
    Rdrand,
    /// ARM RNDR
    ArmRndr,
    /// Unknown algorithm
    Unknown(Guid),
}

impl RngAlgorithm {
    /// Get algorithm GUID
    pub fn guid(&self) -> Guid {
        match self {
            Self::Raw => rng_algorithms::EFI_RNG_ALGORITHM_RAW,
            Self::Sp80090HashDrbgSha256 => rng_algorithms::EFI_RNG_ALGORITHM_SP800_90_HASH_256_GUID,
            Self::Sp80090HmacDrbgSha256 => rng_algorithms::EFI_RNG_ALGORITHM_SP800_90_HMAC_256_GUID,
            Self::Sp80090CtrDrbgAes256 => rng_algorithms::EFI_RNG_ALGORITHM_SP800_90_CTR_256_GUID,
            Self::Rdrand => rng_algorithms::EFI_RNG_ALGORITHM_X9_31_AES_GUID,
            Self::ArmRndr => rng_algorithms::EFI_RNG_ALGORITHM_ARM_RNDR_GUID,
            Self::Unknown(guid) => *guid,
        }
    }

    /// Create from GUID
    pub fn from_guid(guid: Guid) -> Self {
        if guid == rng_algorithms::EFI_RNG_ALGORITHM_RAW {
            Self::Raw
        } else if guid == rng_algorithms::EFI_RNG_ALGORITHM_SP800_90_HASH_256_GUID {
            Self::Sp80090HashDrbgSha256
        } else if guid == rng_algorithms::EFI_RNG_ALGORITHM_SP800_90_HMAC_256_GUID {
            Self::Sp80090HmacDrbgSha256
        } else if guid == rng_algorithms::EFI_RNG_ALGORITHM_SP800_90_CTR_256_GUID {
            Self::Sp80090CtrDrbgAes256
        } else if guid == rng_algorithms::EFI_RNG_ALGORITHM_X9_31_AES_GUID {
            Self::Rdrand
        } else if guid == rng_algorithms::EFI_RNG_ALGORITHM_ARM_RNDR_GUID {
            Self::ArmRndr
        } else {
            Self::Unknown(guid)
        }
    }

    /// Get algorithm name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Raw => "Raw Entropy",
            Self::Sp80090HashDrbgSha256 => "SP800-90 Hash DRBG (SHA-256)",
            Self::Sp80090HmacDrbgSha256 => "SP800-90 HMAC DRBG (SHA-256)",
            Self::Sp80090CtrDrbgAes256 => "SP800-90 CTR DRBG (AES-256)",
            Self::Rdrand => "Intel RDRAND",
            Self::ArmRndr => "ARM RNDR",
            Self::Unknown(_) => "Unknown",
        }
    }

    /// Check if algorithm is a DRBG
    pub fn is_drbg(&self) -> bool {
        matches!(
            self,
            Self::Sp80090HashDrbgSha256 | Self::Sp80090HmacDrbgSha256 | Self::Sp80090CtrDrbgAes256
        )
    }

    /// Check if algorithm is hardware-based
    pub fn is_hardware(&self) -> bool {
        matches!(self, Self::Raw | Self::Rdrand | Self::ArmRndr)
    }
}

// =============================================================================
// RNG ALGORITHM GUIDS
// =============================================================================

/// RNG algorithm GUIDs
pub mod rng_algorithms {
    use super::*;

    /// Raw entropy
    pub const EFI_RNG_ALGORITHM_RAW: Guid = Guid::new(0xE43176D7, 0xB6E8, 0x4827, [
        0xB7, 0x84, 0x7F, 0xFD, 0xC4, 0xB6, 0x85, 0x61,
    ]);

    /// SP800-90 Hash DRBG (SHA-256)
    pub const EFI_RNG_ALGORITHM_SP800_90_HASH_256_GUID: Guid =
        Guid::new(0xA7AF67CB, 0x603B, 0x4D42, [
            0xBA, 0x21, 0x70, 0xBF, 0xB6, 0x29, 0x3F, 0x96,
        ]);

    /// SP800-90 HMAC DRBG (SHA-256)
    pub const EFI_RNG_ALGORITHM_SP800_90_HMAC_256_GUID: Guid =
        Guid::new(0xC5149B43, 0xAE85, 0x4F53, [
            0x99, 0x82, 0xB9, 0x43, 0x35, 0xD3, 0xA9, 0xE7,
        ]);

    /// SP800-90 CTR DRBG (AES-256)
    pub const EFI_RNG_ALGORITHM_SP800_90_CTR_256_GUID: Guid =
        Guid::new(0x44F0DE6E, 0x4D8C, 0x4045, [
            0xA8, 0xC7, 0x4D, 0xD1, 0x68, 0x85, 0x6B, 0x9E,
        ]);

    /// X9.31-3DES (often used for RDRAND)
    pub const EFI_RNG_ALGORITHM_X9_31_AES_GUID: Guid = Guid::new(0x63C4785A, 0xCA34, 0x4012, [
        0xA3, 0xC8, 0x0B, 0x6A, 0x32, 0x4F, 0x55, 0x46,
    ]);

    /// ARM RNDR
    pub const EFI_RNG_ALGORITHM_ARM_RNDR_GUID: Guid = Guid::new(0x43D2FDE3, 0x9D4E, 0x4D79, [
        0xB5, 0xEC, 0x48, 0x7F, 0x53, 0x8B, 0x2F, 0x5E,
    ]);
}

// =============================================================================
// ENTROPY SOURCE
// =============================================================================

/// Entropy source information
#[derive(Debug, Clone)]
pub struct EntropySource {
    /// Source name
    pub name: &'static str,
    /// Source type
    pub source_type: EntropySourceType,
    /// Entropy rate (bits per sample)
    pub entropy_rate: f64,
    /// Available
    pub available: bool,
}

/// Entropy source type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntropySourceType {
    /// Hardware RNG (e.g., RDRAND, RDSEED)
    Hardware,
    /// TPM
    Tpm,
    /// Timing jitter
    TimingJitter,
    /// Interrupt timing
    InterruptTiming,
    /// User input
    UserInput,
    /// Platform-specific
    Platform,
    /// Unknown
    Unknown,
}

impl EntropySourceType {
    /// Get source type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Hardware => "Hardware RNG",
            Self::Tpm => "TPM",
            Self::TimingJitter => "Timing Jitter",
            Self::InterruptTiming => "Interrupt Timing",
            Self::UserInput => "User Input",
            Self::Platform => "Platform-specific",
            Self::Unknown => "Unknown",
        }
    }

    /// Check if source is considered high-quality
    pub fn is_high_quality(&self) -> bool {
        matches!(self, Self::Hardware | Self::Tpm)
    }
}

// =============================================================================
// SEED GENERATOR
// =============================================================================

/// Seed generator for PRNGs
pub struct SeedGenerator<'a> {
    /// RNG reference
    rng: &'a Rng,
}

impl<'a> SeedGenerator<'a> {
    /// Create new seed generator
    pub fn new(rng: &'a Rng) -> Self {
        Self { rng }
    }

    /// Generate seed for ChaCha20
    pub fn chacha20_seed(&self) -> Result<[u8; 32]> {
        let mut seed = [0u8; 32];
        self.rng.get_bytes(&mut seed)?;
        Ok(seed)
    }

    /// Generate seed for Xorshift128+
    pub fn xorshift128plus_seed(&self) -> Result<[u64; 2]> {
        Ok([self.rng.u64()?, self.rng.u64()?])
    }

    /// Generate seed for PCG
    pub fn pcg_seed(&self) -> Result<(u64, u64)> {
        Ok((self.rng.u64()?, self.rng.u64()?))
    }

    /// Generate seed for MT19937
    pub fn mt19937_seed(&self) -> Result<u32> {
        self.rng.u32()
    }

    /// Generate seed for MT19937-64
    pub fn mt19937_64_seed(&self) -> Result<u64> {
        self.rng.u64()
    }

    /// Generate seed array for MT19937
    pub fn mt19937_seed_array(&self) -> Result<[u32; 624]> {
        let mut seeds = [0u32; 624];
        for seed in seeds.iter_mut() {
            *seed = self.rng.u32()?;
        }
        Ok(seeds)
    }
}

// =============================================================================
// SIMPLE PRNG (for fallback)
// =============================================================================

/// Simple PRNG (Xorshift64)
pub struct SimplePrng {
    state: u64,
}

impl SimplePrng {
    /// Create new PRNG with seed
    pub const fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0xDEADBEEF12345678 } else { seed },
        }
    }

    /// Seed from RNG
    pub fn seed_from_rng(rng: &Rng) -> Result<Self> {
        Ok(Self::new(rng.u64()?))
    }

    /// Get next u64
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Get next u32
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Get next value in range [0, max)
    pub fn next_range(&mut self, max: u64) -> u64 {
        if max == 0 {
            return 0;
        }

        let bits_needed = 64 - max.leading_zeros();
        let mask = (1u64 << bits_needed) - 1;

        loop {
            let value = self.next_u64() & mask;
            if value < max {
                return value;
            }
        }
    }

    /// Fill buffer with random bytes
    pub fn fill(&mut self, buffer: &mut [u8]) {
        let mut remaining = buffer.len();
        let mut offset = 0;

        while remaining > 0 {
            let value = self.next_u64();
            let bytes = value.to_ne_bytes();
            let to_copy = remaining.min(8);

            buffer[offset..offset + to_copy].copy_from_slice(&bytes[..to_copy]);
            offset += to_copy;
            remaining -= to_copy;
        }
    }
}

// =============================================================================
// PCG32 PRNG
// =============================================================================

/// PCG32 PRNG
pub struct Pcg32 {
    state: u64,
    increment: u64,
}

impl Pcg32 {
    /// Create new PCG32 with seed and stream
    pub fn new(seed: u64, stream: u64) -> Self {
        let mut rng = Self {
            state: 0,
            increment: (stream << 1) | 1,
        };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }

    /// Seed from RNG
    pub fn seed_from_rng(rng: &Rng) -> Result<Self> {
        Ok(Self::new(rng.u64()?, rng.u64()?))
    }

    /// Get next u32
    pub fn next_u32(&mut self) -> u32 {
        let old_state = self.state;
        self.state = old_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(self.increment);

        let xor_shifted = (((old_state >> 18) ^ old_state) >> 27) as u32;
        let rot = (old_state >> 59) as u32;

        xor_shifted.rotate_right(rot)
    }

    /// Get next u64
    pub fn next_u64(&mut self) -> u64 {
        ((self.next_u32() as u64) << 32) | (self.next_u32() as u64)
    }

    /// Get next value in range [0, max)
    pub fn next_range(&mut self, max: u32) -> u32 {
        if max == 0 {
            return 0;
        }

        let threshold = max.wrapping_neg() % max;

        loop {
            let value = self.next_u32();
            if value >= threshold {
                return value % max;
            }
        }
    }

    /// Fill buffer with random bytes
    pub fn fill(&mut self, buffer: &mut [u8]) {
        let mut remaining = buffer.len();
        let mut offset = 0;

        while remaining > 0 {
            let value = self.next_u32();
            let bytes = value.to_ne_bytes();
            let to_copy = remaining.min(4);

            buffer[offset..offset + to_copy].copy_from_slice(&bytes[..to_copy]);
            offset += to_copy;
            remaining -= to_copy;
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_rng_algorithm() {
        assert_eq!(RngAlgorithm::Raw.name(), "Raw Entropy");
        assert!(RngAlgorithm::Sp80090HashDrbgSha256.is_drbg());
        assert!(RngAlgorithm::Rdrand.is_hardware());
        assert!(!RngAlgorithm::Sp80090CtrDrbgAes256.is_hardware());
    }

    #[test]
    fn test_simple_prng() {
        let mut rng = SimplePrng::new(12345);

        let a = rng.next_u64();
        let b = rng.next_u64();
        assert_ne!(a, b);

        // Same seed should give same sequence
        let mut rng2 = SimplePrng::new(12345);
        assert_eq!(rng2.next_u64(), a);
        assert_eq!(rng2.next_u64(), b);
    }

    #[test]
    fn test_pcg32() {
        let mut rng = Pcg32::new(42, 54);

        let a = rng.next_u32();
        let b = rng.next_u32();
        assert_ne!(a, b);

        // Same seed should give same sequence
        let mut rng2 = Pcg32::new(42, 54);
        assert_eq!(rng2.next_u32(), a);
        assert_eq!(rng2.next_u32(), b);
    }

    #[test]
    fn test_prng_fill() {
        let mut rng = SimplePrng::new(12345);
        let mut buf = [0u8; 100];
        rng.fill(&mut buf);

        // Should not be all zeros
        assert!(buf.iter().any(|&b| b != 0));
    }

    #[test]
    fn test_entropy_source_type() {
        assert!(EntropySourceType::Hardware.is_high_quality());
        assert!(EntropySourceType::Tpm.is_high_quality());
        assert!(!EntropySourceType::TimingJitter.is_high_quality());
    }
}
