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
    /// `BLAKE2b` hash algorithm (512 bits)
    Blake2b,
    /// `BLAKE2s` hash algorithm (256 bits)
    Blake2s,
    /// `BLAKE3` hash algorithm
    Blake3,
}

impl HashAlgorithm {
    /// Get digest size in bytes
    pub const fn digest_size(&self) -> usize {
        match self {
            Self::Md5 => 16,
            Self::Sha1 => 20,
            Self::Sha224 => 28,
            Self::Sha256
            | Self::Sha512_256
            | Self::Sha3_256
            | Self::Sm3
            | Self::Blake2s
            | Self::Blake3 => 32,
            Self::Sha384 | Self::Sha3_384 => 48,
            Self::Sha512 | Self::Sha3_512 | Self::Blake2b => 64,
        }
    }

    /// Get block size in bytes
    pub const fn block_size(&self) -> usize {
        match self {
            Self::Md5
            | Self::Sha1
            | Self::Sha224
            | Self::Sha256
            | Self::Sm3
            | Self::Blake2s
            | Self::Blake3 => 64,
            Self::Sha384 | Self::Sha512 | Self::Sha512_256 | Self::Blake2b => 128,
            Self::Sha3_256 => 136,
            Self::Sha3_384 => 104,
            Self::Sha3_512 => 72,
        }
    }

    /// Check if algorithm is secure
    pub const fn is_secure(&self) -> bool {
        !matches!(self, Self::Md5 | Self::Sha1)
    }

    /// Get OID for this algorithm
    pub const fn oid(&self) -> &'static [u8] {
        match self {
            Self::Md5 => &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x02, 0x05],
            Self::Sha1 => &[0x2B, 0x0E, 0x03, 0x02, 0x1A],
            Self::Sha224 => &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x04],
            Self::Sha256 => &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01],
            Self::Sha384 => &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02],
            Self::Sha512 => &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03],
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
            write!(f, "{b:02x}")?;
        }
        write!(f, ")")
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in &self.bytes[..self.len] {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

// =============================================================================
// SHA-256 CONSTANTS
// =============================================================================

/// SHA-256 round constants
pub const SHA256_K: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

/// SHA-256 initial hash values
pub const SHA256_H: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

/// SHA-512 round constants
pub const SHA512_K: [u64; 80] = [
    0x428a_2f98_d728_ae22,
    0x7137_4491_23ef_65cd,
    0xb5c0_fbcf_ec4d_3b2f,
    0xe9b5_dba5_8189_dbbc,
    0x3956_c25b_f348_b538,
    0x59f1_11f1_b605_d019,
    0x923f_82a4_af19_4f9b,
    0xab1c_5ed5_da6d_8118,
    0xd807_aa98_a303_0242,
    0x1283_5b01_4570_6fbe,
    0x2431_85be_4ee4_b28c,
    0x550c_7dc3_d5ff_b4e2,
    0x72be_5d74_f27b_896f,
    0x80de_b1fe_3b16_96b1,
    0x9bdc_06a7_25c7_1235,
    0xc19b_f174_cf69_2694,
    0xe49b_69c1_9ef1_4ad2,
    0xefbe_4786_384f_25e3,
    0x0fc1_9dc6_8b8c_d5b5,
    0x240c_a1cc_77ac_9c65,
    0x2de9_2c6f_592b_0275,
    0x4a74_84aa_6ea6_e483,
    0x5cb0_a9dc_bd41_fbd4,
    0x76f9_88da_8311_53b5,
    0x983e_5152_ee66_dfab,
    0xa831_c66d_2db4_3210,
    0xb003_27c8_98fb_213f,
    0xbf59_7fc7_beef_0ee4,
    0xc6e0_0bf3_3da8_8fc2,
    0xd5a7_9147_930a_a725,
    0x06ca_6351_e003_826f,
    0x1429_2967_0a0e_6e70,
    0x27b7_0a85_46d2_2ffc,
    0x2e1b_2138_5c26_c926,
    0x4d2c_6dfc_5ac4_2aed,
    0x5338_0d13_9d95_b3df,
    0x650a_7354_8baf_63de,
    0x766a_0abb_3c77_b2a8,
    0x81c2_c92e_47ed_aee6,
    0x9272_2c85_1482_353b,
    0xa2bf_e8a1_4cf1_0364,
    0xa81a_664b_bc42_3001,
    0xc24b_8b70_d0f8_9791,
    0xc76c_51a3_0654_be30,
    0xd192_e819_d6ef_5218,
    0xd699_0624_5565_a910,
    0xf40e_3585_5771_202a,
    0x106a_a070_32bb_d1b8,
    0x19a4_c116_b8d2_d0c8,
    0x1e37_6c08_5141_ab53,
    0x2748_774c_df8e_eb99,
    0x34b0_bcb5_e19b_48a8,
    0x391c_0cb3_c5c9_5a63,
    0x4ed8_aa4a_e341_8acb,
    0x5b9c_ca4f_7763_e373,
    0x682e_6ff3_d6b2_b8a3,
    0x748f_82ee_5def_b2fc,
    0x78a5_636f_4317_2f60,
    0x84c8_7814_a1f0_ab72,
    0x8cc7_0208_1a64_39ec,
    0x90be_fffa_2363_1e28,
    0xa450_6ceb_de82_bde9,
    0xbef9_a3f7_b2c6_7915,
    0xc671_78f2_e372_532b,
    0xca27_3ece_ea26_619c,
    0xd186_b8c7_21c0_c207,
    0xeada_7dd6_cde0_eb1e,
    0xf57d_4f7f_ee6e_d178,
    0x06f0_67aa_7217_6fba,
    0x0a63_7dc5_a2c8_98a6,
    0x113f_9804_bef9_0dae,
    0x1b71_0b35_131c_471b,
    0x28db_77f5_2304_7d84,
    0x32ca_ab7b_40c7_2493,
    0x3c9e_be0a_15c9_bebc,
    0x431d_67c4_9c10_0d4c,
    0x4cc5_d4be_cb3e_42b6,
    0x597f_299c_fc65_7e2a,
    0x5fcb_6fab_3ad6_faec,
    0x6c44_198c_4a47_5817,
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
            Self::RsaPkcs1v15 | Self::RsaPss => 256, // For 2048-bit key
            Self::EcdsaP256 | Self::Ed25519 | Self::Sm2 => 64,
            Self::EcdsaP384 => 96,
            Self::EcdsaP521 => 132,
            Self::Ed448 => 114,
        }
    }

    /// Check if algorithm is elliptic curve based
    pub const fn is_ecc(&self) -> bool {
        !matches!(self, Self::RsaPkcs1v15 | Self::RsaPss)
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
            Self::Rsa1024 => 1024,
            Self::Rsa2048 => 2048,
            Self::Rsa3072 => 3072,
            Self::Rsa4096 => 4096,
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
    /// `BrainpoolP256r1` curve
    BrainpoolP256r1,
    /// `BrainpoolP384r1` curve
    BrainpoolP384r1,
}

impl EllipticCurve {
    /// Get curve size in bits
    pub const fn bits(&self) -> usize {
        match self {
            Self::P256 | Self::Sm2 | Self::Secp256k1 | Self::BrainpoolP256r1 => 256,
            Self::P384 | Self::BrainpoolP384r1 => 384,
            Self::P521 => 521,
            Self::Curve25519 => 255,
            Self::Curve448 => 448,
        }
    }

    /// Get OID
    pub const fn oid(&self) -> &'static [u8] {
        match self {
            Self::P256 => &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x03, 0x01, 0x07],
            Self::P384 => &[0x2B, 0x81, 0x04, 0x00, 0x22],
            Self::P521 => &[0x2B, 0x81, 0x04, 0x00, 0x23],
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
    /// `ChaCha20` stream cipher
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
            Self::Aes128 | Self::Sm4 => 16,
            Self::Aes192 | Self::TripleDes => 24,
            Self::Aes256 | Self::ChaCha20 => 32,
        }
    }

    /// Get block size in bytes
    pub const fn block_size(&self) -> usize {
        match self {
            Self::Aes128 | Self::Aes192 | Self::Aes256 | Self::Sm4 => 16,
            Self::ChaCha20 => 64,
            Self::TripleDes => 8,
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
        matches!(self, Self::Gcm | Self::Ccm | Self::Ocb)
    }

    /// Check if mode requires IV/nonce
    pub const fn requires_iv(&self) -> bool {
        !matches!(self, Self::Ecb)
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
    /// No key usage
    pub const NONE: Self = Self(0);
    /// Key can be used for signing
    pub const SIGN: Self = Self(1 << 0);
    /// Key can be used for verification
    pub const VERIFY: Self = Self(1 << 1);
    /// Key can be used for encryption
    pub const ENCRYPT: Self = Self(1 << 2);
    /// Key can be used for decryption
    pub const DECRYPT: Self = Self(1 << 3);
    /// Key can be used for key wrapping
    pub const KEY_WRAP: Self = Self(1 << 4);
    /// Key can be used for key unwrapping
    pub const KEY_UNWRAP: Self = Self(1 << 5);
    /// Key can be used for key derivation
    pub const DERIVE: Self = Self(1 << 6);

    /// Check if usage is allowed
    pub const fn allows(&self, usage: Self) -> bool {
        (self.0 & usage.0) == usage.0
    }

    /// Combine usages
    #[must_use]
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
    /// Digital signature usage
    pub const DIGITAL_SIGNATURE: Self = Self(1 << 0);
    /// Non-repudiation usage
    pub const NON_REPUDIATION: Self = Self(1 << 1);
    /// Key encipherment usage
    pub const KEY_ENCIPHERMENT: Self = Self(1 << 2);
    /// Data encipherment usage
    pub const DATA_ENCIPHERMENT: Self = Self(1 << 3);
    /// Key agreement usage
    pub const KEY_AGREEMENT: Self = Self(1 << 4);
    /// Key certificate signing usage
    pub const KEY_CERT_SIGN: Self = Self(1 << 5);
    /// CRL signing usage
    pub const CRL_SIGN: Self = Self(1 << 6);
    /// Encipher only usage
    pub const ENCIPHER_ONLY: Self = Self(1 << 7);
    /// Decipher only usage
    pub const DECIPHER_ONLY: Self = Self(1 << 8);

    /// Check if usage contains another
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
            Self::Pk => "PK",
            Self::Kek => "KEK",
            Self::Db => "db",
            Self::Dbx => "dbx",
            Self::Dbr => "dbr",
            Self::Dbt => "dbt",
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
            Self::Sha256 => 32,
            Self::Rsa2048 | Self::Rsa2048Sha256 | Self::Rsa2048Sha1 => 256,
            Self::X509 | Self::X509Sha256 | Self::X509Sha384 | Self::X509Sha512 => 0, // Variable
            Self::Sha1 => 20,
            Self::Sha224 => 28,
            Self::Sha384 => 48,
            Self::Sha512 => 64,
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
    /// SP800-90 `Hash_DRBG` using SHA-256
    Sp80090HashDrbgSha256,
    /// SP800-90 `HMAC_DRBG` using SHA-256
    Sp80090HmacDrbgSha256,
    /// SP800-90 `CTR_DRBG` using AES-256
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
            Self::HmacSha1 => 20,
            Self::HmacSha256 => 32,
            Self::HmacSha384 => 48,
            Self::HmacSha512 => 64,
            Self::CmacAes128 | Self::CmacAes256 | Self::Poly1305 => 16,
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
            iterations: 100_000,
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
