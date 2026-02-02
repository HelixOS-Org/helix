//! HelixFS Crypto Subsystem
//!
//! Provides encryption, authentication, and integrity verification
//! for filesystem data at rest.
//!
//! # Features
//! - Per-file encryption with unique keys
//! - AEAD (Authenticated Encryption with Associated Data)
//! - Key derivation and wrapping
//! - Integrity verification with checksums
//! - Hardware acceleration support (AES-NI, SHA-NI)

#![allow(dead_code)]

pub mod aead;
pub mod cipher;
pub mod integrity;
pub mod key;

use crate::core::error::HfsError;

// ============================================================================
// Constants
// ============================================================================

/// AES-256 key size
pub const AES_256_KEY_SIZE: usize = 32;

/// AES block size
pub const AES_BLOCK_SIZE: usize = 16;

/// GCM nonce size
pub const GCM_NONCE_SIZE: usize = 12;

/// GCM tag size
pub const GCM_TAG_SIZE: usize = 16;

/// ChaCha20 key size
pub const CHACHA20_KEY_SIZE: usize = 32;

/// ChaCha20 nonce size
pub const CHACHA20_NONCE_SIZE: usize = 12;

/// Poly1305 tag size
pub const POLY1305_TAG_SIZE: usize = 16;

/// SHA-256 digest size
pub const SHA256_SIZE: usize = 32;

/// BLAKE3 digest size
pub const BLAKE3_SIZE: usize = 32;

/// XXHash64 size
pub const XXHASH64_SIZE: usize = 8;

/// CRC32C size
pub const CRC32C_SIZE: usize = 4;

/// Maximum key slot count
pub const MAX_KEY_SLOTS: usize = 8;

/// Key derivation iterations
pub const KEY_DERIVE_ITERATIONS: u32 = 100_000;

// ============================================================================
// Cipher Algorithm
// ============================================================================

/// Supported cipher algorithms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CipherAlgorithm {
    /// No encryption
    None             = 0,
    /// AES-256-XTS (disk encryption standard)
    Aes256Xts        = 1,
    /// AES-256-GCM (AEAD)
    Aes256Gcm        = 2,
    /// ChaCha20-Poly1305 (AEAD)
    ChaCha20Poly1305 = 3,
    /// AES-256-CBC (legacy)
    Aes256Cbc        = 4,
}

impl CipherAlgorithm {
    /// From raw value
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::Aes256Xts,
            2 => Self::Aes256Gcm,
            3 => Self::ChaCha20Poly1305,
            4 => Self::Aes256Cbc,
            _ => Self::None,
        }
    }

    /// Key size for algorithm
    pub fn key_size(&self) -> usize {
        match self {
            Self::None => 0,
            Self::Aes256Xts => 64, // Two 256-bit keys
            Self::Aes256Gcm => AES_256_KEY_SIZE,
            Self::ChaCha20Poly1305 => CHACHA20_KEY_SIZE,
            Self::Aes256Cbc => AES_256_KEY_SIZE,
        }
    }

    /// Nonce/IV size
    pub fn nonce_size(&self) -> usize {
        match self {
            Self::None => 0,
            Self::Aes256Xts => 16, // Tweak
            Self::Aes256Gcm => GCM_NONCE_SIZE,
            Self::ChaCha20Poly1305 => CHACHA20_NONCE_SIZE,
            Self::Aes256Cbc => AES_BLOCK_SIZE,
        }
    }

    /// Tag size (for AEAD)
    pub fn tag_size(&self) -> usize {
        match self {
            Self::Aes256Gcm => GCM_TAG_SIZE,
            Self::ChaCha20Poly1305 => POLY1305_TAG_SIZE,
            _ => 0,
        }
    }

    /// Is AEAD algorithm
    #[inline]
    pub fn is_aead(&self) -> bool {
        matches!(self, Self::Aes256Gcm | Self::ChaCha20Poly1305)
    }
}

impl Default for CipherAlgorithm {
    fn default() -> Self {
        Self::None
    }
}

// ============================================================================
// Hash Algorithm
// ============================================================================

/// Supported hash algorithms for integrity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum HashAlgorithm {
    /// No hashing
    None     = 0,
    /// CRC32C (fast, non-cryptographic)
    Crc32c   = 1,
    /// XXHash64 (fast, non-cryptographic)
    XxHash64 = 2,
    /// SHA-256 (cryptographic)
    Sha256   = 3,
    /// BLAKE3 (fast cryptographic)
    Blake3   = 4,
}

impl HashAlgorithm {
    /// From raw value
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::Crc32c,
            2 => Self::XxHash64,
            3 => Self::Sha256,
            4 => Self::Blake3,
            _ => Self::None,
        }
    }

    /// Digest size
    pub fn digest_size(&self) -> usize {
        match self {
            Self::None => 0,
            Self::Crc32c => CRC32C_SIZE,
            Self::XxHash64 => XXHASH64_SIZE,
            Self::Sha256 => SHA256_SIZE,
            Self::Blake3 => BLAKE3_SIZE,
        }
    }

    /// Is cryptographic
    #[inline]
    pub fn is_cryptographic(&self) -> bool {
        matches!(self, Self::Sha256 | Self::Blake3)
    }
}

impl Default for HashAlgorithm {
    fn default() -> Self {
        Self::Crc32c
    }
}

// ============================================================================
// KDF Algorithm
// ============================================================================

/// Key derivation function.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum KdfAlgorithm {
    /// No KDF
    None         = 0,
    /// PBKDF2-HMAC-SHA256
    Pbkdf2Sha256 = 1,
    /// Argon2id
    Argon2id     = 2,
    /// HKDF-SHA256
    HkdfSha256   = 3,
    /// scrypt
    Scrypt       = 4,
}

impl KdfAlgorithm {
    /// From raw value
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::Pbkdf2Sha256,
            2 => Self::Argon2id,
            3 => Self::HkdfSha256,
            4 => Self::Scrypt,
            _ => Self::None,
        }
    }

    /// Is password-based
    #[inline]
    pub fn is_password_based(&self) -> bool {
        matches!(self, Self::Pbkdf2Sha256 | Self::Argon2id | Self::Scrypt)
    }
}

impl Default for KdfAlgorithm {
    fn default() -> Self {
        Self::Argon2id
    }
}

// ============================================================================
// Crypto Configuration
// ============================================================================

/// Crypto configuration.
#[derive(Clone, Copy, Debug)]
pub struct CryptoConfig {
    /// Cipher algorithm for data
    pub cipher: CipherAlgorithm,
    /// Hash algorithm for integrity
    pub hash: HashAlgorithm,
    /// KDF for key derivation
    pub kdf: KdfAlgorithm,
    /// Use per-file keys
    pub per_file_keys: bool,
    /// Enable filename encryption
    pub encrypt_filenames: bool,
    /// Enable metadata encryption
    pub encrypt_metadata: bool,
    /// KDF iterations (password-based)
    pub kdf_iterations: u32,
    /// Argon2 memory cost (KB)
    pub argon2_memory: u32,
    /// Argon2 parallelism
    pub argon2_parallelism: u8,
}

impl CryptoConfig {
    /// Default configuration (no encryption)
    pub const fn none() -> Self {
        Self {
            cipher: CipherAlgorithm::None,
            hash: HashAlgorithm::Crc32c,
            kdf: KdfAlgorithm::None,
            per_file_keys: false,
            encrypt_filenames: false,
            encrypt_metadata: false,
            kdf_iterations: 0,
            argon2_memory: 0,
            argon2_parallelism: 1,
        }
    }

    /// Standard encryption (AES-256-XTS)
    pub const fn standard() -> Self {
        Self {
            cipher: CipherAlgorithm::Aes256Xts,
            hash: HashAlgorithm::XxHash64,
            kdf: KdfAlgorithm::Argon2id,
            per_file_keys: false,
            encrypt_filenames: false,
            encrypt_metadata: false,
            kdf_iterations: KEY_DERIVE_ITERATIONS,
            argon2_memory: 65536, // 64 MB
            argon2_parallelism: 4,
        }
    }

    /// High security (AEAD + per-file keys)
    pub const fn high_security() -> Self {
        Self {
            cipher: CipherAlgorithm::Aes256Gcm,
            hash: HashAlgorithm::Blake3,
            kdf: KdfAlgorithm::Argon2id,
            per_file_keys: true,
            encrypt_filenames: true,
            encrypt_metadata: true,
            kdf_iterations: KEY_DERIVE_ITERATIONS * 2,
            argon2_memory: 262144, // 256 MB
            argon2_parallelism: 8,
        }
    }

    /// Is encryption enabled
    #[inline]
    pub fn is_encrypted(&self) -> bool {
        self.cipher != CipherAlgorithm::None
    }

    /// Has integrity checking
    #[inline]
    pub fn has_integrity(&self) -> bool {
        self.hash != HashAlgorithm::None
    }
}

impl Default for CryptoConfig {
    fn default() -> Self {
        Self::none()
    }
}

// ============================================================================
// Crypto State
// ============================================================================

/// State of crypto subsystem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CryptoState {
    /// Not initialized
    Uninitialized = 0,
    /// Keys not loaded
    Locked        = 1,
    /// Keys loaded, ready to use
    Unlocked      = 2,
    /// Error state
    Error         = 3,
}

impl CryptoState {
    /// From raw value
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::Locked,
            2 => Self::Unlocked,
            3 => Self::Error,
            _ => Self::Uninitialized,
        }
    }

    /// Is usable
    #[inline]
    pub fn is_usable(&self) -> bool {
        *self == Self::Unlocked
    }
}

impl Default for CryptoState {
    fn default() -> Self {
        Self::Uninitialized
    }
}

// ============================================================================
// Crypto Stats
// ============================================================================

/// Crypto statistics.
#[derive(Clone, Copy, Debug, Default)]
pub struct CryptoStats {
    /// Blocks encrypted
    pub blocks_encrypted: u64,
    /// Blocks decrypted
    pub blocks_decrypted: u64,
    /// Bytes encrypted
    pub bytes_encrypted: u64,
    /// Bytes decrypted
    pub bytes_decrypted: u64,
    /// Integrity checks passed
    pub integrity_passed: u64,
    /// Integrity checks failed
    pub integrity_failed: u64,
    /// Keys derived
    pub keys_derived: u64,
    /// Encryption errors
    pub encrypt_errors: u64,
    /// Decryption errors
    pub decrypt_errors: u64,
}

impl CryptoStats {
    /// Create new stats
    pub const fn new() -> Self {
        Self {
            blocks_encrypted: 0,
            blocks_decrypted: 0,
            bytes_encrypted: 0,
            bytes_decrypted: 0,
            integrity_passed: 0,
            integrity_failed: 0,
            keys_derived: 0,
            encrypt_errors: 0,
            decrypt_errors: 0,
        }
    }

    /// Integrity failure rate
    pub fn integrity_failure_rate(&self) -> f32 {
        let total = self.integrity_passed + self.integrity_failed;
        if total == 0 {
            return 0.0;
        }
        (self.integrity_failed as f32 / total as f32) * 100.0
    }
}

// ============================================================================
// Crypto Error
// ============================================================================

/// Crypto-specific errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CryptoError {
    /// No error
    None                 = 0,
    /// Invalid key
    InvalidKey           = 1,
    /// Invalid nonce/IV
    InvalidNonce         = 2,
    /// Authentication failed
    AuthFailed           = 3,
    /// Integrity check failed
    IntegrityFailed      = 4,
    /// Key derivation failed
    KdfFailed            = 5,
    /// Algorithm not supported
    UnsupportedAlgorithm = 6,
    /// Crypto not initialized
    NotInitialized       = 7,
    /// Crypto locked
    Locked               = 8,
    /// Buffer too small
    BufferTooSmall       = 9,
    /// Invalid data
    InvalidData          = 10,
}

impl CryptoError {
    /// From raw value
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::InvalidKey,
            2 => Self::InvalidNonce,
            3 => Self::AuthFailed,
            4 => Self::IntegrityFailed,
            5 => Self::KdfFailed,
            6 => Self::UnsupportedAlgorithm,
            7 => Self::NotInitialized,
            8 => Self::Locked,
            9 => Self::BufferTooSmall,
            10 => Self::InvalidData,
            _ => Self::None,
        }
    }

    /// To HFS error
    pub fn to_hfs_error(&self) -> HfsError {
        match self {
            Self::None => HfsError::Success,
            Self::InvalidKey => HfsError::InvalidKey,
            Self::InvalidNonce => HfsError::InvalidArg,
            Self::AuthFailed => HfsError::AuthFailed,
            Self::IntegrityFailed => HfsError::IntegrityError,
            Self::KdfFailed => HfsError::CryptoError,
            Self::UnsupportedAlgorithm => HfsError::NotSupported,
            Self::NotInitialized => HfsError::NotInitialized,
            Self::Locked => HfsError::Locked,
            Self::BufferTooSmall => HfsError::BufferTooSmall,
            Self::InvalidData => HfsError::InvalidData,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cipher_algorithm() {
        assert_eq!(CipherAlgorithm::Aes256Gcm.key_size(), 32);
        assert_eq!(CipherAlgorithm::Aes256Xts.key_size(), 64);
        assert!(CipherAlgorithm::Aes256Gcm.is_aead());
        assert!(!CipherAlgorithm::Aes256Xts.is_aead());
    }

    #[test]
    fn test_hash_algorithm() {
        assert_eq!(HashAlgorithm::Sha256.digest_size(), 32);
        assert_eq!(HashAlgorithm::Crc32c.digest_size(), 4);
        assert!(HashAlgorithm::Sha256.is_cryptographic());
        assert!(!HashAlgorithm::Crc32c.is_cryptographic());
    }

    #[test]
    fn test_crypto_config() {
        let none = CryptoConfig::none();
        assert!(!none.is_encrypted());

        let std = CryptoConfig::standard();
        assert!(std.is_encrypted());
        assert!(std.has_integrity());

        let high = CryptoConfig::high_security();
        assert!(high.per_file_keys);
        assert!(high.encrypt_filenames);
    }

    #[test]
    fn test_crypto_state() {
        assert!(!CryptoState::Locked.is_usable());
        assert!(CryptoState::Unlocked.is_usable());
    }
}
