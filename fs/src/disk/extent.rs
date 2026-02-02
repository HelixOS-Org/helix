//! Extent tree structures for efficient block mapping.
//!
//! HelixFS uses a B+tree-like extent tree to map logical file offsets to
//! physical disk blocks. This allows efficient handling of large files
//! and enables features like preallocation and sparse files.

use core::cmp::Ordering;
use core::mem::size_of;

use crate::core::error::{HfsError, HfsResult};
use crate::core::hash::Crc32c;
use crate::core::types::*;

// ============================================================================
// Constants
// ============================================================================

/// Extent tree node size (4096 bytes = 1 block)
pub const EXTENT_NODE_SIZE: usize = 4096;

/// Magic number for extent tree header
pub const EXTENT_TREE_MAGIC: u32 = 0x48455854; // "HEXT"

/// Maximum depth of extent tree
pub const MAX_EXTENT_DEPTH: u8 = 5;

/// Maximum extents in a leaf node
pub const MAX_LEAF_EXTENTS: usize = (EXTENT_NODE_SIZE - 64) / size_of::<ExtentEntry>();

/// Maximum keys in an internal node
pub const MAX_INTERNAL_KEYS: usize = (EXTENT_NODE_SIZE - 64) / size_of::<ExtentPtr>();

// ============================================================================
// Extent Entry (Leaf Node)
// ============================================================================

/// Entry in an extent leaf node.
///
/// Maps a range of logical blocks to physical blocks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C, packed)]
pub struct ExtentEntry {
    /// Starting logical block number
    pub logical_start: u64,
    /// Starting physical block number
    pub physical_start: u64,
    /// Length in blocks
    pub length: u32,
    /// Flags (compressed, encrypted, etc.)
    pub flags: u16,
    /// Compression ratio (if compressed, stored as 0.XXX * 1000)
    pub compress_ratio: u16,
    /// Generation/transaction that created this extent
    pub generation: u64,
    /// Checksum of the extent's data (optional, 0 if not used)
    pub data_checksum: u32,
    /// Reserved for alignment
    pub _reserved: u32,
}

impl ExtentEntry {
    /// Size in bytes
    pub const SIZE: usize = 40;

    /// Create a new extent entry
    #[inline]
    pub const fn new(logical_start: u64, physical_start: u64, length: u32) -> Self {
        Self {
            logical_start,
            physical_start,
            length,
            flags: 0,
            compress_ratio: 0,
            generation: 0,
            data_checksum: 0,
            _reserved: 0,
        }
    }

    /// Create with flags
    pub const fn with_flags(mut self, flags: u16) -> Self {
        self.flags = flags;
        self
    }

    /// Create with generation
    pub const fn with_generation(mut self, gen: u64) -> Self {
        self.generation = gen;
        self
    }

    /// Get ending logical block (exclusive)
    #[inline]
    pub fn logical_end(&self) -> u64 {
        self.logical_start + self.length as u64
    }

    /// Get ending physical block (exclusive)
    #[inline]
    pub fn physical_end(&self) -> u64 {
        self.physical_start + self.length as u64
    }

    /// Check if this extent contains a logical block
    #[inline]
    pub fn contains_logical(&self, block: u64) -> bool {
        block >= self.logical_start && block < self.logical_end()
    }

    /// Check if this extent contains a physical block
    #[inline]
    pub fn contains_physical(&self, block: u64) -> bool {
        block >= self.physical_start && block < self.physical_end()
    }

    /// Map logical block to physical block
    #[inline]
    pub fn map_logical(&self, logical: u64) -> Option<u64> {
        if self.contains_logical(logical) {
            Some(self.physical_start + (logical - self.logical_start))
        } else {
            None
        }
    }

    /// Check if compressed
    #[inline]
    pub fn is_compressed(&self) -> bool {
        (self.flags & ExtentFlags::COMPRESSED) != 0
    }

    /// Check if encrypted
    #[inline]
    pub fn is_encrypted(&self) -> bool {
        (self.flags & ExtentFlags::ENCRYPTED) != 0
    }

    /// Check if this is a hole (unwritten extent)
    #[inline]
    pub fn is_hole(&self) -> bool {
        (self.flags & ExtentFlags::HOLE) != 0
    }

    /// Check if preallocated but unwritten
    #[inline]
    pub fn is_prealloc(&self) -> bool {
        (self.flags & ExtentFlags::PREALLOC) != 0
    }

    /// Check if shared (CoW reference)
    #[inline]
    pub fn is_shared(&self) -> bool {
        (self.flags & ExtentFlags::SHARED) != 0
    }

    /// Convert to core Extent type
    pub fn to_extent(&self) -> Extent {
        Extent {
            logical_start: self.logical_start,
            physical_start: self.physical_start,
            block_count: self.length,
            flags: self.flags,
            checksum: 0,
        }
    }

    /// Create from core Extent type
    pub fn from_extent(e: &Extent, generation: u64) -> Self {
        Self {
            logical_start: e.logical_start,
            physical_start: e.physical_start,
            length: e.block_count,
            flags: e.flags,
            compress_ratio: 0,
            generation,
            data_checksum: 0,
            _reserved: 0,
        }
    }

    /// Compare by logical start for sorting
    pub fn cmp_logical(&self, other: &Self) -> Ordering {
        // Copy packed fields to avoid unaligned reference
        let a = self.logical_start;
        let b = other.logical_start;
        a.cmp(&b)
    }
}

// Verify size
const _: () = assert!(size_of::<ExtentEntry>() == ExtentEntry::SIZE);

// ============================================================================
// Extent Pointer (Internal Node)
// ============================================================================

/// Pointer entry in an internal node.
///
/// Points to a child node that contains extents starting at or after
/// the given logical block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C, packed)]
pub struct ExtentPtr {
    /// Minimum logical block in the subtree
    pub logical_start: u64,
    /// Block number of child node
    pub child_block: u64,
    /// Generation when this pointer was created
    pub generation: u64,
}

impl ExtentPtr {
    /// Size in bytes
    pub const SIZE: usize = 24;

    /// Create new extent pointer
    #[inline]
    pub const fn new(logical_start: u64, child_block: u64, generation: u64) -> Self {
        Self {
            logical_start,
            child_block,
            generation,
        }
    }
}

// Verify size
const _: () = assert!(size_of::<ExtentPtr>() == ExtentPtr::SIZE);

// ============================================================================
// Extent Node Header
// ============================================================================

/// Extent tree node type
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum ExtentNodeType {
    /// Internal node with pointers
    Internal = 1,
    /// Leaf node with extent entries
    Leaf     = 2,
}

impl ExtentNodeType {
    /// Convert from byte
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::Internal),
            2 => Some(Self::Leaf),
            _ => None,
        }
    }
}

/// Header for extent tree nodes.
#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct ExtentNodeHeader {
    /// Magic number
    pub magic: u32,
    /// Node type (internal or leaf)
    pub node_type: u8,
    /// Depth in tree (0 = leaf)
    pub depth: u8,
    /// Number of entries in this node
    pub entry_count: u16,
    /// Inode this extent tree belongs to
    pub owner_ino: u64,
    /// Generation of this node
    pub generation: u64,
    /// Block number of this node
    pub block_num: u64,
    /// Block number of previous sibling (for leaf scanning)
    pub prev_block: u64,
    /// Block number of next sibling (for leaf scanning)
    pub next_block: u64,
    /// Checksum of this node
    pub checksum: u32,
    /// Padding
    pub _padding: [u8; 12],
}

impl ExtentNodeHeader {
    /// Size in bytes
    pub const SIZE: usize = 64;

    /// Create new header
    pub const fn new(node_type: ExtentNodeType, depth: u8, owner_ino: u64) -> Self {
        Self {
            magic: EXTENT_TREE_MAGIC,
            node_type: node_type as u8,
            depth,
            entry_count: 0,
            owner_ino,
            generation: 0,
            block_num: 0,
            prev_block: 0,
            next_block: 0,
            checksum: 0,
            _padding: [0; 12],
        }
    }

    /// Validate header
    pub fn validate(&self) -> HfsResult<()> {
        if self.magic != EXTENT_TREE_MAGIC {
            return Err(HfsError::ExtentCorruption);
        }
        if ExtentNodeType::from_u8(self.node_type).is_none() {
            return Err(HfsError::ExtentCorruption);
        }
        if self.depth > MAX_EXTENT_DEPTH {
            return Err(HfsError::ExtentCorruption);
        }
        Ok(())
    }

    /// Check if leaf node
    #[inline]
    pub fn is_leaf(&self) -> bool {
        self.node_type == ExtentNodeType::Leaf as u8
    }

    /// Check if internal node
    #[inline]
    pub fn is_internal(&self) -> bool {
        self.node_type == ExtentNodeType::Internal as u8
    }
}

// Verify size
const _: () = assert!(size_of::<ExtentNodeHeader>() == ExtentNodeHeader::SIZE);

// ============================================================================
// Extent Leaf Node
// ============================================================================

/// Leaf node in the extent tree.
///
/// Contains actual extent entries mapping logical to physical blocks.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct ExtentLeafNode {
    /// Node header
    pub header: ExtentNodeHeader,
    /// Extent entries
    pub entries: [ExtentEntry; MAX_LEAF_EXTENTS],
}

impl ExtentLeafNode {
    /// Create new leaf node
    pub fn new(owner_ino: u64, block_num: u64) -> Self {
        let mut header = ExtentNodeHeader::new(ExtentNodeType::Leaf, 0, owner_ino);
        header.block_num = block_num;

        Self {
            header,
            entries: [ExtentEntry::new(0, 0, 0); MAX_LEAF_EXTENTS],
        }
    }

    /// Get entry count
    #[inline]
    pub fn count(&self) -> usize {
        self.header.entry_count as usize
    }

    /// Check if full
    #[inline]
    pub fn is_full(&self) -> bool {
        self.count() >= MAX_LEAF_EXTENTS
    }

    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// Get entries as slice
    pub fn entries(&self) -> &[ExtentEntry] {
        &self.entries[..self.count()]
    }

    /// Find entry containing logical block
    pub fn find_extent(&self, logical_block: u64) -> Option<&ExtentEntry> {
        // Binary search
        let entries = self.entries();
        let mut left = 0;
        let mut right = entries.len();

        while left < right {
            let mid = left + (right - left) / 2;
            let entry = &entries[mid];

            if entry.contains_logical(logical_block) {
                return Some(entry);
            } else if entry.logical_start > logical_block {
                right = mid;
            } else {
                left = mid + 1;
            }
        }

        None
    }

    /// Insert extent entry (maintains sorted order)
    pub fn insert(&mut self, entry: ExtentEntry) -> HfsResult<()> {
        if self.is_full() {
            return Err(HfsError::ExtentTreeFull);
        }

        let count = self.count();

        // Find insertion point
        let mut pos = count;
        for i in 0..count {
            if self.entries[i].logical_start > entry.logical_start {
                pos = i;
                break;
            }
        }

        // Shift entries
        for i in (pos..count).rev() {
            self.entries[i + 1] = self.entries[i];
        }

        // Insert
        self.entries[pos] = entry;
        self.header.entry_count += 1;

        Ok(())
    }

    /// Remove extent at index
    pub fn remove(&mut self, index: usize) -> HfsResult<ExtentEntry> {
        let count = self.count();
        if index >= count {
            return Err(HfsError::ExtentNotFound);
        }

        let entry = self.entries[index];

        // Shift entries
        for i in index..count - 1 {
            self.entries[i] = self.entries[i + 1];
        }

        self.header.entry_count -= 1;
        Ok(entry)
    }

    /// Calculate checksum
    pub fn calculate_checksum(&self) -> u32 {
        // Hash header without checksum field
        let bytes = unsafe {
            core::slice::from_raw_parts(self as *const Self as *const u8, EXTENT_NODE_SIZE - 4)
        };
        Crc32c::hash(bytes)
    }

    /// Update checksum
    pub fn update_checksum(&mut self) {
        self.header.checksum = self.calculate_checksum();
    }

    /// Validate node
    pub fn validate(&self) -> HfsResult<()> {
        self.header.validate()?;

        if !self.header.is_leaf() {
            return Err(HfsError::ExtentCorruption);
        }

        // Verify entries are sorted
        let entries = self.entries();
        for i in 1..entries.len() {
            if entries[i].logical_start <= entries[i - 1].logical_end() {
                return Err(HfsError::ExtentCorruption);
            }
        }

        Ok(())
    }

    /// Split node in half, return new node
    pub fn split(&mut self, new_block_num: u64) -> Self {
        let count = self.count();
        let mid = count / 2;

        let mut new_node = Self::new(self.header.owner_ino, new_block_num);
        new_node.header.generation = self.header.generation;

        // Copy second half to new node
        for i in mid..count {
            new_node.entries[i - mid] = self.entries[i];
        }
        new_node.header.entry_count = (count - mid) as u16;

        // Update this node
        self.header.entry_count = mid as u16;

        // Update sibling links
        new_node.header.prev_block = self.header.block_num;
        new_node.header.next_block = self.header.next_block;
        self.header.next_block = new_block_num;

        new_node
    }

    /// Get first key (minimum logical block)
    #[inline]
    pub fn first_key(&self) -> u64 {
        if self.is_empty() {
            u64::MAX
        } else {
            self.entries[0].logical_start
        }
    }

    /// Get last key (maximum logical block start)
    #[inline]
    pub fn last_key(&self) -> u64 {
        let count = self.count();
        if count == 0 {
            0
        } else {
            self.entries[count - 1].logical_start
        }
    }
}

// ============================================================================
// Extent Internal Node
// ============================================================================

/// Internal node in the extent tree.
///
/// Contains pointers to child nodes (internal or leaf).
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct ExtentInternalNode {
    /// Node header
    pub header: ExtentNodeHeader,
    /// Child pointers
    pub pointers: [ExtentPtr; MAX_INTERNAL_KEYS],
}

impl ExtentInternalNode {
    /// Create new internal node
    pub fn new(owner_ino: u64, depth: u8, block_num: u64) -> Self {
        let mut header = ExtentNodeHeader::new(ExtentNodeType::Internal, depth, owner_ino);
        header.block_num = block_num;

        Self {
            header,
            pointers: [ExtentPtr::new(0, 0, 0); MAX_INTERNAL_KEYS],
        }
    }

    /// Get entry count
    #[inline]
    pub fn count(&self) -> usize {
        self.header.entry_count as usize
    }

    /// Check if full
    #[inline]
    pub fn is_full(&self) -> bool {
        self.count() >= MAX_INTERNAL_KEYS
    }

    /// Get pointers as slice
    pub fn pointers(&self) -> &[ExtentPtr] {
        &self.pointers[..self.count()]
    }

    /// Find child for logical block
    pub fn find_child(&self, logical_block: u64) -> Option<u64> {
        let ptrs = self.pointers();
        if ptrs.is_empty() {
            return None;
        }

        // Binary search for the appropriate child
        let mut left = 0;
        let mut right = ptrs.len();

        while left < right {
            let mid = left + (right - left) / 2;
            if ptrs[mid].logical_start <= logical_block {
                left = mid + 1;
            } else {
                right = mid;
            }
        }

        // left is now the first pointer with logical_start > logical_block
        // We want the previous one
        if left == 0 {
            Some(ptrs[0].child_block)
        } else {
            Some(ptrs[left - 1].child_block)
        }
    }

    /// Insert pointer (maintains sorted order)
    pub fn insert(&mut self, ptr: ExtentPtr) -> HfsResult<()> {
        if self.is_full() {
            return Err(HfsError::ExtentTreeFull);
        }

        let count = self.count();

        // Find insertion point
        let mut pos = count;
        for i in 0..count {
            if self.pointers[i].logical_start > ptr.logical_start {
                pos = i;
                break;
            }
        }

        // Shift pointers
        for i in (pos..count).rev() {
            self.pointers[i + 1] = self.pointers[i];
        }

        // Insert
        self.pointers[pos] = ptr;
        self.header.entry_count += 1;

        Ok(())
    }

    /// Remove pointer at index
    pub fn remove(&mut self, index: usize) -> HfsResult<ExtentPtr> {
        let count = self.count();
        if index >= count {
            return Err(HfsError::ExtentNotFound);
        }

        let ptr = self.pointers[index];

        // Shift pointers
        for i in index..count - 1 {
            self.pointers[i] = self.pointers[i + 1];
        }

        self.header.entry_count -= 1;
        Ok(ptr)
    }

    /// Update key for child at index
    pub fn update_key(&mut self, index: usize, new_key: u64) {
        if index < self.count() {
            self.pointers[index].logical_start = new_key;
        }
    }

    /// Calculate checksum
    pub fn calculate_checksum(&self) -> u32 {
        let bytes = unsafe {
            core::slice::from_raw_parts(self as *const Self as *const u8, EXTENT_NODE_SIZE - 4)
        };
        Crc32c::hash(bytes)
    }

    /// Update checksum
    pub fn update_checksum(&mut self) {
        self.header.checksum = self.calculate_checksum();
    }

    /// Validate node
    pub fn validate(&self) -> HfsResult<()> {
        self.header.validate()?;

        if !self.header.is_internal() {
            return Err(HfsError::ExtentCorruption);
        }

        // Verify pointers are sorted
        let ptrs = self.pointers();
        for i in 1..ptrs.len() {
            if ptrs[i].logical_start <= ptrs[i - 1].logical_start {
                return Err(HfsError::ExtentCorruption);
            }
        }

        Ok(())
    }

    /// Split node in half
    pub fn split(&mut self, new_block_num: u64) -> Self {
        let count = self.count();
        let mid = count / 2;

        let mut new_node = Self::new(self.header.owner_ino, self.header.depth, new_block_num);
        new_node.header.generation = self.header.generation;

        // Copy second half to new node
        for i in mid..count {
            new_node.pointers[i - mid] = self.pointers[i];
        }
        new_node.header.entry_count = (count - mid) as u16;

        // Update this node
        self.header.entry_count = mid as u16;

        new_node
    }

    /// Get first key
    #[inline]
    pub fn first_key(&self) -> u64 {
        if self.count() == 0 {
            u64::MAX
        } else {
            self.pointers[0].logical_start
        }
    }
}

// ============================================================================
// Extent Tree Root (stored in inode or separate block)
// ============================================================================

/// Extent tree root descriptor.
///
/// Small extent trees can fit inline in the inode. Larger trees
/// use an external root node.
#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct ExtentTreeRoot {
    /// Root block number (0 if inline in inode)
    pub root_block: u64,
    /// Tree depth (0 = all extents inline, 1 = single leaf, etc.)
    pub depth: u8,
    /// Number of inline extents (if depth == 0)
    pub inline_count: u8,
    /// Reserved
    pub _reserved: [u8; 6],
    /// Inline extents for small files (up to 4)
    pub inline_extents: [ExtentEntry; 4],
    /// Total extent count in tree
    pub total_extents: u64,
    /// Total blocks mapped by tree
    pub total_blocks: u64,
}

impl ExtentTreeRoot {
    /// Size in bytes
    pub const SIZE: usize = 192;

    /// Create new empty root
    pub const fn new() -> Self {
        Self {
            root_block: 0,
            depth: 0,
            inline_count: 0,
            _reserved: [0; 6],
            inline_extents: [ExtentEntry::new(0, 0, 0); 4],
            total_extents: 0,
            total_blocks: 0,
        }
    }

    /// Check if tree is inline (no external nodes)
    #[inline]
    pub fn is_inline(&self) -> bool {
        self.depth == 0 && self.root_block == 0
    }

    /// Get inline extents if present
    pub fn inline_extents(&self) -> Option<&[ExtentEntry]> {
        if self.is_inline() {
            Some(&self.inline_extents[..self.inline_count as usize])
        } else {
            None
        }
    }

    /// Check if can fit more inline extents
    #[inline]
    pub fn can_inline(&self) -> bool {
        self.is_inline() && (self.inline_count as usize) < 4
    }

    /// Add inline extent
    pub fn add_inline(&mut self, extent: ExtentEntry) -> HfsResult<()> {
        if !self.can_inline() {
            return Err(HfsError::ExtentTreeFull);
        }

        let idx = self.inline_count as usize;
        self.inline_extents[idx] = extent;
        self.inline_count += 1;
        self.total_extents += 1;
        self.total_blocks += extent.length as u64;

        Ok(())
    }

    /// Find extent for logical block in inline extents
    pub fn find_inline(&self, logical_block: u64) -> Option<&ExtentEntry> {
        if let Some(extents) = self.inline_extents() {
            for e in extents {
                if e.contains_logical(logical_block) {
                    return Some(e);
                }
            }
        }
        None
    }
}

// ============================================================================
// Extent Allocation Hints
// ============================================================================

/// Hints for extent allocation.
#[derive(Clone, Copy, Debug, Default)]
pub struct ExtentAllocHint {
    /// Preferred starting block (for locality)
    pub preferred_start: Option<BlockNum>,
    /// Desired length in blocks
    pub desired_length: u32,
    /// Minimum acceptable length
    pub min_length: u32,
    /// Allocate near inode's existing extents
    pub near_inode: Option<InodeNum>,
    /// Allocate in specific allocation group
    pub alloc_group: Option<u32>,
    /// Flags (prealloc, zeroed, etc.)
    pub flags: u16,
}

impl ExtentAllocHint {
    /// Create new hint
    pub const fn new(desired_length: u32) -> Self {
        Self {
            preferred_start: None,
            desired_length,
            min_length: 1,
            near_inode: None,
            alloc_group: None,
            flags: 0,
        }
    }

    /// Set preferred start
    pub const fn with_start(mut self, block: BlockNum) -> Self {
        self.preferred_start = Some(block);
        self
    }

    /// Set minimum length
    pub const fn with_min_length(mut self, len: u32) -> Self {
        self.min_length = len;
        self
    }
}

// ============================================================================
// Extent Map (for tracking file layout)
// ============================================================================

/// Result of extent lookup
#[derive(Clone, Copy, Debug)]
pub struct ExtentMapResult {
    /// Physical block (or 0 for hole)
    pub physical: BlockNum,
    /// Number of contiguous blocks
    pub length: u32,
    /// Is this a hole (sparse region)?
    pub is_hole: bool,
    /// Is extent shared (CoW)?
    pub is_shared: bool,
    /// Is extent compressed?
    pub is_compressed: bool,
}

impl ExtentMapResult {
    /// Create result for mapped extent
    pub fn mapped(physical: BlockNum, length: u32) -> Self {
        Self {
            physical,
            length,
            is_hole: false,
            is_shared: false,
            is_compressed: false,
        }
    }

    /// Create result for hole
    pub fn hole(length: u32) -> Self {
        Self {
            physical: BlockNum::new(0),
            length,
            is_hole: true,
            is_shared: false,
            is_compressed: false,
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
    fn test_extent_entry_size() {
        assert_eq!(size_of::<ExtentEntry>(), 40);
    }

    #[test]
    fn test_extent_ptr_size() {
        assert_eq!(size_of::<ExtentPtr>(), 24);
    }

    #[test]
    fn test_extent_header_size() {
        assert_eq!(size_of::<ExtentNodeHeader>(), 64);
    }

    #[test]
    fn test_extent_entry_mapping() {
        let entry = ExtentEntry::new(100, 5000, 50);

        assert!(entry.contains_logical(100));
        assert!(entry.contains_logical(149));
        assert!(!entry.contains_logical(150));

        assert_eq!(entry.map_logical(100), Some(5000));
        assert_eq!(entry.map_logical(125), Some(5025));
        assert_eq!(entry.map_logical(200), None);
    }

    #[test]
    fn test_leaf_node_insert() {
        let mut leaf = ExtentLeafNode::new(1, 100);

        assert!(leaf.insert(ExtentEntry::new(100, 1000, 10)).is_ok());
        assert!(leaf.insert(ExtentEntry::new(50, 500, 20)).is_ok());
        assert!(leaf.insert(ExtentEntry::new(200, 2000, 30)).is_ok());

        assert_eq!(leaf.count(), 3);

        // Should be sorted
        let entries = leaf.entries();
        assert_eq!(entries[0].logical_start, 50);
        assert_eq!(entries[1].logical_start, 100);
        assert_eq!(entries[2].logical_start, 200);
    }

    #[test]
    fn test_leaf_node_find() {
        let mut leaf = ExtentLeafNode::new(1, 100);
        leaf.insert(ExtentEntry::new(0, 1000, 100)).unwrap();
        leaf.insert(ExtentEntry::new(200, 2000, 100)).unwrap();
        leaf.insert(ExtentEntry::new(400, 3000, 100)).unwrap();

        assert!(leaf.find_extent(50).is_some());
        assert!(leaf.find_extent(99).is_some());
        assert!(leaf.find_extent(100).is_none()); // Gap
        assert!(leaf.find_extent(200).is_some());
        assert!(leaf.find_extent(500).is_none());
    }

    #[test]
    fn test_internal_node_find_child() {
        let mut internal = ExtentInternalNode::new(1, 1, 100);
        internal.insert(ExtentPtr::new(0, 1000, 0)).unwrap();
        internal.insert(ExtentPtr::new(100, 1001, 0)).unwrap();
        internal.insert(ExtentPtr::new(200, 1002, 0)).unwrap();

        assert_eq!(internal.find_child(0), Some(1000));
        assert_eq!(internal.find_child(50), Some(1000));
        assert_eq!(internal.find_child(100), Some(1001));
        assert_eq!(internal.find_child(150), Some(1001));
        assert_eq!(internal.find_child(300), Some(1002));
    }

    #[test]
    fn test_extent_tree_root() {
        let mut root = ExtentTreeRoot::new();

        assert!(root.is_inline());
        assert!(root.can_inline());

        root.add_inline(ExtentEntry::new(0, 100, 50)).unwrap();
        root.add_inline(ExtentEntry::new(50, 200, 50)).unwrap();

        assert_eq!(root.total_extents, 2);
        assert_eq!(root.total_blocks, 100);

        assert!(root.find_inline(0).is_some());
        assert!(root.find_inline(75).is_some());
        assert!(root.find_inline(100).is_none());
    }
}
