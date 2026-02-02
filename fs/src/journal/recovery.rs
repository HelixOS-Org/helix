//! Crash recovery for the journal.
//!
//! Provides recovery procedures to restore filesystem
//! consistency after a crash or unclean shutdown.

use crate::core::error::{HfsError, HfsResult};
use crate::core::types::*;
use crate::journal::checkpoint::CheckpointDescriptor;
use crate::journal::record::*;
use crate::journal::txn::TxnId;
use crate::journal::wal::WalPosition;
use crate::journal::JournalSuperblock;

// ============================================================================
// Recovery State
// ============================================================================

/// Recovery state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum RecoveryState {
    /// Not started
    NotStarted   = 0,
    /// Scanning journal
    Scanning     = 1,
    /// Finding valid transactions
    FindingTxns  = 2,
    /// Building redo list
    BuildingRedo = 3,
    /// Replaying transactions
    Replaying    = 4,
    /// Finalizing
    Finalizing   = 5,
    /// Complete
    Complete     = 6,
    /// Failed
    Failed       = 7,
}

impl RecoveryState {
    /// From raw value
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::Scanning,
            2 => Self::FindingTxns,
            3 => Self::BuildingRedo,
            4 => Self::Replaying,
            5 => Self::Finalizing,
            6 => Self::Complete,
            7 => Self::Failed,
            _ => Self::NotStarted,
        }
    }

    /// Check if active
    #[inline]
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Scanning
                | Self::FindingTxns
                | Self::BuildingRedo
                | Self::Replaying
                | Self::Finalizing
        )
    }
}

impl Default for RecoveryState {
    fn default() -> Self {
        Self::NotStarted
    }
}

// ============================================================================
// Recovery Options
// ============================================================================

/// Recovery options.
#[derive(Clone, Copy, Debug)]
pub struct RecoveryOptions {
    /// Maximum transactions to recover
    pub max_txns: u32,
    /// Maximum blocks to replay
    pub max_blocks: u64,
    /// Read-only mode (don't actually apply)
    pub read_only: bool,
    /// Verify checksums
    pub verify_checksums: bool,
    /// Skip invalid records (vs fail)
    pub skip_invalid: bool,
    /// Verbose logging
    pub verbose: bool,
}

impl RecoveryOptions {
    /// Default options
    pub const fn default() -> Self {
        Self {
            max_txns: u32::MAX,
            max_blocks: u64::MAX,
            read_only: false,
            verify_checksums: true,
            skip_invalid: false,
            verbose: false,
        }
    }

    /// Read-only recovery (for analysis)
    pub const fn read_only() -> Self {
        Self {
            read_only: true,
            ..Self::default()
        }
    }
}

impl Default for RecoveryOptions {
    fn default() -> Self {
        Self::default()
    }
}

// ============================================================================
// Transaction Log Entry
// ============================================================================

/// Maximum transactions to track during recovery
const MAX_RECOVERY_TXNS: usize = 256;

/// Maximum redo entries per transaction
const MAX_REDO_ENTRIES: usize = 1024;

/// Transaction log entry during recovery.
#[derive(Clone, Copy, Debug, Default)]
pub struct TxnLogEntry {
    /// Transaction ID
    pub txn_id: TxnId,
    /// Start sequence
    pub start_seq: u64,
    /// Commit sequence (0 if not committed)
    pub commit_seq: u64,
    /// First record position
    pub first_record: WalPosition,
    /// Last record position
    pub last_record: WalPosition,
    /// Record count
    pub record_count: u32,
    /// State
    pub state: TxnLogState,
}

/// Transaction log state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum TxnLogState {
    /// Unknown
    #[default]
    Unknown    = 0,
    /// In progress (uncommitted)
    InProgress = 1,
    /// Committed
    Committed  = 2,
    /// Aborted
    Aborted    = 3,
    /// Replayed
    Replayed   = 4,
}

impl TxnLogEntry {
    /// Check if committed
    #[inline]
    pub fn is_committed(&self) -> bool {
        self.state == TxnLogState::Committed
    }

    /// Check if needs replay
    #[inline]
    pub fn needs_replay(&self) -> bool {
        self.state == TxnLogState::Committed
    }
}

// ============================================================================
// Redo Entry
// ============================================================================

/// Redo entry for recovery.
#[derive(Clone, Copy, Debug, Default)]
pub struct RedoEntry {
    /// Transaction ID
    pub txn_id: TxnId,
    /// Sequence number
    pub sequence: u64,
    /// Record type
    pub record_type: u8,
    /// Target block
    pub block: BlockNum,
    /// Data offset in journal
    pub data_offset: u64,
    /// Data length
    pub data_len: u32,
    /// Applied flag
    pub applied: bool,
}

impl RedoEntry {
    /// Create new entry
    pub fn new(
        txn_id: TxnId,
        sequence: u64,
        record_type: RecordType,
        block: BlockNum,
        data_offset: u64,
        data_len: u32,
    ) -> Self {
        Self {
            txn_id,
            sequence,
            record_type: record_type as u8,
            block,
            data_offset,
            data_len,
            applied: false,
        }
    }

    /// Get record type
    #[inline]
    pub fn record_type(&self) -> RecordType {
        RecordType::from_raw(self.record_type)
    }
}

// ============================================================================
// Revoke Table
// ============================================================================

/// Maximum revoke entries
const MAX_REVOKE_ENTRIES: usize = 4096;

/// Revoke entry.
#[derive(Clone, Copy, Debug, Default)]
pub struct RevokeEntry {
    /// Block number
    pub block: BlockNum,
    /// Revoke sequence
    pub sequence: u64,
}

/// Revoke table for recovery.
///
/// Blocks in revoke table should not be replayed from
/// earlier journal entries.
pub struct RevokeTable {
    /// Entries
    pub entries: [RevokeEntry; MAX_REVOKE_ENTRIES],
    /// Count
    pub count: usize,
}

impl RevokeTable {
    /// Create new table
    pub fn new() -> Self {
        Self {
            entries: [RevokeEntry::default(); MAX_REVOKE_ENTRIES],
            count: 0,
        }
    }

    /// Add revoke entry
    pub fn add(&mut self, block: BlockNum, sequence: u64) -> HfsResult<()> {
        // Check if already revoked with higher sequence
        for i in 0..self.count {
            if self.entries[i].block == block {
                if sequence > self.entries[i].sequence {
                    self.entries[i].sequence = sequence;
                }
                return Ok(());
            }
        }

        if self.count >= MAX_REVOKE_ENTRIES {
            return Err(HfsError::OutOfMemory);
        }

        self.entries[self.count] = RevokeEntry { block, sequence };
        self.count += 1;
        Ok(())
    }

    /// Check if block is revoked at sequence
    pub fn is_revoked(&self, block: BlockNum, sequence: u64) -> bool {
        for i in 0..self.count {
            if self.entries[i].block == block && self.entries[i].sequence > sequence {
                return true;
            }
        }
        false
    }

    /// Clear table
    pub fn clear(&mut self) {
        for i in 0..self.count {
            self.entries[i] = RevokeEntry::default();
        }
        self.count = 0;
    }
}

impl Default for RevokeTable {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Recovery Context
// ============================================================================

/// Recovery context.
pub struct RecoveryContext {
    /// Recovery state
    pub state: RecoveryState,
    /// Options
    pub options: RecoveryOptions,
    /// Journal superblock
    pub journal_sb: JournalSuperblock,
    /// Last checkpoint
    pub last_checkpoint: Option<CheckpointDescriptor>,
    /// Transaction log
    pub txn_log: [TxnLogEntry; MAX_RECOVERY_TXNS],
    /// Transaction count
    pub txn_count: usize,
    /// Redo list
    pub redo_list: [RedoEntry; MAX_REDO_ENTRIES],
    /// Redo count
    pub redo_count: usize,
    /// Revoke table
    pub revoke_table: RevokeTable,
    /// Statistics
    pub stats: RecoveryStats,
    /// Last error
    pub last_error: Option<HfsError>,
}

impl RecoveryContext {
    /// Create new context
    pub fn new(journal_sb: JournalSuperblock, options: RecoveryOptions) -> Self {
        Self {
            state: RecoveryState::NotStarted,
            options,
            journal_sb,
            last_checkpoint: None,
            txn_log: [TxnLogEntry::default(); MAX_RECOVERY_TXNS],
            txn_count: 0,
            redo_list: [RedoEntry::default(); MAX_REDO_ENTRIES],
            redo_count: 0,
            revoke_table: RevokeTable::new(),
            stats: RecoveryStats::new(),
            last_error: None,
        }
    }

    /// Set state
    pub fn set_state(&mut self, state: RecoveryState) {
        self.state = state;
    }

    /// Find transaction by ID
    pub fn find_txn(&self, txn_id: TxnId) -> Option<&TxnLogEntry> {
        for i in 0..self.txn_count {
            if self.txn_log[i].txn_id == txn_id {
                return Some(&self.txn_log[i]);
            }
        }
        None
    }

    /// Find transaction by ID (mutable)
    pub fn find_txn_mut(&mut self, txn_id: TxnId) -> Option<&mut TxnLogEntry> {
        for i in 0..self.txn_count {
            if self.txn_log[i].txn_id == txn_id {
                return Some(&mut self.txn_log[i]);
            }
        }
        None
    }

    /// Add transaction
    pub fn add_txn(&mut self, entry: TxnLogEntry) -> HfsResult<()> {
        if self.txn_count >= MAX_RECOVERY_TXNS {
            return Err(HfsError::OutOfMemory);
        }

        self.txn_log[self.txn_count] = entry;
        self.txn_count += 1;
        Ok(())
    }

    /// Add redo entry
    pub fn add_redo(&mut self, entry: RedoEntry) -> HfsResult<()> {
        if self.redo_count >= MAX_REDO_ENTRIES {
            return Err(HfsError::OutOfMemory);
        }

        self.redo_list[self.redo_count] = entry;
        self.redo_count += 1;
        Ok(())
    }

    /// Count committed transactions
    pub fn committed_count(&self) -> usize {
        let mut count = 0;
        for i in 0..self.txn_count {
            if self.txn_log[i].is_committed() {
                count += 1;
            }
        }
        count
    }

    /// Set error and fail
    pub fn fail(&mut self, error: HfsError) {
        self.last_error = Some(error);
        self.state = RecoveryState::Failed;
    }
}

// ============================================================================
// Recovery Statistics
// ============================================================================

/// Recovery statistics.
#[derive(Clone, Copy, Debug, Default)]
pub struct RecoveryStats {
    /// Journal blocks scanned
    pub blocks_scanned: u64,
    /// Valid records found
    pub valid_records: u64,
    /// Invalid records found
    pub invalid_records: u64,
    /// Transactions found
    pub txns_found: u64,
    /// Committed transactions
    pub txns_committed: u64,
    /// Transactions replayed
    pub txns_replayed: u64,
    /// Blocks replayed
    pub blocks_replayed: u64,
    /// Revoke records processed
    pub revokes_processed: u64,
    /// Recovery duration (ns)
    pub duration_ns: u64,
}

impl RecoveryStats {
    /// Create new stats
    pub const fn new() -> Self {
        Self {
            blocks_scanned: 0,
            valid_records: 0,
            invalid_records: 0,
            txns_found: 0,
            txns_committed: 0,
            txns_replayed: 0,
            blocks_replayed: 0,
            revokes_processed: 0,
            duration_ns: 0,
        }
    }
}

// ============================================================================
// Recovery Result
// ============================================================================

/// Recovery result.
#[derive(Clone, Copy, Debug)]
pub struct RecoveryResult {
    /// Success
    pub success: bool,
    /// Final state
    pub state: RecoveryState,
    /// Error (if failed)
    pub error: Option<HfsError>,
    /// Statistics
    pub stats: RecoveryStats,
    /// New journal head position
    pub new_head: WalPosition,
    /// New journal tail position
    pub new_tail: WalPosition,
}

impl RecoveryResult {
    /// Create success result
    pub fn success(stats: RecoveryStats, new_head: WalPosition, new_tail: WalPosition) -> Self {
        Self {
            success: true,
            state: RecoveryState::Complete,
            error: None,
            stats,
            new_head,
            new_tail,
        }
    }

    /// Create failure result
    pub fn failure(error: HfsError, state: RecoveryState, stats: RecoveryStats) -> Self {
        Self {
            success: false,
            state,
            error: Some(error),
            stats,
            new_head: WalPosition::invalid(),
            new_tail: WalPosition::invalid(),
        }
    }

    /// No recovery needed
    pub fn not_needed() -> Self {
        Self {
            success: true,
            state: RecoveryState::Complete,
            error: None,
            stats: RecoveryStats::new(),
            new_head: WalPosition::invalid(),
            new_tail: WalPosition::invalid(),
        }
    }
}

// ============================================================================
// Recovery Operations Interface
// ============================================================================

/// Recovery operations interface.
pub trait RecoveryOps {
    /// Scan journal for valid records.
    fn scan_journal(&mut self, ctx: &mut RecoveryContext) -> HfsResult<()>;

    /// Find committed transactions.
    fn find_committed(&mut self, ctx: &mut RecoveryContext) -> HfsResult<()>;

    /// Build redo list from committed transactions.
    fn build_redo_list(&mut self, ctx: &mut RecoveryContext) -> HfsResult<()>;

    /// Replay redo list.
    fn replay_redo(&mut self, ctx: &mut RecoveryContext) -> HfsResult<()>;

    /// Finalize recovery.
    fn finalize(&mut self, ctx: &mut RecoveryContext) -> HfsResult<()>;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::JournalMode;

    #[test]
    fn test_recovery_state() {
        assert!(!RecoveryState::NotStarted.is_active());
        assert!(RecoveryState::Scanning.is_active());
        assert!(RecoveryState::Replaying.is_active());
        assert!(!RecoveryState::Complete.is_active());
    }

    #[test]
    fn test_recovery_options() {
        let opts = RecoveryOptions::default();
        assert!(!opts.read_only);
        assert!(opts.verify_checksums);

        let ro = RecoveryOptions::read_only();
        assert!(ro.read_only);
    }

    #[test]
    fn test_txn_log_entry() {
        let mut entry = TxnLogEntry::default();
        entry.txn_id = 1;
        entry.state = TxnLogState::Committed;

        assert!(entry.is_committed());
        assert!(entry.needs_replay());
    }

    #[test]
    fn test_redo_entry() {
        let entry = RedoEntry::new(1, 100, RecordType::BlockWrite, 500, 1000, 4096);

        assert_eq!(entry.txn_id, 1);
        assert_eq!(entry.record_type(), RecordType::BlockWrite);
        assert!(!entry.applied);
    }

    #[test]
    fn test_revoke_table() {
        let mut table = RevokeTable::new();

        table.add(100, 50).unwrap();

        // Earlier sequences should be revoked
        assert!(table.is_revoked(100, 40));
        // Later sequences should not be revoked
        assert!(!table.is_revoked(100, 60));
        // Other blocks not affected
        assert!(!table.is_revoked(200, 40));
    }

    #[test]
    fn test_recovery_context() {
        let sb = JournalSuperblock::new(1000, JournalMode::Metadata);
        let mut ctx = RecoveryContext::new(sb, RecoveryOptions::default());

        assert_eq!(ctx.state, RecoveryState::NotStarted);
        assert_eq!(ctx.txn_count, 0);

        // Add transaction
        let entry = TxnLogEntry {
            txn_id: 1,
            state: TxnLogState::Committed,
            ..TxnLogEntry::default()
        };
        ctx.add_txn(entry).unwrap();

        assert_eq!(ctx.txn_count, 1);
        assert_eq!(ctx.committed_count(), 1);

        let found = ctx.find_txn(1);
        assert!(found.is_some());
        assert_eq!(found.unwrap().txn_id, 1);
    }

    #[test]
    fn test_recovery_result() {
        let success = RecoveryResult::success(
            RecoveryStats::new(),
            WalPosition::start(),
            WalPosition::start(),
        );
        assert!(success.success);

        let failure = RecoveryResult::failure(
            HfsError::JournalCorrupted,
            RecoveryState::Scanning,
            RecoveryStats::new(),
        );
        assert!(!failure.success);
        assert!(failure.error.is_some());
    }
}
