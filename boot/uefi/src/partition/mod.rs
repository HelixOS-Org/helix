//! Partition Table Support (GPT/MBR)
//!
//! This module provides partition table parsing for GPT and MBR
//! for the Helix UEFI Bootloader.
//!
//! # Features
//!
//! - GPT parsing
//! - MBR parsing
//! - Protective MBR detection
//! - Partition type identification
//! - GUID handling

#![no_std]

use core::fmt;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Sector size (standard)
pub const SECTOR_SIZE: usize = 512;

/// MBR signature
pub const MBR_SIGNATURE: u16 = 0xAA55;

/// GPT signature
pub const GPT_SIGNATURE: u64 = 0x5452415020494645; // "EFI PART"

/// Maximum GPT partitions
pub const MAX_GPT_PARTITIONS: usize = 128;

/// GPT entry size
pub const GPT_ENTRY_SIZE: usize = 128;

// =============================================================================
// GUID
// =============================================================================

/// GUID (Globally Unique Identifier)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Guid {
    /// Data1 (time_low)
    pub data1: u32,
    /// Data2 (time_mid)
    pub data2: u16,
    /// Data3 (time_hi_and_version)
    pub data3: u16,
    /// Data4 (clock_seq and node)
    pub data4: [u8; 8],
}

impl Guid {
    /// Create GUID from components
    pub const fn new(data1: u32, data2: u16, data3: u16, data4: [u8; 8]) -> Self {
        Self {
            data1,
            data2,
            data3,
            data4,
        }
    }

    /// Parse from bytes (mixed-endian format)
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 16 {
            return None;
        }

        Some(Self {
            data1: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            data2: u16::from_le_bytes([bytes[4], bytes[5]]),
            data3: u16::from_le_bytes([bytes[6], bytes[7]]),
            data4: [
                bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                bytes[15],
            ],
        })
    }

    /// Convert to bytes
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[0..4].copy_from_slice(&self.data1.to_le_bytes());
        bytes[4..6].copy_from_slice(&self.data2.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.data3.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.data4);
        bytes
    }

    /// Check if null GUID
    pub const fn is_null(&self) -> bool {
        self.data1 == 0
            && self.data2 == 0
            && self.data3 == 0
            && self.data4[0] == 0
            && self.data4[1] == 0
            && self.data4[2] == 0
            && self.data4[3] == 0
            && self.data4[4] == 0
            && self.data4[5] == 0
            && self.data4[6] == 0
            && self.data4[7] == 0
    }

    /// Null GUID
    pub const NULL: Guid = Guid::new(0, 0, 0, [0; 8]);
}

impl fmt::Display for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
            self.data1,
            self.data2,
            self.data3,
            self.data4[0],
            self.data4[1],
            self.data4[2],
            self.data4[3],
            self.data4[4],
            self.data4[5],
            self.data4[6],
            self.data4[7]
        )
    }
}

// =============================================================================
// KNOWN GUIDS
// =============================================================================

/// Well-known partition type GUIDs
pub mod partition_types {
    use super::Guid;

    /// Unused entry
    pub const UNUSED: Guid = Guid::new(0x00000000, 0x0000, 0x0000, [0; 8]);

    /// EFI System Partition
    pub const EFI_SYSTEM: Guid = Guid::new(0xC12A7328, 0xF81F, 0x11D2, [
        0xBA, 0x4B, 0x00, 0xA0, 0xC9, 0x3E, 0xC9, 0x3B,
    ]);

    /// Microsoft Reserved
    pub const MS_RESERVED: Guid = Guid::new(0xE3C9E316, 0x0B5C, 0x4DB8, [
        0x81, 0x7D, 0xF9, 0x2D, 0xF0, 0x02, 0x15, 0xAE,
    ]);

    /// Microsoft Basic Data
    pub const MS_BASIC_DATA: Guid = Guid::new(0xEBD0A0A2, 0xB9E5, 0x4433, [
        0x87, 0xC0, 0x68, 0xB6, 0xB7, 0x26, 0x99, 0xC7,
    ]);

    /// Linux Filesystem
    pub const LINUX_FS: Guid = Guid::new(0x0FC63DAF, 0x8483, 0x4772, [
        0x8E, 0x79, 0x3D, 0x69, 0xD8, 0x47, 0x7D, 0xE4,
    ]);

    /// Linux Swap
    pub const LINUX_SWAP: Guid = Guid::new(0x0657FD6D, 0xA4AB, 0x43C4, [
        0x84, 0xE5, 0x09, 0x33, 0xC8, 0x4B, 0x4F, 0x4F,
    ]);

    /// Linux Root (x86-64)
    pub const LINUX_ROOT_X86_64: Guid = Guid::new(0x4F68BCE3, 0xE8CD, 0x4DB1, [
        0x96, 0xE7, 0xFB, 0xCA, 0xF9, 0x84, 0xB7, 0x09,
    ]);

    /// Linux /boot
    pub const LINUX_BOOT: Guid = Guid::new(0xBC13C2FF, 0x59E6, 0x4262, [
        0xA3, 0x52, 0xB2, 0x75, 0xFD, 0x6F, 0x71, 0x72,
    ]);

    /// Apple HFS+
    pub const APPLE_HFS: Guid = Guid::new(0x48465300, 0x0000, 0x11AA, [
        0xAA, 0x11, 0x00, 0x30, 0x65, 0x43, 0xEC, 0xAC,
    ]);

    /// Apple APFS
    pub const APPLE_APFS: Guid = Guid::new(0x7C3457EF, 0x0000, 0x11AA, [
        0xAA, 0x11, 0x00, 0x30, 0x65, 0x43, 0xEC, 0xAC,
    ]);

    /// BIOS Boot
    pub const BIOS_BOOT: Guid = Guid::new(0x21686148, 0x6449, 0x6E6F, [
        0x74, 0x4E, 0x65, 0x65, 0x64, 0x45, 0x46, 0x49,
    ]);
}

/// Partition type identification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PartitionType {
    /// Unknown/unused
    #[default]
    Unknown,
    /// EFI System Partition
    EfiSystem,
    /// Microsoft Reserved
    MsReserved,
    /// Microsoft Basic Data (NTFS/FAT)
    MsBasicData,
    /// Linux Filesystem
    LinuxFs,
    /// Linux Swap
    LinuxSwap,
    /// Linux Root
    LinuxRoot,
    /// Linux Boot
    LinuxBoot,
    /// Apple HFS+
    AppleHfs,
    /// Apple APFS
    AppleApfs,
    /// BIOS Boot
    BiosBoot,
    /// Other
    Other,
}

impl PartitionType {
    /// Identify from GUID
    pub fn from_guid(guid: &Guid) -> Self {
        if *guid == partition_types::UNUSED {
            PartitionType::Unknown
        } else if *guid == partition_types::EFI_SYSTEM {
            PartitionType::EfiSystem
        } else if *guid == partition_types::MS_RESERVED {
            PartitionType::MsReserved
        } else if *guid == partition_types::MS_BASIC_DATA {
            PartitionType::MsBasicData
        } else if *guid == partition_types::LINUX_FS {
            PartitionType::LinuxFs
        } else if *guid == partition_types::LINUX_SWAP {
            PartitionType::LinuxSwap
        } else if *guid == partition_types::LINUX_ROOT_X86_64 {
            PartitionType::LinuxRoot
        } else if *guid == partition_types::LINUX_BOOT {
            PartitionType::LinuxBoot
        } else if *guid == partition_types::APPLE_HFS {
            PartitionType::AppleHfs
        } else if *guid == partition_types::APPLE_APFS {
            PartitionType::AppleApfs
        } else if *guid == partition_types::BIOS_BOOT {
            PartitionType::BiosBoot
        } else {
            PartitionType::Other
        }
    }

    /// Identify from MBR type byte
    pub fn from_mbr_type(type_byte: u8) -> Self {
        match type_byte {
            0x00 => PartitionType::Unknown,
            0x01 | 0x04 | 0x06 | 0x0B | 0x0C | 0x0E => PartitionType::MsBasicData, // FAT
            0x07 => PartitionType::MsBasicData,                                    // NTFS
            0x0F | 0x05 => PartitionType::Other,                                   // Extended
            0x82 => PartitionType::LinuxSwap,
            0x83 => PartitionType::LinuxFs,
            0xEE => PartitionType::Other, // GPT Protective
            0xEF => PartitionType::EfiSystem,
            _ => PartitionType::Other,
        }
    }
}

impl fmt::Display for PartitionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PartitionType::Unknown => write!(f, "Unknown"),
            PartitionType::EfiSystem => write!(f, "EFI System"),
            PartitionType::MsReserved => write!(f, "MS Reserved"),
            PartitionType::MsBasicData => write!(f, "Basic Data"),
            PartitionType::LinuxFs => write!(f, "Linux Filesystem"),
            PartitionType::LinuxSwap => write!(f, "Linux Swap"),
            PartitionType::LinuxRoot => write!(f, "Linux Root"),
            PartitionType::LinuxBoot => write!(f, "Linux Boot"),
            PartitionType::AppleHfs => write!(f, "Apple HFS+"),
            PartitionType::AppleApfs => write!(f, "Apple APFS"),
            PartitionType::BiosBoot => write!(f, "BIOS Boot"),
            PartitionType::Other => write!(f, "Other"),
        }
    }
}

// =============================================================================
// MBR
// =============================================================================

/// MBR partition entry
#[derive(Debug, Clone, Copy, Default)]
pub struct MbrPartition {
    /// Boot indicator (0x80 = bootable)
    pub boot_indicator: u8,
    /// Starting head
    pub start_head: u8,
    /// Starting sector/cylinder
    pub start_sector_cyl: u16,
    /// Partition type
    pub partition_type: u8,
    /// Ending head
    pub end_head: u8,
    /// Ending sector/cylinder
    pub end_sector_cyl: u16,
    /// Starting LBA
    pub start_lba: u32,
    /// Total sectors
    pub total_sectors: u32,
}

impl MbrPartition {
    /// Parse from bytes
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 16 {
            return None;
        }

        Some(Self {
            boot_indicator: data[0],
            start_head: data[1],
            start_sector_cyl: u16::from_le_bytes([data[2], data[3]]),
            partition_type: data[4],
            end_head: data[5],
            end_sector_cyl: u16::from_le_bytes([data[6], data[7]]),
            start_lba: u32::from_le_bytes([data[8], data[9], data[10], data[11]]),
            total_sectors: u32::from_le_bytes([data[12], data[13], data[14], data[15]]),
        })
    }

    /// Check if bootable
    pub const fn is_bootable(&self) -> bool {
        self.boot_indicator == 0x80
    }

    /// Check if empty
    pub const fn is_empty(&self) -> bool {
        self.partition_type == 0x00
    }

    /// Check if GPT protective
    pub const fn is_gpt_protective(&self) -> bool {
        self.partition_type == 0xEE
    }

    /// Check if extended
    pub const fn is_extended(&self) -> bool {
        self.partition_type == 0x05 || self.partition_type == 0x0F
    }

    /// Get partition type
    pub fn partition_type_id(&self) -> PartitionType {
        PartitionType::from_mbr_type(self.partition_type)
    }

    /// Get size in bytes
    pub fn size_bytes(&self) -> u64 {
        self.total_sectors as u64 * SECTOR_SIZE as u64
    }
}

/// MBR (Master Boot Record)
#[derive(Debug, Clone, Copy)]
pub struct Mbr {
    /// Boot code (first 446 bytes)
    pub boot_code: [u8; 446],
    /// Partition entries (4)
    pub partitions: [MbrPartition; 4],
    /// Boot signature
    pub signature: u16,
}

impl Mbr {
    /// Parse from sector data
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < SECTOR_SIZE {
            return None;
        }

        // Check signature
        let signature = u16::from_le_bytes([data[510], data[511]]);
        if signature != MBR_SIGNATURE {
            return None;
        }

        let mut boot_code = [0u8; 446];
        boot_code.copy_from_slice(&data[0..446]);

        let mut partitions = [MbrPartition::default(); 4];
        for i in 0..4 {
            let offset = 446 + i * 16;
            partitions[i] = MbrPartition::parse(&data[offset..])?;
        }

        Some(Self {
            boot_code,
            partitions,
            signature,
        })
    }

    /// Check if this is a protective MBR for GPT
    pub fn is_protective(&self) -> bool {
        self.partitions[0].is_gpt_protective()
    }

    /// Get active partition index
    pub fn active_partition(&self) -> Option<usize> {
        for (i, part) in self.partitions.iter().enumerate() {
            if part.is_bootable() {
                return Some(i);
            }
        }
        None
    }

    /// Count valid partitions
    pub fn partition_count(&self) -> usize {
        self.partitions.iter().filter(|p| !p.is_empty()).count()
    }
}

// =============================================================================
// GPT
// =============================================================================

/// GPT header
#[derive(Debug, Clone, Copy)]
pub struct GptHeader {
    /// Signature ("EFI PART")
    pub signature: u64,
    /// Revision (1.0 = 0x00010000)
    pub revision: u32,
    /// Header size
    pub header_size: u32,
    /// Header CRC32
    pub header_crc32: u32,
    /// Reserved
    pub reserved: u32,
    /// Current LBA (location of this header)
    pub current_lba: u64,
    /// Backup LBA
    pub backup_lba: u64,
    /// First usable LBA
    pub first_usable_lba: u64,
    /// Last usable LBA
    pub last_usable_lba: u64,
    /// Disk GUID
    pub disk_guid: Guid,
    /// Partition entries start LBA
    pub partition_entries_lba: u64,
    /// Number of partition entries
    pub num_partition_entries: u32,
    /// Size of partition entry
    pub partition_entry_size: u32,
    /// Partition entries CRC32
    pub partition_entries_crc32: u32,
}

impl GptHeader {
    /// Parse from sector data
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 92 {
            return None;
        }

        let signature = u64::from_le_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ]);

        if signature != GPT_SIGNATURE {
            return None;
        }

        Some(Self {
            signature,
            revision: u32::from_le_bytes([data[8], data[9], data[10], data[11]]),
            header_size: u32::from_le_bytes([data[12], data[13], data[14], data[15]]),
            header_crc32: u32::from_le_bytes([data[16], data[17], data[18], data[19]]),
            reserved: u32::from_le_bytes([data[20], data[21], data[22], data[23]]),
            current_lba: u64::from_le_bytes([
                data[24], data[25], data[26], data[27], data[28], data[29], data[30], data[31],
            ]),
            backup_lba: u64::from_le_bytes([
                data[32], data[33], data[34], data[35], data[36], data[37], data[38], data[39],
            ]),
            first_usable_lba: u64::from_le_bytes([
                data[40], data[41], data[42], data[43], data[44], data[45], data[46], data[47],
            ]),
            last_usable_lba: u64::from_le_bytes([
                data[48], data[49], data[50], data[51], data[52], data[53], data[54], data[55],
            ]),
            disk_guid: Guid::from_bytes(&data[56..72])?,
            partition_entries_lba: u64::from_le_bytes([
                data[72], data[73], data[74], data[75], data[76], data[77], data[78], data[79],
            ]),
            num_partition_entries: u32::from_le_bytes([data[80], data[81], data[82], data[83]]),
            partition_entry_size: u32::from_le_bytes([data[84], data[85], data[86], data[87]]),
            partition_entries_crc32: u32::from_le_bytes([data[88], data[89], data[90], data[91]]),
        })
    }

    /// Check if valid
    pub fn is_valid(&self) -> bool {
        self.signature == GPT_SIGNATURE
            && self.header_size >= 92
            && self.partition_entry_size >= 128
    }

    /// Get revision as string
    pub fn revision_string(&self) -> (u16, u16) {
        (
            (self.revision >> 16) as u16,
            (self.revision & 0xFFFF) as u16,
        )
    }
}

/// GPT partition entry
#[derive(Debug, Clone, Copy)]
pub struct GptPartition {
    /// Partition type GUID
    pub type_guid: Guid,
    /// Unique partition GUID
    pub partition_guid: Guid,
    /// Starting LBA
    pub start_lba: u64,
    /// Ending LBA
    pub end_lba: u64,
    /// Attributes
    pub attributes: u64,
    /// Partition name (UTF-16LE, up to 36 characters)
    pub name: [u16; 36],
}

impl Default for GptPartition {
    fn default() -> Self {
        Self::empty()
    }
}

impl GptPartition {
    /// Create empty partition
    pub const fn empty() -> Self {
        Self {
            type_guid: Guid::NULL,
            partition_guid: Guid::NULL,
            start_lba: 0,
            end_lba: 0,
            attributes: 0,
            name: [0; 36],
        }
    }

    /// Parse from data
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < GPT_ENTRY_SIZE {
            return None;
        }

        let mut name = [0u16; 36];
        for i in 0..36 {
            name[i] = u16::from_le_bytes([data[56 + i * 2], data[57 + i * 2]]);
        }

        Some(Self {
            type_guid: Guid::from_bytes(&data[0..16])?,
            partition_guid: Guid::from_bytes(&data[16..32])?,
            start_lba: u64::from_le_bytes([
                data[32], data[33], data[34], data[35], data[36], data[37], data[38], data[39],
            ]),
            end_lba: u64::from_le_bytes([
                data[40], data[41], data[42], data[43], data[44], data[45], data[46], data[47],
            ]),
            attributes: u64::from_le_bytes([
                data[48], data[49], data[50], data[51], data[52], data[53], data[54], data[55],
            ]),
            name,
        })
    }

    /// Check if empty/unused
    pub fn is_empty(&self) -> bool {
        self.type_guid.is_null()
    }

    /// Get partition type
    pub fn partition_type(&self) -> PartitionType {
        PartitionType::from_guid(&self.type_guid)
    }

    /// Get size in sectors
    pub fn size_sectors(&self) -> u64 {
        if self.end_lba >= self.start_lba {
            self.end_lba - self.start_lba + 1
        } else {
            0
        }
    }

    /// Get size in bytes
    pub fn size_bytes(&self) -> u64 {
        self.size_sectors() * SECTOR_SIZE as u64
    }

    /// Get name as string (truncated to ASCII)
    pub fn name_ascii(&self) -> ([u8; 36], usize) {
        let mut result = [0u8; 36];
        let mut len = 0;

        for &c in &self.name {
            if c == 0 {
                break;
            }
            if c < 128 {
                result[len] = c as u8;
                len += 1;
            } else {
                result[len] = b'?';
                len += 1;
            }
        }

        (result, len)
    }

    /// Check attribute: Required
    pub const fn is_required(&self) -> bool {
        self.attributes & (1 << 0) != 0
    }

    /// Check attribute: No block I/O
    pub const fn no_block_io(&self) -> bool {
        self.attributes & (1 << 1) != 0
    }

    /// Check attribute: Legacy BIOS bootable
    pub const fn is_legacy_bootable(&self) -> bool {
        self.attributes & (1 << 2) != 0
    }
}

// =============================================================================
// PARTITION TABLE
// =============================================================================

/// Partition table type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PartitionTableType {
    /// Unknown/none
    #[default]
    Unknown,
    /// MBR
    Mbr,
    /// GPT
    Gpt,
    /// Hybrid (GPT with active MBR)
    Hybrid,
}

impl fmt::Display for PartitionTableType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PartitionTableType::Unknown => write!(f, "Unknown"),
            PartitionTableType::Mbr => write!(f, "MBR"),
            PartitionTableType::Gpt => write!(f, "GPT"),
            PartitionTableType::Hybrid => write!(f, "Hybrid"),
        }
    }
}

/// Generic partition info
#[derive(Debug, Clone, Copy)]
pub struct PartitionInfo {
    /// Partition index
    pub index: u8,
    /// Partition type
    pub partition_type: PartitionType,
    /// Type GUID (for GPT)
    pub type_guid: Guid,
    /// Partition GUID (for GPT)
    pub partition_guid: Guid,
    /// Starting LBA
    pub start_lba: u64,
    /// Size in sectors
    pub size_sectors: u64,
    /// Is bootable
    pub bootable: bool,
    /// Is EFI System Partition
    pub is_esp: bool,
    /// Name (ASCII)
    pub name: [u8; 36],
    pub name_len: usize,
}

impl Default for PartitionInfo {
    fn default() -> Self {
        Self {
            index: 0,
            partition_type: PartitionType::default(),
            type_guid: Guid::default(),
            partition_guid: Guid::default(),
            start_lba: 0,
            size_sectors: 0,
            bootable: false,
            is_esp: false,
            name: [0u8; 36],
            name_len: 0,
        }
    }
}

impl PartitionInfo {
    /// Get size in bytes
    pub fn size_bytes(&self) -> u64 {
        self.size_sectors * SECTOR_SIZE as u64
    }

    /// Get name as string
    pub fn name(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len]).unwrap_or("")
    }

    /// Set name
    pub fn set_name(&mut self, name: &str) {
        let bytes = name.as_bytes();
        let len = bytes.len().min(36);
        self.name[..len].copy_from_slice(&bytes[..len]);
        self.name_len = len;
    }
}

/// Maximum partitions in a table
pub const MAX_PARTITIONS: usize = 32;

/// Partition table
#[derive(Debug)]
pub struct PartitionTable {
    /// Table type
    pub table_type: PartitionTableType,
    /// Disk GUID (for GPT)
    pub disk_guid: Guid,
    /// Partitions
    pub partitions: [PartitionInfo; MAX_PARTITIONS],
    /// Partition count
    pub count: usize,
    /// ESP index (if found)
    pub esp_index: Option<usize>,
}

impl Default for PartitionTable {
    fn default() -> Self {
        Self::new()
    }
}

impl PartitionTable {
    /// Create empty partition table
    pub const fn new() -> Self {
        Self {
            table_type: PartitionTableType::Unknown,
            disk_guid: Guid::NULL,
            partitions: [PartitionInfo {
                index: 0,
                partition_type: PartitionType::Unknown,
                type_guid: Guid::NULL,
                partition_guid: Guid::NULL,
                start_lba: 0,
                size_sectors: 0,
                bootable: false,
                is_esp: false,
                name: [0; 36],
                name_len: 0,
            }; MAX_PARTITIONS],
            count: 0,
            esp_index: None,
        }
    }

    /// Add partition from MBR entry
    pub fn add_mbr_partition(&mut self, mbr_part: &MbrPartition) -> bool {
        if self.count >= MAX_PARTITIONS || mbr_part.is_empty() {
            return false;
        }

        let part_type = mbr_part.partition_type_id();
        let is_esp = part_type == PartitionType::EfiSystem;

        let info = PartitionInfo {
            index: self.count as u8,
            partition_type: part_type,
            type_guid: Guid::NULL,
            partition_guid: Guid::NULL,
            start_lba: mbr_part.start_lba as u64,
            size_sectors: mbr_part.total_sectors as u64,
            bootable: mbr_part.is_bootable(),
            is_esp,
            name: [0; 36],
            name_len: 0,
        };

        if is_esp && self.esp_index.is_none() {
            self.esp_index = Some(self.count);
        }

        self.partitions[self.count] = info;
        self.count += 1;
        self.table_type = PartitionTableType::Mbr;
        true
    }

    /// Add partition from GPT entry
    pub fn add_gpt_partition(&mut self, gpt_part: &GptPartition) -> bool {
        if self.count >= MAX_PARTITIONS || gpt_part.is_empty() {
            return false;
        }

        let part_type = gpt_part.partition_type();
        let is_esp = part_type == PartitionType::EfiSystem;
        let (name, name_len) = gpt_part.name_ascii();

        let info = PartitionInfo {
            index: self.count as u8,
            partition_type: part_type,
            type_guid: gpt_part.type_guid,
            partition_guid: gpt_part.partition_guid,
            start_lba: gpt_part.start_lba,
            size_sectors: gpt_part.size_sectors(),
            bootable: gpt_part.is_legacy_bootable(),
            is_esp,
            name,
            name_len,
        };

        if is_esp && self.esp_index.is_none() {
            self.esp_index = Some(self.count);
        }

        self.partitions[self.count] = info;
        self.count += 1;
        self.table_type = PartitionTableType::Gpt;
        true
    }

    /// Get partition by index
    pub fn get(&self, index: usize) -> Option<&PartitionInfo> {
        if index < self.count {
            Some(&self.partitions[index])
        } else {
            None
        }
    }

    /// Find EFI System Partition
    pub fn find_esp(&self) -> Option<&PartitionInfo> {
        self.esp_index.and_then(|i| self.get(i))
    }

    /// Find partition by type
    pub fn find_by_type(&self, part_type: PartitionType) -> Option<&PartitionInfo> {
        for i in 0..self.count {
            if self.partitions[i].partition_type == part_type {
                return Some(&self.partitions[i]);
            }
        }
        None
    }

    /// Get partition count
    pub const fn len(&self) -> usize {
        self.count
    }

    /// Check if empty
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_guid() {
        let guid = Guid::new(0x12345678, 0xABCD, 0xEF01, [1, 2, 3, 4, 5, 6, 7, 8]);
        assert!(!guid.is_null());

        let bytes = guid.to_bytes();
        let parsed = Guid::from_bytes(&bytes).unwrap();
        assert_eq!(guid, parsed);
    }

    #[test]
    fn test_null_guid() {
        assert!(Guid::NULL.is_null());
    }

    #[test]
    fn test_partition_type() {
        assert_eq!(
            PartitionType::from_guid(&partition_types::EFI_SYSTEM),
            PartitionType::EfiSystem
        );
        assert_eq!(PartitionType::from_mbr_type(0xEF), PartitionType::EfiSystem);
    }

    #[test]
    fn test_mbr_partition() {
        let mut data = [0u8; 16];
        data[0] = 0x80; // Bootable
        data[4] = 0x07; // NTFS

        let part = MbrPartition::parse(&data).unwrap();
        assert!(part.is_bootable());
        assert_eq!(part.partition_type, 0x07);
    }

    #[test]
    fn test_partition_table() {
        let mut table = PartitionTable::new();

        let mut mbr_part = MbrPartition::default();
        mbr_part.partition_type = 0xEF;
        mbr_part.start_lba = 2048;
        mbr_part.total_sectors = 204800;

        assert!(table.add_mbr_partition(&mbr_part));
        assert_eq!(table.len(), 1);
        assert!(table.esp_index.is_some());
    }

    #[test]
    fn test_gpt_partition() {
        let mut data = [0u8; 128];
        // Set type GUID (EFI System)
        let type_bytes = partition_types::EFI_SYSTEM.to_bytes();
        data[0..16].copy_from_slice(&type_bytes);

        let part = GptPartition::parse(&data).unwrap();
        assert_eq!(part.partition_type(), PartitionType::EfiSystem);
    }
}
