//! Cryptographic Key Management
//!
//! RSA and ECDSA key handling, X.509 certificate parsing for UEFI Secure Boot.

extern crate alloc;
use alloc::vec;
use alloc::vec::Vec;

use super::hash::{HashAlgorithm, Sha256, Sha512};

// =============================================================================
// RSA
// =============================================================================

/// RSA public key
#[derive(Debug, Clone)]
pub struct RsaPublicKey {
    /// Modulus n
    pub modulus: BigUint,
    /// Public exponent e
    pub exponent: BigUint,
    /// Key size in bits
    pub key_bits: usize,
}

impl RsaPublicKey {
    /// Create RSA public key from modulus and exponent
    pub fn new(modulus: &[u8], exponent: &[u8]) -> Self {
        let n = BigUint::from_be_bytes(modulus);
        let e = BigUint::from_be_bytes(exponent);
        let key_bits = modulus.len() * 8;

        Self {
            modulus: n,
            exponent: e,
            key_bits,
        }
    }

    /// Create RSA public key from DER-encoded SubjectPublicKeyInfo
    pub fn from_der(der: &[u8]) -> Result<Self, KeyError> {
        parse_rsa_public_key_info(der)
    }

    /// Verify PKCS#1 v1.5 signature
    pub fn verify_pkcs1_v15(
        &self,
        hash_algorithm: HashAlgorithm,
        message_hash: &[u8],
        signature: &[u8],
    ) -> Result<bool, KeyError> {
        // Convert signature to number
        let sig_int = BigUint::from_be_bytes(signature);

        // RSA verification: m = s^e mod n
        let decrypted = sig_int.mod_pow(&self.exponent, &self.modulus);
        let decrypted_bytes = decrypted.to_be_bytes_padded(self.key_bits / 8);

        // Verify PKCS#1 v1.5 padding
        verify_pkcs1_padding(&decrypted_bytes, hash_algorithm, message_hash)
    }

    /// Verify PSS signature
    pub fn verify_pss(
        &self,
        hash_algorithm: HashAlgorithm,
        message_hash: &[u8],
        signature: &[u8],
        salt_len: Option<usize>,
    ) -> Result<bool, KeyError> {
        // Convert signature to number
        let sig_int = BigUint::from_be_bytes(signature);

        // RSA verification: m = s^e mod n
        let em = sig_int.mod_pow(&self.exponent, &self.modulus);
        let em_bytes = em.to_be_bytes_padded(self.key_bits / 8);

        // Verify PSS encoding
        verify_pss_encoding(&em_bytes, hash_algorithm, message_hash, salt_len)
    }
}

/// Verify PKCS#1 v1.5 padding
fn verify_pkcs1_padding(
    padded: &[u8],
    hash_alg: HashAlgorithm,
    expected_hash: &[u8],
) -> Result<bool, KeyError> {
    if padded.len() < 11 {
        return Err(KeyError::InvalidPadding);
    }

    // Check 0x00 0x01 prefix
    if padded[0] != 0x00 || padded[1] != 0x01 {
        return Ok(false);
    }

    // Find end of 0xFF padding
    let mut idx = 2;
    while idx < padded.len() && padded[idx] == 0xFF {
        idx += 1;
    }

    // Must have at least 8 bytes of padding
    if idx < 10 {
        return Ok(false);
    }

    // Check separator
    if idx >= padded.len() || padded[idx] != 0x00 {
        return Ok(false);
    }
    idx += 1;

    // Parse DigestInfo
    let digest_info = &padded[idx..];

    // Get expected DigestInfo prefix
    let prefix = get_digest_info_prefix(hash_alg);

    // Verify prefix
    if digest_info.len() < prefix.len() + expected_hash.len() {
        return Ok(false);
    }

    if &digest_info[..prefix.len()] != prefix {
        return Ok(false);
    }

    // Verify hash
    let actual_hash = &digest_info[prefix.len()..prefix.len() + expected_hash.len()];
    Ok(constant_time_eq(actual_hash, expected_hash))
}

/// Get DigestInfo prefix for algorithm
fn get_digest_info_prefix(alg: HashAlgorithm) -> &'static [u8] {
    match alg {
        HashAlgorithm::Sha256 => &[
            0x30, 0x31, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02,
            0x01, 0x05, 0x00, 0x04, 0x20,
        ],
        HashAlgorithm::Sha384 => &[
            0x30, 0x41, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02,
            0x02, 0x05, 0x00, 0x04, 0x30,
        ],
        HashAlgorithm::Sha512 => &[
            0x30, 0x51, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02,
            0x03, 0x05, 0x00, 0x04, 0x40,
        ],
    }
}

/// Verify PSS encoding (RSASSA-PSS)
fn verify_pss_encoding(
    em: &[u8],
    hash_alg: HashAlgorithm,
    message_hash: &[u8],
    salt_len: Option<usize>,
) -> Result<bool, KeyError> {
    let h_len = hash_alg.output_size();
    let salt_len = salt_len.unwrap_or(h_len);
    let em_len = em.len();

    // Check minimum length
    if em_len < h_len + salt_len + 2 {
        return Err(KeyError::InvalidPadding);
    }

    // Check trailer byte
    if em[em_len - 1] != 0xbc {
        return Ok(false);
    }

    // Extract maskedDB and H
    let db_len = em_len - h_len - 1;
    let masked_db = &em[..db_len];
    let h = &em[db_len..em_len - 1];

    // Check that top bits of maskedDB are zero
    let top_bits = 8 * em_len - (em_len * 8 - 1);
    if top_bits > 0 && (masked_db[0] >> (8 - top_bits)) != 0 {
        return Ok(false);
    }

    // MGF1 to unmask DB
    let mut db = masked_db.to_vec();
    mgf1_xor(&mut db, h, hash_alg);

    // Clear top bits
    if top_bits > 0 {
        db[0] &= 0xFF >> top_bits;
    }

    // Check padding
    let ps_len = em_len - h_len - salt_len - 2;
    for &b in &db[..ps_len] {
        if b != 0 {
            return Ok(false);
        }
    }

    if db[ps_len] != 0x01 {
        return Ok(false);
    }

    // Extract salt
    let salt = &db[db_len - salt_len..];

    // Compute H' = Hash(0x00000000 || mHash || salt)
    let mut m_prime = vec![0u8; 8];
    m_prime.extend_from_slice(message_hash);
    m_prime.extend_from_slice(salt);

    let h_prime = hash_data(hash_alg, &m_prime);

    Ok(constant_time_eq(&h_prime, h))
}

/// MGF1 with XOR
fn mgf1_xor(data: &mut [u8], seed: &[u8], hash_alg: HashAlgorithm) {
    let h_len = hash_alg.output_size();
    let mut counter = [0u8; 4];
    let mut offset = 0;

    while offset < data.len() {
        let mut input = seed.to_vec();
        input.extend_from_slice(&counter);

        let hash = hash_data(hash_alg, &input);

        let to_xor = core::cmp::min(h_len, data.len() - offset);
        for i in 0..to_xor {
            data[offset + i] ^= hash[i];
        }

        offset += to_xor;

        // Increment counter
        for i in (0..4).rev() {
            counter[i] = counter[i].wrapping_add(1);
            if counter[i] != 0 {
                break;
            }
        }
    }
}

// =============================================================================
// ECDSA
// =============================================================================

/// Elliptic curve type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcCurve {
    /// NIST P-256 (secp256r1)
    P256,
    /// NIST P-384 (secp384r1)
    P384,
    /// NIST P-521 (secp521r1)
    P521,
}

impl EcCurve {
    /// Get curve parameters
    pub fn params(&self) -> EcCurveParams {
        match self {
            Self::P256 => P256_PARAMS,
            Self::P384 => P384_PARAMS,
            Self::P521 => P521_PARAMS,
        }
    }

    /// Get OID for curve
    pub fn oid(&self) -> &'static [u8] {
        match self {
            // 1.2.840.10045.3.1.7
            Self::P256 => &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07],
            // 1.3.132.0.34
            Self::P384 => &[0x2b, 0x81, 0x04, 0x00, 0x22],
            // 1.3.132.0.35
            Self::P521 => &[0x2b, 0x81, 0x04, 0x00, 0x23],
        }
    }

    /// Get key size in bytes
    pub fn key_size(&self) -> usize {
        match self {
            Self::P256 => 32,
            Self::P384 => 48,
            Self::P521 => 66,
        }
    }
}

/// Curve parameters (simplified)
#[derive(Clone, Copy)]
pub struct EcCurveParams {
    /// Key size in bytes
    pub key_size: usize,
    /// Order n in bytes (for signature verification)
    pub order: &'static [u8],
}

const P256_PARAMS: EcCurveParams = EcCurveParams {
    key_size: 32,
    order: &[
        0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xbc, 0xe6, 0xfa, 0xad, 0xa7, 0x17, 0x9e, 0x84, 0xf3, 0xb9, 0xca, 0xc2, 0xfc, 0x63,
        0x25, 0x51,
    ],
};

const P384_PARAMS: EcCurveParams = EcCurveParams {
    key_size: 48,
    order: &[
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xc7, 0x63, 0x4d, 0x81, 0xf4, 0x37,
        0x2d, 0xdf, 0x58, 0x1a, 0x0d, 0xb2, 0x48, 0xb0, 0xa7, 0x7a, 0xec, 0xec, 0x19, 0x6a, 0xcc,
        0xc5, 0x29, 0x73,
    ],
};

const P521_PARAMS: EcCurveParams = EcCurveParams {
    key_size: 66,
    order: &[
        0x01, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xfa, 0x51, 0x86, 0x87, 0x83, 0xbf, 0x2f, 0x96, 0x6b, 0x7f, 0xcc, 0x01,
        0x48, 0xf7, 0x09, 0xa5, 0xd0, 0x3b, 0xb5, 0xc9, 0xb8, 0x89, 0x9c, 0x47, 0xae, 0xbb, 0x6f,
        0xb7, 0x1e, 0x91, 0x38, 0x64, 0x09,
    ],
};

/// ECDSA public key
#[derive(Debug, Clone)]
pub struct EcPublicKey {
    /// Curve type
    pub curve: EcCurve,
    /// X coordinate
    pub x: BigUint,
    /// Y coordinate
    pub y: BigUint,
}

impl EcPublicKey {
    /// Create from uncompressed point (0x04 || x || y)
    pub fn from_uncompressed(curve: EcCurve, point: &[u8]) -> Result<Self, KeyError> {
        let key_size = curve.key_size();

        if point.len() != 1 + 2 * key_size {
            return Err(KeyError::InvalidKeyFormat);
        }

        if point[0] != 0x04 {
            return Err(KeyError::InvalidKeyFormat);
        }

        let x = BigUint::from_be_bytes(&point[1..=key_size]);
        let y = BigUint::from_be_bytes(&point[1 + key_size..]);

        Ok(Self { curve, x, y })
    }

    /// Create from DER-encoded SubjectPublicKeyInfo
    pub fn from_der(der: &[u8]) -> Result<Self, KeyError> {
        parse_ec_public_key_info(der)
    }

    /// Verify ECDSA signature
    ///
    /// Note: This is a simplified placeholder - real ECDSA verification requires
    /// full elliptic curve arithmetic which is complex to implement safely.
    pub fn verify(
        &self,
        _hash_algorithm: HashAlgorithm,
        _message_hash: &[u8],
        _signature: &EcdsaSignature,
    ) -> Result<bool, KeyError> {
        // Full ECDSA verification requires:
        // 1. Verify r, s are in [1, n-1]
        // 2. Compute w = s^-1 mod n
        // 3. Compute u1 = e*w mod n, u2 = r*w mod n (e = hash)
        // 4. Compute R = u1*G + u2*Q (point multiplication and addition)
        // 5. Verify r == R.x mod n
        //
        // This requires full big integer and EC point arithmetic.
        // In a real implementation, this would be delegated to UEFI crypto protocols
        // or a dedicated crypto library.

        Err(KeyError::NotImplemented)
    }
}

/// ECDSA signature (r, s)
#[derive(Debug, Clone)]
pub struct EcdsaSignature {
    /// r component
    pub r: BigUint,
    /// s component
    pub s: BigUint,
}

impl EcdsaSignature {
    /// Create from r and s values
    pub fn new(r: BigUint, s: BigUint) -> Self {
        Self { r, s }
    }

    /// Parse from DER-encoded signature
    pub fn from_der(der: &[u8]) -> Result<Self, KeyError> {
        parse_ecdsa_signature(der)
    }
}

// =============================================================================
// X.509 CERTIFICATE
// =============================================================================

/// X.509 certificate version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum X509Version {
    V1 = 0,
    V2 = 1,
    V3 = 2,
}

/// X.509 certificate
#[derive(Debug, Clone)]
pub struct X509Certificate {
    /// Version
    pub version: X509Version,
    /// Serial number
    pub serial_number: Vec<u8>,
    /// Signature algorithm OID
    pub signature_algorithm: Vec<u8>,
    /// Issuer (raw DER)
    pub issuer: Vec<u8>,
    /// Subject (raw DER)
    pub subject: Vec<u8>,
    /// Not Before (Unix timestamp)
    pub not_before: u64,
    /// Not After (Unix timestamp)
    pub not_after: u64,
    /// Public key
    pub public_key: PublicKey,
    /// TBS certificate (for signature verification)
    pub tbs_certificate: Vec<u8>,
    /// Signature
    pub signature: Vec<u8>,
    /// Extensions
    pub extensions: Vec<X509Extension>,
}

/// X.509 extension
#[derive(Debug, Clone)]
pub struct X509Extension {
    /// OID
    pub oid: Vec<u8>,
    /// Critical flag
    pub critical: bool,
    /// Value
    pub value: Vec<u8>,
}

/// Public key types
#[derive(Debug, Clone)]
pub enum PublicKey {
    /// RSA public key
    Rsa(RsaPublicKey),
    /// ECDSA public key
    Ec(EcPublicKey),
}

impl X509Certificate {
    /// Parse X.509 certificate from DER
    pub fn from_der(der: &[u8]) -> Result<Self, KeyError> {
        parse_x509_certificate(der)
    }

    /// Verify certificate signature against issuer's public key
    pub fn verify_signature(&self, issuer_key: &PublicKey) -> Result<bool, KeyError> {
        // Determine hash algorithm from signature algorithm OID
        let hash_alg = get_signature_hash_algorithm(&self.signature_algorithm)?;

        // Compute hash of TBS certificate
        let tbs_hash = hash_data(hash_alg, &self.tbs_certificate);

        // Verify signature
        match issuer_key {
            PublicKey::Rsa(rsa_key) => {
                rsa_key.verify_pkcs1_v15(hash_alg, &tbs_hash, &self.signature)
            },
            PublicKey::Ec(ec_key) => {
                let sig = EcdsaSignature::from_der(&self.signature)?;
                ec_key.verify(hash_alg, &tbs_hash, &sig)
            },
        }
    }

    /// Check if certificate is self-signed
    pub fn is_self_signed(&self) -> bool {
        self.issuer == self.subject
    }

    /// Check if certificate is valid at given time
    pub fn is_valid_at(&self, time: u64) -> bool {
        time >= self.not_before && time <= self.not_after
    }

    /// Get key usage extension flags (if present)
    pub fn key_usage(&self) -> Option<u16> {
        const KEY_USAGE_OID: &[u8] = &[0x55, 0x1d, 0x0f]; // 2.5.29.15

        for ext in &self.extensions {
            if ext.oid == KEY_USAGE_OID {
                // Parse BIT STRING
                if ext.value.len() >= 3 && ext.value[0] == 0x03 {
                    let unused_bits = ext.value[2];
                    let mut usage = 0u16;
                    if ext.value.len() > 3 {
                        usage = (ext.value[3] as u16) << 8;
                    }
                    if ext.value.len() > 4 {
                        usage |= ext.value[4] as u16;
                    }
                    usage >>= unused_bits;
                    return Some(usage);
                }
            }
        }
        None
    }

    /// Check if certificate is a CA (has basicConstraints CA:TRUE)
    pub fn is_ca(&self) -> bool {
        const BASIC_CONSTRAINTS_OID: &[u8] = &[0x55, 0x1d, 0x13]; // 2.5.29.19

        for ext in &self.extensions {
            if ext.oid == BASIC_CONSTRAINTS_OID {
                // Parse SEQUENCE { BOOLEAN, INTEGER OPTIONAL }
                if ext.value.len() >= 5 && ext.value[0] == 0x30 && ext.value[2] == 0x01 {
                    return ext.value[4] == 0xff;
                }
            }
        }
        false
    }
}

/// Key usage flags
pub mod key_usage {
    pub const DIGITAL_SIGNATURE: u16 = 0x8000;
    pub const NON_REPUDIATION: u16 = 0x4000;
    pub const KEY_ENCIPHERMENT: u16 = 0x2000;
    pub const DATA_ENCIPHERMENT: u16 = 0x1000;
    pub const KEY_AGREEMENT: u16 = 0x0800;
    pub const KEY_CERT_SIGN: u16 = 0x0400;
    pub const CRL_SIGN: u16 = 0x0200;
    pub const ENCIPHER_ONLY: u16 = 0x0100;
    pub const DECIPHER_ONLY: u16 = 0x0080;
}

// =============================================================================
// BIG INTEGER (simplified)
// =============================================================================

/// Simplified big unsigned integer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BigUint {
    /// Limbs in little-endian order
    limbs: Vec<u64>,
}

impl BigUint {
    /// Create zero
    pub fn zero() -> Self {
        Self { limbs: vec![0] }
    }

    /// Create one
    pub fn one() -> Self {
        Self { limbs: vec![1] }
    }

    /// Create from big-endian bytes
    pub fn from_be_bytes(bytes: &[u8]) -> Self {
        let mut limbs = Vec::new();

        // Skip leading zeros
        let start = bytes.iter().position(|&b| b != 0).unwrap_or(bytes.len());
        let bytes = &bytes[start..];

        if bytes.is_empty() {
            return Self::zero();
        }

        // Process 8 bytes at a time
        let mut i = bytes.len();
        while i > 0 {
            let end = i;
            let start = i.saturating_sub(8);
            let mut limb = 0u64;

            for (j, &b) in bytes[start..end].iter().enumerate() {
                let shift = (end - start - 1 - j) * 8;
                limb |= (b as u64) << shift;
            }

            limbs.push(limb);
            i = start;
        }

        // Ensure at least one limb
        if limbs.is_empty() {
            limbs.push(0);
        }

        Self { limbs }
    }

    /// Convert to big-endian bytes
    pub fn to_be_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        for &limb in self.limbs.iter().rev() {
            bytes.extend_from_slice(&limb.to_be_bytes());
        }

        // Remove leading zeros
        while bytes.len() > 1 && bytes[0] == 0 {
            bytes.remove(0);
        }

        bytes
    }

    /// Convert to big-endian bytes with padding
    pub fn to_be_bytes_padded(&self, len: usize) -> Vec<u8> {
        let bytes = self.to_be_bytes();

        if bytes.len() >= len {
            bytes[bytes.len() - len..].to_vec()
        } else {
            let mut padded = vec![0u8; len - bytes.len()];
            padded.extend_from_slice(&bytes);
            padded
        }
    }

    /// Modular exponentiation: self^exp mod modulus
    pub fn mod_pow(&self, exp: &BigUint, modulus: &BigUint) -> BigUint {
        if modulus.is_zero() {
            return BigUint::zero();
        }

        let mut result = BigUint::one();
        let mut base = self.mod_reduce(modulus);
        let exp_bytes = exp.to_be_bytes();

        // Square-and-multiply from LSB to MSB
        for &byte in exp_bytes.iter().rev() {
            for bit in 0..8 {
                if (byte >> bit) & 1 == 1 {
                    result = result.mul(&base).mod_reduce(modulus);
                }
                base = base.mul(&base).mod_reduce(modulus);
            }
        }

        result
    }

    /// Check if zero
    pub fn is_zero(&self) -> bool {
        self.limbs.iter().all(|&l| l == 0)
    }

    /// Multiplication
    pub fn mul(&self, other: &BigUint) -> BigUint {
        let n = self.limbs.len();
        let m = other.limbs.len();
        let mut result = vec![0u64; n + m];

        for i in 0..n {
            let mut carry = 0u128;
            for j in 0..m {
                let product = (self.limbs[i] as u128) * (other.limbs[j] as u128)
                    + (result[i + j] as u128)
                    + carry;
                result[i + j] = product as u64;
                carry = product >> 64;
            }
            if i + m < result.len() {
                result[i + m] = result[i + m].wrapping_add(carry as u64);
            }
        }

        // Trim leading zeros
        while result.len() > 1 && result.last() == Some(&0) {
            result.pop();
        }

        BigUint { limbs: result }
    }

    /// Modular reduction (simplified)
    pub fn mod_reduce(&self, modulus: &BigUint) -> BigUint {
        // Simple reduction by repeated subtraction
        // In production, use Barrett or Montgomery reduction
        let mut result = self.clone();

        while result.compare(modulus) >= 0 {
            result = result.sub(modulus);
        }

        result
    }

    /// Subtraction (self - other), assumes self >= other
    pub fn sub(&self, other: &BigUint) -> BigUint {
        let n = core::cmp::max(self.limbs.len(), other.limbs.len());
        let mut result = vec![0u64; n];
        let mut borrow = 0i128;

        for (i, res) in result.iter_mut().enumerate().take(n) {
            let a = self.limbs.get(i).copied().unwrap_or(0) as i128;
            let b = other.limbs.get(i).copied().unwrap_or(0) as i128;
            let diff = a - b - borrow;

            if diff < 0 {
                *res = (diff + (1i128 << 64)) as u64;
                borrow = 1;
            } else {
                *res = diff as u64;
                borrow = 0;
            }
        }

        // Trim leading zeros
        while result.len() > 1 && result.last() == Some(&0) {
            result.pop();
        }

        BigUint { limbs: result }
    }

    /// Compare: returns -1, 0, or 1
    pub fn compare(&self, other: &BigUint) -> i32 {
        if self.limbs.len() != other.limbs.len() {
            return if self.limbs.len() > other.limbs.len() {
                1
            } else {
                -1
            };
        }

        for i in (0..self.limbs.len()).rev() {
            match self.limbs[i].cmp(&other.limbs[i]) {
                core::cmp::Ordering::Greater => return 1,
                core::cmp::Ordering::Less => return -1,
                core::cmp::Ordering::Equal => {},
            }
        }

        0
    }
}

// =============================================================================
// ASN.1 / DER PARSING
// =============================================================================

/// ASN.1 tag types
#[allow(dead_code)]
mod asn1_tag {
    pub const BOOLEAN: u8 = 0x01;
    pub const INTEGER: u8 = 0x02;
    pub const BIT_STRING: u8 = 0x03;
    pub const OCTET_STRING: u8 = 0x04;
    pub const NULL: u8 = 0x05;
    pub const OID: u8 = 0x06;
    pub const UTF8_STRING: u8 = 0x0c;
    pub const SEQUENCE: u8 = 0x30;
    pub const SET: u8 = 0x31;
    pub const PRINTABLE_STRING: u8 = 0x13;
    pub const IA5_STRING: u8 = 0x16;
    pub const UTC_TIME: u8 = 0x17;
    pub const GENERALIZED_TIME: u8 = 0x18;
    pub const CONTEXT_0: u8 = 0xa0;
    pub const CONTEXT_3: u8 = 0xa3;
}

/// Parse DER length
fn parse_der_length(data: &[u8]) -> Result<(usize, usize), KeyError> {
    if data.is_empty() {
        return Err(KeyError::InvalidDer);
    }

    let first = data[0];

    match first.cmp(&0x80) {
        core::cmp::Ordering::Less => Ok((first as usize, 1)),
        core::cmp::Ordering::Equal => Err(KeyError::InvalidDer), // Indefinite length not supported
        core::cmp::Ordering::Greater => {
            let num_octets = (first & 0x7f) as usize;
            if num_octets > 4 || data.len() < 1 + num_octets {
                return Err(KeyError::InvalidDer);
            }

            let mut length = 0usize;
            for i in 0..num_octets {
                length = (length << 8) | (data[1 + i] as usize);
            }

            Ok((length, 1 + num_octets))
        }
    }
}

/// Parse DER element (returns tag, content, bytes consumed)
fn parse_der_element(data: &[u8]) -> Result<(u8, &[u8], usize), KeyError> {
    if data.is_empty() {
        return Err(KeyError::InvalidDer);
    }

    let tag = data[0];
    let (length, len_size) = parse_der_length(&data[1..])?;
    let header_size = 1 + len_size;

    if data.len() < header_size + length {
        return Err(KeyError::InvalidDer);
    }

    let content = &data[header_size..header_size + length];
    Ok((tag, content, header_size + length))
}

/// Parse RSA SubjectPublicKeyInfo
fn parse_rsa_public_key_info(der: &[u8]) -> Result<RsaPublicKey, KeyError> {
    // SEQUENCE { algorithm, subjectPublicKey }
    let (tag, content, _) = parse_der_element(der)?;
    if tag != asn1_tag::SEQUENCE {
        return Err(KeyError::InvalidKeyFormat);
    }

    let mut offset = 0;

    // AlgorithmIdentifier
    let (tag, _alg, consumed) = parse_der_element(&content[offset..])?;
    if tag != asn1_tag::SEQUENCE {
        return Err(KeyError::InvalidKeyFormat);
    }
    offset += consumed;

    // subjectPublicKey (BIT STRING)
    let (tag, pub_key, _) = parse_der_element(&content[offset..])?;
    if tag != asn1_tag::BIT_STRING {
        return Err(KeyError::InvalidKeyFormat);
    }

    // Skip unused bits byte
    if pub_key.is_empty() {
        return Err(KeyError::InvalidKeyFormat);
    }
    let pub_key = &pub_key[1..];

    // Parse RSA public key: SEQUENCE { modulus INTEGER, publicExponent INTEGER }
    let (tag, rsa_content, _) = parse_der_element(pub_key)?;
    if tag != asn1_tag::SEQUENCE {
        return Err(KeyError::InvalidKeyFormat);
    }

    let mut rsa_offset = 0;

    // Modulus
    let (tag, modulus, consumed) = parse_der_element(&rsa_content[rsa_offset..])?;
    if tag != asn1_tag::INTEGER {
        return Err(KeyError::InvalidKeyFormat);
    }
    rsa_offset += consumed;

    // Public exponent
    let (tag, exponent, _) = parse_der_element(&rsa_content[rsa_offset..])?;
    if tag != asn1_tag::INTEGER {
        return Err(KeyError::InvalidKeyFormat);
    }

    // Remove leading zero if present (for unsigned representation)
    let modulus = if !modulus.is_empty() && modulus[0] == 0 {
        &modulus[1..]
    } else {
        modulus
    };

    let exponent = if !exponent.is_empty() && exponent[0] == 0 {
        &exponent[1..]
    } else {
        exponent
    };

    Ok(RsaPublicKey::new(modulus, exponent))
}

/// Parse EC SubjectPublicKeyInfo
fn parse_ec_public_key_info(der: &[u8]) -> Result<EcPublicKey, KeyError> {
    // SEQUENCE { algorithm, subjectPublicKey }
    let (tag, content, _) = parse_der_element(der)?;
    if tag != asn1_tag::SEQUENCE {
        return Err(KeyError::InvalidKeyFormat);
    }

    let mut offset = 0;

    // AlgorithmIdentifier SEQUENCE { OID, curve OID }
    let (tag, alg, consumed) = parse_der_element(&content[offset..])?;
    if tag != asn1_tag::SEQUENCE {
        return Err(KeyError::InvalidKeyFormat);
    }
    offset += consumed;

    // Parse algorithm to get curve
    let mut alg_offset = 0;
    let (tag, _algo_oid, consumed) = parse_der_element(&alg[alg_offset..])?;
    if tag != asn1_tag::OID {
        return Err(KeyError::InvalidKeyFormat);
    }
    alg_offset += consumed;

    // Curve OID
    let (tag, curve_oid, _) = parse_der_element(&alg[alg_offset..])?;
    if tag != asn1_tag::OID {
        return Err(KeyError::InvalidKeyFormat);
    }

    // Determine curve
    let curve = if curve_oid == EcCurve::P256.oid() {
        EcCurve::P256
    } else if curve_oid == EcCurve::P384.oid() {
        EcCurve::P384
    } else if curve_oid == EcCurve::P521.oid() {
        EcCurve::P521
    } else {
        return Err(KeyError::UnsupportedCurve);
    };

    // subjectPublicKey (BIT STRING)
    let (tag, pub_key, _) = parse_der_element(&content[offset..])?;
    if tag != asn1_tag::BIT_STRING {
        return Err(KeyError::InvalidKeyFormat);
    }

    // Skip unused bits byte
    if pub_key.is_empty() {
        return Err(KeyError::InvalidKeyFormat);
    }
    let point = &pub_key[1..];

    EcPublicKey::from_uncompressed(curve, point)
}

/// Parse ECDSA signature (DER encoded)
fn parse_ecdsa_signature(der: &[u8]) -> Result<EcdsaSignature, KeyError> {
    // SEQUENCE { r INTEGER, s INTEGER }
    let (tag, content, _) = parse_der_element(der)?;
    if tag != asn1_tag::SEQUENCE {
        return Err(KeyError::InvalidSignature);
    }

    let mut offset = 0;

    // r
    let (tag, r_bytes, consumed) = parse_der_element(&content[offset..])?;
    if tag != asn1_tag::INTEGER {
        return Err(KeyError::InvalidSignature);
    }
    offset += consumed;

    // s
    let (tag, s_bytes, _) = parse_der_element(&content[offset..])?;
    if tag != asn1_tag::INTEGER {
        return Err(KeyError::InvalidSignature);
    }

    let r = BigUint::from_be_bytes(r_bytes);
    let s = BigUint::from_be_bytes(s_bytes);

    Ok(EcdsaSignature::new(r, s))
}

/// Parse X.509 certificate
fn parse_x509_certificate(der: &[u8]) -> Result<X509Certificate, KeyError> {
    // Certificate ::= SEQUENCE {
    //   tbsCertificate TBSCertificate,
    //   signatureAlgorithm AlgorithmIdentifier,
    //   signatureValue BIT STRING
    // }

    let (tag, content, _) = parse_der_element(der)?;
    if tag != asn1_tag::SEQUENCE {
        return Err(KeyError::InvalidCertificate);
    }

    let mut offset = 0;

    // TBSCertificate
    let (tag, tbs, consumed) = parse_der_element(&content[offset..])?;
    if tag != asn1_tag::SEQUENCE {
        return Err(KeyError::InvalidCertificate);
    }
    let tbs_start = offset;
    let tbs_end = offset + consumed;
    offset += consumed;

    // SignatureAlgorithm
    let (tag, sig_alg, consumed) = parse_der_element(&content[offset..])?;
    if tag != asn1_tag::SEQUENCE {
        return Err(KeyError::InvalidCertificate);
    }
    offset += consumed;

    // Parse signature algorithm OID
    let (tag, sig_alg_oid, _) = parse_der_element(sig_alg)?;
    if tag != asn1_tag::OID {
        return Err(KeyError::InvalidCertificate);
    }

    // SignatureValue
    let (tag, sig_value, _) = parse_der_element(&content[offset..])?;
    if tag != asn1_tag::BIT_STRING {
        return Err(KeyError::InvalidCertificate);
    }

    // Skip unused bits
    let signature = if sig_value.is_empty() {
        Vec::new()
    } else {
        sig_value[1..].to_vec()
    };

    // Parse TBSCertificate
    let mut tbs_offset = 0;

    // Version (optional, context tag 0)
    let version = if !tbs.is_empty() && tbs[0] == asn1_tag::CONTEXT_0 {
        let (_, ver_content, consumed) = parse_der_element(&tbs[tbs_offset..])?;
        tbs_offset += consumed;

        let (_, ver_int, _) = parse_der_element(ver_content)?;
        match ver_int.first() {
            Some(0) => X509Version::V1,
            Some(1) => X509Version::V2,
            Some(2) => X509Version::V3,
            _ => X509Version::V1,
        }
    } else {
        X509Version::V1
    };

    // Serial number
    let (tag, serial, consumed) = parse_der_element(&tbs[tbs_offset..])?;
    if tag != asn1_tag::INTEGER {
        return Err(KeyError::InvalidCertificate);
    }
    tbs_offset += consumed;

    // Signature algorithm (in TBS)
    let (tag, _, consumed) = parse_der_element(&tbs[tbs_offset..])?;
    if tag != asn1_tag::SEQUENCE {
        return Err(KeyError::InvalidCertificate);
    }
    tbs_offset += consumed;

    // Issuer
    let issuer_start = tbs_offset;
    let (tag, _, consumed) = parse_der_element(&tbs[tbs_offset..])?;
    if tag != asn1_tag::SEQUENCE {
        return Err(KeyError::InvalidCertificate);
    }
    let issuer = tbs[issuer_start..tbs_offset + consumed].to_vec();
    tbs_offset += consumed;

    // Validity
    let (tag, validity, consumed) = parse_der_element(&tbs[tbs_offset..])?;
    if tag != asn1_tag::SEQUENCE {
        return Err(KeyError::InvalidCertificate);
    }
    tbs_offset += consumed;

    // Parse validity times
    let (not_before, not_after) = parse_validity(validity)?;

    // Subject
    let subject_start = tbs_offset;
    let (tag, _, consumed) = parse_der_element(&tbs[tbs_offset..])?;
    if tag != asn1_tag::SEQUENCE {
        return Err(KeyError::InvalidCertificate);
    }
    let subject = tbs[subject_start..tbs_offset + consumed].to_vec();
    tbs_offset += consumed;

    // SubjectPublicKeyInfo
    let (tag, spki, consumed) = parse_der_element(&tbs[tbs_offset..])?;
    if tag != asn1_tag::SEQUENCE {
        return Err(KeyError::InvalidCertificate);
    }
    tbs_offset += consumed;

    // Parse public key
    let public_key = parse_subject_public_key_info(spki)?;

    // Extensions (V3 only, context tag 3)
    let mut extensions = Vec::new();
    if version == X509Version::V3 && tbs_offset < tbs.len() {
        while tbs_offset < tbs.len() {
            if tbs[tbs_offset] == asn1_tag::CONTEXT_3 {
                let (_, ext_seq, consumed) = parse_der_element(&tbs[tbs_offset..])?;
                tbs_offset += consumed;

                // Parse extensions sequence
                let (tag, ext_list, _) = parse_der_element(ext_seq)?;
                if tag == asn1_tag::SEQUENCE {
                    extensions = parse_extensions(ext_list)?;
                }
            } else {
                // Skip unknown elements
                let (_, _, consumed) = parse_der_element(&tbs[tbs_offset..])?;
                tbs_offset += consumed;
            }
        }
    }

    Ok(X509Certificate {
        version,
        serial_number: serial.to_vec(),
        signature_algorithm: sig_alg_oid.to_vec(),
        issuer,
        subject,
        not_before,
        not_after,
        public_key,
        tbs_certificate: content[tbs_start..tbs_end].to_vec(),
        signature,
        extensions,
    })
}

/// Parse validity times
fn parse_validity(data: &[u8]) -> Result<(u64, u64), KeyError> {
    let mut offset = 0;

    // Not Before
    let (_, not_before, consumed) = parse_der_element(&data[offset..])?;
    let not_before_ts = parse_time(not_before)?;
    offset += consumed;

    // Not After
    let (_, not_after, _) = parse_der_element(&data[offset..])?;
    let not_after_ts = parse_time(not_after)?;

    Ok((not_before_ts, not_after_ts))
}

/// Parse time (UTCTime or GeneralizedTime)
#[allow(clippy::unnecessary_wraps)]
fn parse_time(data: &[u8]) -> Result<u64, KeyError> {
    // Simplified: just return 0 for now
    // Real implementation would parse YYMMDDHHMMSSZ or YYYYMMDDHHMMSSZ
    let _ = data;
    Ok(0)
}

/// Parse SubjectPublicKeyInfo
fn parse_subject_public_key_info(spki: &[u8]) -> Result<PublicKey, KeyError> {
    // Try RSA first
    if let Ok(rsa) = parse_rsa_public_key_info(spki) {
        return Ok(PublicKey::Rsa(rsa));
    }

    // Try EC
    if let Ok(ec) = parse_ec_public_key_info(spki) {
        return Ok(PublicKey::Ec(ec));
    }

    Err(KeyError::UnsupportedKeyType)
}

/// Parse extensions
fn parse_extensions(data: &[u8]) -> Result<Vec<X509Extension>, KeyError> {
    let mut extensions = Vec::new();
    let mut offset = 0;

    while offset < data.len() {
        let (tag, ext, consumed) = parse_der_element(&data[offset..])?;
        if tag != asn1_tag::SEQUENCE {
            offset += consumed;
            continue;
        }

        let mut ext_offset = 0;

        // OID
        let (tag, oid, consumed) = parse_der_element(&ext[ext_offset..])?;
        if tag != asn1_tag::OID {
            offset += consumed;
            continue;
        }
        ext_offset += consumed;

        // Critical (optional)
        let mut critical = false;
        if ext_offset < ext.len() && ext[ext_offset] == asn1_tag::BOOLEAN {
            let (_, crit_val, consumed) = parse_der_element(&ext[ext_offset..])?;
            critical = !crit_val.is_empty() && crit_val[0] != 0;
            ext_offset += consumed;
        }

        // Value (OCTET STRING)
        let (tag, value, _) = parse_der_element(&ext[ext_offset..])?;
        if tag != asn1_tag::OCTET_STRING {
            offset += consumed;
            continue;
        }

        extensions.push(X509Extension {
            oid: oid.to_vec(),
            critical,
            value: value.to_vec(),
        });

        offset += consumed;
    }

    Ok(extensions)
}

/// Get hash algorithm from signature algorithm OID
fn get_signature_hash_algorithm(oid: &[u8]) -> Result<HashAlgorithm, KeyError> {
    // sha256WithRSAEncryption: 1.2.840.113549.1.1.11
    const SHA256_RSA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
    // sha384WithRSAEncryption: 1.2.840.113549.1.1.12
    const SHA384_RSA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0c];
    // sha512WithRSAEncryption: 1.2.840.113549.1.1.13
    const SHA512_RSA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0d];
    // ecdsa-with-SHA256: 1.2.840.10045.4.3.2
    const SHA256_ECDSA: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02];
    // ecdsa-with-SHA384: 1.2.840.10045.4.3.3
    const SHA384_ECDSA: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x03];
    // ecdsa-with-SHA512: 1.2.840.10045.4.3.4
    const SHA512_ECDSA: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x04];

    if oid == SHA256_RSA || oid == SHA256_ECDSA {
        Ok(HashAlgorithm::Sha256)
    } else if oid == SHA384_RSA || oid == SHA384_ECDSA {
        Ok(HashAlgorithm::Sha384)
    } else if oid == SHA512_RSA || oid == SHA512_ECDSA {
        Ok(HashAlgorithm::Sha512)
    } else {
        Err(KeyError::UnsupportedAlgorithm)
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Compute hash with specified algorithm
fn hash_data(alg: HashAlgorithm, data: &[u8]) -> Vec<u8> {
    match alg {
        HashAlgorithm::Sha256 => Sha256::digest(data).to_vec(),
        HashAlgorithm::Sha384 => Sha512::digest_384(data).to_vec(),
        HashAlgorithm::Sha512 => Sha512::digest_512(data).to_vec(),
    }
}

/// Constant-time comparison
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }

    result == 0
}

// =============================================================================
// ERRORS
// =============================================================================

/// Key/certificate error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyError {
    /// Invalid DER encoding
    InvalidDer,
    /// Invalid key format
    InvalidKeyFormat,
    /// Invalid certificate
    InvalidCertificate,
    /// Invalid signature
    InvalidSignature,
    /// Invalid padding
    InvalidPadding,
    /// Unsupported key type
    UnsupportedKeyType,
    /// Unsupported curve
    UnsupportedCurve,
    /// Unsupported algorithm
    UnsupportedAlgorithm,
    /// Not implemented
    NotImplemented,
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_biguint_from_bytes() {
        let bytes = [0x01, 0x02, 0x03, 0x04];
        let n = BigUint::from_be_bytes(&bytes);
        assert_eq!(n.to_be_bytes(), bytes.to_vec());
    }

    #[test]
    fn test_biguint_zero() {
        let n = BigUint::zero();
        assert!(n.is_zero());
    }

    #[test]
    fn test_biguint_mul() {
        let a = BigUint::from_be_bytes(&[0x02]);
        let b = BigUint::from_be_bytes(&[0x03]);
        let c = a.mul(&b);
        assert_eq!(c.to_be_bytes(), vec![0x06]);
    }
}
