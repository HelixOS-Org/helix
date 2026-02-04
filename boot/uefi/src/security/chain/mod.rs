//! Boot Chain of Trust for Helix UEFI Bootloader
//!
//! This module implements a complete chain of trust verification system
//! for secure boot environments, including certificate management,
//! signature verification, and measured boot support.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                        Chain of Trust                                   │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐             │
//! │  │   Platform   │───▶│    UEFI      │───▶│   Shim/     │             │
//! │  │     Key      │    │   Firmware   │    │   Bootloader│             │
//! │  │   (PK/KEK)   │    │   Signed     │    │   Signed    │             │
//! │  └──────────────┘    └──────────────┘    └──────────────┘             │
//! │         │                   │                   │                      │
//! │         ▼                   ▼                   ▼                      │
//! │  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐             │
//! │  │   Secure     │───▶│   Driver     │───▶│   Kernel     │             │
//! │  │   Variables  │    │   Signed     │    │   Signed     │             │
//! │  └──────────────┘    └──────────────┘    └──────────────┘             │
//! │                                                                         │
//! │  TPM Measurements:  PCR[0-7] ────▶ PCR[8-15] ────▶ PCR Extension       │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Security Features
//!
//! - Secure Boot verification
//! - TPM 2.0 measured boot
//! - Certificate chain validation
//! - Signature algorithms (RSA, ECDSA)
//! - Hash algorithms (SHA-256, SHA-384, SHA-512)
//! - Revocation list support (dbx)
//! - Policy enforcement

#![no_std]

use core::fmt;

// =============================================================================
// SECURE BOOT DATABASES
// =============================================================================

/// Secure Boot database GUIDs
pub mod db_guids {
    use crate::guid::Guid;

    /// EFI Global Variable GUID
    pub const EFI_GLOBAL_VARIABLE: Guid = Guid::new(
        0x8BE4DF61,
        0x93CA,
        0x11D2,
        [0xAA, 0x0D, 0x00, 0xE0, 0x98, 0x03, 0x2B, 0x8C],
    );

    /// EFI Image Security Database GUID
    pub const EFI_IMAGE_SECURITY_DATABASE: Guid = Guid::new(
        0xD719B2CB,
        0x3D3A,
        0x4596,
        [0xA3, 0xBC, 0xDA, 0xD0, 0x0E, 0x67, 0x65, 0x6F],
    );

    /// Microsoft KEK CA GUID
    pub const MICROSOFT_KEK_CA: Guid = Guid::new(
        0x77FA9ABD,
        0x0359,
        0x4D32,
        [0xBD, 0x60, 0x28, 0xF4, 0xE7, 0x8F, 0x78, 0x4B],
    );

    /// Microsoft Windows Production PCA GUID
    pub const MICROSOFT_WINDOWS_PCA: Guid = Guid::new(
        0x46215B06,
        0x72BD,
        0x428D,
        [0xA5, 0x55, 0xCC, 0x9A, 0x06, 0x67, 0xE0, 0xC7],
    );

    /// Microsoft UEFI CA GUID
    pub const MICROSOFT_UEFI_CA: Guid = Guid::new(
        0xBD9AFB91,
        0xC02A,
        0x4C5F,
        [0x96, 0x28, 0x5B, 0x10, 0x8D, 0x58, 0x80, 0x00],
    );
}

/// Secure Boot database names
pub const DB_NAME: &str = "db";
pub const DBX_NAME: &str = "dbx";
pub const DBT_NAME: &str = "dbt";
pub const PK_NAME: &str = "PK";
pub const KEK_NAME: &str = "KEK";

/// Secure Boot state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecureBootState {
    /// Secure Boot is disabled
    Disabled,
    /// Secure Boot is in setup mode
    SetupMode,
    /// Secure Boot is in user mode (enabled)
    UserMode,
    /// Secure Boot is in audit mode
    AuditMode,
    /// Secure Boot is in deployed mode
    DeployedMode,
}

impl SecureBootState {
    /// Check if Secure Boot is enforcing
    pub const fn is_enforcing(&self) -> bool {
        matches!(self, SecureBootState::UserMode | SecureBootState::DeployedMode)
    }

    /// Check if Secure Boot allows enrollment
    pub const fn allows_enrollment(&self) -> bool {
        matches!(self, SecureBootState::SetupMode | SecureBootState::AuditMode)
    }
}

impl Default for SecureBootState {
    fn default() -> Self {
        SecureBootState::Disabled
    }
}

// =============================================================================
// CERTIFICATE TYPES
// =============================================================================

/// Certificate type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertificateType {
    /// X.509 certificate
    X509,
    /// RSA-2048 public key
    Rsa2048,
    /// RSA-2048 with SHA-256
    Rsa2048Sha256,
    /// RSA-2048 with SHA-1
    Rsa2048Sha1,
    /// SHA-256 hash
    Sha256,
    /// SHA-1 hash (deprecated)
    Sha1,
    /// X.509 with SHA-256
    X509Sha256,
    /// X.509 with SHA-384
    X509Sha384,
    /// X.509 with SHA-512
    X509Sha512,
    /// ECDSA with P-256
    EcdsaP256,
    /// ECDSA with P-384
    EcdsaP384,
}

impl CertificateType {
    /// Get type GUID
    pub const fn guid(&self) -> [u8; 16] {
        match self {
            CertificateType::X509 => [
                0xA5, 0xC0, 0x59, 0xA1, 0x63, 0xB9, 0xCC, 0x4A,
                0x9A, 0x9F, 0x89, 0x71, 0x33, 0x43, 0x42, 0xE3,
            ],
            CertificateType::Rsa2048 => [
                0xE8, 0x66, 0x57, 0x3C, 0x9C, 0x26, 0x34, 0x4E,
                0xAA, 0x14, 0xED, 0x77, 0x6E, 0x85, 0xB3, 0xB6,
            ],
            CertificateType::Rsa2048Sha256 => [
                0x90, 0x61, 0xB3, 0xE2, 0x29, 0x91, 0xF3, 0x4A,
                0xAD, 0xB4, 0x7E, 0x79, 0x8C, 0x3A, 0x94, 0xE8,
            ],
            CertificateType::Sha256 => [
                0x26, 0x16, 0xC4, 0xC1, 0x4C, 0x50, 0x92, 0x40,
                0xAC, 0xA9, 0x41, 0xF9, 0x36, 0x93, 0x43, 0x28,
            ],
            _ => [0u8; 16],
        }
    }
}

/// Certificate entry in database
#[derive(Debug, Clone)]
pub struct CertificateEntry {
    /// Certificate type
    pub cert_type: CertificateType,
    /// Certificate owner GUID
    pub owner: [u8; 16],
    /// Certificate data
    pub data: [u8; 4096],
    /// Data length
    pub data_len: usize,
}

impl CertificateEntry {
    /// Create new certificate entry
    pub const fn new(cert_type: CertificateType, owner: [u8; 16]) -> Self {
        Self {
            cert_type,
            owner,
            data: [0u8; 4096],
            data_len: 0,
        }
    }

    /// Set certificate data
    pub fn set_data(&mut self, data: &[u8]) -> bool {
        if data.len() > self.data.len() {
            return false;
        }
        self.data[..data.len()].copy_from_slice(data);
        self.data_len = data.len();
        true
    }

    /// Get certificate data
    pub fn data(&self) -> &[u8] {
        &self.data[..self.data_len]
    }
}

// =============================================================================
// SIGNATURE VERIFICATION
// =============================================================================

/// Signature algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureAlgorithm {
    /// RSA with PKCS#1 v1.5 padding
    RsaPkcs1v15,
    /// RSA with PSS padding
    RsaPss,
    /// ECDSA
    Ecdsa,
    /// EdDSA (Ed25519)
    EdDsa,
}

/// Hash algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    /// SHA-1 (deprecated)
    Sha1,
    /// SHA-256
    Sha256,
    /// SHA-384
    Sha384,
    /// SHA-512
    Sha512,
    /// SHA3-256
    Sha3_256,
    /// SHA3-384
    Sha3_384,
    /// SHA3-512
    Sha3_512,
    /// SM3 (Chinese standard)
    Sm3,
}

impl HashAlgorithm {
    /// Get hash digest size in bytes
    pub const fn digest_size(&self) -> usize {
        match self {
            HashAlgorithm::Sha1 => 20,
            HashAlgorithm::Sha256 | HashAlgorithm::Sha3_256 | HashAlgorithm::Sm3 => 32,
            HashAlgorithm::Sha384 | HashAlgorithm::Sha3_384 => 48,
            HashAlgorithm::Sha512 | HashAlgorithm::Sha3_512 => 64,
        }
    }

    /// Get TPM algorithm ID
    pub const fn tpm_alg_id(&self) -> u16 {
        match self {
            HashAlgorithm::Sha1 => 0x0004,
            HashAlgorithm::Sha256 => 0x000B,
            HashAlgorithm::Sha384 => 0x000C,
            HashAlgorithm::Sha512 => 0x000D,
            HashAlgorithm::Sha3_256 => 0x0027,
            HashAlgorithm::Sha3_384 => 0x0028,
            HashAlgorithm::Sha3_512 => 0x0029,
            HashAlgorithm::Sm3 => 0x0012,
        }
    }

    /// Check if algorithm is considered secure
    pub const fn is_secure(&self) -> bool {
        !matches!(self, HashAlgorithm::Sha1)
    }
}

/// Signature structure
#[derive(Debug, Clone)]
pub struct Signature {
    /// Signature algorithm
    pub algorithm: SignatureAlgorithm,
    /// Hash algorithm
    pub hash_algorithm: HashAlgorithm,
    /// Signature data
    pub data: [u8; 512],
    /// Data length
    pub data_len: usize,
}

impl Signature {
    /// Create new signature
    pub const fn new(algorithm: SignatureAlgorithm, hash: HashAlgorithm) -> Self {
        Self {
            algorithm,
            hash_algorithm: hash,
            data: [0u8; 512],
            data_len: 0,
        }
    }

    /// Set signature data
    pub fn set_data(&mut self, data: &[u8]) -> bool {
        if data.len() > self.data.len() {
            return false;
        }
        self.data[..data.len()].copy_from_slice(data);
        self.data_len = data.len();
        true
    }

    /// Get signature data
    pub fn data(&self) -> &[u8] {
        &self.data[..self.data_len]
    }
}

// =============================================================================
// AUTHENTICODE
// =============================================================================

/// WIN_CERTIFICATE types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum WinCertificateType {
    /// X.509 certificate
    X509 = 0x0001,
    /// PKCS#7 SignedData
    PkcsSignedData = 0x0002,
    /// Reserved
    Reserved = 0x0003,
    /// Authenticode signature
    PkcsSignedDataTs = 0x0004,
}

/// WIN_CERTIFICATE header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct WinCertificate {
    /// Length of certificate
    pub length: u32,
    /// Revision
    pub revision: u16,
    /// Certificate type
    pub certificate_type: u16,
}

impl WinCertificate {
    /// UEFI revision
    pub const UEFI_REVISION: u16 = 0x0200;

    /// Create new certificate header
    pub const fn new(length: u32, cert_type: WinCertificateType) -> Self {
        Self {
            length,
            revision: Self::UEFI_REVISION,
            certificate_type: cert_type as u16,
        }
    }
}

/// EFI_CERT_TYPE_GUID for Authenticode
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct WinCertificateUefiGuid {
    /// Base header
    pub header: WinCertificate,
    /// Certificate type GUID
    pub cert_type: [u8; 16],
    /// Certificate data follows
}

impl WinCertificateUefiGuid {
    /// PKCS#7 GUID
    pub const PKCS7_GUID: [u8; 16] = [
        0x9D, 0xD2, 0xAF, 0x4A, 0xDF, 0x68, 0xEE, 0x49,
        0x8A, 0xA9, 0x34, 0x7D, 0x37, 0x56, 0x65, 0xA7,
    ];
}

/// Authenticode digest info
#[derive(Debug, Clone)]
pub struct AuthenticodeDigest {
    /// Hash algorithm
    pub algorithm: HashAlgorithm,
    /// Digest value
    pub digest: [u8; 64],
    /// Digest length
    pub digest_len: usize,
}

impl AuthenticodeDigest {
    /// Create new digest
    pub const fn new(algorithm: HashAlgorithm) -> Self {
        Self {
            algorithm,
            digest: [0u8; 64],
            digest_len: 0,
        }
    }

    /// Set digest value
    pub fn set(&mut self, value: &[u8]) -> bool {
        if value.len() > self.digest.len() {
            return false;
        }
        self.digest[..value.len()].copy_from_slice(value);
        self.digest_len = value.len();
        true
    }

    /// Get digest value
    pub fn value(&self) -> &[u8] {
        &self.digest[..self.digest_len]
    }

    /// Compare with another digest
    pub fn matches(&self, other: &[u8]) -> bool {
        if self.digest_len != other.len() {
            return false;
        }
        // Constant-time comparison
        let mut result = 0u8;
        for i in 0..self.digest_len {
            result |= self.digest[i] ^ other[i];
        }
        result == 0
    }
}

// =============================================================================
// IMAGE VERIFICATION
// =============================================================================

/// Image verification policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationPolicy {
    /// Allow all images
    Allow,
    /// Deny all images
    Deny,
    /// Verify signature
    Verify,
    /// Allow unsigned (audit mode)
    AuditAllow,
    /// Deny but log (audit mode)
    AuditDeny,
}

/// Image type for verification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageType {
    /// Application
    Application,
    /// Boot service driver
    BootServiceDriver,
    /// Runtime driver
    RuntimeDriver,
    /// ROM image
    Rom,
    /// Platform driver
    PlatformDriver,
}

/// Image verification result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationResult {
    /// Image verified successfully
    Success,
    /// Image is in db (allowed)
    AllowedByDb,
    /// Image is in dbx (denied)
    DeniedByDbx,
    /// Image is not signed
    NotSigned,
    /// Signature is invalid
    InvalidSignature,
    /// Certificate is expired
    CertificateExpired,
    /// Certificate is revoked
    CertificateRevoked,
    /// Certificate chain invalid
    InvalidChain,
    /// Hash mismatch
    HashMismatch,
    /// Unknown signer
    UnknownSigner,
    /// Policy denied
    PolicyDenied,
}

impl VerificationResult {
    /// Check if verification passed
    pub const fn is_success(&self) -> bool {
        matches!(self, VerificationResult::Success | VerificationResult::AllowedByDb)
    }

    /// Check if verification should be logged
    pub const fn should_log(&self) -> bool {
        !matches!(self, VerificationResult::Success)
    }
}

/// Image verification context
#[derive(Debug, Clone)]
pub struct VerificationContext {
    /// Image type
    pub image_type: ImageType,
    /// Policy
    pub policy: VerificationPolicy,
    /// Required hash algorithm
    pub required_hash: HashAlgorithm,
    /// Allow SHA-1
    pub allow_sha1: bool,
    /// Check revocation
    pub check_revocation: bool,
    /// Verify timestamp
    pub verify_timestamp: bool,
    /// Allow expired certificates
    pub allow_expired: bool,
}

impl VerificationContext {
    /// Create strict context
    pub const fn strict() -> Self {
        Self {
            image_type: ImageType::Application,
            policy: VerificationPolicy::Verify,
            required_hash: HashAlgorithm::Sha256,
            allow_sha1: false,
            check_revocation: true,
            verify_timestamp: true,
            allow_expired: false,
        }
    }

    /// Create permissive context
    pub const fn permissive() -> Self {
        Self {
            image_type: ImageType::Application,
            policy: VerificationPolicy::AuditAllow,
            required_hash: HashAlgorithm::Sha256,
            allow_sha1: true,
            check_revocation: false,
            verify_timestamp: false,
            allow_expired: true,
        }
    }
}

impl Default for VerificationContext {
    fn default() -> Self {
        Self::strict()
    }
}

// =============================================================================
// TPM MEASURED BOOT
// =============================================================================

/// PCR (Platform Configuration Register) index
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PcrIndex {
    /// SRTM, BIOS, Host Platform Extensions
    Pcr0 = 0,
    /// Host Platform Configuration
    Pcr1 = 1,
    /// UEFI driver and application code
    Pcr2 = 2,
    /// UEFI driver and application configuration and data
    Pcr3 = 3,
    /// UEFI Boot Manager Code
    Pcr4 = 4,
    /// GPT/Partition Table
    Pcr5 = 5,
    /// Platform specific (resume from S4/S5)
    Pcr6 = 6,
    /// SecureBoot Policy
    Pcr7 = 7,
    /// NTFS Boot Sector
    Pcr8 = 8,
    /// NTFS Boot Block
    Pcr9 = 9,
    /// Boot Manager
    Pcr10 = 10,
    /// BitLocker Access Control
    Pcr11 = 11,
    /// Data events and configuration
    Pcr12 = 12,
    /// Boot Module Details
    Pcr13 = 13,
    /// Boot Authorities
    Pcr14 = 14,
    /// PCR[15] - User defined
    Pcr15 = 15,
    /// PCR[16] - Debug
    Pcr16 = 16,
    /// PCR[17] - DRTM
    Pcr17 = 17,
    /// PCR[18] - DRTM TXT
    Pcr18 = 18,
    /// PCR[19] - DRTM TXT
    Pcr19 = 19,
    /// PCR[20] - DRTM TXT
    Pcr20 = 20,
    /// PCR[21] - DRTM TXT
    Pcr21 = 21,
    /// PCR[22] - DRTM TXT
    Pcr22 = 22,
    /// PCR[23] - Application Support
    Pcr23 = 23,
}

impl PcrIndex {
    /// Get PCR description
    pub const fn description(&self) -> &'static str {
        match self {
            PcrIndex::Pcr0 => "SRTM, BIOS, Platform Extensions",
            PcrIndex::Pcr1 => "Host Platform Configuration",
            PcrIndex::Pcr2 => "UEFI driver and application code",
            PcrIndex::Pcr3 => "UEFI driver and application data",
            PcrIndex::Pcr4 => "UEFI Boot Manager Code",
            PcrIndex::Pcr5 => "GPT/Partition Table",
            PcrIndex::Pcr6 => "Platform specific",
            PcrIndex::Pcr7 => "SecureBoot Policy",
            _ => "Platform/Application defined",
        }
    }
}

/// PCR bank (hash algorithm)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcrBank {
    /// SHA-1 bank
    Sha1,
    /// SHA-256 bank
    Sha256,
    /// SHA-384 bank
    Sha384,
    /// SHA-512 bank
    Sha512,
    /// SM3 bank
    Sm3,
}

impl PcrBank {
    /// Get hash algorithm
    pub const fn hash_algorithm(&self) -> HashAlgorithm {
        match self {
            PcrBank::Sha1 => HashAlgorithm::Sha1,
            PcrBank::Sha256 => HashAlgorithm::Sha256,
            PcrBank::Sha384 => HashAlgorithm::Sha384,
            PcrBank::Sha512 => HashAlgorithm::Sha512,
            PcrBank::Sm3 => HashAlgorithm::Sm3,
        }
    }
}

/// PCR value
#[derive(Debug, Clone)]
pub struct PcrValue {
    /// PCR index
    pub index: PcrIndex,
    /// Hash algorithm
    pub algorithm: HashAlgorithm,
    /// Digest value
    pub digest: [u8; 64],
    /// Digest length
    pub digest_len: usize,
}

impl PcrValue {
    /// Create new PCR value
    pub const fn new(index: PcrIndex, algorithm: HashAlgorithm) -> Self {
        Self {
            index,
            algorithm,
            digest: [0u8; 64],
            digest_len: algorithm.digest_size(),
        }
    }

    /// Get digest
    pub fn digest(&self) -> &[u8] {
        &self.digest[..self.digest_len]
    }
}

/// Event type for TCG event log
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TcgEventType {
    /// Pre-boot certificate
    PrebootCert = 0x00000000,
    /// POST code
    PostCode = 0x00000001,
    /// Unused
    Unused = 0x00000002,
    /// No action
    NoAction = 0x00000003,
    /// Separator
    Separator = 0x00000004,
    /// Action
    Action = 0x00000005,
    /// Event tag
    EventTag = 0x00000006,
    /// S-CRTM contents
    SCrtmContents = 0x00000007,
    /// S-CRTM version
    SCrtmVersion = 0x00000008,
    /// CPU microcode
    CpuMicrocode = 0x00000009,
    /// Platform config flags
    PlatformConfigFlags = 0x0000000A,
    /// Table of devices
    TableOfDevices = 0x0000000B,
    /// Compact hash
    CompactHash = 0x0000000C,
    /// IPL
    Ipl = 0x0000000D,
    /// IPL partition data
    IplPartitionData = 0x0000000E,
    /// Non-host code
    NonhostCode = 0x0000000F,
    /// Non-host config
    NonhostConfig = 0x00000010,
    /// Non-host info
    NonhostInfo = 0x00000011,
    /// Omit boot device events
    OmitBootDeviceEvents = 0x00000012,
    /// EFI event base
    EfiEventBase = 0x80000000,
    /// EFI variable driver config
    EfiVariableDriverConfig = 0x80000001,
    /// EFI variable boot
    EfiVariableBoot = 0x80000002,
    /// EFI boot services application
    EfiBootServicesApplication = 0x80000003,
    /// EFI boot services driver
    EfiBootServicesDriver = 0x80000004,
    /// EFI runtime services driver
    EfiRuntimeServicesDriver = 0x80000005,
    /// EFI GPT event
    EfiGptEvent = 0x80000006,
    /// EFI action
    EfiAction = 0x80000007,
    /// EFI platform firmware blob
    EfiPlatformFirmwareBlob = 0x80000008,
    /// EFI handoff tables
    EfiHandoffTables = 0x80000009,
    /// EFI platform firmware blob 2
    EfiPlatformFirmwareBlob2 = 0x8000000A,
    /// EFI handoff tables 2
    EfiHandoffTables2 = 0x8000000B,
    /// EFI variable boot 2
    EfiVariableBoot2 = 0x8000000C,
    /// EFI GPT event 2
    EfiGptEvent2 = 0x8000000D,
    /// EFI HCRTM event
    EfiHcrtmEvent = 0x80000010,
    /// EFI variable authority
    EfiVariableAuthority = 0x800000E0,
    /// EFI SPDM firmware blob
    EfiSpdmFirmwareBlob = 0x800000E1,
    /// EFI SPDM firmware config
    EfiSpdmFirmwareConfig = 0x800000E2,
}

/// TCG event log entry (crypto-agile format)
#[derive(Debug, Clone)]
pub struct TcgEventEntry {
    /// PCR index
    pub pcr_index: u32,
    /// Event type
    pub event_type: TcgEventType,
    /// Digests (multiple hash algorithms)
    pub digests: [Option<AuthenticodeDigest>; 5],
    /// Number of digests
    pub digest_count: usize,
    /// Event data
    pub event_data: [u8; 256],
    /// Event data size
    pub event_size: usize,
}

impl TcgEventEntry {
    /// Create new event entry
    pub const fn new(pcr_index: u32, event_type: TcgEventType) -> Self {
        Self {
            pcr_index,
            event_type,
            digests: [None, None, None, None, None],
            digest_count: 0,
            event_data: [0u8; 256],
            event_size: 0,
        }
    }

    /// Add digest
    pub fn add_digest(&mut self, digest: AuthenticodeDigest) -> bool {
        if self.digest_count >= 5 {
            return false;
        }
        self.digests[self.digest_count] = Some(digest);
        self.digest_count += 1;
        true
    }

    /// Set event data
    pub fn set_event_data(&mut self, data: &[u8]) -> bool {
        if data.len() > self.event_data.len() {
            return false;
        }
        self.event_data[..data.len()].copy_from_slice(data);
        self.event_size = data.len();
        true
    }
}

// =============================================================================
// POLICY
// =============================================================================

/// Boot security policy
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    /// Allow unsigned bootloaders
    pub allow_unsigned_boot: bool,
    /// Allow unsigned drivers
    pub allow_unsigned_drivers: bool,
    /// Allow expired certificates
    pub allow_expired_certs: bool,
    /// Require TPM measurements
    pub require_tpm: bool,
    /// Minimum hash algorithm
    pub min_hash_algorithm: HashAlgorithm,
    /// Minimum key size (bits)
    pub min_key_size: u32,
    /// Allow self-signed certificates
    pub allow_self_signed: bool,
    /// Enforce dbx checking
    pub enforce_dbx: bool,
}

impl SecurityPolicy {
    /// Strict policy (production)
    pub const fn strict() -> Self {
        Self {
            allow_unsigned_boot: false,
            allow_unsigned_drivers: false,
            allow_expired_certs: false,
            require_tpm: true,
            min_hash_algorithm: HashAlgorithm::Sha256,
            min_key_size: 2048,
            allow_self_signed: false,
            enforce_dbx: true,
        }
    }

    /// Permissive policy (development)
    pub const fn permissive() -> Self {
        Self {
            allow_unsigned_boot: true,
            allow_unsigned_drivers: true,
            allow_expired_certs: true,
            require_tpm: false,
            min_hash_algorithm: HashAlgorithm::Sha1,
            min_key_size: 1024,
            allow_self_signed: true,
            enforce_dbx: false,
        }
    }

    /// Default policy
    pub const fn default_policy() -> Self {
        Self {
            allow_unsigned_boot: false,
            allow_unsigned_drivers: false,
            allow_expired_certs: false,
            require_tpm: false,
            min_hash_algorithm: HashAlgorithm::Sha256,
            min_key_size: 2048,
            allow_self_signed: false,
            enforce_dbx: true,
        }
    }
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self::default_policy()
    }
}

// =============================================================================
// ERROR TYPES
// =============================================================================

/// Chain of trust error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainError {
    /// Certificate not found
    CertificateNotFound,
    /// Certificate expired
    CertificateExpired,
    /// Certificate revoked
    CertificateRevoked,
    /// Invalid signature
    InvalidSignature,
    /// Invalid certificate chain
    InvalidChain,
    /// Hash mismatch
    HashMismatch,
    /// Unknown algorithm
    UnknownAlgorithm,
    /// Policy violation
    PolicyViolation,
    /// TPM error
    TpmError,
    /// Database error
    DatabaseError,
    /// Invalid format
    InvalidFormat,
    /// Buffer too small
    BufferTooSmall,
}

impl fmt::Display for ChainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChainError::CertificateNotFound => write!(f, "Certificate not found"),
            ChainError::CertificateExpired => write!(f, "Certificate expired"),
            ChainError::CertificateRevoked => write!(f, "Certificate revoked"),
            ChainError::InvalidSignature => write!(f, "Invalid signature"),
            ChainError::InvalidChain => write!(f, "Invalid certificate chain"),
            ChainError::HashMismatch => write!(f, "Hash mismatch"),
            ChainError::UnknownAlgorithm => write!(f, "Unknown algorithm"),
            ChainError::PolicyViolation => write!(f, "Policy violation"),
            ChainError::TpmError => write!(f, "TPM error"),
            ChainError::DatabaseError => write!(f, "Database error"),
            ChainError::InvalidFormat => write!(f, "Invalid format"),
            ChainError::BufferTooSmall => write!(f, "Buffer too small"),
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
    fn test_secure_boot_state() {
        let state = SecureBootState::UserMode;
        assert!(state.is_enforcing());
        assert!(!state.allows_enrollment());

        let setup = SecureBootState::SetupMode;
        assert!(!setup.is_enforcing());
        assert!(setup.allows_enrollment());
    }

    #[test]
    fn test_hash_algorithm() {
        assert_eq!(HashAlgorithm::Sha256.digest_size(), 32);
        assert_eq!(HashAlgorithm::Sha512.digest_size(), 64);
        assert!(HashAlgorithm::Sha256.is_secure());
        assert!(!HashAlgorithm::Sha1.is_secure());
    }

    #[test]
    fn test_verification_result() {
        assert!(VerificationResult::Success.is_success());
        assert!(VerificationResult::AllowedByDb.is_success());
        assert!(!VerificationResult::DeniedByDbx.is_success());
    }

    #[test]
    fn test_authenticode_digest() {
        let mut digest = AuthenticodeDigest::new(HashAlgorithm::Sha256);
        let value = [0x12u8; 32];
        assert!(digest.set(&value));
        assert!(digest.matches(&value));
        assert!(!digest.matches(&[0x00u8; 32]));
    }

    #[test]
    fn test_security_policy() {
        let strict = SecurityPolicy::strict();
        assert!(!strict.allow_unsigned_boot);
        assert!(strict.require_tpm);

        let permissive = SecurityPolicy::permissive();
        assert!(permissive.allow_unsigned_boot);
        assert!(!permissive.require_tpm);
    }

    #[test]
    fn test_pcr_index() {
        let pcr = PcrIndex::Pcr7;
        assert_eq!(pcr.description(), "SecureBoot Policy");
    }
}
