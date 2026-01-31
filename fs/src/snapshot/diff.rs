//! Snapshot diff computation.
//!
//! Computes differences between two snapshots efficiently
//! using CoW metadata.

use crate::core::types::*;
use crate::core::error::{HfsError, HfsResult};
use crate::snapshot::{SnapshotId, ROOT_SNAPSHOT_ID};

// ============================================================================
// Diff Types
// ============================================================================

/// Type of change in diff.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DiffType {
    /// No change
    None = 0,
    /// Item added
    Added = 1,
    /// Item removed
    Removed = 2,
    /// Item modified
    Modified = 3,
    /// Item renamed
    Renamed = 4,
    /// Type changed (e.g., file to directory)
    TypeChanged = 5,
}

impl DiffType {
    /// From raw value
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::Added,
            2 => Self::Removed,
            3 => Self::Modified,
            4 => Self::Renamed,
            5 => Self::TypeChanged,
            _ => Self::None,
        }
    }
    
    /// Check if is change
    #[inline]
    pub fn is_change(&self) -> bool {
        *self != Self::None
    }
}

impl Default for DiffType {
    fn default() -> Self {
        Self::None
    }
}

// ============================================================================
// Diff Entry
// ============================================================================

/// Single diff entry.
#[derive(Clone, Copy, Debug)]
pub struct DiffEntry {
    /// Inode number
    pub ino: u64,
    /// Parent inode
    pub parent_ino: u64,
    /// Change type
    pub diff_type: DiffType,
    /// File type
    pub file_type: FileType,
    /// Old generation (in source snapshot)
    pub old_gen: u32,
    /// New generation (in target snapshot)
    pub new_gen: u32,
    /// Old size
    pub old_size: u64,
    /// New size
    pub new_size: u64,
    /// Name hash
    pub name_hash: u64,
    /// Flags
    pub flags: u8,
}

impl DiffEntry {
    /// Create new entry
    pub fn new(ino: u64, diff_type: DiffType) -> Self {
        Self {
            ino,
            parent_ino: 0,
            diff_type,
            file_type: FileType::Regular,
            old_gen: 0,
            new_gen: 0,
            old_size: 0,
            new_size: 0,
            name_hash: 0,
            flags: 0,
        }
    }
    
    /// Create added entry
    pub fn added(ino: u64, file_type: FileType) -> Self {
        Self {
            ino,
            parent_ino: 0,
            diff_type: DiffType::Added,
            file_type,
            old_gen: 0,
            new_gen: 1,
            old_size: 0,
            new_size: 0,
            name_hash: 0,
            flags: 0,
        }
    }
    
    /// Create removed entry
    pub fn removed(ino: u64, file_type: FileType) -> Self {
        Self {
            ino,
            parent_ino: 0,
            diff_type: DiffType::Removed,
            file_type,
            old_gen: 1,
            new_gen: 0,
            old_size: 0,
            new_size: 0,
            name_hash: 0,
            flags: 0,
        }
    }
    
    /// Create modified entry
    pub fn modified(ino: u64, old_size: u64, new_size: u64) -> Self {
        Self {
            ino,
            parent_ino: 0,
            diff_type: DiffType::Modified,
            file_type: FileType::Regular,
            old_gen: 1,
            new_gen: 2,
            old_size,
            new_size,
            name_hash: 0,
            flags: 0,
        }
    }
    
    /// Size delta
    pub fn size_delta(&self) -> i64 {
        self.new_size as i64 - self.old_size as i64
    }
}

impl Default for DiffEntry {
    fn default() -> Self {
        Self::new(0, DiffType::None)
    }
}

// ============================================================================
// Diff Options
// ============================================================================

/// Options for diff computation.
#[derive(Clone, Copy, Debug)]
pub struct DiffOptions {
    /// Include content diffs (block-level)
    pub include_content: bool,
    /// Include metadata changes
    pub include_metadata: bool,
    /// Maximum entries to return
    pub max_entries: usize,
    /// Filter by file type
    pub file_type_filter: Option<FileType>,
    /// Filter by parent directory
    pub parent_filter: Option<u64>,
    /// Recursive (include subdirectories)
    pub recursive: bool,
}

impl DiffOptions {
    /// Default options
    pub const fn default() -> Self {
        Self {
            include_content: false,
            include_metadata: true,
            max_entries: 10000,
            file_type_filter: None,
            parent_filter: None,
            recursive: true,
        }
    }
    
    /// Include content diffs
    pub fn with_content(mut self) -> Self {
        self.include_content = true;
        self
    }
    
    /// Limit entries
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.max_entries = limit;
        self
    }
    
    /// Filter by file type
    pub fn files_only(mut self) -> Self {
        self.file_type_filter = Some(FileType::Regular);
        self
    }
    
    /// Filter by directory
    pub fn in_directory(mut self, parent: u64) -> Self {
        self.parent_filter = Some(parent);
        self
    }
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self::default()
    }
}

// ============================================================================
// Block Diff
// ============================================================================

/// Block-level diff entry.
#[derive(Clone, Copy, Debug)]
pub struct BlockDiff {
    /// Inode number
    pub ino: u64,
    /// Logical block offset
    pub logical: u64,
    /// Old physical block (0 if new)
    pub old_physical: BlockNum,
    /// New physical block (0 if deleted)
    pub new_physical: BlockNum,
    /// Block count
    pub count: u32,
    /// Flags
    pub flags: u8,
}

impl BlockDiff {
    /// Create new block diff
    pub fn new(ino: u64, logical: u64, old: BlockNum, new: BlockNum, count: u32) -> Self {
        Self {
            ino,
            logical,
            old_physical: old,
            new_physical: new,
            count,
            flags: 0,
        }
    }
    
    /// Check if is addition
    #[inline]
    pub fn is_added(&self) -> bool {
        self.old_physical.is_null() && !self.new_physical.is_null()
    }
    
    /// Check if is removal
    #[inline]
    pub fn is_removed(&self) -> bool {
        !self.old_physical.is_null() && self.new_physical.is_null()
    }
    
    /// Check if is change
    #[inline]
    pub fn is_changed(&self) -> bool {
        self.old_physical != self.new_physical
    }
}

// ============================================================================
// Diff Result
// ============================================================================

/// Maximum entries in diff result (for fixed buffer)
const MAX_DIFF_ENTRIES: usize = 1024;

/// Diff computation result.
pub struct DiffResult {
    /// Source snapshot
    pub source: SnapshotId,
    /// Target snapshot
    pub target: SnapshotId,
    /// Entries
    pub entries: [DiffEntry; MAX_DIFF_ENTRIES],
    /// Entry count
    pub count: usize,
    /// Has more entries
    pub has_more: bool,
    /// Statistics
    pub stats: DiffStats,
}

impl DiffResult {
    /// Create new result
    pub fn new(source: SnapshotId, target: SnapshotId) -> Self {
        Self {
            source,
            target,
            entries: [DiffEntry::default(); MAX_DIFF_ENTRIES],
            count: 0,
            has_more: false,
            stats: DiffStats::new(),
        }
    }
    
    /// Add entry
    pub fn add(&mut self, entry: DiffEntry) -> HfsResult<()> {
        if self.count >= MAX_DIFF_ENTRIES {
            self.has_more = true;
            return Err(HfsError::OutOfMemory);
        }
        
        self.entries[self.count] = entry;
        self.count += 1;
        
        // Update stats
        match entry.diff_type {
            DiffType::Added => self.stats.added += 1,
            DiffType::Removed => self.stats.removed += 1,
            DiffType::Modified => self.stats.modified += 1,
            DiffType::Renamed => self.stats.renamed += 1,
            DiffType::TypeChanged => self.stats.type_changed += 1,
            _ => {}
        }
        
        Ok(())
    }
    
    /// Get entries as slice
    pub fn entries(&self) -> &[DiffEntry] {
        &self.entries[..self.count]
    }
    
    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    
    /// Total changes
    #[inline]
    pub fn total_changes(&self) -> usize {
        self.count
    }
}

/// Diff statistics.
#[derive(Clone, Copy, Debug, Default)]
pub struct DiffStats {
    /// Items added
    pub added: u64,
    /// Items removed
    pub removed: u64,
    /// Items modified
    pub modified: u64,
    /// Items renamed
    pub renamed: u64,
    /// Items type-changed
    pub type_changed: u64,
    /// Inodes compared
    pub inodes_compared: u64,
    /// Blocks compared
    pub blocks_compared: u64,
    /// Computation time (ns)
    pub compute_time_ns: u64,
}

impl DiffStats {
    /// Create new stats
    pub const fn new() -> Self {
        Self {
            added: 0,
            removed: 0,
            modified: 0,
            renamed: 0,
            type_changed: 0,
            inodes_compared: 0,
            blocks_compared: 0,
            compute_time_ns: 0,
        }
    }
    
    /// Total changes
    #[inline]
    pub fn total_changes(&self) -> u64 {
        self.added + self.removed + self.modified + self.renamed + self.type_changed
    }
    
    /// Is empty (no changes)
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.total_changes() == 0
    }
}

// ============================================================================
// Diff Context
// ============================================================================

/// Context for diff computation.
pub struct DiffContext {
    /// Source snapshot ID
    pub source: SnapshotId,
    /// Target snapshot ID
    pub target: SnapshotId,
    /// Common ancestor
    pub ancestor: SnapshotId,
    /// Options
    pub options: DiffOptions,
    /// Current state
    pub state: DiffState,
    /// Continuation token (for pagination)
    pub continuation: DiffContinuation,
}

/// Diff computation state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffState {
    /// Not started
    NotStarted,
    /// Comparing inodes
    ComparingInodes,
    /// Comparing extents
    ComparingExtents,
    /// Comparing directories
    ComparingDirs,
    /// Complete
    Complete,
}

impl Default for DiffState {
    fn default() -> Self {
        Self::NotStarted
    }
}

/// Continuation token for paginated diff.
#[derive(Clone, Copy, Debug, Default)]
pub struct DiffContinuation {
    /// Last processed inode
    pub last_ino: u64,
    /// Last processed block
    pub last_block: BlockNum,
    /// Page number
    pub page: u32,
}

impl DiffContext {
    /// Create new context
    pub fn new(source: SnapshotId, target: SnapshotId, options: DiffOptions) -> Self {
        Self {
            source,
            target,
            ancestor: ROOT_SNAPSHOT_ID,
            options,
            state: DiffState::NotStarted,
            continuation: DiffContinuation::default(),
        }
    }
    
    /// Check if complete
    #[inline]
    pub fn is_complete(&self) -> bool {
        self.state == DiffState::Complete
    }
}

// ============================================================================
// Incremental Diff
// ============================================================================

/// Incremental diff for streaming.
pub struct IncrementalDiff {
    /// Context
    pub context: DiffContext,
    /// Buffer for current batch
    pub buffer: [DiffEntry; 64],
    /// Buffer count
    pub buffer_count: usize,
    /// Total returned
    pub total_returned: usize,
}

impl IncrementalDiff {
    /// Create new incremental diff
    pub fn new(source: SnapshotId, target: SnapshotId) -> Self {
        Self {
            context: DiffContext::new(source, target, DiffOptions::default()),
            buffer: [DiffEntry::default(); 64],
            buffer_count: 0,
            total_returned: 0,
        }
    }
    
    /// Check if has more
    #[inline]
    pub fn has_more(&self) -> bool {
        !self.context.is_complete()
    }
    
    /// Get current batch
    pub fn current_batch(&self) -> &[DiffEntry] {
        &self.buffer[..self.buffer_count]
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_diff_type() {
        assert!(DiffType::Added.is_change());
        assert!(DiffType::Modified.is_change());
        assert!(!DiffType::None.is_change());
    }
    
    #[test]
    fn test_diff_entry() {
        let added = DiffEntry::added(100, FileType::Regular);
        assert_eq!(added.diff_type, DiffType::Added);
        
        let removed = DiffEntry::removed(100, FileType::Directory);
        assert_eq!(removed.diff_type, DiffType::Removed);
        
        let modified = DiffEntry::modified(100, 1000, 2000);
        assert_eq!(modified.size_delta(), 1000);
    }
    
    #[test]
    fn test_block_diff() {
        let added = BlockDiff::new(1, 0, 0, 100, 1);
        assert!(added.is_added());
        assert!(!added.is_removed());
        
        let removed = BlockDiff::new(1, 0, 100, 0, 1);
        assert!(!removed.is_added());
        assert!(removed.is_removed());
    }
    
    #[test]
    fn test_diff_result() {
        let mut result = DiffResult::new(1, 2);
        
        assert!(result.is_empty());
        
        result.add(DiffEntry::added(100, FileType::Regular)).unwrap();
        result.add(DiffEntry::modified(101, 100, 200)).unwrap();
        
        assert_eq!(result.count, 2);
        assert_eq!(result.stats.added, 1);
        assert_eq!(result.stats.modified, 1);
    }
    
    #[test]
    fn test_diff_options() {
        let opts = DiffOptions::default()
            .with_content()
            .with_limit(100)
            .files_only();
        
        assert!(opts.include_content);
        assert_eq!(opts.max_entries, 100);
        assert_eq!(opts.file_type_filter, Some(FileType::Regular));
    }
    
    #[test]
    fn test_diff_context() {
        let ctx = DiffContext::new(1, 2, DiffOptions::default());
        
        assert_eq!(ctx.source, 1);
        assert_eq!(ctx.target, 2);
        assert!(!ctx.is_complete());
    }
}
