//! Random Number Generator Protocol
//!
//! Provides cryptographically secure random numbers.

use core::fmt;

use crate::raw::types::*;

// =============================================================================
// RNG PROTOCOL
// =============================================================================

/// Random Number Generator Protocol
#[repr(C)]
pub struct EfiRngProtocol {
    /// Get RNG algorithm information
    pub get_info: unsafe extern "efiapi" fn(
        this: *mut Self,
        rng_algorithm_list_size: *mut usize,
        rng_algorithm_list: *mut Guid,
    ) -> Status,

    /// Get random number
    pub get_rng: unsafe extern "efiapi" fn(
        this: *mut Self,
        rng_algorithm: *const Guid,
        rng_value_length: usize,
        rng_value: *mut u8,
    ) -> Status,
}

impl EfiRngProtocol {
    /// Protocol GUID
    pub const GUID: Guid = guids::RNG_PROTOCOL;

    /// Get supported algorithms
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn get_supported_algorithms(&self, buffer: &mut [Guid]) -> Result<usize, Status> {
        let mut size = buffer.len() * core::mem::size_of::<Guid>();
        let status = (self.get_info)(self as *const _ as *mut _, &mut size, buffer.as_mut_ptr());

        match status {
            Status::SUCCESS => Ok(size / core::mem::size_of::<Guid>()),
            Status::BUFFER_TOO_SMALL => Ok(size / core::mem::size_of::<Guid>()),
            _ => Err(status),
        }
    }

    /// Get random bytes using default algorithm
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer and buffer are valid.
    pub unsafe fn get_random_bytes(&self, buffer: &mut [u8]) -> Result<(), Status> {
        let status = (self.get_rng)(
            self as *const _ as *mut _,
            core::ptr::null(),
            buffer.len(),
            buffer.as_mut_ptr(),
        );
        status.to_status_result()
    }

    /// Get random bytes using specified algorithm
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer and buffer are valid.
    pub unsafe fn get_random_bytes_with_algorithm(
        &self,
        algorithm: &Guid,
        buffer: &mut [u8],
    ) -> Result<(), Status> {
        let status = (self.get_rng)(
            self as *const _ as *mut _,
            algorithm,
            buffer.len(),
            buffer.as_mut_ptr(),
        );
        status.to_status_result()
    }

    /// Get a single random u8
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn get_u8(&self) -> Result<u8, Status> {
        let mut value = 0u8;
        self.get_random_bytes(core::slice::from_mut(&mut value))?;
        Ok(value)
    }

    /// Get a single random u16
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn get_u16(&self) -> Result<u16, Status> {
        let mut bytes = [0u8; 2];
        self.get_random_bytes(&mut bytes)?;
        Ok(u16::from_le_bytes(bytes))
    }

    /// Get a single random u32
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn get_u32(&self) -> Result<u32, Status> {
        let mut bytes = [0u8; 4];
        self.get_random_bytes(&mut bytes)?;
        Ok(u32::from_le_bytes(bytes))
    }

    /// Get a single random u64
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn get_u64(&self) -> Result<u64, Status> {
        let mut bytes = [0u8; 8];
        self.get_random_bytes(&mut bytes)?;
        Ok(u64::from_le_bytes(bytes))
    }

    /// Get a single random u128
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn get_u128(&self) -> Result<u128, Status> {
        let mut bytes = [0u8; 16];
        self.get_random_bytes(&mut bytes)?;
        Ok(u128::from_le_bytes(bytes))
    }

    /// Generate a random GUID
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn generate_guid(&self) -> Result<Guid, Status> {
        let mut bytes = [0u8; 16];
        self.get_random_bytes(&mut bytes)?;

        // Set version (4) and variant (RFC 4122)
        bytes[6] = (bytes[6] & 0x0F) | 0x40; // Version 4
        bytes[8] = (bytes[8] & 0x3F) | 0x80; // Variant RFC 4122

        Ok(Guid::from_bytes(bytes))
    }

    /// Fill buffer with random bytes (convenience wrapper)
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn fill(&self, buffer: &mut [u8]) -> Result<(), Status> {
        self.get_random_bytes(buffer)
    }
}

impl fmt::Debug for EfiRngProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EfiRngProtocol").finish()
    }
}

// =============================================================================
// RNG ALGORITHMS
// =============================================================================

/// RNG algorithm GUIDs
pub mod rng_algorithm {
    use super::*;

    /// Raw RDRAND/RDSEED output (x86)
    pub const RAW: Guid = Guid::new(0xE43176D7, 0xB6E8, 0x4827, [
        0xB7, 0x84, 0x7F, 0xFD, 0xC4, 0xB6, 0x85, 0x61,
    ]);

    /// SP800-90 Hash DRBG
    pub const SP800_90_HASH_256_DRBG: Guid = Guid::new(0xA7AF67CB, 0x603B, 0x4D42, [
        0xBA, 0x21, 0x70, 0xBF, 0xB6, 0x29, 0x3F, 0x96,
    ]);

    /// SP800-90 HMAC DRBG
    pub const SP800_90_HMAC_256_DRBG: Guid = Guid::new(0xC5149B43, 0xAE85, 0x4F53, [
        0x99, 0x82, 0xB9, 0x43, 0x35, 0xD3, 0xA9, 0xE7,
    ]);

    /// SP800-90 CTR DRBG
    pub const SP800_90_CTR_256_DRBG: Guid = Guid::new(0x44F0DE6E, 0x4D8C, 0x4045, [
        0xA8, 0xC7, 0x4D, 0xD1, 0x68, 0x85, 0x6B, 0x9E,
    ]);

    /// X9.31 RNG using 3DES
    pub const X9_31_3DES: Guid = Guid::new(0x63C4785A, 0xCA34, 0x4012, [
        0xA3, 0xC8, 0x0B, 0x6A, 0x32, 0x4F, 0x55, 0x46,
    ]);

    /// X9.31 RNG using AES
    pub const X9_31_AES: Guid = Guid::new(0xACD03321, 0x777E, 0x4D3D, [
        0xB1, 0xC8, 0x20, 0xCF, 0xD8, 0x82, 0x20, 0xE9,
    ]);

    /// ARM RNDR instruction
    pub const ARM_RNDR: Guid = Guid::new(0x43D2FDE3, 0x9D4E, 0x4D79, [
        0xB5, 0xEC, 0xA0, 0xB1, 0xA3, 0xC7, 0x84, 0x29,
    ]);
}

// =============================================================================
// RNG ALGORITHM INFO
// =============================================================================

/// RNG algorithm information
#[derive(Debug, Clone, Copy)]
pub struct RngAlgorithmInfo {
    /// Algorithm GUID
    pub algorithm: Guid,
    /// Algorithm name
    pub name: &'static str,
    /// Description
    pub description: &'static str,
}

impl RngAlgorithmInfo {
    /// Get info for known algorithm
    pub fn from_guid(guid: &Guid) -> Option<Self> {
        if *guid == rng_algorithm::RAW {
            Some(Self {
                algorithm: *guid,
                name: "RAW",
                description: "Raw hardware RNG (RDRAND/RDSEED)",
            })
        } else if *guid == rng_algorithm::SP800_90_HASH_256_DRBG {
            Some(Self {
                algorithm: *guid,
                name: "SP800-90-HASH-256",
                description: "NIST SP800-90 Hash DRBG with SHA-256",
            })
        } else if *guid == rng_algorithm::SP800_90_HMAC_256_DRBG {
            Some(Self {
                algorithm: *guid,
                name: "SP800-90-HMAC-256",
                description: "NIST SP800-90 HMAC DRBG with SHA-256",
            })
        } else if *guid == rng_algorithm::SP800_90_CTR_256_DRBG {
            Some(Self {
                algorithm: *guid,
                name: "SP800-90-CTR-256",
                description: "NIST SP800-90 CTR DRBG with AES-256",
            })
        } else if *guid == rng_algorithm::X9_31_3DES {
            Some(Self {
                algorithm: *guid,
                name: "X9.31-3DES",
                description: "ANSI X9.31 RNG using 3DES",
            })
        } else if *guid == rng_algorithm::X9_31_AES {
            Some(Self {
                algorithm: *guid,
                name: "X9.31-AES",
                description: "ANSI X9.31 RNG using AES",
            })
        } else if *guid == rng_algorithm::ARM_RNDR {
            Some(Self {
                algorithm: *guid,
                name: "ARM-RNDR",
                description: "ARM RNDR instruction",
            })
        } else {
            None
        }
    }
}

// =============================================================================
// HASH PROTOCOL (for additional entropy)
// =============================================================================

/// Hash Protocol
#[repr(C)]
pub struct EfiHashProtocol {
    /// Get hash size
    pub get_hash_size: unsafe extern "efiapi" fn(
        this: *mut Self,
        hash_algorithm: *const Guid,
        hash_size: *mut usize,
    ) -> Status,

    /// Calculate hash
    pub hash: unsafe extern "efiapi" fn(
        this: *mut Self,
        hash_algorithm: *const Guid,
        extend: bool,
        message: *const u8,
        message_size: u64,
        hash_output: *mut Efi2HashOutput,
    ) -> Status,
}

impl EfiHashProtocol {
    /// Protocol GUID
    pub const GUID: Guid = guids::HASH_PROTOCOL;

    /// Get hash size for algorithm
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn get_size(&self, algorithm: &Guid) -> Result<usize, Status> {
        let mut size = 0;
        let status = (self.get_hash_size)(self as *const _ as *mut _, algorithm, &mut size);
        status.to_status_result_with(size)
    }

    /// Calculate hash of data
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer and data are valid.
    pub unsafe fn calculate(
        &self,
        algorithm: &Guid,
        data: &[u8],
        output: &mut Efi2HashOutput,
    ) -> Result<(), Status> {
        let status = (self.hash)(
            self as *const _ as *mut _,
            algorithm,
            false,
            data.as_ptr(),
            data.len() as u64,
            output,
        );
        status.to_status_result()
    }
}

impl fmt::Debug for EfiHashProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EfiHashProtocol").finish()
    }
}

/// Hash output union
#[repr(C)]
pub union Efi2HashOutput {
    /// SHA-1 output (20 bytes)
    pub sha1: [u8; 20],
    /// SHA-224 output (28 bytes)
    pub sha224: [u8; 28],
    /// SHA-256 output (32 bytes)
    pub sha256: [u8; 32],
    /// SHA-384 output (48 bytes)
    pub sha384: [u8; 48],
    /// SHA-512 output (64 bytes)
    pub sha512: [u8; 64],
}

impl Default for Efi2HashOutput {
    fn default() -> Self {
        Self { sha512: [0; 64] }
    }
}

/// Hash algorithm GUIDs
pub mod hash_algorithm {
    use super::*;

    /// SHA-1
    pub const SHA1: Guid = Guid::new(0x2AE9D80F, 0x3FB2, 0x4095, [
        0xB7, 0xB1, 0xE9, 0x31, 0x57, 0xB9, 0x46, 0xB6,
    ]);

    /// SHA-224
    pub const SHA224: Guid = Guid::new(0x8DF01A06, 0x9BD5, 0x4BF7, [
        0xB0, 0x21, 0xDB, 0x4F, 0xD9, 0xCC, 0xF4, 0x5B,
    ]);

    /// SHA-256
    pub const SHA256: Guid = Guid::new(0x51AA59DE, 0xFDF2, 0x4EA3, [
        0xBC, 0x63, 0x87, 0x5F, 0xB7, 0x84, 0x2E, 0xE9,
    ]);

    /// SHA-384
    pub const SHA384: Guid = Guid::new(0xEFA96432, 0xDE33, 0x4DD2, [
        0xAE, 0xE6, 0x32, 0x8C, 0x33, 0xDF, 0x77, 0x7A,
    ]);

    /// SHA-512
    pub const SHA512: Guid = Guid::new(0xCAA4381E, 0x750C, 0x4770, [
        0xB8, 0x70, 0x7A, 0x23, 0xB4, 0xE4, 0x21, 0x30,
    ]);

    /// MD5 (deprecated, for compatibility only)
    pub const MD5: Guid = Guid::new(0xAF7C79C0, 0x8F39, 0x4535, [
        0x8A, 0xD5, 0x83, 0x90, 0x11, 0x6B, 0x3E, 0xE0,
    ]);
}

// =============================================================================
// TIMESTAMP PROTOCOL (for entropy seeding)
// =============================================================================

/// Timestamp Protocol
#[repr(C)]
pub struct EfiTimestampProtocol {
    /// Get timestamp
    pub get_timestamp: unsafe extern "efiapi" fn(this: *mut Self) -> u64,

    /// Get properties
    pub get_properties: unsafe extern "efiapi" fn(
        this: *mut Self,
        properties: *mut EfiTimestampProperties,
    ) -> Status,
}

impl EfiTimestampProtocol {
    /// Protocol GUID
    pub const GUID: Guid = Guid::new(0xAFBFDE41, 0x2E6E, 0x4262, [
        0xBA, 0x65, 0x62, 0xB9, 0x23, 0x6E, 0x54, 0x95,
    ]);

    /// Get current timestamp
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn get_timestamp(&self) -> u64 {
        (self.get_timestamp)(self as *const _ as *mut _)
    }

    /// Get timestamp properties
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn get_properties(&self) -> Result<EfiTimestampProperties, Status> {
        let mut props = EfiTimestampProperties::default();
        let status = (self.get_properties)(self as *const _ as *mut _, &mut props);
        status.to_status_result_with(props)
    }
}

impl fmt::Debug for EfiTimestampProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EfiTimestampProtocol").finish()
    }
}

/// Timestamp properties
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct EfiTimestampProperties {
    /// Frequency in Hz
    pub frequency: u64,
    /// End value (counter rolls over)
    pub end_value: u64,
}

// =============================================================================
// ENTROPY COLLECTION
// =============================================================================

/// Collect entropy from various sources
#[derive(Debug, Clone, Copy)]
pub struct EntropyCollector {
    /// Accumulated entropy
    pool: [u8; 64],
    /// Current position in pool
    position: usize,
    /// Total entropy bits collected
    bits_collected: usize,
}

impl EntropyCollector {
    /// Create a new entropy collector
    pub const fn new() -> Self {
        Self {
            pool: [0; 64],
            position: 0,
            bits_collected: 0,
        }
    }

    /// Add entropy from bytes
    pub fn add_entropy(&mut self, data: &[u8], bits: usize) {
        for &byte in data {
            self.pool[self.position] ^= byte;
            self.position = (self.position + 1) % 64;
        }
        self.bits_collected = self.bits_collected.saturating_add(bits);
    }

    /// Add entropy from a u64 value
    pub fn add_u64(&mut self, value: u64, bits: usize) {
        self.add_entropy(&value.to_le_bytes(), bits);
    }

    /// Check if we have enough entropy
    pub fn has_sufficient_entropy(&self) -> bool {
        self.bits_collected >= 256
    }

    /// Get the entropy pool
    pub fn pool(&self) -> &[u8; 64] {
        &self.pool
    }

    /// Get estimated entropy bits
    pub fn bits(&self) -> usize {
        self.bits_collected
    }

    /// Reset the collector
    pub fn reset(&mut self) {
        self.pool = [0; 64];
        self.position = 0;
        self.bits_collected = 0;
    }
}

impl Default for EntropyCollector {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// SIMPLE RNG STATE
// =============================================================================

/// Simple PRNG for fallback (xorshift64)
#[derive(Debug, Clone, Copy)]
pub struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    /// Create with seed
    pub const fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x853C49E6748FEA9B } else { seed },
        }
    }

    /// Get next random u64
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Get next random u32
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Fill buffer with random bytes
    pub fn fill(&mut self, buffer: &mut [u8]) {
        let mut chunks = buffer.chunks_exact_mut(8);
        for chunk in chunks.by_ref() {
            let val = self.next_u64();
            chunk.copy_from_slice(&val.to_le_bytes());
        }

        let remainder = chunks.into_remainder();
        if !remainder.is_empty() {
            let val = self.next_u64();
            let bytes = val.to_le_bytes();
            remainder.copy_from_slice(&bytes[..remainder.len()]);
        }
    }

    /// Get random value in range [0, max)
    pub fn next_range(&mut self, max: u64) -> u64 {
        if max == 0 {
            return 0;
        }

        // Avoid modulo bias
        let threshold = max.wrapping_neg() % max;
        loop {
            let r = self.next_u64();
            if r >= threshold {
                return r % max;
            }
        }
    }

    /// Shuffle a slice
    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        let len = slice.len();
        if len <= 1 {
            return;
        }

        for i in (1..len).rev() {
            let j = self.next_range((i + 1) as u64) as usize;
            slice.swap(i, j);
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
    fn test_simple_rng() {
        let mut rng = SimpleRng::new(12345);
        let v1 = rng.next_u64();
        let v2 = rng.next_u64();
        assert_ne!(v1, v2);
    }

    #[test]
    fn test_entropy_collector() {
        let mut collector = EntropyCollector::new();
        assert!(!collector.has_sufficient_entropy());

        for i in 0..32 {
            collector.add_u64(i, 8);
        }

        assert!(collector.has_sufficient_entropy());
    }

    #[test]
    fn test_simple_rng_fill() {
        let mut rng = SimpleRng::new(42);
        let mut buffer = [0u8; 100];
        rng.fill(&mut buffer);

        // Check that buffer is not all zeros
        assert!(buffer.iter().any(|&b| b != 0));
    }

    #[test]
    fn test_simple_rng_range() {
        let mut rng = SimpleRng::new(12345);
        for _ in 0..100 {
            let val = rng.next_range(10);
            assert!(val < 10);
        }
    }

    #[test]
    fn test_rng_algorithm_info() {
        let info = RngAlgorithmInfo::from_guid(&rng_algorithm::RAW);
        assert!(info.is_some());
        assert_eq!(info.unwrap().name, "RAW");
    }
}
