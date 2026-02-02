//! Block I/O Protocol
//!
//! High-level block device abstraction for disk access.

use super::{EnumerableProtocol, Protocol};
use crate::error::{Error, Result};
use crate::raw::protocols::block::*;
use crate::raw::types::*;

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

/// Block I/O Protocol GUID
const BLOCK_IO_PROTOCOL_GUID: Guid = guids::BLOCK_IO_PROTOCOL;

// =============================================================================
// BLOCK DEVICE
// =============================================================================

/// High-level block device abstraction
pub struct BlockDevice {
    /// Raw protocol pointer
    protocol: *mut EfiBlockIoProtocol,
    /// Handle
    handle: Handle,
    /// Cached media info
    media: MediaInfo,
}

impl BlockDevice {
    /// Create from raw protocol
    ///
    /// # Safety
    /// Protocol pointer must be valid
    pub unsafe fn from_raw(protocol: *mut EfiBlockIoProtocol, handle: Handle) -> Self {
        let media_ptr = (*protocol).media;
        let media = if !media_ptr.is_null() {
            MediaInfo::from_raw(&*media_ptr)
        } else {
            MediaInfo::default()
        };

        Self {
            protocol,
            handle,
            media,
        }
    }

    /// Get media information
    pub fn media(&self) -> &MediaInfo {
        &self.media
    }

    /// Refresh media information
    pub fn refresh_media(&mut self) {
        unsafe {
            let media_ptr = (*self.protocol).media;
            if !media_ptr.is_null() {
                self.media = MediaInfo::from_raw(&*media_ptr);
            }
        }
    }

    /// Check if media is present
    pub fn media_present(&self) -> bool {
        self.media.present
    }

    /// Check if media is removable
    pub fn is_removable(&self) -> bool {
        self.media.removable
    }

    /// Check if media is read-only
    pub fn is_readonly(&self) -> bool {
        self.media.readonly
    }

    /// Get block size
    pub fn block_size(&self) -> u32 {
        self.media.block_size
    }

    /// Get total block count
    pub fn block_count(&self) -> u64 {
        self.media.last_block + 1
    }

    /// Get total size in bytes
    pub fn size(&self) -> u64 {
        self.block_count() * self.block_size() as u64
    }

    /// Read blocks
    pub fn read_blocks(&self, lba: u64, buffer: &mut [u8]) -> Result<()> {
        if !self.media.present {
            return Err(Error::NoMedia);
        }

        let result = unsafe {
            ((*self.protocol).read_blocks)(
                self.protocol,
                self.media.id,
                lba,
                buffer.len(),
                buffer.as_mut_ptr(),
            )
        };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Write blocks
    pub fn write_blocks(&self, lba: u64, buffer: &[u8]) -> Result<()> {
        if !self.media.present {
            return Err(Error::NoMedia);
        }

        if self.media.readonly {
            return Err(Error::WriteProtected);
        }

        let result = unsafe {
            ((*self.protocol).write_blocks)(
                self.protocol,
                self.media.id,
                lba,
                buffer.len(),
                buffer.as_ptr(),
            )
        };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Flush blocks (ensure writes are completed)
    pub fn flush(&self) -> Result<()> {
        let result = unsafe { ((*self.protocol).flush_blocks)(self.protocol) };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Reset device
    pub fn reset(&self, extended: bool) -> Result<()> {
        let result = unsafe { ((*self.protocol).reset)(self.protocol, extended as u8) };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Read single block
    pub fn read_block(&self, lba: u64) -> Result<Vec<u8>> {
        let mut buffer = alloc::vec![0u8; self.block_size() as usize];
        self.read_blocks(lba, &mut buffer)?;
        Ok(buffer)
    }

    /// Write single block
    pub fn write_block(&self, lba: u64, data: &[u8]) -> Result<()> {
        if data.len() != self.block_size() as usize {
            return Err(Error::InvalidParameter);
        }
        self.write_blocks(lba, data)
    }

    /// Read bytes at offset
    pub fn read_at(&self, offset: u64, buffer: &mut [u8]) -> Result<()> {
        let block_size = self.block_size() as u64;
        let start_lba = offset / block_size;
        let start_offset = (offset % block_size) as usize;

        // Simple case: aligned read within one block
        if start_offset == 0 && buffer.len() as u64 == block_size {
            return self.read_blocks(start_lba, buffer);
        }

        // Complex case: unaligned or multi-block read
        let end = offset + buffer.len() as u64;
        let end_lba = (end + block_size - 1) / block_size;
        let blocks_needed = (end_lba - start_lba) as usize;

        let mut temp = alloc::vec![0u8; blocks_needed * block_size as usize];
        self.read_blocks(start_lba, &mut temp)?;

        buffer.copy_from_slice(&temp[start_offset..start_offset + buffer.len()]);

        Ok(())
    }

    /// Write bytes at offset
    pub fn write_at(&self, offset: u64, data: &[u8]) -> Result<()> {
        let block_size = self.block_size() as u64;
        let start_lba = offset / block_size;
        let start_offset = (offset % block_size) as usize;

        // Simple case: aligned write of exactly one block
        if start_offset == 0 && data.len() as u64 == block_size {
            return self.write_blocks(start_lba, data);
        }

        // Complex case: need read-modify-write
        let end = offset + data.len() as u64;
        let end_lba = (end + block_size - 1) / block_size;
        let blocks_needed = (end_lba - start_lba) as usize;

        // Read existing data
        let mut temp = alloc::vec![0u8; blocks_needed * block_size as usize];
        self.read_blocks(start_lba, &mut temp)?;

        // Modify
        temp[start_offset..start_offset + data.len()].copy_from_slice(data);

        // Write back
        self.write_blocks(start_lba, &temp)
    }

    /// Read MBR
    pub fn read_mbr(&self) -> Result<Mbr> {
        let block = self.read_block(0)?;
        Mbr::parse(&block)
    }

    /// Read GPT header
    pub fn read_gpt_header(&self) -> Result<GptHeader> {
        let block = self.read_block(1)?;
        GptHeader::parse(&block)
    }

    /// Get partitions
    pub fn partitions(&self) -> Result<Vec<Partition>> {
        // Try GPT first
        if let Ok(gpt) = self.read_gpt_header() {
            return self.read_gpt_partitions(&gpt);
        }

        // Fall back to MBR
        let mbr = self.read_mbr()?;
        Ok(mbr
            .partitions
            .into_iter()
            .filter(|p| p.partition_type != 0)
            .map(|p| Partition {
                start_lba: p.start_lba as u64,
                end_lba: p.start_lba as u64 + p.size_lba as u64 - 1,
                size_lba: p.size_lba as u64,
                partition_type: PartitionType::Mbr(p.partition_type),
                name: String::new(),
                guid: Guid::NULL,
                bootable: p.bootable,
            })
            .collect())
    }

    /// Read GPT partitions
    fn read_gpt_partitions(&self, header: &GptHeader) -> Result<Vec<Partition>> {
        let entries_per_block = self.block_size() as usize / header.entry_size as usize;
        let blocks_needed =
            (header.entry_count as usize + entries_per_block - 1) / entries_per_block;

        let mut data = alloc::vec![0u8; blocks_needed * self.block_size() as usize];
        self.read_blocks(header.partition_entry_lba, &mut data)?;

        let mut partitions = Vec::new();

        for i in 0..header.entry_count as usize {
            let offset = i * header.entry_size as usize;
            if offset + header.entry_size as usize > data.len() {
                break;
            }

            let entry = &data[offset..offset + header.entry_size as usize];

            // Check if entry is used (type GUID not zero)
            let type_guid = read_guid(entry, 0);
            if type_guid == Guid::NULL {
                continue;
            }

            let unique_guid = read_guid(entry, 16);
            let start_lba = read_u64(entry, 32);
            let end_lba = read_u64(entry, 40);
            let attributes = read_u64(entry, 48);

            // Read name (UCS-2, 36 chars max)
            let name_bytes = &entry[56..128.min(entry.len())];
            let name = read_ucs2_string(name_bytes);

            partitions.push(Partition {
                start_lba,
                end_lba,
                size_lba: end_lba - start_lba + 1,
                partition_type: PartitionType::Gpt(type_guid),
                name,
                guid: unique_guid,
                bootable: (attributes & 0x04) != 0,
            });
        }

        Ok(partitions)
    }

    /// Zero disk (dangerous!)
    pub fn zero(&self) -> Result<()> {
        let zeros = alloc::vec![0u8; self.block_size() as usize];

        // Just zero first and last blocks for safety
        self.write_blocks(0, &zeros)?;
        self.write_blocks(self.media.last_block, &zeros)?;

        Ok(())
    }
}

impl Protocol for BlockDevice {
    const GUID: Guid = BLOCK_IO_PROTOCOL_GUID;

    fn open(handle: Handle) -> Result<Self> {
        use crate::services::boot_services;

        let bs = unsafe { boot_services() };
        let image = crate::services::image_handle().ok_or(Error::NotReady)?;

        let mut protocol: *mut core::ffi::c_void = core::ptr::null_mut();
        let result = unsafe {
            ((*bs).open_protocol)(
                handle,
                &Self::GUID as *const Guid,
                &mut protocol,
                image,
                Handle(core::ptr::null_mut()),
                0x00000002,
            )
        };

        if result != Status::SUCCESS {
            return Err(Error::from_status(result));
        }

        Ok(unsafe { Self::from_raw(protocol as *mut EfiBlockIoProtocol, handle) })
    }
}

impl EnumerableProtocol for BlockDevice {
    fn enumerate() -> Result<Vec<Self>> {
        super::ProtocolLocator::locate_all::<Self>()
            .map(|handles| handles.into_iter().map(|h| h.leak()).collect())
    }
}

// =============================================================================
// MEDIA INFO
// =============================================================================

/// Block device media information
#[derive(Debug, Clone, Default)]
pub struct MediaInfo {
    /// Media ID
    pub id: u32,
    /// Is removable
    pub removable: bool,
    /// Is present
    pub present: bool,
    /// Is logical partition
    pub logical: bool,
    /// Is read-only
    pub readonly: bool,
    /// Supports write caching
    pub write_caching: bool,
    /// Block size in bytes
    pub block_size: u32,
    /// I/O alignment
    pub io_align: u32,
    /// Last block index
    pub last_block: u64,
    /// Lowest aligned LBA
    pub lowest_aligned_lba: u64,
    /// Logical blocks per physical block
    pub logical_blocks_per_physical: u32,
    /// Optimal transfer length
    pub optimal_transfer_length: u32,
}

impl MediaInfo {
    /// Create from raw UEFI structure
    fn from_raw(raw: &EfiBlockIoMedia) -> Self {
        Self {
            id: raw.media_id,
            removable: raw.removable_media != 0,
            present: raw.media_present != 0,
            logical: raw.logical_partition != 0,
            readonly: raw.read_only != 0,
            write_caching: raw.write_caching != 0,
            block_size: raw.block_size,
            io_align: raw.io_align,
            last_block: raw.last_block,
            lowest_aligned_lba: 0,
            logical_blocks_per_physical: 1,
            optimal_transfer_length: 0,
        }
    }

    /// Get total size in bytes
    pub fn size(&self) -> u64 {
        (self.last_block + 1) * self.block_size as u64
    }

    /// Get size in human-readable format
    pub fn size_string(&self) -> String {
        let bytes = self.size();

        if bytes >= 1024 * 1024 * 1024 * 1024 {
            alloc::format!(
                "{:.1} TB",
                bytes as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0)
            )
        } else if bytes >= 1024 * 1024 * 1024 {
            alloc::format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        } else if bytes >= 1024 * 1024 {
            alloc::format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        } else if bytes >= 1024 {
            alloc::format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            alloc::format!("{} B", bytes)
        }
    }
}

// =============================================================================
// DISK INFO
// =============================================================================

/// Comprehensive disk information
#[derive(Debug, Clone)]
pub struct DiskInfo {
    /// Media info
    pub media: MediaInfo,
    /// Partition table type
    pub partition_table: PartitionTable,
    /// Partitions
    pub partitions: Vec<Partition>,
    /// Disk identifier
    pub identifier: DiskIdentifier,
}

/// Partition table type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionTable {
    /// No partition table
    None,
    /// Master Boot Record
    Mbr,
    /// GUID Partition Table
    Gpt,
    /// Hybrid (GPT + protective MBR)
    Hybrid,
}

/// Disk identifier
#[derive(Debug, Clone)]
pub enum DiskIdentifier {
    /// MBR disk signature
    Mbr(u32),
    /// GPT disk GUID
    Gpt(Guid),
    /// Unknown
    Unknown,
}

// =============================================================================
// PARTITION
// =============================================================================

/// Partition information
#[derive(Debug, Clone)]
pub struct Partition {
    /// Starting LBA
    pub start_lba: u64,
    /// Ending LBA
    pub end_lba: u64,
    /// Size in blocks
    pub size_lba: u64,
    /// Partition type
    pub partition_type: PartitionType,
    /// Partition name (GPT only)
    pub name: String,
    /// Partition GUID (GPT only)
    pub guid: Guid,
    /// Is bootable
    pub bootable: bool,
}

impl Partition {
    /// Get size in bytes
    pub fn size(&self, block_size: u32) -> u64 {
        self.size_lba * block_size as u64
    }

    /// Get human-readable size
    pub fn size_string(&self, block_size: u32) -> String {
        let bytes = self.size(block_size);

        if bytes >= 1024 * 1024 * 1024 {
            alloc::format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        } else if bytes >= 1024 * 1024 {
            alloc::format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        } else if bytes >= 1024 {
            alloc::format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            alloc::format!("{} B", bytes)
        }
    }

    /// Get type name
    pub fn type_name(&self) -> &str {
        self.partition_type.name()
    }
}

/// Partition type
#[derive(Debug, Clone)]
pub enum PartitionType {
    /// MBR partition type
    Mbr(u8),
    /// GPT partition type GUID
    Gpt(Guid),
}

impl PartitionType {
    /// Get type name
    pub fn name(&self) -> &str {
        match self {
            Self::Mbr(t) => match t {
                0x00 => "Empty",
                0x01 => "FAT12",
                0x04 => "FAT16 (<32MB)",
                0x05 => "Extended",
                0x06 => "FAT16",
                0x07 => "NTFS/HPFS",
                0x0B => "FAT32",
                0x0C => "FAT32 (LBA)",
                0x0E => "FAT16 (LBA)",
                0x0F => "Extended (LBA)",
                0x11 => "Hidden FAT12",
                0x14 => "Hidden FAT16",
                0x1B => "Hidden FAT32",
                0x1C => "Hidden FAT32 (LBA)",
                0x1E => "Hidden FAT16 (LBA)",
                0x82 => "Linux Swap",
                0x83 => "Linux",
                0x85 => "Linux Extended",
                0x8E => "Linux LVM",
                0xEE => "GPT Protective",
                0xEF => "EFI System",
                _ => "Unknown",
            },
            Self::Gpt(guid) => {
                // Check well-known GUIDs
                if *guid == GPT_PARTITION_TYPE_EFI_SYSTEM {
                    "EFI System"
                } else if *guid == GPT_PARTITION_TYPE_BASIC_DATA {
                    "Basic Data"
                } else if *guid == GPT_PARTITION_TYPE_LINUX_FILESYSTEM {
                    "Linux Filesystem"
                } else if *guid == GPT_PARTITION_TYPE_LINUX_SWAP {
                    "Linux Swap"
                } else if *guid == GPT_PARTITION_TYPE_LINUX_LVM {
                    "Linux LVM"
                } else if *guid == GPT_PARTITION_TYPE_MICROSOFT_RESERVED {
                    "Microsoft Reserved"
                } else {
                    "Unknown GPT"
                }
            },
        }
    }
}

// Well-known GPT partition type GUIDs
const GPT_PARTITION_TYPE_EFI_SYSTEM: Guid = Guid::new(0xC12A7328, 0xF81F, 0x11D2, [
    0xBA, 0x4B, 0x00, 0xA0, 0xC9, 0x3E, 0xC9, 0x3B,
]);

const GPT_PARTITION_TYPE_BASIC_DATA: Guid = Guid::new(0xEBD0A0A2, 0xB9E5, 0x4433, [
    0x87, 0xC0, 0x68, 0xB6, 0xB7, 0x26, 0x99, 0xC7,
]);

const GPT_PARTITION_TYPE_LINUX_FILESYSTEM: Guid = Guid::new(0x0FC63DAF, 0x8483, 0x4772, [
    0x8E, 0x79, 0x3D, 0x69, 0xD8, 0x47, 0x7D, 0xE4,
]);

const GPT_PARTITION_TYPE_LINUX_SWAP: Guid = Guid::new(0x0657FD6D, 0xA4AB, 0x43C4, [
    0x84, 0xE5, 0x09, 0x33, 0xC8, 0x4B, 0x4F, 0x4F,
]);

const GPT_PARTITION_TYPE_LINUX_LVM: Guid = Guid::new(0xE6D6D379, 0xF507, 0x44C2, [
    0xA2, 0x3C, 0x23, 0x8F, 0x2A, 0x3D, 0xF9, 0x28,
]);

const GPT_PARTITION_TYPE_MICROSOFT_RESERVED: Guid = Guid::new(0xE3C9E316, 0x0B5C, 0x4DB8, [
    0x81, 0x7D, 0xF9, 0x2D, 0xF0, 0x02, 0x15, 0xAE,
]);

// =============================================================================
// MBR
// =============================================================================

/// Master Boot Record
#[derive(Debug, Clone)]
pub struct Mbr {
    /// Boot code (first 446 bytes)
    pub boot_code: [u8; 446],
    /// Partition entries
    pub partitions: [MbrPartition; 4],
    /// Disk signature
    pub signature: u32,
}

impl Mbr {
    /// Parse MBR from sector data
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 512 {
            return Err(Error::InvalidParameter);
        }

        // Check signature
        if data[510] != 0x55 || data[511] != 0xAA {
            return Err(Error::InvalidParameter);
        }

        let mut boot_code = [0u8; 446];
        boot_code.copy_from_slice(&data[0..446]);

        let signature = read_u32(data, 440);

        let mut partitions = [MbrPartition::default(); 4];
        for i in 0..4 {
            let offset = 446 + i * 16;
            partitions[i] = MbrPartition::parse(&data[offset..offset + 16]);
        }

        Ok(Self {
            boot_code,
            partitions,
            signature,
        })
    }

    /// Check if this is a protective MBR (GPT disk)
    pub fn is_protective(&self) -> bool {
        self.partitions[0].partition_type == 0xEE
    }
}

/// MBR partition entry
#[derive(Debug, Clone, Copy, Default)]
pub struct MbrPartition {
    /// Is bootable
    pub bootable: bool,
    /// Partition type
    pub partition_type: u8,
    /// Starting LBA
    pub start_lba: u32,
    /// Size in sectors
    pub size_lba: u32,
}

impl MbrPartition {
    fn parse(data: &[u8]) -> Self {
        Self {
            bootable: data[0] == 0x80,
            partition_type: data[4],
            start_lba: read_u32(data, 8),
            size_lba: read_u32(data, 12),
        }
    }
}

// =============================================================================
// GPT HEADER
// =============================================================================

/// GPT Header
#[derive(Debug, Clone)]
pub struct GptHeader {
    /// Signature ("EFI PART")
    pub signature: u64,
    /// Revision
    pub revision: u32,
    /// Header size
    pub header_size: u32,
    /// Header CRC32
    pub header_crc32: u32,
    /// Current LBA
    pub current_lba: u64,
    /// Backup LBA
    pub backup_lba: u64,
    /// First usable LBA
    pub first_usable_lba: u64,
    /// Last usable LBA
    pub last_usable_lba: u64,
    /// Disk GUID
    pub disk_guid: Guid,
    /// Partition entry LBA
    pub partition_entry_lba: u64,
    /// Number of partition entries
    pub entry_count: u32,
    /// Partition entry size
    pub entry_size: u32,
    /// Partition entries CRC32
    pub entries_crc32: u32,
}

impl GptHeader {
    /// GPT signature
    const SIGNATURE: u64 = 0x5452415020494645; // "EFI PART"

    /// Parse GPT header from sector data
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 92 {
            return Err(Error::InvalidParameter);
        }

        let signature = read_u64(data, 0);
        if signature != Self::SIGNATURE {
            return Err(Error::InvalidParameter);
        }

        Ok(Self {
            signature,
            revision: read_u32(data, 8),
            header_size: read_u32(data, 12),
            header_crc32: read_u32(data, 16),
            current_lba: read_u64(data, 24),
            backup_lba: read_u64(data, 32),
            first_usable_lba: read_u64(data, 40),
            last_usable_lba: read_u64(data, 48),
            disk_guid: read_guid(data, 56),
            partition_entry_lba: read_u64(data, 72),
            entry_count: read_u32(data, 80),
            entry_size: read_u32(data, 84),
            entries_crc32: read_u32(data, 88),
        })
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

fn read_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

fn read_u64(data: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
    ])
}

fn read_guid(data: &[u8], offset: usize) -> Guid {
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&data[offset..offset + 16]);

    // GUIDs in GPT are mixed-endian
    Guid::new(
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        u16::from_le_bytes([bytes[4], bytes[5]]),
        u16::from_le_bytes([bytes[6], bytes[7]]),
        [
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ],
    )
}

fn read_ucs2_string(data: &[u8]) -> String {
    let mut s = String::new();
    let mut i = 0;

    while i + 1 < data.len() {
        let c = u16::from_le_bytes([data[i], data[i + 1]]);
        if c == 0 {
            break;
        }
        if let Some(ch) = char::from_u32(c as u32) {
            s.push(ch);
        }
        i += 2;
    }

    s
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mbr_parse() {
        let mut data = [0u8; 512];
        data[510] = 0x55;
        data[511] = 0xAA;

        let mbr = Mbr::parse(&data).unwrap();
        assert!(!mbr.is_protective());
    }

    #[test]
    fn test_gpt_signature() {
        let mut data = [0u8; 512];
        // "EFI PART" in little endian
        data[0..8].copy_from_slice(&[0x45, 0x46, 0x49, 0x20, 0x50, 0x41, 0x52, 0x54]);
        data[8..12].copy_from_slice(&[0x00, 0x00, 0x01, 0x00]); // revision
        data[12..16].copy_from_slice(&[0x5C, 0x00, 0x00, 0x00]); // header size 92

        let gpt = GptHeader::parse(&data).unwrap();
        assert_eq!(gpt.header_size, 92);
    }

    #[test]
    fn test_partition_type_names() {
        assert_eq!(PartitionType::Mbr(0xEF).name(), "EFI System");
        assert_eq!(PartitionType::Mbr(0x83).name(), "Linux");
    }
}
