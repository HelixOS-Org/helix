//! Copy-on-Write block management.
//!
//! Tracks block reference counts and handles CoW semantics
//! for snapshots and clones.

use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crate::core::error::{HfsError, HfsResult};
use crate::core::types::*;

// ============================================================================
// Constants
// ============================================================================

/// Maximum reference count per block
pub const MAX_REFCOUNT: u32 = u32::MAX - 1;

/// Entries per refcount block
pub const REFCOUNT_ENTRIES_PER_BLOCK: usize = 1024; // 4096 / 4

/// Refcount indicating block is not tracked
pub const REFCOUNT_UNTRACKED: u32 = 0;

/// Refcount indicating single reference (no CoW needed)
pub const REFCOUNT_SINGLE: u32 = 1;

// ============================================================================
// Refcount Entry
// ============================================================================

/// Reference count for a block.
#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub struct RefCount(u32);

impl RefCount {
    /// Create new refcount
    #[inline]
    pub const fn new(count: u32) -> Self {
        Self(count)
    }

    /// Get count
    #[inline]
    pub fn get(&self) -> u32 {
        self.0
    }

    /// Check if untracked
    #[inline]
    pub fn is_untracked(&self) -> bool {
        self.0 == REFCOUNT_UNTRACKED
    }

    /// Check if single reference
    #[inline]
    pub fn is_single(&self) -> bool {
        self.0 == REFCOUNT_SINGLE
    }

    /// Check if shared (CoW needed for write)
    #[inline]
    pub fn is_shared(&self) -> bool {
        self.0 > REFCOUNT_SINGLE
    }

    /// Increment refcount
    pub fn increment(&mut self) -> HfsResult<()> {
        if self.0 >= MAX_REFCOUNT {
            return Err(HfsError::RefcountOverflow);
        }
        self.0 += 1;
        Ok(())
    }

    /// Decrement refcount, returns true if reached zero
    pub fn decrement(&mut self) -> HfsResult<bool> {
        if self.0 == 0 {
            return Err(HfsError::RefcountUnderflow);
        }
        self.0 -= 1;
        Ok(self.0 == 0)
    }
}

// ============================================================================
// Refcount Block
// ============================================================================

/// Block of reference counts (covers REFCOUNT_ENTRIES_PER_BLOCK blocks).
#[repr(C, align(4096))]
pub struct RefCountBlock {
    /// Reference counts
    entries: [AtomicU32; REFCOUNT_ENTRIES_PER_BLOCK],
}

impl RefCountBlock {
    /// Create new block (all untracked)
    pub const fn new() -> Self {
        const ZERO: AtomicU32 = AtomicU32::new(0);
        Self {
            entries: [ZERO; REFCOUNT_ENTRIES_PER_BLOCK],
        }
    }

    /// Get refcount for entry
    #[inline]
    pub fn get(&self, index: usize) -> RefCount {
        debug_assert!(index < REFCOUNT_ENTRIES_PER_BLOCK);
        RefCount(self.entries[index].load(Ordering::Relaxed))
    }

    /// Set refcount
    #[inline]
    pub fn set(&self, index: usize, count: u32) {
        debug_assert!(index < REFCOUNT_ENTRIES_PER_BLOCK);
        self.entries[index].store(count, Ordering::Relaxed);
    }

    /// Atomic increment
    pub fn increment(&self, index: usize) -> HfsResult<u32> {
        debug_assert!(index < REFCOUNT_ENTRIES_PER_BLOCK);

        let mut current = self.entries[index].load(Ordering::Relaxed);
        loop {
            if current >= MAX_REFCOUNT {
                return Err(HfsError::RefcountOverflow);
            }

            match self.entries[index].compare_exchange_weak(
                current,
                current + 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Ok(current + 1),
                Err(c) => current = c,
            }
        }
    }

    /// Atomic decrement, returns true if reached zero
    pub fn decrement(&self, index: usize) -> HfsResult<bool> {
        debug_assert!(index < REFCOUNT_ENTRIES_PER_BLOCK);

        let mut current = self.entries[index].load(Ordering::Relaxed);
        loop {
            if current == 0 {
                return Err(HfsError::RefcountUnderflow);
            }

            match self.entries[index].compare_exchange_weak(
                current,
                current - 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Ok(current == 1),
                Err(c) => current = c,
            }
        }
    }

    /// Check if all entries are untracked (block can be freed)
    pub fn is_empty(&self) -> bool {
        self.entries.iter().all(|e| e.load(Ordering::Relaxed) == 0)
    }

    /// Count non-zero entries
    pub fn count_tracked(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.load(Ordering::Relaxed) > 0)
            .count()
    }

    /// Load from bytes
    pub fn from_bytes(bytes: &[u8; 4096]) -> Self {
        let block = Self::new();
        for (i, chunk) in bytes.chunks_exact(4).enumerate() {
            let value = u32::from_le_bytes(chunk.try_into().unwrap());
            block.entries[i].store(value, Ordering::Relaxed);
        }
        block
    }

    /// Store to bytes
    pub fn to_bytes(&self, bytes: &mut [u8; 4096]) {
        for (i, entry) in self.entries.iter().enumerate() {
            let value = entry.load(Ordering::Relaxed);
            bytes[i * 4..(i + 1) * 4].copy_from_slice(&value.to_le_bytes());
        }
    }
}

impl Default for RefCountBlock {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// CoW State Machine
// ============================================================================

/// State of a CoW operation
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CowState {
    /// Block is unique (no sharing)
    Unique,
    /// Block is shared but not being modified
    SharedClean,
    /// Block is being copied for modification
    Copying,
    /// Copy complete, old block can be decremented
    CopyComplete,
    /// Error during CoW
    Error,
}

/// Result of CoW check
#[derive(Clone, Copy, Debug)]
pub struct CowCheckResult {
    /// Original block
    pub original: BlockNum,
    /// Reference count of original
    pub refcount: u32,
    /// Whether CoW is needed
    pub needs_cow: bool,
}

impl CowCheckResult {
    /// Create result
    pub fn new(original: BlockNum, refcount: u32) -> Self {
        Self {
            original,
            refcount,
            needs_cow: refcount > 1,
        }
    }

    /// Check if unique (no CoW needed)
    #[inline]
    pub fn is_unique(&self) -> bool {
        !self.needs_cow
    }
}

/// Result of CoW operation
#[derive(Clone, Copy, Debug)]
pub struct CowResult {
    /// Original block
    pub original: BlockNum,
    /// New block (same as original if unique)
    pub new_block: BlockNum,
    /// Whether data was copied
    pub copied: bool,
}

impl CowResult {
    /// Create for unique block (no copy needed)
    pub fn unique(block: BlockNum) -> Self {
        Self {
            original: block,
            new_block: block,
            copied: false,
        }
    }

    /// Create for copied block
    pub fn copied(original: BlockNum, new_block: BlockNum) -> Self {
        Self {
            original,
            new_block,
            copied: true,
        }
    }
}

// ============================================================================
// CoW Transaction
// ============================================================================

/// Tracks blocks involved in a CoW transaction.
pub struct CowTransaction {
    /// Transaction ID
    txn_id: u64,
    /// Original blocks being copied
    original_blocks: [u64; 64],
    /// New blocks allocated
    new_blocks: [u64; 64],
    /// Number of blocks in transaction
    count: usize,
    /// State
    state: CowState,
}

impl CowTransaction {
    /// Create new transaction
    pub fn new(txn_id: u64) -> Self {
        Self {
            txn_id,
            original_blocks: [0; 64],
            new_blocks: [0; 64],
            count: 0,
            state: CowState::Unique,
        }
    }

    /// Add a block mapping
    pub fn add(&mut self, original: u64, new: u64) -> HfsResult<()> {
        if self.count >= 64 {
            return Err(HfsError::TransactionTooLarge);
        }

        self.original_blocks[self.count] = original;
        self.new_blocks[self.count] = new;
        self.count += 1;
        self.state = CowState::Copying;

        Ok(())
    }

    /// Mark complete
    pub fn complete(&mut self) {
        self.state = CowState::CopyComplete;
    }

    /// Mark error
    pub fn error(&mut self) {
        self.state = CowState::Error;
    }

    /// Get transaction ID
    #[inline]
    pub fn txn_id(&self) -> u64 {
        self.txn_id
    }

    /// Get state
    #[inline]
    pub fn state(&self) -> CowState {
        self.state
    }

    /// Get mappings
    pub fn mappings(&self) -> impl Iterator<Item = (u64, u64)> + '_ {
        self.original_blocks[..self.count]
            .iter()
            .copied()
            .zip(self.new_blocks[..self.count].iter().copied())
    }
}

// ============================================================================
// CoW Statistics
// ============================================================================

/// CoW operation statistics.
#[derive(Default)]
pub struct CowStats {
    /// Total CoW checks
    pub checks: AtomicU64,
    /// Blocks found unique (no CoW)
    pub unique: AtomicU64,
    /// Blocks requiring copy
    pub copies: AtomicU64,
    /// Bytes copied
    pub bytes_copied: AtomicU64,
    /// Failed CoW operations
    pub failures: AtomicU64,
    /// Refcount increments
    pub refcount_incs: AtomicU64,
    /// Refcount decrements
    pub refcount_decs: AtomicU64,
    /// Blocks freed by decrement
    pub freed_by_dec: AtomicU64,
}

impl CowStats {
    /// Create new stats
    pub const fn new() -> Self {
        Self {
            checks: AtomicU64::new(0),
            unique: AtomicU64::new(0),
            copies: AtomicU64::new(0),
            bytes_copied: AtomicU64::new(0),
            failures: AtomicU64::new(0),
            refcount_incs: AtomicU64::new(0),
            refcount_decs: AtomicU64::new(0),
            freed_by_dec: AtomicU64::new(0),
        }
    }

    /// Record check
    pub fn record_check(&self, needs_cow: bool) {
        self.checks.fetch_add(1, Ordering::Relaxed);
        if needs_cow {
            // Will record copy when actually done
        } else {
            self.unique.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record copy
    pub fn record_copy(&self, bytes: u64) {
        self.copies.fetch_add(1, Ordering::Relaxed);
        self.bytes_copied.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record refcount increment
    pub fn record_inc(&self) {
        self.refcount_incs.fetch_add(1, Ordering::Relaxed);
    }

    /// Record refcount decrement
    pub fn record_dec(&self, freed: bool) {
        self.refcount_decs.fetch_add(1, Ordering::Relaxed);
        if freed {
            self.freed_by_dec.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get snapshot
    pub fn snapshot(&self) -> CowStatsSnapshot {
        CowStatsSnapshot {
            checks: self.checks.load(Ordering::Relaxed),
            unique: self.unique.load(Ordering::Relaxed),
            copies: self.copies.load(Ordering::Relaxed),
            bytes_copied: self.bytes_copied.load(Ordering::Relaxed),
            failures: self.failures.load(Ordering::Relaxed),
            refcount_incs: self.refcount_incs.load(Ordering::Relaxed),
            refcount_decs: self.refcount_decs.load(Ordering::Relaxed),
            freed_by_dec: self.freed_by_dec.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of CoW stats
#[derive(Clone, Copy, Debug, Default)]
pub struct CowStatsSnapshot {
    pub checks: u64,
    pub unique: u64,
    pub copies: u64,
    pub bytes_copied: u64,
    pub failures: u64,
    pub refcount_incs: u64,
    pub refcount_decs: u64,
    pub freed_by_dec: u64,
}

impl CowStatsSnapshot {
    /// CoW ratio (copies / checks)
    pub fn cow_ratio(&self) -> f64 {
        if self.checks == 0 {
            0.0
        } else {
            self.copies as f64 / self.checks as f64
        }
    }

    /// Average refcount changes per check
    pub fn avg_refcount_ops(&self) -> f64 {
        if self.checks == 0 {
            0.0
        } else {
            (self.refcount_incs + self.refcount_decs) as f64 / self.checks as f64
        }
    }
}

// ============================================================================
// Shared Extent Tracker
// ============================================================================

/// Tracks shared extents (for efficient range operations).
#[derive(Clone, Copy, Debug)]
pub struct SharedExtent {
    /// Start block
    pub start: u64,
    /// Length in blocks
    pub length: u32,
    /// Reference count
    pub refcount: u32,
    /// Owner snapshot ID
    pub snapshot_id: u64,
}

impl SharedExtent {
    /// Create new shared extent
    pub const fn new(start: u64, length: u32, refcount: u32) -> Self {
        Self {
            start,
            length,
            refcount,
            snapshot_id: 0,
        }
    }

    /// Get end block (exclusive)
    #[inline]
    pub fn end(&self) -> u64 {
        self.start + self.length as u64
    }

    /// Check if block is in extent
    #[inline]
    pub fn contains(&self, block: u64) -> bool {
        block >= self.start && block < self.end()
    }

    /// Check if shared
    #[inline]
    pub fn is_shared(&self) -> bool {
        self.refcount > 1
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refcount_basic() {
        let mut rc = RefCount::new(0);
        assert!(rc.is_untracked());

        rc.increment().unwrap();
        assert!(rc.is_single());

        rc.increment().unwrap();
        assert!(rc.is_shared());
        assert_eq!(rc.get(), 2);

        let freed = rc.decrement().unwrap();
        assert!(!freed);

        let freed = rc.decrement().unwrap();
        assert!(freed);
    }

    #[test]
    fn test_refcount_block() {
        let block = RefCountBlock::new();

        assert!(block.is_empty());

        block.set(0, 1);
        block.set(100, 2);

        assert!(!block.is_empty());
        assert_eq!(block.get(0).get(), 1);
        assert_eq!(block.get(100).get(), 2);

        let new_val = block.increment(0).unwrap();
        assert_eq!(new_val, 2);

        let freed = block.decrement(0).unwrap();
        assert!(!freed);

        let freed = block.decrement(0).unwrap();
        assert!(freed);
    }

    #[test]
    fn test_cow_check_result() {
        let unique = CowCheckResult::new(BlockNum::new(100), 1);
        assert!(unique.is_unique());
        assert!(!unique.needs_cow);

        let shared = CowCheckResult::new(BlockNum::new(100), 3);
        assert!(!shared.is_unique());
        assert!(shared.needs_cow);
    }

    #[test]
    fn test_cow_transaction() {
        let mut txn = CowTransaction::new(42);

        assert_eq!(txn.txn_id(), 42);
        assert_eq!(txn.state(), CowState::Unique);

        txn.add(100, 200).unwrap();
        txn.add(101, 201).unwrap();

        assert_eq!(txn.state(), CowState::Copying);

        let mappings: Vec<_> = txn.mappings().collect();
        assert_eq!(mappings.len(), 2);
        assert_eq!(mappings[0], (100, 200));

        txn.complete();
        assert_eq!(txn.state(), CowState::CopyComplete);
    }

    #[test]
    fn test_cow_stats() {
        let stats = CowStats::new();

        stats.record_check(false);
        stats.record_check(true);
        stats.record_copy(4096);
        stats.record_inc();
        stats.record_dec(true);

        let snap = stats.snapshot();
        assert_eq!(snap.checks, 2);
        assert_eq!(snap.unique, 1);
        assert_eq!(snap.copies, 1);
        assert_eq!(snap.bytes_copied, 4096);
    }
}
