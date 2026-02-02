//! Snapshot cloning and rollback operations.
//!
//! Provides writable clone creation from snapshots
//! and filesystem rollback functionality.

use crate::core::error::HfsError;
use crate::snapshot::{SnapshotId, INVALID_SNAPSHOT_ID};

// ============================================================================
// Clone Types
// ============================================================================

/// Clone type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CloneType {
    /// Full clone (complete copy)
    Full   = 1,
    /// Linked clone (CoW sharing)
    Linked = 2,
    /// Sparse clone (on-demand copy)
    Sparse = 3,
}

impl CloneType {
    /// From raw value
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::Full,
            2 => Self::Linked,
            _ => Self::Sparse,
        }
    }

    /// Does this type share data
    #[inline]
    pub fn shares_data(&self) -> bool {
        matches!(self, Self::Linked | Self::Sparse)
    }
}

impl Default for CloneType {
    fn default() -> Self {
        Self::Linked
    }
}

// ============================================================================
// Clone Options
// ============================================================================

/// Options for clone operation.
#[derive(Clone, Copy, Debug)]
pub struct CloneOptions {
    /// Clone type
    pub clone_type: CloneType,
    /// Make clone the active snapshot
    pub make_active: bool,
    /// Preserve source permissions
    pub preserve_permissions: bool,
    /// Include extended attributes
    pub include_xattrs: bool,
    /// Limit to subtree (inode)
    pub subtree_root: Option<u64>,
}

impl CloneOptions {
    /// Default options (linked clone)
    pub const fn default() -> Self {
        Self {
            clone_type: CloneType::Linked,
            make_active: false,
            preserve_permissions: true,
            include_xattrs: true,
            subtree_root: None,
        }
    }

    /// Create linked clone options
    pub const fn linked() -> Self {
        Self {
            clone_type: CloneType::Linked,
            ..Self::default()
        }
    }

    /// Create sparse clone options
    pub const fn sparse() -> Self {
        Self {
            clone_type: CloneType::Sparse,
            ..Self::default()
        }
    }

    /// Clone subtree only
    pub fn subtree(mut self, root_ino: u64) -> Self {
        self.subtree_root = Some(root_ino);
        self
    }
}

impl Default for CloneOptions {
    fn default() -> Self {
        Self::default()
    }
}

// ============================================================================
// Clone State
// ============================================================================

/// Clone operation state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CloneState {
    /// Not started
    NotStarted      = 0,
    /// Preparing (allocating resources)
    Preparing       = 1,
    /// Copying metadata
    CopyingMetadata = 2,
    /// Updating references
    UpdatingRefs    = 3,
    /// Finalizing
    Finalizing      = 4,
    /// Complete
    Complete        = 5,
    /// Failed
    Failed          = 6,
}

impl CloneState {
    /// From raw value
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::Preparing,
            2 => Self::CopyingMetadata,
            3 => Self::UpdatingRefs,
            4 => Self::Finalizing,
            5 => Self::Complete,
            6 => Self::Failed,
            _ => Self::NotStarted,
        }
    }

    /// Check if active
    #[inline]
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Preparing | Self::CopyingMetadata | Self::UpdatingRefs | Self::Finalizing
        )
    }
}

impl Default for CloneState {
    fn default() -> Self {
        Self::NotStarted
    }
}

// ============================================================================
// Clone Context
// ============================================================================

/// Context for clone operation.
pub struct CloneContext {
    /// Source snapshot
    pub source: SnapshotId,
    /// New clone ID
    pub clone_id: SnapshotId,
    /// Options
    pub options: CloneOptions,
    /// State
    pub state: CloneState,
    /// Progress
    pub progress: CloneProgress,
    /// Error (if failed)
    pub error: Option<HfsError>,
}

impl CloneContext {
    /// Create new context
    pub fn new(source: SnapshotId, clone_id: SnapshotId, options: CloneOptions) -> Self {
        Self {
            source,
            clone_id,
            options,
            state: CloneState::NotStarted,
            progress: CloneProgress::new(),
            error: None,
        }
    }

    /// Check if complete
    #[inline]
    pub fn is_complete(&self) -> bool {
        self.state == CloneState::Complete
    }

    /// Check if failed
    #[inline]
    pub fn is_failed(&self) -> bool {
        self.state == CloneState::Failed
    }

    /// Set state
    pub fn set_state(&mut self, state: CloneState) {
        self.state = state;
    }

    /// Fail with error
    pub fn fail(&mut self, error: HfsError) {
        self.error = Some(error);
        self.state = CloneState::Failed;
    }
}

/// Clone progress.
#[derive(Clone, Copy, Debug, Default)]
pub struct CloneProgress {
    /// Inodes processed
    pub inodes_processed: u64,
    /// Total inodes
    pub total_inodes: u64,
    /// Blocks processed
    pub blocks_processed: u64,
    /// Total blocks
    pub total_blocks: u64,
    /// Bytes processed
    pub bytes_processed: u64,
    /// Total bytes
    pub total_bytes: u64,
}

impl CloneProgress {
    /// Create new progress
    pub const fn new() -> Self {
        Self {
            inodes_processed: 0,
            total_inodes: 0,
            blocks_processed: 0,
            total_blocks: 0,
            bytes_processed: 0,
            total_bytes: 0,
        }
    }

    /// Get percentage complete
    pub fn percentage(&self) -> f32 {
        if self.total_inodes == 0 {
            return 0.0;
        }
        (self.inodes_processed as f32 / self.total_inodes as f32) * 100.0
    }
}

// ============================================================================
// Clone Result
// ============================================================================

/// Result of clone operation.
#[derive(Clone, Copy, Debug)]
pub struct CloneResult {
    /// Success
    pub success: bool,
    /// Clone snapshot ID
    pub clone_id: SnapshotId,
    /// Clone type used
    pub clone_type: CloneType,
    /// Error (if failed)
    pub error: Option<HfsError>,
    /// Statistics
    pub stats: CloneStats,
}

impl CloneResult {
    /// Create success result
    pub fn success(clone_id: SnapshotId, clone_type: CloneType, stats: CloneStats) -> Self {
        Self {
            success: true,
            clone_id,
            clone_type,
            error: None,
            stats,
        }
    }

    /// Create failure result
    pub fn failure(error: HfsError) -> Self {
        Self {
            success: false,
            clone_id: INVALID_SNAPSHOT_ID,
            clone_type: CloneType::Linked,
            error: Some(error),
            stats: CloneStats::new(),
        }
    }
}

/// Clone statistics.
#[derive(Clone, Copy, Debug, Default)]
pub struct CloneStats {
    /// Inodes cloned
    pub inodes_cloned: u64,
    /// Blocks shared
    pub blocks_shared: u64,
    /// Blocks copied
    pub blocks_copied: u64,
    /// Bytes saved by sharing
    pub bytes_saved: u64,
    /// Duration (ns)
    pub duration_ns: u64,
}

impl CloneStats {
    /// Create new stats
    pub const fn new() -> Self {
        Self {
            inodes_cloned: 0,
            blocks_shared: 0,
            blocks_copied: 0,
            bytes_saved: 0,
            duration_ns: 0,
        }
    }

    /// Sharing ratio
    pub fn sharing_ratio(&self) -> f32 {
        let total = self.blocks_shared + self.blocks_copied;
        if total == 0 {
            return 0.0;
        }
        self.blocks_shared as f32 / total as f32
    }
}

// ============================================================================
// Rollback Types
// ============================================================================

/// Rollback mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum RollbackMode {
    /// Full rollback (discard all changes)
    Full      = 1,
    /// Selective rollback (keep some changes)
    Selective = 2,
    /// Preview (show what would change)
    Preview   = 3,
}

impl RollbackMode {
    /// From raw value
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::Full,
            2 => Self::Selective,
            _ => Self::Preview,
        }
    }
}

impl Default for RollbackMode {
    fn default() -> Self {
        Self::Full
    }
}

// ============================================================================
// Rollback Options
// ============================================================================

/// Options for rollback operation.
#[derive(Clone, Copy, Debug)]
pub struct RollbackOptions {
    /// Rollback mode
    pub mode: RollbackMode,
    /// Create backup snapshot before rollback
    pub create_backup: bool,
    /// Preserve newer files
    pub preserve_newer: bool,
    /// Subtree only
    pub subtree_root: Option<u64>,
    /// Max files to affect (0 = unlimited)
    pub max_files: u64,
}

impl RollbackOptions {
    /// Default options
    pub const fn default() -> Self {
        Self {
            mode: RollbackMode::Full,
            create_backup: true,
            preserve_newer: false,
            subtree_root: None,
            max_files: 0,
        }
    }

    /// Preview mode
    pub const fn preview() -> Self {
        Self {
            mode: RollbackMode::Preview,
            ..Self::default()
        }
    }

    /// Selective mode
    pub const fn selective() -> Self {
        Self {
            mode: RollbackMode::Selective,
            ..Self::default()
        }
    }
}

impl Default for RollbackOptions {
    fn default() -> Self {
        Self::default()
    }
}

// ============================================================================
// Rollback State
// ============================================================================

/// Rollback operation state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum RollbackState {
    /// Not started
    NotStarted     = 0,
    /// Validating
    Validating     = 1,
    /// Creating backup
    CreatingBackup = 2,
    /// Reverting files
    RevertingFiles = 3,
    /// Reverting directories
    RevertingDirs  = 4,
    /// Updating metadata
    UpdatingMeta   = 5,
    /// Finalizing
    Finalizing     = 6,
    /// Complete
    Complete       = 7,
    /// Failed
    Failed         = 8,
}

impl RollbackState {
    /// From raw value
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::Validating,
            2 => Self::CreatingBackup,
            3 => Self::RevertingFiles,
            4 => Self::RevertingDirs,
            5 => Self::UpdatingMeta,
            6 => Self::Finalizing,
            7 => Self::Complete,
            8 => Self::Failed,
            _ => Self::NotStarted,
        }
    }

    /// Check if active
    #[inline]
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Validating
                | Self::CreatingBackup
                | Self::RevertingFiles
                | Self::RevertingDirs
                | Self::UpdatingMeta
                | Self::Finalizing
        )
    }
}

impl Default for RollbackState {
    fn default() -> Self {
        Self::NotStarted
    }
}

// ============================================================================
// Rollback Context
// ============================================================================

/// Context for rollback operation.
pub struct RollbackContext {
    /// Target snapshot to roll back to
    pub target: SnapshotId,
    /// Current snapshot
    pub current: SnapshotId,
    /// Options
    pub options: RollbackOptions,
    /// State
    pub state: RollbackState,
    /// Backup snapshot ID (if created)
    pub backup_id: Option<SnapshotId>,
    /// Progress
    pub progress: RollbackProgress,
    /// Error
    pub error: Option<HfsError>,
}

impl RollbackContext {
    /// Create new context
    pub fn new(target: SnapshotId, current: SnapshotId, options: RollbackOptions) -> Self {
        Self {
            target,
            current,
            options,
            state: RollbackState::NotStarted,
            backup_id: None,
            progress: RollbackProgress::new(),
            error: None,
        }
    }

    /// Check if complete
    #[inline]
    pub fn is_complete(&self) -> bool {
        self.state == RollbackState::Complete
    }

    /// Check if failed
    #[inline]
    pub fn is_failed(&self) -> bool {
        self.state == RollbackState::Failed
    }
}

/// Rollback progress.
#[derive(Clone, Copy, Debug, Default)]
pub struct RollbackProgress {
    /// Files reverted
    pub files_reverted: u64,
    /// Directories reverted
    pub dirs_reverted: u64,
    /// Files skipped
    pub files_skipped: u64,
    /// Bytes changed
    pub bytes_changed: u64,
}

impl RollbackProgress {
    /// Create new progress
    pub const fn new() -> Self {
        Self {
            files_reverted: 0,
            dirs_reverted: 0,
            files_skipped: 0,
            bytes_changed: 0,
        }
    }

    /// Total items processed
    #[inline]
    pub fn total_processed(&self) -> u64 {
        self.files_reverted + self.dirs_reverted + self.files_skipped
    }
}

// ============================================================================
// Rollback Result
// ============================================================================

/// Result of rollback operation.
#[derive(Clone, Copy, Debug)]
pub struct RollbackResult {
    /// Success
    pub success: bool,
    /// Backup snapshot ID (if created)
    pub backup_id: Option<SnapshotId>,
    /// Error
    pub error: Option<HfsError>,
    /// Statistics
    pub stats: RollbackStats,
}

impl RollbackResult {
    /// Create success result
    pub fn success(backup_id: Option<SnapshotId>, stats: RollbackStats) -> Self {
        Self {
            success: true,
            backup_id,
            error: None,
            stats,
        }
    }

    /// Create failure result
    pub fn failure(error: HfsError) -> Self {
        Self {
            success: false,
            backup_id: None,
            error: Some(error),
            stats: RollbackStats::new(),
        }
    }
}

/// Rollback statistics.
#[derive(Clone, Copy, Debug, Default)]
pub struct RollbackStats {
    /// Files restored
    pub files_restored: u64,
    /// Files removed
    pub files_removed: u64,
    /// Directories restored
    pub dirs_restored: u64,
    /// Blocks restored
    pub blocks_restored: u64,
    /// Duration (ns)
    pub duration_ns: u64,
}

impl RollbackStats {
    /// Create new stats
    pub const fn new() -> Self {
        Self {
            files_restored: 0,
            files_removed: 0,
            dirs_restored: 0,
            blocks_restored: 0,
            duration_ns: 0,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clone_type() {
        assert!(CloneType::Linked.shares_data());
        assert!(CloneType::Sparse.shares_data());
        assert!(!CloneType::Full.shares_data());
    }

    #[test]
    fn test_clone_options() {
        let opts = CloneOptions::linked();
        assert_eq!(opts.clone_type, CloneType::Linked);

        let subtree = CloneOptions::default().subtree(100);
        assert_eq!(subtree.subtree_root, Some(100));
    }

    #[test]
    fn test_clone_context() {
        let ctx = CloneContext::new(1, 2, CloneOptions::default());

        assert_eq!(ctx.source, 1);
        assert_eq!(ctx.clone_id, 2);
        assert!(!ctx.is_complete());
        assert!(!ctx.is_failed());
    }

    #[test]
    fn test_clone_progress() {
        let mut progress = CloneProgress::new();
        progress.total_inodes = 100;
        progress.inodes_processed = 50;

        assert_eq!(progress.percentage(), 50.0);
    }

    #[test]
    fn test_clone_stats() {
        let mut stats = CloneStats::new();
        stats.blocks_shared = 80;
        stats.blocks_copied = 20;

        assert_eq!(stats.sharing_ratio(), 0.8);
    }

    #[test]
    fn test_rollback_options() {
        let preview = RollbackOptions::preview();
        assert_eq!(preview.mode, RollbackMode::Preview);

        let default = RollbackOptions::default();
        assert!(default.create_backup);
    }

    #[test]
    fn test_rollback_context() {
        let ctx = RollbackContext::new(1, 2, RollbackOptions::default());

        assert_eq!(ctx.target, 1);
        assert_eq!(ctx.current, 2);
        assert!(!ctx.is_complete());
    }
}
