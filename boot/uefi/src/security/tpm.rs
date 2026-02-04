//! TPM 2.0 Protocol Interface
//!
//! Measured boot with TPM 2.0, PCR extension, and event logging.

extern crate alloc;
use alloc::vec;
use alloc::vec::Vec;

use super::hash::{Sha256, Sha512};

// =============================================================================
// TPM2 GUIDS
// =============================================================================

/// TPM2 protocol GUIDs
pub mod guid {
    /// EFI_TCG2_PROTOCOL GUID
    pub const EFI_TCG2_PROTOCOL: [u8; 16] = [
        0x8f, 0x9a, 0xf6, 0x60, 0xc5, 0x25, 0x14, 0x45, 0x93, 0x27, 0xf5, 0x5e, 0xe4, 0x2c, 0x4e,
        0xf3,
    ];

    /// EFI_TCG2_FINAL_EVENTS_TABLE GUID
    pub const EFI_TCG2_FINAL_EVENTS_TABLE: [u8; 16] = [
        0xb9, 0x83, 0x82, 0x77, 0xb3, 0x1f, 0xd0, 0x4b, 0xae, 0x83, 0xb6, 0x31, 0x47, 0xfa, 0x8e,
        0x2f,
    ];
}

// =============================================================================
// TPM2 CONSTANTS
// =============================================================================

/// TPM2 algorithms
pub mod algorithm {
    pub const TPM_ALG_SHA1: u16 = 0x0004;
    pub const TPM_ALG_SHA256: u16 = 0x000B;
    pub const TPM_ALG_SHA384: u16 = 0x000C;
    pub const TPM_ALG_SHA512: u16 = 0x000D;
    pub const TPM_ALG_SM3_256: u16 = 0x0012;
    pub const TPM_ALG_SHA3_256: u16 = 0x0027;
    pub const TPM_ALG_SHA3_384: u16 = 0x0028;
    pub const TPM_ALG_SHA3_512: u16 = 0x0029;
}

/// TPM2 command codes
pub mod command {
    pub const TPM_CC_STARTUP: u32 = 0x00000144;
    pub const TPM_CC_SHUTDOWN: u32 = 0x00000145;
    pub const TPM_CC_SELF_TEST: u32 = 0x00000143;
    pub const TPM_CC_PCR_EXTEND: u32 = 0x00000182;
    pub const TPM_CC_PCR_READ: u32 = 0x0000017E;
    pub const TPM_CC_PCR_RESET: u32 = 0x0000013D;
    pub const TPM_CC_GET_CAPABILITY: u32 = 0x0000017A;
    pub const TPM_CC_GET_RANDOM: u32 = 0x0000017B;
    pub const TPM_CC_HASH: u32 = 0x0000017D;
    pub const TPM_CC_CREATE: u32 = 0x00000153;
    pub const TPM_CC_LOAD: u32 = 0x00000157;
    pub const TPM_CC_UNSEAL: u32 = 0x0000015E;
    pub const TPM_CC_QUOTE: u32 = 0x00000158;
    pub const TPM_CC_NV_READ: u32 = 0x0000014E;
    pub const TPM_CC_NV_WRITE: u32 = 0x00000137;
}

/// TPM2 response codes
pub mod response {
    pub const TPM_RC_SUCCESS: u32 = 0x00000000;
    pub const TPM_RC_FAILURE: u32 = 0x00000101;
    pub const TPM_RC_LOCALITY: u32 = 0x0000007F;
    pub const TPM_RC_RETRY: u32 = 0x00000922;
    pub const TPM_RC_NV_UNAVAILABLE: u32 = 0x00000923;
}

/// TPM2 startup types
pub mod startup_type {
    pub const TPM_SU_CLEAR: u16 = 0x0000;
    pub const TPM_SU_STATE: u16 = 0x0001;
}

/// TPM2 capabilities
pub mod capability {
    pub const TPM_CAP_TPM_PROPERTIES: u32 = 0x00000006;
    pub const TPM_CAP_PCRS: u32 = 0x00000005;
    pub const TPM_CAP_ALGS: u32 = 0x00000000;
}

/// TPM2 properties
pub mod property {
    pub const TPM_PT_FAMILY_INDICATOR: u32 = 0x00000100;
    pub const TPM_PT_LEVEL: u32 = 0x00000101;
    pub const TPM_PT_REVISION: u32 = 0x00000102;
    pub const TPM_PT_MANUFACTURER: u32 = 0x00000105;
    pub const TPM_PT_FIRMWARE_VERSION_1: u32 = 0x0000010B;
    pub const TPM_PT_FIRMWARE_VERSION_2: u32 = 0x0000010C;
    pub const TPM_PT_PCR_COUNT: u32 = 0x00000112;
    pub const TPM_PT_MAX_DIGEST: u32 = 0x00000119;
}

/// PCR indices
pub mod pcr {
    /// SRTM (Static Root of Trust for Measurement)
    pub const PCR0_SRTM: u32 = 0;
    /// Host Platform Configuration
    pub const PCR1_HOST_CONFIG: u32 = 1;
    /// UEFI driver and application code
    pub const PCR2_UEFI_DRIVER: u32 = 2;
    /// UEFI driver and application configuration and data
    pub const PCR3_UEFI_CONFIG: u32 = 3;
    /// UEFI Boot Manager Code and Boot Attempts
    pub const PCR4_BOOT_MANAGER: u32 = 4;
    /// Boot Manager Code Configuration and Data
    pub const PCR5_BOOT_MANAGER_CONFIG: u32 = 5;
    /// Host Platform Manufacturer Specific
    pub const PCR6_OEM: u32 = 6;
    /// Secure Boot Policy
    pub const PCR7_SECURE_BOOT: u32 = 7;
    /// DRTM (Dynamic Root of Trust for Measurement)
    pub const PCR17_DRTM: u32 = 17;
    /// Kernel and initrd
    pub const PCR8_KERNEL: u32 = 8;
    /// Command line
    pub const PCR9_CMDLINE: u32 = 9;
}

// =============================================================================
// TCG2 EVENT TYPES
// =============================================================================

/// TCG2 event types
pub mod event_type {
    pub const EV_PREBOOT_CERT: u32 = 0x00000000;
    pub const EV_POST_CODE: u32 = 0x00000001;
    pub const EV_UNUSED: u32 = 0x00000002;
    pub const EV_NO_ACTION: u32 = 0x00000003;
    pub const EV_SEPARATOR: u32 = 0x00000004;
    pub const EV_ACTION: u32 = 0x00000005;
    pub const EV_EVENT_TAG: u32 = 0x00000006;
    pub const EV_S_CRTM_CONTENTS: u32 = 0x00000007;
    pub const EV_S_CRTM_VERSION: u32 = 0x00000008;
    pub const EV_CPU_MICROCODE: u32 = 0x00000009;
    pub const EV_PLATFORM_CONFIG_FLAGS: u32 = 0x0000000A;
    pub const EV_TABLE_OF_DEVICES: u32 = 0x0000000B;
    pub const EV_COMPACT_HASH: u32 = 0x0000000C;
    pub const EV_IPL: u32 = 0x0000000D;
    pub const EV_IPL_PARTITION_DATA: u32 = 0x0000000E;
    pub const EV_NONHOST_CODE: u32 = 0x0000000F;
    pub const EV_NONHOST_CONFIG: u32 = 0x00000010;
    pub const EV_NONHOST_INFO: u32 = 0x00000011;
    pub const EV_OMIT_BOOT_DEVICE_EVENTS: u32 = 0x00000012;

    // EFI-specific events
    pub const EV_EFI_EVENT_BASE: u32 = 0x80000000;
    pub const EV_EFI_VARIABLE_DRIVER_CONFIG: u32 = 0x80000001;
    pub const EV_EFI_VARIABLE_BOOT: u32 = 0x80000002;
    pub const EV_EFI_BOOT_SERVICES_APPLICATION: u32 = 0x80000003;
    pub const EV_EFI_BOOT_SERVICES_DRIVER: u32 = 0x80000004;
    pub const EV_EFI_RUNTIME_SERVICES_DRIVER: u32 = 0x80000005;
    pub const EV_EFI_GPT_EVENT: u32 = 0x80000006;
    pub const EV_EFI_ACTION: u32 = 0x80000007;
    pub const EV_EFI_PLATFORM_FIRMWARE_BLOB: u32 = 0x80000008;
    pub const EV_EFI_HANDOFF_TABLES: u32 = 0x80000009;
    pub const EV_EFI_PLATFORM_FIRMWARE_BLOB2: u32 = 0x8000000A;
    pub const EV_EFI_HANDOFF_TABLES2: u32 = 0x8000000B;
    pub const EV_EFI_VARIABLE_AUTHORITY: u32 = 0x800000E0;
    pub const EV_EFI_SPDM_FIRMWARE_BLOB: u32 = 0x800000E1;
    pub const EV_EFI_SPDM_FIRMWARE_CONFIG: u32 = 0x800000E2;
}

// =============================================================================
// TPM2 TYPES
// =============================================================================

/// TPM2B structure (size + data)
#[derive(Debug, Clone)]
pub struct Tpm2bDigest {
    pub size: u16,
    pub buffer: Vec<u8>,
}

impl Tpm2bDigest {
    pub fn new(data: &[u8]) -> Self {
        Self {
            size: data.len() as u16,
            buffer: data.to_vec(),
        }
    }

    pub fn empty() -> Self {
        Self {
            size: 0,
            buffer: Vec::new(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();
        result.extend_from_slice(&self.size.to_be_bytes());
        result.extend_from_slice(&self.buffer);
        result
    }

    pub fn from_bytes(data: &[u8]) -> Option<(Self, usize)> {
        if data.len() < 2 {
            return None;
        }
        let size = u16::from_be_bytes([data[0], data[1]]) as usize;
        if data.len() < 2 + size {
            return None;
        }
        Some((
            Self {
                size: size as u16,
                buffer: data[2..2 + size].to_vec(),
            },
            2 + size,
        ))
    }
}

/// TPML_DIGEST_VALUES
#[derive(Debug, Clone)]
pub struct TpmlDigestValues {
    pub count: u32,
    pub digests: Vec<TpmtHa>,
}

impl TpmlDigestValues {
    pub fn new() -> Self {
        Self {
            count: 0,
            digests: Vec::new(),
        }
    }

    pub fn add(&mut self, alg: u16, digest: &[u8]) {
        self.digests.push(TpmtHa {
            hash_alg: alg,
            digest: digest.to_vec(),
        });
        self.count = self.digests.len() as u32;
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();
        result.extend_from_slice(&self.count.to_be_bytes());
        for d in &self.digests {
            result.extend_from_slice(&d.to_bytes());
        }
        result
    }
}

impl Default for TpmlDigestValues {
    fn default() -> Self {
        Self::new()
    }
}

/// TPMT_HA
#[derive(Debug, Clone)]
pub struct TpmtHa {
    pub hash_alg: u16,
    pub digest: Vec<u8>,
}

impl TpmtHa {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();
        result.extend_from_slice(&self.hash_alg.to_be_bytes());
        result.extend_from_slice(&self.digest);
        result
    }

    pub fn digest_size(alg: u16) -> usize {
        match alg {
            algorithm::TPM_ALG_SHA1 => 20,
            algorithm::TPM_ALG_SHA256 => 32,
            algorithm::TPM_ALG_SHA384 => 48,
            algorithm::TPM_ALG_SHA512 => 64,
            algorithm::TPM_ALG_SM3_256 => 32,
            _ => 0,
        }
    }
}

/// TPMS_PCR_SELECTION
#[derive(Debug, Clone)]
pub struct TpmsPcrSelection {
    pub hash: u16,
    pub size_of_select: u8,
    pub pcr_select: Vec<u8>,
}

impl TpmsPcrSelection {
    pub fn new(hash: u16, pcrs: &[u32]) -> Self {
        let mut pcr_select = vec![0u8; 3]; // Support up to 24 PCRs

        for &pcr in pcrs {
            if pcr < 24 {
                pcr_select[(pcr / 8) as usize] |= 1 << (pcr % 8);
            }
        }

        Self {
            hash,
            size_of_select: 3,
            pcr_select,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();
        result.extend_from_slice(&self.hash.to_be_bytes());
        result.push(self.size_of_select);
        result.extend_from_slice(&self.pcr_select);
        result
    }
}

// =============================================================================
// TCG2 PROTOCOL
// =============================================================================

/// TCG2 boot service capability
#[derive(Debug, Clone)]
pub struct Tcg2BootServiceCapability {
    pub size: u8,
    pub structure_version_major: u8,
    pub structure_version_minor: u8,
    pub protocol_version_major: u8,
    pub protocol_version_minor: u8,
    pub hash_algorithm_bitmap: u32,
    pub supported_event_logs: u32,
    pub tpm_present: bool,
    pub max_command_size: u16,
    pub max_response_size: u16,
    pub manufacturer_id: u32,
    pub number_of_pcr_banks: u32,
    pub active_pcr_banks: u32,
}

impl Tcg2BootServiceCapability {
    /// Parse from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 27 {
            return None;
        }

        Some(Self {
            size: data[0],
            structure_version_major: data[1],
            structure_version_minor: data[2],
            protocol_version_major: data[3],
            protocol_version_minor: data[4],
            hash_algorithm_bitmap: u32::from_le_bytes([data[5], data[6], data[7], data[8]]),
            supported_event_logs: u32::from_le_bytes([data[9], data[10], data[11], data[12]]),
            tpm_present: data[13] != 0,
            max_command_size: u16::from_le_bytes([data[14], data[15]]),
            max_response_size: u16::from_le_bytes([data[16], data[17]]),
            manufacturer_id: u32::from_le_bytes([data[18], data[19], data[20], data[21]]),
            number_of_pcr_banks: u32::from_le_bytes([data[22], data[23], data[24], data[25]]),
            active_pcr_banks: if data.len() > 26 {
                u32::from_le_bytes([
                    data[26],
                    data.get(27).copied().unwrap_or(0),
                    data.get(28).copied().unwrap_or(0),
                    data.get(29).copied().unwrap_or(0),
                ])
            } else {
                0
            },
        })
    }

    /// Check if SHA-256 is supported
    pub fn supports_sha256(&self) -> bool {
        self.hash_algorithm_bitmap & (1 << 1) != 0 // SHA256 is bit 1
    }

    /// Check if SHA-384 is supported
    pub fn supports_sha384(&self) -> bool {
        self.hash_algorithm_bitmap & (1 << 2) != 0
    }

    /// Check if SHA-512 is supported
    pub fn supports_sha512(&self) -> bool {
        self.hash_algorithm_bitmap & (1 << 3) != 0
    }
}

/// TCG2 event header (crypto-agile format)
#[derive(Debug, Clone)]
pub struct TcgPcrEvent2 {
    pub pcr_index: u32,
    pub event_type: u32,
    pub digests: TpmlDigestValues,
    pub event_size: u32,
    pub event: Vec<u8>,
}

impl TcgPcrEvent2 {
    /// Create new event
    pub fn new(pcr_index: u32, event_type: u32, data: &[u8]) -> Self {
        let mut digests = TpmlDigestValues::new();

        // Compute SHA-256 digest
        let sha256 = Sha256::digest(data);
        digests.add(algorithm::TPM_ALG_SHA256, &sha256);

        Self {
            pcr_index,
            event_type,
            digests,
            event_size: data.len() as u32,
            event: data.to_vec(),
        }
    }

    /// Create with multiple hash algorithms
    pub fn new_multi_hash(
        pcr_index: u32,
        event_type: u32,
        data: &[u8],
        algorithms: &[u16],
    ) -> Self {
        let mut digests = TpmlDigestValues::new();

        for &alg in algorithms {
            match alg {
                algorithm::TPM_ALG_SHA256 => {
                    let hash = Sha256::digest(data);
                    digests.add(alg, &hash);
                },
                algorithm::TPM_ALG_SHA384 => {
                    let hash = Sha512::digest_384(data);
                    digests.add(alg, &hash);
                },
                algorithm::TPM_ALG_SHA512 => {
                    let hash = Sha512::digest_512(data);
                    digests.add(alg, &hash);
                },
                _ => {},
            }
        }

        Self {
            pcr_index,
            event_type,
            digests,
            event_size: data.len() as u32,
            event: data.to_vec(),
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();
        result.extend_from_slice(&self.pcr_index.to_le_bytes());
        result.extend_from_slice(&self.event_type.to_le_bytes());
        result.extend_from_slice(&self.digests.to_bytes());
        result.extend_from_slice(&self.event_size.to_le_bytes());
        result.extend_from_slice(&self.event);
        result
    }
}

/// TCG EFI Spec ID Event (event log header)
#[derive(Debug, Clone)]
pub struct TcgEfiSpecIdEvent {
    pub signature: [u8; 16],
    pub platform_class: u32,
    pub spec_version_minor: u8,
    pub spec_version_major: u8,
    pub spec_errata: u8,
    pub uintn_size: u8,
    pub number_of_algorithms: u32,
    pub digest_sizes: Vec<(u16, u16)>, // (algId, digestSize)
    pub vendor_info_size: u8,
    pub vendor_info: Vec<u8>,
}

impl TcgEfiSpecIdEvent {
    /// Create for TCG2 event log
    pub fn new_tcg2(algorithms: &[(u16, u16)]) -> Self {
        Self {
            signature: *b"Spec ID Event03\0",
            platform_class: 0,
            spec_version_minor: 0,
            spec_version_major: 2,
            spec_errata: 0,
            uintn_size: core::mem::size_of::<usize>() as u8,
            number_of_algorithms: algorithms.len() as u32,
            digest_sizes: algorithms.to_vec(),
            vendor_info_size: 0,
            vendor_info: Vec::new(),
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();
        result.extend_from_slice(&self.signature);
        result.extend_from_slice(&self.platform_class.to_le_bytes());
        result.push(self.spec_version_minor);
        result.push(self.spec_version_major);
        result.push(self.spec_errata);
        result.push(self.uintn_size);
        result.extend_from_slice(&self.number_of_algorithms.to_le_bytes());

        for (alg, size) in &self.digest_sizes {
            result.extend_from_slice(&alg.to_le_bytes());
            result.extend_from_slice(&size.to_le_bytes());
        }

        result.push(self.vendor_info_size);
        result.extend_from_slice(&self.vendor_info);

        result
    }
}

// =============================================================================
// TPM COMMAND BUILDER
// =============================================================================

/// TPM2 command builder
pub struct Tpm2Command {
    tag: u16,
    command_code: u32,
    data: Vec<u8>,
}

impl Tpm2Command {
    /// Create new command
    pub fn new(command_code: u32) -> Self {
        Self {
            tag: 0x8001, // TPM_ST_NO_SESSIONS
            command_code,
            data: Vec::new(),
        }
    }

    /// Create command with sessions
    pub fn new_with_sessions(command_code: u32) -> Self {
        Self {
            tag: 0x8002, // TPM_ST_SESSIONS
            command_code,
            data: Vec::new(),
        }
    }

    /// Add u8
    pub fn add_u8(&mut self, value: u8) -> &mut Self {
        self.data.push(value);
        self
    }

    /// Add u16
    pub fn add_u16(&mut self, value: u16) -> &mut Self {
        self.data.extend_from_slice(&value.to_be_bytes());
        self
    }

    /// Add u32
    pub fn add_u32(&mut self, value: u32) -> &mut Self {
        self.data.extend_from_slice(&value.to_be_bytes());
        self
    }

    /// Add bytes
    pub fn add_bytes(&mut self, data: &[u8]) -> &mut Self {
        self.data.extend_from_slice(data);
        self
    }

    /// Add TPM2B
    pub fn add_tpm2b(&mut self, data: &[u8]) -> &mut Self {
        self.add_u16(data.len() as u16);
        self.add_bytes(data);
        self
    }

    /// Build command
    pub fn build(&self) -> Vec<u8> {
        let size = 10 + self.data.len() as u32;

        let mut result = Vec::new();
        result.extend_from_slice(&self.tag.to_be_bytes());
        result.extend_from_slice(&size.to_be_bytes());
        result.extend_from_slice(&self.command_code.to_be_bytes());
        result.extend_from_slice(&self.data);

        result
    }
}

/// TPM2 response parser
pub struct Tpm2Response<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Tpm2Response<'a> {
    /// Parse response
    pub fn new(data: &'a [u8]) -> Result<Self, TpmError> {
        if data.len() < 10 {
            return Err(TpmError::InvalidResponse);
        }

        Ok(Self { data, offset: 0 })
    }

    /// Get tag
    pub fn tag(&self) -> u16 {
        u16::from_be_bytes([self.data[0], self.data[1]])
    }

    /// Get size
    pub fn size(&self) -> u32 {
        u32::from_be_bytes([self.data[2], self.data[3], self.data[4], self.data[5]])
    }

    /// Get response code
    pub fn response_code(&self) -> u32 {
        u32::from_be_bytes([self.data[6], self.data[7], self.data[8], self.data[9]])
    }

    /// Check if successful
    pub fn is_success(&self) -> bool {
        self.response_code() == response::TPM_RC_SUCCESS
    }

    /// Get payload
    pub fn payload(&self) -> &[u8] {
        &self.data[10..]
    }

    /// Read u8
    pub fn read_u8(&mut self) -> Result<u8, TpmError> {
        if self.offset >= self.payload().len() {
            return Err(TpmError::InvalidResponse);
        }
        let value = self.payload()[self.offset];
        self.offset += 1;
        Ok(value)
    }

    /// Read u16
    pub fn read_u16(&mut self) -> Result<u16, TpmError> {
        if self.offset + 2 > self.payload().len() {
            return Err(TpmError::InvalidResponse);
        }
        let value =
            u16::from_be_bytes([self.payload()[self.offset], self.payload()[self.offset + 1]]);
        self.offset += 2;
        Ok(value)
    }

    /// Read u32
    pub fn read_u32(&mut self) -> Result<u32, TpmError> {
        if self.offset + 4 > self.payload().len() {
            return Err(TpmError::InvalidResponse);
        }
        let value = u32::from_be_bytes([
            self.payload()[self.offset],
            self.payload()[self.offset + 1],
            self.payload()[self.offset + 2],
            self.payload()[self.offset + 3],
        ]);
        self.offset += 4;
        Ok(value)
    }

    /// Read bytes
    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], TpmError> {
        if self.offset + len > self.payload().len() {
            return Err(TpmError::InvalidResponse);
        }
        let start = 10 + self.offset;
        self.offset += len;
        Ok(&self.data[start..start + len])
    }

    /// Read TPM2B
    pub fn read_tpm2b(&mut self) -> Result<&'a [u8], TpmError> {
        let size = self.read_u16()? as usize;
        self.read_bytes(size)
    }
}

// =============================================================================
// MEASURED BOOT
// =============================================================================

/// PCR bank state
#[derive(Debug, Clone)]
pub struct PcrBank {
    /// Algorithm
    pub algorithm: u16,
    /// PCR values (indexed by PCR number)
    pub values: Vec<Vec<u8>>,
}

impl PcrBank {
    /// Create new bank
    pub fn new(algorithm: u16, num_pcrs: usize) -> Self {
        let digest_size = TpmtHa::digest_size(algorithm);
        let mut values = Vec::with_capacity(num_pcrs);

        for _ in 0..num_pcrs {
            values.push(vec![0u8; digest_size]);
        }

        Self { algorithm, values }
    }

    /// Extend PCR
    pub fn extend(&mut self, pcr: u32, data: &[u8]) {
        if pcr as usize >= self.values.len() {
            return;
        }

        let current = &self.values[pcr as usize];
        let digest_size = TpmtHa::digest_size(self.algorithm);

        // Compute hash(current || data)
        let mut to_hash = Vec::with_capacity(current.len() + data.len());
        to_hash.extend_from_slice(current);
        to_hash.extend_from_slice(data);

        let new_value = match self.algorithm {
            algorithm::TPM_ALG_SHA256 => Sha256::digest(&to_hash).to_vec(),
            algorithm::TPM_ALG_SHA384 => Sha512::digest_384(&to_hash).to_vec(),
            algorithm::TPM_ALG_SHA512 => Sha512::digest_512(&to_hash).to_vec(),
            _ => return,
        };

        self.values[pcr as usize] = new_value[..digest_size].to_vec();
    }

    /// Get PCR value
    pub fn get(&self, pcr: u32) -> Option<&[u8]> {
        self.values.get(pcr as usize).map(|v| v.as_slice())
    }
}

/// Event log
#[derive(Debug, Clone)]
pub struct EventLog {
    /// Events
    events: Vec<TcgPcrEvent2>,
    /// Algorithms supported
    algorithms: Vec<(u16, u16)>,
}

impl EventLog {
    /// Create new event log
    pub fn new(algorithms: &[(u16, u16)]) -> Self {
        Self {
            events: Vec::new(),
            algorithms: algorithms.to_vec(),
        }
    }

    /// Add event
    pub fn add_event(&mut self, event: TcgPcrEvent2) {
        self.events.push(event);
    }

    /// Get events
    pub fn events(&self) -> &[TcgPcrEvent2] {
        &self.events
    }

    /// Get events for PCR
    pub fn events_for_pcr(&self, pcr: u32) -> Vec<&TcgPcrEvent2> {
        self.events.iter().filter(|e| e.pcr_index == pcr).collect()
    }

    /// Serialize header
    pub fn serialize_header(&self) -> Vec<u8> {
        let spec_id = TcgEfiSpecIdEvent::new_tcg2(&self.algorithms);
        let header_data = spec_id.to_bytes();

        // Wrap in PCR event (legacy format for header)
        let mut result = Vec::new();
        result.extend_from_slice(&0u32.to_le_bytes()); // PCR index
        result.extend_from_slice(&event_type::EV_NO_ACTION.to_le_bytes()); // Event type
        result.extend_from_slice(&[0u8; 20]); // SHA-1 digest (zeros for NO_ACTION)
        result.extend_from_slice(&(header_data.len() as u32).to_le_bytes()); // Event size
        result.extend_from_slice(&header_data);

        result
    }

    /// Serialize full log
    pub fn serialize(&self) -> Vec<u8> {
        let mut result = self.serialize_header();

        for event in &self.events {
            result.extend_from_slice(&event.to_bytes());
        }

        result
    }
}

impl Default for EventLog {
    fn default() -> Self {
        Self::new(&[(algorithm::TPM_ALG_SHA256, 32)])
    }
}

/// Measured boot context
#[derive(Debug)]
pub struct MeasuredBoot {
    /// PCR banks
    banks: Vec<PcrBank>,
    /// Event log
    event_log: EventLog,
    /// Number of PCRs
    num_pcrs: usize,
}

impl MeasuredBoot {
    /// Create new measured boot context
    pub fn new(algorithms: &[u16], num_pcrs: usize) -> Self {
        let mut banks = Vec::new();
        let mut alg_sizes = Vec::new();

        for &alg in algorithms {
            let size = TpmtHa::digest_size(alg);
            if size > 0 {
                banks.push(PcrBank::new(alg, num_pcrs));
                alg_sizes.push((alg, size as u16));
            }
        }

        Self {
            banks,
            event_log: EventLog::new(&alg_sizes),
            num_pcrs,
        }
    }

    /// Create with SHA-256 only
    pub fn new_sha256(num_pcrs: usize) -> Self {
        Self::new(&[algorithm::TPM_ALG_SHA256], num_pcrs)
    }

    /// Extend PCR with data
    pub fn extend(&mut self, pcr: u32, event_type: u32, data: &[u8]) {
        // Compute digests
        let algorithms: Vec<u16> = self.banks.iter().map(|b| b.algorithm).collect();

        // Create event
        let event = TcgPcrEvent2::new_multi_hash(pcr, event_type, data, &algorithms);

        // Extend all banks
        for (bank_idx, bank) in self.banks.iter_mut().enumerate() {
            if bank_idx < event.digests.digests.len() {
                bank.extend(pcr, &event.digests.digests[bank_idx].digest);
            }
        }

        // Add to event log
        self.event_log.add_event(event);
    }

    /// Extend PCR with pre-computed digest
    pub fn extend_digest(&mut self, pcr: u32, event_type: u32, digest: &[u8], event_data: &[u8]) {
        // Create digests for all algorithms
        let mut digests = TpmlDigestValues::new();

        for bank in &self.banks {
            // Use provided digest if it matches, otherwise compute
            let dig = if digest.len() == TpmtHa::digest_size(bank.algorithm) {
                digest.to_vec()
            } else {
                match bank.algorithm {
                    algorithm::TPM_ALG_SHA256 => Sha256::digest(event_data).to_vec(),
                    algorithm::TPM_ALG_SHA384 => Sha512::digest_384(event_data).to_vec(),
                    algorithm::TPM_ALG_SHA512 => Sha512::digest_512(event_data).to_vec(),
                    _ => continue,
                }
            };
            digests.add(bank.algorithm, &dig);
        }

        // Create event
        let event = TcgPcrEvent2 {
            pcr_index: pcr,
            event_type,
            digests: digests.clone(),
            event_size: event_data.len() as u32,
            event: event_data.to_vec(),
        };

        // Extend all banks
        for (bank_idx, bank) in self.banks.iter_mut().enumerate() {
            if bank_idx < digests.digests.len() {
                bank.extend(pcr, &digests.digests[bank_idx].digest);
            }
        }

        // Add to event log
        self.event_log.add_event(event);
    }

    /// Measure separator
    pub fn measure_separator(&mut self, pcrs: &[u32]) {
        let separator = [0u8; 4];

        for &pcr in pcrs {
            self.extend(pcr, event_type::EV_SEPARATOR, &separator);
        }
    }

    /// Measure UEFI variable
    pub fn measure_variable(
        &mut self,
        pcr: u32,
        variable_name: &str,
        vendor_guid: &[u8; 16],
        data: &[u8],
    ) {
        // Build UEFI_VARIABLE_DATA structure
        let mut event_data = Vec::new();

        // VariableName GUID
        event_data.extend_from_slice(vendor_guid);

        // UnicodeNameLength
        let name_len = variable_name.encode_utf16().count() as u64;
        event_data.extend_from_slice(&name_len.to_le_bytes());

        // VariableDataLength
        event_data.extend_from_slice(&(data.len() as u64).to_le_bytes());

        // UnicodeName
        for c in variable_name.encode_utf16() {
            event_data.extend_from_slice(&c.to_le_bytes());
        }

        // VariableData
        event_data.extend_from_slice(data);

        self.extend(pcr, event_type::EV_EFI_VARIABLE_DRIVER_CONFIG, &event_data);
    }

    /// Measure PE image
    pub fn measure_pe_image(&mut self, pcr: u32, image_path: &str, image_hash: &[u8; 32]) {
        // Build UEFI_IMAGE_LOAD_EVENT structure
        let mut event_data = Vec::new();

        // ImageLocationInMemory (placeholder)
        event_data.extend_from_slice(&0u64.to_le_bytes());

        // ImageLengthInMemory (placeholder)
        event_data.extend_from_slice(&0u64.to_le_bytes());

        // ImageLinkTimeAddress (placeholder)
        event_data.extend_from_slice(&0u64.to_le_bytes());

        // LengthOfDevicePath
        let path_bytes: Vec<u8> = image_path.bytes().collect();
        event_data.extend_from_slice(&(path_bytes.len() as u64).to_le_bytes());

        // DevicePath
        event_data.extend_from_slice(&path_bytes);

        self.extend_digest(
            pcr,
            event_type::EV_EFI_BOOT_SERVICES_APPLICATION,
            image_hash,
            &event_data,
        );
    }

    /// Get PCR value
    pub fn get_pcr(&self, algorithm: u16, pcr: u32) -> Option<&[u8]> {
        for bank in &self.banks {
            if bank.algorithm == algorithm {
                return bank.get(pcr);
            }
        }
        None
    }

    /// Get event log
    pub fn event_log(&self) -> &EventLog {
        &self.event_log
    }

    /// Get serialized event log
    pub fn get_event_log_bytes(&self) -> Vec<u8> {
        self.event_log.serialize()
    }
}

impl Default for MeasuredBoot {
    fn default() -> Self {
        Self::new_sha256(24)
    }
}

// =============================================================================
// ERRORS
// =============================================================================

/// TPM error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpmError {
    /// TPM not present
    NotPresent,
    /// TPM not ready
    NotReady,
    /// Invalid command
    InvalidCommand,
    /// Invalid response
    InvalidResponse,
    /// Command failed
    CommandFailed(u32),
    /// Communication error
    CommunicationError,
    /// Locality error
    LocalityError,
    /// Unsupported algorithm
    UnsupportedAlgorithm,
}

impl TpmError {
    /// Create from response code
    pub fn from_response_code(code: u32) -> Option<Self> {
        if code == response::TPM_RC_SUCCESS {
            None
        } else {
            Some(Self::CommandFailed(code))
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
    fn test_tpm2b_digest() {
        let data = [1, 2, 3, 4];
        let digest = Tpm2bDigest::new(&data);
        assert_eq!(digest.size, 4);
        assert_eq!(digest.buffer, data);

        let bytes = digest.to_bytes();
        let (parsed, consumed) = Tpm2bDigest::from_bytes(&bytes).unwrap();
        assert_eq!(consumed, 6);
        assert_eq!(parsed.buffer, data);
    }

    #[test]
    fn test_tpml_digest_values() {
        let mut digests = TpmlDigestValues::new();
        digests.add(algorithm::TPM_ALG_SHA256, &[0u8; 32]);

        assert_eq!(digests.count, 1);
        assert_eq!(digests.digests[0].hash_alg, algorithm::TPM_ALG_SHA256);
    }

    #[test]
    fn test_pcr_bank_extend() {
        let mut bank = PcrBank::new(algorithm::TPM_ALG_SHA256, 24);

        // Initial value should be zeros
        assert_eq!(bank.get(0).unwrap(), &[0u8; 32]);

        // Extend
        bank.extend(0, &[1, 2, 3, 4]);

        // Value should change
        assert_ne!(bank.get(0).unwrap(), &[0u8; 32]);
    }

    #[test]
    fn test_measured_boot() {
        let mut mb = MeasuredBoot::new_sha256(24);

        // Measure some data
        mb.extend(0, event_type::EV_S_CRTM_VERSION, b"test");

        // PCR should be extended
        let pcr0 = mb.get_pcr(algorithm::TPM_ALG_SHA256, 0).unwrap();
        assert_ne!(pcr0, &[0u8; 32]);

        // Event log should have entry
        assert_eq!(mb.event_log().events().len(), 1);
    }

    #[test]
    fn test_tpm2_command_build() {
        let cmd = Tpm2Command::new(command::TPM_CC_STARTUP)
            .add_u16(startup_type::TPM_SU_CLEAR)
            .build();

        // Check header
        assert_eq!(&cmd[0..2], &0x8001u16.to_be_bytes());
        assert_eq!(&cmd[6..10], &command::TPM_CC_STARTUP.to_be_bytes());
    }

    #[test]
    fn test_event_log_serialize() {
        let log = EventLog::default();
        let header = log.serialize_header();

        // Should contain Spec ID Event signature
        assert!(header.windows(16).any(|w| w == b"Spec ID Event03\0"));
    }
}
