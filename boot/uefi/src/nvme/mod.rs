//! NVMe (Non-Volatile Memory Express) Support for Helix UEFI Bootloader
//!
//! This module provides comprehensive NVMe controller and namespace support
//! for high-performance SSD access in the UEFI boot environment.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         NVMe Protocol Stack                             │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Block Layer    │  Read  │  Write  │  Flush  │  Trim  │  Secure Erase  │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Namespace      │  Identify  │  Format  │  Attach  │  Detach          │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Controller     │  Admin Queue  │  I/O Queues  │  Interrupts          │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Hardware       │  PCIe  │  Registers  │  Doorbells                    │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Features
//!
//! - NVMe 1.4 specification support
//! - Admin and I/O queue management
//! - Namespace identification and management
//! - Read/write/flush commands
//! - Dataset management (TRIM/deallocate)
//! - Power state management
//! - Health monitoring (SMART)

use core::fmt;

// =============================================================================
// NVME CONSTANTS
// =============================================================================

/// NVMe controller register base
pub const NVME_REG_BASE: usize = 0;

/// Maximum number of queues
pub const NVME_MAX_QUEUES: usize = 65535;

/// Maximum queue entries (power of 2)
pub const NVME_MAX_QUEUE_ENTRIES: u16 = 4096;

/// Default queue entries
pub const NVME_DEFAULT_QUEUE_ENTRIES: u16 = 256;

/// Admin queue ID
pub const NVME_ADMIN_QUEUE_ID: u16 = 0;

/// Maximum data transfer size (MDTS) - 1MB default
pub const NVME_MAX_DATA_TRANSFER: usize = 1024 * 1024;

/// Sector size (common)
pub const NVME_SECTOR_SIZE: usize = 512;

/// NVMe command timeout (milliseconds)
pub const NVME_COMMAND_TIMEOUT_MS: u32 = 30000;

// =============================================================================
// NVME CONTROLLER REGISTERS
// =============================================================================

/// NVMe controller register offsets
pub mod regs {
    /// Controller Capabilities (CAP)
    pub const CAP: usize = 0x0000;
    /// Version (VS)
    pub const VS: usize = 0x0008;
    /// Interrupt Mask Set (INTMS)
    pub const INTMS: usize = 0x000C;
    /// Interrupt Mask Clear (INTMC)
    pub const INTMC: usize = 0x0010;
    /// Controller Configuration (CC)
    pub const CC: usize = 0x0014;
    /// Reserved
    pub const RESERVED: usize = 0x0018;
    /// Controller Status (CSTS)
    pub const CSTS: usize = 0x001C;
    /// NVM Subsystem Reset (NSSR)
    pub const NSSR: usize = 0x0020;
    /// Admin Queue Attributes (AQA)
    pub const AQA: usize = 0x0024;
    /// Admin Submission Queue Base Address (ASQ)
    pub const ASQ: usize = 0x0028;
    /// Admin Completion Queue Base Address (ACQ)
    pub const ACQ: usize = 0x0030;
    /// Controller Memory Buffer Location (CMBLOC)
    pub const CMBLOC: usize = 0x0038;
    /// Controller Memory Buffer Size (CMBSZ)
    pub const CMBSZ: usize = 0x003C;
    /// Boot Partition Information (BPINFO)
    pub const BPINFO: usize = 0x0040;
    /// Boot Partition Read Select (BPRSEL)
    pub const BPRSEL: usize = 0x0044;
    /// Boot Partition Memory Buffer Location (BPMBL)
    pub const BPMBL: usize = 0x0048;
    /// Controller Memory Buffer Memory Space Control (CMBMSC)
    pub const CMBMSC: usize = 0x0050;
    /// Controller Memory Buffer Status (CMBSTS)
    pub const CMBSTS: usize = 0x0058;
    /// Submission Queue 0 Tail Doorbell (Admin)
    pub const SQ0TDBL: usize = 0x1000;
}

/// Controller Capabilities (CAP) register bits
pub mod cap {
    /// Maximum Queue Entries Supported (bits 0-15)
    pub const MQES_SHIFT: u64 = 0;
    pub const MQES_MASK: u64 = 0xFFFF;
    /// Contiguous Queues Required (bit 16)
    pub const CQR: u64 = 1 << 16;
    /// Arbitration Mechanism Supported (bits 17-18)
    pub const AMS_SHIFT: u64 = 17;
    pub const AMS_MASK: u64 = 0x3;
    /// Timeout (bits 24-31)
    pub const TO_SHIFT: u64 = 24;
    pub const TO_MASK: u64 = 0xFF;
    /// Doorbell Stride (bits 32-35)
    pub const DSTRD_SHIFT: u64 = 32;
    pub const DSTRD_MASK: u64 = 0xF;
    /// NVM Subsystem Reset Supported (bit 36)
    pub const NSSRS: u64 = 1 << 36;
    /// Command Sets Supported (bits 37-44)
    pub const CSS_SHIFT: u64 = 37;
    pub const CSS_MASK: u64 = 0xFF;
    /// NVM Command Set (bit 37)
    pub const CSS_NVM: u64 = 1 << 37;
    /// Boot Partition Support (bit 45)
    pub const BPS: u64 = 1 << 45;
    /// Memory Page Size Minimum (bits 48-51)
    pub const MPSMIN_SHIFT: u64 = 48;
    pub const MPSMIN_MASK: u64 = 0xF;
    /// Memory Page Size Maximum (bits 52-55)
    pub const MPSMAX_SHIFT: u64 = 52;
    pub const MPSMAX_MASK: u64 = 0xF;
    /// Persistent Memory Region Supported (bit 56)
    pub const PMRS: u64 = 1 << 56;
    /// Controller Memory Buffer Supported (bit 57)
    pub const CMBS: u64 = 1 << 57;
}

/// Controller Configuration (CC) register bits
pub mod cc {
    /// Enable (bit 0)
    pub const EN: u32 = 1 << 0;
    /// I/O Command Set Selected (bits 4-6)
    pub const CSS_SHIFT: u32 = 4;
    pub const CSS_MASK: u32 = 0x7;
    /// Memory Page Size (bits 7-10)
    pub const MPS_SHIFT: u32 = 7;
    pub const MPS_MASK: u32 = 0xF;
    /// Arbitration Mechanism Selected (bits 11-13)
    pub const AMS_SHIFT: u32 = 11;
    pub const AMS_MASK: u32 = 0x7;
    /// Shutdown Notification (bits 14-15)
    pub const SHN_SHIFT: u32 = 14;
    pub const SHN_MASK: u32 = 0x3;
    /// I/O Submission Queue Entry Size (bits 16-19)
    pub const IOSQES_SHIFT: u32 = 16;
    pub const IOSQES_MASK: u32 = 0xF;
    /// I/O Completion Queue Entry Size (bits 20-23)
    pub const IOCQES_SHIFT: u32 = 20;
    pub const IOCQES_MASK: u32 = 0xF;
}

/// Shutdown notification values
pub mod shn {
    /// No notification
    pub const NONE: u32 = 0;
    /// Normal shutdown
    pub const NORMAL: u32 = 1;
    /// Abrupt shutdown
    pub const ABRUPT: u32 = 2;
}

/// Controller Status (CSTS) register bits
pub mod csts {
    /// Ready (bit 0)
    pub const RDY: u32 = 1 << 0;
    /// Controller Fatal Status (bit 1)
    pub const CFS: u32 = 1 << 1;
    /// Shutdown Status (bits 2-3)
    pub const SHST_SHIFT: u32 = 2;
    pub const SHST_MASK: u32 = 0x3;
    /// NVM Subsystem Reset Occurred (bit 4)
    pub const NSSRO: u32 = 1 << 4;
    /// Processing Paused (bit 5)
    pub const PP: u32 = 1 << 5;
}

/// Shutdown status values
pub mod shst {
    /// Normal operation
    pub const NORMAL: u32 = 0;
    /// Shutdown processing occurring
    pub const PROCESSING: u32 = 1;
    /// Shutdown processing complete
    pub const COMPLETE: u32 = 2;
}

// =============================================================================
// NVME COMMAND OPCODES
// =============================================================================

/// Admin command opcodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AdminOpcode {
    /// Delete I/O Submission Queue
    DeleteIoSq           = 0x00,
    /// Create I/O Submission Queue
    CreateIoSq           = 0x01,
    /// Get Log Page
    GetLogPage           = 0x02,
    /// Delete I/O Completion Queue
    DeleteIoCq           = 0x04,
    /// Create I/O Completion Queue
    CreateIoCq           = 0x05,
    /// Identify
    Identify             = 0x06,
    /// Abort
    Abort                = 0x08,
    /// Set Features
    SetFeatures          = 0x09,
    /// Get Features
    GetFeatures          = 0x0A,
    /// Asynchronous Event Request
    AsyncEventReq        = 0x0C,
    /// Namespace Management
    NsManagement         = 0x0D,
    /// Firmware Commit
    FirmwareCommit       = 0x10,
    /// Firmware Image Download
    FirmwareDownload     = 0x11,
    /// Device Self-test
    DeviceSelfTest       = 0x14,
    /// Namespace Attachment
    NsAttachment         = 0x15,
    /// Keep Alive
    KeepAlive            = 0x18,
    /// Directive Send
    DirectiveSend        = 0x19,
    /// Directive Receive
    DirectiveReceive     = 0x1A,
    /// Virtualization Management
    VirtualizationMgmt   = 0x1C,
    /// NVMe-MI Send
    NvmeMiSend           = 0x1D,
    /// NVMe-MI Receive
    NvmeMiReceive        = 0x1E,
    /// Doorbell Buffer Config
    DoorbellBufferConfig = 0x7C,
    /// Format NVM
    FormatNvm            = 0x80,
    /// Security Send
    SecuritySend         = 0x81,
    /// Security Receive
    SecurityReceive      = 0x82,
    /// Sanitize
    Sanitize             = 0x84,
    /// Get LBA Status
    GetLbaStatus         = 0x86,
}

/// NVM command opcodes (I/O commands)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NvmOpcode {
    /// Flush
    Flush               = 0x00,
    /// Write
    Write               = 0x01,
    /// Read
    Read                = 0x02,
    /// Write Uncorrectable
    WriteUncorrectable  = 0x04,
    /// Compare
    Compare             = 0x05,
    /// Write Zeroes
    WriteZeroes         = 0x08,
    /// Dataset Management
    DatasetManagement   = 0x09,
    /// Verify
    Verify              = 0x0C,
    /// Reservation Register
    ReservationRegister = 0x0D,
    /// Reservation Report
    ReservationReport   = 0x0E,
    /// Reservation Acquire
    ReservationAcquire  = 0x11,
    /// Reservation Release
    ReservationRelease  = 0x15,
    /// Copy
    Copy                = 0x19,
}

// =============================================================================
// NVME COMMAND STRUCTURES
// =============================================================================

/// NVMe Submission Queue Entry (64 bytes)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct NvmeCommand {
    /// Command dword 0 (opcode, fused, psdt, cid)
    pub cdw0: u32,
    /// Namespace ID
    pub nsid: u32,
    /// Command dword 2
    pub cdw2: u32,
    /// Command dword 3
    pub cdw3: u32,
    /// Metadata pointer
    pub mptr: u64,
    /// PRP entry 1 or SGL
    pub prp1: u64,
    /// PRP entry 2 or SGL
    pub prp2: u64,
    /// Command dword 10
    pub cdw10: u32,
    /// Command dword 11
    pub cdw11: u32,
    /// Command dword 12
    pub cdw12: u32,
    /// Command dword 13
    pub cdw13: u32,
    /// Command dword 14
    pub cdw14: u32,
    /// Command dword 15
    pub cdw15: u32,
}

impl NvmeCommand {
    /// Create empty command
    pub const fn new() -> Self {
        Self {
            cdw0: 0,
            nsid: 0,
            cdw2: 0,
            cdw3: 0,
            mptr: 0,
            prp1: 0,
            prp2: 0,
            cdw10: 0,
            cdw11: 0,
            cdw12: 0,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        }
    }

    /// Set opcode and command ID
    pub fn set_opcode(&mut self, opcode: u8, cid: u16) {
        self.cdw0 = (opcode as u32) | ((cid as u32) << 16);
    }

    /// Get command ID
    pub const fn command_id(&self) -> u16 {
        (self.cdw0 >> 16) as u16
    }

    /// Get opcode
    pub const fn opcode(&self) -> u8 {
        self.cdw0 as u8
    }

    /// Create Identify Controller command
    pub fn identify_controller(cid: u16, buffer: u64) -> Self {
        let mut command = Self::new();
        command.set_opcode(AdminOpcode::Identify as u8, cid);
        command.prp1 = buffer;
        command.cdw10 = 1; // Controller
        command
    }

    /// Create Identify Namespace command
    pub fn identify_namespace(cid: u16, nsid: u32, buffer: u64) -> Self {
        let mut command = Self::new();
        command.set_opcode(AdminOpcode::Identify as u8, cid);
        command.nsid = nsid;
        command.prp1 = buffer;
        command.cdw10 = 0; // Namespace
        command
    }

    /// Create Identify Active Namespace List command
    pub fn identify_active_ns_list(cid: u16, start_nsid: u32, buffer: u64) -> Self {
        let mut command = Self::new();
        command.set_opcode(AdminOpcode::Identify as u8, cid);
        command.nsid = start_nsid;
        command.prp1 = buffer;
        command.cdw10 = 2; // Active Namespace ID List
        command
    }

    /// Create Create I/O Completion Queue command
    pub fn create_io_cq(cid: u16, qid: u16, size: u16, buffer: u64, iv: u16) -> Self {
        let mut command = Self::new();
        command.set_opcode(AdminOpcode::CreateIoCq as u8, cid);
        command.prp1 = buffer;
        command.cdw10 = (((size - 1) as u32) << 16) | (qid as u32);
        command.cdw11 = 1 | ((iv as u32) << 16); // Physically contiguous, interrupt enabled
        command
    }

    /// Create Create I/O Submission Queue command
    pub fn create_io_sq(cid: u16, qid: u16, size: u16, buffer: u64, comp_qid: u16) -> Self {
        let mut command = Self::new();
        command.set_opcode(AdminOpcode::CreateIoSq as u8, cid);
        command.prp1 = buffer;
        command.cdw10 = (((size - 1) as u32) << 16) | (qid as u32);
        command.cdw11 = 1 | ((comp_qid as u32) << 16); // Physically contiguous
        command
    }

    /// Create Delete I/O Submission Queue command
    pub fn delete_io_sq(cid: u16, qid: u16) -> Self {
        let mut command = Self::new();
        command.set_opcode(AdminOpcode::DeleteIoSq as u8, cid);
        command.cdw10 = qid as u32;
        command
    }

    /// Create Delete I/O Completion Queue command
    pub fn delete_io_cq(cid: u16, qid: u16) -> Self {
        let mut command = Self::new();
        command.set_opcode(AdminOpcode::DeleteIoCq as u8, cid);
        command.cdw10 = qid as u32;
        command
    }

    /// Create Read command
    pub fn read(cid: u16, nsid: u32, lba: u64, num_blocks: u16, prp1: u64, prp2: u64) -> Self {
        let mut command = Self::new();
        command.set_opcode(NvmOpcode::Read as u8, cid);
        command.nsid = nsid;
        command.prp1 = prp1;
        command.prp2 = prp2;
        command.cdw10 = lba as u32;
        command.cdw11 = (lba >> 32) as u32;
        command.cdw12 = (num_blocks - 1) as u32;
        command
    }

    /// Create Write command
    pub fn write(cid: u16, nsid: u32, lba: u64, num_blocks: u16, prp1: u64, prp2: u64) -> Self {
        let mut command = Self::new();
        command.set_opcode(NvmOpcode::Write as u8, cid);
        command.nsid = nsid;
        command.prp1 = prp1;
        command.prp2 = prp2;
        command.cdw10 = lba as u32;
        command.cdw11 = (lba >> 32) as u32;
        command.cdw12 = (num_blocks - 1) as u32;
        command
    }

    /// Create Flush command
    pub fn flush(cid: u16, nsid: u32) -> Self {
        let mut command = Self::new();
        command.set_opcode(NvmOpcode::Flush as u8, cid);
        command.nsid = nsid;
        command
    }

    /// Create Get Log Page command
    pub fn get_log_page(cid: u16, lid: u8, buffer: u64, size: u32) -> Self {
        let mut command = Self::new();
        command.set_opcode(AdminOpcode::GetLogPage as u8, cid);
        command.prp1 = buffer;
        command.cdw10 = (lid as u32) | ((size / 4 - 1) << 16);
        command.cdw11 = 0;
        command.cdw12 = 0;
        command.cdw13 = 0;
        command
    }

    /// Create Set Features command
    pub fn set_features(cid: u16, fid: u8, value: u32) -> Self {
        let mut command = Self::new();
        command.set_opcode(AdminOpcode::SetFeatures as u8, cid);
        command.cdw10 = fid as u32;
        command.cdw11 = value;
        command
    }

    /// Create Get Features command
    pub fn get_features(cid: u16, fid: u8) -> Self {
        let mut command = Self::new();
        command.set_opcode(AdminOpcode::GetFeatures as u8, cid);
        command.cdw10 = fid as u32;
        command
    }
}

impl Default for NvmeCommand {
    fn default() -> Self {
        Self::new()
    }
}

/// NVMe Completion Queue Entry (16 bytes)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct NvmeCompletion {
    /// Command specific
    pub result: u64,
    /// Submission queue head pointer
    pub sq_head: u16,
    /// Submission queue identifier
    pub sq_id: u16,
    /// Command identifier
    pub cid: u16,
    /// Status and phase
    pub status: u16,
}

impl NvmeCompletion {
    /// Create empty completion
    pub const fn new() -> Self {
        Self {
            result: 0,
            sq_head: 0,
            sq_id: 0,
            cid: 0,
            status: 0,
        }
    }

    /// Get phase tag
    pub const fn phase(&self) -> bool {
        (self.status & 1) != 0
    }

    /// Get status code type
    pub const fn status_code_type(&self) -> u8 {
        ((self.status >> 9) & 0x7) as u8
    }

    /// Get status code
    pub const fn status_code(&self) -> u8 {
        ((self.status >> 1) & 0xFF) as u8
    }

    /// Check if more data available
    pub const fn more(&self) -> bool {
        (self.status & (1 << 14)) != 0
    }

    /// Check if do not retry
    pub const fn do_not_retry(&self) -> bool {
        (self.status & (1 << 15)) != 0
    }

    /// Check if command succeeded
    pub const fn is_success(&self) -> bool {
        self.status_code_type() == 0 && self.status_code() == 0
    }

    /// Get error as NvmeStatus
    pub fn status(&self) -> NvmeStatus {
        NvmeStatus::from_completion(self)
    }
}

impl Default for NvmeCompletion {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// NVME STATUS CODES
// =============================================================================

/// NVMe status code type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StatusCodeType {
    /// Generic command status
    Generic         = 0,
    /// Command specific status
    CommandSpecific = 1,
    /// Media and data integrity errors
    MediaError      = 2,
    /// Path related status
    PathRelated     = 3,
    /// Vendor specific
    VendorSpecific  = 7,
}

/// Generic status codes
pub mod generic_status {
    /// Successful completion
    pub const SUCCESS: u8 = 0x00;
    /// Invalid command opcode
    pub const INVALID_OPCODE: u8 = 0x01;
    /// Invalid field in command
    pub const INVALID_FIELD: u8 = 0x02;
    /// Command ID conflict
    pub const COMMAND_ID_CONFLICT: u8 = 0x03;
    /// Data transfer error
    pub const DATA_TRANSFER_ERROR: u8 = 0x04;
    /// Abort due to power loss
    pub const POWER_LOSS_ABORT: u8 = 0x05;
    /// Internal error
    pub const INTERNAL_ERROR: u8 = 0x06;
    /// Command abort requested
    pub const ABORT_REQUESTED: u8 = 0x07;
    /// Abort due to SQ deletion
    pub const ABORT_SQ_DELETED: u8 = 0x08;
    /// Abort due to fused fail
    pub const ABORT_FUSED_FAIL: u8 = 0x09;
    /// Abort due to missing fused
    pub const ABORT_MISSING_FUSED: u8 = 0x0A;
    /// Invalid namespace or format
    pub const INVALID_NAMESPACE: u8 = 0x0B;
    /// Command sequence error
    pub const COMMAND_SEQUENCE_ERROR: u8 = 0x0C;
    /// Invalid SGL segment
    pub const INVALID_SGL_SEGMENT: u8 = 0x0D;
    /// Invalid SGL count
    pub const INVALID_SGL_COUNT: u8 = 0x0E;
    /// Invalid data length
    pub const INVALID_DATA_LENGTH: u8 = 0x0F;
    /// Invalid metadata length
    pub const INVALID_METADATA_LENGTH: u8 = 0x10;
    /// Invalid SGL descriptor
    pub const INVALID_SGL_DESCRIPTOR: u8 = 0x11;
    /// Invalid controller memory buffer use
    pub const INVALID_CMB_USE: u8 = 0x12;
    /// Invalid PRP offset
    pub const INVALID_PRP_OFFSET: u8 = 0x13;
    /// Atomic write unit exceeded
    pub const ATOMIC_WRITE_EXCEEDED: u8 = 0x14;
    /// Operation denied
    pub const OPERATION_DENIED: u8 = 0x15;
    /// Invalid SGL offset
    pub const INVALID_SGL_OFFSET: u8 = 0x16;
    /// Host identifier inconsistent format
    pub const HOST_ID_FORMAT: u8 = 0x18;
    /// Keep alive timer expired
    pub const KEEP_ALIVE_EXPIRED: u8 = 0x19;
    /// Keep alive timeout invalid
    pub const KEEP_ALIVE_INVALID: u8 = 0x1A;
    /// Abort preempted
    pub const ABORT_PREEMPTED: u8 = 0x1B;
    /// Sanitize failed
    pub const SANITIZE_FAILED: u8 = 0x1C;
    /// Sanitize in progress
    pub const SANITIZE_IN_PROGRESS: u8 = 0x1D;
    /// SGL data block too small
    pub const SGL_DATA_BLOCK_SMALL: u8 = 0x1E;
    /// Command not supported
    pub const COMMAND_NOT_SUPPORTED: u8 = 0x1F;
    /// LBA out of range
    pub const LBA_OUT_OF_RANGE: u8 = 0x80;
    /// Capacity exceeded
    pub const CAPACITY_EXCEEDED: u8 = 0x81;
    /// Namespace not ready
    pub const NAMESPACE_NOT_READY: u8 = 0x82;
    /// Reservation conflict
    pub const RESERVATION_CONFLICT: u8 = 0x83;
    /// Format in progress
    pub const FORMAT_IN_PROGRESS: u8 = 0x84;
}

/// Command specific status codes
pub mod cmd_status {
    /// Completion queue invalid
    pub const CQ_INVALID: u8 = 0x00;
    /// Invalid queue identifier
    pub const QID_INVALID: u8 = 0x01;
    /// Invalid queue size
    pub const QUEUE_SIZE_INVALID: u8 = 0x02;
    /// Abort command limit exceeded
    pub const ABORT_LIMIT_EXCEEDED: u8 = 0x03;
    /// Asynchronous event request limit exceeded
    pub const ASYNC_EVENT_LIMIT: u8 = 0x05;
    /// Invalid firmware slot
    pub const INVALID_FIRMWARE_SLOT: u8 = 0x06;
    /// Invalid firmware image
    pub const INVALID_FIRMWARE_IMAGE: u8 = 0x07;
    /// Invalid interrupt vector
    pub const INVALID_INTERRUPT_VECTOR: u8 = 0x08;
    /// Invalid log page
    pub const INVALID_LOG_PAGE: u8 = 0x09;
    /// Invalid format
    pub const INVALID_FORMAT: u8 = 0x0A;
    /// Firmware activation requires reset
    pub const FIRMWARE_NEEDS_RESET: u8 = 0x0B;
    /// Invalid queue deletion
    pub const INVALID_QUEUE_DELETION: u8 = 0x0C;
    /// Feature identifier not savable
    pub const FEATURE_NOT_SAVABLE: u8 = 0x0D;
    /// Feature not changeable
    pub const FEATURE_NOT_CHANGEABLE: u8 = 0x0E;
    /// Feature not namespace specific
    pub const FEATURE_NOT_NS_SPECIFIC: u8 = 0x0F;
    /// Firmware activation requires NVM subsystem reset
    pub const FW_NEEDS_SUBSYSTEM_RESET: u8 = 0x10;
    /// Firmware activation requires controller reset
    pub const FW_NEEDS_CONTROLLER_RESET: u8 = 0x11;
    /// Firmware activation requires maximum time violation
    pub const FW_MAX_TIME_VIOLATION: u8 = 0x12;
    /// Firmware activation prohibited
    pub const FW_ACTIVATION_PROHIBITED: u8 = 0x13;
    /// Overlapping range
    pub const OVERLAPPING_RANGE: u8 = 0x14;
    /// Namespace insufficient capacity
    pub const NS_INSUFFICIENT_CAPACITY: u8 = 0x15;
    /// Namespace ID unavailable
    pub const NS_ID_UNAVAILABLE: u8 = 0x16;
    /// Namespace already attached
    pub const NS_ALREADY_ATTACHED: u8 = 0x18;
    /// Namespace is private
    pub const NS_IS_PRIVATE: u8 = 0x19;
    /// Namespace not attached
    pub const NS_NOT_ATTACHED: u8 = 0x1A;
    /// Thin provisioning not supported
    pub const THIN_NOT_SUPPORTED: u8 = 0x1B;
    /// Controller list invalid
    pub const CONTROLLER_LIST_INVALID: u8 = 0x1C;
    /// Device self-test in progress
    pub const SELF_TEST_IN_PROGRESS: u8 = 0x1D;
    /// Boot partition write prohibited
    pub const BOOT_PARTITION_WRITE_PROHIBITED: u8 = 0x1E;
    /// Invalid controller identifier
    pub const INVALID_CONTROLLER_ID: u8 = 0x1F;
    /// Invalid secondary controller state
    pub const INVALID_SECONDARY_STATE: u8 = 0x20;
    /// Invalid number of controller resources
    pub const INVALID_CONTROLLER_RESOURCES: u8 = 0x21;
    /// Invalid resource identifier
    pub const INVALID_RESOURCE_ID: u8 = 0x22;
}

/// Media error status codes
pub mod media_status {
    /// Write fault
    pub const WRITE_FAULT: u8 = 0x80;
    /// Unrecovered read error
    pub const UNRECOVERED_READ: u8 = 0x81;
    /// End-to-end guard check error
    pub const GUARD_CHECK_ERROR: u8 = 0x82;
    /// End-to-end application tag check error
    pub const APP_TAG_CHECK_ERROR: u8 = 0x83;
    /// End-to-end reference tag check error
    pub const REF_TAG_CHECK_ERROR: u8 = 0x84;
    /// Compare failure
    pub const COMPARE_FAILURE: u8 = 0x85;
    /// Access denied
    pub const ACCESS_DENIED: u8 = 0x86;
    /// Deallocated or unwritten logical block
    pub const DEALLOCATED_LBA: u8 = 0x87;
}

/// NVMe status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NvmeStatus {
    /// Status code type
    pub sct: StatusCodeType,
    /// Status code
    pub sc: u8,
    /// More information available
    pub more: bool,
    /// Do not retry
    pub dnr: bool,
}

impl NvmeStatus {
    /// Create from completion entry
    pub fn from_completion(cqe: &NvmeCompletion) -> Self {
        let sct = match cqe.status_code_type() {
            0 => StatusCodeType::Generic,
            1 => StatusCodeType::CommandSpecific,
            2 => StatusCodeType::MediaError,
            3 => StatusCodeType::PathRelated,
            _ => StatusCodeType::VendorSpecific,
        };

        Self {
            sct,
            sc: cqe.status_code(),
            more: cqe.more(),
            dnr: cqe.do_not_retry(),
        }
    }

    /// Check if success
    pub const fn is_success(&self) -> bool {
        matches!(self.sct, StatusCodeType::Generic) && self.sc == 0
    }

    /// Get error message
    pub fn message(&self) -> &'static str {
        match self.sct {
            StatusCodeType::Generic => match self.sc {
                generic_status::SUCCESS => "Success",
                generic_status::INVALID_OPCODE => "Invalid opcode",
                generic_status::INVALID_FIELD => "Invalid field",
                generic_status::COMMAND_ID_CONFLICT => "Command ID conflict",
                generic_status::DATA_TRANSFER_ERROR => "Data transfer error",
                generic_status::INTERNAL_ERROR => "Internal error",
                generic_status::ABORT_REQUESTED => "Abort requested",
                generic_status::LBA_OUT_OF_RANGE => "LBA out of range",
                generic_status::NAMESPACE_NOT_READY => "Namespace not ready",
                _ => "Unknown generic error",
            },
            StatusCodeType::CommandSpecific => "Command specific error",
            StatusCodeType::MediaError => "Media error",
            StatusCodeType::PathRelated => "Path related error",
            StatusCodeType::VendorSpecific => "Vendor specific error",
        }
    }
}

impl fmt::Display for NvmeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}

// =============================================================================
// NVME IDENTIFY STRUCTURES
// =============================================================================

/// NVMe Identify Controller data structure (4096 bytes)
#[derive(Clone)]
#[repr(C)]
pub struct IdentifyController {
    /// PCI Vendor ID
    pub vid: u16,
    /// PCI Subsystem Vendor ID
    pub ssvid: u16,
    /// Serial Number
    pub sn: [u8; 20],
    /// Model Number
    pub mn: [u8; 40],
    /// Firmware Revision
    pub fr: [u8; 8],
    /// Recommended Arbitration Burst
    pub rab: u8,
    /// IEEE OUI Identifier
    pub ieee: [u8; 3],
    /// Controller Multi-Path I/O and Namespace Sharing Capabilities
    pub cmic: u8,
    /// Maximum Data Transfer Size
    pub mdts: u8,
    /// Controller ID
    pub cntlid: u16,
    /// Version
    pub ver: u32,
    /// RTD3 Resume Latency
    pub rtd3r: u32,
    /// RTD3 Entry Latency
    pub rtd3e: u32,
    /// Optional Asynchronous Events Supported
    pub oaes: u32,
    /// Controller Attributes
    pub ctratt: u32,
    /// Read Recovery Levels Supported
    pub rrls: u16,
    /// Reserved
    pub reserved1: [u8; 9],
    /// Controller Type
    pub cntrltype: u8,
    /// FRU Globally Unique Identifier
    pub fguid: [u8; 16],
    /// Command Retry Delay Time 1
    pub crdt1: u16,
    /// Command Retry Delay Time 2
    pub crdt2: u16,
    /// Command Retry Delay Time 3
    pub crdt3: u16,
    /// Reserved
    pub reserved2: [u8; 106],
    /// Reserved (NVMe-MI)
    pub reserved_mi: [u8; 16],
    /// Optional Admin Command Support
    pub oacs: u16,
    /// Abort Command Limit
    pub acl: u8,
    /// Asynchronous Event Request Limit
    pub aerl: u8,
    /// Firmware Updates
    pub frmw: u8,
    /// Log Page Attributes
    pub lpa: u8,
    /// Error Log Page Entries
    pub elpe: u8,
    /// Number of Power States Supported
    pub npss: u8,
    /// Admin Vendor Specific Command Configuration
    pub avscc: u8,
    /// Autonomous Power State Transition Attributes
    pub apsta: u8,
    /// Warning Composite Temperature Threshold
    pub wctemp: u16,
    /// Critical Composite Temperature Threshold
    pub cctemp: u16,
    /// Maximum Time for Firmware Activation
    pub mtfa: u16,
    /// Host Memory Buffer Preferred Size
    pub hmpre: u32,
    /// Host Memory Buffer Minimum Size
    pub hmmin: u32,
    /// Total NVM Capacity
    pub tnvmcap: [u8; 16],
    /// Unallocated NVM Capacity
    pub unvmcap: [u8; 16],
    /// Replay Protected Memory Block Support
    pub rpmbs: u32,
    /// Extended Device Self-test Time
    pub edstt: u16,
    /// Device Self-test Options
    pub dsto: u8,
    /// Firmware Update Granularity
    pub fwug: u8,
    /// Keep Alive Support
    pub kas: u16,
    /// Host Controlled Thermal Management Attributes
    pub hctma: u16,
    /// Minimum Thermal Management Temperature
    pub mntmt: u16,
    /// Maximum Thermal Management Temperature
    pub mxtmt: u16,
    /// Sanitize Capabilities
    pub sanicap: u32,
    /// Host Memory Buffer Minimum Descriptor Entry Size
    pub hmminds: u32,
    /// Host Memory Maximum Descriptors Entries
    pub hmmaxd: u16,
    /// NVM Set Identifier Maximum
    pub nsetidmax: u16,
    /// Endurance Group Identifier Maximum
    pub endgidmax: u16,
    /// ANA Transition Time
    pub anatt: u8,
    /// Asymmetric Namespace Access Capabilities
    pub anacap: u8,
    /// ANA Group Identifier Maximum
    pub anagrpmax: u32,
    /// Number of ANA Group Identifiers
    pub nanagrpid: u32,
    /// Persistent Event Log Size
    pub pels: u32,
    /// Reserved
    pub reserved3: [u8; 156],
    /// Submission Queue Entry Size
    pub sqes: u8,
    /// Completion Queue Entry Size
    pub cqes: u8,
    /// Maximum Outstanding Commands
    pub maxcmd: u16,
    /// Number of Namespaces
    pub nn: u32,
    /// Optional NVM Command Support
    pub oncs: u16,
    /// Fused Operation Support
    pub fuses: u16,
    /// Format NVM Attributes
    pub fna: u8,
    /// Volatile Write Cache
    pub vwc: u8,
    /// Atomic Write Unit Normal
    pub awun: u16,
    /// Atomic Write Unit Power Fail
    pub awupf: u16,
    /// NVM Vendor Specific Command Configuration
    pub nvscc: u8,
    /// Namespace Write Protection Capabilities
    pub nwpc: u8,
    /// Atomic Compare & Write Unit
    pub acwu: u16,
    /// Reserved
    pub reserved4: u16,
    /// SGL Support
    pub sgls: u32,
    /// Maximum Number of Allowed Namespaces
    pub mnan: u32,
    /// Reserved
    pub reserved5: [u8; 224],
    /// NVM Subsystem NVMe Qualified Name
    pub subnqn: [u8; 256],
    /// Reserved
    pub reserved6: [u8; 768],
    /// I/O Queue Command Capsule Supported Size (NVMe over Fabrics)
    pub ioccsz: u32,
    /// I/O Queue Response Capsule Supported Size (NVMe over Fabrics)
    pub iorcsz: u32,
    /// In Capsule Data Offset (NVMe over Fabrics)
    pub icdoff: u16,
    /// Controller Attributes (NVMe over Fabrics)
    pub fcatt: u8,
    /// Maximum SGL Data Block Descriptors (NVMe over Fabrics)
    pub msdbd: u8,
    /// Optional Fabric Commands Support (NVMe over Fabrics)
    pub ofcs: u16,
    /// Reserved
    pub reserved_fabrics: [u8; 242],
    /// Power State Descriptors
    pub psd: [PowerStateDescriptor; 32],
    /// Vendor Specific
    pub vs: [u8; 1024],
}

impl IdentifyController {
    /// Get serial number as string (trimmed)
    pub fn serial_number(&self) -> &[u8] {
        let end = self
            .sn
            .iter()
            .rposition(|&c| c != b' ' && c != 0)
            .map_or(0, |i| i + 1);
        &self.sn[..end]
    }

    /// Get model number as string (trimmed)
    pub fn model_number(&self) -> &[u8] {
        let end = self
            .mn
            .iter()
            .rposition(|&c| c != b' ' && c != 0)
            .map_or(0, |i| i + 1);
        &self.mn[..end]
    }

    /// Get firmware revision as string (trimmed)
    pub fn firmware_revision(&self) -> &[u8] {
        let end = self
            .fr
            .iter()
            .rposition(|&c| c != b' ' && c != 0)
            .map_or(0, |i| i + 1);
        &self.fr[..end]
    }

    /// Get NVMe version as (major, minor, tertiary)
    pub const fn version(&self) -> (u8, u8, u8) {
        let major = (self.ver >> 16) as u8;
        let minor = ((self.ver >> 8) & 0xFF) as u8;
        let tertiary = (self.ver & 0xFF) as u8;
        (major, minor, tertiary)
    }

    /// Get maximum data transfer size in bytes
    pub const fn max_transfer_size(&self, page_size: usize) -> usize {
        if self.mdts == 0 {
            // No limit reported, use default
            NVME_MAX_DATA_TRANSFER
        } else {
            page_size << self.mdts
        }
    }

    /// Check if namespace management is supported
    pub const fn supports_ns_management(&self) -> bool {
        (self.oacs & (1 << 3)) != 0
    }

    /// Check if firmware download/commit is supported
    pub const fn supports_firmware(&self) -> bool {
        (self.oacs & (1 << 2)) != 0
    }

    /// Check if format NVM is supported
    pub const fn supports_format(&self) -> bool {
        (self.oacs & (1 << 1)) != 0
    }

    /// Check if security commands are supported
    pub const fn supports_security(&self) -> bool {
        (self.oacs & (1 << 0)) != 0
    }

    /// Check if volatile write cache is present
    pub const fn has_volatile_write_cache(&self) -> bool {
        (self.vwc & 1) != 0
    }
}

/// Power State Descriptor
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PowerStateDescriptor {
    /// Maximum Power
    pub max_power: u16,
    /// Reserved
    pub reserved1: u8,
    /// Flags
    pub flags: u8,
    /// Entry Latency
    pub entry_lat: u32,
    /// Exit Latency
    pub exit_lat: u32,
    /// Relative Read Throughput
    pub rrt: u8,
    /// Relative Read Latency
    pub rrl: u8,
    /// Relative Write Throughput
    pub rwt: u8,
    /// Relative Write Latency
    pub rwl: u8,
    /// Idle Power
    pub idle_power: u16,
    /// Idle Power Scale
    pub idle_scale: u8,
    /// Reserved
    pub reserved2: u8,
    /// Active Power
    pub active_power: u16,
    /// Active Power Workload/Scale
    pub active_work_scale: u8,
    /// Reserved
    pub reserved3: [u8; 9],
}

impl PowerStateDescriptor {
    /// Get maximum power in milliwatts
    pub const fn max_power_mw(&self) -> u32 {
        if (self.flags & (1 << 0)) != 0 {
            // Non-operational state uses centiwatts
            (self.max_power as u32) * 10
        } else {
            // Operational state uses centiwatts
            (self.max_power as u32) * 10
        }
    }

    /// Check if this is a non-operational power state
    pub const fn is_non_operational(&self) -> bool {
        (self.flags & (1 << 1)) != 0
    }
}

/// NVMe Identify Namespace data structure (4096 bytes)
#[derive(Clone)]
#[repr(C)]
pub struct IdentifyNamespace {
    /// Namespace Size
    pub nsze: u64,
    /// Namespace Capacity
    pub ncap: u64,
    /// Namespace Utilization
    pub nuse: u64,
    /// Namespace Features
    pub nsfeat: u8,
    /// Number of LBA Formats
    pub nlbaf: u8,
    /// Formatted LBA Size
    pub flbas: u8,
    /// Metadata Capabilities
    pub mc: u8,
    /// End-to-end Data Protection Capabilities
    pub dpc: u8,
    /// End-to-end Data Protection Type Settings
    pub dps: u8,
    /// Namespace Multi-path I/O and Namespace Sharing Capabilities
    pub nmic: u8,
    /// Reservation Capabilities
    pub rescap: u8,
    /// Format Progress Indicator
    pub fpi: u8,
    /// Deallocate Logical Block Features
    pub dlfeat: u8,
    /// Namespace Atomic Write Unit Normal
    pub nawun: u16,
    /// Namespace Atomic Write Unit Power Fail
    pub nawupf: u16,
    /// Namespace Atomic Compare & Write Unit
    pub nacwu: u16,
    /// Namespace Atomic Boundary Size Normal
    pub nabsn: u16,
    /// Namespace Atomic Boundary Offset
    pub nabo: u16,
    /// Namespace Atomic Boundary Size Power Fail
    pub nabspf: u16,
    /// Namespace Optimal I/O Boundary
    pub noiob: u16,
    /// NVM Capacity
    pub nvmcap: [u8; 16],
    /// Namespace Preferred Write Granularity
    pub npwg: u16,
    /// Namespace Preferred Write Alignment
    pub npwa: u16,
    /// Namespace Preferred Deallocate Granularity
    pub npdg: u16,
    /// Namespace Preferred Deallocate Alignment
    pub npda: u16,
    /// Namespace Optimal Write Size
    pub nows: u16,
    /// Reserved
    pub reserved1: [u8; 18],
    /// ANA Group Identifier
    pub anagrpid: u32,
    /// Reserved
    pub reserved2: [u8; 3],
    /// Namespace Attributes
    pub nsattr: u8,
    /// NVM Set Identifier
    pub nvmsetid: u16,
    /// Endurance Group Identifier
    pub endgid: u16,
    /// Namespace Globally Unique Identifier
    pub nguid: [u8; 16],
    /// IEEE Extended Unique Identifier
    pub eui64: [u8; 8],
    /// LBA Format Support
    pub lbaf: [LbaFormat; 16],
    /// Reserved
    pub reserved3: [u8; 192],
    /// Vendor Specific
    pub vs: [u8; 3712],
}

impl IdentifyNamespace {
    /// Get current LBA format index
    pub const fn current_lba_format_index(&self) -> u8 {
        self.flbas & 0x0F
    }

    /// Get current LBA format
    pub const fn current_lba_format(&self) -> &LbaFormat {
        &self.lbaf[(self.flbas & 0x0F) as usize]
    }

    /// Get block size in bytes
    pub const fn block_size(&self) -> usize {
        1 << self.current_lba_format().ds
    }

    /// Get namespace size in bytes
    pub const fn size_bytes(&self) -> u64 {
        self.nsze * (self.block_size() as u64)
    }

    /// Get namespace capacity in bytes
    pub const fn capacity_bytes(&self) -> u64 {
        self.ncap * (self.block_size() as u64)
    }

    /// Check if namespace supports thin provisioning
    pub const fn thin_provisioning(&self) -> bool {
        (self.nsfeat & (1 << 0)) != 0
    }

    /// Check if namespace supports NAWUN/NAWUPF/NACWU
    pub const fn supports_atomic_writes(&self) -> bool {
        (self.nsfeat & (1 << 1)) != 0
    }

    /// Check if namespace supports deallocated or unwritten logical block error
    pub const fn supports_dealloc_error(&self) -> bool {
        (self.nsfeat & (1 << 2)) != 0
    }

    /// Check if metadata is extended LBA
    pub const fn metadata_extended_lba(&self) -> bool {
        (self.flbas & (1 << 4)) != 0
    }
}

/// LBA Format
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct LbaFormat {
    /// Metadata Size
    pub ms: u16,
    /// LBA Data Size (power of 2)
    pub ds: u8,
    /// Relative Performance
    pub rp: u8,
}

impl LbaFormat {
    /// Get data size in bytes
    pub const fn data_size(&self) -> usize {
        1 << self.ds
    }

    /// Get metadata size in bytes
    pub const fn metadata_size(&self) -> usize {
        self.ms as usize
    }

    /// Get relative performance (0=best, 3=degraded)
    pub const fn relative_performance(&self) -> u8 {
        self.rp & 0x03
    }
}

// =============================================================================
// NVME LOG PAGES
// =============================================================================

/// Log page identifiers
pub mod log_page {
    /// Error Information
    pub const ERROR_INFO: u8 = 0x01;
    /// SMART / Health Information
    pub const SMART_HEALTH: u8 = 0x02;
    /// Firmware Slot Information
    pub const FIRMWARE_SLOT: u8 = 0x03;
    /// Changed Namespace List
    pub const CHANGED_NS_LIST: u8 = 0x04;
    /// Commands Supported and Effects
    pub const COMMANDS_EFFECTS: u8 = 0x05;
    /// Device Self-test
    pub const DEVICE_SELF_TEST: u8 = 0x06;
    /// Telemetry Host-Initiated
    pub const TELEMETRY_HOST: u8 = 0x07;
    /// Telemetry Controller-Initiated
    pub const TELEMETRY_CONTROLLER: u8 = 0x08;
    /// Endurance Group Information
    pub const ENDURANCE_GROUP: u8 = 0x09;
    /// Predictable Latency Per NVM Set
    pub const PREDICTABLE_LATENCY: u8 = 0x0A;
    /// Predictable Latency Event Aggregate
    pub const PREDICTABLE_LATENCY_AGG: u8 = 0x0B;
    /// Asymmetric Namespace Access
    pub const ANA: u8 = 0x0C;
    /// Persistent Event Log
    pub const PERSISTENT_EVENT: u8 = 0x0D;
    /// LBA Status Information
    pub const LBA_STATUS: u8 = 0x0E;
    /// Endurance Group Event Aggregate
    pub const ENDURANCE_GROUP_EVENT: u8 = 0x0F;
    /// Discovery
    pub const DISCOVERY: u8 = 0x70;
    /// Reservation Notification
    pub const RESERVATION_NOTIFICATION: u8 = 0x80;
    /// Sanitize Status
    pub const SANITIZE_STATUS: u8 = 0x81;
}

/// SMART / Health Information Log
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SmartLog {
    /// Critical Warning
    pub critical_warning: u8,
    /// Composite Temperature
    pub temperature: [u8; 2],
    /// Available Spare
    pub avail_spare: u8,
    /// Available Spare Threshold
    pub spare_thresh: u8,
    /// Percentage Used
    pub percent_used: u8,
    /// Endurance Group Critical Warning Summary
    pub endurance_critical_warning: u8,
    /// Reserved
    pub reserved1: [u8; 25],
    /// Data Units Read
    pub data_units_read: [u8; 16],
    /// Data Units Written
    pub data_units_written: [u8; 16],
    /// Host Read Commands
    pub host_reads: [u8; 16],
    /// Host Write Commands
    pub host_writes: [u8; 16],
    /// Controller Busy Time
    pub ctrl_busy_time: [u8; 16],
    /// Power Cycles
    pub power_cycles: [u8; 16],
    /// Power On Hours
    pub power_on_hours: [u8; 16],
    /// Unsafe Shutdowns
    pub unsafe_shutdowns: [u8; 16],
    /// Media and Data Integrity Errors
    pub media_errors: [u8; 16],
    /// Number of Error Information Log Entries
    pub num_err_log_entries: [u8; 16],
    /// Warning Composite Temperature Time
    pub warning_temp_time: u32,
    /// Critical Composite Temperature Time
    pub critical_temp_time: u32,
    /// Temperature Sensor 1
    pub temp_sensor1: u16,
    /// Temperature Sensor 2
    pub temp_sensor2: u16,
    /// Temperature Sensor 3
    pub temp_sensor3: u16,
    /// Temperature Sensor 4
    pub temp_sensor4: u16,
    /// Temperature Sensor 5
    pub temp_sensor5: u16,
    /// Temperature Sensor 6
    pub temp_sensor6: u16,
    /// Temperature Sensor 7
    pub temp_sensor7: u16,
    /// Temperature Sensor 8
    pub temp_sensor8: u16,
    /// Thermal Management Temperature 1 Transition Count
    pub thm_temp1_trans_count: u32,
    /// Thermal Management Temperature 2 Transition Count
    pub thm_temp2_trans_count: u32,
    /// Total Time For Thermal Management Temperature 1
    pub thm_temp1_total_time: u32,
    /// Total Time For Thermal Management Temperature 2
    pub thm_temp2_total_time: u32,
    /// Reserved
    pub reserved2: [u8; 280],
}

impl SmartLog {
    /// Get composite temperature in Kelvin
    pub fn temperature_kelvin(&self) -> u16 {
        u16::from_le_bytes(self.temperature)
    }

    /// Get composite temperature in Celsius
    pub fn temperature_celsius(&self) -> i16 {
        (self.temperature_kelvin() as i16) - 273
    }

    /// Check if spare capacity is below threshold
    pub const fn spare_below_threshold(&self) -> bool {
        (self.critical_warning & (1 << 0)) != 0
    }

    /// Check if temperature is above critical threshold
    pub const fn temperature_critical(&self) -> bool {
        (self.critical_warning & (1 << 1)) != 0
    }

    /// Check if NVM subsystem reliability is degraded
    pub const fn reliability_degraded(&self) -> bool {
        (self.critical_warning & (1 << 2)) != 0
    }

    /// Check if media is in read-only mode
    pub const fn read_only(&self) -> bool {
        (self.critical_warning & (1 << 3)) != 0
    }

    /// Check if volatile memory backup failed
    pub const fn volatile_memory_failed(&self) -> bool {
        (self.critical_warning & (1 << 4)) != 0
    }

    /// Check if PMR is in read-only mode
    pub const fn pmr_read_only(&self) -> bool {
        (self.critical_warning & (1 << 5)) != 0
    }
}

// =============================================================================
// NVME FEATURES
// =============================================================================

/// Feature identifiers
pub mod feature {
    /// Arbitration
    pub const ARBITRATION: u8 = 0x01;
    /// Power Management
    pub const POWER_MANAGEMENT: u8 = 0x02;
    /// LBA Range Type
    pub const LBA_RANGE_TYPE: u8 = 0x03;
    /// Temperature Threshold
    pub const TEMPERATURE_THRESHOLD: u8 = 0x04;
    /// Error Recovery
    pub const ERROR_RECOVERY: u8 = 0x05;
    /// Volatile Write Cache
    pub const VOLATILE_WRITE_CACHE: u8 = 0x06;
    /// Number of Queues
    pub const NUMBER_OF_QUEUES: u8 = 0x07;
    /// Interrupt Coalescing
    pub const INTERRUPT_COALESCING: u8 = 0x08;
    /// Interrupt Vector Configuration
    pub const INTERRUPT_VECTOR_CONFIG: u8 = 0x09;
    /// Write Atomicity Normal
    pub const WRITE_ATOMICITY_NORMAL: u8 = 0x0A;
    /// Asynchronous Event Configuration
    pub const ASYNC_EVENT_CONFIG: u8 = 0x0B;
    /// Autonomous Power State Transition
    pub const AUTO_POWER_STATE: u8 = 0x0C;
    /// Host Memory Buffer
    pub const HOST_MEMORY_BUFFER: u8 = 0x0D;
    /// Timestamp
    pub const TIMESTAMP: u8 = 0x0E;
    /// Keep Alive Timer
    pub const KEEP_ALIVE_TIMER: u8 = 0x0F;
    /// Host Controlled Thermal Management
    pub const HOST_THERMAL_MGMT: u8 = 0x10;
    /// Non-Operational Power State Config
    pub const NON_OP_POWER_STATE: u8 = 0x11;
    /// Read Recovery Level
    pub const READ_RECOVERY_LEVEL: u8 = 0x12;
    /// Predictable Latency Mode Config
    pub const PREDICTABLE_LATENCY: u8 = 0x13;
    /// Predictable Latency Mode Window
    pub const PREDICTABLE_LATENCY_WINDOW: u8 = 0x14;
    /// LBA Status Information Report Interval
    pub const LBA_STATUS_INTERVAL: u8 = 0x15;
    /// Host Behavior Support
    pub const HOST_BEHAVIOR: u8 = 0x16;
    /// Sanitize Config
    pub const SANITIZE_CONFIG: u8 = 0x17;
    /// Endurance Group Event Configuration
    pub const ENDURANCE_EVENT_CONFIG: u8 = 0x18;
    /// Software Progress Marker
    pub const SOFTWARE_PROGRESS: u8 = 0x80;
    /// Host Identifier
    pub const HOST_IDENTIFIER: u8 = 0x81;
    /// Reservation Notification Mask
    pub const RESERVATION_MASK: u8 = 0x82;
    /// Reservation Persistence
    pub const RESERVATION_PERSISTENCE: u8 = 0x83;
    /// Namespace Write Protection Config
    pub const NS_WRITE_PROTECTION: u8 = 0x84;
}

// =============================================================================
// NVME ERROR TYPES
// =============================================================================

/// NVMe error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvmeError {
    /// Controller not found
    ControllerNotFound,
    /// Controller not ready
    ControllerNotReady,
    /// Command timeout
    Timeout,
    /// Command failed
    CommandFailed(NvmeStatus),
    /// Invalid namespace
    InvalidNamespace,
    /// Queue full
    QueueFull,
    /// Invalid queue
    InvalidQueue,
    /// Out of memory
    OutOfMemory,
    /// Invalid parameter
    InvalidParameter,
    /// Data transfer error
    DataTransferError,
    /// Not supported
    NotSupported,
}

impl fmt::Display for NvmeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NvmeError::ControllerNotFound => write!(f, "Controller not found"),
            NvmeError::ControllerNotReady => write!(f, "Controller not ready"),
            NvmeError::Timeout => write!(f, "Command timeout"),
            NvmeError::CommandFailed(status) => write!(f, "Command failed: {}", status),
            NvmeError::InvalidNamespace => write!(f, "Invalid namespace"),
            NvmeError::QueueFull => write!(f, "Queue full"),
            NvmeError::InvalidQueue => write!(f, "Invalid queue"),
            NvmeError::OutOfMemory => write!(f, "Out of memory"),
            NvmeError::InvalidParameter => write!(f, "Invalid parameter"),
            NvmeError::DataTransferError => write!(f, "Data transfer error"),
            NvmeError::NotSupported => write!(f, "Not supported"),
        }
    }
}

// =============================================================================
// QUEUE STRUCTURES
// =============================================================================

/// Queue pair state
#[derive(Debug, Clone, Copy)]
pub struct QueueState {
    /// Queue ID
    pub qid: u16,
    /// Queue size (entries)
    pub size: u16,
    /// Submission queue tail
    pub sq_tail: u16,
    /// Completion queue head
    pub cq_head: u16,
    /// Phase bit
    pub phase: bool,
    /// Command ID counter
    pub cid: u16,
}

impl QueueState {
    /// Create new queue state
    pub const fn new(qid: u16, size: u16) -> Self {
        Self {
            qid,
            size,
            sq_tail: 0,
            cq_head: 0,
            phase: true,
            cid: 0,
        }
    }

    /// Get next command ID
    pub fn next_cid(&mut self) -> u16 {
        let cid = self.cid;
        self.cid = self.cid.wrapping_add(1);
        cid
    }

    /// Advance submission queue tail
    pub fn advance_sq_tail(&mut self) {
        self.sq_tail = (self.sq_tail + 1) % self.size;
    }

    /// Advance completion queue head
    pub fn advance_cq_head(&mut self) {
        self.cq_head = (self.cq_head + 1) % self.size;
        if self.cq_head == 0 {
            self.phase = !self.phase;
        }
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Calculate doorbell offset for a queue
pub const fn doorbell_offset(qid: u16, doorbell_stride: u32, is_completion: bool) -> usize {
    let offset = if is_completion { 1 } else { 0 };
    regs::SQ0TDBL + (((2 * qid as u32 + offset) * (4 << doorbell_stride)) as usize)
}

/// Calculate PRP list entries needed for a transfer
pub const fn prp_entries_needed(offset: usize, length: usize, page_size: usize) -> usize {
    if length == 0 {
        return 0;
    }

    let first_page_bytes = page_size - (offset & (page_size - 1));
    if length <= first_page_bytes {
        return 0;
    }

    let remaining = length - first_page_bytes;
    remaining.div_ceil(page_size)
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_command_id() {
        let mut cmd = NvmeCommand::new();
        cmd.set_opcode(AdminOpcode::Identify as u8, 0x1234);
        assert_eq!(cmd.command_id(), 0x1234);
        assert_eq!(cmd.opcode(), AdminOpcode::Identify as u8);
    }

    #[test]
    fn test_completion_status() {
        let cqe = NvmeCompletion {
            result: 0,
            sq_head: 0,
            sq_id: 0,
            cid: 1,
            status: 0x0001, // Phase bit set, success
        };

        assert!(cqe.phase());
        assert!(cqe.is_success());
        assert_eq!(cqe.status_code_type(), 0);
        assert_eq!(cqe.status_code(), 0);
    }

    #[test]
    fn test_queue_state() {
        let mut state = QueueState::new(1, 256);
        assert_eq!(state.next_cid(), 0);
        assert_eq!(state.next_cid(), 1);

        state.advance_sq_tail();
        assert_eq!(state.sq_tail, 1);

        for _ in 0..255 {
            state.advance_cq_head();
        }
        assert_eq!(state.cq_head, 255);
        assert!(state.phase);

        state.advance_cq_head();
        assert_eq!(state.cq_head, 0);
        assert!(!state.phase); // Phase flipped
    }

    #[test]
    fn test_doorbell_offset() {
        // Admin queue, submission, stride = 0
        assert_eq!(doorbell_offset(0, 0, false), regs::SQ0TDBL);
        // Admin queue, completion, stride = 0
        assert_eq!(doorbell_offset(0, 0, true), regs::SQ0TDBL + 4);
        // I/O queue 1, submission, stride = 0
        assert_eq!(doorbell_offset(1, 0, false), regs::SQ0TDBL + 8);
    }

    #[test]
    fn test_prp_entries() {
        let page_size = 4096;

        // Single page, no offset
        assert_eq!(prp_entries_needed(0, 4096, page_size), 0);

        // Two pages
        assert_eq!(prp_entries_needed(0, 8192, page_size), 1);

        // Offset causes extra PRP
        assert_eq!(prp_entries_needed(512, 4096, page_size), 1);
    }

    #[test]
    fn test_lba_format() {
        let format = LbaFormat {
            ms: 0,
            ds: 9, // 512 bytes
            rp: 0,
        };

        assert_eq!(format.data_size(), 512);
        assert_eq!(format.metadata_size(), 0);
        assert_eq!(format.relative_performance(), 0);
    }
}
