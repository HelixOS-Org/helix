//! Security Protocol
//!
//! High-level Secure Boot and security abstraction.

extern crate alloc;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use super::Protocol;
use crate::error::{Error, Result};
use crate::raw::types::*;

// =============================================================================
// SECURE BOOT
// =============================================================================

/// Secure Boot protocol for managing secure boot state
pub struct SecureBoot {
    /// Handle
    handle: Handle,
    /// Secure boot enabled
    enabled: bool,
    /// Setup mode
    setup_mode: bool,
    /// Audit mode
    audit_mode: bool,
    /// Deployed mode
    deployed_mode: bool,
}

impl SecureBoot {
    /// Create new Secure Boot accessor
    pub fn new(handle: Handle) -> Self {
        Self {
            handle,
            enabled: false,
            setup_mode: false,
            audit_mode: false,
            deployed_mode: false,
        }
    }

    /// Check if Secure Boot is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Check if in Setup Mode
    pub fn is_setup_mode(&self) -> bool {
        self.setup_mode
    }

    /// Check if in Audit Mode
    pub fn is_audit_mode(&self) -> bool {
        self.audit_mode
    }

    /// Check if in Deployed Mode
    pub fn is_deployed_mode(&self) -> bool {
        self.deployed_mode
    }

    /// Get current secure boot mode
    pub fn mode(&self) -> SecureBootMode {
        if !self.enabled {
            SecureBootMode::Disabled
        } else if self.setup_mode {
            SecureBootMode::Setup
        } else if self.audit_mode {
            SecureBootMode::Audit
        } else if self.deployed_mode {
            SecureBootMode::Deployed
        } else {
            SecureBootMode::User
        }
    }

    /// Initialize from UEFI variables
    pub fn init(&mut self) -> Result<()> {
        // Read SecureBoot variable
        // TODO: Implement actual variable read
        Ok(())
    }

    /// Get Platform Key (PK)
    pub fn platform_key(&self) -> Result<Option<SignatureList>> {
        // TODO: Implement actual PK read
        Ok(None)
    }

    /// Get Key Exchange Keys (KEK)
    pub fn key_exchange_keys(&self) -> Result<Vec<SignatureList>> {
        // TODO: Implement actual KEK read
        Ok(Vec::new())
    }

    /// Get authorized signature database (db)
    pub fn authorized_db(&self) -> Result<Vec<SignatureList>> {
        // TODO: Implement actual db read
        Ok(Vec::new())
    }

    /// Get forbidden signature database (dbx)
    pub fn forbidden_dbx(&self) -> Result<Vec<SignatureList>> {
        // TODO: Implement actual dbx read
        Ok(Vec::new())
    }

    /// Get timestamp signature database (dbt)
    pub fn timestamp_dbt(&self) -> Result<Vec<SignatureList>> {
        // TODO: Implement actual dbt read
        Ok(Vec::new())
    }

    /// Get recovery signature database (dbr)
    pub fn recovery_dbr(&self) -> Result<Vec<SignatureList>> {
        // TODO: Implement actual dbr read
        Ok(Vec::new())
    }

    /// Verify image signature
    pub fn verify_image(&self, _image_data: &[u8]) -> Result<VerificationResult> {
        if !self.enabled {
            return Ok(VerificationResult::NotEnforced);
        }

        // TODO: Implement actual signature verification
        Ok(VerificationResult::NotSigned)
    }

    /// Enroll Platform Key
    pub fn enroll_pk(&mut self, _key: &SignatureList) -> Result<()> {
        if !self.setup_mode {
            return Err(Error::AccessDenied);
        }

        // TODO: Implement actual PK enrollment
        Ok(())
    }

    /// Enroll Key Exchange Key
    pub fn enroll_kek(&mut self, _key: &SignatureList) -> Result<()> {
        if !self.setup_mode && !self.audit_mode {
            return Err(Error::AccessDenied);
        }

        // TODO: Implement actual KEK enrollment
        Ok(())
    }

    /// Add authorized signature
    pub fn add_authorized(&mut self, _signature: &SignatureList) -> Result<()> {
        // TODO: Implement actual db update
        Ok(())
    }

    /// Add forbidden signature
    pub fn add_forbidden(&mut self, _signature: &SignatureList) -> Result<()> {
        // TODO: Implement actual dbx update
        Ok(())
    }

    /// Clear all keys (enter Setup Mode)
    pub fn clear_keys(&mut self) -> Result<()> {
        // TODO: Implement actual key clearing
        self.setup_mode = true;
        Ok(())
    }
}

impl Protocol for SecureBoot {
    const GUID: Guid = Guid::new(0xD719B2CB, 0x3D3A, 0x4596, [
        0xA3, 0xBC, 0xDA, 0xD0, 0x0E, 0x67, 0x65, 0x6F,
    ]);

    fn open(handle: Handle) -> Result<Self> {
        Ok(Self::new(handle))
    }
}

// =============================================================================
// SECURE BOOT MODE
// =============================================================================

/// Secure Boot mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecureBootMode {
    /// Secure Boot is disabled
    Disabled,
    /// Setup mode - keys can be enrolled
    Setup,
    /// User mode - normal operation
    User,
    /// Audit mode - signatures checked but not enforced
    Audit,
    /// Deployed mode - most restrictive
    Deployed,
}

impl SecureBootMode {
    /// Get mode name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Disabled => "Disabled",
            Self::Setup => "Setup Mode",
            Self::User => "User Mode",
            Self::Audit => "Audit Mode",
            Self::Deployed => "Deployed Mode",
        }
    }

    /// Check if keys can be modified
    pub fn can_modify_keys(&self) -> bool {
        matches!(self, Self::Setup | Self::Audit)
    }

    /// Check if signature verification is enforced
    pub fn enforces_signatures(&self) -> bool {
        matches!(self, Self::User | Self::Deployed)
    }
}

// =============================================================================
// VERIFICATION RESULT
// =============================================================================

/// Image verification result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationResult {
    /// Image signature is valid and trusted
    Valid,
    /// Image signature is valid but in forbidden list
    Forbidden,
    /// Image has invalid signature
    Invalid,
    /// Image is not signed
    NotSigned,
    /// Signature verification not enforced
    NotEnforced,
}

impl VerificationResult {
    /// Check if image can be loaded
    pub fn can_load(&self) -> bool {
        matches!(self, Self::Valid | Self::NotEnforced)
    }

    /// Get result description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Valid => "Valid signature from trusted authority",
            Self::Forbidden => "Valid signature but in forbidden list",
            Self::Invalid => "Invalid or corrupted signature",
            Self::NotSigned => "Image is not signed",
            Self::NotEnforced => "Signature verification not enforced",
        }
    }
}

// =============================================================================
// SIGNATURE LIST
// =============================================================================

/// Signature list (EFI_SIGNATURE_LIST)
#[derive(Debug, Clone)]
pub struct SignatureList {
    /// Signature type GUID
    pub signature_type: Guid,
    /// Signatures
    pub signatures: Vec<SignatureData>,
}

impl SignatureList {
    /// Create empty signature list
    pub fn new(signature_type: Guid) -> Self {
        Self {
            signature_type,
            signatures: Vec::new(),
        }
    }

    /// Create X.509 certificate list
    pub fn x509() -> Self {
        Self::new(signature_types::EFI_CERT_X509_GUID)
    }

    /// Create SHA-256 hash list
    pub fn sha256() -> Self {
        Self::new(signature_types::EFI_CERT_SHA256_GUID)
    }

    /// Add signature
    pub fn add(&mut self, data: SignatureData) {
        self.signatures.push(data);
    }

    /// Get signature count
    pub fn count(&self) -> usize {
        self.signatures.len()
    }

    /// Check if list is for X.509 certificates
    pub fn is_x509(&self) -> bool {
        self.signature_type == signature_types::EFI_CERT_X509_GUID
    }

    /// Check if list is for SHA-256 hashes
    pub fn is_sha256(&self) -> bool {
        self.signature_type == signature_types::EFI_CERT_SHA256_GUID
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Calculate sizes
        let header_size = 28u32; // GUID + list size + header size + signature size
        let signature_size = if self.signatures.is_empty() {
            0u32
        } else {
            16 + self.signatures[0].data.len() as u32 // Owner GUID + data
        };
        let list_size = header_size + (signature_size * self.signatures.len() as u32);

        // Write header
        bytes.extend_from_slice(&self.signature_type.to_bytes());
        bytes.extend_from_slice(&list_size.to_le_bytes());
        bytes.extend_from_slice(&header_size.to_le_bytes());
        bytes.extend_from_slice(&signature_size.to_le_bytes());

        // Write signatures
        for sig in &self.signatures {
            bytes.extend_from_slice(&sig.owner.to_bytes());
            bytes.extend_from_slice(&sig.data);
        }

        bytes
    }

    /// Parse from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 28 {
            return Err(Error::InvalidParameter);
        }

        let signature_type = Guid::from_bytes(
            data[0..16]
                .try_into()
                .map_err(|_| Error::InvalidParameter)?,
        );
        let list_size = u32::from_le_bytes([data[16], data[17], data[18], data[19]]) as usize;
        let header_size = u32::from_le_bytes([data[20], data[21], data[22], data[23]]) as usize;
        let signature_size = u32::from_le_bytes([data[24], data[25], data[26], data[27]]) as usize;

        if list_size > data.len() {
            return Err(Error::InvalidParameter);
        }

        let mut signatures = Vec::new();
        let mut offset = header_size;

        while offset + signature_size <= list_size {
            let owner = Guid::from_bytes(
                data[offset..offset + 16]
                    .try_into()
                    .map_err(|_| Error::InvalidParameter)?,
            );
            let sig_data = data[offset + 16..offset + signature_size].to_vec();

            signatures.push(SignatureData {
                owner,
                data: sig_data,
            });

            offset += signature_size;
        }

        Ok(Self {
            signature_type,
            signatures,
        })
    }
}

/// Signature data (EFI_SIGNATURE_DATA)
#[derive(Debug, Clone)]
pub struct SignatureData {
    /// Signature owner GUID
    pub owner: Guid,
    /// Signature data
    pub data: Vec<u8>,
}

impl SignatureData {
    /// Create new signature data
    pub fn new(owner: Guid, data: Vec<u8>) -> Self {
        Self { owner, data }
    }

    /// Create from X.509 certificate
    pub fn from_x509(owner: Guid, cert: &[u8]) -> Self {
        Self {
            owner,
            data: cert.to_vec(),
        }
    }

    /// Create from SHA-256 hash
    pub fn from_sha256(owner: Guid, hash: [u8; 32]) -> Self {
        Self {
            owner,
            data: hash.to_vec(),
        }
    }
}

// =============================================================================
// SIGNATURE TYPES
// =============================================================================

/// Signature type GUIDs
pub mod signature_types {
    use super::*;

    /// SHA-256 hash
    pub const EFI_CERT_SHA256_GUID: Guid = Guid::new(0xC1C41626, 0x504C, 0x4092, [
        0xAC, 0xA9, 0x41, 0xF9, 0x36, 0x93, 0x43, 0x28,
    ]);

    /// SHA-384 hash
    pub const EFI_CERT_SHA384_GUID: Guid = Guid::new(0xFF3E5307, 0x9FD0, 0x48C9, [
        0x85, 0xF1, 0x8A, 0xD5, 0x6C, 0x70, 0x1E, 0x01,
    ]);

    /// SHA-512 hash
    pub const EFI_CERT_SHA512_GUID: Guid = Guid::new(0x093E0FAE, 0xA6C4, 0x4F50, [
        0x9F, 0x1B, 0xD4, 0x1E, 0x2B, 0x89, 0xC1, 0x9A,
    ]);

    /// RSA-2048 key
    pub const EFI_CERT_RSA2048_GUID: Guid = Guid::new(0x3C5766E8, 0x269C, 0x4E34, [
        0xAA, 0x14, 0xED, 0x77, 0x6E, 0x85, 0xB3, 0xB6,
    ]);

    /// RSA-2048 + SHA-256 signature
    pub const EFI_CERT_RSA2048_SHA256_GUID: Guid = Guid::new(0xE2B36190, 0x879B, 0x4A3D, [
        0xAD, 0x8D, 0xF2, 0xE7, 0xBB, 0xA3, 0x27, 0x84,
    ]);

    /// SHA-1 hash
    pub const EFI_CERT_SHA1_GUID: Guid = Guid::new(0x826CA512, 0xCF10, 0x4AC9, [
        0xB1, 0x87, 0xBE, 0x01, 0x49, 0x66, 0x31, 0xBD,
    ]);

    /// RSA-2048 + SHA-1 signature
    pub const EFI_CERT_RSA2048_SHA1_GUID: Guid = Guid::new(0x67F8444F, 0x8743, 0x48F1, [
        0xA3, 0x28, 0x1E, 0xAA, 0xB8, 0x73, 0x60, 0x80,
    ]);

    /// X.509 certificate
    pub const EFI_CERT_X509_GUID: Guid = Guid::new(0xA5C059A1, 0x94E4, 0x4AA7, [
        0x87, 0xB5, 0xAB, 0x15, 0x5C, 0x2B, 0xF0, 0x72,
    ]);

    /// X.509 + SHA-256 signature
    pub const EFI_CERT_X509_SHA256_GUID: Guid = Guid::new(0x3BD2A492, 0x96C0, 0x4079, [
        0xB4, 0x20, 0xFC, 0xF9, 0x8E, 0xF1, 0x03, 0xED,
    ]);

    /// X.509 + SHA-384 signature
    pub const EFI_CERT_X509_SHA384_GUID: Guid = Guid::new(0x7076876E, 0x80C2, 0x4EE6, [
        0xAA, 0xD2, 0x28, 0xB3, 0x49, 0xA6, 0x86, 0x5B,
    ]);

    /// X.509 + SHA-512 signature
    pub const EFI_CERT_X509_SHA512_GUID: Guid = Guid::new(0x446DBF63, 0x2502, 0x4CDA, [
        0xBC, 0xFA, 0x24, 0x65, 0xD2, 0xB0, 0xFE, 0x9D,
    ]);

    /// PKCS#7 signature
    pub const EFI_CERT_PKCS7_GUID: Guid = Guid::new(0x4AAFD29D, 0x68DF, 0x49EE, [
        0x8A, 0xA9, 0x34, 0x7D, 0x37, 0x56, 0x65, 0xA7,
    ]);
}

// =============================================================================
// MEASURED BOOT (TPM)
// =============================================================================

/// Measured Boot protocol for TPM interactions
pub struct MeasuredBoot {
    /// Handle
    handle: Handle,
    /// TPM version
    tpm_version: TpmVersion,
    /// TPM present
    tpm_present: bool,
    /// Measurements available
    measurements: Vec<Measurement>,
}

impl MeasuredBoot {
    /// Create new Measured Boot accessor
    pub fn new(handle: Handle) -> Self {
        Self {
            handle,
            tpm_version: TpmVersion::Unknown,
            tpm_present: false,
            measurements: Vec::new(),
        }
    }

    /// Check if TPM is present
    pub fn is_tpm_present(&self) -> bool {
        self.tpm_present
    }

    /// Get TPM version
    pub fn tpm_version(&self) -> TpmVersion {
        self.tpm_version
    }

    /// Get all measurements
    pub fn measurements(&self) -> &[Measurement] {
        &self.measurements
    }

    /// Get measurements for specific PCR
    pub fn pcr_measurements(&self, pcr: u8) -> Vec<&Measurement> {
        self.measurements
            .iter()
            .filter(|m| m.pcr_index == pcr)
            .collect()
    }

    /// Extend PCR with measurement
    pub fn extend_pcr(
        &mut self,
        pcr: u8,
        hash: &[u8],
        event_type: EventType,
        event_data: Vec<u8>,
    ) -> Result<()> {
        if !self.tpm_present {
            return Err(Error::NotReady);
        }

        if pcr > 23 {
            return Err(Error::InvalidParameter);
        }

        // TODO: Implement actual PCR extension

        self.measurements.push(Measurement {
            pcr_index: pcr,
            event_type,
            digest: hash.to_vec(),
            event_data,
        });

        Ok(())
    }

    /// Read PCR value
    pub fn read_pcr(&self, _pcr: u8) -> Result<Vec<u8>> {
        if !self.tpm_present {
            return Err(Error::NotReady);
        }

        // TODO: Implement actual PCR read
        Ok(vec![0u8; 32])
    }

    /// Get event log
    pub fn event_log(&self) -> Result<EventLog> {
        if !self.tpm_present {
            return Err(Error::NotReady);
        }

        Ok(EventLog {
            version: self.tpm_version,
            events: self.measurements.clone(),
        })
    }
}

impl Protocol for MeasuredBoot {
    const GUID: Guid = Guid::new(0x1B31ED20, 0x6DD1, 0x4B5A, [
        0x8A, 0x2A, 0xD9, 0x94, 0x03, 0x64, 0x2F, 0xC9,
    ]);

    fn open(handle: Handle) -> Result<Self> {
        Ok(Self::new(handle))
    }
}

// =============================================================================
// TPM VERSION
// =============================================================================

/// TPM version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpmVersion {
    /// Unknown
    Unknown,
    /// TPM 1.2
    Tpm12,
    /// TPM 2.0
    Tpm20,
}

impl TpmVersion {
    /// Get version name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::Tpm12 => "TPM 1.2",
            Self::Tpm20 => "TPM 2.0",
        }
    }

    /// Get digest size
    pub fn digest_size(&self) -> usize {
        match self {
            Self::Unknown => 0,
            Self::Tpm12 => 20, // SHA-1
            Self::Tpm20 => 32, // SHA-256
        }
    }
}

// =============================================================================
// MEASUREMENT
// =============================================================================

/// TPM measurement
#[derive(Debug, Clone)]
pub struct Measurement {
    /// PCR index
    pub pcr_index: u8,
    /// Event type
    pub event_type: EventType,
    /// Digest
    pub digest: Vec<u8>,
    /// Event data
    pub event_data: Vec<u8>,
}

impl Measurement {
    /// Get PCR name
    pub fn pcr_name(&self) -> &'static str {
        match self.pcr_index {
            0 => "PCR0: BIOS/UEFI",
            1 => "PCR1: Configuration",
            2 => "PCR2: Option ROMs",
            3 => "PCR3: Option ROM Config",
            4 => "PCR4: MBR/GPT",
            5 => "PCR5: MBR/GPT Config",
            6 => "PCR6: State Transitions",
            7 => "PCR7: Secure Boot",
            8 => "PCR8: OS Loader",
            9 => "PCR9: OS Loader",
            10 => "PCR10: Kernel",
            11 => "PCR11: Kernel",
            12 => "PCR12: User",
            13 => "PCR13: User",
            14 => "PCR14: Boot Authority",
            15 => "PCR15: User",
            16 => "PCR16: Debug",
            17 => "PCR17: DRTM",
            18 => "PCR18: DRTM",
            19 => "PCR19: DRTM",
            20 => "PCR20: DRTM",
            21 => "PCR21: DRTM",
            22 => "PCR22: DRTM",
            23 => "PCR23: Application",
            _ => "Unknown PCR",
        }
    }

    /// Get digest as hex string
    pub fn digest_hex(&self) -> String {
        self.digest
            .iter()
            .map(|b| alloc::format!("{:02x}", b))
            .collect()
    }
}

// =============================================================================
// EVENT TYPE
// =============================================================================

/// TCG event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    /// Pre-boot environment
    PrebootCert,
    /// POST code
    PostCode,
    /// Unused
    Unused,
    /// No action
    NoAction,
    /// Separator
    Separator,
    /// Action
    Action,
    /// Event tag
    EventTag,
    /// S-CRTM Contents
    ScrtmContents,
    /// S-CRTM Version
    ScrtmVersion,
    /// CPU Microcode
    CpuMicrocode,
    /// Platform Config Flags
    PlatformConfigFlags,
    /// Table of Devices
    TableOfDevices,
    /// Compact Hash
    CompactHash,
    /// IPL (Initial Program Load)
    Ipl,
    /// IPL Partition Data
    IplPartitionData,
    /// Non-Host Code
    NonHostCode,
    /// Non-Host Config
    NonHostConfig,
    /// Non-Host Info
    NonHostInfo,
    /// Omit Boot Device Events
    OmitBootDeviceEvents,
    /// EFI Event Base
    EfiEventBase,
    /// EFI Variable Driver Config
    EfiVariableDriverConfig,
    /// EFI Variable Boot
    EfiVariableBoot,
    /// EFI Boot Services Application
    EfiBootServicesApplication,
    /// EFI Boot Services Driver
    EfiBootServicesDriver,
    /// EFI Runtime Services Driver
    EfiRuntimeServicesDriver,
    /// EFI GPT Event
    EfiGptEvent,
    /// EFI Action
    EfiAction,
    /// EFI Platform Firmware Blob
    EfiPlatformFirmwareBlob,
    /// EFI Handoff Tables
    EfiHandoffTables,
    /// EFI Hcrtm Event
    EfiHcrtmEvent,
    /// EFI Variable Authority
    EfiVariableAuthority,
    /// Unknown event type
    Unknown(u32),
}

impl EventType {
    /// Create from raw value
    pub fn from_u32(value: u32) -> Self {
        match value {
            0x00000000 => Self::PrebootCert,
            0x00000001 => Self::PostCode,
            0x00000002 => Self::Unused,
            0x00000003 => Self::NoAction,
            0x00000004 => Self::Separator,
            0x00000005 => Self::Action,
            0x00000006 => Self::EventTag,
            0x00000007 => Self::ScrtmContents,
            0x00000008 => Self::ScrtmVersion,
            0x00000009 => Self::CpuMicrocode,
            0x0000000A => Self::PlatformConfigFlags,
            0x0000000B => Self::TableOfDevices,
            0x0000000C => Self::CompactHash,
            0x0000000D => Self::Ipl,
            0x0000000E => Self::IplPartitionData,
            0x0000000F => Self::NonHostCode,
            0x00000010 => Self::NonHostConfig,
            0x00000011 => Self::NonHostInfo,
            0x00000012 => Self::OmitBootDeviceEvents,
            0x80000000 => Self::EfiEventBase,
            0x80000001 => Self::EfiVariableDriverConfig,
            0x80000002 => Self::EfiVariableBoot,
            0x80000003 => Self::EfiBootServicesApplication,
            0x80000004 => Self::EfiBootServicesDriver,
            0x80000005 => Self::EfiRuntimeServicesDriver,
            0x80000006 => Self::EfiGptEvent,
            0x80000007 => Self::EfiAction,
            0x80000008 => Self::EfiPlatformFirmwareBlob,
            0x80000009 => Self::EfiHandoffTables,
            0x80000010 => Self::EfiHcrtmEvent,
            0x800000E0 => Self::EfiVariableAuthority,
            other => Self::Unknown(other),
        }
    }

    /// Get event type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::PrebootCert => "PREBOOT_CERT",
            Self::PostCode => "POST_CODE",
            Self::Unused => "UNUSED",
            Self::NoAction => "NO_ACTION",
            Self::Separator => "SEPARATOR",
            Self::Action => "ACTION",
            Self::EventTag => "EVENT_TAG",
            Self::ScrtmContents => "S-CRTM_CONTENTS",
            Self::ScrtmVersion => "S-CRTM_VERSION",
            Self::CpuMicrocode => "CPU_MICROCODE",
            Self::PlatformConfigFlags => "PLATFORM_CONFIG_FLAGS",
            Self::TableOfDevices => "TABLE_OF_DEVICES",
            Self::CompactHash => "COMPACT_HASH",
            Self::Ipl => "IPL",
            Self::IplPartitionData => "IPL_PARTITION_DATA",
            Self::NonHostCode => "NON_HOST_CODE",
            Self::NonHostConfig => "NON_HOST_CONFIG",
            Self::NonHostInfo => "NON_HOST_INFO",
            Self::OmitBootDeviceEvents => "OMIT_BOOT_DEVICE_EVENTS",
            Self::EfiEventBase => "EFI_EVENT_BASE",
            Self::EfiVariableDriverConfig => "EFI_VARIABLE_DRIVER_CONFIG",
            Self::EfiVariableBoot => "EFI_VARIABLE_BOOT",
            Self::EfiBootServicesApplication => "EFI_BOOT_SERVICES_APPLICATION",
            Self::EfiBootServicesDriver => "EFI_BOOT_SERVICES_DRIVER",
            Self::EfiRuntimeServicesDriver => "EFI_RUNTIME_SERVICES_DRIVER",
            Self::EfiGptEvent => "EFI_GPT_EVENT",
            Self::EfiAction => "EFI_ACTION",
            Self::EfiPlatformFirmwareBlob => "EFI_PLATFORM_FIRMWARE_BLOB",
            Self::EfiHandoffTables => "EFI_HANDOFF_TABLES",
            Self::EfiHcrtmEvent => "EFI_HCRTM_EVENT",
            Self::EfiVariableAuthority => "EFI_VARIABLE_AUTHORITY",
            Self::Unknown(_) => "UNKNOWN",
        }
    }
}

// =============================================================================
// EVENT LOG
// =============================================================================

/// TPM event log
#[derive(Debug, Clone)]
pub struct EventLog {
    /// TPM version
    pub version: TpmVersion,
    /// Events
    pub events: Vec<Measurement>,
}

impl EventLog {
    /// Get event count
    pub fn count(&self) -> usize {
        self.events.len()
    }

    /// Get events by PCR
    pub fn by_pcr(&self, pcr: u8) -> Vec<&Measurement> {
        self.events.iter().filter(|e| e.pcr_index == pcr).collect()
    }

    /// Calculate final PCR value (by extending all measurements)
    pub fn calculate_pcr(&self, pcr: u8) -> Vec<u8> {
        let digest_size = self.version.digest_size();
        let mut current = vec![0u8; digest_size];

        for event in self.by_pcr(pcr) {
            // Extend: new_pcr = hash(old_pcr || measurement)
            // TODO: Implement actual hash extension
            for (i, b) in event.digest.iter().enumerate() {
                if i < current.len() {
                    current[i] ^= b;
                }
            }
        }

        current
    }
}

// =============================================================================
// SECURITY GUIDS
// =============================================================================

/// Security-related GUIDs
pub mod security_guids {
    use super::*;

    /// Secure Boot variable vendor GUID
    pub const EFI_GLOBAL_VARIABLE: Guid = Guid::new(0x8BE4DF61, 0x93CA, 0x11D2, [
        0xAA, 0x0D, 0x00, 0xE0, 0x98, 0x03, 0x2B, 0x8C,
    ]);

    /// Image security database vendor GUID
    pub const EFI_IMAGE_SECURITY_DATABASE: Guid = Guid::new(0xD719B2CB, 0x3D3A, 0x4596, [
        0xA3, 0xBC, 0xDA, 0xD0, 0x0E, 0x67, 0x65, 0x6F,
    ]);

    /// TCG EFI spec vendor GUID
    pub const EFI_TCG_VENDOR: Guid = Guid::new(0x6E7AD8F0, 0x95C8, 0x4B49, [
        0x9D, 0xB4, 0x62, 0x53, 0x23, 0x3E, 0x18, 0x90,
    ]);
}

/// Security variable names
pub mod security_vars {
    /// Platform Key
    pub const PK: &str = "PK";
    /// Key Exchange Key
    pub const KEK: &str = "KEK";
    /// Authorized signature database
    pub const DB: &str = "db";
    /// Forbidden signature database
    pub const DBX: &str = "dbx";
    /// Timestamp signature database
    pub const DBT: &str = "dbt";
    /// Recovery signature database
    pub const DBR: &str = "dbr";
    /// Secure Boot enable
    pub const SECURE_BOOT: &str = "SecureBoot";
    /// Setup mode
    pub const SETUP_MODE: &str = "SetupMode";
    /// Audit mode
    pub const AUDIT_MODE: &str = "AuditMode";
    /// Deployed mode
    pub const DEPLOYED_MODE: &str = "DeployedMode";
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_secure_boot_mode() {
        assert!(SecureBootMode::Setup.can_modify_keys());
        assert!(!SecureBootMode::User.can_modify_keys());
        assert!(SecureBootMode::User.enforces_signatures());
        assert!(!SecureBootMode::Audit.enforces_signatures());
    }

    #[test]
    fn test_verification_result() {
        assert!(VerificationResult::Valid.can_load());
        assert!(VerificationResult::NotEnforced.can_load());
        assert!(!VerificationResult::Forbidden.can_load());
        assert!(!VerificationResult::Invalid.can_load());
    }

    #[test]
    fn test_signature_list() {
        let mut list = SignatureList::sha256();
        assert!(list.is_sha256());
        assert_eq!(list.count(), 0);

        list.add(SignatureData::from_sha256(Guid::ZERO, [0u8; 32]));
        assert_eq!(list.count(), 1);
    }

    #[test]
    fn test_event_type() {
        assert_eq!(EventType::from_u32(0x00000004), EventType::Separator);
        assert_eq!(
            EventType::from_u32(0x80000003),
            EventType::EfiBootServicesApplication
        );
        assert_eq!(EventType::Separator.name(), "SEPARATOR");
    }

    #[test]
    fn test_tpm_version() {
        assert_eq!(TpmVersion::Tpm12.digest_size(), 20);
        assert_eq!(TpmVersion::Tpm20.digest_size(), 32);
    }
}
