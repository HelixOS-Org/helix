//! Block I/O Protocol
//!
//! Provides raw block-level access to storage devices.

use core::fmt;

use crate::raw::types::{Boolean, Char16, Event, Guid, Status, guids};

// =============================================================================
// BLOCK I/O PROTOCOL
// =============================================================================

/// Block I/O Protocol
#[repr(C)]
#[derive(Debug)]
pub struct EfiBlockIoProtocol {
    /// Revision of the protocol
    pub revision: u64,

    /// Media information
    pub media: *mut EfiBlockIoMedia,

    /// Reset the block device
    pub reset: unsafe extern "efiapi" fn(this: *mut Self, extended_verification: Boolean) -> Status,

    /// Read blocks from the device
    pub read_blocks: unsafe extern "efiapi" fn(
        this: *mut Self,
        media_id: u32,
        lba: u64,
        buffer_size: usize,
        buffer: *mut u8,
    ) -> Status,

    /// Write blocks to the device
    pub write_blocks: unsafe extern "efiapi" fn(
        this: *mut Self,
        media_id: u32,
        lba: u64,
        buffer_size: usize,
        buffer: *const u8,
    ) -> Status,

    /// Flush blocks to the device
    pub flush_blocks: unsafe extern "efiapi" fn(this: *mut Self) -> Status,
}

impl EfiBlockIoProtocol {
    /// Protocol GUID
    pub const GUID: Guid = guids::BLOCK_IO_PROTOCOL;

    /// Protocol revision 1.0
    pub const REVISION_1: u64 = 0x0001_0000;
    /// Protocol revision 2.0
    pub const REVISION_2: u64 = 0x0002_0001;
    /// Protocol revision 3.0
    pub const REVISION_3: u64 = 0x0002_001F;

    /// Reset the device
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn reset(&mut self, extended_verification: bool) -> Result<(), Status> {
        let status = (self.reset)(self, Boolean::from(extended_verification));
        status.to_status_result()
    }

    /// Read blocks from the device
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer and buffer are valid.
    pub unsafe fn read_blocks(&mut self, lba: u64, buffer: &mut [u8]) -> Result<(), Status> {
        let media_id = self.media_info().ok_or(Status::DEVICE_ERROR)?.media_id;
        let status = (self.read_blocks)(self, media_id, lba, buffer.len(), buffer.as_mut_ptr());
        status.to_status_result()
    }

    /// Write blocks to the device
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn write_blocks(&mut self, lba: u64, buffer: &[u8]) -> Result<(), Status> {
        let media_id = self.media_info().ok_or(Status::DEVICE_ERROR)?.media_id;
        let status = (self.write_blocks)(self, media_id, lba, buffer.len(), buffer.as_ptr());
        status.to_status_result()
    }

    /// Flush pending writes
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn flush(&mut self) -> Result<(), Status> {
        let status = (self.flush_blocks)(self);
        status.to_status_result()
    }

    /// Get media information
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn media_info(&self) -> Option<&EfiBlockIoMedia> {
        if self.media.is_null() {
            None
        } else {
            Some(&*self.media)
        }
    }

    /// Get the block size
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn block_size(&self) -> Option<u32> {
        self.media_info().map(|m| m.block_size)
    }

    /// Get the total number of blocks
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn last_block(&self) -> Option<u64> {
        self.media_info().map(|m| m.last_block)
    }

    /// Calculate total size in bytes
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn total_size(&self) -> Option<u64> {
        self.media_info()
            .map(|m| (m.last_block + 1) * u64::from(m.block_size))
    }
}

// =============================================================================
// BLOCK I/O MEDIA
// =============================================================================

/// Block I/O media information
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EfiBlockIoMedia {
    /// Media ID
    pub media_id: u32,
    /// Removable media flag
    pub removable_media: Boolean,
    /// Media present flag
    pub media_present: Boolean,
    /// Logical partition flag
    pub logical_partition: Boolean,
    /// Read-only flag
    pub read_only: Boolean,
    /// Write caching flag
    pub write_caching: Boolean,
    /// Block size in bytes
    pub block_size: u32,
    /// I/O alignment
    pub io_align: u32,
    /// Last block number (0-indexed)
    pub last_block: u64,

    // Revision 2.0+
    /// Lowest aligned LBA
    pub lowest_aligned_lba: u64,
    /// Logical blocks per physical block
    pub logical_blocks_per_physical_block: u32,

    // Revision 3.0+
    /// Optimal transfer length granularity
    pub optimal_transfer_length_granularity: u32,
}

impl EfiBlockIoMedia {
    /// Check if media is present
    pub const fn is_present(&self) -> bool {
        self.media_present != 0
    }

    /// Check if media is removable
    pub const fn is_removable(&self) -> bool {
        self.removable_media != 0
    }

    /// Check if media is read-only
    pub const fn is_read_only(&self) -> bool {
        self.read_only != 0
    }

    /// Check if this is a logical partition
    pub const fn is_partition(&self) -> bool {
        self.logical_partition != 0
    }

    /// Check if write caching is enabled
    pub const fn has_write_cache(&self) -> bool {
        self.write_caching != 0
    }

    /// Get total size in bytes
    pub fn total_size(&self) -> u64 {
        (self.last_block + 1) * u64::from(self.block_size)
    }

    /// Get total number of blocks
    pub const fn block_count(&self) -> u64 {
        self.last_block + 1
    }
}

// =============================================================================
// BLOCK I/O 2 PROTOCOL
// =============================================================================

/// Block I/O 2 Protocol (async operations)
#[repr(C)]
#[derive(Debug)]
pub struct EfiBlockIo2Protocol {
    /// Media information
    pub media: *mut EfiBlockIoMedia,

    /// Reset the device
    pub reset: unsafe extern "efiapi" fn(this: *mut Self, extended_verification: Boolean) -> Status,

    /// Read blocks (async)
    pub read_blocks_ex: unsafe extern "efiapi" fn(
        this: *mut Self,
        media_id: u32,
        lba: u64,
        token: *mut EfiBlockIo2Token,
        buffer_size: usize,
        buffer: *mut u8,
    ) -> Status,

    /// Write blocks (async)
    pub write_blocks_ex: unsafe extern "efiapi" fn(
        this: *mut Self,
        media_id: u32,
        lba: u64,
        token: *mut EfiBlockIo2Token,
        buffer_size: usize,
        buffer: *const u8,
    ) -> Status,

    /// Flush blocks (async)
    pub flush_blocks_ex:
        unsafe extern "efiapi" fn(this: *mut Self, token: *mut EfiBlockIo2Token) -> Status,
}

impl EfiBlockIo2Protocol {
    /// Protocol GUID
    pub const GUID: Guid = guids::BLOCK_IO2_PROTOCOL;
}

/// Block I/O 2 token for async operations
#[derive(Debug)]
#[repr(C)]
pub struct EfiBlockIo2Token {
    /// Event to signal on completion
    pub event: Event,
    /// Status of the operation
    pub transaction_status: Status,
}

// =============================================================================
// DISK I/O PROTOCOL
// =============================================================================

/// Disk I/O Protocol
#[repr(C)]
#[derive(Debug)]
pub struct EfiDiskIoProtocol {
    /// Revision of the protocol
    pub revision: u64,

    /// Read from disk
    pub read_disk: unsafe extern "efiapi" fn(
        this: *mut Self,
        media_id: u32,
        offset: u64,
        buffer_size: usize,
        buffer: *mut u8,
    ) -> Status,

    /// Write to disk
    pub write_disk: unsafe extern "efiapi" fn(
        this: *mut Self,
        media_id: u32,
        offset: u64,
        buffer_size: usize,
        buffer: *const u8,
    ) -> Status,
}

impl EfiDiskIoProtocol {
    /// Protocol GUID
    pub const GUID: Guid = guids::DISK_IO_PROTOCOL;

    /// Protocol revision 1.0
    pub const REVISION_1: u64 = 0x0001_0000;

    /// Read from disk at byte offset
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer and buffer are valid.
    pub unsafe fn read(
        &mut self,
        media_id: u32,
        offset: u64,
        buffer: &mut [u8],
    ) -> Result<(), Status> {
        let status = (self.read_disk)(self, media_id, offset, buffer.len(), buffer.as_mut_ptr());
        status.to_status_result()
    }

    /// Write to disk at byte offset
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn write(
        &mut self,
        media_id: u32,
        offset: u64,
        buffer: &[u8],
    ) -> Result<(), Status> {
        let status = (self.write_disk)(self, media_id, offset, buffer.len(), buffer.as_ptr());
        status.to_status_result()
    }
}

// =============================================================================
// DISK I/O 2 PROTOCOL
// =============================================================================

/// Disk I/O 2 Protocol (async operations)
#[repr(C)]
#[derive(Debug)]
pub struct EfiDiskIo2Protocol {
    /// Revision of the protocol
    pub revision: u64,

    /// Cancel outstanding operations
    pub cancel: unsafe extern "efiapi" fn(this: *mut Self) -> Status,

    /// Read from disk (async)
    pub read_disk_ex: unsafe extern "efiapi" fn(
        this: *mut Self,
        media_id: u32,
        offset: u64,
        token: *mut EfiDiskIo2Token,
        buffer_size: usize,
        buffer: *mut u8,
    ) -> Status,

    /// Write to disk (async)
    pub write_disk_ex: unsafe extern "efiapi" fn(
        this: *mut Self,
        media_id: u32,
        offset: u64,
        token: *mut EfiDiskIo2Token,
        buffer_size: usize,
        buffer: *const u8,
    ) -> Status,

    /// Flush pending writes (async)
    pub flush_disk_ex:
        unsafe extern "efiapi" fn(this: *mut Self, token: *mut EfiDiskIo2Token) -> Status,
}

impl EfiDiskIo2Protocol {
    /// Protocol GUID
    pub const GUID: Guid = guids::DISK_IO2_PROTOCOL;
}

/// Disk I/O 2 token for async operations
#[derive(Debug)]
#[repr(C)]
pub struct EfiDiskIo2Token {
    /// Event to signal on completion
    pub event: Event,
    /// Status of the operation
    pub transaction_status: Status,
}

// =============================================================================
// PARTITION INFO PROTOCOL
// =============================================================================

/// Partition type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum EfiPartitionType {
    /// Other partition type
    Other = 0,
    /// MBR partition
    Mbr   = 1,
    /// GPT partition
    Gpt   = 2,
}

/// Partition Info Protocol
#[repr(C)]
pub struct EfiPartitionInfoProtocol {
    /// Revision of the protocol
    pub revision: u32,
    /// Type of partition
    pub partition_type: EfiPartitionType,
    /// System flag
    pub system: Boolean,
    /// Reserved
    pub reserved: [u8; 7],
    /// Partition info union (depends on `partition_type`)
    pub info: EfiPartitionInfoUnion,
}

impl EfiPartitionInfoProtocol {
    /// Protocol GUID
    pub const GUID: Guid = guids::PARTITION_INFO_PROTOCOL;

    /// Protocol revision 1.0
    pub const REVISION_1: u32 = 0x0001_0000;

    /// Check if this is a GPT partition
    pub fn is_gpt(&self) -> bool {
        self.partition_type == EfiPartitionType::Gpt
    }

    /// Check if this is an MBR partition
    pub fn is_mbr(&self) -> bool {
        self.partition_type == EfiPartitionType::Mbr
    }

    /// Check if this is a system partition
    pub const fn is_system(&self) -> bool {
        self.system != 0
    }
}

impl fmt::Debug for EfiPartitionInfoProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EfiPartitionInfoProtocol")
            .field("revision", &self.revision)
            .field("partition_type", &self.partition_type)
            .field("system", &(self.system != 0))
            .field("reserved", &self.reserved)
            .finish_non_exhaustive()
    }
}

/// Partition info union
#[repr(C)]
pub union EfiPartitionInfoUnion {
    /// MBR partition entry
    pub mbr: MbrPartitionRecord,
    /// GPT partition entry
    pub gpt: GptPartitionEntry,
}

/// MBR partition record
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MbrPartitionRecord {
    /// Boot indicator
    pub boot_indicator: u8,
    /// Starting head
    pub start_head: u8,
    /// Starting sector
    pub start_sector: u8,
    /// Starting track
    pub start_track: u8,
    /// OS indicator / partition type
    pub os_indicator: u8,
    /// Ending head
    pub end_head: u8,
    /// Ending sector
    pub end_sector: u8,
    /// Ending track
    pub end_track: u8,
    /// Starting LBA
    pub starting_lba: [u8; 4],
    /// Size in LBA
    pub size_in_lba: [u8; 4],
}

impl MbrPartitionRecord {
    /// Get starting LBA
    pub const fn starting_lba(&self) -> u32 {
        u32::from_le_bytes(self.starting_lba)
    }

    /// Get size in LBA
    pub const fn size_in_lba(&self) -> u32 {
        u32::from_le_bytes(self.size_in_lba)
    }

    /// Check if bootable
    pub const fn is_bootable(&self) -> bool {
        self.boot_indicator == 0x80
    }
}

/// GPT partition entry
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GptPartitionEntry {
    /// Partition type GUID
    pub partition_type_guid: Guid,
    /// Unique partition GUID
    pub unique_partition_guid: Guid,
    /// Starting LBA
    pub starting_lba: u64,
    /// Ending LBA
    pub ending_lba: u64,
    /// Attributes
    pub attributes: u64,
    /// Partition name (36 UCS-2 characters)
    pub partition_name: [Char16; 36],
}

impl GptPartitionEntry {
    /// GPT partition attribute: Required for platform
    pub const ATTR_REQUIRED: u64 = 0x0000_0000_0000_0001;
    /// GPT partition attribute: No block I/O protocol
    pub const ATTR_NO_BLOCK_IO: u64 = 0x0000_0000_0000_0002;
    /// GPT partition attribute: Legacy BIOS bootable
    pub const ATTR_LEGACY_BIOS_BOOTABLE: u64 = 0x0000_0000_0000_0004;

    /// Get partition size in blocks
    pub const fn size_in_blocks(&self) -> u64 {
        self.ending_lba - self.starting_lba + 1
    }

    /// Check if this is a required partition
    pub const fn is_required(&self) -> bool {
        (self.attributes & Self::ATTR_REQUIRED) != 0
    }

    /// Check if this is bootable (legacy BIOS)
    pub const fn is_legacy_bootable(&self) -> bool {
        (self.attributes & Self::ATTR_LEGACY_BIOS_BOOTABLE) != 0
    }
}

// =============================================================================
// WELL-KNOWN PARTITION TYPE GUIDS
// =============================================================================

/// Well-known GPT partition type GUIDs
pub mod partition_types {
    use super::Guid;

    /// Unused entry
    pub const UNUSED: Guid = Guid::NULL;

    /// EFI System Partition
    pub const EFI_SYSTEM: Guid = Guid::new(0xC12A7328, 0xF81F, 0x11D2, [
        0xBA, 0x4B, 0x00, 0xA0, 0xC9, 0x3E, 0xC9, 0x3B,
    ]);

    /// Microsoft Reserved Partition
    pub const MICROSOFT_RESERVED: Guid = Guid::new(0xE3C9E316, 0x0B5C, 0x4DB8, [
        0x81, 0x7D, 0xF9, 0x2D, 0xF0, 0x02, 0x15, 0xAE,
    ]);

    /// Microsoft Basic Data Partition
    pub const MICROSOFT_BASIC_DATA: Guid = Guid::new(0xEBD0A0A2, 0xB9E5, 0x4433, [
        0x87, 0xC0, 0x68, 0xB6, 0xB7, 0x26, 0x99, 0xC7,
    ]);

    /// Linux Filesystem Data
    pub const LINUX_FILESYSTEM: Guid = Guid::new(0x0FC63DAF, 0x8483, 0x4772, [
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

    /// Linux Root (`AArch64`)
    pub const LINUX_ROOT_ARM64: Guid = Guid::new(0xB921B045, 0x1DF0, 0x41C3, [
        0xAF, 0x44, 0x4C, 0x6F, 0x28, 0x0D, 0x3F, 0xAE,
    ]);

    /// Linux Home
    pub const LINUX_HOME: Guid = Guid::new(0x933AC7E1, 0x2EB4, 0x4F13, [
        0xB8, 0x44, 0x0E, 0x14, 0xE2, 0xAE, 0xF9, 0x15,
    ]);
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_block_io_media() {
        let media = EfiBlockIoMedia {
            media_id: 1,
            removable_media: 0,
            media_present: 1,
            logical_partition: 0,
            read_only: 0,
            write_caching: 1,
            block_size: 512,
            io_align: 0,
            last_block: 1023,
            lowest_aligned_lba: 0,
            logical_blocks_per_physical_block: 1,
            optimal_transfer_length_granularity: 1,
        };

        assert!(media.is_present());
        assert!(!media.is_removable());
        assert!(!media.is_read_only());
        assert_eq!(media.block_count(), 1024);
        assert_eq!(media.total_size(), 512 * 1024);
    }

    #[test]
    fn test_gpt_partition_entry() {
        let entry = GptPartitionEntry {
            partition_type_guid: partition_types::EFI_SYSTEM,
            unique_partition_guid: Guid::NULL,
            starting_lba: 2048,
            ending_lba: 4095,
            attributes: GptPartitionEntry::ATTR_REQUIRED,
            partition_name: [0; 36],
        };

        assert_eq!(entry.size_in_blocks(), 2048);
        assert!(entry.is_required());
    }
}
