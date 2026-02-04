//! SCSI (Small Computer System Interface) Support for Helix UEFI Bootloader
//!
//! This module provides comprehensive SCSI, SAS, and ISCSI protocol support
//! for enterprise storage access in the UEFI boot environment.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         SCSI Protocol Stack                             │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Block Layer    │  Read  │  Write  │  Verify  │  Sync  │  Unmap        │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  SCSI Layer     │  CDB Parsing  │  Sense Data  │  Mode Pages           │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Transport      │  SAS  │  iSCSI  │  FC  │  USB Mass Storage           │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Hardware       │  HBA  │  Network  │  DMA  │  Interrupts              │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Features
//!
//! - SCSI Primary Commands (SPC-5)
//! - SCSI Block Commands (SBC-4)
//! - SCSI Enclosure Services (SES-3)
//! - SAS 3.0/4.0 support
//! - iSCSI boot support
//! - Sense data parsing
//! - Mode page handling

use core::fmt;

// =============================================================================
// SCSI CONSTANTS
// =============================================================================

/// Maximum CDB length
pub const MAX_CDB_LENGTH: usize = 32;

/// Maximum sense data length
pub const MAX_SENSE_LENGTH: usize = 252;

/// Standard inquiry data length
pub const STD_INQUIRY_LENGTH: usize = 36;

/// Default block size
pub const DEFAULT_BLOCK_SIZE: u32 = 512;

/// LUN addressing
pub const MAX_LUN: u8 = 255;

// =============================================================================
// SCSI OPERATION CODES
// =============================================================================

/// SCSI operation codes
pub mod opcode {
    // 6-byte commands
    /// Test Unit Ready
    pub const TEST_UNIT_READY: u8 = 0x00;
    /// Request Sense
    pub const REQUEST_SENSE: u8 = 0x03;
    /// Format Unit
    pub const FORMAT_UNIT: u8 = 0x04;
    /// Read (6)
    pub const READ_6: u8 = 0x08;
    /// Write (6)
    pub const WRITE_6: u8 = 0x0A;
    /// Inquiry
    pub const INQUIRY: u8 = 0x12;
    /// Mode Select (6)
    pub const MODE_SELECT_6: u8 = 0x15;
    /// Reserve (6)
    pub const RESERVE_6: u8 = 0x16;
    /// Release (6)
    pub const RELEASE_6: u8 = 0x17;
    /// Mode Sense (6)
    pub const MODE_SENSE_6: u8 = 0x1A;
    /// Start Stop Unit
    pub const START_STOP_UNIT: u8 = 0x1B;
    /// Receive Diagnostic Results
    pub const RECEIVE_DIAGNOSTIC: u8 = 0x1C;
    /// Send Diagnostic
    pub const SEND_DIAGNOSTIC: u8 = 0x1D;
    /// Prevent Allow Medium Removal
    pub const PREVENT_ALLOW_MEDIUM: u8 = 0x1E;

    // 10-byte commands
    /// Read Capacity (10)
    pub const READ_CAPACITY_10: u8 = 0x25;
    /// Read (10)
    pub const READ_10: u8 = 0x28;
    /// Write (10)
    pub const WRITE_10: u8 = 0x2A;
    /// Seek (10)
    pub const SEEK_10: u8 = 0x2B;
    /// Write and Verify (10)
    pub const WRITE_VERIFY_10: u8 = 0x2E;
    /// Verify (10)
    pub const VERIFY_10: u8 = 0x2F;
    /// Synchronize Cache (10)
    pub const SYNCHRONIZE_CACHE_10: u8 = 0x35;
    /// Read Defect Data (10)
    pub const READ_DEFECT_DATA_10: u8 = 0x37;
    /// Write Buffer
    pub const WRITE_BUFFER: u8 = 0x3B;
    /// Read Buffer
    pub const READ_BUFFER: u8 = 0x3C;
    /// Unmap
    pub const UNMAP: u8 = 0x42;
    /// Log Select
    pub const LOG_SELECT: u8 = 0x4C;
    /// Log Sense
    pub const LOG_SENSE: u8 = 0x4D;
    /// Mode Select (10)
    pub const MODE_SELECT_10: u8 = 0x55;
    /// Mode Sense (10)
    pub const MODE_SENSE_10: u8 = 0x5A;
    /// Persistent Reserve In
    pub const PERSISTENT_RESERVE_IN: u8 = 0x5E;
    /// Persistent Reserve Out
    pub const PERSISTENT_RESERVE_OUT: u8 = 0x5F;

    // 12-byte commands
    /// Report Luns
    pub const REPORT_LUNS: u8 = 0xA0;
    /// Security Protocol In
    pub const SECURITY_PROTOCOL_IN: u8 = 0xA2;
    /// Maintenance In
    pub const MAINTENANCE_IN: u8 = 0xA3;
    /// Maintenance Out
    pub const MAINTENANCE_OUT: u8 = 0xA4;
    /// Read (12)
    pub const READ_12: u8 = 0xA8;
    /// Write (12)
    pub const WRITE_12: u8 = 0xAA;
    /// Verify (12)
    pub const VERIFY_12: u8 = 0xAF;
    /// Security Protocol Out
    pub const SECURITY_PROTOCOL_OUT: u8 = 0xB5;

    // 16-byte commands
    /// Service Action In (16)
    pub const SERVICE_ACTION_IN_16: u8 = 0x9E;
    /// Service Action Out (16)
    pub const SERVICE_ACTION_OUT_16: u8 = 0x9F;
    /// Read (16)
    pub const READ_16: u8 = 0x88;
    /// Write (16)
    pub const WRITE_16: u8 = 0x8A;
    /// Write and Verify (16)
    pub const WRITE_VERIFY_16: u8 = 0x8E;
    /// Verify (16)
    pub const VERIFY_16: u8 = 0x8F;
    /// Synchronize Cache (16)
    pub const SYNCHRONIZE_CACHE_16: u8 = 0x91;
    /// Write Same (16)
    pub const WRITE_SAME_16: u8 = 0x93;

    // Service actions for SERVICE_ACTION_IN_16
    /// Read Capacity (16)
    pub const SA_READ_CAPACITY_16: u8 = 0x10;
    /// Get LBA Status
    pub const SA_GET_LBA_STATUS: u8 = 0x12;
    /// Report Referrals
    pub const SA_REPORT_REFERRALS: u8 = 0x13;

    // Variable length commands (32 bytes)
    /// Variable length CDB
    pub const VARIABLE_LENGTH_CDB: u8 = 0x7F;
}

// =============================================================================
// SCSI STATUS CODES
// =============================================================================

/// SCSI status byte values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ScsiStatus {
    /// Command completed successfully
    Good                = 0x00,
    /// Check Condition status
    CheckCondition      = 0x02,
    /// Condition Met
    ConditionMet        = 0x04,
    /// Target is busy
    Busy                = 0x08,
    /// Intermediate status
    Intermediate        = 0x10,
    /// Intermediate Condition Met
    IntermediateConditionMet = 0x14,
    /// Reservation Conflict
    ReservationConflict = 0x18,
    /// Command Terminated
    CommandTerminated   = 0x22,
    /// Task Set Full
    TaskSetFull         = 0x28,
    /// ACA Active
    AcaActive           = 0x30,
    /// Task Aborted
    TaskAborted         = 0x40,
}

impl ScsiStatus {
    /// Create from status byte
    pub fn from_byte(byte: u8) -> Self {
        match byte & 0x7E {
            0x00 => ScsiStatus::Good,
            0x02 => ScsiStatus::CheckCondition,
            0x04 => ScsiStatus::ConditionMet,
            0x08 => ScsiStatus::Busy,
            0x10 => ScsiStatus::Intermediate,
            0x14 => ScsiStatus::IntermediateConditionMet,
            0x18 => ScsiStatus::ReservationConflict,
            0x22 => ScsiStatus::CommandTerminated,
            0x28 => ScsiStatus::TaskSetFull,
            0x30 => ScsiStatus::AcaActive,
            0x40 => ScsiStatus::TaskAborted,
            _ => ScsiStatus::CheckCondition, // Unknown treated as check condition
        }
    }

    /// Check if status indicates success
    pub const fn is_good(&self) -> bool {
        matches!(self, ScsiStatus::Good)
    }

    /// Check if sense data is available
    pub const fn has_sense(&self) -> bool {
        matches!(self, ScsiStatus::CheckCondition)
    }
}

// =============================================================================
// SENSE KEY AND CODES
// =============================================================================

/// SCSI sense keys
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SenseKey {
    /// No error or no sense information
    NoSense        = 0x00,
    /// Recovered error
    RecoveredError = 0x01,
    /// Device not ready
    NotReady       = 0x02,
    /// Medium error
    MediumError    = 0x03,
    /// Hardware error
    HardwareError  = 0x04,
    /// Illegal request
    IllegalRequest = 0x05,
    /// Unit attention
    UnitAttention  = 0x06,
    /// Write protected
    DataProtect    = 0x07,
    /// Blank check
    BlankCheck     = 0x08,
    /// Vendor specific
    VendorSpecific = 0x09,
    /// Copy aborted
    CopyAborted    = 0x0A,
    /// Aborted command
    AbortedCommand = 0x0B,
    /// Equal (obsolete)
    Equal          = 0x0C,
    /// Volume overflow
    VolumeOverflow = 0x0D,
    /// Miscompare
    Miscompare     = 0x0E,
    /// Completed
    Completed      = 0x0F,
}

impl SenseKey {
    /// Create from sense key value
    pub fn from_byte(byte: u8) -> Self {
        match byte & 0x0F {
            0x00 => SenseKey::NoSense,
            0x01 => SenseKey::RecoveredError,
            0x02 => SenseKey::NotReady,
            0x03 => SenseKey::MediumError,
            0x04 => SenseKey::HardwareError,
            0x05 => SenseKey::IllegalRequest,
            0x06 => SenseKey::UnitAttention,
            0x07 => SenseKey::DataProtect,
            0x08 => SenseKey::BlankCheck,
            0x09 => SenseKey::VendorSpecific,
            0x0A => SenseKey::CopyAborted,
            0x0B => SenseKey::AbortedCommand,
            0x0C => SenseKey::Equal,
            0x0D => SenseKey::VolumeOverflow,
            0x0E => SenseKey::Miscompare,
            0x0F => SenseKey::Completed,
            _ => SenseKey::NoSense,
        }
    }

    /// Get human-readable description
    pub const fn description(&self) -> &'static str {
        match self {
            SenseKey::NoSense => "No sense",
            SenseKey::RecoveredError => "Recovered error",
            SenseKey::NotReady => "Not ready",
            SenseKey::MediumError => "Medium error",
            SenseKey::HardwareError => "Hardware error",
            SenseKey::IllegalRequest => "Illegal request",
            SenseKey::UnitAttention => "Unit attention",
            SenseKey::DataProtect => "Data protect",
            SenseKey::BlankCheck => "Blank check",
            SenseKey::VendorSpecific => "Vendor specific",
            SenseKey::CopyAborted => "Copy aborted",
            SenseKey::AbortedCommand => "Aborted command",
            SenseKey::Equal => "Equal",
            SenseKey::VolumeOverflow => "Volume overflow",
            SenseKey::Miscompare => "Miscompare",
            SenseKey::Completed => "Completed",
        }
    }
}

/// Additional Sense Code (ASC) values
pub mod asc {
    /// No additional sense
    pub const NO_ADDITIONAL_SENSE: u8 = 0x00;
    /// Logical unit not ready
    pub const LU_NOT_READY: u8 = 0x04;
    /// Logical unit communication failure
    pub const LU_COMMUNICATION_FAILURE: u8 = 0x08;
    /// Write error
    pub const WRITE_ERROR: u8 = 0x0C;
    /// Read error
    pub const READ_ERROR: u8 = 0x11;
    /// Parameter list length error
    pub const PARAMETER_LIST_LENGTH_ERROR: u8 = 0x1A;
    /// Invalid command operation code
    pub const INVALID_OPCODE: u8 = 0x20;
    /// Invalid field in CDB
    pub const INVALID_FIELD_IN_CDB: u8 = 0x24;
    /// Logical unit not supported
    pub const LU_NOT_SUPPORTED: u8 = 0x25;
    /// Invalid field in parameter list
    pub const INVALID_FIELD_IN_PARAM_LIST: u8 = 0x26;
    /// Write protected
    pub const WRITE_PROTECTED: u8 = 0x27;
    /// Not ready to ready transition
    pub const NOT_READY_TO_READY: u8 = 0x28;
    /// Power on, reset, or bus reset occurred
    pub const POWER_ON_RESET: u8 = 0x29;
    /// Parameters changed
    pub const PARAMETERS_CHANGED: u8 = 0x2A;
    /// Copy cannot execute
    pub const COPY_CANNOT_EXECUTE: u8 = 0x2B;
    /// Commands cleared
    pub const COMMANDS_CLEARED: u8 = 0x2F;
    /// Medium not present
    pub const MEDIUM_NOT_PRESENT: u8 = 0x3A;
    /// Internal target failure
    pub const INTERNAL_TARGET_FAILURE: u8 = 0x44;
    /// Overlapped commands
    pub const OVERLAPPED_COMMANDS: u8 = 0x4E;
}

/// Additional Sense Code Qualifier (ASCQ) values
pub mod ascq {
    /// LU is in process of becoming ready
    pub const LU_BECOMING_READY: u8 = 0x01;
    /// Initializing command required
    pub const INIT_COMMAND_REQUIRED: u8 = 0x02;
    /// Manual intervention required
    pub const MANUAL_INTERVENTION: u8 = 0x03;
    /// Format in progress
    pub const FORMAT_IN_PROGRESS: u8 = 0x04;
    /// Operation in progress
    pub const OPERATION_IN_PROGRESS: u8 = 0x07;
    /// Self-test in progress
    pub const SELF_TEST_IN_PROGRESS: u8 = 0x09;
    /// Write error - auto reallocation failed
    pub const WRITE_REALLOC_FAILED: u8 = 0x02;
    /// Error recovery - recovered data with retries
    pub const RECOVERED_WITH_RETRIES: u8 = 0x01;
    /// Error recovery - recovered with ECC
    pub const RECOVERED_WITH_ECC: u8 = 0x02;
}

// =============================================================================
// SENSE DATA STRUCTURES
// =============================================================================

/// Sense data response format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SenseFormat {
    /// Fixed format sense data (70h, 71h)
    Fixed,
    /// Descriptor format sense data (72h, 73h)
    Descriptor,
    /// Unknown format
    Unknown,
}

/// Fixed format sense data (18 bytes minimum)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FixedSenseData {
    /// Response code (70h or 71h)
    pub response_code: u8,
    /// Obsolete
    pub obsolete: u8,
    /// Sense key, flags
    pub sense_key: u8,
    /// Information bytes
    pub information: [u8; 4],
    /// Additional sense length
    pub additional_length: u8,
    /// Command-specific information
    pub command_info: [u8; 4],
    /// Additional Sense Code
    pub asc: u8,
    /// Additional Sense Code Qualifier
    pub ascq: u8,
    /// Field Replaceable Unit Code
    pub fruc: u8,
    /// Sense key specific bytes
    pub sks: [u8; 3],
}

impl FixedSenseData {
    /// Create from byte slice
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 18 {
            return None;
        }

        Some(Self {
            response_code: data[0],
            obsolete: data[1],
            sense_key: data[2],
            information: [data[3], data[4], data[5], data[6]],
            additional_length: data[7],
            command_info: [data[8], data[9], data[10], data[11]],
            asc: data[12],
            ascq: data[13],
            fruc: data[14],
            sks: [data[15], data[16], data[17]],
        })
    }

    /// Check if current error
    pub const fn is_current(&self) -> bool {
        (self.response_code & 0x7F) == 0x70
    }

    /// Check if deferred error
    pub const fn is_deferred(&self) -> bool {
        (self.response_code & 0x7F) == 0x71
    }

    /// Get sense key
    pub fn sense_key(&self) -> SenseKey {
        SenseKey::from_byte(self.sense_key & 0x0F)
    }

    /// Check if FILEMARK bit is set
    pub const fn filemark(&self) -> bool {
        (self.sense_key & 0x80) != 0
    }

    /// Check if EOM bit is set
    pub const fn eom(&self) -> bool {
        (self.sense_key & 0x40) != 0
    }

    /// Check if ILI bit is set
    pub const fn ili(&self) -> bool {
        (self.sense_key & 0x20) != 0
    }

    /// Get information field as u32
    pub fn information(&self) -> u32 {
        u32::from_be_bytes(self.information)
    }

    /// Check if sense key specific is valid
    pub const fn sks_valid(&self) -> bool {
        (self.sks[0] & 0x80) != 0
    }
}

/// Descriptor format sense data header
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DescriptorSenseHeader {
    /// Response code (72h or 73h)
    pub response_code: u8,
    /// Sense key
    pub sense_key: u8,
    /// Additional Sense Code
    pub asc: u8,
    /// Additional Sense Code Qualifier
    pub ascq: u8,
    /// Reserved
    pub reserved: [u8; 3],
    /// Additional sense length
    pub additional_length: u8,
}

impl DescriptorSenseHeader {
    /// Create from byte slice
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }

        Some(Self {
            response_code: data[0],
            sense_key: data[1],
            asc: data[2],
            ascq: data[3],
            reserved: [data[4], data[5], data[6]],
            additional_length: data[7],
        })
    }

    /// Get sense key
    pub fn sense_key(&self) -> SenseKey {
        SenseKey::from_byte(self.sense_key & 0x0F)
    }
}

/// Sense descriptor types
pub mod sense_descriptor {
    /// Information
    pub const INFORMATION: u8 = 0x00;
    /// Command specific information
    pub const COMMAND_SPECIFIC: u8 = 0x01;
    /// Sense key specific
    pub const SENSE_KEY_SPECIFIC: u8 = 0x02;
    /// Field replaceable unit
    pub const FRU: u8 = 0x03;
    /// Stream commands
    pub const STREAM: u8 = 0x04;
    /// Block commands
    pub const BLOCK: u8 = 0x05;
    /// OSD object identification
    pub const OSD_OBJECT_ID: u8 = 0x06;
    /// OSD response integrity check value
    pub const OSD_INTEGRITY: u8 = 0x07;
    /// OSD attribute identification
    pub const OSD_ATTRIBUTE: u8 = 0x08;
    /// ATA status return
    pub const ATA_STATUS: u8 = 0x09;
    /// Another progress indication
    pub const PROGRESS: u8 = 0x0A;
    /// User data segment referral
    pub const USER_DATA_REFERRAL: u8 = 0x0B;
    /// Forwarded sense data
    pub const FORWARDED: u8 = 0x0C;
    /// Direct-access block device
    pub const DIRECT_ACCESS: u8 = 0x0D;
}

// =============================================================================
// INQUIRY DATA
// =============================================================================

/// Device type codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DeviceType {
    /// Direct access block device (disk)
    DirectAccess      = 0x00,
    /// Sequential access device (tape)
    SequentialAccess  = 0x01,
    /// Printer device
    Printer           = 0x02,
    /// Processor device
    Processor         = 0x03,
    /// Write-once device
    WriteOnce         = 0x04,
    /// CD/DVD device
    CdDvd             = 0x05,
    /// Scanner device
    Scanner           = 0x06,
    /// Optical memory device
    OpticalMemory     = 0x07,
    /// Medium changer device
    MediumChanger     = 0x08,
    /// Communications device
    Communications    = 0x09,
    /// Storage array controller
    StorageArrayController = 0x0C,
    /// Enclosure services device
    EnclosureServices = 0x0D,
    /// Simplified direct access
    SimplifiedDirectAccess = 0x0E,
    /// Optical card reader/writer
    OpticalCard       = 0x0F,
    /// Bridge controller
    Bridge            = 0x10,
    /// Object-based storage
    ObjectStorage     = 0x11,
    /// Automation/drive interface
    Automation        = 0x12,
    /// Security manager device
    SecurityManager   = 0x13,
    /// Host managed zoned block
    ZonedBlock        = 0x14,
    /// Well known logical unit
    WellKnown         = 0x1E,
    /// Unknown or no device type
    Unknown           = 0x1F,
}

impl DeviceType {
    /// Create from peripheral device type byte
    pub fn from_byte(byte: u8) -> Self {
        match byte & 0x1F {
            0x00 => DeviceType::DirectAccess,
            0x01 => DeviceType::SequentialAccess,
            0x02 => DeviceType::Printer,
            0x03 => DeviceType::Processor,
            0x04 => DeviceType::WriteOnce,
            0x05 => DeviceType::CdDvd,
            0x06 => DeviceType::Scanner,
            0x07 => DeviceType::OpticalMemory,
            0x08 => DeviceType::MediumChanger,
            0x09 => DeviceType::Communications,
            0x0C => DeviceType::StorageArrayController,
            0x0D => DeviceType::EnclosureServices,
            0x0E => DeviceType::SimplifiedDirectAccess,
            0x0F => DeviceType::OpticalCard,
            0x10 => DeviceType::Bridge,
            0x11 => DeviceType::ObjectStorage,
            0x12 => DeviceType::Automation,
            0x13 => DeviceType::SecurityManager,
            0x14 => DeviceType::ZonedBlock,
            0x1E => DeviceType::WellKnown,
            _ => DeviceType::Unknown,
        }
    }

    /// Check if this is a block device
    pub const fn is_block_device(&self) -> bool {
        matches!(
            self,
            DeviceType::DirectAccess | DeviceType::SimplifiedDirectAccess | DeviceType::ZonedBlock
        )
    }
}

/// Standard Inquiry data (36 bytes minimum)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct InquiryData {
    /// Peripheral device type and qualifier
    pub peripheral: u8,
    /// Removable medium bit and device type modifier
    pub rmb: u8,
    /// Version
    pub version: u8,
    /// Response data format and other flags
    pub response_format: u8,
    /// Additional length
    pub additional_length: u8,
    /// Various capability bits
    pub protect: u8,
    /// More capability bits
    pub cmdque: u8,
    /// VS, linked, etc.
    pub vs: u8,
    /// Vendor identification (8 bytes)
    pub vendor: [u8; 8],
    /// Product identification (16 bytes)
    pub product: [u8; 16],
    /// Product revision level (4 bytes)
    pub revision: [u8; 4],
}

impl InquiryData {
    /// Create from byte slice
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 36 {
            return None;
        }

        let mut vendor = [0u8; 8];
        let mut product = [0u8; 16];
        let mut revision = [0u8; 4];

        vendor.copy_from_slice(&data[8..16]);
        product.copy_from_slice(&data[16..32]);
        revision.copy_from_slice(&data[32..36]);

        Some(Self {
            peripheral: data[0],
            rmb: data[1],
            version: data[2],
            response_format: data[3],
            additional_length: data[4],
            protect: data[5],
            cmdque: data[6],
            vs: data[7],
            vendor,
            product,
            revision,
        })
    }

    /// Get device type
    pub fn device_type(&self) -> DeviceType {
        DeviceType::from_byte(self.peripheral)
    }

    /// Get peripheral qualifier
    pub const fn peripheral_qualifier(&self) -> u8 {
        (self.peripheral >> 5) & 0x07
    }

    /// Check if device is connected
    pub const fn is_connected(&self) -> bool {
        self.peripheral_qualifier() == 0
    }

    /// Check if removable medium
    pub const fn is_removable(&self) -> bool {
        (self.rmb & 0x80) != 0
    }

    /// Get SCSI version
    pub const fn scsi_version(&self) -> u8 {
        self.version
    }

    /// Get response data format
    pub const fn response_data_format(&self) -> u8 {
        self.response_format & 0x0F
    }

    /// Check HiSup (hierarchical support)
    pub const fn hi_sup(&self) -> bool {
        (self.response_format & 0x10) != 0
    }

    /// Check NormACA (normal ACA supported)
    pub const fn norm_aca(&self) -> bool {
        (self.response_format & 0x20) != 0
    }

    /// Check if command queuing supported
    pub const fn cmd_que(&self) -> bool {
        (self.cmdque & 0x02) != 0
    }

    /// Check if linked commands supported
    pub const fn linked(&self) -> bool {
        (self.vs & 0x08) != 0
    }

    /// Get vendor string (trimmed)
    pub fn vendor_str(&self) -> &[u8] {
        let end = self
            .vendor
            .iter()
            .rposition(|&c| c != b' ' && c != 0)
            .map(|i| i + 1)
            .unwrap_or(0);
        &self.vendor[..end]
    }

    /// Get product string (trimmed)
    pub fn product_str(&self) -> &[u8] {
        let end = self
            .product
            .iter()
            .rposition(|&c| c != b' ' && c != 0)
            .map(|i| i + 1)
            .unwrap_or(0);
        &self.product[..end]
    }

    /// Get revision string (trimmed)
    pub fn revision_str(&self) -> &[u8] {
        let end = self
            .revision
            .iter()
            .rposition(|&c| c != b' ' && c != 0)
            .map(|i| i + 1)
            .unwrap_or(0);
        &self.revision[..end]
    }
}

/// VPD page codes
pub mod vpd {
    /// Supported VPD pages
    pub const SUPPORTED_PAGES: u8 = 0x00;
    /// Unit serial number
    pub const UNIT_SERIAL_NUMBER: u8 = 0x80;
    /// Device identification
    pub const DEVICE_IDENTIFICATION: u8 = 0x83;
    /// Software interface identification
    pub const SOFTWARE_INTERFACE_ID: u8 = 0x84;
    /// Management network addresses
    pub const MANAGEMENT_NETWORK: u8 = 0x85;
    /// Extended inquiry data
    pub const EXTENDED_INQUIRY: u8 = 0x86;
    /// Mode page policy
    pub const MODE_PAGE_POLICY: u8 = 0x87;
    /// SCSI ports
    pub const SCSI_PORTS: u8 = 0x88;
    /// ATA information
    pub const ATA_INFO: u8 = 0x89;
    /// Power condition
    pub const POWER_CONDITION: u8 = 0x8A;
    /// Device constituents
    pub const DEVICE_CONSTITUENTS: u8 = 0x8B;
    /// CFA profile information
    pub const CFA_PROFILE: u8 = 0x8C;
    /// Power consumption
    pub const POWER_CONSUMPTION: u8 = 0x8D;
    /// Third party copy
    pub const THIRD_PARTY_COPY: u8 = 0x8F;
    /// Protocol specific logical unit info
    pub const PROTOCOL_LU_INFO: u8 = 0x90;
    /// Protocol specific port info
    pub const PROTOCOL_PORT_INFO: u8 = 0x91;
    /// SCSI Feature Sets
    pub const SCSI_FEATURE_SETS: u8 = 0x92;
    /// Block limits
    pub const BLOCK_LIMITS: u8 = 0xB0;
    /// Block device characteristics
    pub const BLOCK_DEVICE_CHARACTERISTICS: u8 = 0xB1;
    /// Logical block provisioning
    pub const LOGICAL_BLOCK_PROVISIONING: u8 = 0xB2;
    /// Referrals
    pub const REFERRALS: u8 = 0xB3;
    /// Supported block lengths and protection
    pub const BLOCK_LENGTHS_PROTECTION: u8 = 0xB4;
    /// Block device characteristics extension
    pub const BLOCK_DEVICE_EXT: u8 = 0xB5;
    /// Zoned block device characteristics
    pub const ZONED_BLOCK_DEVICE: u8 = 0xB6;
    /// Block limits extension
    pub const BLOCK_LIMITS_EXT: u8 = 0xB7;
    /// Format presets
    pub const FORMAT_PRESETS: u8 = 0xB8;
}

// =============================================================================
// READ CAPACITY DATA
// =============================================================================

/// Read Capacity (10) response
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ReadCapacity10 {
    /// Returned logical block address (last LBA)
    pub lba: [u8; 4],
    /// Block length in bytes
    pub block_length: [u8; 4],
}

impl ReadCapacity10 {
    /// Create from byte slice
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }

        Some(Self {
            lba: [data[0], data[1], data[2], data[3]],
            block_length: [data[4], data[5], data[6], data[7]],
        })
    }

    /// Get last LBA
    pub fn last_lba(&self) -> u32 {
        u32::from_be_bytes(self.lba)
    }

    /// Get block length
    pub fn block_length(&self) -> u32 {
        u32::from_be_bytes(self.block_length)
    }

    /// Get total capacity in bytes
    pub fn capacity_bytes(&self) -> u64 {
        (self.last_lba() as u64 + 1) * (self.block_length() as u64)
    }

    /// Get total number of blocks
    pub fn total_blocks(&self) -> u64 {
        self.last_lba() as u64 + 1
    }
}

/// Read Capacity (16) response
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ReadCapacity16 {
    /// Returned logical block address (last LBA)
    pub lba: [u8; 8],
    /// Block length in bytes
    pub block_length: [u8; 4],
    /// Protection and other flags
    pub flags: u8,
    /// Logical blocks per physical block exponent and other
    pub lb_per_pb: u8,
    /// Lowest aligned LBA
    pub lowest_aligned: [u8; 2],
    /// Reserved
    pub reserved: [u8; 16],
}

impl ReadCapacity16 {
    /// Create from byte slice
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 32 {
            return None;
        }

        let mut reserved = [0u8; 16];
        reserved.copy_from_slice(&data[16..32]);

        Some(Self {
            lba: [
                data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
            ],
            block_length: [data[8], data[9], data[10], data[11]],
            flags: data[12],
            lb_per_pb: data[13],
            lowest_aligned: [data[14], data[15]],
            reserved,
        })
    }

    /// Get last LBA
    pub fn last_lba(&self) -> u64 {
        u64::from_be_bytes(self.lba)
    }

    /// Get block length
    pub fn block_length(&self) -> u32 {
        u32::from_be_bytes(self.block_length)
    }

    /// Get total capacity in bytes
    pub fn capacity_bytes(&self) -> u128 {
        (self.last_lba() as u128 + 1) * (self.block_length() as u128)
    }

    /// Get total number of blocks
    pub fn total_blocks(&self) -> u64 {
        self.last_lba() + 1
    }

    /// Check if protection enabled
    pub const fn protection_enabled(&self) -> bool {
        (self.flags & 0x01) != 0
    }

    /// Get protection type
    pub const fn protection_type(&self) -> u8 {
        (self.flags >> 1) & 0x07
    }

    /// Get logical blocks per physical block exponent
    pub const fn lb_per_pb_exponent(&self) -> u8 {
        self.lb_per_pb & 0x0F
    }

    /// Check thin provisioning enabled
    pub const fn thin_provisioning_enabled(&self) -> bool {
        (self.lb_per_pb & 0x80) != 0
    }

    /// Check thin provisioning read zeros
    pub const fn thin_provisioning_read_zeros(&self) -> bool {
        (self.lb_per_pb & 0x40) != 0
    }
}

// =============================================================================
// MODE PAGES
// =============================================================================

/// Mode page codes
pub mod mode_page {
    /// Read-write error recovery
    pub const READ_WRITE_ERROR_RECOVERY: u8 = 0x01;
    /// Disconnect-reconnect
    pub const DISCONNECT_RECONNECT: u8 = 0x02;
    /// Format device (obsolete)
    pub const FORMAT_DEVICE: u8 = 0x03;
    /// Rigid disk geometry (obsolete)
    pub const RIGID_DISK_GEOMETRY: u8 = 0x04;
    /// Verify error recovery
    pub const VERIFY_ERROR_RECOVERY: u8 = 0x07;
    /// Caching
    pub const CACHING: u8 = 0x08;
    /// Control
    pub const CONTROL: u8 = 0x0A;
    /// Medium types supported (obsolete)
    pub const MEDIUM_TYPES: u8 = 0x0B;
    /// Notch and partition (obsolete)
    pub const NOTCH_PARTITION: u8 = 0x0C;
    /// Power condition (obsolete)
    pub const POWER_CONDITION_LEGACY: u8 = 0x0D;
    /// XOR control
    pub const XOR_CONTROL: u8 = 0x10;
    /// Enclosure services management
    pub const ENCLOSURE_SERVICES: u8 = 0x14;
    /// Protocol specific LUN
    pub const PROTOCOL_SPECIFIC_LUN: u8 = 0x18;
    /// Protocol specific port
    pub const PROTOCOL_SPECIFIC_PORT: u8 = 0x19;
    /// Power condition
    pub const POWER_CONDITION: u8 = 0x1A;
    /// Informational exceptions control
    pub const INFORMATIONAL_EXCEPTIONS: u8 = 0x1C;
    /// All pages
    pub const ALL_PAGES: u8 = 0x3F;
}

/// Mode parameter header (6)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ModeHeader6 {
    /// Mode data length
    pub data_length: u8,
    /// Medium type
    pub medium_type: u8,
    /// Device-specific parameter
    pub device_specific: u8,
    /// Block descriptor length
    pub block_desc_length: u8,
}

/// Mode parameter header (10)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ModeHeader10 {
    /// Mode data length (MSB)
    pub data_length_msb: u8,
    /// Mode data length (LSB)
    pub data_length_lsb: u8,
    /// Medium type
    pub medium_type: u8,
    /// Device-specific parameter
    pub device_specific: u8,
    /// Reserved
    pub reserved: [u8; 2],
    /// Block descriptor length (MSB)
    pub block_desc_length_msb: u8,
    /// Block descriptor length (LSB)
    pub block_desc_length_lsb: u8,
}

impl ModeHeader10 {
    /// Get data length
    pub const fn data_length(&self) -> u16 {
        ((self.data_length_msb as u16) << 8) | (self.data_length_lsb as u16)
    }

    /// Get block descriptor length
    pub const fn block_desc_length(&self) -> u16 {
        ((self.block_desc_length_msb as u16) << 8) | (self.block_desc_length_lsb as u16)
    }
}

/// Caching mode page (page code 0x08)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CachingModePage {
    /// Page code and PS bit
    pub page_code: u8,
    /// Page length
    pub page_length: u8,
    /// IC, ABPF, CAP, DISC, SIZE, WCE, MF, RCD
    pub flags1: u8,
    /// Demand read/write retention priority
    pub retention: u8,
    /// Disable pre-fetch transfer length
    pub prefetch_length: [u8; 2],
    /// Minimum pre-fetch
    pub min_prefetch: [u8; 2],
    /// Maximum pre-fetch
    pub max_prefetch: [u8; 2],
    /// Maximum pre-fetch ceiling
    pub max_prefetch_ceiling: [u8; 2],
    /// FSW, LBCSS, DRA, VS
    pub flags2: u8,
    /// Number of cache segments
    pub cache_segments: u8,
    /// Cache segment size
    pub cache_segment_size: [u8; 2],
    /// Reserved
    pub reserved: u8,
    /// Obsolete
    pub obsolete: [u8; 3],
}

impl CachingModePage {
    /// Check if write cache enabled
    pub const fn write_cache_enabled(&self) -> bool {
        (self.flags1 & 0x04) != 0
    }

    /// Check if read cache disabled
    pub const fn read_cache_disabled(&self) -> bool {
        (self.flags1 & 0x01) != 0
    }
}

// =============================================================================
// CDB BUILDERS
// =============================================================================

/// Command Descriptor Block builder
pub struct CdbBuilder {
    /// CDB data
    cdb: [u8; MAX_CDB_LENGTH],
    /// Current length
    len: usize,
}

impl CdbBuilder {
    /// Create new CDB builder
    pub const fn new() -> Self {
        Self {
            cdb: [0u8; MAX_CDB_LENGTH],
            len: 0,
        }
    }

    /// Build TEST UNIT READY command
    pub fn test_unit_ready(&mut self) -> &[u8] {
        self.cdb[0] = opcode::TEST_UNIT_READY;
        self.len = 6;
        &self.cdb[..self.len]
    }

    /// Build REQUEST SENSE command
    pub fn request_sense(&mut self, allocation_length: u8) -> &[u8] {
        self.cdb[0] = opcode::REQUEST_SENSE;
        self.cdb[4] = allocation_length;
        self.len = 6;
        &self.cdb[..self.len]
    }

    /// Build INQUIRY command
    pub fn inquiry(&mut self, evpd: bool, page_code: u8, allocation_length: u16) -> &[u8] {
        self.cdb[0] = opcode::INQUIRY;
        self.cdb[1] = if evpd { 0x01 } else { 0x00 };
        self.cdb[2] = page_code;
        self.cdb[3] = (allocation_length >> 8) as u8;
        self.cdb[4] = allocation_length as u8;
        self.len = 6;
        &self.cdb[..self.len]
    }

    /// Build READ CAPACITY (10) command
    pub fn read_capacity_10(&mut self) -> &[u8] {
        self.cdb[0] = opcode::READ_CAPACITY_10;
        self.len = 10;
        &self.cdb[..self.len]
    }

    /// Build READ CAPACITY (16) command
    pub fn read_capacity_16(&mut self, allocation_length: u32) -> &[u8] {
        self.cdb[0] = opcode::SERVICE_ACTION_IN_16;
        self.cdb[1] = opcode::SA_READ_CAPACITY_16;
        self.cdb[10] = (allocation_length >> 24) as u8;
        self.cdb[11] = (allocation_length >> 16) as u8;
        self.cdb[12] = (allocation_length >> 8) as u8;
        self.cdb[13] = allocation_length as u8;
        self.len = 16;
        &self.cdb[..self.len]
    }

    /// Build READ (10) command
    pub fn read_10(&mut self, lba: u32, transfer_length: u16) -> &[u8] {
        self.cdb[0] = opcode::READ_10;
        self.cdb[2] = (lba >> 24) as u8;
        self.cdb[3] = (lba >> 16) as u8;
        self.cdb[4] = (lba >> 8) as u8;
        self.cdb[5] = lba as u8;
        self.cdb[7] = (transfer_length >> 8) as u8;
        self.cdb[8] = transfer_length as u8;
        self.len = 10;
        &self.cdb[..self.len]
    }

    /// Build READ (16) command
    pub fn read_16(&mut self, lba: u64, transfer_length: u32) -> &[u8] {
        self.cdb[0] = opcode::READ_16;
        self.cdb[2] = (lba >> 56) as u8;
        self.cdb[3] = (lba >> 48) as u8;
        self.cdb[4] = (lba >> 40) as u8;
        self.cdb[5] = (lba >> 32) as u8;
        self.cdb[6] = (lba >> 24) as u8;
        self.cdb[7] = (lba >> 16) as u8;
        self.cdb[8] = (lba >> 8) as u8;
        self.cdb[9] = lba as u8;
        self.cdb[10] = (transfer_length >> 24) as u8;
        self.cdb[11] = (transfer_length >> 16) as u8;
        self.cdb[12] = (transfer_length >> 8) as u8;
        self.cdb[13] = transfer_length as u8;
        self.len = 16;
        &self.cdb[..self.len]
    }

    /// Build WRITE (10) command
    pub fn write_10(&mut self, lba: u32, transfer_length: u16) -> &[u8] {
        self.cdb[0] = opcode::WRITE_10;
        self.cdb[2] = (lba >> 24) as u8;
        self.cdb[3] = (lba >> 16) as u8;
        self.cdb[4] = (lba >> 8) as u8;
        self.cdb[5] = lba as u8;
        self.cdb[7] = (transfer_length >> 8) as u8;
        self.cdb[8] = transfer_length as u8;
        self.len = 10;
        &self.cdb[..self.len]
    }

    /// Build WRITE (16) command
    pub fn write_16(&mut self, lba: u64, transfer_length: u32) -> &[u8] {
        self.cdb[0] = opcode::WRITE_16;
        self.cdb[2] = (lba >> 56) as u8;
        self.cdb[3] = (lba >> 48) as u8;
        self.cdb[4] = (lba >> 40) as u8;
        self.cdb[5] = (lba >> 32) as u8;
        self.cdb[6] = (lba >> 24) as u8;
        self.cdb[7] = (lba >> 16) as u8;
        self.cdb[8] = (lba >> 8) as u8;
        self.cdb[9] = lba as u8;
        self.cdb[10] = (transfer_length >> 24) as u8;
        self.cdb[11] = (transfer_length >> 16) as u8;
        self.cdb[12] = (transfer_length >> 8) as u8;
        self.cdb[13] = transfer_length as u8;
        self.len = 16;
        &self.cdb[..self.len]
    }

    /// Build SYNCHRONIZE CACHE (10) command
    pub fn synchronize_cache_10(&mut self, lba: u32, num_blocks: u16) -> &[u8] {
        self.cdb[0] = opcode::SYNCHRONIZE_CACHE_10;
        self.cdb[2] = (lba >> 24) as u8;
        self.cdb[3] = (lba >> 16) as u8;
        self.cdb[4] = (lba >> 8) as u8;
        self.cdb[5] = lba as u8;
        self.cdb[7] = (num_blocks >> 8) as u8;
        self.cdb[8] = num_blocks as u8;
        self.len = 10;
        &self.cdb[..self.len]
    }

    /// Build UNMAP command
    pub fn unmap(&mut self, anchor: bool, group_number: u8, param_list_length: u16) -> &[u8] {
        self.cdb[0] = opcode::UNMAP;
        self.cdb[1] = if anchor { 0x01 } else { 0x00 };
        self.cdb[6] = group_number & 0x1F;
        self.cdb[7] = (param_list_length >> 8) as u8;
        self.cdb[8] = param_list_length as u8;
        self.len = 10;
        &self.cdb[..self.len]
    }

    /// Build MODE SENSE (6) command
    pub fn mode_sense_6(&mut self, dbd: bool, page_code: u8, allocation_length: u8) -> &[u8] {
        self.cdb[0] = opcode::MODE_SENSE_6;
        self.cdb[1] = if dbd { 0x08 } else { 0x00 };
        self.cdb[2] = page_code;
        self.cdb[4] = allocation_length;
        self.len = 6;
        &self.cdb[..self.len]
    }

    /// Build MODE SENSE (10) command
    pub fn mode_sense_10(
        &mut self,
        llbaa: bool,
        dbd: bool,
        page_code: u8,
        allocation_length: u16,
    ) -> &[u8] {
        self.cdb[0] = opcode::MODE_SENSE_10;
        let mut byte1 = 0u8;
        if llbaa {
            byte1 |= 0x10;
        }
        if dbd {
            byte1 |= 0x08;
        }
        self.cdb[1] = byte1;
        self.cdb[2] = page_code;
        self.cdb[7] = (allocation_length >> 8) as u8;
        self.cdb[8] = allocation_length as u8;
        self.len = 10;
        &self.cdb[..self.len]
    }

    /// Build START STOP UNIT command
    pub fn start_stop_unit(
        &mut self,
        immed: bool,
        power_condition: u8,
        start: bool,
        loej: bool,
    ) -> &[u8] {
        self.cdb[0] = opcode::START_STOP_UNIT;
        self.cdb[1] = if immed { 0x01 } else { 0x00 };
        let mut byte4 = (power_condition & 0x0F) << 4;
        if loej {
            byte4 |= 0x02;
        }
        if start {
            byte4 |= 0x01;
        }
        self.cdb[4] = byte4;
        self.len = 6;
        &self.cdb[..self.len]
    }

    /// Build REPORT LUNS command
    pub fn report_luns(&mut self, select_report: u8, allocation_length: u32) -> &[u8] {
        self.cdb[0] = opcode::REPORT_LUNS;
        self.cdb[2] = select_report;
        self.cdb[6] = (allocation_length >> 24) as u8;
        self.cdb[7] = (allocation_length >> 16) as u8;
        self.cdb[8] = (allocation_length >> 8) as u8;
        self.cdb[9] = allocation_length as u8;
        self.len = 12;
        &self.cdb[..self.len]
    }
}

impl Default for CdbBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// ISCSI STRUCTURES
// =============================================================================

/// iSCSI PDU operation codes
pub mod iscsi_opcode {
    // Initiator opcodes
    /// NOP-Out
    pub const NOP_OUT: u8 = 0x00;
    /// SCSI Command
    pub const SCSI_CMD: u8 = 0x01;
    /// SCSI Task Management Request
    pub const TASK_MGT_REQ: u8 = 0x02;
    /// Login Request
    pub const LOGIN_REQ: u8 = 0x03;
    /// Text Request
    pub const TEXT_REQ: u8 = 0x04;
    /// SCSI Data-Out
    pub const SCSI_DATA_OUT: u8 = 0x05;
    /// Logout Request
    pub const LOGOUT_REQ: u8 = 0x06;
    /// SNACK Request
    pub const SNACK_REQ: u8 = 0x10;

    // Target opcodes
    /// NOP-In
    pub const NOP_IN: u8 = 0x20;
    /// SCSI Response
    pub const SCSI_RSP: u8 = 0x21;
    /// SCSI Task Management Response
    pub const TASK_MGT_RSP: u8 = 0x22;
    /// Login Response
    pub const LOGIN_RSP: u8 = 0x23;
    /// Text Response
    pub const TEXT_RSP: u8 = 0x24;
    /// SCSI Data-In
    pub const SCSI_DATA_IN: u8 = 0x25;
    /// Logout Response
    pub const LOGOUT_RSP: u8 = 0x26;
    /// Ready To Transfer
    pub const R2T: u8 = 0x31;
    /// Asynchronous Message
    pub const ASYNC_MSG: u8 = 0x32;
    /// Reject
    pub const REJECT: u8 = 0x3F;
}

/// iSCSI Basic Header Segment (48 bytes)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IscsiBasicHeader {
    /// Opcode
    pub opcode: u8,
    /// Flags
    pub flags: u8,
    /// Reserved or opcode-specific
    pub reserved1: [u8; 2],
    /// Total AHS length
    pub total_ahs_len: u8,
    /// Data segment length (3 bytes)
    pub data_segment_len: [u8; 3],
    /// LUN or opcode-specific
    pub lun: [u8; 8],
    /// Initiator task tag
    pub itt: u32,
    /// Opcode-specific fields
    pub opcode_specific: [u8; 28],
}

impl IscsiBasicHeader {
    /// Get data segment length
    pub fn data_segment_length(&self) -> u32 {
        ((self.data_segment_len[0] as u32) << 16)
            | ((self.data_segment_len[1] as u32) << 8)
            | (self.data_segment_len[2] as u32)
    }

    /// Set data segment length
    pub fn set_data_segment_length(&mut self, len: u32) {
        self.data_segment_len[0] = ((len >> 16) & 0xFF) as u8;
        self.data_segment_len[1] = ((len >> 8) & 0xFF) as u8;
        self.data_segment_len[2] = (len & 0xFF) as u8;
    }

    /// Check if final PDU
    pub const fn is_final(&self) -> bool {
        (self.flags & 0x80) != 0
    }

    /// Check if immediate
    pub const fn is_immediate(&self) -> bool {
        (self.opcode & 0x40) != 0
    }
}

// =============================================================================
// SAS STRUCTURES
// =============================================================================

/// SAS address type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct SasAddress(pub [u8; 8]);

impl SasAddress {
    /// Create from bytes
    pub const fn from_bytes(bytes: [u8; 8]) -> Self {
        Self(bytes)
    }

    /// Create from u64
    pub const fn from_u64(val: u64) -> Self {
        Self(val.to_be_bytes())
    }

    /// Convert to u64
    pub fn to_u64(&self) -> u64 {
        u64::from_be_bytes(self.0)
    }

    /// Check if valid
    pub fn is_valid(&self) -> bool {
        self.to_u64() != 0
    }
}

impl fmt::Display for SasAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5], self.0[6], self.0[7]
        )
    }
}

/// SAS device types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SasDeviceType {
    /// No device attached
    NoDevice       = 0,
    /// End device
    EndDevice      = 1,
    /// Expander device (edge)
    EdgeExpander   = 2,
    /// Expander device (fanout)
    FanoutExpander = 3,
}

/// SAS link rate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SasLinkRate {
    /// Unknown
    Unknown      = 0,
    /// Disabled
    Disabled     = 1,
    /// Reset problem
    ResetProblem = 2,
    /// Spin-up hold
    SpinupHold   = 3,
    /// Port selector
    PortSelector = 4,
    /// 1.5 Gbps
    Rate1_5      = 8,
    /// 3.0 Gbps
    Rate3_0      = 9,
    /// 6.0 Gbps
    Rate6_0      = 10,
    /// 12.0 Gbps
    Rate12_0     = 11,
    /// 22.5 Gbps
    Rate22_5     = 12,
}

impl SasLinkRate {
    /// Get speed in Mbps
    pub const fn speed_mbps(&self) -> u32 {
        match self {
            SasLinkRate::Rate1_5 => 1500,
            SasLinkRate::Rate3_0 => 3000,
            SasLinkRate::Rate6_0 => 6000,
            SasLinkRate::Rate12_0 => 12000,
            SasLinkRate::Rate22_5 => 22500,
            _ => 0,
        }
    }
}

// =============================================================================
// ERROR TYPES
// =============================================================================

/// SCSI error types
#[derive(Debug, Clone, Copy)]
pub enum ScsiError {
    /// Command timeout
    Timeout,
    /// Transport error
    TransportError,
    /// Check condition (with sense data)
    CheckCondition {
        /// Sense key
        sense_key: SenseKey,
        /// ASC
        asc: u8,
        /// ASCQ
        ascq: u8,
    },
    /// Busy
    Busy,
    /// Reservation conflict
    ReservationConflict,
    /// Task set full
    TaskSetFull,
    /// Invalid LUN
    InvalidLun,
    /// Medium not present
    MediumNotPresent,
    /// Write protected
    WriteProtected,
    /// Internal error
    InternalError,
}

impl fmt::Display for ScsiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScsiError::Timeout => write!(f, "Command timeout"),
            ScsiError::TransportError => write!(f, "Transport error"),
            ScsiError::CheckCondition {
                sense_key,
                asc,
                ascq,
            } => {
                write!(
                    f,
                    "Check condition: {} (ASC={:#04X}, ASCQ={:#04X})",
                    sense_key.description(),
                    asc,
                    ascq
                )
            },
            ScsiError::Busy => write!(f, "Device busy"),
            ScsiError::ReservationConflict => write!(f, "Reservation conflict"),
            ScsiError::TaskSetFull => write!(f, "Task set full"),
            ScsiError::InvalidLun => write!(f, "Invalid LUN"),
            ScsiError::MediumNotPresent => write!(f, "Medium not present"),
            ScsiError::WriteProtected => write!(f, "Write protected"),
            ScsiError::InternalError => write!(f, "Internal error"),
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
    fn test_scsi_status() {
        assert!(ScsiStatus::Good.is_good());
        assert!(!ScsiStatus::CheckCondition.is_good());
        assert!(ScsiStatus::CheckCondition.has_sense());
    }

    #[test]
    fn test_sense_key() {
        assert_eq!(SenseKey::from_byte(0x00), SenseKey::NoSense);
        assert_eq!(SenseKey::from_byte(0x05), SenseKey::IllegalRequest);
        assert_eq!(SenseKey::from_byte(0x03), SenseKey::MediumError);
    }

    #[test]
    fn test_device_type() {
        assert!(DeviceType::DirectAccess.is_block_device());
        assert!(!DeviceType::CdDvd.is_block_device());
        assert_eq!(DeviceType::from_byte(0x00), DeviceType::DirectAccess);
    }

    #[test]
    fn test_read_capacity_10() {
        let data = [0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00];
        let cap = ReadCapacity10::from_bytes(&data).unwrap();
        assert_eq!(cap.last_lba(), 0x00010000);
        assert_eq!(cap.block_length(), 512);
        assert_eq!(cap.total_blocks(), 0x00010001);
    }

    #[test]
    fn test_cdb_builder() {
        let mut builder = CdbBuilder::new();
        let cdb = builder.test_unit_ready();
        assert_eq!(cdb.len(), 6);
        assert_eq!(cdb[0], opcode::TEST_UNIT_READY);
    }

    #[test]
    fn test_sas_address() {
        let addr = SasAddress::from_u64(0x5000C5000BADF00D);
        assert!(addr.is_valid());
        assert_eq!(addr.to_u64(), 0x5000C5000BADF00D);
    }

    #[test]
    fn test_sas_link_rate() {
        assert_eq!(SasLinkRate::Rate6_0.speed_mbps(), 6000);
        assert_eq!(SasLinkRate::Rate12_0.speed_mbps(), 12000);
    }
}
