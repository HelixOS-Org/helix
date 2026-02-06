//! FAT32 Filesystem Support
//!
//! This module provides FAT32 filesystem parsing and file access
//! for the Helix UEFI Bootloader.
//!
//! # Features
//!
//! - FAT12/16/32 support
//! - Long filename (LFN) support
//! - Directory traversal
//! - File reading
//! - Path parsing
//! - Cluster chain following

use core::fmt;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Sector size (standard)
pub const SECTOR_SIZE: usize = 512;

/// Maximum path length
pub const MAX_PATH_LEN: usize = 260;

/// Maximum filename length (LFN)
pub const MAX_FILENAME_LEN: usize = 255;

/// Short filename length (8.3)
pub const SHORT_NAME_LEN: usize = 11;

/// LFN entry characters
pub const LFN_CHARS_PER_ENTRY: usize = 13;

/// Directory entry size
pub const DIR_ENTRY_SIZE: usize = 32;

/// End of chain marker
pub const FAT32_EOC: u32 = 0x0FFF_FFF8;

/// Bad cluster marker
pub const FAT32_BAD: u32 = 0x0FFF_FFF7;

/// Free cluster marker
pub const FAT32_FREE: u32 = 0x0000_0000;

// =============================================================================
// FAT TYPE
// =============================================================================

/// FAT filesystem type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FatType {
    /// Unknown/invalid
    #[default]
    Unknown,
    /// FAT12
    Fat12,
    /// FAT16
    Fat16,
    /// FAT32
    Fat32,
    /// exFAT
    ExFat,
}

impl FatType {
    /// Determine FAT type from cluster count
    pub fn from_cluster_count(count: u32) -> Self {
        if count < 4085 {
            FatType::Fat12
        } else if count < 65525 {
            FatType::Fat16
        } else {
            FatType::Fat32
        }
    }

    /// Get bits per FAT entry
    pub const fn entry_bits(&self) -> u8 {
        match self {
            FatType::Fat12 => 12,
            FatType::Fat16 => 16,
            FatType::Fat32 => 28, // Upper 4 bits reserved
            _ => 0,
        }
    }

    /// Get end of chain marker
    pub const fn eoc_marker(&self) -> u32 {
        match self {
            FatType::Fat12 => 0x0FF8,
            FatType::Fat16 => 0xFFF8,
            FatType::Fat32 => 0x0FFF_FFF8,
            _ => 0,
        }
    }

    /// Check if cluster value is end of chain
    pub fn is_eoc(&self, cluster: u32) -> bool {
        match self {
            FatType::Fat12 => cluster >= 0x0FF8,
            FatType::Fat16 => cluster >= 0xFFF8,
            FatType::Fat32 => cluster >= 0x0FFF_FFF8,
            _ => true,
        }
    }
}

impl fmt::Display for FatType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FatType::Unknown => write!(f, "Unknown"),
            FatType::Fat12 => write!(f, "FAT12"),
            FatType::Fat16 => write!(f, "FAT16"),
            FatType::Fat32 => write!(f, "FAT32"),
            FatType::ExFat => write!(f, "exFAT"),
        }
    }
}

// =============================================================================
// BIOS PARAMETER BLOCK
// =============================================================================

/// BIOS Parameter Block (common fields)
#[derive(Debug, Clone, Copy, Default)]
pub struct BiosParameterBlock {
    /// Bytes per sector
    pub bytes_per_sector: u16,
    /// Sectors per cluster
    pub sectors_per_cluster: u8,
    /// Reserved sectors (before first FAT)
    pub reserved_sectors: u16,
    /// Number of FATs
    pub num_fats: u8,
    /// Root entry count (FAT12/16)
    pub root_entry_count: u16,
    /// Total sectors (16-bit)
    pub total_sectors_16: u16,
    /// Media type
    pub media_type: u8,
    /// Sectors per FAT (FAT12/16)
    pub sectors_per_fat_16: u16,
    /// Sectors per track
    pub sectors_per_track: u16,
    /// Number of heads
    pub num_heads: u16,
    /// Hidden sectors
    pub hidden_sectors: u32,
    /// Total sectors (32-bit)
    pub total_sectors_32: u32,
}

impl BiosParameterBlock {
    /// Parse from sector data
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 36 {
            return None;
        }

        // Check jump instruction
        if data[0] != 0xEB && data[0] != 0xE9 {
            return None;
        }

        Some(Self {
            bytes_per_sector: u16::from_le_bytes([data[11], data[12]]),
            sectors_per_cluster: data[13],
            reserved_sectors: u16::from_le_bytes([data[14], data[15]]),
            num_fats: data[16],
            root_entry_count: u16::from_le_bytes([data[17], data[18]]),
            total_sectors_16: u16::from_le_bytes([data[19], data[20]]),
            media_type: data[21],
            sectors_per_fat_16: u16::from_le_bytes([data[22], data[23]]),
            sectors_per_track: u16::from_le_bytes([data[24], data[25]]),
            num_heads: u16::from_le_bytes([data[26], data[27]]),
            hidden_sectors: u32::from_le_bytes([data[28], data[29], data[30], data[31]]),
            total_sectors_32: u32::from_le_bytes([data[32], data[33], data[34], data[35]]),
        })
    }

    /// Validate BPB
    pub fn validate(&self) -> bool {
        // Check bytes per sector (must be power of 2, 512-4096)
        if !matches!(self.bytes_per_sector, 512 | 1024 | 2048 | 4096) {
            return false;
        }

        // Check sectors per cluster (must be power of 2, 1-128)
        if self.sectors_per_cluster == 0
            || (self.sectors_per_cluster & (self.sectors_per_cluster - 1)) != 0
            || self.sectors_per_cluster > 128
        {
            return false;
        }

        // Check FAT count
        if self.num_fats == 0 || self.num_fats > 2 {
            return false;
        }

        // Check reserved sectors
        if self.reserved_sectors == 0 {
            return false;
        }

        true
    }

    /// Get total sectors
    pub const fn total_sectors(&self) -> u32 {
        if self.total_sectors_16 != 0 {
            self.total_sectors_16 as u32
        } else {
            self.total_sectors_32
        }
    }

    /// Get bytes per cluster
    pub const fn bytes_per_cluster(&self) -> u32 {
        self.bytes_per_sector as u32 * self.sectors_per_cluster as u32
    }
}

/// FAT32 Extended BIOS Parameter Block
#[derive(Debug, Clone, Copy, Default)]
pub struct Fat32Ebpb {
    /// Sectors per FAT
    pub sectors_per_fat: u32,
    /// Extended flags
    pub ext_flags: u16,
    /// Filesystem version
    pub fs_version: u16,
    /// Root directory cluster
    pub root_cluster: u32,
    /// FSInfo sector
    pub fsinfo_sector: u16,
    /// Backup boot sector
    pub backup_boot_sector: u16,
    /// Drive number
    pub drive_number: u8,
    /// Boot signature
    pub boot_signature: u8,
    /// Volume serial number
    pub volume_serial: u32,
    /// Volume label
    pub volume_label: [u8; 11],
    /// Filesystem type string
    pub fs_type: [u8; 8],
}

impl Fat32Ebpb {
    /// Parse from sector data
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 90 {
            return None;
        }

        let mut volume_label = [0u8; 11];
        let mut fs_type = [0u8; 8];
        volume_label.copy_from_slice(&data[71..82]);
        fs_type.copy_from_slice(&data[82..90]);

        Some(Self {
            sectors_per_fat: u32::from_le_bytes([data[36], data[37], data[38], data[39]]),
            ext_flags: u16::from_le_bytes([data[40], data[41]]),
            fs_version: u16::from_le_bytes([data[42], data[43]]),
            root_cluster: u32::from_le_bytes([data[44], data[45], data[46], data[47]]),
            fsinfo_sector: u16::from_le_bytes([data[48], data[49]]),
            backup_boot_sector: u16::from_le_bytes([data[50], data[51]]),
            drive_number: data[64],
            boot_signature: data[66],
            volume_serial: u32::from_le_bytes([data[67], data[68], data[69], data[70]]),
            volume_label,
            fs_type,
        })
    }

    /// Get volume label as string
    pub fn volume_label_str(&self) -> &str {
        // Find end (space padded)
        let end = self
            .volume_label
            .iter()
            .rposition(|&c| c != b' ')
            .map(|i| i + 1)
            .unwrap_or(0);
        core::str::from_utf8(&self.volume_label[..end]).unwrap_or("")
    }
}

// =============================================================================
// FILESYSTEM INFO
// =============================================================================

/// FAT32 FSInfo structure
#[derive(Debug, Clone, Copy, Default)]
pub struct FsInfo {
    /// Lead signature (0x41615252)
    pub lead_sig: u32,
    /// Structure signature (0x61417272)
    pub struct_sig: u32,
    /// Free cluster count (0xFFFFFFFF if unknown)
    pub free_count: u32,
    /// Next free cluster hint
    pub next_free: u32,
    /// Trail signature (0xAA550000)
    pub trail_sig: u32,
}

impl FsInfo {
    /// Parse from sector data
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 512 {
            return None;
        }

        let lead_sig = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let struct_sig = u32::from_le_bytes([data[484], data[485], data[486], data[487]]);
        let trail_sig = u32::from_le_bytes([data[508], data[509], data[510], data[511]]);

        // Validate signatures
        if lead_sig != 0x4161_5252 || struct_sig != 0x6141_7272 || trail_sig != 0xAA55_0000 {
            return None;
        }

        Some(Self {
            lead_sig,
            struct_sig,
            free_count: u32::from_le_bytes([data[488], data[489], data[490], data[491]]),
            next_free: u32::from_le_bytes([data[492], data[493], data[494], data[495]]),
            trail_sig,
        })
    }

    /// Check if free count is valid
    pub const fn has_free_count(&self) -> bool {
        self.free_count != 0xFFFF_FFFF
    }

    /// Check if next free hint is valid
    pub const fn has_next_free(&self) -> bool {
        self.next_free != 0xFFFF_FFFF && self.next_free >= 2
    }
}

// =============================================================================
// DIRECTORY ENTRY
// =============================================================================

/// Directory entry attributes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FileAttributes(u8);

impl FileAttributes {
    pub const READ_ONLY: FileAttributes = FileAttributes(0x01);
    pub const HIDDEN: FileAttributes = FileAttributes(0x02);
    pub const SYSTEM: FileAttributes = FileAttributes(0x04);
    pub const VOLUME_ID: FileAttributes = FileAttributes(0x08);
    pub const DIRECTORY: FileAttributes = FileAttributes(0x10);
    pub const ARCHIVE: FileAttributes = FileAttributes(0x20);
    pub const LFN: FileAttributes = FileAttributes(0x0F);

    /// Check if read-only
    pub const fn is_read_only(&self) -> bool {
        self.0 & Self::READ_ONLY.0 != 0
    }

    /// Check if hidden
    pub const fn is_hidden(&self) -> bool {
        self.0 & Self::HIDDEN.0 != 0
    }

    /// Check if system
    pub const fn is_system(&self) -> bool {
        self.0 & Self::SYSTEM.0 != 0
    }

    /// Check if volume ID
    pub const fn is_volume_id(&self) -> bool {
        self.0 & Self::VOLUME_ID.0 != 0
    }

    /// Check if directory
    pub const fn is_directory(&self) -> bool {
        self.0 & Self::DIRECTORY.0 != 0
    }

    /// Check if archive
    pub const fn is_archive(&self) -> bool {
        self.0 & Self::ARCHIVE.0 != 0
    }

    /// Check if LFN entry
    pub const fn is_lfn(&self) -> bool {
        self.0 == Self::LFN.0
    }
}

/// DOS date/time
#[derive(Debug, Clone, Copy, Default)]
pub struct DosDateTime {
    /// Date (packed)
    pub date: u16,
    /// Time (packed)
    pub time: u16,
    /// Centiseconds (0-199)
    pub centiseconds: u8,
}

impl DosDateTime {
    /// Get year (1980-2107)
    pub const fn year(&self) -> u16 {
        ((self.date >> 9) & 0x7F) + 1980
    }

    /// Get month (1-12)
    pub const fn month(&self) -> u8 {
        ((self.date >> 5) & 0x0F) as u8
    }

    /// Get day (1-31)
    pub const fn day(&self) -> u8 {
        (self.date & 0x1F) as u8
    }

    /// Get hour (0-23)
    pub const fn hour(&self) -> u8 {
        ((self.time >> 11) & 0x1F) as u8
    }

    /// Get minute (0-59)
    pub const fn minute(&self) -> u8 {
        ((self.time >> 5) & 0x3F) as u8
    }

    /// Get second (0-59)
    pub const fn second(&self) -> u8 {
        ((self.time & 0x1F) * 2) as u8 + self.centiseconds / 100
    }
}

/// Directory entry (short name)
#[derive(Debug, Clone, Copy, Default)]
pub struct DirectoryEntry {
    /// Short name (8.3 format)
    pub name: [u8; 11],
    /// Attributes
    pub attributes: FileAttributes,
    /// Reserved (NT)
    pub nt_reserved: u8,
    /// Creation time (centiseconds)
    pub create_time_tenth: u8,
    /// Creation time
    pub create_time: u16,
    /// Creation date
    pub create_date: u16,
    /// Last access date
    pub access_date: u16,
    /// First cluster (high 16 bits)
    pub first_cluster_hi: u16,
    /// Write time
    pub write_time: u16,
    /// Write date
    pub write_date: u16,
    /// First cluster (low 16 bits)
    pub first_cluster_lo: u16,
    /// File size
    pub file_size: u32,
}

impl DirectoryEntry {
    /// Parse from raw data
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < DIR_ENTRY_SIZE {
            return None;
        }

        let mut name = [0u8; 11];
        name.copy_from_slice(&data[0..11]);

        Some(Self {
            name,
            attributes: FileAttributes(data[11]),
            nt_reserved: data[12],
            create_time_tenth: data[13],
            create_time: u16::from_le_bytes([data[14], data[15]]),
            create_date: u16::from_le_bytes([data[16], data[17]]),
            access_date: u16::from_le_bytes([data[18], data[19]]),
            first_cluster_hi: u16::from_le_bytes([data[20], data[21]]),
            write_time: u16::from_le_bytes([data[22], data[23]]),
            write_date: u16::from_le_bytes([data[24], data[25]]),
            first_cluster_lo: u16::from_le_bytes([data[26], data[27]]),
            file_size: u32::from_le_bytes([data[28], data[29], data[30], data[31]]),
        })
    }

    /// Check if entry is free
    pub fn is_free(&self) -> bool {
        self.name[0] == 0xE5
    }

    /// Check if entry is end marker
    pub fn is_end(&self) -> bool {
        self.name[0] == 0x00
    }

    /// Check if entry is "." or ".."
    pub fn is_dot_entry(&self) -> bool {
        self.name[0] == b'.'
    }

    /// Get first cluster
    pub const fn first_cluster(&self) -> u32 {
        ((self.first_cluster_hi as u32) << 16) | (self.first_cluster_lo as u32)
    }

    /// Check if directory
    pub const fn is_directory(&self) -> bool {
        self.attributes.is_directory()
    }

    /// Check if LFN entry
    pub const fn is_lfn(&self) -> bool {
        self.attributes.is_lfn()
    }

    /// Get short name (8.3 without padding)
    pub fn short_name(&self) -> ([u8; 12], usize) {
        let mut result = [0u8; 12];
        let mut len = 0;

        // Copy base name (remove trailing spaces)
        for i in 0..8 {
            if self.name[i] != b' ' {
                result[len] = self.name[i];
                len += 1;
            }
        }

        // Check for extension
        if self.name[8] != b' ' {
            result[len] = b'.';
            len += 1;
            for i in 8..11 {
                if self.name[i] != b' ' {
                    result[len] = self.name[i];
                    len += 1;
                }
            }
        }

        (result, len)
    }

    /// Get creation datetime
    pub const fn creation_time(&self) -> DosDateTime {
        DosDateTime {
            date: self.create_date,
            time: self.create_time,
            centiseconds: self.create_time_tenth,
        }
    }

    /// Get modification datetime
    pub const fn modification_time(&self) -> DosDateTime {
        DosDateTime {
            date: self.write_date,
            time: self.write_time,
            centiseconds: 0,
        }
    }
}

/// Long filename entry
#[derive(Debug, Clone, Copy, Default)]
pub struct LfnEntry {
    /// Sequence number
    pub sequence: u8,
    /// Name characters 1-5
    pub name1: [u16; 5],
    /// Attributes (always 0x0F)
    pub attributes: u8,
    /// Type (always 0)
    pub entry_type: u8,
    /// Checksum of short name
    pub checksum: u8,
    /// Name characters 6-11
    pub name2: [u16; 6],
    /// First cluster (always 0)
    pub first_cluster: u16,
    /// Name characters 12-13
    pub name3: [u16; 2],
}

impl LfnEntry {
    /// Parse from raw data
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < DIR_ENTRY_SIZE {
            return None;
        }

        if data[11] != 0x0F {
            return None;
        }

        let mut name1 = [0u16; 5];
        let mut name2 = [0u16; 6];
        let mut name3 = [0u16; 2];

        for i in 0..5 {
            name1[i] = u16::from_le_bytes([data[1 + i * 2], data[2 + i * 2]]);
        }
        for i in 0..6 {
            name2[i] = u16::from_le_bytes([data[14 + i * 2], data[15 + i * 2]]);
        }
        for i in 0..2 {
            name3[i] = u16::from_le_bytes([data[28 + i * 2], data[29 + i * 2]]);
        }

        Some(Self {
            sequence: data[0],
            name1,
            attributes: data[11],
            entry_type: data[12],
            checksum: data[13],
            name2,
            first_cluster: u16::from_le_bytes([data[26], data[27]]),
            name3,
        })
    }

    /// Check if this is the last entry
    pub const fn is_last(&self) -> bool {
        self.sequence & 0x40 != 0
    }

    /// Get sequence number (1-based index)
    pub const fn index(&self) -> u8 {
        self.sequence & 0x1F
    }

    /// Extract characters to buffer (returns count)
    pub fn extract_chars(&self, buffer: &mut [u16]) -> usize {
        let mut count = 0;

        for &c in &self.name1 {
            if c == 0 || c == 0xFFFF {
                return count;
            }
            if count < buffer.len() {
                buffer[count] = c;
                count += 1;
            }
        }

        for &c in &self.name2 {
            if c == 0 || c == 0xFFFF {
                return count;
            }
            if count < buffer.len() {
                buffer[count] = c;
                count += 1;
            }
        }

        for &c in &self.name3 {
            if c == 0 || c == 0xFFFF {
                return count;
            }
            if count < buffer.len() {
                buffer[count] = c;
                count += 1;
            }
        }

        count
    }
}

// =============================================================================
// FAT FILESYSTEM
// =============================================================================

/// FAT filesystem information
#[derive(Debug, Clone, Copy)]
pub struct FatFilesystem {
    /// FAT type
    pub fat_type: FatType,
    /// BPB
    pub bpb: BiosParameterBlock,
    /// FAT32 EBPB (if FAT32)
    pub ebpb32: Option<Fat32Ebpb>,
    /// First data sector
    pub first_data_sector: u32,
    /// Root directory sector (FAT12/16)
    pub root_dir_sector: u32,
    /// Root directory sectors (FAT12/16)
    pub root_dir_sectors: u32,
    /// Data sectors
    pub data_sectors: u32,
    /// Cluster count
    pub cluster_count: u32,
    /// FAT start sector
    pub fat_start_sector: u32,
    /// Sectors per FAT
    pub sectors_per_fat: u32,
}

impl FatFilesystem {
    /// Parse filesystem from boot sector
    pub fn parse(boot_sector: &[u8]) -> Option<Self> {
        let bpb = BiosParameterBlock::parse(boot_sector)?;

        if !bpb.validate() {
            return None;
        }

        // Determine FAT type and parse extended BPB
        let sectors_per_fat = if bpb.sectors_per_fat_16 != 0 {
            bpb.sectors_per_fat_16 as u32
        } else {
            let ebpb = Fat32Ebpb::parse(boot_sector)?;
            ebpb.sectors_per_fat
        };

        // Calculate filesystem layout
        let root_dir_sectors = ((bpb.root_entry_count as u32 * 32)
            + (bpb.bytes_per_sector as u32 - 1))
            / bpb.bytes_per_sector as u32;

        let fat_sectors = sectors_per_fat * bpb.num_fats as u32;
        let first_data_sector = bpb.reserved_sectors as u32 + fat_sectors + root_dir_sectors;
        let total_sectors = bpb.total_sectors();
        let data_sectors = total_sectors.saturating_sub(first_data_sector);
        let cluster_count = data_sectors / bpb.sectors_per_cluster as u32;

        let fat_type = FatType::from_cluster_count(cluster_count);

        let ebpb32 = if fat_type == FatType::Fat32 {
            Fat32Ebpb::parse(boot_sector)
        } else {
            None
        };

        Some(Self {
            fat_type,
            bpb,
            ebpb32,
            first_data_sector,
            root_dir_sector: bpb.reserved_sectors as u32 + fat_sectors,
            root_dir_sectors,
            data_sectors,
            cluster_count,
            fat_start_sector: bpb.reserved_sectors as u32,
            sectors_per_fat,
        })
    }

    /// Get root cluster (FAT32) or 0 (FAT12/16)
    pub fn root_cluster(&self) -> u32 {
        self.ebpb32.map_or(0, |e| e.root_cluster)
    }

    /// Convert cluster number to sector number
    pub fn cluster_to_sector(&self, cluster: u32) -> u32 {
        self.first_data_sector + (cluster - 2) * self.bpb.sectors_per_cluster as u32
    }

    /// Get bytes per cluster
    pub fn bytes_per_cluster(&self) -> u32 {
        self.bpb.bytes_per_cluster()
    }

    /// Check if cluster is valid
    pub fn is_valid_cluster(&self, cluster: u32) -> bool {
        cluster >= 2 && cluster < (self.cluster_count + 2)
    }

    /// Calculate FAT entry offset
    pub fn fat_entry_offset(&self, cluster: u32) -> (u32, u32) {
        match self.fat_type {
            FatType::Fat12 => {
                let offset = cluster + (cluster / 2);
                let sector = self.fat_start_sector + offset / self.bpb.bytes_per_sector as u32;
                let byte_offset = offset % self.bpb.bytes_per_sector as u32;
                (sector, byte_offset)
            },
            FatType::Fat16 => {
                let offset = cluster * 2;
                let sector = self.fat_start_sector + offset / self.bpb.bytes_per_sector as u32;
                let byte_offset = offset % self.bpb.bytes_per_sector as u32;
                (sector, byte_offset)
            },
            FatType::Fat32 => {
                let offset = cluster * 4;
                let sector = self.fat_start_sector + offset / self.bpb.bytes_per_sector as u32;
                let byte_offset = offset % self.bpb.bytes_per_sector as u32;
                (sector, byte_offset)
            },
            _ => (0, 0),
        }
    }

    /// Read FAT entry from data
    pub fn read_fat_entry(&self, data: &[u8], cluster: u32, byte_offset: u32) -> u32 {
        let off = byte_offset as usize;

        match self.fat_type {
            FatType::Fat12 => {
                if off + 1 >= data.len() {
                    return 0;
                }
                let value = u16::from_le_bytes([data[off], data[off + 1]]);
                if cluster & 1 != 0 {
                    (value >> 4) as u32
                } else {
                    (value & 0x0FFF) as u32
                }
            },
            FatType::Fat16 => {
                if off + 1 >= data.len() {
                    return 0;
                }
                u16::from_le_bytes([data[off], data[off + 1]]) as u32
            },
            FatType::Fat32 => {
                if off + 3 >= data.len() {
                    return 0;
                }
                u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
                    & 0x0FFF_FFFF
            },
            _ => 0,
        }
    }

    /// Get volume label
    pub fn volume_label(&self) -> &str {
        self.ebpb32
            .as_ref()
            .map_or("", Fat32Ebpb::volume_label_str)
    }

    /// Get volume serial number
    pub fn volume_serial(&self) -> u32 {
        self.ebpb32.map_or(0, |e| e.volume_serial)
    }
}

// =============================================================================
// PATH UTILITIES
// =============================================================================

/// Path component
#[derive(Debug, Clone, Copy)]
pub struct PathComponent<'a> {
    /// Component name
    pub name: &'a str,
    /// Is last component
    pub is_last: bool,
}

/// Path iterator
pub struct PathIterator<'a> {
    path: &'a str,
    pos: usize,
}

impl<'a> PathIterator<'a> {
    /// Create new path iterator
    pub fn new(path: &'a str) -> Self {
        // Skip leading separators
        let path = path.trim_start_matches(['/', '\\']);
        Self { path, pos: 0 }
    }
}

impl<'a> Iterator for PathIterator<'a> {
    type Item = PathComponent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.path.len() {
            return None;
        }

        let remaining = &self.path[self.pos..];

        // Find next separator
        let end = remaining.find(['/', '\\']).unwrap_or(remaining.len());

        let name = &remaining[..end];

        // Skip separator for next iteration
        self.pos += end;
        while self.pos < self.path.len() {
            let c = self.path.as_bytes()[self.pos];
            if c != b'/' && c != b'\\' {
                break;
            }
            self.pos += 1;
        }

        if name.is_empty() {
            return self.next();
        }

        Some(PathComponent {
            name,
            is_last: self.pos >= self.path.len(),
        })
    }
}

/// Check if names match (case-insensitive)
pub fn names_match(a: &[u8], a_len: usize, b: &str) -> bool {
    if a_len != b.len() {
        return false;
    }

    let b_bytes = b.as_bytes();
    for i in 0..a_len {
        let ca = a[i].to_ascii_uppercase();
        let cb = b_bytes[i].to_ascii_uppercase();
        if ca != cb {
            return false;
        }
    }

    true
}

/// Convert short name to 8.3 format for comparison
pub fn to_short_name(name: &str) -> Option<[u8; 11]> {
    let mut result = [b' '; 11];

    let bytes = name.as_bytes();
    let dot_pos = bytes.iter().rposition(|&c| c == b'.');

    let (base, ext) = match dot_pos {
        Some(pos) => (&bytes[..pos], &bytes[pos + 1..]),
        None => (bytes, &[][..]),
    };

    // Check lengths
    if base.len() > 8 || ext.len() > 3 {
        return None;
    }

    // Copy and uppercase
    for (i, &c) in base.iter().take(8).enumerate() {
        result[i] = c.to_ascii_uppercase();
    }
    for (i, &c) in ext.iter().take(3).enumerate() {
        result[8 + i] = c.to_ascii_uppercase();
    }

    Some(result)
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_fat_type() {
        assert_eq!(FatType::from_cluster_count(1000), FatType::Fat12);
        assert_eq!(FatType::from_cluster_count(10000), FatType::Fat16);
        assert_eq!(FatType::from_cluster_count(100000), FatType::Fat32);
    }

    #[test]
    fn test_file_attributes() {
        let attr = FileAttributes(0x30); // Directory + Archive
        assert!(attr.is_directory());
        assert!(attr.is_archive());
        assert!(!attr.is_read_only());
    }

    #[test]
    fn test_dos_datetime() {
        // Date: 2024-01-15, Time: 12:30:00
        let dt = DosDateTime {
            date: ((44 << 9) | (1 << 5) | 15), // Year 2024 = 1980 + 44
            time: ((12 << 11) | (30 << 5) | 0),
            centiseconds: 0,
        };
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 12);
        assert_eq!(dt.minute(), 30);
    }

    #[test]
    fn test_path_iterator() {
        let path = "/boot/EFI/helix/helix.efi";
        let components: Vec<_> = PathIterator::new(path).collect();

        assert_eq!(components.len(), 4);
        assert_eq!(components[0].name, "boot");
        assert_eq!(components[1].name, "EFI");
        assert_eq!(components[2].name, "helix");
        assert_eq!(components[3].name, "helix.efi");
        assert!(components[3].is_last);
    }

    #[test]
    fn test_short_name() {
        let name = to_short_name("KERNEL.EFI").unwrap();
        assert_eq!(&name[..6], b"KERNEL");
        assert_eq!(&name[8..11], b"EFI");
    }

    #[test]
    fn test_names_match() {
        let name = b"KERNEL  EFI";
        assert!(names_match(&name[..6], 6, "kernel"));
        assert!(names_match(&name[..6], 6, "KERNEL"));
    }
}
