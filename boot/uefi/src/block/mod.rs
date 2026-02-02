//! Block Device Abstraction Layer for Helix UEFI Bootloader
//!
//! This module provides a unified block device abstraction layer
//! supporting multiple storage protocols and partition schemes.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                     Block Device Architecture                           │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌──────────────────────────────────────────────────────────────────┐  │
//! │  │                     Filesystem Layer                             │  │
//! │  │  FAT │ NTFS │ ext4 │ XFS │ Btrfs │ ISO9660 │ UDF                 │  │
//! │  └──────────────────────────────────────────────────────────────────┘  │
//! │                              │                                         │
//! │  ┌──────────────────────────────────────────────────────────────────┐  │
//! │  │                    Partition Layer                               │  │
//! │  │  GPT │ MBR │ Apple Partition Map │ LVM │ Software RAID           │  │
//! │  └──────────────────────────────────────────────────────────────────┘  │
//! │                              │                                         │
//! │  ┌──────────────────────────────────────────────────────────────────┐  │
//! │  │                   Block Device Layer                             │  │
//! │  │  Read │ Write │ Flush │ Info │ Media Changed                     │  │
//! │  └──────────────────────────────────────────────────────────────────┘  │
//! │                              │                                         │
//! │  ┌──────────────────────────────────────────────────────────────────┐  │
//! │  │                    Protocol Layer                                │  │
//! │  │  NVMe │ AHCI/SATA │ USB Mass Storage │ SD/MMC │ VirtIO            │  │
//! │  └──────────────────────────────────────────────────────────────────┘  │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Features
//!
//! - Unified block device interface
//! - GPT and MBR partition parsing
//! - Media change detection
//! - Async I/O support
//! - Block caching
//! - SMART data access

#![no_std]

use core::fmt;

// =============================================================================
// BLOCK DEVICE TYPES
// =============================================================================

/// Block device type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockDeviceType {
    /// Unknown device
    Unknown,
    /// Hard disk drive
    Hdd,
    /// Solid state drive
    Ssd,
    /// NVMe drive
    Nvme,
    /// USB flash drive
    UsbFlash,
    /// USB hard drive
    UsbHdd,
    /// SD card
    SdCard,
    /// MMC card
    MmcCard,
    /// eMMC storage
    Emmc,
    /// Optical drive (CD/DVD/BD)
    Optical,
    /// Floppy drive
    Floppy,
    /// RAM disk
    RamDisk,
    /// Network storage
    Network,
    /// Virtual disk
    Virtual,
}

impl BlockDeviceType {
    /// Get device type name
    pub const fn name(&self) -> &'static str {
        match self {
            BlockDeviceType::Unknown => "Unknown",
            BlockDeviceType::Hdd => "Hard Disk",
            BlockDeviceType::Ssd => "SSD",
            BlockDeviceType::Nvme => "NVMe",
            BlockDeviceType::UsbFlash => "USB Flash",
            BlockDeviceType::UsbHdd => "USB HDD",
            BlockDeviceType::SdCard => "SD Card",
            BlockDeviceType::MmcCard => "MMC Card",
            BlockDeviceType::Emmc => "eMMC",
            BlockDeviceType::Optical => "Optical",
            BlockDeviceType::Floppy => "Floppy",
            BlockDeviceType::RamDisk => "RAM Disk",
            BlockDeviceType::Network => "Network",
            BlockDeviceType::Virtual => "Virtual",
        }
    }

    /// Check if device is removable
    pub const fn is_removable(&self) -> bool {
        matches!(
            self,
            BlockDeviceType::UsbFlash
                | BlockDeviceType::UsbHdd
                | BlockDeviceType::SdCard
                | BlockDeviceType::MmcCard
                | BlockDeviceType::Optical
                | BlockDeviceType::Floppy
        )
    }

    /// Check if device is rotational
    pub const fn is_rotational(&self) -> bool {
        matches!(
            self,
            BlockDeviceType::Hdd | BlockDeviceType::UsbHdd | BlockDeviceType::Floppy
        )
    }
}

/// Media type for optical drives
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    /// No media
    NoMedia,
    /// CD-ROM
    CdRom,
    /// CD-R
    CdR,
    /// CD-RW
    CdRw,
    /// DVD-ROM
    DvdRom,
    /// DVD-R
    DvdR,
    /// DVD-RW
    DvdRw,
    /// DVD+R
    DvdPlusR,
    /// DVD+RW
    DvdPlusRw,
    /// DVD-RAM
    DvdRam,
    /// BD-ROM
    BdRom,
    /// BD-R
    BdR,
    /// BD-RE
    BdRe,
}

// =============================================================================
// BLOCK DEVICE INFO
// =============================================================================

/// Block device information
#[derive(Debug, Clone)]
pub struct BlockDeviceInfo {
    /// Device type
    pub device_type: BlockDeviceType,
    /// Block size in bytes
    pub block_size: u32,
    /// Total number of blocks
    pub total_blocks: u64,
    /// Optimal transfer size in blocks
    pub optimal_transfer_blocks: u32,
    /// Device is read-only
    pub read_only: bool,
    /// Device is removable
    pub removable: bool,
    /// Media present
    pub media_present: bool,
    /// Logical blocks per physical block
    pub logical_blocks_per_physical: u32,
    /// Physical block size
    pub physical_block_size: u32,
    /// Alignment offset
    pub alignment_offset: u32,
    /// Device model
    pub model: [u8; 40],
    /// Model length
    pub model_len: usize,
    /// Serial number
    pub serial: [u8; 20],
    /// Serial length
    pub serial_len: usize,
    /// Firmware revision
    pub firmware: [u8; 8],
    /// Firmware length
    pub firmware_len: usize,
}

impl BlockDeviceInfo {
    /// Create new block device info
    pub const fn new(device_type: BlockDeviceType) -> Self {
        Self {
            device_type,
            block_size: 512,
            total_blocks: 0,
            optimal_transfer_blocks: 128,
            read_only: false,
            removable: false,
            media_present: true,
            logical_blocks_per_physical: 1,
            physical_block_size: 512,
            alignment_offset: 0,
            model: [0u8; 40],
            model_len: 0,
            serial: [0u8; 20],
            serial_len: 0,
            firmware: [0u8; 8],
            firmware_len: 0,
        }
    }

    /// Get total capacity in bytes
    pub const fn capacity_bytes(&self) -> u64 {
        self.total_blocks * self.block_size as u64
    }

    /// Get capacity in megabytes
    pub const fn capacity_mb(&self) -> u64 {
        self.capacity_bytes() / (1024 * 1024)
    }

    /// Get capacity in gigabytes
    pub const fn capacity_gb(&self) -> u64 {
        self.capacity_bytes() / (1024 * 1024 * 1024)
    }

    /// Set model string
    pub fn set_model(&mut self, model: &[u8]) {
        let len = model.len().min(self.model.len());
        self.model[..len].copy_from_slice(&model[..len]);
        self.model_len = len;
    }

    /// Set serial string
    pub fn set_serial(&mut self, serial: &[u8]) {
        let len = serial.len().min(self.serial.len());
        self.serial[..len].copy_from_slice(&serial[..len]);
        self.serial_len = len;
    }

    /// Get model as str
    pub fn model_str(&self) -> &str {
        core::str::from_utf8(&self.model[..self.model_len]).unwrap_or("")
    }

    /// Get serial as str
    pub fn serial_str(&self) -> &str {
        core::str::from_utf8(&self.serial[..self.serial_len]).unwrap_or("")
    }
}

impl Default for BlockDeviceInfo {
    fn default() -> Self {
        Self::new(BlockDeviceType::Unknown)
    }
}

// =============================================================================
// PARTITION SCHEMES
// =============================================================================

/// Partition scheme
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionScheme {
    /// No partitions (raw disk)
    None,
    /// Master Boot Record
    Mbr,
    /// GUID Partition Table
    Gpt,
    /// Apple Partition Map
    Apm,
    /// BSD disklabel
    BsdLabel,
    /// Sun/Solaris partition table
    Sun,
    /// LVM physical volume
    Lvm,
    /// Software RAID
    Raid,
}

impl PartitionScheme {
    /// Get scheme name
    pub const fn name(&self) -> &'static str {
        match self {
            PartitionScheme::None => "None",
            PartitionScheme::Mbr => "MBR",
            PartitionScheme::Gpt => "GPT",
            PartitionScheme::Apm => "Apple Partition Map",
            PartitionScheme::BsdLabel => "BSD Disklabel",
            PartitionScheme::Sun => "Sun",
            PartitionScheme::Lvm => "LVM",
            PartitionScheme::Raid => "RAID",
        }
    }
}

// =============================================================================
// MBR STRUCTURES
// =============================================================================

/// MBR partition entry
#[derive(Debug, Clone, Copy, Default)]
#[repr(C, packed)]
pub struct MbrPartitionEntry {
    /// Boot indicator (0x80 = bootable)
    pub boot_indicator: u8,
    /// Starting CHS head
    pub start_head: u8,
    /// Starting CHS sector + cylinder high bits
    pub start_sector: u8,
    /// Starting CHS cylinder low bits
    pub start_cylinder: u8,
    /// Partition type
    pub partition_type: u8,
    /// Ending CHS head
    pub end_head: u8,
    /// Ending CHS sector + cylinder high bits
    pub end_sector: u8,
    /// Ending CHS cylinder low bits
    pub end_cylinder: u8,
    /// Starting LBA
    pub start_lba: u32,
    /// Size in sectors
    pub size_sectors: u32,
}

impl MbrPartitionEntry {
    /// Check if partition is bootable
    pub const fn is_bootable(&self) -> bool {
        self.boot_indicator == 0x80
    }

    /// Check if partition is empty
    pub const fn is_empty(&self) -> bool {
        self.partition_type == 0
    }

    /// Check if partition is extended
    pub const fn is_extended(&self) -> bool {
        matches!(self.partition_type, 0x05 | 0x0F | 0x85)
    }

    /// Get partition type name
    pub const fn type_name(&self) -> &'static str {
        match self.partition_type {
            0x00 => "Empty",
            0x01 => "FAT12",
            0x04 => "FAT16 <32M",
            0x05 => "Extended",
            0x06 => "FAT16",
            0x07 => "HPFS/NTFS",
            0x0B => "FAT32",
            0x0C => "FAT32 LBA",
            0x0E => "FAT16 LBA",
            0x0F => "Extended LBA",
            0x11 => "Hidden FAT12",
            0x14 => "Hidden FAT16 <32M",
            0x16 => "Hidden FAT16",
            0x17 => "Hidden HPFS/NTFS",
            0x1B => "Hidden FAT32",
            0x1C => "Hidden FAT32 LBA",
            0x1E => "Hidden FAT16 LBA",
            0x27 => "Windows RE",
            0x42 => "Dynamic Disk",
            0x82 => "Linux Swap",
            0x83 => "Linux",
            0x85 => "Linux Extended",
            0x8E => "Linux LVM",
            0xA5 => "FreeBSD",
            0xA6 => "OpenBSD",
            0xA8 => "Darwin UFS",
            0xA9 => "NetBSD",
            0xAB => "Darwin Boot",
            0xAF => "HFS/HFS+",
            0xEE => "GPT Protective",
            0xEF => "EFI System",
            0xFB => "VMware VMFS",
            0xFC => "VMware Swap",
            0xFD => "Linux RAID",
            _ => "Unknown",
        }
    }
}

/// MBR boot sector
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct MbrBootSector {
    /// Boot code
    pub boot_code: [u8; 446],
    /// Partition entries
    pub partitions: [MbrPartitionEntry; 4],
    /// Boot signature (0x55AA)
    pub signature: u16,
}

impl MbrBootSector {
    /// MBR signature
    pub const SIGNATURE: u16 = 0xAA55;

    /// Check if valid MBR
    pub const fn is_valid(&self) -> bool {
        self.signature == Self::SIGNATURE
    }

    /// Check if this is a GPT protective MBR
    pub const fn is_gpt_protective(&self) -> bool {
        self.partitions[0].partition_type == 0xEE
    }

    /// Count valid partitions
    pub fn count_partitions(&self) -> usize {
        self.partitions.iter().filter(|p| !p.is_empty()).count()
    }
}

// =============================================================================
// GPT STRUCTURES
// =============================================================================

/// GPT header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct GptHeader {
    /// Signature ("EFI PART")
    pub signature: [u8; 8],
    /// Revision
    pub revision: u32,
    /// Header size
    pub header_size: u32,
    /// CRC32 of header
    pub header_crc32: u32,
    /// Reserved
    pub reserved: u32,
    /// Current LBA
    pub current_lba: u64,
    /// Backup LBA
    pub backup_lba: u64,
    /// First usable LBA
    pub first_usable_lba: u64,
    /// Last usable LBA
    pub last_usable_lba: u64,
    /// Disk GUID
    pub disk_guid: [u8; 16],
    /// Partition entry LBA
    pub partition_entry_lba: u64,
    /// Number of partition entries
    pub num_partition_entries: u32,
    /// Size of partition entry
    pub partition_entry_size: u32,
    /// CRC32 of partition entries
    pub partition_entry_crc32: u32,
}

impl GptHeader {
    /// GPT signature
    pub const SIGNATURE: [u8; 8] = *b"EFI PART";

    /// GPT revision 1.0
    pub const REVISION_1_0: u32 = 0x00010000;

    /// Check if valid signature
    pub const fn is_valid_signature(&self) -> bool {
        self.signature[0] == b'E'
            && self.signature[1] == b'F'
            && self.signature[2] == b'I'
            && self.signature[3] == b' '
            && self.signature[4] == b'P'
            && self.signature[5] == b'A'
            && self.signature[6] == b'R'
            && self.signature[7] == b'T'
    }
}

/// GPT partition entry
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct GptPartitionEntry {
    /// Partition type GUID
    pub partition_type_guid: [u8; 16],
    /// Unique partition GUID
    pub unique_partition_guid: [u8; 16],
    /// Starting LBA
    pub starting_lba: u64,
    /// Ending LBA
    pub ending_lba: u64,
    /// Attributes
    pub attributes: u64,
    /// Partition name (UTF-16LE)
    pub partition_name: [u16; 36],
}

impl GptPartitionEntry {
    /// Required partition
    pub const ATTR_REQUIRED: u64 = 1 << 0;
    /// No block I/O protocol
    pub const ATTR_NO_BLOCK_IO: u64 = 1 << 1;
    /// Legacy BIOS bootable
    pub const ATTR_LEGACY_BOOTABLE: u64 = 1 << 2;

    /// Check if entry is empty
    pub fn is_empty(&self) -> bool {
        self.partition_type_guid == [0u8; 16]
    }

    /// Get partition size in blocks
    pub const fn size_blocks(&self) -> u64 {
        if self.ending_lba >= self.starting_lba {
            self.ending_lba - self.starting_lba + 1
        } else {
            0
        }
    }

    /// Check if required partition
    pub const fn is_required(&self) -> bool {
        (self.attributes & Self::ATTR_REQUIRED) != 0
    }

    /// Check if legacy bootable
    pub const fn is_legacy_bootable(&self) -> bool {
        (self.attributes & Self::ATTR_LEGACY_BOOTABLE) != 0
    }
}

/// Well-known GPT partition type GUIDs
pub mod gpt_types {
    /// Unused entry
    pub const UNUSED: [u8; 16] = [0; 16];

    /// EFI System Partition
    pub const EFI_SYSTEM: [u8; 16] = [
        0x28, 0x73, 0x2A, 0xC1, 0x1F, 0xF8, 0xD2, 0x11, 0xBA, 0x4B, 0x00, 0xA0, 0xC9, 0x3E, 0xC9,
        0x3B,
    ];

    /// BIOS Boot Partition
    pub const BIOS_BOOT: [u8; 16] = [
        0x48, 0x61, 0x68, 0x21, 0x49, 0x64, 0x6F, 0x6E, 0x74, 0x4E, 0x65, 0x65, 0x64, 0x45, 0x46,
        0x49,
    ];

    /// Microsoft Reserved
    pub const MICROSOFT_RESERVED: [u8; 16] = [
        0x16, 0xE3, 0xC9, 0xE3, 0x5C, 0x0B, 0xB8, 0x4D, 0x81, 0x7D, 0xF9, 0x2D, 0xF0, 0x02, 0x15,
        0xAE,
    ];

    /// Microsoft Basic Data
    pub const MICROSOFT_BASIC_DATA: [u8; 16] = [
        0xA2, 0xA0, 0xD0, 0xEB, 0xE5, 0xB9, 0x33, 0x44, 0x87, 0xC0, 0x68, 0xB6, 0xB7, 0x26, 0x99,
        0xC7,
    ];

    /// Linux filesystem
    pub const LINUX_FILESYSTEM: [u8; 16] = [
        0xAF, 0x3D, 0xC6, 0x0F, 0x83, 0x84, 0x72, 0x47, 0x8E, 0x79, 0x3D, 0x69, 0xD8, 0x47, 0x7D,
        0xE4,
    ];

    /// Linux Swap
    pub const LINUX_SWAP: [u8; 16] = [
        0x6D, 0xFD, 0x57, 0x06, 0xAB, 0xA4, 0xC4, 0x43, 0x84, 0xE5, 0x09, 0x33, 0xC8, 0x4B, 0x4F,
        0x4F,
    ];

    /// Linux LVM
    pub const LINUX_LVM: [u8; 16] = [
        0x79, 0xD3, 0xD6, 0xE6, 0x07, 0xF5, 0xC2, 0x44, 0xA2, 0x3C, 0x23, 0x8F, 0x2A, 0x3D, 0xF9,
        0x28,
    ];

    /// Apple HFS+
    pub const APPLE_HFS_PLUS: [u8; 16] = [
        0x00, 0x53, 0x46, 0x48, 0x00, 0x00, 0xAA, 0x11, 0xAA, 0x11, 0x00, 0x30, 0x65, 0x43, 0xEC,
        0xAC,
    ];

    /// Apple APFS
    pub const APPLE_APFS: [u8; 16] = [
        0xEF, 0x57, 0x34, 0x7C, 0x00, 0x00, 0xAA, 0x11, 0xAA, 0x11, 0x00, 0x30, 0x65, 0x43, 0xEC,
        0xAC,
    ];
}

// =============================================================================
// PARTITION INFO
// =============================================================================

/// Partition information
#[derive(Debug, Clone)]
pub struct PartitionInfo {
    /// Partition number (1-based)
    pub number: u32,
    /// Partition scheme
    pub scheme: PartitionScheme,
    /// Start LBA
    pub start_lba: u64,
    /// End LBA
    pub end_lba: u64,
    /// Size in sectors
    pub size_sectors: u64,
    /// Is bootable
    pub bootable: bool,
    /// Partition type GUID (for GPT)
    pub type_guid: [u8; 16],
    /// Unique GUID (for GPT)
    pub unique_guid: [u8; 16],
    /// MBR partition type
    pub mbr_type: u8,
    /// Partition name
    pub name: [u8; 72],
    /// Name length
    pub name_len: usize,
}

impl PartitionInfo {
    /// Create new partition info
    pub const fn new(number: u32, scheme: PartitionScheme) -> Self {
        Self {
            number,
            scheme,
            start_lba: 0,
            end_lba: 0,
            size_sectors: 0,
            bootable: false,
            type_guid: [0u8; 16],
            unique_guid: [0u8; 16],
            mbr_type: 0,
            name: [0u8; 72],
            name_len: 0,
        }
    }

    /// Set name
    pub fn set_name(&mut self, name: &[u8]) {
        let len = name.len().min(self.name.len());
        self.name[..len].copy_from_slice(&name[..len]);
        self.name_len = len;
    }

    /// Get size in bytes (assuming 512-byte sectors)
    pub const fn size_bytes(&self) -> u64 {
        self.size_sectors * 512
    }

    /// Check if this is an EFI System Partition
    pub fn is_esp(&self) -> bool {
        self.type_guid == gpt_types::EFI_SYSTEM || self.mbr_type == 0xEF
    }

    /// Check if this is a Linux partition
    pub fn is_linux(&self) -> bool {
        self.type_guid == gpt_types::LINUX_FILESYSTEM || self.mbr_type == 0x83
    }

    /// Check if this is a Windows partition
    pub fn is_windows(&self) -> bool {
        self.type_guid == gpt_types::MICROSOFT_BASIC_DATA || self.mbr_type == 0x07
    }
}

// =============================================================================
// I/O OPERATIONS
// =============================================================================

/// Block I/O operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoOperation {
    /// Read blocks
    Read,
    /// Write blocks
    Write,
    /// Flush cache
    Flush,
    /// Trim/Discard blocks
    Trim,
    /// Secure erase
    SecureErase,
}

/// Block I/O request
#[derive(Debug, Clone)]
pub struct IoRequest {
    /// Operation type
    pub operation: IoOperation,
    /// Starting LBA
    pub start_lba: u64,
    /// Number of blocks
    pub block_count: u32,
    /// Priority (0 = highest)
    pub priority: u8,
    /// Flags
    pub flags: u32,
}

impl IoRequest {
    /// Create read request
    pub const fn read(start_lba: u64, block_count: u32) -> Self {
        Self {
            operation: IoOperation::Read,
            start_lba,
            block_count,
            priority: 4,
            flags: 0,
        }
    }

    /// Create write request
    pub const fn write(start_lba: u64, block_count: u32) -> Self {
        Self {
            operation: IoOperation::Write,
            start_lba,
            block_count,
            priority: 4,
            flags: 0,
        }
    }

    /// Create flush request
    pub const fn flush() -> Self {
        Self {
            operation: IoOperation::Flush,
            start_lba: 0,
            block_count: 0,
            priority: 0,
            flags: 0,
        }
    }
}

/// I/O request flags
pub mod io_flags {
    /// Force write through (bypass cache)
    pub const FUA: u32 = 1 << 0;
    /// Metadata I/O
    pub const METADATA: u32 = 1 << 1;
    /// High priority
    pub const HIPRI: u32 = 1 << 2;
    /// Synchronous I/O
    pub const SYNC: u32 = 1 << 3;
}

// =============================================================================
// SMART DATA
// =============================================================================

/// SMART status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmartStatus {
    /// SMART data not available
    NotAvailable,
    /// Drive is healthy
    Healthy,
    /// Warning threshold exceeded
    Warning,
    /// Drive failure predicted
    Critical,
    /// Drive failed
    Failed,
}

impl SmartStatus {
    /// Get status description
    pub const fn description(&self) -> &'static str {
        match self {
            SmartStatus::NotAvailable => "SMART not available",
            SmartStatus::Healthy => "Drive is healthy",
            SmartStatus::Warning => "Warning - backup recommended",
            SmartStatus::Critical => "Critical - failure predicted",
            SmartStatus::Failed => "Drive failed",
        }
    }
}

/// SMART attribute
#[derive(Debug, Clone, Copy)]
pub struct SmartAttribute {
    /// Attribute ID
    pub id: u8,
    /// Attribute name
    pub name: &'static str,
    /// Current value (normalized, 1-253)
    pub current: u8,
    /// Worst value ever
    pub worst: u8,
    /// Threshold
    pub threshold: u8,
    /// Raw value
    pub raw: u64,
    /// Is pre-failure attribute
    pub pre_failure: bool,
}

impl SmartAttribute {
    /// Check if below threshold
    pub const fn is_failed(&self) -> bool {
        self.current <= self.threshold
    }

    /// Check if near threshold
    pub const fn is_warning(&self) -> bool {
        self.current <= self.threshold + 10
    }
}

/// Common SMART attribute IDs
pub mod smart_ids {
    pub const READ_ERROR_RATE: u8 = 1;
    pub const THROUGHPUT_PERFORMANCE: u8 = 2;
    pub const SPIN_UP_TIME: u8 = 3;
    pub const START_STOP_COUNT: u8 = 4;
    pub const REALLOCATED_SECTOR_COUNT: u8 = 5;
    pub const SEEK_ERROR_RATE: u8 = 7;
    pub const POWER_ON_HOURS: u8 = 9;
    pub const SPIN_RETRY_COUNT: u8 = 10;
    pub const POWER_CYCLE_COUNT: u8 = 12;
    pub const SOFT_READ_ERROR_RATE: u8 = 13;
    pub const TEMPERATURE: u8 = 194;
    pub const REALLOCATED_EVENT_COUNT: u8 = 196;
    pub const CURRENT_PENDING_SECTOR: u8 = 197;
    pub const OFFLINE_UNCORRECTABLE: u8 = 198;
    pub const UDMA_CRC_ERROR_COUNT: u8 = 199;
    pub const WRITE_ERROR_RATE: u8 = 200;
    pub const TOTAL_LBAS_WRITTEN: u8 = 241;
    pub const TOTAL_LBAS_READ: u8 = 242;
}

/// SMART data summary
#[derive(Debug, Clone)]
pub struct SmartData {
    /// Overall status
    pub status: SmartStatus,
    /// Power on hours
    pub power_on_hours: u32,
    /// Power cycle count
    pub power_cycles: u32,
    /// Temperature in Celsius
    pub temperature_c: u8,
    /// Reallocated sector count
    pub reallocated_sectors: u32,
    /// Pending sector count
    pub pending_sectors: u32,
    /// Total bytes written (TB)
    pub total_written_tb: u32,
    /// Total bytes read (TB)
    pub total_read_tb: u32,
    /// Remaining life percentage (for SSDs)
    pub remaining_life_percent: u8,
}

impl SmartData {
    /// Create new empty SMART data
    pub const fn new() -> Self {
        Self {
            status: SmartStatus::NotAvailable,
            power_on_hours: 0,
            power_cycles: 0,
            temperature_c: 0,
            reallocated_sectors: 0,
            pending_sectors: 0,
            total_written_tb: 0,
            total_read_tb: 0,
            remaining_life_percent: 100,
        }
    }
}

impl Default for SmartData {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// ERROR TYPES
// =============================================================================

/// Block device error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockError {
    /// Device not found
    DeviceNotFound,
    /// Media not present
    MediaNotPresent,
    /// Media changed
    MediaChanged,
    /// Read error
    ReadError,
    /// Write error
    WriteError,
    /// Write protected
    WriteProtected,
    /// Invalid LBA
    InvalidLba,
    /// Buffer too small
    BufferTooSmall,
    /// Timeout
    Timeout,
    /// I/O error
    IoError,
    /// Invalid partition table
    InvalidPartitionTable,
    /// Device busy
    DeviceBusy,
}

impl fmt::Display for BlockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockError::DeviceNotFound => write!(f, "Device not found"),
            BlockError::MediaNotPresent => write!(f, "Media not present"),
            BlockError::MediaChanged => write!(f, "Media changed"),
            BlockError::ReadError => write!(f, "Read error"),
            BlockError::WriteError => write!(f, "Write error"),
            BlockError::WriteProtected => write!(f, "Write protected"),
            BlockError::InvalidLba => write!(f, "Invalid LBA"),
            BlockError::BufferTooSmall => write!(f, "Buffer too small"),
            BlockError::Timeout => write!(f, "Timeout"),
            BlockError::IoError => write!(f, "I/O error"),
            BlockError::InvalidPartitionTable => write!(f, "Invalid partition table"),
            BlockError::DeviceBusy => write!(f, "Device busy"),
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_device_type() {
        assert!(BlockDeviceType::UsbFlash.is_removable());
        assert!(!BlockDeviceType::Nvme.is_removable());
        assert!(BlockDeviceType::Hdd.is_rotational());
        assert!(!BlockDeviceType::Ssd.is_rotational());
    }

    #[test]
    fn test_block_device_info() {
        let mut info = BlockDeviceInfo::new(BlockDeviceType::Ssd);
        info.block_size = 512;
        info.total_blocks = 2 * 1024 * 1024 * 1024; // 1TB
        assert_eq!(info.capacity_gb(), 1024);
    }

    #[test]
    fn test_mbr_partition_entry() {
        let entry = MbrPartitionEntry {
            boot_indicator: 0x80,
            partition_type: 0x07,
            start_lba: 2048,
            size_sectors: 1024000,
            ..Default::default()
        };
        assert!(entry.is_bootable());
        assert!(!entry.is_empty());
        assert_eq!(entry.type_name(), "HPFS/NTFS");
    }

    #[test]
    fn test_gpt_partition_entry() {
        let entry = GptPartitionEntry {
            partition_type_guid: gpt_types::EFI_SYSTEM,
            unique_partition_guid: [1u8; 16],
            starting_lba: 2048,
            ending_lba: 206847,
            attributes: 0,
            partition_name: [0u16; 36],
        };
        assert!(!entry.is_empty());
        assert_eq!(entry.size_blocks(), 204800);
    }

    #[test]
    fn test_smart_status() {
        let status = SmartStatus::Healthy;
        assert_eq!(status.description(), "Drive is healthy");
    }
}
