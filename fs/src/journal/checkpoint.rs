//! Checkpoint management for journal space reclamation.
//!
//! Provides checkpoint creation and management to allow
//! the journal to reclaim space by writing dirty data to disk.

use crate::core::types::*;
use crate::core::error::{HfsError, HfsResult};
use crate::core::hash::Crc32c;
use crate::journal::{JournalSuperblock, JournalState, CHECKPOINT_INTERVAL, CHECKPOINT_TIME_NS};
use crate::journal::wal::WalPosition;
use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};

// ============================================================================
// Checkpoint State
// ============================================================================

/// Checkpoint state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CheckpointState {
    /// Idle (no checkpoint in progress)
    Idle = 0,
    /// Preparing (gathering dirty blocks)
    Preparing = 1,
    /// Writing (flushing dirty blocks)
    Writing = 2,
    /// Syncing (syncing to disk)
    Syncing = 3,
    /// Completing (updating journal)
    Completing = 4,
    /// Complete
    Complete = 5,
    /// Failed
    Failed = 6,
}

impl CheckpointState {
    /// From raw value
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::Preparing,
            2 => Self::Writing,
            3 => Self::Syncing,
            4 => Self::Completing,
            5 => Self::Complete,
            6 => Self::Failed,
            _ => Self::Idle,
        }
    }
    
    /// Check if checkpoint is active
    #[inline]
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Preparing | Self::Writing | Self::Syncing | Self::Completing)
    }
}

impl Default for CheckpointState {
    fn default() -> Self {
        Self::Idle
    }
}

// ============================================================================
// Checkpoint Trigger
// ============================================================================

/// Reason for checkpoint trigger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckpointTrigger {
    /// Periodic timer
    Timer,
    /// Journal space low
    SpaceLow,
    /// Record count threshold
    RecordCount,
    /// Transaction count threshold
    TxnCount,
    /// Manual request
    Manual,
    /// Unmount/shutdown
    Shutdown,
    /// Sync request
    Sync,
}

// ============================================================================
// Checkpoint Descriptor (On-disk)
// ============================================================================

/// On-disk checkpoint descriptor.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct CheckpointDescriptor {
    /// Magic number
    pub magic: u32,
    /// Version
    pub version: u8,
    /// State
    pub state: u8,
    /// Flags
    pub flags: u16,
    /// Checkpoint generation
    pub generation: u64,
    /// Start timestamp
    pub start_time: u64,
    /// End timestamp
    pub end_time: u64,
    /// Oldest transaction at start
    pub oldest_txn: u64,
    /// Newest transaction at start
    pub newest_txn: u64,
    /// WAL position before checkpoint
    pub wal_before: u64,
    /// WAL position after checkpoint
    pub wal_after: u64,
    /// Blocks written
    pub blocks_written: u64,
    /// Inodes written
    pub inodes_written: u64,
    /// Bytes written
    pub bytes_written: u64,
    /// Journal blocks freed
    pub journal_freed: u64,
    /// Reserved
    pub _reserved: [u8; 412],
    /// Checksum
    pub checksum: u32,
}

impl CheckpointDescriptor {
    /// Size in bytes
    pub const SIZE: usize = 512;
    
    /// Magic number
    pub const MAGIC: u32 = 0x48435054; // "HCPT"
    
    /// Create new descriptor
    pub fn new(generation: u64) -> Self {
        Self {
            magic: Self::MAGIC,
            version: 1,
            state: CheckpointState::Idle as u8,
            flags: 0,
            generation,
            start_time: 0,
            end_time: 0,
            oldest_txn: 0,
            newest_txn: 0,
            wal_before: 0,
            wal_after: 0,
            blocks_written: 0,
            inodes_written: 0,
            bytes_written: 0,
            journal_freed: 0,
            _reserved: [0; 412],
            checksum: 0,
        }
    }
    
    /// Validate descriptor
    pub fn validate(&self) -> HfsResult<()> {
        if self.magic != Self::MAGIC {
            return Err(HfsError::InvalidCheckpoint);
        }
        Ok(())
    }
    
    /// Get state
    #[inline]
    pub fn state(&self) -> CheckpointState {
        CheckpointState::from_raw(self.state)
    }
    
    /// Compute checksum
    pub fn compute_checksum(&self) -> u32 {
        let mut hasher = Crc32c::new();
        let bytes = unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u8,
                Self::SIZE - 4,
            )
        };
        hasher.write(bytes);
        hasher.finish()
    }
    
    /// Update checksum
    pub fn update_checksum(&mut self) {
        self.checksum = self.compute_checksum();
    }
}

// Verify size
const _: () = assert!(core::mem::size_of::<CheckpointDescriptor>() == 512);

// ============================================================================
// Dirty Block Tracker
// ============================================================================

/// Maximum dirty blocks to track (for fixed-size array)
const MAX_DIRTY_BLOCKS: usize = 4096;

/// Dirty block entry.
#[derive(Clone, Copy, Debug, Default)]
pub struct DirtyBlock {
    /// Block number
    pub block: BlockNum,
    /// Transaction that dirtied it
    pub txn_id: u64,
    /// Journal sequence
    pub journal_seq: u64,
    /// Flags
    pub flags: u8,
}

impl DirtyBlock {
    /// Create new entry
    pub fn new(block: BlockNum, txn_id: u64, journal_seq: u64) -> Self {
        Self {
            block,
            txn_id,
            journal_seq,
            flags: 0,
        }
    }
    
    /// Check if valid
    #[inline]
    pub fn is_valid(&self) -> bool {
        !self.block.is_null()
    }
}

/// Dirty block tracker.
pub struct DirtyTracker {
    /// Dirty blocks
    pub blocks: [DirtyBlock; MAX_DIRTY_BLOCKS],
    /// Number of dirty blocks
    pub count: usize,
    /// Oldest dirty sequence
    pub oldest_seq: u64,
    /// Newest dirty sequence
    pub newest_seq: u64,
}

impl DirtyTracker {
    /// Create new tracker
    pub fn new() -> Self {
        Self {
            blocks: [DirtyBlock::default(); MAX_DIRTY_BLOCKS],
            count: 0,
            oldest_seq: u64::MAX,
            newest_seq: 0,
        }
    }
    
    /// Add dirty block
    pub fn add(&mut self, block: BlockNum, txn_id: u64, seq: u64) -> HfsResult<()> {
        // Check if already tracked
        for i in 0..self.count {
            if self.blocks[i].block == block {
                // Update to newer transaction
                if txn_id > self.blocks[i].txn_id {
                    self.blocks[i].txn_id = txn_id;
                    self.blocks[i].journal_seq = seq;
                }
                return Ok(());
            }
        }
        
        // Add new
        if self.count >= MAX_DIRTY_BLOCKS {
            return Err(HfsError::OutOfMemory);
        }
        
        self.blocks[self.count] = DirtyBlock::new(block, txn_id, seq);
        self.count += 1;
        
        // Update bounds
        if seq < self.oldest_seq {
            self.oldest_seq = seq;
        }
        if seq > self.newest_seq {
            self.newest_seq = seq;
        }
        
        Ok(())
    }
    
    /// Remove dirty block (after write)
    pub fn remove(&mut self, block: BlockNum) -> bool {
        for i in 0..self.count {
            if self.blocks[i].block == block {
                // Swap with last
                self.blocks[i] = self.blocks[self.count - 1];
                self.blocks[self.count - 1] = DirtyBlock::default();
                self.count -= 1;
                return true;
            }
        }
        false
    }
    
    /// Clear all dirty blocks
    pub fn clear(&mut self) {
        for i in 0..self.count {
            self.blocks[i] = DirtyBlock::default();
        }
        self.count = 0;
        self.oldest_seq = u64::MAX;
        self.newest_seq = 0;
    }
    
    /// Get blocks older than sequence
    pub fn blocks_before(&self, seq: u64) -> usize {
        let mut count = 0;
        for i in 0..self.count {
            if self.blocks[i].journal_seq < seq {
                count += 1;
            }
        }
        count
    }
    
    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

impl Default for DirtyTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Checkpoint Manager
// ============================================================================

/// Checkpoint manager state.
pub struct CheckpointManager {
    /// Current checkpoint generation
    pub generation: AtomicU64,
    /// Checkpoint in progress
    pub in_progress: AtomicBool,
    /// Last checkpoint time (nanoseconds)
    pub last_checkpoint_time: AtomicU64,
    /// Records since last checkpoint
    pub records_since: AtomicU64,
    /// Transactions since last checkpoint
    pub txns_since: AtomicU64,
    /// Statistics
    pub stats: CheckpointStats,
}

impl CheckpointManager {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            generation: AtomicU64::new(0),
            in_progress: AtomicBool::new(false),
            last_checkpoint_time: AtomicU64::new(0),
            records_since: AtomicU64::new(0),
            txns_since: AtomicU64::new(0),
            stats: CheckpointStats::new(),
        }
    }
    
    /// Get current generation
    #[inline]
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Relaxed)
    }
    
    /// Check if checkpoint in progress
    #[inline]
    pub fn is_in_progress(&self) -> bool {
        self.in_progress.load(Ordering::Acquire)
    }
    
    /// Try to start checkpoint
    pub fn try_start(&self) -> bool {
        self.in_progress.compare_exchange(
            false,
            true,
            Ordering::AcqRel,
            Ordering::Acquire,
        ).is_ok()
    }
    
    /// End checkpoint
    pub fn end(&self) {
        self.generation.fetch_add(1, Ordering::Relaxed);
        self.records_since.store(0, Ordering::Relaxed);
        self.txns_since.store(0, Ordering::Relaxed);
        self.in_progress.store(false, Ordering::Release);
    }
    
    /// Record a transaction commit
    pub fn record_commit(&self, records: u64) {
        self.records_since.fetch_add(records, Ordering::Relaxed);
        self.txns_since.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Check if checkpoint should be triggered
    pub fn should_checkpoint(&self, current_time: u64, free_space_pct: u8) -> Option<CheckpointTrigger> {
        // Already in progress?
        if self.is_in_progress() {
            return None;
        }
        
        // Low space (< 25%)
        if free_space_pct < 25 {
            return Some(CheckpointTrigger::SpaceLow);
        }
        
        // Record count
        if self.records_since.load(Ordering::Relaxed) >= CHECKPOINT_INTERVAL {
            return Some(CheckpointTrigger::RecordCount);
        }
        
        // Time-based
        let last = self.last_checkpoint_time.load(Ordering::Relaxed);
        if current_time.saturating_sub(last) >= CHECKPOINT_TIME_NS {
            return Some(CheckpointTrigger::Timer);
        }
        
        None
    }
}

impl Default for CheckpointManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Checkpoint statistics.
#[derive(Clone, Copy, Debug, Default)]
pub struct CheckpointStats {
    /// Total checkpoints completed
    pub completed: u64,
    /// Total checkpoints failed
    pub failed: u64,
    /// Total blocks written by checkpoints
    pub blocks_written: u64,
    /// Total bytes written by checkpoints
    pub bytes_written: u64,
    /// Total journal space freed
    pub journal_freed: u64,
    /// Average checkpoint duration (ns)
    pub avg_duration_ns: u64,
    /// Maximum checkpoint duration (ns)
    pub max_duration_ns: u64,
}

impl CheckpointStats {
    /// Create new stats
    pub const fn new() -> Self {
        Self {
            completed: 0,
            failed: 0,
            blocks_written: 0,
            bytes_written: 0,
            journal_freed: 0,
            avg_duration_ns: 0,
            max_duration_ns: 0,
        }
    }
    
    /// Record checkpoint completion
    pub fn record_complete(&mut self, desc: &CheckpointDescriptor) {
        self.completed += 1;
        self.blocks_written += desc.blocks_written;
        self.bytes_written += desc.bytes_written;
        self.journal_freed += desc.journal_freed;
        
        let duration = desc.end_time.saturating_sub(desc.start_time);
        
        // Update average
        if self.completed > 0 {
            self.avg_duration_ns = (self.avg_duration_ns * (self.completed - 1) + duration)
                / self.completed;
        }
        
        if duration > self.max_duration_ns {
            self.max_duration_ns = duration;
        }
    }
    
    /// Record checkpoint failure
    pub fn record_failure(&mut self) {
        self.failed += 1;
    }
}

// ============================================================================
// Checkpoint Work Item
// ============================================================================

/// Work item for checkpoint worker.
#[derive(Clone, Copy, Debug)]
pub struct CheckpointWork {
    /// Block number to write
    pub block: BlockNum,
    /// Source (journal or cache)
    pub source: WorkSource,
    /// Priority
    pub priority: u8,
    /// Transaction that dirtied
    pub txn_id: u64,
}

/// Source of checkpoint work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkSource {
    /// From journal
    Journal,
    /// From buffer cache
    Cache,
    /// From metadata cache
    Metadata,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_checkpoint_state() {
        assert!(!CheckpointState::Idle.is_active());
        assert!(CheckpointState::Writing.is_active());
        assert!(!CheckpointState::Complete.is_active());
    }
    
    #[test]
    fn test_checkpoint_descriptor() {
        let desc = CheckpointDescriptor::new(1);
        
        assert_eq!(desc.magic, CheckpointDescriptor::MAGIC);
        assert_eq!(desc.generation, 1);
        assert!(desc.validate().is_ok());
    }
    
    #[test]
    fn test_dirty_tracker() {
        let mut tracker = DirtyTracker::new();
        
        assert!(tracker.is_empty());
        
        tracker.add(100, 1, 10).unwrap();
        tracker.add(200, 1, 20).unwrap();
        
        assert_eq!(tracker.count, 2);
        assert_eq!(tracker.oldest_seq, 10);
        assert_eq!(tracker.newest_seq, 20);
        
        // Duplicate should update
        tracker.add(100, 2, 15).unwrap();
        assert_eq!(tracker.count, 2);
        
        // Remove
        assert!(tracker.remove(100));
        assert_eq!(tracker.count, 1);
        
        tracker.clear();
        assert!(tracker.is_empty());
    }
    
    #[test]
    fn test_checkpoint_manager() {
        let manager = CheckpointManager::new();
        
        assert_eq!(manager.generation(), 0);
        assert!(!manager.is_in_progress());
        
        assert!(manager.try_start());
        assert!(manager.is_in_progress());
        assert!(!manager.try_start()); // Can't start again
        
        manager.end();
        assert!(!manager.is_in_progress());
        assert_eq!(manager.generation(), 1);
    }
    
    #[test]
    fn test_checkpoint_trigger() {
        let manager = CheckpointManager::new();
        
        // Should not trigger immediately
        assert!(manager.should_checkpoint(0, 100).is_none());
        
        // Low space should trigger
        assert_eq!(
            manager.should_checkpoint(0, 20),
            Some(CheckpointTrigger::SpaceLow)
        );
        
        // Add many records
        for _ in 0..CHECKPOINT_INTERVAL {
            manager.record_commit(1);
        }
        assert_eq!(
            manager.should_checkpoint(0, 100),
            Some(CheckpointTrigger::RecordCount)
        );
    }
}
