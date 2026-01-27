//! Journal record types and encoding.
//!
//! Defines all record types stored in the journal
//! for transaction logging and recovery.

use crate::core::types::*;
use crate::core::error::{HfsError, HfsResult};
use crate::core::hash::Crc32c;

// ============================================================================
// Record Types
// ============================================================================

/// Journal record type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum RecordType {
    /// Invalid/empty record
    Invalid = 0,
    /// Block write (full block)
    BlockWrite = 1,
    /// Block delta (partial update)
    BlockDelta = 2,
    /// Inode update
    InodeUpdate = 3,
    /// Inode create
    InodeCreate = 4,
    /// Inode delete
    InodeDelete = 5,
    /// Directory entry add
    DirAdd = 6,
    /// Directory entry remove
    DirRemove = 7,
    /// Directory entry rename
    DirRename = 8,
    /// Extent allocate
    ExtentAlloc = 9,
    /// Extent free
    ExtentFree = 10,
    /// Extent update
    ExtentUpdate = 11,
    /// Xattr set
    XattrSet = 12,
    /// Xattr remove
    XattrRemove = 13,
    /// Symlink data
    Symlink = 14,
    /// Transaction begin
    TxnBegin = 32,
    /// Transaction commit
    TxnCommit = 33,
    /// Transaction abort
    TxnAbort = 34,
    /// Checkpoint begin
    CheckpointBegin = 48,
    /// Checkpoint end
    CheckpointEnd = 49,
    /// Block revoke (for checkpoint)
    Revoke = 50,
    /// Descriptor block
    Descriptor = 51,
    /// Padding (filler)
    Padding = 255,
}

impl RecordType {
    /// From raw value
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::BlockWrite,
            2 => Self::BlockDelta,
            3 => Self::InodeUpdate,
            4 => Self::InodeCreate,
            5 => Self::InodeDelete,
            6 => Self::DirAdd,
            7 => Self::DirRemove,
            8 => Self::DirRename,
            9 => Self::ExtentAlloc,
            10 => Self::ExtentFree,
            11 => Self::ExtentUpdate,
            12 => Self::XattrSet,
            13 => Self::XattrRemove,
            14 => Self::Symlink,
            32 => Self::TxnBegin,
            33 => Self::TxnCommit,
            34 => Self::TxnAbort,
            48 => Self::CheckpointBegin,
            49 => Self::CheckpointEnd,
            50 => Self::Revoke,
            51 => Self::Descriptor,
            255 => Self::Padding,
            _ => Self::Invalid,
        }
    }
    
    /// Check if this is a data record
    #[inline]
    pub fn is_data(&self) -> bool {
        matches!(self, Self::BlockWrite | Self::BlockDelta |
                       Self::InodeUpdate | Self::InodeCreate |
                       Self::DirAdd | Self::DirRemove |
                       Self::ExtentAlloc | Self::ExtentFree)
    }
    
    /// Check if this is a control record
    #[inline]
    pub fn is_control(&self) -> bool {
        matches!(self, Self::TxnBegin | Self::TxnCommit | Self::TxnAbort |
                       Self::CheckpointBegin | Self::CheckpointEnd)
    }
    
    /// Check if this requires redo
    #[inline]
    pub fn needs_redo(&self) -> bool {
        self.is_data()
    }
    
    /// Get record category
    pub fn category(&self) -> RecordCategory {
        match self {
            Self::BlockWrite | Self::BlockDelta => RecordCategory::Block,
            Self::InodeUpdate | Self::InodeCreate | Self::InodeDelete => RecordCategory::Inode,
            Self::DirAdd | Self::DirRemove | Self::DirRename => RecordCategory::Directory,
            Self::ExtentAlloc | Self::ExtentFree | Self::ExtentUpdate => RecordCategory::Extent,
            Self::XattrSet | Self::XattrRemove => RecordCategory::Xattr,
            Self::TxnBegin | Self::TxnCommit | Self::TxnAbort => RecordCategory::Transaction,
            Self::CheckpointBegin | Self::CheckpointEnd | Self::Revoke => RecordCategory::Checkpoint,
            _ => RecordCategory::Other,
        }
    }
}

/// Record category.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordCategory {
    Block,
    Inode,
    Directory,
    Extent,
    Xattr,
    Transaction,
    Checkpoint,
    Other,
}

// ============================================================================
// Record Header
// ============================================================================

/// Common record header.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct RecordHeader {
    /// Record type
    pub record_type: u8,
    /// Flags
    pub flags: u8,
    /// Reserved
    pub _reserved: u16,
    /// Record length (including header)
    pub length: u32,
    /// Transaction ID
    pub txn_id: u64,
    /// Sequence number in transaction
    pub seq_in_txn: u32,
    /// CRC32 of record data
    pub crc: u32,
}

impl RecordHeader {
    /// Header size
    pub const SIZE: usize = 24;
    
    /// Create new header
    pub fn new(record_type: RecordType, txn_id: u64, length: u32) -> Self {
        Self {
            record_type: record_type as u8,
            flags: 0,
            _reserved: 0,
            length,
            txn_id,
            seq_in_txn: 0,
            crc: 0,
        }
    }
    
    /// Get record type
    #[inline]
    pub fn record_type(&self) -> RecordType {
        RecordType::from_raw(self.record_type)
    }
    
    /// Data length (excluding header)
    #[inline]
    pub fn data_length(&self) -> u32 {
        self.length.saturating_sub(Self::SIZE as u32)
    }
    
    /// Compute CRC of data
    pub fn compute_crc(&self, data: &[u8]) -> u32 {
        let mut hasher = Crc32c::new();
        hasher.write(data);
        hasher.finish()
    }
    
    /// Validate header
    pub fn validate(&self) -> HfsResult<()> {
        if self.record_type().is_control() || self.record_type().is_data() {
            Ok(())
        } else if self.record_type == 0 {
            Err(HfsError::InvalidRecordType)
        } else {
            Ok(()) // Unknown but not invalid
        }
    }
}

// ============================================================================
// Block Write Record
// ============================================================================

/// Block write record header.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct BlockWriteRecord {
    /// Common header
    pub header: RecordHeader,
    /// Target block number
    pub block_num: u64,
    /// Block offset (for partial writes)
    pub offset: u16,
    /// Data length
    pub data_len: u16,
    /// Block generation
    pub generation: u32,
    // Followed by block data
}

impl BlockWriteRecord {
    /// Record header size (excluding data)
    pub const HEADER_SIZE: usize = RecordHeader::SIZE + 16;
    
    /// Create new record
    pub fn new(txn_id: u64, block_num: u64, data_len: u16) -> Self {
        let total_len = Self::HEADER_SIZE as u32 + data_len as u32;
        Self {
            header: RecordHeader::new(RecordType::BlockWrite, txn_id, total_len),
            block_num,
            offset: 0,
            data_len,
            generation: 0,
        }
    }
    
    /// Create partial write record
    pub fn partial(txn_id: u64, block_num: u64, offset: u16, data_len: u16) -> Self {
        let total_len = Self::HEADER_SIZE as u32 + data_len as u32;
        Self {
            header: RecordHeader::new(RecordType::BlockDelta, txn_id, total_len),
            block_num,
            offset,
            data_len,
            generation: 0,
        }
    }
}

// ============================================================================
// Inode Update Record
// ============================================================================

/// Inode update record.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct InodeUpdateRecord {
    /// Common header
    pub header: RecordHeader,
    /// Inode number
    pub ino: u64,
    /// Update mask (which fields changed)
    pub update_mask: u32,
    /// Padding
    pub _pad: u32,
    // Followed by inode data (256 bytes for full update)
}

impl InodeUpdateRecord {
    /// Record header size
    pub const HEADER_SIZE: usize = RecordHeader::SIZE + 16;
    
    /// Update mask: mode changed
    pub const MASK_MODE: u32 = 1 << 0;
    /// Update mask: owner changed
    pub const MASK_OWNER: u32 = 1 << 1;
    /// Update mask: size changed
    pub const MASK_SIZE: u32 = 1 << 2;
    /// Update mask: times changed
    pub const MASK_TIMES: u32 = 1 << 3;
    /// Update mask: links changed
    pub const MASK_LINKS: u32 = 1 << 4;
    /// Update mask: flags changed
    pub const MASK_FLAGS: u32 = 1 << 5;
    /// Update mask: extents changed
    pub const MASK_EXTENTS: u32 = 1 << 6;
    /// Update mask: full update
    pub const MASK_FULL: u32 = 0xFFFFFFFF;
    
    /// Create new record
    pub fn new(txn_id: u64, ino: u64, update_mask: u32, data_len: u16) -> Self {
        let total_len = Self::HEADER_SIZE as u32 + data_len as u32;
        Self {
            header: RecordHeader::new(RecordType::InodeUpdate, txn_id, total_len),
            ino,
            update_mask,
            _pad: 0,
        }
    }
}

// ============================================================================
// Directory Operation Record
// ============================================================================

/// Directory entry operation record.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct DirOpRecord {
    /// Common header
    pub header: RecordHeader,
    /// Parent directory inode
    pub parent_ino: u64,
    /// Target inode
    pub target_ino: u64,
    /// Name hash
    pub name_hash: u64,
    /// Name length
    pub name_len: u8,
    /// File type
    pub file_type: u8,
    /// Reserved
    pub _reserved: [u8; 6],
    // Followed by name data
}

impl DirOpRecord {
    /// Record header size
    pub const HEADER_SIZE: usize = RecordHeader::SIZE + 32;
    
    /// Create add entry record
    pub fn add(txn_id: u64, parent: u64, target: u64, name_hash: u64, 
               name_len: u8, file_type: u8) -> Self {
        let total_len = Self::HEADER_SIZE as u32 + name_len as u32;
        Self {
            header: RecordHeader::new(RecordType::DirAdd, txn_id, total_len),
            parent_ino: parent,
            target_ino: target,
            name_hash,
            name_len,
            file_type,
            _reserved: [0; 6],
        }
    }
    
    /// Create remove entry record
    pub fn remove(txn_id: u64, parent: u64, target: u64, name_hash: u64,
                  name_len: u8) -> Self {
        let total_len = Self::HEADER_SIZE as u32 + name_len as u32;
        Self {
            header: RecordHeader::new(RecordType::DirRemove, txn_id, total_len),
            parent_ino: parent,
            target_ino: target,
            name_hash,
            name_len,
            file_type: 0,
            _reserved: [0; 6],
        }
    }
}

// ============================================================================
// Extent Operation Record
// ============================================================================

/// Extent operation record.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct ExtentOpRecord {
    /// Common header
    pub header: RecordHeader,
    /// Inode number
    pub ino: u64,
    /// Logical block offset
    pub logical: u64,
    /// Physical block
    pub physical: u64,
    /// Block count
    pub count: u32,
    /// Flags
    pub flags: u32,
}

impl ExtentOpRecord {
    /// Record size
    pub const SIZE: usize = RecordHeader::SIZE + 32;
    
    /// Create allocate record
    pub fn alloc(txn_id: u64, ino: u64, logical: u64, physical: u64, count: u32) -> Self {
        Self {
            header: RecordHeader::new(RecordType::ExtentAlloc, txn_id, Self::SIZE as u32),
            ino,
            logical,
            physical,
            count,
            flags: 0,
        }
    }
    
    /// Create free record
    pub fn free(txn_id: u64, ino: u64, logical: u64, physical: u64, count: u32) -> Self {
        Self {
            header: RecordHeader::new(RecordType::ExtentFree, txn_id, Self::SIZE as u32),
            ino,
            logical,
            physical,
            count,
            flags: 0,
        }
    }
}

// ============================================================================
// Transaction Control Records
// ============================================================================

/// Transaction begin record.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct TxnBeginRecord {
    /// Common header
    pub header: RecordHeader,
    /// Parent transaction ID (for nested)
    pub parent_txn: u64,
    /// Flags
    pub flags: u32,
    /// Reserved
    pub _reserved: u32,
}

impl TxnBeginRecord {
    /// Record size
    pub const SIZE: usize = RecordHeader::SIZE + 16;
    
    /// Create new record
    pub fn new(txn_id: u64, parent_txn: u64, flags: u32) -> Self {
        Self {
            header: RecordHeader::new(RecordType::TxnBegin, txn_id, Self::SIZE as u32),
            parent_txn,
            flags,
            _reserved: 0,
        }
    }
}

/// Transaction commit record.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct TxnCommitRecord {
    /// Common header
    pub header: RecordHeader,
    /// Commit timestamp
    pub timestamp: u64,
    /// Record count in transaction
    pub record_count: u32,
    /// Block count touched
    pub block_count: u32,
    /// Commit sequence
    pub commit_seq: u64,
    /// Checksum of all records
    pub txn_checksum: u32,
    /// Reserved
    pub _reserved: u32,
}

impl TxnCommitRecord {
    /// Record size
    pub const SIZE: usize = RecordHeader::SIZE + 32;
    
    /// Create new record
    pub fn new(txn_id: u64, timestamp: u64, record_count: u32, block_count: u32) -> Self {
        Self {
            header: RecordHeader::new(RecordType::TxnCommit, txn_id, Self::SIZE as u32),
            timestamp,
            record_count,
            block_count,
            commit_seq: 0,
            txn_checksum: 0,
            _reserved: 0,
        }
    }
}

/// Transaction abort record.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct TxnAbortRecord {
    /// Common header
    pub header: RecordHeader,
    /// Abort reason
    pub reason: u32,
    /// Reserved
    pub _reserved: u32,
}

impl TxnAbortRecord {
    /// Record size
    pub const SIZE: usize = RecordHeader::SIZE + 8;
    
    /// Create new record
    pub fn new(txn_id: u64, reason: u32) -> Self {
        Self {
            header: RecordHeader::new(RecordType::TxnAbort, txn_id, Self::SIZE as u32),
            reason,
            _reserved: 0,
        }
    }
}

// ============================================================================
// Checkpoint Records
// ============================================================================

/// Checkpoint begin record.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct CheckpointBeginRecord {
    /// Common header
    pub header: RecordHeader,
    /// Checkpoint generation
    pub generation: u64,
    /// Oldest active transaction
    pub oldest_txn: u64,
    /// Sequence of first uncommitted
    pub first_uncommitted: u64,
}

impl CheckpointBeginRecord {
    /// Record size
    pub const SIZE: usize = RecordHeader::SIZE + 24;
    
    /// Create new record
    pub fn new(txn_id: u64, generation: u64, oldest_txn: u64, first_uncommitted: u64) -> Self {
        Self {
            header: RecordHeader::new(RecordType::CheckpointBegin, txn_id, Self::SIZE as u32),
            generation,
            oldest_txn,
            first_uncommitted,
        }
    }
}

/// Checkpoint end record.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct CheckpointEndRecord {
    /// Common header
    pub header: RecordHeader,
    /// Checkpoint generation
    pub generation: u64,
    /// New tail position
    pub new_tail: u64,
    /// Blocks freed
    pub blocks_freed: u64,
}

impl CheckpointEndRecord {
    /// Record size
    pub const SIZE: usize = RecordHeader::SIZE + 24;
    
    /// Create new record
    pub fn new(txn_id: u64, generation: u64, new_tail: u64, blocks_freed: u64) -> Self {
        Self {
            header: RecordHeader::new(RecordType::CheckpointEnd, txn_id, Self::SIZE as u32),
            generation,
            new_tail,
            blocks_freed,
        }
    }
}

/// Revoke record (block no longer needs journal protection).
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct RevokeRecord {
    /// Common header
    pub header: RecordHeader,
    /// Number of revoked blocks
    pub count: u32,
    /// Reserved
    pub _reserved: u32,
    // Followed by array of u64 block numbers
}

impl RevokeRecord {
    /// Record header size
    pub const HEADER_SIZE: usize = RecordHeader::SIZE + 8;
    
    /// Create new record
    pub fn new(txn_id: u64, count: u32) -> Self {
        let total_len = Self::HEADER_SIZE as u32 + count * 8;
        Self {
            header: RecordHeader::new(RecordType::Revoke, txn_id, total_len),
            count,
            _reserved: 0,
        }
    }
    
    /// Maximum blocks per revoke record
    pub const fn max_blocks(block_size: usize) -> u32 {
        ((block_size - Self::HEADER_SIZE) / 8) as u32
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_record_type() {
        assert!(RecordType::BlockWrite.is_data());
        assert!(RecordType::TxnCommit.is_control());
        assert!(RecordType::BlockWrite.needs_redo());
        
        assert_eq!(RecordType::BlockWrite.category(), RecordCategory::Block);
        assert_eq!(RecordType::InodeCreate.category(), RecordCategory::Inode);
    }
    
    #[test]
    fn test_record_header() {
        let header = RecordHeader::new(RecordType::BlockWrite, 100, 512);
        
        assert_eq!(header.record_type(), RecordType::BlockWrite);
        assert_eq!(header.txn_id, 100);
        assert_eq!(header.data_length(), 512 - RecordHeader::SIZE as u32);
        assert!(header.validate().is_ok());
    }
    
    #[test]
    fn test_block_write_record() {
        let record = BlockWriteRecord::new(1, 100, 4096);
        
        assert_eq!(record.header.record_type(), RecordType::BlockWrite);
        assert_eq!(record.block_num, 100);
        assert_eq!(record.data_len, 4096);
    }
    
    #[test]
    fn test_inode_update_record() {
        let record = InodeUpdateRecord::new(1, 42, InodeUpdateRecord::MASK_SIZE, 256);
        
        assert_eq!(record.header.record_type(), RecordType::InodeUpdate);
        assert_eq!(record.ino, 42);
        assert!((record.update_mask & InodeUpdateRecord::MASK_SIZE) != 0);
    }
    
    #[test]
    fn test_dir_op_record() {
        let add = DirOpRecord::add(1, 2, 100, 0x12345678, 8, 1);
        let remove = DirOpRecord::remove(1, 2, 100, 0x12345678, 8);
        
        assert_eq!(add.header.record_type(), RecordType::DirAdd);
        assert_eq!(remove.header.record_type(), RecordType::DirRemove);
    }
    
    #[test]
    fn test_extent_op_record() {
        let alloc = ExtentOpRecord::alloc(1, 42, 0, 1000, 10);
        let free = ExtentOpRecord::free(1, 42, 0, 1000, 10);
        
        assert_eq!(alloc.header.record_type(), RecordType::ExtentAlloc);
        assert_eq!(free.header.record_type(), RecordType::ExtentFree);
    }
    
    #[test]
    fn test_txn_records() {
        let begin = TxnBeginRecord::new(1, 0, 0);
        let commit = TxnCommitRecord::new(1, 12345, 10, 5);
        let abort = TxnAbortRecord::new(1, 1);
        
        assert_eq!(begin.header.record_type(), RecordType::TxnBegin);
        assert_eq!(commit.header.record_type(), RecordType::TxnCommit);
        assert_eq!(abort.header.record_type(), RecordType::TxnAbort);
    }
    
    #[test]
    fn test_checkpoint_records() {
        let begin = CheckpointBeginRecord::new(1, 5, 10, 100);
        let end = CheckpointEndRecord::new(1, 5, 200, 50);
        
        assert_eq!(begin.header.record_type(), RecordType::CheckpointBegin);
        assert_eq!(end.header.record_type(), RecordType::CheckpointEnd);
    }
}
