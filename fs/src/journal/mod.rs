//! Write-Ahead Logging (WAL) for crash-consistent operations.
//!
//! This module provides:
//! - Write-ahead logging for atomic operations
//! - Transaction management and commit protocols
//! - Crash recovery and replay
//! - Checkpoint management

pub mod checkpoint;
pub mod record;
pub mod recovery;
pub mod txn;
pub mod wal;

pub use checkpoint::*;
pub use record::*;
pub use recovery::*;
pub use txn::*;
pub use wal::*;

use crate::core::error::{HfsError, HfsResult};
use crate::core::hash::Crc32c;

// ============================================================================
// Journal Configuration
// ============================================================================

/// Journal magic number
pub const JOURNAL_MAGIC: u32 = 0x484A524E; // "HJRN"

/// Journal version
pub const JOURNAL_VERSION: u8 = 1;

/// Minimum journal size (16 MB)
pub const MIN_JOURNAL_SIZE: u64 = 16 * 1024 * 1024;

/// Maximum journal size (4 GB)
pub const MAX_JOURNAL_SIZE: u64 = 4 * 1024 * 1024 * 1024;

/// Default journal size (256 MB)
pub const DEFAULT_JOURNAL_SIZE: u64 = 256 * 1024 * 1024;

/// Journal block size
pub const JOURNAL_BLOCK_SIZE: usize = 4096;

/// Maximum record size
pub const MAX_RECORD_SIZE: usize = 1024 * 1024; // 1 MB

/// Checkpoint interval (records)
pub const CHECKPOINT_INTERVAL: u64 = 1000;

/// Checkpoint interval (time) - 30 seconds in nanoseconds
pub const CHECKPOINT_TIME_NS: u64 = 30_000_000_000;

// ============================================================================
// Journal State
// ============================================================================

/// Journal state
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum JournalState {
    /// Not initialized
    Uninitialized = 0,
    /// Clean (no uncommitted transactions)
    Clean         = 1,
    /// Active (has uncommitted transactions)
    Active        = 2,
    /// Recovery needed
    NeedsRecovery = 3,
    /// Recovery in progress
    Recovering    = 4,
    /// Corrupted
    Corrupted     = 5,
    /// Disabled
    Disabled      = 6,
}

impl JournalState {
    /// From raw value
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::Clean,
            2 => Self::Active,
            3 => Self::NeedsRecovery,
            4 => Self::Recovering,
            5 => Self::Corrupted,
            6 => Self::Disabled,
            _ => Self::Uninitialized,
        }
    }

    /// Check if journal needs recovery
    #[inline]
    pub fn needs_recovery(&self) -> bool {
        matches!(self, Self::NeedsRecovery | Self::Active)
    }

    /// Check if journal is usable
    #[inline]
    pub fn is_usable(&self) -> bool {
        matches!(self, Self::Clean | Self::Active)
    }
}

impl Default for JournalState {
    fn default() -> Self {
        Self::Uninitialized
    }
}

// ============================================================================
// Journal Mode
// ============================================================================

/// Journal mode (consistency level)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum JournalMode {
    /// Metadata only (default)
    Metadata  = 0,
    /// Full data journaling
    Full      = 1,
    /// Ordered writes (data before metadata)
    Ordered   = 2,
    /// Writeback (maximum performance, less safety)
    Writeback = 3,
}

impl JournalMode {
    /// From raw value
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            0 => Self::Metadata,
            1 => Self::Full,
            2 => Self::Ordered,
            3 => Self::Writeback,
            _ => Self::Metadata,
        }
    }

    /// Does this mode journal data?
    #[inline]
    pub fn journals_data(&self) -> bool {
        matches!(self, Self::Full)
    }

    /// Does this mode order data?
    #[inline]
    pub fn orders_data(&self) -> bool {
        matches!(self, Self::Ordered | Self::Full)
    }
}

impl Default for JournalMode {
    fn default() -> Self {
        Self::Metadata
    }
}

// ============================================================================
// Journal Superblock
// ============================================================================

/// Journal superblock (first block of journal area).
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct JournalSuperblock {
    /// Magic number
    pub magic: u32,
    /// Version
    pub version: u8,
    /// Journal mode
    pub mode: u8,
    /// State
    pub state: u8,
    /// Flags
    pub flags: u8,
    /// Journal size in blocks
    pub size_blocks: u64,
    /// First data block
    pub first_block: u64,
    /// Last block (wraps)
    pub last_block: u64,
    /// Head sequence number
    pub head_seq: u64,
    /// Tail sequence number
    pub tail_seq: u64,
    /// First uncommitted transaction
    pub first_uncommitted: u64,
    /// Last checkpoint sequence
    pub last_checkpoint: u64,
    /// Last checkpoint block
    pub checkpoint_block: u64,
    /// Checkpoint generation
    pub checkpoint_gen: u32,
    /// Error code (if corrupted)
    pub error_code: u32,
    /// Mount count
    pub mount_count: u32,
    /// Max transaction size (blocks)
    pub max_txn_blocks: u32,
    /// Reserved
    pub _reserved: [u8; 420],
    /// Checksum
    pub checksum: u32,
}

impl JournalSuperblock {
    /// Size in bytes
    pub const SIZE: usize = 512;

    /// Create new journal superblock
    pub fn new(size_blocks: u64, mode: JournalMode) -> Self {
        Self {
            magic: JOURNAL_MAGIC,
            version: JOURNAL_VERSION,
            mode: mode as u8,
            state: JournalState::Clean as u8,
            flags: 0,
            size_blocks,
            first_block: 1,
            last_block: size_blocks - 1,
            head_seq: 1,
            tail_seq: 1,
            first_uncommitted: 0,
            last_checkpoint: 0,
            checkpoint_block: 0,
            checkpoint_gen: 0,
            error_code: 0,
            mount_count: 0,
            max_txn_blocks: 1024,
            _reserved: [0; 420],
            checksum: 0,
        }
    }

    /// Validate superblock
    pub fn validate(&self) -> HfsResult<()> {
        if self.magic != JOURNAL_MAGIC {
            return Err(HfsError::JournalCorrupted);
        }

        if self.version != JOURNAL_VERSION {
            return Err(HfsError::InvalidVersion);
        }

        if self.size_blocks < (MIN_JOURNAL_SIZE / JOURNAL_BLOCK_SIZE as u64) {
            return Err(HfsError::InvalidJournalSize);
        }

        // Verify checksum
        let computed = self.compute_checksum();
        if self.checksum != 0 && self.checksum != computed {
            return Err(HfsError::ChecksumMismatch);
        }

        Ok(())
    }

    /// Compute checksum
    pub fn compute_checksum(&self) -> u32 {
        let mut hasher = Crc32c::new();

        // Hash all fields except checksum
        let bytes = unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u8,
                core::mem::size_of::<Self>() - 4,
            )
        };
        hasher.write(bytes);

        hasher.finish()
    }

    /// Update checksum
    pub fn update_checksum(&mut self) {
        self.checksum = self.compute_checksum();
    }

    /// Get journal state
    #[inline]
    pub fn state(&self) -> JournalState {
        JournalState::from_raw(self.state)
    }

    /// Get journal mode
    #[inline]
    pub fn mode(&self) -> JournalMode {
        JournalMode::from_raw(self.mode)
    }

    /// Calculate used space
    pub fn used_blocks(&self) -> u64 {
        if self.head_seq >= self.tail_seq {
            self.head_seq - self.tail_seq
        } else {
            // Wrapped
            self.size_blocks - (self.tail_seq - self.head_seq)
        }
    }

    /// Calculate free space
    pub fn free_blocks(&self) -> u64 {
        self.size_blocks.saturating_sub(self.used_blocks() + 1)
    }

    /// Check if journal has space for transaction
    pub fn has_space(&self, blocks: u64) -> bool {
        self.free_blocks() >= blocks
    }
}

// Verify size
const _: () = assert!(core::mem::size_of::<JournalSuperblock>() == 512);

// ============================================================================
// Journal Statistics
// ============================================================================

/// Journal statistics.
#[derive(Clone, Copy, Debug, Default)]
pub struct JournalStats {
    /// Total transactions committed
    pub txn_committed: u64,
    /// Total transactions aborted
    pub txn_aborted: u64,
    /// Total records written
    pub records_written: u64,
    /// Total bytes logged
    pub bytes_logged: u64,
    /// Total checkpoints
    pub checkpoints: u64,
    /// Total recoveries
    pub recoveries: u64,
    /// Journal wraps
    pub wraps: u64,
    /// Current transaction count
    pub active_txn: u32,
    /// Peak transaction count
    pub peak_txn: u32,
}

impl JournalStats {
    /// Create new stats
    pub const fn new() -> Self {
        Self {
            txn_committed: 0,
            txn_aborted: 0,
            records_written: 0,
            bytes_logged: 0,
            checkpoints: 0,
            recoveries: 0,
            wraps: 0,
            active_txn: 0,
            peak_txn: 0,
        }
    }

    /// Record transaction commit
    pub fn record_commit(&mut self, bytes: u64, records: u64) {
        self.txn_committed += 1;
        self.bytes_logged += bytes;
        self.records_written += records;
        self.active_txn = self.active_txn.saturating_sub(1);
    }

    /// Record transaction abort
    pub fn record_abort(&mut self) {
        self.txn_aborted += 1;
        self.active_txn = self.active_txn.saturating_sub(1);
    }

    /// Record new transaction
    pub fn record_begin(&mut self) {
        self.active_txn += 1;
        if self.active_txn > self.peak_txn {
            self.peak_txn = self.active_txn;
        }
    }

    /// Record checkpoint
    pub fn record_checkpoint(&mut self) {
        self.checkpoints += 1;
    }

    /// Commit rate
    pub fn commit_rate(&self) -> f32 {
        let total = self.txn_committed + self.txn_aborted;
        if total == 0 {
            return 100.0;
        }
        (self.txn_committed as f32 / total as f32) * 100.0
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_journal_state() {
        assert!(JournalState::NeedsRecovery.needs_recovery());
        assert!(JournalState::Active.needs_recovery());
        assert!(!JournalState::Clean.needs_recovery());

        assert!(JournalState::Clean.is_usable());
        assert!(!JournalState::Corrupted.is_usable());
    }

    #[test]
    fn test_journal_mode() {
        assert!(JournalMode::Full.journals_data());
        assert!(!JournalMode::Metadata.journals_data());

        assert!(JournalMode::Ordered.orders_data());
        assert!(JournalMode::Full.orders_data());
    }

    #[test]
    fn test_journal_superblock() {
        let size = MIN_JOURNAL_SIZE / JOURNAL_BLOCK_SIZE as u64;
        let sb = JournalSuperblock::new(size, JournalMode::Metadata);

        assert_eq!(sb.magic, JOURNAL_MAGIC);
        assert_eq!(sb.version, JOURNAL_VERSION);
        assert!(sb.validate().is_ok());
    }

    #[test]
    fn test_journal_stats() {
        let mut stats = JournalStats::new();

        stats.record_begin();
        assert_eq!(stats.active_txn, 1);

        stats.record_commit(1000, 10);
        assert_eq!(stats.txn_committed, 1);
        assert_eq!(stats.bytes_logged, 1000);
        assert_eq!(stats.active_txn, 0);
    }
}
