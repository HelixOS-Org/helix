//! File metadata for analysis.

use super::types::{FileType, Inode};
use crate::core::NexusTimestamp;

// ============================================================================
// FILE METADATA
// ============================================================================

/// File metadata for analysis
#[derive(Debug, Clone)]
pub struct FileMeta {
    /// Inode number
    pub inode: Inode,
    /// File type
    pub file_type: FileType,
    /// File size
    pub size: u64,
    /// Block count
    pub blocks: u64,
    /// Access count
    pub access_count: u64,
    /// Last access time
    pub last_access: NexusTimestamp,
    /// Last modification time
    pub last_modified: NexusTimestamp,
    /// Creation time
    pub created: NexusTimestamp,
    /// Is file hot (frequently accessed)?
    pub is_hot: bool,
    /// Fragmentation level (0.0 - 1.0)
    pub fragmentation: f64,
    /// Parent directory inode
    pub parent_inode: Option<Inode>,
}

impl FileMeta {
    /// Create new file metadata
    pub fn new(inode: Inode, file_type: FileType, size: u64) -> Self {
        let now = NexusTimestamp::now();
        Self {
            inode,
            file_type,
            size,
            blocks: (size + 4095) / 4096,
            access_count: 0,
            last_access: now,
            last_modified: now,
            created: now,
            is_hot: false,
            fragmentation: 0.0,
            parent_inode: None,
        }
    }

    /// Record access
    #[inline]
    pub fn record_access(&mut self) {
        self.access_count += 1;
        self.last_access = NexusTimestamp::now();

        // Update hot status
        self.is_hot = self.access_count > 100
            && self.last_access.duration_since(self.created) < 3_600_000_000_000;
    }

    /// Get access rate (accesses per hour)
    #[inline]
    pub fn access_rate(&self) -> f64 {
        let age = self.last_access.duration_since(self.created);
        if age == 0 {
            0.0
        } else {
            self.access_count as f64 * 3_600_000_000_000.0 / age as f64
        }
    }

    /// Is file cold (rarely accessed)?
    #[inline(always)]
    pub fn is_cold(&self) -> bool {
        let idle = NexusTimestamp::now().duration_since(self.last_access);
        idle > 86_400_000_000_000 && self.access_count < 10 // 24 hours
    }
}
