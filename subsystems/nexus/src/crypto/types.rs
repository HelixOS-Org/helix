//! Core crypto types and identifiers.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// ID TYPES
// ============================================================================

/// Algorithm ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlgorithmId(pub u64);

impl AlgorithmId {
    /// Create new algorithm ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get raw value
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Key ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyId(pub u64);

impl KeyId {
    /// Create new key ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get raw value
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Transform ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransformId(pub u64);

impl TransformId {
    /// Create new transform ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get raw value
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

// ============================================================================
// ALGORITHM TYPE
// ============================================================================

/// Algorithm type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AlgorithmType {
    /// Cipher (symmetric encryption)
    Cipher,
    /// Block cipher
    BlockCipher,
    /// Stream cipher
    StreamCipher,
    /// AEAD (Authenticated Encryption with Associated Data)
    Aead,
    /// Hash function
    Hash,
    /// HMAC
    Hmac,
    /// KDF (Key Derivation Function)
    Kdf,
    /// Asymmetric cipher
    Akcipher,
    /// Digital signature
    Signature,
    /// Key exchange
    Kpp,
    /// Random number generator
    Rng,
    /// Compression
    Compress,
    /// Unknown
    Unknown,
}

impl AlgorithmType {
    /// Get type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Cipher => "cipher",
            Self::BlockCipher => "blkcipher",
            Self::StreamCipher => "stream",
            Self::Aead => "aead",
            Self::Hash => "hash",
            Self::Hmac => "hmac",
            Self::Kdf => "kdf",
            Self::Akcipher => "akcipher",
            Self::Signature => "sig",
            Self::Kpp => "kpp",
            Self::Rng => "rng",
            Self::Compress => "compress",
            Self::Unknown => "unknown",
        }
    }

    /// Is symmetric
    pub fn is_symmetric(&self) -> bool {
        matches!(
            self,
            Self::Cipher | Self::BlockCipher | Self::StreamCipher | Self::Aead
        )
    }

    /// Is asymmetric
    pub fn is_asymmetric(&self) -> bool {
        matches!(self, Self::Akcipher | Self::Signature | Self::Kpp)
    }

    /// Is hash
    pub fn is_hash(&self) -> bool {
        matches!(self, Self::Hash | Self::Hmac)
    }
}

// ============================================================================
// PRIORITY
// ============================================================================

/// Algorithm priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Priority(pub u32);

impl Priority {
    /// Create new priority
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Get raw value
    pub const fn raw(&self) -> u32 {
        self.0
    }

    /// Low priority
    pub const LOW: Self = Self(100);
    /// Normal priority
    pub const NORMAL: Self = Self(200);
    /// High priority
    pub const HIGH: Self = Self(300);
    /// Hardware accelerated
    pub const HARDWARE: Self = Self(400);
}

// ============================================================================
// SECURITY STRENGTH
// ============================================================================

/// Security strength (bits)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SecurityStrength(pub u16);

impl SecurityStrength {
    /// Create new security strength
    pub const fn new(bits: u16) -> Self {
        Self(bits)
    }

    /// Get bits
    pub const fn bits(&self) -> u16 {
        self.0
    }

    /// Is considered secure
    pub fn is_secure(&self) -> bool {
        self.0 >= 128
    }

    /// Is considered weak
    pub fn is_weak(&self) -> bool {
        self.0 < 80
    }

    /// Common strengths
    pub const BITS_80: Self = Self(80);
    pub const BITS_112: Self = Self(112);
    pub const BITS_128: Self = Self(128);
    pub const BITS_192: Self = Self(192);
    pub const BITS_256: Self = Self(256);
}

// ============================================================================
// ALGORITHM STATUS
// ============================================================================

/// Algorithm status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlgorithmStatus {
    /// Active and available
    Active,
    /// Deprecated (still available but not recommended)
    Deprecated,
    /// Broken (should not be used)
    Broken,
    /// Testing only
    Testing,
    /// Disabled
    Disabled,
}

impl AlgorithmStatus {
    /// Get status name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Deprecated => "deprecated",
            Self::Broken => "broken",
            Self::Testing => "testing",
            Self::Disabled => "disabled",
        }
    }
}

// ============================================================================
// CIPHER MODE
// ============================================================================

/// Cipher mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CipherMode {
    /// ECB (Electronic Codebook)
    Ecb,
    /// CBC (Cipher Block Chaining)
    Cbc,
    /// CFB (Cipher Feedback)
    Cfb,
    /// OFB (Output Feedback)
    Ofb,
    /// CTR (Counter)
    Ctr,
    /// GCM (Galois/Counter Mode)
    Gcm,
    /// CCM (Counter with CBC-MAC)
    Ccm,
    /// XTS (XEX-based Tweaked-codebook mode with ciphertext Stealing)
    Xts,
    /// CTS (Ciphertext Stealing)
    Cts,
    /// Poly1305
    Poly1305,
    /// ChaCha20-Poly1305
    ChaCha20Poly1305,
    /// Unknown
    Unknown,
}

impl CipherMode {
    /// Get mode name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Ecb => "ecb",
            Self::Cbc => "cbc",
            Self::Cfb => "cfb",
            Self::Ofb => "ofb",
            Self::Ctr => "ctr",
            Self::Gcm => "gcm",
            Self::Ccm => "ccm",
            Self::Xts => "xts",
            Self::Cts => "cts",
            Self::Poly1305 => "poly1305",
            Self::ChaCha20Poly1305 => "chacha20-poly1305",
            Self::Unknown => "unknown",
        }
    }

    /// Is authenticated
    pub fn is_authenticated(&self) -> bool {
        matches!(
            self,
            Self::Gcm | Self::Ccm | Self::Poly1305 | Self::ChaCha20Poly1305
        )
    }

    /// Is vulnerable to padding oracle
    pub fn is_padding_vulnerable(&self) -> bool {
        matches!(self, Self::Cbc | Self::Ecb)
    }

    /// Is ECB (insecure for most uses)
    pub fn is_ecb(&self) -> bool {
        matches!(self, Self::Ecb)
    }
}
