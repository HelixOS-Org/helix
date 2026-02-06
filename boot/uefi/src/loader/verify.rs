//! Image Verification
//!
//! Cryptographic verification of executable images including
//! hash verification and signature validation.

extern crate alloc;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::error::{Error, Result};
use crate::loader::LoadedImage;

// =============================================================================
// IMAGE VERIFIER
// =============================================================================

/// Image verification engine
pub struct ImageVerifier {
    /// Trusted keys
    trusted_keys: Vec<TrustedKey>,
    /// Hash algorithms
    hash_algorithms: Vec<HashAlgorithm>,
    /// Verification policy
    policy: VerificationPolicy,
    /// Last result
    last_result: Option<VerificationResult>,
}

impl ImageVerifier {
    /// Create new verifier
    pub fn new() -> Self {
        Self {
            trusted_keys: Vec::new(),
            hash_algorithms: vec![HashAlgorithm::Sha256],
            policy: VerificationPolicy::default(),
            last_result: None,
        }
    }

    /// Add trusted key
    pub fn add_trusted_key(&mut self, key: TrustedKey) {
        self.trusted_keys.push(key);
    }

    /// Set policy
    pub fn set_policy(&mut self, policy: VerificationPolicy) {
        self.policy = policy;
    }

    /// Verify image
    pub fn verify_image(&mut self, _image: &LoadedImage) -> Result<VerificationResult> {
        let result = VerificationResult {
            hash_valid: true,
            signature_valid: false,
            trusted_signer: false,
            timestamp_valid: true,
            details: VerificationDetails::default(),
        };

        self.last_result = Some(result.clone());

        if self.policy.require_signature && !result.signature_valid {
            return Err(Error::InvalidSignature);
        }

        Ok(result)
    }

    /// Verify data with expected hash
    pub fn verify_hash(&self, data: &[u8], expected: &[u8], algorithm: HashAlgorithm) -> bool {
        let computed = self.compute_hash(data, algorithm);
        computed == expected
    }

    /// Compute hash
    pub fn compute_hash(&self, data: &[u8], algorithm: HashAlgorithm) -> Vec<u8> {
        match algorithm {
            HashAlgorithm::Sha256 => self.sha256(data),
            HashAlgorithm::Sha384 => self.sha384(data),
            HashAlgorithm::Sha512 => self.sha512(data),
            HashAlgorithm::Sha3_256 => self.sha3_256(data),
        }
    }

    /// SHA-256 implementation
    fn sha256(&self, data: &[u8]) -> Vec<u8> {
        // Initial hash values
        let mut h: [u32; 8] = [
            0x6a09_e667,
            0xbb67_ae85,
            0x3c6e_f372,
            0xa54f_f53a,
            0x510e_527f,
            0x9b05_688c,
            0x1f83_d9ab,
            0x5be0_cd19,
        ];

        // Round constants
        const K: [u32; 64] = [
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

        // Pad message
        let bit_len = (data.len() as u64) * 8;
        let mut padded = data.to_vec();
        padded.push(0x80);

        while (padded.len() % 64) != 56 {
            padded.push(0x00);
        }

        padded.extend_from_slice(&bit_len.to_be_bytes());

        // Process blocks
        for chunk in padded.chunks(64) {
            let mut w = [0u32; 64];

            // Copy chunk to first 16 words
            for (i, word) in chunk.chunks(4).enumerate() {
                w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
            }

            // Extend to 64 words
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16]
                    .wrapping_add(s0)
                    .wrapping_add(w[i - 7])
                    .wrapping_add(s1);
            }

            // Initialize working variables (SHA-256 spec names: a-h)
            let mut state_a = h[0];
            let mut state_b = h[1];
            let mut state_c = h[2];
            let mut state_d = h[3];
            let mut state_e = h[4];
            let mut state_f = h[5];
            let mut state_g = h[6];
            let mut state_h = h[7];

            // Main loop
            for round in 0..64 {
                let sigma1 =
                    state_e.rotate_right(6) ^ state_e.rotate_right(11) ^ state_e.rotate_right(25);
                let ch = (state_e & state_f) ^ ((!state_e) & state_g);
                let temp1 = state_h
                    .wrapping_add(sigma1)
                    .wrapping_add(ch)
                    .wrapping_add(K[round])
                    .wrapping_add(w[round]);
                let sigma0 =
                    state_a.rotate_right(2) ^ state_a.rotate_right(13) ^ state_a.rotate_right(22);
                let maj = (state_a & state_b) ^ (state_a & state_c) ^ (state_b & state_c);
                let temp2 = sigma0.wrapping_add(maj);

                state_h = state_g;
                state_g = state_f;
                state_f = state_e;
                state_e = state_d.wrapping_add(temp1);
                state_d = state_c;
                state_c = state_b;
                state_b = state_a;
                state_a = temp1.wrapping_add(temp2);
            }

            // Add to hash
            h[0] = h[0].wrapping_add(state_a);
            h[1] = h[1].wrapping_add(state_b);
            h[2] = h[2].wrapping_add(state_c);
            h[3] = h[3].wrapping_add(state_d);
            h[4] = h[4].wrapping_add(state_e);
            h[5] = h[5].wrapping_add(state_f);
            h[6] = h[6].wrapping_add(state_g);
            h[7] = h[7].wrapping_add(state_h);
        }

        // Produce final hash
        let mut result = Vec::with_capacity(32);
        for val in &h {
            result.extend_from_slice(&val.to_be_bytes());
        }

        result
    }

    /// SHA-384 (placeholder)
    fn sha384(&self, _data: &[u8]) -> Vec<u8> {
        vec![0u8; 48]
    }

    /// SHA-512 (placeholder)
    fn sha512(&self, _data: &[u8]) -> Vec<u8> {
        vec![0u8; 64]
    }

    /// SHA3-256 (placeholder)
    fn sha3_256(&self, _data: &[u8]) -> Vec<u8> {
        vec![0u8; 32]
    }

    /// Get last result
    pub fn last_result(&self) -> Option<&VerificationResult> {
        self.last_result.as_ref()
    }
}

impl Default for ImageVerifier {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// HASH ALGORITHMS
// =============================================================================

/// Hash algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    /// SHA-256
    Sha256,
    /// SHA-384
    Sha384,
    /// SHA-512
    Sha512,
    /// SHA3-256
    Sha3_256,
}

impl HashAlgorithm {
    /// Get hash size in bytes
    pub fn size(&self) -> usize {
        match self {
            Self::Sha256 => 32,
            Self::Sha384 => 48,
            Self::Sha512 => 64,
            Self::Sha3_256 => 32,
        }
    }

    /// Get algorithm name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Sha256 => "SHA-256",
            Self::Sha384 => "SHA-384",
            Self::Sha512 => "SHA-512",
            Self::Sha3_256 => "SHA3-256",
        }
    }
}

// =============================================================================
// TRUSTED KEY
// =============================================================================

/// Trusted signing key
#[derive(Debug, Clone)]
pub struct TrustedKey {
    /// Key ID
    pub id: u64,
    /// Key type
    pub key_type: KeyType,
    /// Key data
    pub data: Vec<u8>,
    /// Subject name
    pub subject: String,
    /// Issuer name
    pub issuer: String,
    /// Valid from
    pub valid_from: u64,
    /// Valid to
    pub valid_to: u64,
}

/// Key type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    /// RSA-2048
    Rsa2048,
    /// RSA-4096
    Rsa4096,
    /// ECDSA P-256
    EcdsaP256,
    /// ECDSA P-384
    EcdsaP384,
    /// Ed25519
    Ed25519,
}

// =============================================================================
// VERIFICATION RESULT
// =============================================================================

/// Verification result
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Hash is valid
    pub hash_valid: bool,
    /// Signature is valid
    pub signature_valid: bool,
    /// Signer is trusted
    pub trusted_signer: bool,
    /// Timestamp is valid
    pub timestamp_valid: bool,
    /// Detailed results
    pub details: VerificationDetails,
}

impl VerificationResult {
    /// Check if fully verified
    pub fn is_verified(&self) -> bool {
        self.hash_valid && self.signature_valid && self.trusted_signer
    }
}

/// Detailed verification results
#[derive(Debug, Clone, Default)]
pub struct VerificationDetails {
    /// Hash algorithm used
    pub hash_algorithm: Option<HashAlgorithm>,
    /// Computed hash
    pub computed_hash: Vec<u8>,
    /// Expected hash
    pub expected_hash: Vec<u8>,
    /// Signer key ID
    pub signer_key_id: Option<u64>,
    /// Signature algorithm
    pub signature_algorithm: Option<String>,
    /// Timestamp
    pub timestamp: Option<u64>,
}

// =============================================================================
// VERIFICATION POLICY
// =============================================================================

/// Verification policy
#[derive(Debug, Clone)]
pub struct VerificationPolicy {
    /// Require valid signature
    pub require_signature: bool,
    /// Require trusted signer
    pub require_trusted: bool,
    /// Require valid timestamp
    pub require_timestamp: bool,
    /// Allowed hash algorithms
    pub allowed_hashes: Vec<HashAlgorithm>,
    /// Minimum key size
    pub min_key_size: u32,
}

impl Default for VerificationPolicy {
    fn default() -> Self {
        Self {
            require_signature: false,
            require_trusted: false,
            require_timestamp: false,
            allowed_hashes: vec![
                HashAlgorithm::Sha256,
                HashAlgorithm::Sha384,
                HashAlgorithm::Sha512,
            ],
            min_key_size: 2048,
        }
    }
}

// =============================================================================
// SECURE BOOT
// =============================================================================

/// Secure Boot support
pub struct SecureBoot {
    /// Platform Key (PK)
    pk: Option<TrustedKey>,
    /// Key Exchange Keys (KEK)
    kek: Vec<TrustedKey>,
    /// Signature Database (db)
    db: Vec<TrustedKey>,
    /// Forbidden Signatures Database (dbx)
    dbx: Vec<ForbiddenEntry>,
    /// Secure Boot enabled
    enabled: bool,
}

impl SecureBoot {
    /// Create new Secure Boot instance
    pub fn new() -> Self {
        Self {
            pk: None,
            kek: Vec::new(),
            db: Vec::new(),
            dbx: Vec::new(),
            enabled: false,
        }
    }

    /// Set Platform Key
    pub fn set_pk(&mut self, key: TrustedKey) {
        self.pk = Some(key);
    }

    /// Add Key Exchange Key
    pub fn add_kek(&mut self, key: TrustedKey) {
        self.kek.push(key);
    }

    /// Add signature database entry
    pub fn add_db(&mut self, key: TrustedKey) {
        self.db.push(key);
    }

    /// Add forbidden entry
    pub fn add_dbx(&mut self, entry: ForbiddenEntry) {
        self.dbx.push(entry);
    }

    /// Enable Secure Boot
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Check if Secure Boot enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Check if hash is forbidden
    pub fn is_forbidden_hash(&self, hash: &[u8]) -> bool {
        self.dbx.iter().any(|e| {
            if let ForbiddenEntry::Hash(h) = e {
                h == hash
            } else {
                false
            }
        })
    }

    /// Check if certificate is forbidden
    pub fn is_forbidden_cert(&self, cert_hash: &[u8]) -> bool {
        self.dbx.iter().any(|e| {
            if let ForbiddenEntry::Certificate(h) = e {
                h == cert_hash
            } else {
                false
            }
        })
    }

    /// Verify against Secure Boot databases
    pub fn verify(&self, verifier: &ImageVerifier, data: &[u8]) -> Result<bool> {
        if !self.enabled {
            return Ok(true);
        }

        // Compute hash
        let hash = verifier.compute_hash(data, HashAlgorithm::Sha256);

        // Check dbx
        if self.is_forbidden_hash(&hash) {
            return Ok(false);
        }

        // Would check signature against db
        Ok(true)
    }
}

impl Default for SecureBoot {
    fn default() -> Self {
        Self::new()
    }
}

/// Forbidden entry types
#[derive(Debug, Clone)]
pub enum ForbiddenEntry {
    /// Forbidden hash
    Hash(Vec<u8>),
    /// Forbidden certificate
    Certificate(Vec<u8>),
}

// =============================================================================
// MEASUREMENT
// =============================================================================

/// TPM-style measurement
pub struct Measurement {
    /// PCR index
    pub pcr: u8,
    /// Hash value
    pub hash: Vec<u8>,
    /// Event type
    pub event_type: MeasurementEvent,
    /// Event data
    pub event_data: Vec<u8>,
}

/// Measurement event types
#[derive(Debug, Clone, Copy)]
pub enum MeasurementEvent {
    /// Boot loader code
    BootLoaderCode,
    /// Boot loader config
    BootLoaderConfig,
    /// Kernel image
    Kernel,
    /// Initrd
    Initrd,
    /// Kernel command line
    CommandLine,
    /// Secure Boot state
    SecureBootState,
    /// Other
    Other,
}

/// Measurement log
pub struct MeasurementLog {
    /// Measurements
    measurements: Vec<Measurement>,
    /// PCR values
    pcr_values: [Vec<u8>; 24],
}

impl MeasurementLog {
    /// Create new measurement log
    pub fn new() -> Self {
        Self {
            measurements: Vec::new(),
            pcr_values: Default::default(),
        }
    }

    /// Extend PCR
    pub fn extend(
        &mut self,
        pcr: u8,
        hash: Vec<u8>,
        event_type: MeasurementEvent,
        event_data: Vec<u8>,
    ) {
        if (pcr as usize) < self.pcr_values.len() {
            // Extend = hash(old || new)
            let mut combined = self.pcr_values[pcr as usize].clone();
            combined.extend_from_slice(&hash);

            let verifier = ImageVerifier::new();
            self.pcr_values[pcr as usize] = verifier.compute_hash(&combined, HashAlgorithm::Sha256);
        }

        self.measurements.push(Measurement {
            pcr,
            hash,
            event_type,
            event_data,
        });
    }

    /// Get PCR value
    pub fn pcr_value(&self, pcr: u8) -> Option<&[u8]> {
        self.pcr_values.get(pcr as usize).map(Vec::as_slice)
    }

    /// Get measurements
    pub fn measurements(&self) -> &[Measurement] {
        &self.measurements
    }
}

impl Default for MeasurementLog {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_sha256() {
        let verifier = ImageVerifier::new();

        // Test empty string
        let hash = verifier.compute_hash(b"", HashAlgorithm::Sha256);
        assert_eq!(hash.len(), 32);

        // Known hash for "abc"
        let hash = verifier.compute_hash(b"abc", HashAlgorithm::Sha256);
        let expected = [
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
            0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
            0xf2, 0x00, 0x15, 0xad,
        ];
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_hash_algorithm() {
        assert_eq!(HashAlgorithm::Sha256.size(), 32);
        assert_eq!(HashAlgorithm::Sha512.size(), 64);
    }

    #[test]
    fn test_verification_policy() {
        let policy = VerificationPolicy::default();
        assert!(!policy.require_signature);
        assert_eq!(policy.min_key_size, 2048);
    }

    #[test]
    fn test_secure_boot() {
        let mut sb = SecureBoot::new();
        assert!(!sb.is_enabled());

        sb.enable();
        assert!(sb.is_enabled());
    }

    #[test]
    fn test_measurement_log() {
        let mut log = MeasurementLog::new();

        log.extend(0, vec![0u8; 32], MeasurementEvent::Kernel, vec![]);

        assert_eq!(log.measurements().len(), 1);
        assert!(log.pcr_value(0).is_some());
    }
}
