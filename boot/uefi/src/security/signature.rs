//! Digital Signature Verification
//!
//! Authenticode and PE/COFF signature verification for UEFI Secure Boot.

extern crate alloc;
use alloc::vec;
use alloc::vec::Vec;

use super::hash::{HashAlgorithm, Sha256, Sha512, SHA256_OUTPUT_SIZE};
use super::keys::{EcdsaSignature, PublicKey, X509Certificate};

// =============================================================================
// SIGNATURE TYPES
// =============================================================================

/// Signature type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureType {
    /// Authenticode PE signature
    Authenticode,
    /// PKCS#7 / CMS signature
    Pkcs7,
    /// Detached signature
    Detached,
}

/// Signature verification result
#[derive(Debug, Clone)]
pub struct SignatureVerificationResult {
    /// Whether signature is valid
    pub valid: bool,
    /// Signer certificate
    pub signer: Option<X509Certificate>,
    /// Certificate chain
    pub chain: Vec<X509Certificate>,
    /// Signing time (if present)
    pub signing_time: Option<u64>,
    /// Countersignature time (if present)
    pub countersign_time: Option<u64>,
}

impl SignatureVerificationResult {
    /// Create successful verification result
    pub fn success(signer: X509Certificate, chain: Vec<X509Certificate>) -> Self {
        Self {
            valid: true,
            signer: Some(signer),
            chain,
            signing_time: None,
            countersign_time: None,
        }
    }

    /// Create failed verification result
    pub fn failure() -> Self {
        Self {
            valid: false,
            signer: None,
            chain: Vec::new(),
            signing_time: None,
            countersign_time: None,
        }
    }
}

// =============================================================================
// AUTHENTICODE
// =============================================================================

/// Authenticode signature verifier
pub struct AuthenticodeVerifier {
    /// Trusted certificates (roots)
    trusted_certs: Vec<X509Certificate>,
    /// Revoked certificate hashes
    revoked_hashes: Vec<[u8; SHA256_OUTPUT_SIZE]>,
    /// Current time for validation
    current_time: Option<u64>,
}

impl AuthenticodeVerifier {
    /// Create new verifier
    pub fn new() -> Self {
        Self {
            trusted_certs: Vec::new(),
            revoked_hashes: Vec::new(),
            current_time: None,
        }
    }

    /// Add trusted root certificate
    pub fn add_trusted_cert(&mut self, cert: X509Certificate) {
        self.trusted_certs.push(cert);
    }

    /// Add revoked certificate hash
    pub fn add_revoked_hash(&mut self, hash: [u8; SHA256_OUTPUT_SIZE]) {
        self.revoked_hashes.push(hash);
    }

    /// Set current time for validation
    pub fn set_time(&mut self, time: u64) {
        self.current_time = Some(time);
    }

    /// Verify PE/COFF file signature
    pub fn verify_pe(&self, pe_data: &[u8]) -> Result<SignatureVerificationResult, SignatureError> {
        // Parse PE header
        let pe = PeFile::parse(pe_data)?;

        // Get security directory
        let security_dir = pe.security_directory()?;

        // Parse WIN_CERTIFICATE
        let win_cert = WinCertificate::parse(security_dir)?;

        // Compute PE hash (excluding signature)
        let pe_hash = self.compute_pe_hash(&pe, pe_data)?;

        // Parse PKCS#7 signature
        let pkcs7 = Pkcs7::parse(&win_cert.certificate)?;

        // Verify signature
        self.verify_pkcs7(&pkcs7, &pe_hash)
    }

    /// Verify PKCS#7 signature
    pub fn verify_pkcs7(
        &self,
        pkcs7: &Pkcs7,
        expected_hash: &[u8],
    ) -> Result<SignatureVerificationResult, SignatureError> {
        // Get signer info
        let signer_info = pkcs7
            .signer_info
            .as_ref()
            .ok_or(SignatureError::NoSignerInfo)?;

        // Find signer certificate
        let signer_cert = self.find_signer_cert(pkcs7, signer_info)?;

        // Check if certificate is revoked
        let cert_hash = Sha256::digest(&signer_cert.tbs_certificate);
        if self.revoked_hashes.iter().any(|h| h == &cert_hash) {
            return Ok(SignatureVerificationResult::failure());
        }

        // Verify content hash matches expected
        if !self.verify_content_hash(signer_info, expected_hash)? {
            return Ok(SignatureVerificationResult::failure());
        }

        // Verify signature
        if !self.verify_signer_signature(signer_info, signer_cert)? {
            return Ok(SignatureVerificationResult::failure());
        }

        // Build and verify certificate chain
        let chain = self.build_chain(signer_cert, &pkcs7.certificates)?;

        if chain.is_empty() {
            return Ok(SignatureVerificationResult::failure());
        }

        // Verify chain
        if !self.verify_chain(&chain)? {
            return Ok(SignatureVerificationResult::failure());
        }

        // Check expiration
        if let Some(time) = self.current_time {
            for cert in &chain {
                if !cert.is_valid_at(time) {
                    return Ok(SignatureVerificationResult::failure());
                }
            }
        }

        Ok(SignatureVerificationResult::success(
            signer_cert.clone(),
            chain,
        ))
    }

    /// Compute PE hash for Authenticode
    #[allow(clippy::unused_self, clippy::unnecessary_wraps)]
    fn compute_pe_hash(&self, pe: &PeFile, data: &[u8]) -> Result<Vec<u8>, SignatureError> {
        // Authenticode hash excludes:
        // 1. Checksum field in optional header
        // 2. Certificate table entry in data directories
        // 3. Certificate table itself

        let mut hasher = Sha256::new();

        let checksum_offset = pe.checksum_offset;
        let cert_table_offset = pe.cert_table_dir_offset;
        let cert_data_offset = pe.cert_data_offset;

        // Hash up to checksum
        hasher.update(&data[..checksum_offset]);

        // Skip checksum (4 bytes)
        hasher.update(&data[checksum_offset + 4..cert_table_offset]);

        // Skip certificate table directory entry (8 bytes)
        hasher.update(&data[cert_table_offset + 8..cert_data_offset]);

        // Don't hash certificate data

        // Add remaining sections (after certificate data)
        if cert_data_offset + pe.cert_data_size < data.len() {
            hasher.update(&data[cert_data_offset + pe.cert_data_size..]);
        }

        Ok(hasher.finalize().to_vec())
    }

    /// Find signer certificate
    #[allow(clippy::unused_self)]
    fn find_signer_cert<'a>(
        &self,
        pkcs7: &'a Pkcs7,
        signer_info: &SignerInfo,
    ) -> Result<&'a X509Certificate, SignatureError> {
        for cert in &pkcs7.certificates {
            if cert.serial_number == signer_info.serial_number && cert.issuer == signer_info.issuer
            {
                return Ok(cert);
            }
        }

        Err(SignatureError::SignerCertNotFound)
    }

    /// Verify content hash in signer info
    #[allow(clippy::unused_self)]
    fn verify_content_hash(
        &self,
        signer_info: &SignerInfo,
        expected_hash: &[u8],
    ) -> Result<bool, SignatureError> {
        // Find message digest attribute
        for attr in &signer_info.signed_attributes {
            if attr.oid == OID_MESSAGE_DIGEST {
                return Ok(constant_time_eq(&attr.value, expected_hash));
            }
        }

        Err(SignatureError::MissingAttribute)
    }

    /// Verify signer signature
    fn verify_signer_signature(
        &self,
        signer_info: &SignerInfo,
        cert: &X509Certificate,
    ) -> Result<bool, SignatureError> {
        // Compute hash of signed attributes
        let attrs_hash = self.hash_signed_attributes(signer_info)?;

        // Verify signature
        match &cert.public_key {
            PublicKey::Rsa(rsa) => rsa
                .verify_pkcs1_v15(
                    signer_info.digest_algorithm,
                    &attrs_hash,
                    &signer_info.signature,
                )
                .map_err(|_| SignatureError::VerificationFailed),
            PublicKey::Ec(ec) => {
                let sig = EcdsaSignature::from_der(&signer_info.signature)
                    .map_err(|_| SignatureError::InvalidSignature)?;
                ec.verify(signer_info.digest_algorithm, &attrs_hash, &sig)
                    .map_err(|_| SignatureError::VerificationFailed)
            },
        }
    }

    /// Hash signed attributes
    #[allow(clippy::unused_self)]
    fn hash_signed_attributes(&self, signer_info: &SignerInfo) -> Result<Vec<u8>, SignatureError> {
        // Re-encode signed attributes as SET
        let attrs_der = encode_signed_attributes(&signer_info.signed_attributes)?;

        Ok(match signer_info.digest_algorithm {
            HashAlgorithm::Sha256 => Sha256::digest(&attrs_der).to_vec(),
            HashAlgorithm::Sha384 => Sha512::digest_384(&attrs_der).to_vec(),
            HashAlgorithm::Sha512 => Sha512::digest_512(&attrs_der).to_vec(),
        })
    }

    /// Build certificate chain
    fn build_chain(
        &self,
        signer: &X509Certificate,
        certs: &[X509Certificate],
    ) -> Result<Vec<X509Certificate>, SignatureError> {
        let mut chain = vec![signer.clone()];
        let mut current = signer;

        // Follow issuer chain
        for _ in 0..10 {
            // Check if current is a root
            if current.is_self_signed() {
                // Check if it's trusted
                if self.is_trusted(current) {
                    return Ok(chain);
                }
                break;
            }

            // Find issuer
            let mut found = false;
            for cert in certs {
                if cert.subject == current.issuer {
                    chain.push(cert.clone());
                    current = cert;
                    found = true;
                    break;
                }
            }

            // Check trusted roots
            for cert in &self.trusted_certs {
                if cert.subject == current.issuer {
                    chain.push(cert.clone());
                    return Ok(chain);
                }
            }

            if !found {
                break;
            }
        }

        // Check if chain ends at trusted root
        if let Some(last) = chain.last() {
            if self.is_trusted(last) {
                return Ok(chain);
            }
        }

        Err(SignatureError::UntrustedRoot)
    }

    /// Check if certificate is trusted
    fn is_trusted(&self, cert: &X509Certificate) -> bool {
        for trusted in &self.trusted_certs {
            if cert.subject == trusted.subject && cert.serial_number == trusted.serial_number {
                return true;
            }
        }
        false
    }

    /// Verify certificate chain
    #[allow(clippy::unused_self, clippy::unnecessary_wraps)]
    fn verify_chain(&self, chain: &[X509Certificate]) -> Result<bool, SignatureError> {
        for i in 0..chain.len().saturating_sub(1) {
            let cert = &chain[i];
            let issuer = &chain[i + 1];

            // Verify signature
            match cert.verify_signature(&issuer.public_key) {
                Ok(true) => {},
                _ => return Ok(false),
            }

            // Check CA constraint
            if i + 1 < chain.len() - 1 && !issuer.is_ca() {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

impl Default for AuthenticodeVerifier {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// PE FILE PARSING
// =============================================================================

/// Minimal PE file parser for Authenticode
struct PeFile {
    /// Offset of checksum field
    checksum_offset: usize,
    /// Offset of certificate table directory entry
    cert_table_dir_offset: usize,
    /// Offset of certificate data
    cert_data_offset: usize,
    /// Size of certificate data
    cert_data_size: usize,
}

impl PeFile {
    /// Parse PE file
    fn parse(data: &[u8]) -> Result<Self, SignatureError> {
        // Check DOS header
        if data.len() < 64 || &data[0..2] != b"MZ" {
            return Err(SignatureError::InvalidPe);
        }

        // Get PE header offset
        let pe_offset = u32::from_le_bytes([data[60], data[61], data[62], data[63]]) as usize;

        if data.len() < pe_offset + 4 || &data[pe_offset..pe_offset + 4] != b"PE\0\0" {
            return Err(SignatureError::InvalidPe);
        }

        // Parse COFF header
        let coff_offset = pe_offset + 4;
        if data.len() < coff_offset + 20 {
            return Err(SignatureError::InvalidPe);
        }

        let size_of_optional =
            u16::from_le_bytes([data[coff_offset + 16], data[coff_offset + 17]]) as usize;

        // Parse optional header
        let opt_offset = coff_offset + 20;
        if data.len() < opt_offset + size_of_optional || size_of_optional < 2 {
            return Err(SignatureError::InvalidPe);
        }

        let magic = u16::from_le_bytes([data[opt_offset], data[opt_offset + 1]]);
        let is_pe32_plus = magic == 0x20b;

        // Calculate offsets
        let checksum_offset = opt_offset + 64;

        let data_dir_offset = if is_pe32_plus {
            opt_offset + 112
        } else {
            opt_offset + 96
        };

        // Certificate table is data directory entry 4
        let cert_table_dir_offset = data_dir_offset + 4 * 8;

        if data.len() < cert_table_dir_offset + 8 {
            return Err(SignatureError::InvalidPe);
        }

        let cert_data_offset = u32::from_le_bytes([
            data[cert_table_dir_offset],
            data[cert_table_dir_offset + 1],
            data[cert_table_dir_offset + 2],
            data[cert_table_dir_offset + 3],
        ]) as usize;

        let cert_data_size = u32::from_le_bytes([
            data[cert_table_dir_offset + 4],
            data[cert_table_dir_offset + 5],
            data[cert_table_dir_offset + 6],
            data[cert_table_dir_offset + 7],
        ]) as usize;

        Ok(Self {
            checksum_offset,
            cert_table_dir_offset,
            cert_data_offset,
            cert_data_size,
        })
    }

    /// Get security directory data
    fn security_directory<'a>(&self) -> Result<&'a [u8], SignatureError> {
        if self.cert_data_size == 0 {
            return Err(SignatureError::NoSignature);
        }

        // This would return the actual data slice
        // In practice this needs the original data reference
        Err(SignatureError::NoSignature)
    }
}

// =============================================================================
// WIN_CERTIFICATE
// =============================================================================

/// WIN_CERTIFICATE revision
#[allow(dead_code)]
mod win_cert_revision {
    pub const WIN_CERT_REVISION_1_0: u16 = 0x0100;
    pub const WIN_CERT_REVISION_2_0: u16 = 0x0200;
}

/// WIN_CERTIFICATE type
#[allow(dead_code)]
mod win_cert_type {
    pub const WIN_CERT_TYPE_PKCS_SIGNED_DATA: u16 = 0x0002;
}

/// WIN_CERTIFICATE structure
struct WinCertificate {
    #[allow(dead_code)]
    length: u32,
    #[allow(dead_code)]
    revision: u16,
    #[allow(dead_code)]
    certificate_type: u16,
    certificate: Vec<u8>,
}

impl WinCertificate {
    /// Parse WIN_CERTIFICATE
    fn parse(data: &[u8]) -> Result<Self, SignatureError> {
        if data.len() < 8 {
            return Err(SignatureError::InvalidSignature);
        }

        let length = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let revision = u16::from_le_bytes([data[4], data[5]]);
        let certificate_type = u16::from_le_bytes([data[6], data[7]]);

        if certificate_type != win_cert_type::WIN_CERT_TYPE_PKCS_SIGNED_DATA {
            return Err(SignatureError::UnsupportedSignatureType);
        }

        let cert_len = length as usize - 8;
        if data.len() < 8 + cert_len {
            return Err(SignatureError::InvalidSignature);
        }

        Ok(Self {
            length,
            revision,
            certificate_type,
            certificate: data[8..8 + cert_len].to_vec(),
        })
    }
}

// =============================================================================
// PKCS#7 / CMS
// =============================================================================

/// OID for message digest attribute
const OID_MESSAGE_DIGEST: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x09, 0x04];

/// OID for signing time attribute
const _OID_SIGNING_TIME: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x09, 0x05];

/// OID for content type attribute
const _OID_CONTENT_TYPE: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x09, 0x03];

/// PKCS#7 SignedData
pub struct Pkcs7 {
    /// Version
    pub version: u32,
    /// Digest algorithms
    pub digest_algorithms: Vec<Vec<u8>>,
    /// Encapsulated content
    pub content: Option<Vec<u8>>,
    /// Certificates
    pub certificates: Vec<X509Certificate>,
    /// Signer info
    pub signer_info: Option<SignerInfo>,
}

impl Pkcs7 {
    /// Parse PKCS#7 SignedData
    pub fn parse(data: &[u8]) -> Result<Self, SignatureError> {
        // ContentInfo ::= SEQUENCE {
        //   contentType ContentType,
        //   content [0] EXPLICIT ANY OPTIONAL
        // }

        let (tag, content, _) =
            parse_der_element(data).map_err(|_| SignatureError::InvalidSignature)?;

        if tag != 0x30 {
            return Err(SignatureError::InvalidSignature);
        }

        let mut offset = 0;

        // Content type OID
        let (tag, _, consumed) =
            parse_der_element(&content[offset..]).map_err(|_| SignatureError::InvalidSignature)?;
        if tag != 0x06 {
            return Err(SignatureError::InvalidSignature);
        }
        offset += consumed;

        // Content [0]
        let (tag, signed_data, _) =
            parse_der_element(&content[offset..]).map_err(|_| SignatureError::InvalidSignature)?;
        if tag != 0xa0 {
            return Err(SignatureError::InvalidSignature);
        }

        // Parse SignedData
        Self::parse_signed_data(signed_data)
    }

    fn parse_signed_data(data: &[u8]) -> Result<Self, SignatureError> {
        // SignedData ::= SEQUENCE {
        //   version CMSVersion,
        //   digestAlgorithms DigestAlgorithmIdentifiers,
        //   encapContentInfo EncapsulatedContentInfo,
        //   certificates [0] IMPLICIT CertificateSet OPTIONAL,
        //   crls [1] IMPLICIT CertificateRevocationLists OPTIONAL,
        //   signerInfos SignerInfos
        // }

        let (tag, content, _) =
            parse_der_element(data).map_err(|_| SignatureError::InvalidSignature)?;

        if tag != 0x30 {
            return Err(SignatureError::InvalidSignature);
        }

        let mut offset = 0;

        // Version
        let (tag, ver_data, consumed) =
            parse_der_element(&content[offset..]).map_err(|_| SignatureError::InvalidSignature)?;
        if tag != 0x02 {
            return Err(SignatureError::InvalidSignature);
        }
        let version = if ver_data.is_empty() {
            0
        } else {
            ver_data[0] as u32
        };
        offset += consumed;

        // Digest algorithms
        let (tag, alg_set, consumed) =
            parse_der_element(&content[offset..]).map_err(|_| SignatureError::InvalidSignature)?;
        if tag != 0x31 {
            return Err(SignatureError::InvalidSignature);
        }
        let digest_algorithms = parse_algorithm_set(alg_set)?;
        offset += consumed;

        // Encapsulated content info
        let (tag, _, consumed) =
            parse_der_element(&content[offset..]).map_err(|_| SignatureError::InvalidSignature)?;
        if tag != 0x30 {
            return Err(SignatureError::InvalidSignature);
        }
        offset += consumed;

        // Certificates (optional, [0])
        let mut certificates = Vec::new();
        if offset < content.len() && content[offset] == 0xa0 {
            let (_, cert_set, consumed) = parse_der_element(&content[offset..])
                .map_err(|_| SignatureError::InvalidSignature)?;
            certificates = parse_certificate_set(cert_set)?;
            offset += consumed;
        }

        // CRLs (optional, [1]) - skip
        if offset < content.len() && content[offset] == 0xa1 {
            let (_, _, consumed) = parse_der_element(&content[offset..])
                .map_err(|_| SignatureError::InvalidSignature)?;
            offset += consumed;
        }

        // Signer infos
        let (tag, signer_set, _) =
            parse_der_element(&content[offset..]).map_err(|_| SignatureError::InvalidSignature)?;
        if tag != 0x31 {
            return Err(SignatureError::InvalidSignature);
        }

        let signer_info = parse_signer_info_set(signer_set)?;

        Ok(Self {
            version,
            digest_algorithms,
            content: None,
            certificates,
            signer_info,
        })
    }
}

/// Signer info
pub struct SignerInfo {
    /// Version
    pub version: u32,
    /// Issuer
    pub issuer: Vec<u8>,
    /// Serial number
    pub serial_number: Vec<u8>,
    /// Digest algorithm
    pub digest_algorithm: HashAlgorithm,
    /// Signed attributes
    pub signed_attributes: Vec<Attribute>,
    /// Signature algorithm
    pub signature_algorithm: Vec<u8>,
    /// Signature
    pub signature: Vec<u8>,
}

/// Attribute (OID + value)
pub struct Attribute {
    /// OID
    pub oid: Vec<u8>,
    /// Value
    pub value: Vec<u8>,
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Parse DER element
fn parse_der_element(data: &[u8]) -> Result<(u8, &[u8], usize), SignatureError> {
    if data.is_empty() {
        return Err(SignatureError::InvalidSignature);
    }

    let tag = data[0];

    if data.len() < 2 {
        return Err(SignatureError::InvalidSignature);
    }

    let (length, len_size) = match data[1].cmp(&0x80) {
        core::cmp::Ordering::Less => (data[1] as usize, 1),
        core::cmp::Ordering::Equal => return Err(SignatureError::InvalidSignature),
        core::cmp::Ordering::Greater => {
            let num_octets = (data[1] & 0x7f) as usize;
            if data.len() < 2 + num_octets {
                return Err(SignatureError::InvalidSignature);
            }

            let mut length = 0usize;
            for i in 0..num_octets {
                length = (length << 8) | (data[2 + i] as usize);
            }

            (length, 1 + num_octets)
        },
    };

    let header_size = 1 + len_size;
    if data.len() < header_size + length {
        return Err(SignatureError::InvalidSignature);
    }

    Ok((
        tag,
        &data[header_size..header_size + length],
        header_size + length,
    ))
}

/// Parse algorithm set
fn parse_algorithm_set(data: &[u8]) -> Result<Vec<Vec<u8>>, SignatureError> {
    let mut algorithms = Vec::new();
    let mut offset = 0;

    while offset < data.len() {
        let (tag, alg, consumed) =
            parse_der_element(&data[offset..]).map_err(|_| SignatureError::InvalidSignature)?;

        if tag == 0x30 {
            algorithms.push(alg.to_vec());
        }

        offset += consumed;
    }

    Ok(algorithms)
}

/// Parse certificate set
fn parse_certificate_set(data: &[u8]) -> Result<Vec<X509Certificate>, SignatureError> {
    let mut certs = Vec::new();
    let mut offset = 0;

    while offset < data.len() {
        // Get full certificate including header
        let start = offset;
        let (tag, _, consumed) =
            parse_der_element(&data[offset..]).map_err(|_| SignatureError::InvalidSignature)?;

        if tag == 0x30 {
            match X509Certificate::from_der(&data[start..start + consumed]) {
                Ok(cert) => certs.push(cert),
                Err(_) => {}, // Skip invalid certificates
            }
        }

        offset += consumed;
    }

    Ok(certs)
}

/// Parse signer info set
fn parse_signer_info_set(data: &[u8]) -> Result<Option<SignerInfo>, SignatureError> {
    if data.is_empty() {
        return Ok(None);
    }

    let (tag, signer_data, _) = parse_der_element(data)?;
    if tag != 0x30 {
        return Ok(None);
    }

    parse_signer_info(signer_data).map(Some)
}

/// Parse single signer info
fn parse_signer_info(data: &[u8]) -> Result<SignerInfo, SignatureError> {
    let mut offset = 0;

    // Version
    let (tag, ver_data, consumed) = parse_der_element(&data[offset..])?;
    if tag != 0x02 {
        return Err(SignatureError::InvalidSignature);
    }
    let version = if ver_data.is_empty() {
        0
    } else {
        ver_data[0] as u32
    };
    offset += consumed;

    // IssuerAndSerialNumber or SubjectKeyIdentifier
    let (tag, sid_data, consumed) = parse_der_element(&data[offset..])?;
    offset += consumed;

    let (issuer, serial_number) = if tag == 0x30 {
        // IssuerAndSerialNumber
        let mut sid_offset = 0;

        // Issuer
        let issuer_start = sid_offset;
        let (_, _, consumed) = parse_der_element(&sid_data[sid_offset..])?;
        let issuer = sid_data[issuer_start..sid_offset + consumed].to_vec();
        sid_offset += consumed;

        // Serial number
        let (tag, serial, _) = parse_der_element(&sid_data[sid_offset..])?;
        if tag != 0x02 {
            return Err(SignatureError::InvalidSignature);
        }

        (issuer, serial.to_vec())
    } else {
        (Vec::new(), Vec::new())
    };

    // Digest algorithm
    let (tag, alg_data, consumed) = parse_der_element(&data[offset..])?;
    if tag != 0x30 {
        return Err(SignatureError::InvalidSignature);
    }
    let digest_algorithm = parse_hash_algorithm(alg_data)?;
    offset += consumed;

    // Signed attributes (optional, [0])
    let mut signed_attributes = Vec::new();
    if offset < data.len() && data[offset] == 0xa0 {
        let (_, attrs, consumed) = parse_der_element(&data[offset..])?;
        signed_attributes = parse_attributes(attrs)?;
        offset += consumed;
    }

    // Signature algorithm
    let (tag, sig_alg, consumed) = parse_der_element(&data[offset..])?;
    if tag != 0x30 {
        return Err(SignatureError::InvalidSignature);
    }
    let signature_algorithm = sig_alg.to_vec();
    offset += consumed;

    // Signature
    let (tag, signature_data, _) = parse_der_element(&data[offset..])?;
    if tag != 0x04 {
        return Err(SignatureError::InvalidSignature);
    }
    let signature = signature_data.to_vec();

    Ok(SignerInfo {
        version,
        issuer,
        serial_number,
        digest_algorithm,
        signed_attributes,
        signature_algorithm,
        signature,
    })
}

/// Parse hash algorithm from AlgorithmIdentifier
fn parse_hash_algorithm(data: &[u8]) -> Result<HashAlgorithm, SignatureError> {
    // SHA-256: 2.16.840.1.101.3.4.2.1
    const SHA256_OID: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01];
    // SHA-384: 2.16.840.1.101.3.4.2.2
    const SHA384_OID: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02];
    // SHA-512: 2.16.840.1.101.3.4.2.3
    const SHA512_OID: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03];

    let (tag, oid, _) = parse_der_element(data)?;
    if tag != 0x06 {
        return Err(SignatureError::InvalidSignature);
    }

    if oid == SHA256_OID {
        Ok(HashAlgorithm::Sha256)
    } else if oid == SHA384_OID {
        Ok(HashAlgorithm::Sha384)
    } else if oid == SHA512_OID {
        Ok(HashAlgorithm::Sha512)
    } else {
        Err(SignatureError::UnsupportedAlgorithm)
    }
}

/// Parse attributes
fn parse_attributes(data: &[u8]) -> Result<Vec<Attribute>, SignatureError> {
    let mut attrs = Vec::new();
    let mut offset = 0;

    while offset < data.len() {
        let (tag, attr_data, consumed) = parse_der_element(&data[offset..])?;

        if tag == 0x30 {
            let mut attr_offset = 0;

            // OID
            let (tag, oid, consumed) = parse_der_element(&attr_data[attr_offset..])?;
            if tag != 0x06 {
                offset += consumed;
                continue;
            }
            attr_offset += consumed;

            // Values (SET)
            let (tag, values, _) = parse_der_element(&attr_data[attr_offset..])?;
            if tag != 0x31 {
                offset += consumed;
                continue;
            }

            // Get first value
            let value = if values.is_empty() {
                Vec::new()
            } else {
                let (_, val, _) = parse_der_element(values)?;
                val.to_vec()
            };

            attrs.push(Attribute {
                oid: oid.to_vec(),
                value,
            });
        }

        offset += consumed;
    }

    Ok(attrs)
}

/// Encode signed attributes as SET
#[allow(clippy::unnecessary_wraps)]
fn encode_signed_attributes(attrs: &[Attribute]) -> Result<Vec<u8>, SignatureError> {
    // This would re-encode attributes with SET tag instead of CONTEXT [0]
    // Simplified: just return empty for now
    let _ = attrs;
    Ok(Vec::new())
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

/// Signature error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureError {
    /// Invalid PE file
    InvalidPe,
    /// No signature present
    NoSignature,
    /// Invalid signature format
    InvalidSignature,
    /// Unsupported signature type
    UnsupportedSignatureType,
    /// Unsupported algorithm
    UnsupportedAlgorithm,
    /// No signer info
    NoSignerInfo,
    /// Signer certificate not found
    SignerCertNotFound,
    /// Missing required attribute
    MissingAttribute,
    /// Verification failed
    VerificationFailed,
    /// Untrusted root
    UntrustedRoot,
    /// Certificate revoked
    CertificateRevoked,
    /// Certificate expired
    CertificateExpired,
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_authenticode_verifier_new() {
        let verifier = AuthenticodeVerifier::new();
        assert!(verifier.trusted_certs.is_empty());
    }

    #[test]
    fn test_signature_verification_result() {
        let result = SignatureVerificationResult::failure();
        assert!(!result.valid);
        assert!(result.signer.is_none());
    }
}
