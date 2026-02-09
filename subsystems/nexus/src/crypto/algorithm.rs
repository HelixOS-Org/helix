//! Algorithm information and known algorithms.

extern crate alloc;

use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};

use super::types::{
    AlgorithmId, AlgorithmStatus, AlgorithmType, CipherMode, Priority, SecurityStrength,
};

// ============================================================================
// ALGORITHM INFO
// ============================================================================

/// Algorithm info
#[derive(Debug, Clone)]
pub struct AlgorithmInfo {
    /// Algorithm ID
    pub id: AlgorithmId,
    /// Name
    pub name: String,
    /// Type
    pub alg_type: AlgorithmType,
    /// Driver name
    pub driver: String,
    /// Priority
    pub priority: Priority,
    /// Block size (bytes)
    pub block_size: u32,
    /// Key size min (bytes)
    pub key_size_min: u32,
    /// Key size max (bytes)
    pub key_size_max: u32,
    /// IV size (bytes)
    pub iv_size: u32,
    /// Auth tag size (bytes, for AEAD)
    pub auth_size: u32,
    /// Security strength
    pub strength: SecurityStrength,
    /// Status
    pub status: AlgorithmStatus,
    /// Is hardware accelerated
    pub hw_accelerated: bool,
    /// Cipher mode (if applicable)
    pub mode: Option<CipherMode>,
    /// Reference count
    pub refcount: AtomicU64,
    /// Use count
    pub use_count: AtomicU64,
    /// Total bytes processed
    pub bytes_processed: AtomicU64,
}

impl AlgorithmInfo {
    /// Create new algorithm info
    pub fn new(id: AlgorithmId, name: String, alg_type: AlgorithmType) -> Self {
        Self {
            id,
            name,
            alg_type,
            driver: String::new(),
            priority: Priority::NORMAL,
            block_size: 0,
            key_size_min: 0,
            key_size_max: 0,
            iv_size: 0,
            auth_size: 0,
            strength: SecurityStrength::BITS_128,
            status: AlgorithmStatus::Active,
            hw_accelerated: false,
            mode: None,
            refcount: AtomicU64::new(0),
            use_count: AtomicU64::new(0),
            bytes_processed: AtomicU64::new(0),
        }
    }

    /// Record use
    #[inline(always)]
    pub fn record_use(&self, bytes: u64) {
        self.use_count.fetch_add(1, Ordering::Relaxed);
        self.bytes_processed.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Get use count
    #[inline(always)]
    pub fn use_count(&self) -> u64 {
        self.use_count.load(Ordering::Relaxed)
    }

    /// Get bytes processed
    #[inline(always)]
    pub fn bytes_processed(&self) -> u64 {
        self.bytes_processed.load(Ordering::Relaxed)
    }

    /// Is secure
    #[inline]
    pub fn is_secure(&self) -> bool {
        self.status == AlgorithmStatus::Active
            && self.strength.is_secure()
            && !self.mode.map(|m| m.is_ecb()).unwrap_or(false)
    }

    /// Is deprecated
    #[inline]
    pub fn is_deprecated(&self) -> bool {
        matches!(
            self.status,
            AlgorithmStatus::Deprecated | AlgorithmStatus::Broken
        )
    }
}

// ============================================================================
// KNOWN ALGORITHMS
// ============================================================================

/// Known algorithm definitions
pub struct KnownAlgorithms;

impl KnownAlgorithms {
    // Symmetric ciphers
    pub const AES: &'static str = "aes";
    pub const AES_128: &'static str = "aes-128";
    pub const AES_192: &'static str = "aes-192";
    pub const AES_256: &'static str = "aes-256";
    pub const CHACHA20: &'static str = "chacha20";
    pub const DES: &'static str = "des";
    pub const DES3: &'static str = "des3_ede";
    pub const BLOWFISH: &'static str = "blowfish";
    pub const TWOFISH: &'static str = "twofish";
    pub const SERPENT: &'static str = "serpent";
    pub const CAMELLIA: &'static str = "camellia";
    pub const CAST5: &'static str = "cast5";
    pub const CAST6: &'static str = "cast6";
    pub const SM4: &'static str = "sm4";
    pub const ARIA: &'static str = "aria";

    // AEAD
    pub const AES_GCM: &'static str = "gcm(aes)";
    pub const AES_CCM: &'static str = "ccm(aes)";
    pub const CHACHA20_POLY1305: &'static str = "rfc7539(chacha20,poly1305)";

    // Hash functions
    pub const MD5: &'static str = "md5";
    pub const SHA1: &'static str = "sha1";
    pub const SHA224: &'static str = "sha224";
    pub const SHA256: &'static str = "sha256";
    pub const SHA384: &'static str = "sha384";
    pub const SHA512: &'static str = "sha512";
    pub const SHA3_256: &'static str = "sha3-256";
    pub const SHA3_512: &'static str = "sha3-512";
    pub const BLAKE2B: &'static str = "blake2b-256";
    pub const BLAKE2S: &'static str = "blake2s-256";
    pub const SM3: &'static str = "sm3";

    // HMAC
    pub const HMAC_SHA256: &'static str = "hmac(sha256)";
    pub const HMAC_SHA512: &'static str = "hmac(sha512)";

    // KDF
    pub const HKDF_SHA256: &'static str = "hkdf(sha256)";
    pub const PBKDF2_SHA256: &'static str = "pbkdf2(sha256)";

    // Asymmetric
    pub const RSA: &'static str = "rsa";
    pub const ECDSA: &'static str = "ecdsa";
    pub const ECDH: &'static str = "ecdh";
    pub const ED25519: &'static str = "ed25519";
    pub const X25519: &'static str = "curve25519";

    // RNG
    pub const DRBG_CTR: &'static str = "drbg_pr_ctr_aes256";
    pub const DRBG_HASH: &'static str = "drbg_pr_sha256";

    /// Get security strength for algorithm
    pub fn strength(name: &str) -> SecurityStrength {
        match name {
            "md5" => SecurityStrength::new(64),  // Broken
            "sha1" => SecurityStrength::new(80), // Deprecated
            "des" => SecurityStrength::new(56),  // Broken
            "des3_ede" => SecurityStrength::BITS_112,
            "aes-128" | "aes" => SecurityStrength::BITS_128,
            "aes-192" => SecurityStrength::BITS_192,
            "aes-256" => SecurityStrength::BITS_256,
            "sha256" | "sha3-256" | "blake2s-256" => SecurityStrength::BITS_128,
            "sha384" => SecurityStrength::BITS_192,
            "sha512" | "sha3-512" | "blake2b-512" => SecurityStrength::BITS_256,
            "chacha20" | "chacha20-poly1305" => SecurityStrength::BITS_256,
            "rsa-2048" => SecurityStrength::BITS_112,
            "rsa-3072" => SecurityStrength::BITS_128,
            "rsa-4096" => SecurityStrength::BITS_192,
            "ed25519" | "x25519" => SecurityStrength::BITS_128,
            "ecdsa-p256" | "ecdh-p256" => SecurityStrength::BITS_128,
            "ecdsa-p384" | "ecdh-p384" => SecurityStrength::BITS_192,
            _ => SecurityStrength::BITS_128, // Default assumption
        }
    }

    /// Get status for algorithm
    #[inline]
    pub fn status(name: &str) -> AlgorithmStatus {
        match name {
            "md5" | "md4" | "rc4" => AlgorithmStatus::Broken,
            "sha1" | "des" | "des3_ede" | "blowfish" => AlgorithmStatus::Deprecated,
            _ => AlgorithmStatus::Active,
        }
    }
}
