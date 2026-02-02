//! Disk layout calculations and region management.
//!
//! This module defines how the filesystem is laid out on disk, including
//! the locations and sizes of various regions like the superblock area,
//! journal, allocation bitmaps, and data area.

use crate::core::types::BlockNum;
use crate::BLOCK_SIZE;

/// Minimum filesystem size (64 MB)
pub const MIN_FS_SIZE_BLOCKS: u64 = 16384;

/// Maximum filesystem size (limited by 64-bit addressing)
pub const MAX_FS_SIZE_BLOCKS: u64 = u64::MAX / BLOCK_SIZE as u64;

/// Journal size as percentage of filesystem (default 1%)
pub const JOURNAL_PERCENT: u64 = 1;

/// Minimum journal size (8 MB = 2048 blocks)
pub const MIN_JOURNAL_BLOCKS: u64 = 2048;

/// Maximum journal size (1 GB = 262144 blocks)
pub const MAX_JOURNAL_BLOCKS: u64 = 262144;

/// Reserved blocks for root user (percentage)
pub const RESERVED_PERCENT: u64 = 5;

/// Disk layout calculator.
///
/// Determines the optimal placement of all filesystem regions
/// based on total disk size and configuration.
#[derive(Clone, Copy, Debug)]
pub struct DiskLayout {
    /// Block size in bytes
    pub block_size: u32,
    /// Total blocks in filesystem
    pub total_blocks: u64,

    // Region starts (in blocks)
    /// Primary superblock region start
    pub superblock_start: u64,
    /// Primary superblock region size (blocks)
    pub superblock_blocks: u64,

    /// Backup superblock region start
    pub backup_superblock_start: u64,
    /// Backup superblock region size
    pub backup_superblock_blocks: u64,

    /// Allocation bitmap start
    pub alloc_bitmap_start: u64,
    /// Allocation bitmap size (blocks)
    pub alloc_bitmap_blocks: u64,

    /// Inode bitmap start
    pub inode_bitmap_start: u64,
    /// Inode bitmap size (blocks)
    pub inode_bitmap_blocks: u64,

    /// Journal region start
    pub journal_start: u64,
    /// Journal region size (blocks)
    pub journal_blocks: u64,

    /// Metadata tree region start
    pub metadata_start: u64,
    /// Metadata tree region size (blocks)
    pub metadata_blocks: u64,

    /// Snapshot metadata start
    pub snapshot_start: u64,
    /// Snapshot metadata size (blocks)
    pub snapshot_blocks: u64,

    /// Crypto key storage start
    pub crypto_start: u64,
    /// Crypto key storage size (blocks)
    pub crypto_blocks: u64,

    /// Data region start
    pub data_start: u64,
    /// Data region size (blocks)
    pub data_blocks: u64,

    /// Reserved blocks for root
    pub reserved_blocks: u64,
}

impl DiskLayout {
    /// Calculate disk layout for given size.
    ///
    /// This function determines the optimal placement of all filesystem
    /// regions based on the total disk size and block size.
    pub fn calculate(total_blocks: u64, block_size: u32) -> Self {
        let block_size = block_size as u64;
        let bits_per_block = block_size * 8;

        // Superblock regions (16 blocks total for primary + backup)
        let superblock_start = 0;
        let superblock_blocks = 8;
        let backup_superblock_start = 8;
        let backup_superblock_blocks = 8;

        // Allocation bitmap: 1 bit per block
        // Each block can track block_size * 8 blocks
        let alloc_bitmap_start = 16;
        let alloc_bitmap_blocks = (total_blocks + bits_per_block - 1) / bits_per_block;

        // Inode bitmap (if using fixed inode allocation)
        // Assume 1 inode per 16KB of data
        let max_inodes = total_blocks * block_size / 16384;
        let inode_bitmap_start = alloc_bitmap_start + alloc_bitmap_blocks;
        let inode_bitmap_blocks = (max_inodes + bits_per_block - 1) / bits_per_block;

        // Journal: 1% of filesystem, bounded
        let journal_start = inode_bitmap_start + inode_bitmap_blocks;
        let journal_blocks = {
            let suggested = total_blocks * JOURNAL_PERCENT / 100;
            suggested.clamp(MIN_JOURNAL_BLOCKS, MAX_JOURNAL_BLOCKS)
        };

        // Metadata tree region (for B-trees): ~0.5% of filesystem
        let metadata_start = journal_start + journal_blocks;
        let metadata_blocks = (total_blocks / 200).max(1024);

        // Snapshot metadata: fixed size (16 MB = 4096 blocks)
        let snapshot_start = metadata_start + metadata_blocks;
        let snapshot_blocks = 4096;

        // Crypto key storage: fixed size (1 MB = 256 blocks)
        let crypto_start = snapshot_start + snapshot_blocks;
        let crypto_blocks = 256;

        // Data region: everything else
        let data_start = crypto_start + crypto_blocks;
        let data_blocks = total_blocks.saturating_sub(data_start);

        // Reserved blocks for root
        let reserved_blocks = data_blocks * RESERVED_PERCENT / 100;

        Self {
            block_size: block_size as u32,
            total_blocks,
            superblock_start,
            superblock_blocks,
            backup_superblock_start,
            backup_superblock_blocks,
            alloc_bitmap_start,
            alloc_bitmap_blocks,
            inode_bitmap_start,
            inode_bitmap_blocks,
            journal_start,
            journal_blocks,
            metadata_start,
            metadata_blocks,
            snapshot_start,
            snapshot_blocks,
            crypto_start,
            crypto_blocks,
            data_start,
            data_blocks,
            reserved_blocks,
        }
    }

    /// Calculate layout for given size in bytes
    pub fn calculate_bytes(total_bytes: u64, block_size: u32) -> Self {
        let total_blocks = total_bytes / block_size as u64;
        Self::calculate(total_blocks, block_size)
    }

    /// Get usable data blocks (excluding reserved)
    #[inline]
    pub fn usable_blocks(&self) -> u64 {
        self.data_blocks.saturating_sub(self.reserved_blocks)
    }

    /// Get total metadata overhead in blocks
    #[inline]
    pub fn metadata_overhead(&self) -> u64 {
        self.data_start
    }

    /// Get metadata overhead percentage
    #[inline]
    pub fn overhead_percent(&self) -> f64 {
        (self.metadata_overhead() as f64 / self.total_blocks as f64) * 100.0
    }

    /// Check if a block is in the data region
    #[inline]
    pub fn is_data_block(&self, block: BlockNum) -> bool {
        block.get() >= self.data_start && block.get() < self.total_blocks
    }

    /// Check if a block is in the journal region
    #[inline]
    pub fn is_journal_block(&self, block: BlockNum) -> bool {
        block.get() >= self.journal_start && block.get() < self.journal_start + self.journal_blocks
    }

    /// Check if a block is in the metadata region
    #[inline]
    pub fn is_metadata_block(&self, block: BlockNum) -> bool {
        block.get() >= self.metadata_start
            && block.get() < self.metadata_start + self.metadata_blocks
    }

    /// Get region containing a block
    pub fn block_region(&self, block: BlockNum) -> DiskRegion {
        let b = block.get();

        if b < self.superblock_blocks {
            DiskRegion::Superblock
        } else if b < self.backup_superblock_start + self.backup_superblock_blocks {
            DiskRegion::BackupSuperblock
        } else if b < self.alloc_bitmap_start + self.alloc_bitmap_blocks {
            DiskRegion::AllocBitmap
        } else if b < self.inode_bitmap_start + self.inode_bitmap_blocks {
            DiskRegion::InodeBitmap
        } else if b < self.journal_start + self.journal_blocks {
            DiskRegion::Journal
        } else if b < self.metadata_start + self.metadata_blocks {
            DiskRegion::Metadata
        } else if b < self.snapshot_start + self.snapshot_blocks {
            DiskRegion::Snapshot
        } else if b < self.crypto_start + self.crypto_blocks {
            DiskRegion::Crypto
        } else if b < self.total_blocks {
            DiskRegion::Data
        } else {
            DiskRegion::Invalid
        }
    }

    /// Validate layout
    pub fn validate(&self) -> Result<(), LayoutError> {
        // Check minimum size
        if self.total_blocks < MIN_FS_SIZE_BLOCKS {
            return Err(LayoutError::TooSmall);
        }

        // Check we have data blocks
        if self.data_blocks == 0 {
            return Err(LayoutError::NoDataBlocks);
        }

        // Check regions don't overlap
        let regions = [
            (self.superblock_start, self.superblock_blocks),
            (self.backup_superblock_start, self.backup_superblock_blocks),
            (self.alloc_bitmap_start, self.alloc_bitmap_blocks),
            (self.inode_bitmap_start, self.inode_bitmap_blocks),
            (self.journal_start, self.journal_blocks),
            (self.metadata_start, self.metadata_blocks),
            (self.snapshot_start, self.snapshot_blocks),
            (self.crypto_start, self.crypto_blocks),
            (self.data_start, self.data_blocks),
        ];

        for i in 0..regions.len() {
            let (start_i, len_i) = regions[i];
            if len_i == 0 {
                continue;
            }
            let end_i = start_i + len_i;

            for j in (i + 1)..regions.len() {
                let (start_j, len_j) = regions[j];
                if len_j == 0 {
                    continue;
                }
                let end_j = start_j + len_j;

                // Check for overlap
                if start_i < end_j && start_j < end_i {
                    return Err(LayoutError::Overlap);
                }
            }
        }

        // Check total doesn't exceed disk
        if self.data_start + self.data_blocks > self.total_blocks {
            return Err(LayoutError::Overflow);
        }

        Ok(())
    }
}

/// Disk region types
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DiskRegion {
    Superblock,
    BackupSuperblock,
    AllocBitmap,
    InodeBitmap,
    Journal,
    Metadata,
    Snapshot,
    Crypto,
    Data,
    Invalid,
}

impl DiskRegion {
    /// Check if this region is writable during normal operation
    pub fn is_writable(&self) -> bool {
        matches!(
            self,
            Self::Journal | Self::Metadata | Self::Snapshot | Self::Data
        )
    }

    /// Check if this is a metadata region
    pub fn is_metadata(&self) -> bool {
        !matches!(self, Self::Data | Self::Invalid)
    }
}

/// Layout calculation errors
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LayoutError {
    /// Filesystem too small
    TooSmall,
    /// No data blocks available
    NoDataBlocks,
    /// Regions overlap
    Overlap,
    /// Size overflow
    Overflow,
}

/// Allocation group for NUMA-aware and parallel allocation.
///
/// Large filesystems are divided into allocation groups to allow
/// parallel allocation and reduce lock contention.
#[derive(Clone, Copy, Debug)]
pub struct AllocGroup {
    /// Group index
    pub index: u32,
    /// First block in this group
    pub start_block: u64,
    /// Number of blocks in this group
    pub block_count: u64,
    /// Free blocks in this group
    pub free_blocks: u64,
    /// First free block hint
    pub first_free: u64,
}

impl AllocGroup {
    /// Target allocation group size (1 GB)
    pub const TARGET_SIZE_BLOCKS: u64 = 262144; // 1GB / 4KB

    /// Calculate allocation groups for a filesystem
    pub fn calculate_groups(layout: &DiskLayout) -> impl Iterator<Item = AllocGroup> {
        let data_start = layout.data_start;
        let data_blocks = layout.data_blocks;
        let group_size = Self::TARGET_SIZE_BLOCKS;

        let num_groups = (data_blocks + group_size - 1) / group_size;

        (0..num_groups as u32).map(move |i| {
            let start = data_start + (i as u64) * group_size;
            let remaining = data_blocks.saturating_sub((i as u64) * group_size);
            let count = remaining.min(group_size);

            AllocGroup {
                index: i,
                start_block: start,
                block_count: count,
                free_blocks: count, // Initially all free
                first_free: start,
            }
        })
    }
}

// ============================================================================
// Disk Statistics
// ============================================================================

/// Filesystem space statistics
#[derive(Clone, Copy, Debug, Default)]
pub struct SpaceStats {
    /// Total space in bytes
    pub total_bytes: u64,
    /// Used space in bytes
    pub used_bytes: u64,
    /// Free space in bytes
    pub free_bytes: u64,
    /// Reserved space in bytes
    pub reserved_bytes: u64,
    /// Metadata overhead in bytes
    pub metadata_bytes: u64,
    /// Data space in bytes
    pub data_bytes: u64,
    /// Snapshot space in bytes
    pub snapshot_bytes: u64,
}

impl SpaceStats {
    /// Calculate from layout and allocation info
    pub fn from_layout(layout: &DiskLayout, free_blocks: u64) -> Self {
        let block_size = layout.block_size as u64;
        let total_bytes = layout.total_blocks * block_size;
        let data_bytes = layout.data_blocks * block_size;
        let free_bytes = free_blocks * block_size;
        let metadata_bytes = layout.metadata_overhead() * block_size;
        let reserved_bytes = layout.reserved_blocks * block_size;
        let used_bytes = total_bytes - free_bytes - reserved_bytes;

        Self {
            total_bytes,
            used_bytes,
            free_bytes,
            reserved_bytes,
            metadata_bytes,
            data_bytes,
            snapshot_bytes: 0, // Updated separately
        }
    }

    /// Get usage percentage (0-100)
    pub fn usage_percent(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.used_bytes as f64 / self.total_bytes as f64) * 100.0
        }
    }

    /// Format as human-readable string
    #[cfg(feature = "alloc")]
    pub fn format(&self) -> alloc_crate::string::String {
        extern crate alloc as alloc_crate;
        use alloc_crate::format;

        fn format_size(bytes: u64) -> alloc_crate::string::String {
            if bytes >= 1024 * 1024 * 1024 * 1024 {
                format!(
                    "{:.2} TB",
                    bytes as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0)
                )
            } else if bytes >= 1024 * 1024 * 1024 {
                format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
            } else if bytes >= 1024 * 1024 {
                format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
            } else if bytes >= 1024 {
                format!("{:.2} KB", bytes as f64 / 1024.0)
            } else {
                format!("{} B", bytes)
            }
        }

        format!(
            "Total: {}, Used: {} ({:.1}%), Free: {}, Reserved: {}",
            format_size(self.total_bytes),
            format_size(self.used_bytes),
            self.usage_percent(),
            format_size(self.free_bytes),
            format_size(self.reserved_bytes)
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_small_fs() {
        // 256 MB filesystem
        let layout = DiskLayout::calculate(65536, 4096);

        assert_eq!(layout.superblock_start, 0);
        assert_eq!(layout.superblock_blocks, 8);
        assert!(layout.data_blocks > 0);
        assert!(layout.validate().is_ok());
    }

    #[test]
    fn test_layout_large_fs() {
        // 1 TB filesystem
        let blocks_1tb = 1024 * 1024 * 1024 / 4; // 256M blocks
        let layout = DiskLayout::calculate(blocks_1tb, 4096);

        assert!(layout.validate().is_ok());
        assert!(layout.journal_blocks >= MIN_JOURNAL_BLOCKS);
        assert!(layout.journal_blocks <= MAX_JOURNAL_BLOCKS);

        // Check overhead is reasonable (<5%)
        assert!(layout.overhead_percent() < 5.0);
    }

    #[test]
    fn test_layout_regions() {
        let layout = DiskLayout::calculate(1_000_000, 4096);

        // Check first data block
        assert!(layout.is_data_block(BlockNum::new(layout.data_start)));

        // Check journal block
        assert!(layout.is_journal_block(BlockNum::new(layout.journal_start)));
        assert!(!layout.is_journal_block(BlockNum::new(layout.data_start)));

        // Check region classification
        assert_eq!(
            layout.block_region(BlockNum::new(0)),
            DiskRegion::Superblock
        );
        assert_eq!(
            layout.block_region(BlockNum::new(layout.data_start)),
            DiskRegion::Data
        );
    }

    #[test]
    fn test_alloc_groups() {
        let layout = DiskLayout::calculate(10_000_000, 4096); // ~40 GB
        let groups: alloc::vec::Vec<_> = AllocGroup::calculate_groups(&layout).collect();

        assert!(groups.len() > 1);

        // First group starts at data region
        assert_eq!(groups[0].start_block, layout.data_start);

        // Groups cover all data blocks
        let total_group_blocks: u64 = groups.iter().map(|g| g.block_count).sum();
        assert_eq!(total_group_blocks, layout.data_blocks);
    }

    #[test]
    fn test_layout_too_small() {
        let layout = DiskLayout::calculate(1000, 4096); // Too small
        assert!(matches!(layout.validate(), Err(LayoutError::TooSmall)));
    }
}
