//! Cryptographic Primitives for Helix UEFI Bootloader
//!
//! This module provides comprehensive cryptographic support including
//! hash functions, digital signatures, and encryption primitives.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                     Cryptographic Subsystem                             │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Hash Functions                                │   │
//! │  │  SHA-256 │ SHA-384 │ SHA-512 │ SHA-1 │ MD5 │ SM3                │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Digital Signatures                            │   │
//! │  │  RSA │ ECDSA │ EdDSA │ SM2                                      │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Encryption                                    │   │
//! │  │  AES-128 │ AES-256 │ ChaCha20 │ SM4                             │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Key Management                                │   │
//! │  │  PKCS#7 │ X.509 │ Key Derivation │ Random                       │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

#![no_std]

use core::fmt;

// =============================================================================
// HASH ALGORITHMS
// =============================================================================

/// Hash algorithm identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    /// MD5 (128 bits) - INSECURE, for legacy only
    Md5,
    /// SHA-1 (160 bits) - WEAK, for legacy only
    Sha1,
    /// SHA-224 (224 bits)
    Sha224,
    /// SHA-256 (256 bits) - Recommended
    Sha256,
    /// SHA-384 (384 bits)
    Sha384,
    /// SHA-512 (512 bits)
    Sha512,
    /// SHA-512/256
    Sha512_256,
    /// SHA3-256
    Sha3_256,
    /// SHA3-384
    Sha3_384,
    /// SHA3-512
    Sha3_512,
    /// SM3 (Chinese standard)
    Sm3,
    /// BLAKE2b
    Blake2b,
    /// BLAKE2s
    Blake2s,
    /// BLAKE3
    Blake3,
}

impl HashAlgorithm {
    /// Get digest size in bytes
    pub const fn digest_size(&self) -> usize {
        match self {
            HashAlgorithm::Md5 => 16,
            HashAlgorithm::Sha1 => 20,
            HashAlgorithm::Sha224 => 28,
            HashAlgorithm::Sha256 => 32,
            HashAlgorithm::Sha384 => 48,
            HashAlgorithm::Sha512 => 64,
            HashAlgorithm::Sha512_256 => 32,
            HashAlgorithm::Sha3_256 => 32,
            HashAlgorithm::Sha3_384 => 48,
            HashAlgorithm::Sha3_512 => 64,
            HashAlgorithm::Sm3 => 32,
            HashAlgorithm::Blake2b => 64,
            HashAlgorithm::Blake2s => 32,
            HashAlgorithm::Blake3 => 32,
        }
    }

    /// Get block size in bytes
    pub const fn block_size(&self) -> usize {
        match self {
            HashAlgorithm::Md5 => 64,
            HashAlgorithm::Sha1 => 64,
            HashAlgorithm::Sha224 => 64,
            HashAlgorithm::Sha256 => 64,
            HashAlgorithm::Sha384 => 128,
            HashAlgorithm::Sha512 => 128,
            HashAlgorithm::Sha512_256 => 128,
            HashAlgorithm::Sha3_256 => 136,
            HashAlgorithm::Sha3_384 => 104,
            HashAlgorithm::Sha3_512 => 72,
            HashAlgorithm::Sm3 => 64,
            HashAlgorithm::Blake2b => 128,
            HashAlgorithm::Blake2s => 64,
            HashAlgorithm::Blake3 => 64,
        }
    }

    /// Check if algorithm is secure
    pub const fn is_secure(&self) -> bool {
        match self {
            HashAlgorithm::Md5 => false,
            HashAlgorithm::Sha1 => false,
            _ => true,
        }
    }

    /// Get OID for this algorithm
    pub const fn oid(&self) -> &'static [u8] {
        match self {
            HashAlgorithm::Md5 => &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x02, 0x05],
            HashAlgorithm::Sha1 => &[0x2B, 0x0E, 0x03, 0x02, 0x1A],
            HashAlgorithm::Sha224 => &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x04],
            HashAlgorithm::Sha256 => &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01],
            HashAlgorithm::Sha384 => &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02],
            HashAlgorithm::Sha512 => &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03],
            _ => &[],
        }
    }
}

/// Digest output (up to 64 bytes)
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Digest {
    /// Digest bytes
    pub bytes: [u8; 64],
    /// Actual length
    pub len: usize,
}

impl Digest {
    /// Create empty digest
    pub const fn empty() -> Self {
        Self {
            bytes: [0u8; 64],
            len: 0,
        }
    }

    /// Create from bytes
    pub fn from_bytes(data: &[u8]) -> Self {
        let mut digest = Self::empty();
        let len = data.len().min(64);
        digest.bytes[..len].copy_from_slice(&data[..len]);
        digest.len = len;
        digest
    }

    /// Get as slice
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

impl fmt::Debug for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Digest(")?;
        for b in &self.bytes[..self.len] {
            write!(f, "{:02x}", b)?;
        }
        write!(f, ")")
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in &self.bytes[..self.len] {
            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}

// =============================================================================
// SHA-256 CONSTANTS
// =============================================================================

/// SHA-256 round constants
pub const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// SHA-256 initial hash values
pub const SHA256_H: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

/// SHA-512 round constants
pub const SHA512_K: [u64; 80] = [
    0x428a2f98d728ae22,
    0x7137449123ef65cd,
    0xb5c0fbcfec4d3b2f,
    0xe9b5dba58189dbbc,
    0x3956c25bf348b538,
    0x59f111f1b605d019,
    0x923f82a4af194f9b,
    0xab1c5ed5da6d8118,
    0xd807aa98a3030242,
    0x12835b0145706fbe,
    0x243185be4ee4b28c,
    0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f,
    0x80deb1fe3b1696b1,
    0x9bdc06a725c71235,
    0xc19bf174cf692694,
    0xe49b69c19ef14ad2,
    0xefbe4786384f25e3,
    0x0fc19dc68b8cd5b5,
    0x240ca1cc77ac9c65,
    0x2de92c6f592b0275,
    0x4a7484aa6ea6e483,
    0x5cb0a9dcbd41fbd4,
    0x76f988da831153b5,
    0x983e5152ee66dfab,
    0xa831c66d2db43210,
    0xb00327c898fb213f,
    0xbf597fc7beef0ee4,
    0xc6e00bf33da88fc2,
    0xd5a79147930aa725,
    0x06ca6351e003826f,
    0x142929670a0e6e70,
    0x27b70a8546d22ffc,
    0x2e1b21385c26c926,
    0x4d2c6dfc5ac42aed,
    0x53380d139d95b3df,
    0x650a73548baf63de,
    0x766a0abb3c77b2a8,
    0x81c2c92e47edaee6,
    0x92722c851482353b,
    0xa2bfe8a14cf10364,
    0xa81a664bbc423001,
    0xc24b8b70d0f89791,
    0xc76c51a30654be30,
    0xd192e819d6ef5218,
    0xd69906245565a910,
    0xf40e35855771202a,
    0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8,
    0x1e376c085141ab53,
    0x2748774cdf8eeb99,
    0x34b0bcb5e19b48a8,
    0x391c0cb3c5c95a63,
    0x4ed8aa4ae3418acb,
    0x5b9cca4f7763e373,
    0x682e6ff3d6b2b8a3,
    0x748f82ee5defb2fc,
    0x78a5636f43172f60,
    0x84c87814a1f0ab72,
    0x8cc702081a6439ec,
    0x90befffa23631e28,
    0xa4506cebde82bde9,
    0xbef9a3f7b2c67915,
    0xc67178f2e372532b,
    0xca273eceea26619c,
    0xd186b8c721c0c207,
    0xeada7dd6cde0eb1e,
    0xf57d4f7fee6ed178,
    0x06f067aa72176fba,
    0x0a637dc5a2c898a6,
    0x113f9804bef90dae,
    0x1b710b35131c471b,
    0x28db77f523047d84,
    0x32caab7b40c72493,
    0x3c9ebe0a15c9bebc,
    0x431d67c49c100d4c,
    0x4cc5d4becb3e42b6,
    0x597f299cfc657e2a,
    0x5fcb6fab3ad6faec,
    0x6c44198c4a475817,
];

// =============================================================================
// SIGNATURE ALGORITHMS
// =============================================================================

/// Signature algorithm identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureAlgorithm {
    /// RSA with PKCS#1 v1.5 padding
    RsaPkcs1v15,
    /// RSA with PSS padding
    RsaPss,
    /// ECDSA with P-256 (secp256r1)
    EcdsaP256,
    /// ECDSA with P-384 (secp384r1)
    EcdsaP384,
    /// ECDSA with P-521 (secp521r1)
    EcdsaP521,
    /// Ed25519
    Ed25519,
    /// Ed448
    Ed448,
    /// SM2 (Chinese standard)
    Sm2,
}

impl SignatureAlgorithm {
    /// Get expected signature size in bytes
    pub const fn signature_size(&self) -> usize {
        match self {
            SignatureAlgorithm::RsaPkcs1v15 => 256, // For 2048-bit key
            SignatureAlgorithm::RsaPss => 256,
            SignatureAlgorithm::EcdsaP256 => 64,
            SignatureAlgorithm::EcdsaP384 => 96,
            SignatureAlgorithm::EcdsaP521 => 132,
            SignatureAlgorithm::Ed25519 => 64,
            SignatureAlgorithm::Ed448 => 114,
            SignatureAlgorithm::Sm2 => 64,
        }
    }

    /// Check if algorithm is elliptic curve based
    pub const fn is_ecc(&self) -> bool {
        match self {
            SignatureAlgorithm::RsaPkcs1v15 | SignatureAlgorithm::RsaPss => false,
            _ => true,
        }
    }
}

/// RSA key sizes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RsaKeySize {
    /// 1024 bits - WEAK
    Rsa1024,
    /// 2048 bits - Minimum recommended
    Rsa2048,
    /// 3072 bits
    Rsa3072,
    /// 4096 bits
    Rsa4096,
}

impl RsaKeySize {
    /// Get size in bits
    pub const fn bits(&self) -> usize {
        match self {
            RsaKeySize::Rsa1024 => 1024,
            RsaKeySize::Rsa2048 => 2048,
            RsaKeySize::Rsa3072 => 3072,
            RsaKeySize::Rsa4096 => 4096,
        }
    }

    /// Get size in bytes
    pub const fn bytes(&self) -> usize {
        self.bits() / 8
    }
}

/// Elliptic curve identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EllipticCurve {
    /// NIST P-256 (secp256r1, prime256v1)
    P256,
    /// NIST P-384 (secp384r1)
    P384,
    /// NIST P-521 (secp521r1)
    P521,
    /// Curve25519
    Curve25519,
    /// Curve448
    Curve448,
    /// SM2 curve
    Sm2,
    /// secp256k1 (Bitcoin)
    Secp256k1,
    /// BrainpoolP256r1
    BrainpoolP256r1,
    /// BrainpoolP384r1
    BrainpoolP384r1,
}

impl EllipticCurve {
    /// Get curve size in bits
    pub const fn bits(&self) -> usize {
        match self {
            EllipticCurve::P256 => 256,
            EllipticCurve::P384 => 384,
            EllipticCurve::P521 => 521,
            EllipticCurve::Curve25519 => 255,
            EllipticCurve::Curve448 => 448,
            EllipticCurve::Sm2 => 256,
            EllipticCurve::Secp256k1 => 256,
            EllipticCurve::BrainpoolP256r1 => 256,
            EllipticCurve::BrainpoolP384r1 => 384,
        }
    }

    /// Get OID
    pub const fn oid(&self) -> &'static [u8] {
        match self {
            EllipticCurve::P256 => &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x03, 0x01, 0x07],
            EllipticCurve::P384 => &[0x2B, 0x81, 0x04, 0x00, 0x22],
            EllipticCurve::P521 => &[0x2B, 0x81, 0x04, 0x00, 0x23],
            _ => &[],
        }
    }
}

// =============================================================================
// ENCRYPTION ALGORITHMS
// =============================================================================

/// Symmetric encryption algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymmetricAlgorithm {
    /// AES-128
    Aes128,
    /// AES-192
    Aes192,
    /// AES-256
    Aes256,
    /// ChaCha20
    ChaCha20,
    /// SM4
    Sm4,
    /// 3DES
    TripleDes,
}

impl SymmetricAlgorithm {
    /// Get key size in bytes
    pub const fn key_size(&self) -> usize {
        match self {
            SymmetricAlgorithm::Aes128 => 16,
            SymmetricAlgorithm::Aes192 => 24,
            SymmetricAlgorithm::Aes256 => 32,
            SymmetricAlgorithm::ChaCha20 => 32,
            SymmetricAlgorithm::Sm4 => 16,
            SymmetricAlgorithm::TripleDes => 24,
        }
    }

    /// Get block size in bytes
    pub const fn block_size(&self) -> usize {
        match self {
            SymmetricAlgorithm::Aes128
            | SymmetricAlgorithm::Aes192
            | SymmetricAlgorithm::Aes256 => 16,
            SymmetricAlgorithm::ChaCha20 => 64,
            SymmetricAlgorithm::Sm4 => 16,
            SymmetricAlgorithm::TripleDes => 8,
        }
    }
}

/// Block cipher mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipherMode {
    /// Electronic Codebook
    Ecb,
    /// Cipher Block Chaining
    Cbc,
    /// Counter
    Ctr,
    /// Galois/Counter Mode (authenticated)
    Gcm,
    /// Counter with CBC-MAC (authenticated)
    Ccm,
    /// Offset Codebook Mode
    Ocb,
    /// XEX-based Tweaked-codebook mode
    Xts,
}

impl CipherMode {
    /// Check if mode provides authentication
    pub const fn is_authenticated(&self) -> bool {
        match self {
            CipherMode::Gcm | CipherMode::Ccm | CipherMode::Ocb => true,
            _ => false,
        }
    }

    /// Check if mode requires IV/nonce
    pub const fn requires_iv(&self) -> bool {
        match self {
            CipherMode::Ecb => false,
            _ => true,
        }
    }
}

// =============================================================================
// KEY TYPES
// =============================================================================

/// Key type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    /// RSA public key
    RsaPublic,
    /// RSA private key
    RsaPrivate,
    /// EC public key
    EcPublic,
    /// EC private key
    EcPrivate,
    /// Symmetric key
    Symmetric,
    /// HMAC key
    Hmac,
}

/// Key usage flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyUsage(u32);

impl KeyUsage {
    pub const NONE: Self = Self(0);
    pub const SIGN: Self = Self(1 << 0);
    pub const VERIFY: Self = Self(1 << 1);
    pub const ENCRYPT: Self = Self(1 << 2);
    pub const DECRYPT: Self = Self(1 << 3);
    pub const KEY_WRAP: Self = Self(1 << 4);
    pub const KEY_UNWRAP: Self = Self(1 << 5);
    pub const DERIVE: Self = Self(1 << 6);

    /// Check if usage is allowed
    pub const fn allows(&self, usage: Self) -> bool {
        (self.0 & usage.0) == usage.0
    }

    /// Combine usages
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// =============================================================================
// X.509 CERTIFICATE
// =============================================================================

/// X.509 certificate version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum X509Version {
    /// Version 1
    V1,
    /// Version 2
    V2,
    /// Version 3
    V3,
}

/// X.509 basic constraints
#[derive(Debug, Clone, Copy)]
pub struct BasicConstraints {
    /// Is CA
    pub ca: bool,
    /// Path length constraint
    pub path_len: Option<u8>,
}

/// X.509 key usage
#[derive(Debug, Clone, Copy)]
pub struct X509KeyUsage(u16);

impl X509KeyUsage {
    pub const DIGITAL_SIGNATURE: Self = Self(1 << 0);
    pub const NON_REPUDIATION: Self = Self(1 << 1);
    pub const KEY_ENCIPHERMENT: Self = Self(1 << 2);
    pub const DATA_ENCIPHERMENT: Self = Self(1 << 3);
    pub const KEY_AGREEMENT: Self = Self(1 << 4);
    pub const KEY_CERT_SIGN: Self = Self(1 << 5);
    pub const CRL_SIGN: Self = Self(1 << 6);
    pub const ENCIPHER_ONLY: Self = Self(1 << 7);
    pub const DECIPHER_ONLY: Self = Self(1 << 8);

    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

/// Extended key usage OIDs
pub mod eku {
    /// Server authentication
    pub const SERVER_AUTH: &[u8] = &[0x2B, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x01];
    /// Client authentication
    pub const CLIENT_AUTH: &[u8] = &[0x2B, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x02];
    /// Code signing
    pub const CODE_SIGNING: &[u8] = &[0x2B, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x03];
    /// Email protection
    pub const EMAIL_PROTECTION: &[u8] = &[0x2B, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x04];
    /// Time stamping
    pub const TIME_STAMPING: &[u8] = &[0x2B, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x08];
    /// OCSP signing
    pub const OCSP_SIGNING: &[u8] = &[0x2B, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x09];
}

/// Certificate validity period
#[derive(Debug, Clone, Copy)]
pub struct Validity {
    /// Not before (Unix timestamp)
    pub not_before: u64,
    /// Not after (Unix timestamp)
    pub not_after: u64,
}

impl Validity {
    /// Check if timestamp is within validity period
    pub const fn is_valid_at(&self, timestamp: u64) -> bool {
        timestamp >= self.not_before && timestamp <= self.not_after
    }
}

/// Distinguished name component
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnComponent {
    /// Common Name
    CommonName,
    /// Country
    Country,
    /// State/Province
    State,
    /// Locality
    Locality,
    /// Organization
    Organization,
    /// Organizational Unit
    OrganizationalUnit,
    /// Email Address
    EmailAddress,
    /// Serial Number
    SerialNumber,
}

// =============================================================================
// PKCS#7 / CMS
// =============================================================================

/// PKCS#7 content type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pkcs7ContentType {
    /// Data
    Data,
    /// Signed data
    SignedData,
    /// Enveloped data
    EnvelopedData,
    /// Signed and enveloped data
    SignedEnvelopedData,
    /// Digested data
    DigestedData,
    /// Encrypted data
    EncryptedData,
}

/// Authenticode signature info
#[derive(Debug, Clone)]
pub struct AuthenticodeInfo {
    /// Hash algorithm used
    pub hash_algorithm: HashAlgorithm,
    /// Digest of signed content
    pub digest: Digest,
    /// Signature algorithm
    pub signature_algorithm: SignatureAlgorithm,
    /// Signer certificate (DER encoded, partial)
    pub signer_subject: [u8; 256],
    /// Subject length
    pub signer_subject_len: usize,
    /// Timestamp (if present)
    pub timestamp: Option<u64>,
    /// Certificate chain valid
    pub chain_valid: bool,
}

// =============================================================================
// SECURE BOOT
// =============================================================================

/// Secure Boot database type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecureBootDb {
    /// Platform Key (PK)
    Pk,
    /// Key Exchange Key (KEK)
    Kek,
    /// Authorized Database (db)
    Db,
    /// Forbidden Database (dbx)
    Dbx,
    /// Authorized Recovery Database (dbr)
    Dbr,
    /// Timestamp Database (dbt)
    Dbt,
}

impl SecureBootDb {
    /// Get variable name
    pub const fn variable_name(&self) -> &'static str {
        match self {
            SecureBootDb::Pk => "PK",
            SecureBootDb::Kek => "KEK",
            SecureBootDb::Db => "db",
            SecureBootDb::Dbx => "dbx",
            SecureBootDb::Dbr => "dbr",
            SecureBootDb::Dbt => "dbt",
        }
    }
}

/// Signature list type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureType {
    /// SHA-256 hash
    Sha256,
    /// RSA-2048 key
    Rsa2048,
    /// RSA-2048 + SHA-256
    Rsa2048Sha256,
    /// RSA-2048 + SHA-1
    Rsa2048Sha1,
    /// X.509 certificate
    X509,
    /// SHA-1 hash
    Sha1,
    /// SHA-224 hash
    Sha224,
    /// SHA-384 hash
    Sha384,
    /// SHA-512 hash
    Sha512,
    /// X.509 + SHA-256
    X509Sha256,
    /// X.509 + SHA-384
    X509Sha384,
    /// X.509 + SHA-512
    X509Sha512,
}

impl SignatureType {
    /// Get signature size in bytes
    pub const fn signature_size(&self) -> usize {
        match self {
            SignatureType::Sha256 => 32,
            SignatureType::Rsa2048 => 256,
            SignatureType::Rsa2048Sha256 => 256,
            SignatureType::Rsa2048Sha1 => 256,
            SignatureType::X509 => 0, // Variable
            SignatureType::Sha1 => 20,
            SignatureType::Sha224 => 28,
            SignatureType::Sha384 => 48,
            SignatureType::Sha512 => 64,
            SignatureType::X509Sha256 => 0,
            SignatureType::X509Sha384 => 0,
            SignatureType::X509Sha512 => 0,
        }
    }
}

// =============================================================================
// RANDOM NUMBER GENERATION
// =============================================================================

/// Random source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RandomSource {
    /// Hardware RNG (RDRAND)
    Hardware,
    /// TPM RNG
    Tpm,
    /// UEFI RNG Protocol
    UefiProtocol,
    /// Software PRNG (fallback)
    Software,
}

/// RNG algorithm (for UEFI RNG Protocol)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RngAlgorithm {
    /// Raw entropy
    Raw,
    /// SP800-90 Hash_DRBG using SHA-256
    Sp80090HashDrbgSha256,
    /// SP800-90 HMAC_DRBG using SHA-256
    Sp80090HmacDrbgSha256,
    /// SP800-90 CTR_DRBG using AES-256
    Sp80090CtrDrbgAes256,
    /// X9.31 using 3DES
    X931Aes256,
}

// =============================================================================
// MAC ALGORITHMS
// =============================================================================

/// MAC algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacAlgorithm {
    /// HMAC-SHA1
    HmacSha1,
    /// HMAC-SHA256
    HmacSha256,
    /// HMAC-SHA384
    HmacSha384,
    /// HMAC-SHA512
    HmacSha512,
    /// CMAC-AES-128
    CmacAes128,
    /// CMAC-AES-256
    CmacAes256,
    /// Poly1305
    Poly1305,
}

impl MacAlgorithm {
    /// Get MAC output size in bytes
    pub const fn output_size(&self) -> usize {
        match self {
            MacAlgorithm::HmacSha1 => 20,
            MacAlgorithm::HmacSha256 => 32,
            MacAlgorithm::HmacSha384 => 48,
            MacAlgorithm::HmacSha512 => 64,
            MacAlgorithm::CmacAes128 => 16,
            MacAlgorithm::CmacAes256 => 16,
            MacAlgorithm::Poly1305 => 16,
        }
    }
}

// =============================================================================
// KEY DERIVATION
// =============================================================================

/// Key derivation function
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kdf {
    /// HKDF with SHA-256
    HkdfSha256,
    /// HKDF with SHA-384
    HkdfSha384,
    /// HKDF with SHA-512
    HkdfSha512,
    /// PBKDF2 with SHA-256
    Pbkdf2Sha256,
    /// PBKDF2 with SHA-512
    Pbkdf2Sha512,
    /// scrypt
    Scrypt,
    /// Argon2id
    Argon2id,
}

/// PBKDF2 parameters
#[derive(Debug, Clone, Copy)]
pub struct Pbkdf2Params {
    /// Salt
    pub salt: [u8; 32],
    /// Salt length
    pub salt_len: usize,
    /// Iteration count
    pub iterations: u32,
    /// Output key length
    pub key_length: usize,
}

impl Default for Pbkdf2Params {
    fn default() -> Self {
        Self {
            salt: [0u8; 32],
            salt_len: 16,
            iterations: 100000,
            key_length: 32,
        }
    }
}

// =============================================================================
// ERROR TYPES
// =============================================================================

/// Cryptographic error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoError {
    /// Invalid key
    InvalidKey,
    /// Invalid signature
    InvalidSignature,
    /// Signature verification failed
    VerificationFailed,
    /// Invalid certificate
    InvalidCertificate,
    /// Certificate expired
    CertificateExpired,
    /// Certificate not yet valid
    CertificateNotYetValid,
    /// Certificate revoked
    CertificateRevoked,
    /// Invalid chain
    InvalidChain,
    /// Hash mismatch
    HashMismatch,
    /// Encryption failed
    EncryptionFailed,
    /// Decryption failed
    DecryptionFailed,
    /// Buffer too small
    BufferTooSmall,
    /// Invalid parameter
    InvalidParameter,
    /// Algorithm not supported
    UnsupportedAlgorithm,
    /// Random generation failed
    RandomFailed,
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CryptoError::InvalidKey => write!(f, "Invalid key"),
            CryptoError::InvalidSignature => write!(f, "Invalid signature"),
            CryptoError::VerificationFailed => write!(f, "Signature verification failed"),
            CryptoError::InvalidCertificate => write!(f, "Invalid certificate"),
            CryptoError::CertificateExpired => write!(f, "Certificate expired"),
            CryptoError::CertificateNotYetValid => write!(f, "Certificate not yet valid"),
            CryptoError::CertificateRevoked => write!(f, "Certificate revoked"),
            CryptoError::InvalidChain => write!(f, "Invalid certificate chain"),
            CryptoError::HashMismatch => write!(f, "Hash mismatch"),
            CryptoError::EncryptionFailed => write!(f, "Encryption failed"),
            CryptoError::DecryptionFailed => write!(f, "Decryption failed"),
            CryptoError::BufferTooSmall => write!(f, "Buffer too small"),
            CryptoError::InvalidParameter => write!(f, "Invalid parameter"),
            CryptoError::UnsupportedAlgorithm => write!(f, "Algorithm not supported"),
            CryptoError::RandomFailed => write!(f, "Random generation failed"),
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
    fn test_hash_algorithm_sizes() {
        assert_eq!(HashAlgorithm::Sha256.digest_size(), 32);
        assert_eq!(HashAlgorithm::Sha512.digest_size(), 64);
        assert_eq!(HashAlgorithm::Md5.digest_size(), 16);
    }

    #[test]
    fn test_hash_security() {
        assert!(!HashAlgorithm::Md5.is_secure());
        assert!(!HashAlgorithm::Sha1.is_secure());
        assert!(HashAlgorithm::Sha256.is_secure());
    }

    #[test]
    fn test_digest() {
        let data = [0x41u8; 32];
        let digest = Digest::from_bytes(&data);
        assert_eq!(digest.len, 32);
        assert_eq!(digest.as_bytes(), &data);
    }

    #[test]
    fn test_rsa_key_size() {
        assert_eq!(RsaKeySize::Rsa2048.bits(), 2048);
        assert_eq!(RsaKeySize::Rsa2048.bytes(), 256);
    }

    #[test]
    fn test_key_usage() {
        let usage = KeyUsage::SIGN.union(KeyUsage::VERIFY);
        assert!(usage.allows(KeyUsage::SIGN));
        assert!(usage.allows(KeyUsage::VERIFY));
        assert!(!usage.allows(KeyUsage::ENCRYPT));
    }

    #[test]
    fn test_cipher_mode() {
        assert!(CipherMode::Gcm.is_authenticated());
        assert!(!CipherMode::Cbc.is_authenticated());
        assert!(!CipherMode::Ecb.requires_iv());
        assert!(CipherMode::Cbc.requires_iv());
    }

    #[test]
    fn test_validity() {
        let validity = Validity {
            not_before: 1000,
            not_after: 2000,
        };
        assert!(!validity.is_valid_at(500));
        assert!(validity.is_valid_at(1500));
        assert!(!validity.is_valid_at(2500));
    }
}
