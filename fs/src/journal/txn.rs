//! Transaction management for journal operations.
//!
//! Provides atomic transaction semantics with commit/abort
//! and nested transaction support.

use core::sync::atomic::{AtomicU64, AtomicU8, Ordering};

use crate::core::error::{HfsError, HfsResult};
use crate::core::types::*;
use crate::journal::wal::WalPosition;

// ============================================================================
// Transaction ID
// ============================================================================

/// Transaction ID type.
pub type TxnId = u64;

/// Invalid transaction ID
pub const INVALID_TXN_ID: TxnId = 0;

// ============================================================================
// Transaction State
// ============================================================================

/// Transaction state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TxnState {
    /// Not started
    None       = 0,
    /// Active (in progress)
    Active     = 1,
    /// Preparing (writing commit record)
    Preparing  = 2,
    /// Committing (flushing)
    Committing = 3,
    /// Committed successfully
    Committed  = 4,
    /// Aborting
    Aborting   = 5,
    /// Aborted
    Aborted    = 6,
    /// Error state
    Error      = 7,
}

impl TxnState {
    /// From raw value
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::Active,
            2 => Self::Preparing,
            3 => Self::Committing,
            4 => Self::Committed,
            5 => Self::Aborting,
            6 => Self::Aborted,
            7 => Self::Error,
            _ => Self::None,
        }
    }

    /// Check if transaction is active
    #[inline]
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active | Self::Preparing | Self::Committing)
    }

    /// Check if transaction is complete
    #[inline]
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Committed | Self::Aborted | Self::Error)
    }

    /// Check if transaction can be committed
    #[inline]
    pub fn can_commit(&self) -> bool {
        *self == Self::Active
    }

    /// Check if transaction can be aborted
    #[inline]
    pub fn can_abort(&self) -> bool {
        matches!(self, Self::Active | Self::Preparing)
    }
}

impl Default for TxnState {
    fn default() -> Self {
        Self::None
    }
}

// ============================================================================
// Transaction Flags
// ============================================================================

/// Transaction flags.
#[derive(Clone, Copy, Debug, Default)]
pub struct TxnFlags(pub u32);

impl TxnFlags {
    /// No flags
    pub const NONE: u32 = 0;
    /// Read-only transaction
    pub const READ_ONLY: u32 = 1 << 0;
    /// Synchronous commit
    pub const SYNC: u32 = 1 << 1;
    /// No wait for space
    pub const NO_WAIT: u32 = 1 << 2;
    /// High priority
    pub const HIGH_PRIORITY: u32 = 1 << 3;
    /// Nested transaction
    pub const NESTED: u32 = 1 << 4;
    /// System transaction (internal use)
    pub const SYSTEM: u32 = 1 << 5;
    /// Checkpoint transaction
    pub const CHECKPOINT: u32 = 1 << 6;

    /// Create from raw
    pub const fn new(flags: u32) -> Self {
        Self(flags)
    }

    /// Check if flag is set
    #[inline]
    pub fn has(&self, flag: u32) -> bool {
        (self.0 & flag) != 0
    }

    /// Set flag
    #[inline]
    pub fn set(&mut self, flag: u32) {
        self.0 |= flag;
    }

    /// Clear flag
    #[inline]
    pub fn clear(&mut self, flag: u32) {
        self.0 &= !flag;
    }

    /// Is read-only
    #[inline]
    pub fn is_read_only(&self) -> bool {
        self.has(Self::READ_ONLY)
    }

    /// Is synchronous
    #[inline]
    pub fn is_sync(&self) -> bool {
        self.has(Self::SYNC)
    }
}

// ============================================================================
// Transaction Handle
// ============================================================================

/// Transaction handle (on-disk representation).
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct TxnHandle {
    /// Transaction ID
    pub id: TxnId,
    /// Parent transaction ID (for nested)
    pub parent_id: TxnId,
    /// State
    pub state: u8,
    /// Flags
    pub flags: u8,
    /// Reserved
    pub _reserved: [u8; 6],
    /// Start timestamp
    pub start_time: u64,
    /// Commit timestamp
    pub commit_time: u64,
    /// Start sequence
    pub start_seq: u64,
    /// Commit sequence
    pub commit_seq: u64,
    /// Record count
    pub record_count: u32,
    /// Block count
    pub block_count: u32,
}

impl TxnHandle {
    /// Size in bytes
    pub const SIZE: usize = 64;

    /// Create new handle
    pub fn new(id: TxnId) -> Self {
        Self {
            id,
            parent_id: 0,
            state: TxnState::Active as u8,
            flags: 0,
            _reserved: [0; 6],
            start_time: 0,
            commit_time: 0,
            start_seq: 0,
            commit_seq: 0,
            record_count: 0,
            block_count: 0,
        }
    }

    /// Get state
    #[inline]
    pub fn state(&self) -> TxnState {
        TxnState::from_raw(self.state)
    }

    /// Set state
    #[inline]
    pub fn set_state(&mut self, state: TxnState) {
        self.state = state as u8;
    }

    /// Get flags
    #[inline]
    pub fn flags(&self) -> TxnFlags {
        TxnFlags::new(self.flags as u32)
    }

    /// Is nested transaction
    #[inline]
    pub fn is_nested(&self) -> bool {
        self.parent_id != 0
    }
}

// Verify size
const _: () = assert!(core::mem::size_of::<TxnHandle>() == 64);

// ============================================================================
// Transaction Runtime State
// ============================================================================

/// Maximum records per transaction (for fixed buffer)
const MAX_TXN_RECORDS: usize = 256;

/// Maximum blocks per transaction
const MAX_TXN_BLOCKS: usize = 1024;

/// Transaction runtime state.
pub struct Transaction {
    /// Handle
    pub handle: TxnHandle,
    /// Atomic state
    state: AtomicU8,
    /// Start position in WAL
    pub start_pos: WalPosition,
    /// Current position in WAL
    pub current_pos: WalPosition,
    /// Record buffer (fixed size for no_std)
    pub records: [RecordPtr; MAX_TXN_RECORDS],
    /// Number of records
    pub record_count: usize,
    /// Block numbers touched
    pub blocks: [BlockNum; MAX_TXN_BLOCKS],
    /// Number of blocks
    pub block_count: usize,
    /// Bytes written
    pub bytes_written: u64,
    /// Last error
    pub last_error: Option<HfsError>,
}

/// Record pointer for transaction buffer.
#[derive(Clone, Copy, Debug, Default)]
pub struct RecordPtr {
    /// Record type
    pub record_type: u8,
    /// Flags
    pub flags: u8,
    /// Size
    pub size: u16,
    /// Block number
    pub block: BlockNum,
    /// Offset in WAL
    pub wal_offset: u32,
}

impl Transaction {
    /// Create new transaction
    pub fn new(id: TxnId) -> Self {
        Self {
            handle: TxnHandle::new(id),
            state: AtomicU8::new(TxnState::Active as u8),
            start_pos: WalPosition::invalid(),
            current_pos: WalPosition::invalid(),
            records: [RecordPtr::default(); MAX_TXN_RECORDS],
            record_count: 0,
            blocks: [BlockNum::NULL; MAX_TXN_BLOCKS],
            block_count: 0,
            bytes_written: 0,
            last_error: None,
        }
    }

    /// Get transaction ID
    #[inline]
    pub fn id(&self) -> TxnId {
        self.handle.id
    }

    /// Get state
    #[inline]
    pub fn state(&self) -> TxnState {
        TxnState::from_raw(self.state.load(Ordering::Acquire))
    }

    /// Set state atomically
    pub fn set_state(&self, state: TxnState) {
        self.state.store(state as u8, Ordering::Release);
    }

    /// Try to transition state
    pub fn try_transition(&self, from: TxnState, to: TxnState) -> bool {
        self.state
            .compare_exchange(from as u8, to as u8, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    /// Check if active
    #[inline]
    pub fn is_active(&self) -> bool {
        self.state().is_active()
    }

    /// Check if complete
    #[inline]
    pub fn is_complete(&self) -> bool {
        self.state().is_complete()
    }

    /// Add record to transaction
    pub fn add_record(&mut self, ptr: RecordPtr) -> HfsResult<()> {
        if self.record_count >= MAX_TXN_RECORDS {
            return Err(HfsError::TransactionTooLarge);
        }

        self.records[self.record_count] = ptr;
        self.record_count += 1;

        Ok(())
    }

    /// Add block to transaction
    pub fn add_block(&mut self, block: BlockNum) -> HfsResult<()> {
        // Check if already tracked
        for i in 0..self.block_count {
            if self.blocks[i] == block {
                return Ok(());
            }
        }

        if self.block_count >= MAX_TXN_BLOCKS {
            return Err(HfsError::TransactionTooLarge);
        }

        self.blocks[self.block_count] = block;
        self.block_count += 1;

        Ok(())
    }

    /// Set error
    pub fn set_error(&mut self, error: HfsError) {
        self.last_error = Some(error);
        self.set_state(TxnState::Error);
    }

    /// Get transaction size estimate
    pub fn size_estimate(&self) -> u64 {
        self.bytes_written + (self.record_count as u64 * 64)
    }
}

// ============================================================================
// Transaction Manager
// ============================================================================

/// Maximum concurrent transactions
const MAX_CONCURRENT_TXN: usize = 64;

/// Transaction manager state.
pub struct TxnManager {
    /// Next transaction ID
    pub next_id: AtomicU64,
    /// Active transaction count
    pub active_count: AtomicU64,
    /// Active transaction IDs (for tracking)
    pub active_ids: [AtomicU64; MAX_CONCURRENT_TXN],
    /// Statistics
    pub stats: TxnManagerStats,
}

impl TxnManager {
    /// Create new transaction manager
    pub fn new() -> Self {
        #[allow(clippy::declare_interior_mutable_const)]
        const ZERO: AtomicU64 = AtomicU64::new(0);
        Self {
            next_id: AtomicU64::new(1),
            active_count: AtomicU64::new(0),
            active_ids: [ZERO; MAX_CONCURRENT_TXN],
            stats: TxnManagerStats::new(),
        }
    }

    /// Allocate new transaction ID
    pub fn alloc_id(&self) -> TxnId {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Register active transaction
    pub fn register(&self, id: TxnId) -> HfsResult<usize> {
        for i in 0..MAX_CONCURRENT_TXN {
            if self.active_ids[i]
                .compare_exchange(0, id, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                self.active_count.fetch_add(1, Ordering::Relaxed);
                return Ok(i);
            }
        }

        Err(HfsError::TooManyTransactions)
    }

    /// Unregister transaction
    pub fn unregister(&self, id: TxnId) {
        for i in 0..MAX_CONCURRENT_TXN {
            if self.active_ids[i]
                .compare_exchange(id, 0, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                self.active_count.fetch_sub(1, Ordering::Relaxed);
                return;
            }
        }
    }

    /// Get active transaction count
    #[inline]
    pub fn active_count(&self) -> u64 {
        self.active_count.load(Ordering::Relaxed)
    }

    /// Get oldest active transaction
    pub fn oldest_active(&self) -> Option<TxnId> {
        let mut oldest = TxnId::MAX;

        for i in 0..MAX_CONCURRENT_TXN {
            let id = self.active_ids[i].load(Ordering::Relaxed);
            if id != 0 && id < oldest {
                oldest = id;
            }
        }

        if oldest == TxnId::MAX {
            None
        } else {
            Some(oldest)
        }
    }
}

impl Default for TxnManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Transaction manager statistics.
#[derive(Clone, Copy, Debug, Default)]
pub struct TxnManagerStats {
    /// Total transactions started
    pub started: u64,
    /// Total transactions committed
    pub committed: u64,
    /// Total transactions aborted
    pub aborted: u64,
    /// Peak concurrent transactions
    pub peak_concurrent: u64,
    /// Total wait time (for space)
    pub wait_time_ns: u64,
}

impl TxnManagerStats {
    /// Create new stats
    pub const fn new() -> Self {
        Self {
            started: 0,
            committed: 0,
            aborted: 0,
            peak_concurrent: 0,
            wait_time_ns: 0,
        }
    }

    /// Commit rate
    pub fn commit_rate(&self) -> f32 {
        let total = self.committed + self.aborted;
        if total == 0 {
            return 100.0;
        }
        (self.committed as f32 / total as f32) * 100.0
    }
}

// ============================================================================
// Commit Protocol
// ============================================================================

/// Commit protocol phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitPhase {
    /// Not started
    None,
    /// Phase 1: Write all records
    WriteRecords,
    /// Phase 2: Write commit record
    WriteCommit,
    /// Phase 3: Flush to disk
    Flush,
    /// Phase 4: Update metadata
    UpdateMeta,
    /// Complete
    Complete,
    /// Failed
    Failed,
}

/// Commit result.
#[derive(Clone, Copy, Debug)]
pub struct CommitResult {
    /// Transaction ID
    pub txn_id: TxnId,
    /// Commit sequence number
    pub sequence: u64,
    /// Phase reached
    pub phase: CommitPhase,
    /// Success
    pub success: bool,
    /// Error (if failed)
    pub error: Option<HfsError>,
    /// Commit duration (ns)
    pub duration_ns: u64,
}

impl CommitResult {
    /// Create success result
    pub fn success(txn_id: TxnId, sequence: u64, duration_ns: u64) -> Self {
        Self {
            txn_id,
            sequence,
            phase: CommitPhase::Complete,
            success: true,
            error: None,
            duration_ns,
        }
    }

    /// Create failure result
    pub fn failure(txn_id: TxnId, phase: CommitPhase, error: HfsError) -> Self {
        Self {
            txn_id,
            sequence: 0,
            phase,
            success: false,
            error: Some(error),
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
    fn test_txn_state() {
        assert!(TxnState::Active.is_active());
        assert!(TxnState::Committing.is_active());
        assert!(!TxnState::Committed.is_active());

        assert!(TxnState::Committed.is_complete());
        assert!(TxnState::Aborted.is_complete());

        assert!(TxnState::Active.can_commit());
        assert!(!TxnState::Committing.can_commit());
    }

    #[test]
    fn test_txn_flags() {
        let mut flags = TxnFlags::new(0);

        assert!(!flags.is_read_only());
        flags.set(TxnFlags::READ_ONLY);
        assert!(flags.is_read_only());

        flags.set(TxnFlags::SYNC);
        assert!(flags.is_sync());

        flags.clear(TxnFlags::SYNC);
        assert!(!flags.is_sync());
    }

    #[test]
    fn test_txn_handle() {
        let handle = TxnHandle::new(100);

        assert_eq!(handle.id, 100);
        assert_eq!(handle.state(), TxnState::Active);
        assert!(!handle.is_nested());
    }

    #[test]
    fn test_transaction() {
        let mut txn = Transaction::new(1);

        assert_eq!(txn.id(), 1);
        assert!(txn.is_active());
        assert!(!txn.is_complete());

        // Add record
        let ptr = RecordPtr {
            record_type: 1,
            flags: 0,
            size: 100,
            block: 50,
            wal_offset: 0,
        };
        txn.add_record(ptr).unwrap();
        assert_eq!(txn.record_count, 1);

        // Add block
        txn.add_block(100).unwrap();
        txn.add_block(100).unwrap(); // Duplicate, should not add
        assert_eq!(txn.block_count, 1);

        // State transition
        txn.set_state(TxnState::Committed);
        assert!(txn.is_complete());
    }

    #[test]
    fn test_txn_manager() {
        let manager = TxnManager::new();

        let id1 = manager.alloc_id();
        let id2 = manager.alloc_id();
        assert_eq!(id2, id1 + 1);

        let slot = manager.register(id1).unwrap();
        assert_eq!(manager.active_count(), 1);

        let oldest = manager.oldest_active();
        assert_eq!(oldest, Some(id1));

        manager.unregister(id1);
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_commit_result() {
        let success = CommitResult::success(1, 100, 1000);
        assert!(success.success);
        assert_eq!(success.phase, CommitPhase::Complete);

        let failure = CommitResult::failure(2, CommitPhase::Flush, HfsError::IoError);
        assert!(!failure.success);
        assert!(failure.error.is_some());
    }
}
