//! Write-Ahead Log implementation.
//!
//! Provides circular buffer log with ordered writes
//! and crash-consistent commit protocol.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::core::error::{HfsError, HfsResult};
use crate::core::hash::Crc32c;
use crate::journal::JOURNAL_BLOCK_SIZE;

// ============================================================================
// WAL Block Header
// ============================================================================

/// WAL block header.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct WalBlockHeader {
    /// Magic number
    pub magic: u32,
    /// Block type
    pub block_type: u8,
    /// Flags
    pub flags: u8,
    /// Reserved
    pub _reserved: u16,
    /// Sequence number
    pub sequence: u64,
    /// Transaction ID
    pub txn_id: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Data length in block
    pub data_len: u16,
    /// Continued from previous block
    pub continued: u8,
    /// Continues to next block
    pub continues: u8,
    /// Checksum of header
    pub header_crc: u32,
    /// Checksum of data
    pub data_crc: u32,
    /// Padding
    pub _pad: [u8; 8],
}

impl WalBlockHeader {
    /// Header size
    pub const SIZE: usize = 56;

    /// Magic number
    pub const MAGIC: u32 = 0x57414C42; // "WALB"

    /// Create new header
    pub fn new(sequence: u64, txn_id: u64, data_len: u16) -> Self {
        Self {
            magic: Self::MAGIC,
            block_type: WalBlockType::Data as u8,
            flags: 0,
            _reserved: 0,
            sequence,
            txn_id,
            timestamp: 0, // Filled in on write
            data_len,
            continued: 0,
            continues: 0,
            header_crc: 0,
            data_crc: 0,
            _pad: [0; 8],
        }
    }

    /// Validate header
    pub fn validate(&self) -> HfsResult<()> {
        if self.magic != Self::MAGIC {
            return Err(HfsError::JournalCorrupted);
        }
        Ok(())
    }

    /// Maximum data per block
    pub const fn max_data_per_block() -> usize {
        JOURNAL_BLOCK_SIZE - Self::SIZE
    }

    /// Compute header checksum
    pub fn compute_header_crc(&self) -> u32 {
        let mut hasher = Crc32c::new();

        // Hash fields before header_crc
        let bytes = unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u8,
                40, // Offset of header_crc
            )
        };
        hasher.write(bytes);

        hasher.finish()
    }

    /// Update header checksum
    pub fn update_header_crc(&mut self) {
        self.header_crc = self.compute_header_crc();
    }
}

/// WAL block type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum WalBlockType {
    /// Empty block
    Empty      = 0,
    /// Data block
    Data       = 1,
    /// Commit record
    Commit     = 2,
    /// Checkpoint
    Checkpoint = 3,
    /// Descriptor block
    Descriptor = 4,
    /// Revoke block
    Revoke     = 5,
}

impl WalBlockType {
    /// From raw value
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::Data,
            2 => Self::Commit,
            3 => Self::Checkpoint,
            4 => Self::Descriptor,
            5 => Self::Revoke,
            _ => Self::Empty,
        }
    }
}

// ============================================================================
// WAL Position
// ============================================================================

/// Position in the WAL.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct WalPosition {
    /// Block number (within journal)
    pub block: u64,
    /// Offset within block
    pub offset: u16,
    /// Sequence number
    pub sequence: u64,
}

impl WalPosition {
    /// Create position
    pub const fn new(block: u64, offset: u16, sequence: u64) -> Self {
        Self {
            block,
            offset,
            sequence,
        }
    }

    /// Start of journal
    pub const fn start() -> Self {
        Self {
            block: 1,
            offset: 0,
            sequence: 1,
        }
    }

    /// Invalid position
    pub const fn invalid() -> Self {
        Self {
            block: 0,
            offset: 0,
            sequence: 0,
        }
    }

    /// Check if valid
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.sequence > 0
    }

    /// Advance by blocks
    pub fn advance(&mut self, blocks: u64, journal_size: u64) {
        self.block += blocks;
        if self.block >= journal_size {
            self.block = 1; // Wrap around (skip superblock)
        }
        self.sequence += blocks;
    }
}

impl Default for WalPosition {
    fn default() -> Self {
        Self::invalid()
    }
}

// ============================================================================
// WAL Buffer
// ============================================================================

/// Size of WAL buffer (must be power of 2)
const WAL_BUFFER_SIZE: usize = 64;

/// In-memory WAL buffer entry.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct WalBufferEntry {
    /// Data
    pub data: [u8; JOURNAL_BLOCK_SIZE],
    /// Sequence number
    pub sequence: u64,
    /// Transaction ID
    pub txn_id: u64,
    /// State
    pub state: WalEntryState,
    /// Padding
    pub _pad: [u8; 7],
}

/// WAL entry state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum WalEntryState {
    /// Free
    Free     = 0,
    /// Being filled
    Filling  = 1,
    /// Ready to flush
    Ready    = 2,
    /// Being flushed
    Flushing = 3,
    /// Flushed
    Flushed  = 4,
}

impl Default for WalBufferEntry {
    fn default() -> Self {
        Self {
            data: [0; JOURNAL_BLOCK_SIZE],
            sequence: 0,
            txn_id: 0,
            state: WalEntryState::Free,
            _pad: [0; 7],
        }
    }
}

/// WAL buffer statistics.
#[derive(Clone, Copy, Debug, Default)]
pub struct WalBufferStats {
    /// Buffers allocated
    pub allocated: u32,
    /// Buffers in use
    pub in_use: u32,
    /// Buffers waiting flush
    pub pending_flush: u32,
    /// Total flushes
    pub total_flushes: u64,
    /// Bytes flushed
    pub bytes_flushed: u64,
}

// ============================================================================
// WAL Writer State
// ============================================================================

/// WAL writer state.
pub struct WalWriter {
    /// Current write position
    pub write_pos: WalPosition,
    /// Flush position (last flushed)
    pub flush_pos: WalPosition,
    /// Commit position (last committed)
    pub commit_pos: WalPosition,
    /// Journal size in blocks
    pub journal_size: u64,
    /// Current sequence
    pub sequence: AtomicU64,
    /// Next transaction ID
    pub next_txn_id: AtomicU64,
    /// Is running
    pub running: AtomicBool,
    /// Statistics
    pub stats: WalBufferStats,
}

impl WalWriter {
    /// Create new WAL writer.
    pub fn new(journal_size: u64) -> Self {
        Self {
            write_pos: WalPosition::start(),
            flush_pos: WalPosition::start(),
            commit_pos: WalPosition::start(),
            journal_size,
            sequence: AtomicU64::new(1),
            next_txn_id: AtomicU64::new(1),
            running: AtomicBool::new(false),
            stats: WalBufferStats::default(),
        }
    }

    /// Start writer
    pub fn start(&self) {
        self.running.store(true, Ordering::Release);
    }

    /// Stop writer
    pub fn stop(&self) {
        self.running.store(false, Ordering::Release);
    }

    /// Check if running
    #[inline]
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    /// Get next sequence number
    pub fn next_sequence(&self) -> u64 {
        self.sequence.fetch_add(1, Ordering::Relaxed)
    }

    /// Allocate transaction ID
    pub fn alloc_txn_id(&self) -> u64 {
        self.next_txn_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Calculate blocks needed for data
    pub fn blocks_for_data(data_len: usize) -> u64 {
        let per_block = WalBlockHeader::max_data_per_block();
        ((data_len + per_block - 1) / per_block) as u64
    }

    /// Check if there's space for write
    pub fn has_space(&self, blocks: u64) -> bool {
        let used = self.used_blocks();
        used + blocks < self.journal_size - 1 // Reserve at least 1 block
    }

    /// Calculate used blocks
    pub fn used_blocks(&self) -> u64 {
        let write = self.write_pos.block;
        let commit = self.commit_pos.block;

        if write >= commit {
            write - commit
        } else {
            // Wrapped
            (self.journal_size - commit) + write
        }
    }

    /// Calculate free blocks
    pub fn free_blocks(&self) -> u64 {
        self.journal_size.saturating_sub(self.used_blocks() + 1)
    }
}

// ============================================================================
// WAL Flush Request
// ============================================================================

/// Flush request.
#[derive(Clone, Copy, Debug)]
pub struct WalFlushRequest {
    /// Start position
    pub start: WalPosition,
    /// End position
    pub end: WalPosition,
    /// Transaction ID (for sync)
    pub txn_id: u64,
    /// Is sync request
    pub sync: bool,
    /// Is commit record
    pub is_commit: bool,
}

impl WalFlushRequest {
    /// Create flush request
    pub fn new(start: WalPosition, end: WalPosition, txn_id: u64) -> Self {
        Self {
            start,
            end,
            txn_id,
            sync: false,
            is_commit: false,
        }
    }

    /// Create sync flush request
    pub fn sync(start: WalPosition, end: WalPosition, txn_id: u64) -> Self {
        Self {
            start,
            end,
            txn_id,
            sync: true,
            is_commit: false,
        }
    }

    /// Create commit flush request
    pub fn commit(start: WalPosition, end: WalPosition, txn_id: u64) -> Self {
        Self {
            start,
            end,
            txn_id,
            sync: true,
            is_commit: true,
        }
    }
}

// ============================================================================
// WAL Reader
// ============================================================================

/// WAL reader state.
pub struct WalReader {
    /// Current read position
    pub read_pos: WalPosition,
    /// End position
    pub end_pos: WalPosition,
    /// Journal size
    pub journal_size: u64,
    /// Blocks read
    pub blocks_read: u64,
    /// Errors encountered
    pub errors: u32,
}

impl WalReader {
    /// Create new reader
    pub fn new(start: WalPosition, end: WalPosition, journal_size: u64) -> Self {
        Self {
            read_pos: start,
            end_pos: end,
            journal_size,
            blocks_read: 0,
            errors: 0,
        }
    }

    /// Check if at end
    #[inline]
    pub fn at_end(&self) -> bool {
        self.read_pos.sequence >= self.end_pos.sequence
    }

    /// Advance to next block
    pub fn advance(&mut self) {
        self.read_pos.block += 1;
        if self.read_pos.block >= self.journal_size {
            self.read_pos.block = 1;
        }
        self.read_pos.sequence += 1;
        self.blocks_read += 1;
    }

    /// Remaining blocks to read
    pub fn remaining(&self) -> u64 {
        if self.end_pos.sequence > self.read_pos.sequence {
            self.end_pos.sequence - self.read_pos.sequence
        } else {
            0
        }
    }
}

// ============================================================================
// WAL Scanner (for recovery)
// ============================================================================

/// WAL scanner for recovery operations.
pub struct WalScanner {
    /// Reader
    pub reader: WalReader,
    /// Valid transactions found
    pub valid_txns: u64,
    /// Invalid transactions found
    pub invalid_txns: u64,
    /// Last valid commit sequence
    pub last_valid_commit: u64,
    /// First invalid block
    pub first_invalid: Option<WalPosition>,
}

impl WalScanner {
    /// Create new scanner
    pub fn new(start: WalPosition, journal_size: u64) -> Self {
        Self {
            reader: WalReader::new(
                start,
                WalPosition::new(journal_size - 1, 0, u64::MAX),
                journal_size,
            ),
            valid_txns: 0,
            invalid_txns: 0,
            last_valid_commit: 0,
            first_invalid: None,
        }
    }

    /// Mark valid commit found
    pub fn found_valid_commit(&mut self, sequence: u64) {
        self.valid_txns += 1;
        if sequence > self.last_valid_commit {
            self.last_valid_commit = sequence;
        }
    }

    /// Mark invalid block found
    pub fn found_invalid(&mut self, pos: WalPosition) {
        self.invalid_txns += 1;
        if self.first_invalid.is_none() {
            self.first_invalid = Some(pos);
        }
    }

    /// Get scan result
    pub fn result(&self) -> WalScanResult {
        WalScanResult {
            valid_txns: self.valid_txns,
            invalid_txns: self.invalid_txns,
            last_valid_commit: self.last_valid_commit,
            first_invalid: self.first_invalid,
            blocks_scanned: self.reader.blocks_read,
        }
    }
}

/// WAL scan result.
#[derive(Clone, Copy, Debug)]
pub struct WalScanResult {
    /// Valid transactions
    pub valid_txns: u64,
    /// Invalid transactions
    pub invalid_txns: u64,
    /// Last valid commit sequence
    pub last_valid_commit: u64,
    /// First invalid block position
    pub first_invalid: Option<WalPosition>,
    /// Blocks scanned
    pub blocks_scanned: u64,
}

impl WalScanResult {
    /// Check if journal is clean
    #[inline]
    pub fn is_clean(&self) -> bool {
        self.invalid_txns == 0
    }

    /// Check if recovery needed
    #[inline]
    pub fn needs_recovery(&self) -> bool {
        self.valid_txns > 0 || self.invalid_txns > 0
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wal_block_header() {
        let header = WalBlockHeader::new(1, 100, 500);

        assert_eq!(header.magic, WalBlockHeader::MAGIC);
        assert_eq!(header.sequence, 1);
        assert_eq!(header.txn_id, 100);
        assert!(header.validate().is_ok());
    }

    #[test]
    fn test_wal_position() {
        let mut pos = WalPosition::start();

        assert!(pos.is_valid());
        assert_eq!(pos.block, 1);

        pos.advance(10, 100);
        assert_eq!(pos.block, 11);
        assert_eq!(pos.sequence, 11);

        // Test wrap
        pos.advance(95, 100);
        assert_eq!(pos.block, 6); // Wrapped
    }

    #[test]
    fn test_wal_writer() {
        let writer = WalWriter::new(1000);

        assert!(!writer.is_running());
        writer.start();
        assert!(writer.is_running());

        let seq1 = writer.next_sequence();
        let seq2 = writer.next_sequence();
        assert_eq!(seq2, seq1 + 1);

        let txn1 = writer.alloc_txn_id();
        let txn2 = writer.alloc_txn_id();
        assert_eq!(txn2, txn1 + 1);
    }

    #[test]
    fn test_wal_reader() {
        let start = WalPosition::new(10, 0, 100);
        let end = WalPosition::new(20, 0, 110);
        let mut reader = WalReader::new(start, end, 1000);

        assert!(!reader.at_end());
        assert_eq!(reader.remaining(), 10);

        for _ in 0..10 {
            reader.advance();
        }

        assert!(reader.at_end());
        assert_eq!(reader.blocks_read, 10);
    }

    #[test]
    fn test_wal_scanner() {
        let start = WalPosition::start();
        let mut scanner = WalScanner::new(start, 1000);

        scanner.found_valid_commit(5);
        scanner.found_valid_commit(10);

        let result = scanner.result();
        assert_eq!(result.valid_txns, 2);
        assert_eq!(result.last_valid_commit, 10);
        assert!(result.is_clean());
    }
}
