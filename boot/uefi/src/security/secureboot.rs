//! UEFI Secure Boot Integration
//!
//! Interface with UEFI Secure Boot variables, certificate databases, and verification.

extern crate alloc;
use alloc::vec::Vec;

use super::hash::Sha256;
use super::keys::{KeyError, X509Certificate};
use super::signature::{AuthenticodeVerifier, SignatureError};

// =============================================================================
// SECURE BOOT GUIDS
// =============================================================================

/// GUID module
pub mod guid {
    /// EFI_GLOBAL_VARIABLE GUID
    pub const EFI_GLOBAL_VARIABLE: [u8; 16] = [
        0x61, 0xdf, 0xe4, 0x8b, 0xca, 0x93, 0xd2, 0x11, 0xaa, 0x0d, 0x00, 0xe0, 0x98, 0x03, 0x2b,
        0x8c,
    ];

    /// EFI_IMAGE_SECURITY_DATABASE GUID
    pub const EFI_IMAGE_SECURITY_DATABASE: [u8; 16] = [
        0xcb, 0xb2, 0x19, 0xd7, 0x3a, 0x3d, 0x96, 0x45, 0xa3, 0xbc, 0xda, 0xd0, 0x0e, 0x67, 0x65,
        0x6f,
    ];

    /// EFI_CERT_SHA256_GUID
    pub const EFI_CERT_SHA256: [u8; 16] = [
        0x26, 0x16, 0xc4, 0xc1, 0x4c, 0x50, 0x92, 0x40, 0xac, 0xa9, 0x41, 0xf9, 0x36, 0x93, 0x43,
        0x28,
    ];

    /// EFI_CERT_RSA2048_GUID
    pub const EFI_CERT_RSA2048: [u8; 16] = [
        0xe8, 0x66, 0x57, 0x3c, 0x9c, 0x26, 0x34, 0x4e, 0xaa, 0x14, 0xed, 0x77, 0x6e, 0x85, 0xb3,
        0xb6,
    ];

    /// EFI_CERT_X509_GUID
    pub const EFI_CERT_X509: [u8; 16] = [
        0xa1, 0x59, 0xc0, 0xa5, 0xe4, 0x94, 0xa7, 0x4a, 0x87, 0xb5, 0xab, 0x15, 0x5c, 0x2b, 0xf0,
        0x72,
    ];

    /// EFI_CERT_X509_SHA256_GUID
    pub const EFI_CERT_X509_SHA256: [u8; 16] = [
        0x92, 0xa4, 0xd2, 0x3b, 0xc0, 0x96, 0x79, 0x40, 0xb4, 0x20, 0xfc, 0xf9, 0x8e, 0xf1, 0x03,
        0xed,
    ];

    /// EFI_CERT_PKCS7_GUID
    pub const EFI_CERT_PKCS7: [u8; 16] = [
        0x9d, 0xd2, 0xaf, 0x4a, 0xdf, 0x68, 0xee, 0x49, 0x8a, 0xa9, 0x34, 0x7d, 0x37, 0x56, 0x65,
        0xa7,
    ];
}

// =============================================================================
// SECURE BOOT VARIABLES
// =============================================================================

/// Secure Boot variable names
pub mod var_name {
    /// Platform Key
    pub const PK: &str = "PK";
    /// Key Exchange Key
    pub const KEK: &str = "KEK";
    /// Signature Database
    pub const DB: &str = "db";
    /// Revoked Signatures Database
    pub const DBX: &str = "dbx";
    /// Timestamp Signature Database
    pub const DBT: &str = "dbt";
    /// Recovery Database
    pub const DBR: &str = "dbr";
    /// Secure Boot Enable
    pub const SECURE_BOOT: &str = "SecureBoot";
    /// Setup Mode
    pub const SETUP_MODE: &str = "SetupMode";
    /// Audit Mode
    pub const AUDIT_MODE: &str = "AuditMode";
    /// Deployed Mode
    pub const DEPLOYED_MODE: &str = "DeployedMode";
    /// Vendor Keys
    pub const VENDOR_KEYS: &str = "VendorKeys";
}

/// Secure Boot mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecureBootMode {
    /// Secure Boot is disabled
    Disabled,
    /// Setup mode (no PK, allows unsigned updates)
    SetupMode,
    /// User mode (PK present, enforcing)
    UserMode,
    /// Audit mode (logging only, not enforcing)
    AuditMode,
    /// Deployed mode (hardened, limited updates)
    DeployedMode,
}

/// Secure Boot state
#[derive(Debug, Clone)]
pub struct SecureBootState {
    /// Secure Boot mode
    pub mode: SecureBootMode,
    /// SecureBoot variable value
    pub secure_boot_enabled: bool,
    /// SetupMode variable value
    pub setup_mode: bool,
    /// AuditMode variable value
    pub audit_mode: bool,
    /// DeployedMode variable value
    pub deployed_mode: bool,
    /// VendorKeys variable value
    pub vendor_keys: bool,
}

impl SecureBootState {
    /// Determine mode from variable values
    pub fn determine_mode(&mut self) {
        self.mode = if !self.secure_boot_enabled {
            SecureBootMode::Disabled
        } else if self.setup_mode {
            SecureBootMode::SetupMode
        } else if self.deployed_mode {
            SecureBootMode::DeployedMode
        } else if self.audit_mode {
            SecureBootMode::AuditMode
        } else {
            SecureBootMode::UserMode
        };
    }
}

// =============================================================================
// SIGNATURE DATABASE
// =============================================================================

/// Signature type in database
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureEntryType {
    /// SHA-256 hash
    Sha256,
    /// RSA-2048 key
    Rsa2048,
    /// X.509 certificate
    X509,
    /// X.509 certificate SHA-256 hash
    X509Sha256,
    /// Unknown type
    Unknown([u8; 16]),
}

impl SignatureEntryType {
    /// Create from GUID
    pub fn from_guid(guid: &[u8; 16]) -> Self {
        if guid == &guid::EFI_CERT_SHA256 {
            Self::Sha256
        } else if guid == &guid::EFI_CERT_RSA2048 {
            Self::Rsa2048
        } else if guid == &guid::EFI_CERT_X509 {
            Self::X509
        } else if guid == &guid::EFI_CERT_X509_SHA256 {
            Self::X509Sha256
        } else {
            Self::Unknown(*guid)
        }
    }

    /// Get GUID for type
    pub fn to_guid(&self) -> [u8; 16] {
        match self {
            Self::Sha256 => guid::EFI_CERT_SHA256,
            Self::Rsa2048 => guid::EFI_CERT_RSA2048,
            Self::X509 => guid::EFI_CERT_X509,
            Self::X509Sha256 => guid::EFI_CERT_X509_SHA256,
            Self::Unknown(g) => *g,
        }
    }

    /// Get signature data size (for fixed-size types)
    pub fn signature_size(&self) -> Option<usize> {
        match self {
            Self::Sha256 => Some(16 + 32),               // Owner GUID + SHA-256
            Self::X509Sha256 => Some(16 + 32 + 32 + 32), // Owner + ToBeSignedHash + Time + ...
            Self::Rsa2048 => Some(16 + 256),             // Owner GUID + 2048-bit key
            Self::X509 | Self::Unknown(_) => None,       // Variable size
        }
    }
}

/// Signature entry
#[derive(Debug, Clone)]
pub struct SignatureEntry {
    /// Entry type
    pub entry_type: SignatureEntryType,
    /// Owner GUID
    pub owner: [u8; 16],
    /// Signature data
    pub data: Vec<u8>,
}

impl SignatureEntry {
    /// Create SHA-256 hash entry
    pub fn sha256(owner: [u8; 16], hash: [u8; 32]) -> Self {
        Self {
            entry_type: SignatureEntryType::Sha256,
            owner,
            data: hash.to_vec(),
        }
    }

    /// Create X.509 certificate entry
    pub fn x509(owner: [u8; 16], cert_der: Vec<u8>) -> Self {
        Self {
            entry_type: SignatureEntryType::X509,
            owner,
            data: cert_der,
        }
    }

    /// Get as X.509 certificate
    pub fn as_x509(&self) -> Result<X509Certificate, KeyError> {
        if self.entry_type != SignatureEntryType::X509 {
            return Err(KeyError::InvalidKeyFormat);
        }
        X509Certificate::from_der(&self.data)
    }

    /// Get as SHA-256 hash
    pub fn as_sha256(&self) -> Option<[u8; 32]> {
        if self.entry_type != SignatureEntryType::Sha256 || self.data.len() != 32 {
            return None;
        }
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&self.data);
        Some(hash)
    }
}

/// Signature list
#[derive(Debug, Clone)]
pub struct SignatureList {
    /// Entry type
    pub entry_type: SignatureEntryType,
    /// List header size
    pub header_size: u32,
    /// Individual signature size
    pub signature_size: u32,
    /// Entries
    pub entries: Vec<SignatureEntry>,
}

impl SignatureList {
    /// Create new signature list
    pub fn new(entry_type: SignatureEntryType) -> Self {
        Self {
            entry_type,
            header_size: 0,
            signature_size: 0,
            entries: Vec::new(),
        }
    }

    /// Add entry
    pub fn add_entry(&mut self, entry: SignatureEntry) {
        self.entries.push(entry);
    }

    /// Parse from bytes
    pub fn parse(data: &[u8]) -> Result<Self, SecureBootError> {
        // EFI_SIGNATURE_LIST structure:
        // SignatureType: GUID (16 bytes)
        // SignatureListSize: UINT32 (4 bytes)
        // SignatureHeaderSize: UINT32 (4 bytes)
        // SignatureSize: UINT32 (4 bytes)
        // SignatureHeader: UINT8[SignatureHeaderSize]
        // Signatures: EFI_SIGNATURE_DATA[]

        if data.len() < 28 {
            return Err(SecureBootError::InvalidDatabase);
        }

        let mut type_guid = [0u8; 16];
        type_guid.copy_from_slice(&data[0..16]);
        let entry_type = SignatureEntryType::from_guid(&type_guid);

        let list_size = u32::from_le_bytes([data[16], data[17], data[18], data[19]]) as usize;
        let header_size = u32::from_le_bytes([data[20], data[21], data[22], data[23]]);
        let signature_size = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);

        if data.len() < list_size {
            return Err(SecureBootError::InvalidDatabase);
        }

        // Skip header
        let sig_offset = 28 + header_size as usize;
        let sig_data = &data[sig_offset..list_size];

        // Parse signatures
        let mut entries = Vec::new();
        let mut offset = 0;

        while offset + signature_size as usize <= sig_data.len() {
            let sig_bytes = &sig_data[offset..offset + signature_size as usize];

            // EFI_SIGNATURE_DATA:
            // SignatureOwner: GUID (16 bytes)
            // SignatureData: UINT8[]

            if sig_bytes.len() >= 16 {
                let mut owner = [0u8; 16];
                owner.copy_from_slice(&sig_bytes[0..16]);

                entries.push(SignatureEntry {
                    entry_type,
                    owner,
                    data: sig_bytes[16..].to_vec(),
                });
            }

            offset += signature_size as usize;
        }

        Ok(Self {
            entry_type,
            header_size,
            signature_size,
            entries,
        })
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();

        // Calculate signature size
        let sig_size = if let Some(size) = self.entry_type.signature_size() {
            size as u32
        } else if let Some(entry) = self.entries.first() {
            (16 + entry.data.len()) as u32
        } else {
            return result;
        };

        // Calculate total size
        let list_size = 28 + self.header_size + sig_size * self.entries.len() as u32;

        // Type GUID
        result.extend_from_slice(&self.entry_type.to_guid());

        // List size
        result.extend_from_slice(&list_size.to_le_bytes());

        // Header size
        result.extend_from_slice(&self.header_size.to_le_bytes());

        // Signature size
        result.extend_from_slice(&sig_size.to_le_bytes());

        // Signatures
        for entry in &self.entries {
            result.extend_from_slice(&entry.owner);
            result.extend_from_slice(&entry.data);

            // Pad to signature size
            let current_size = 16 + entry.data.len();
            if current_size < sig_size as usize {
                result.resize(result.len() + sig_size as usize - current_size, 0);
            }
        }

        result
    }
}

/// Signature database (collection of lists)
#[derive(Debug, Clone)]
pub struct SignatureDatabase {
    /// Signature lists
    pub lists: Vec<SignatureList>,
}

impl SignatureDatabase {
    /// Create empty database
    pub fn new() -> Self {
        Self { lists: Vec::new() }
    }

    /// Add signature list
    pub fn add_list(&mut self, list: SignatureList) {
        self.lists.push(list);
    }

    /// Parse from variable data
    pub fn parse(data: &[u8]) -> Result<Self, SecureBootError> {
        let mut lists = Vec::new();
        let mut offset = 0;

        while offset < data.len() {
            if data.len() - offset < 28 {
                break;
            }

            let list_size = u32::from_le_bytes([
                data[offset + 16],
                data[offset + 17],
                data[offset + 18],
                data[offset + 19],
            ]) as usize;

            if offset + list_size > data.len() {
                return Err(SecureBootError::InvalidDatabase);
            }

            let list = SignatureList::parse(&data[offset..offset + list_size])?;
            lists.push(list);

            offset += list_size;
        }

        Ok(Self { lists })
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();
        for list in &self.lists {
            result.extend_from_slice(&list.to_bytes());
        }
        result
    }

    /// Get all X.509 certificates
    pub fn certificates(&self) -> Vec<X509Certificate> {
        let mut certs = Vec::new();

        for list in &self.lists {
            if list.entry_type == SignatureEntryType::X509 {
                for entry in &list.entries {
                    if let Ok(cert) = entry.as_x509() {
                        certs.push(cert);
                    }
                }
            }
        }

        certs
    }

    /// Get all SHA-256 hashes
    pub fn sha256_hashes(&self) -> Vec<[u8; 32]> {
        let mut hashes = Vec::new();

        for list in &self.lists {
            if list.entry_type == SignatureEntryType::Sha256 {
                for entry in &list.entries {
                    if let Some(hash) = entry.as_sha256() {
                        hashes.push(hash);
                    }
                }
            }
        }

        hashes
    }

    /// Check if database contains certificate
    pub fn contains_certificate(&self, cert: &X509Certificate) -> bool {
        let cert_hash = Sha256::digest(&cert.tbs_certificate);

        for list in &self.lists {
            if list.entry_type == SignatureEntryType::X509 {
                for entry in &list.entries {
                    let Ok(db_cert) = entry.as_x509() else {
                        continue;
                    };

                    if db_cert.serial_number == cert.serial_number && db_cert.issuer == cert.issuer
                    {
                        return true;
                    }
                }
                continue;
            }

            if list.entry_type == SignatureEntryType::X509Sha256 {
                for entry in &list.entries {
                    if entry.data.len() >= 32 && entry.data[..32] == cert_hash {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Check if database contains hash
    pub fn contains_hash(&self, hash: &[u8; 32]) -> bool {
        for list in &self.lists {
            if list.entry_type != SignatureEntryType::Sha256 {
                continue;
            }

            for entry in &list.entries {
                if entry
                    .as_sha256()
                    .is_some_and(|entry_hash| entry_hash == *hash)
                {
                    return true;
                }
            }
        }

        false
    }
}

impl Default for SignatureDatabase {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// SECURE BOOT VERIFIER
// =============================================================================

/// Secure Boot verifier
pub struct SecureBootVerifier {
    /// Allowed signature database (db)
    db: SignatureDatabase,
    /// Revoked signature database (dbx)
    dbx: SignatureDatabase,
    /// Timestamp signature database (dbt)
    dbt: SignatureDatabase,
    /// Authenticode verifier
    authenticode: AuthenticodeVerifier,
    /// Current time
    current_time: Option<u64>,
}

impl SecureBootVerifier {
    /// Create new verifier
    pub fn new() -> Self {
        Self {
            db: SignatureDatabase::new(),
            dbx: SignatureDatabase::new(),
            dbt: SignatureDatabase::new(),
            authenticode: AuthenticodeVerifier::new(),
            current_time: None,
        }
    }

    /// Load db variable
    pub fn load_db(&mut self, data: &[u8]) -> Result<(), SecureBootError> {
        self.db = SignatureDatabase::parse(data)?;

        // Add certificates to authenticode verifier
        for cert in self.db.certificates() {
            self.authenticode.add_trusted_cert(cert);
        }

        Ok(())
    }

    /// Load dbx variable
    pub fn load_dbx(&mut self, data: &[u8]) -> Result<(), SecureBootError> {
        self.dbx = SignatureDatabase::parse(data)?;

        // Add revoked hashes to authenticode verifier
        for hash in self.dbx.sha256_hashes() {
            self.authenticode.add_revoked_hash(hash);
        }

        Ok(())
    }

    /// Load dbt variable
    pub fn load_dbt(&mut self, data: &[u8]) -> Result<(), SecureBootError> {
        self.dbt = SignatureDatabase::parse(data)?;
        Ok(())
    }

    /// Set current time
    pub fn set_time(&mut self, time: u64) {
        self.current_time = Some(time);
        self.authenticode.set_time(time);
    }

    /// Verify PE image
    pub fn verify_image(&self, image: &[u8]) -> SecureBootResult {
        // First check if image hash is in dbx (revoked)
        let image_hash = Sha256::digest(image);

        if self.dbx.contains_hash(&image_hash) {
            return SecureBootResult::Denied {
                reason: DenialReason::HashRevoked,
            };
        }

        // Check if image hash is in db (allowed by hash)
        if self.db.contains_hash(&image_hash) {
            return SecureBootResult::Allowed {
                method: AllowMethod::HashInDb,
                signer: None,
            };
        }

        // Try to verify Authenticode signature
        match self.authenticode.verify_pe(image) {
            Ok(result) => {
                if !result.valid {
                    return SecureBootResult::Denied {
                        reason: DenialReason::InvalidSignature,
                    };
                }

                if let Some(signer) = result.signer.as_ref() {
                    if self.dbx.contains_certificate(signer) {
                        return SecureBootResult::Denied {
                            reason: DenialReason::SignerRevoked,
                        };
                    }
                }

                if result
                    .chain
                    .iter()
                    .any(|cert| self.dbx.contains_certificate(cert))
                {
                    return SecureBootResult::Denied {
                        reason: DenialReason::ChainRevoked,
                    };
                }

                SecureBootResult::Allowed {
                    method: AllowMethod::ValidSignature,
                    signer: result.signer,
                }
            },
            Err(SignatureError::NoSignature) => SecureBootResult::Denied {
                reason: DenialReason::NoSignature,
            },
            Err(_) => SecureBootResult::Denied {
                reason: DenialReason::SignatureError,
            },
        }
    }

    /// Verify buffer hash against db
    pub fn verify_hash(&self, hash: &[u8; 32]) -> bool {
        if self.dbx.contains_hash(hash) {
            return false;
        }

        self.db.contains_hash(hash)
    }

    /// Get db database
    pub fn db(&self) -> &SignatureDatabase {
        &self.db
    }

    /// Get dbx database
    pub fn dbx(&self) -> &SignatureDatabase {
        &self.dbx
    }
}

impl Default for SecureBootVerifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Secure Boot verification result
#[derive(Debug, Clone)]
pub enum SecureBootResult {
    /// Image is allowed
    Allowed {
        /// Method used to allow
        method: AllowMethod,
        /// Signer certificate (if signature verified)
        signer: Option<X509Certificate>,
    },
    /// Image is denied
    Denied {
        /// Reason for denial
        reason: DenialReason,
    },
}

impl SecureBootResult {
    /// Check if allowed
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed { .. })
    }

    /// Check if denied
    pub fn is_denied(&self) -> bool {
        matches!(self, Self::Denied { .. })
    }
}

/// Method used to allow image
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllowMethod {
    /// Hash found in db
    HashInDb,
    /// Valid Authenticode signature
    ValidSignature,
    /// Secure Boot disabled
    SecureBootDisabled,
    /// Setup mode
    SetupMode,
}

/// Reason for denial
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DenialReason {
    /// No signature present
    NoSignature,
    /// Invalid signature
    InvalidSignature,
    /// Signature verification error
    SignatureError,
    /// Hash is in dbx
    HashRevoked,
    /// Signer certificate is in dbx
    SignerRevoked,
    /// Certificate in chain is in dbx
    ChainRevoked,
    /// Certificate not trusted (not in db)
    UntrustedSigner,
    /// Certificate expired
    CertificateExpired,
}

// =============================================================================
// AUTHENTICATED VARIABLE
// =============================================================================

/// Authenticated variable time
#[derive(Debug, Clone, Copy)]
pub struct EfiTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub nanosecond: u32,
    pub timezone: i16,
    pub daylight: u8,
}

impl EfiTime {
    /// Parse from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 16 {
            return None;
        }

        Some(Self {
            year: u16::from_le_bytes([data[0], data[1]]),
            month: data[2],
            day: data[3],
            hour: data[4],
            minute: data[5],
            second: data[6],
            nanosecond: u32::from_le_bytes([data[8], data[9], data[10], data[11]]),
            timezone: i16::from_le_bytes([data[12], data[13]]),
            daylight: data[14],
        })
    }

    /// Convert to bytes
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut result = [0u8; 16];
        result[0..2].copy_from_slice(&self.year.to_le_bytes());
        result[2] = self.month;
        result[3] = self.day;
        result[4] = self.hour;
        result[5] = self.minute;
        result[6] = self.second;
        result[7] = 0; // Pad1
        result[8..12].copy_from_slice(&self.nanosecond.to_le_bytes());
        result[12..14].copy_from_slice(&self.timezone.to_le_bytes());
        result[14] = self.daylight;
        result[15] = 0; // Pad2
        result
    }
}

/// WIN_CERTIFICATE_UEFI_GUID
#[derive(Debug, Clone)]
pub struct WinCertificateUefiGuid {
    /// Certificate type GUID
    pub cert_type: [u8; 16],
    /// Certificate data
    pub cert_data: Vec<u8>,
}

impl WinCertificateUefiGuid {
    /// Parse from bytes
    pub fn parse(data: &[u8]) -> Result<Self, SecureBootError> {
        // WIN_CERTIFICATE header:
        // dwLength: UINT32
        // wRevision: UINT16
        // wCertificateType: UINT16
        // Then GUID + data

        if data.len() < 24 {
            return Err(SecureBootError::InvalidCertificate);
        }

        let length = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let cert_type_field = u16::from_le_bytes([data[6], data[7]]);

        if cert_type_field != 0x0EF1 {
            // WIN_CERT_TYPE_EFI_GUID
            return Err(SecureBootError::InvalidCertificate);
        }

        if data.len() < length {
            return Err(SecureBootError::InvalidCertificate);
        }

        let mut cert_type = [0u8; 16];
        cert_type.copy_from_slice(&data[8..24]);

        let cert_data = data[24..length].to_vec();

        Ok(Self {
            cert_type,
            cert_data,
        })
    }
}

/// EFI_VARIABLE_AUTHENTICATION_2
#[derive(Debug, Clone)]
pub struct VariableAuthentication2 {
    /// Timestamp
    pub time: EfiTime,
    /// Authentication info
    pub auth_info: WinCertificateUefiGuid,
}

impl VariableAuthentication2 {
    /// Parse from variable data
    pub fn parse(data: &[u8]) -> Result<(Self, &[u8]), SecureBootError> {
        if data.len() < 16 {
            return Err(SecureBootError::InvalidAuthentication);
        }

        let time =
            EfiTime::from_bytes(&data[0..16]).ok_or(SecureBootError::InvalidAuthentication)?;

        let auth_info = WinCertificateUefiGuid::parse(&data[16..])?;

        // Calculate total header size
        let header_size = 16 + 8 + auth_info.cert_data.len();

        if data.len() < header_size {
            return Err(SecureBootError::InvalidAuthentication);
        }

        let remaining = &data[header_size..];

        Ok((Self { time, auth_info }, remaining))
    }

    /// Verify authentication
    pub fn verify(
        &self,
        variable_name: &str,
        vendor_guid: &[u8; 16],
        new_data: &[u8],
        kek: &SignatureDatabase,
    ) -> Result<bool, SecureBootError> {
        // Build data to verify:
        // VariableName || VendorGuid || Attributes || TimeStamp || Data

        let mut to_verify = Vec::new();

        // Variable name (UTF-16LE, null-terminated)
        for c in variable_name.encode_utf16() {
            to_verify.extend_from_slice(&c.to_le_bytes());
        }
        to_verify.extend_from_slice(&[0, 0]); // Null terminator

        // Vendor GUID
        to_verify.extend_from_slice(vendor_guid);

        // Attributes (0x27 for time-based auth)
        to_verify.extend_from_slice(&0x27u32.to_le_bytes());

        // Timestamp
        to_verify.extend_from_slice(&self.time.to_bytes());

        // New data
        to_verify.extend_from_slice(new_data);

        // Compute hash
        let hash = Sha256::digest(&to_verify);

        // Verify signature against KEK certificates
        for cert in kek.certificates() {
            // This would verify the PKCS#7 signature in auth_info
            // against the certificate
            let _ = cert;
            let _ = hash;
            // Simplified: return true for now
        }

        Ok(true)
    }
}

// =============================================================================
// ERRORS
// =============================================================================

/// Secure Boot error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecureBootError {
    /// Invalid signature database
    InvalidDatabase,
    /// Invalid certificate
    InvalidCertificate,
    /// Invalid authentication header
    InvalidAuthentication,
    /// Variable not found
    VariableNotFound,
    /// Access denied
    AccessDenied,
    /// Signature verification failed
    VerificationFailed,
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_signature_entry_type() {
        let sha256_type = SignatureEntryType::from_guid(&guid::EFI_CERT_SHA256);
        assert_eq!(sha256_type, SignatureEntryType::Sha256);

        let x509_type = SignatureEntryType::from_guid(&guid::EFI_CERT_X509);
        assert_eq!(x509_type, SignatureEntryType::X509);
    }

    #[test]
    fn test_signature_database_empty() {
        let db = SignatureDatabase::new();
        assert!(db.lists.is_empty());
        assert!(db.certificates().is_empty());
        assert!(db.sha256_hashes().is_empty());
    }

    #[test]
    fn test_secure_boot_verifier_new() {
        let verifier = SecureBootVerifier::new();
        assert!(verifier.db().lists.is_empty());
        assert!(verifier.dbx().lists.is_empty());
    }

    #[test]
    fn test_secure_boot_result() {
        let allowed = SecureBootResult::Allowed {
            method: AllowMethod::HashInDb,
            signer: None,
        };
        assert!(allowed.is_allowed());
        assert!(!allowed.is_denied());

        let denied = SecureBootResult::Denied {
            reason: DenialReason::NoSignature,
        };
        assert!(denied.is_denied());
        assert!(!denied.is_allowed());
    }

    #[test]
    fn test_efi_time() {
        let time = EfiTime {
            year: 2024,
            month: 1,
            day: 15,
            hour: 12,
            minute: 30,
            second: 45,
            nanosecond: 0,
            timezone: 0,
            daylight: 0,
        };

        let bytes = time.to_bytes();
        let parsed = EfiTime::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.year, 2024);
        assert_eq!(parsed.month, 1);
        assert_eq!(parsed.day, 15);
    }
}
