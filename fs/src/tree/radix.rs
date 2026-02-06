//! Radix tree for fast integer key lookup.
//!
//! A compressed radix tree optimized for sparse integer keys.
//! Used for inode number to block mapping and other integer-keyed data.

use core::ptr::null_mut;
use core::sync::atomic::{AtomicPtr, AtomicU64, Ordering};

use crate::core::error::{HfsError, HfsResult};

// ============================================================================
// Constants
// ============================================================================

/// Radix tree radix (bits per level)
pub const RADIX_BITS: usize = 6;

/// Entries per node
pub const RADIX_SIZE: usize = 1 << RADIX_BITS; // 64

/// Maximum tree height for 64-bit keys
pub const MAX_RADIX_HEIGHT: usize = 64_usize.div_ceil(RADIX_BITS); // 11

/// Mask for extracting index at level
const RADIX_MASK: u64 = (RADIX_SIZE - 1) as u64;

// ============================================================================
// Radix Node
// ============================================================================

/// Radix tree node.
///
/// Each node has RADIX_SIZE slots, each pointing to either
/// a child node (for internal nodes) or a value (for leaves).
pub struct RadixNode {
    /// Slots (pointer-sized, can be node or value)
    slots: [AtomicU64; RADIX_SIZE],
    /// Parent node (for tree walking)
    parent: AtomicPtr<RadixNode>,
    /// Index in parent
    parent_index: u8,
    /// Height (0 = leaf level)
    height: u8,
    /// Number of non-null slots
    count: u16,
    /// Padding
    _pad: [u8; 4],
}

impl RadixNode {
    /// Create new node
    pub fn new(height: u8) -> Self {
        #[allow(clippy::declare_interior_mutable_const)]
        const ZERO: AtomicU64 = AtomicU64::new(0);
        Self {
            slots: [ZERO; RADIX_SIZE],
            parent: AtomicPtr::new(null_mut()),
            parent_index: 0,
            height,
            count: 0,
            _pad: [0; 4],
        }
    }

    /// Get slot value
    #[inline]
    pub fn get(&self, index: usize) -> u64 {
        debug_assert!(index < RADIX_SIZE);
        self.slots[index].load(Ordering::Relaxed)
    }

    /// Set slot value
    #[inline]
    pub fn set(&self, index: usize, value: u64) {
        debug_assert!(index < RADIX_SIZE);
        self.slots[index].store(value, Ordering::Relaxed);
    }

    /// Compare and swap slot
    pub fn cas(&self, index: usize, expected: u64, new: u64) -> Result<u64, u64> {
        debug_assert!(index < RADIX_SIZE);
        self.slots[index].compare_exchange(expected, new, Ordering::Relaxed, Ordering::Relaxed)
    }

    /// Check if slot is empty
    #[inline]
    pub fn is_empty_slot(&self, index: usize) -> bool {
        self.get(index) == 0
    }

    /// Count non-empty slots
    pub fn count_used(&self) -> usize {
        self.slots
            .iter()
            .filter(|s| s.load(Ordering::Relaxed) != 0)
            .count()
    }

    /// Check if node is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get height
    #[inline]
    pub fn height(&self) -> u8 {
        self.height
    }
}

// ============================================================================
// Key Operations
// ============================================================================

/// Get index at given height for key
#[inline]
pub fn key_index(key: u64, height: u8) -> usize {
    ((key >> (height as usize * RADIX_BITS)) & RADIX_MASK) as usize
}

/// Get maximum key for given height
#[inline]
pub fn max_key_for_height(height: u8) -> u64 {
    if height >= MAX_RADIX_HEIGHT as u8 {
        u64::MAX
    } else {
        (1u64 << ((height as usize + 1) * RADIX_BITS)) - 1
    }
}

/// Calculate height needed for key
pub fn height_for_key(key: u64) -> u8 {
    if key == 0 {
        return 0;
    }

    let bits = 64 - key.leading_zeros();
    (bits as usize).div_ceil(RADIX_BITS).saturating_sub(1) as u8
}

// ============================================================================
// Tagged Pointer
// ============================================================================

/// Tag bits in pointer (using low bits since nodes are aligned)
const TAG_MASK: u64 = 0b11;

/// Pointer is to internal node
const TAG_NODE: u64 = 0b00;

/// Pointer is to value
const TAG_VALUE: u64 = 0b01;

/// Pointer is marked (for concurrent operations)
const TAG_MARKED: u64 = 0b10;

/// Encode node pointer
#[inline]
pub fn encode_node(ptr: *mut RadixNode) -> u64 {
    (ptr as u64) | TAG_NODE
}

/// Encode value
#[inline]
pub fn encode_value(value: u64) -> u64 {
    // Values are stored shifted left to make room for tag
    (value << 2) | TAG_VALUE
}

/// Decode tagged pointer
#[inline]
pub fn decode(tagged: u64) -> TaggedPtr {
    let tag = tagged & TAG_MASK;
    let ptr = tagged & !TAG_MASK;

    match tag {
        TAG_NODE if ptr != 0 => TaggedPtr::Node(ptr as *mut RadixNode),
        TAG_VALUE => TaggedPtr::Value(tagged >> 2),
        TAG_MARKED => TaggedPtr::Marked(ptr),
        _ => TaggedPtr::Empty,
    }
}

/// Decoded tagged pointer
#[derive(Clone, Copy, Debug)]
pub enum TaggedPtr {
    /// Empty slot
    Empty,
    /// Internal node pointer
    Node(*mut RadixNode),
    /// Value
    Value(u64),
    /// Marked for deletion
    Marked(u64),
}

impl TaggedPtr {
    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Check if value
    #[inline]
    pub fn is_value(&self) -> bool {
        matches!(self, Self::Value(_))
    }

    /// Get value
    #[inline]
    pub fn value(&self) -> Option<u64> {
        match self {
            Self::Value(v) => Some(*v),
            _ => None,
        }
    }

    /// Get node pointer
    #[inline]
    pub fn node(&self) -> Option<*mut RadixNode> {
        match self {
            Self::Node(p) => Some(*p),
            _ => None,
        }
    }
}

// ============================================================================
// Radix Tree State
// ============================================================================

/// Radix tree state.
pub struct RadixTree {
    /// Root node pointer
    root: AtomicPtr<RadixNode>,
    /// Current height
    height: u8,
    /// Item count
    count: u64,
}

impl RadixTree {
    /// Create new empty tree
    pub const fn new() -> Self {
        Self {
            root: AtomicPtr::new(null_mut()),
            height: 0,
            count: 0,
        }
    }

    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.root.load(Ordering::Relaxed).is_null()
    }

    /// Get item count
    #[inline]
    pub fn len(&self) -> u64 {
        self.count
    }

    /// Get tree height
    #[inline]
    pub fn height(&self) -> u8 {
        self.height
    }

    /// Get root node
    #[inline]
    pub fn root(&self) -> *mut RadixNode {
        self.root.load(Ordering::Relaxed)
    }
}

impl Default for RadixTree {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Radix Tree Slot (for compact storage)
// ============================================================================

/// Compact radix tree slot for on-disk storage.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, packed)]
pub struct RadixSlot {
    /// Key prefix (bits that are common to all keys in subtree)
    pub prefix: u64,
    /// Block number or inline value
    pub value: u64,
    /// Flags
    pub flags: u8,
    /// Prefix length in bits
    pub prefix_len: u8,
    /// Reserved
    pub _reserved: [u8; 6],
}

impl RadixSlot {
    /// Size in bytes
    pub const SIZE: usize = 24;

    /// Create empty slot
    pub const fn empty() -> Self {
        Self {
            prefix: 0,
            value: 0,
            flags: 0,
            prefix_len: 0,
            _reserved: [0; 6],
        }
    }

    /// Create slot with value
    pub const fn with_value(prefix: u64, prefix_len: u8, value: u64) -> Self {
        Self {
            prefix,
            value,
            flags: 1, // Has value
            prefix_len,
            _reserved: [0; 6],
        }
    }

    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.flags == 0
    }

    /// Check if has value
    #[inline]
    pub fn has_value(&self) -> bool {
        (self.flags & 1) != 0
    }

    /// Check if is subtree
    #[inline]
    pub fn is_subtree(&self) -> bool {
        (self.flags & 2) != 0
    }
}

// ============================================================================
// Radix Node (on-disk)
// ============================================================================

/// On-disk radix node.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct RadixNodeDisk {
    /// Magic number
    pub magic: u32,
    /// Node level
    pub level: u8,
    /// Number of used slots
    pub count: u8,
    /// Flags
    pub flags: u16,
    /// Parent block
    pub parent: u64,
    /// This block number
    pub block_num: u64,
    /// Key prefix
    pub prefix: u64,
    /// Prefix length
    pub prefix_len: u8,
    /// Padding
    pub _pad: [u8; 7],
    /// Slots
    pub slots: [RadixSlot; 64],
    /// Checksum
    pub checksum: u32,
    /// Reserved
    pub _reserved: [u8; 28],
}

impl RadixNodeDisk {
    /// Size in bytes
    pub const SIZE: usize = 4096;

    /// Magic number
    pub const MAGIC: u32 = 0x52444958; // "RDIX"

    /// Create new node
    pub fn new(level: u8, block_num: u64) -> Self {
        Self {
            magic: Self::MAGIC,
            level,
            count: 0,
            flags: 0,
            parent: 0,
            block_num,
            prefix: 0,
            prefix_len: 0,
            _pad: [0; 7],
            slots: [RadixSlot::empty(); 64],
            checksum: 0,
            _reserved: [0; 28],
        }
    }

    /// Validate node
    pub fn validate(&self) -> HfsResult<()> {
        if self.magic != Self::MAGIC {
            return Err(HfsError::BtreeCorruption);
        }
        Ok(())
    }

    /// Find slot for key
    pub fn find(&self, key: u64) -> Option<u64> {
        let idx = key_index(key, self.level);
        if !self.slots[idx].is_empty() && self.slots[idx].has_value() {
            Some(self.slots[idx].value)
        } else {
            None
        }
    }

    /// Insert key-value
    pub fn insert(&mut self, key: u64, value: u64) -> HfsResult<()> {
        let idx = key_index(key, self.level);

        if self.slots[idx].is_empty() {
            self.count += 1;
        }

        self.slots[idx] = RadixSlot::with_value(
            key & !(RADIX_MASK << (self.level as usize * RADIX_BITS)),
            (self.level + 1) * RADIX_BITS as u8,
            value,
        );

        Ok(())
    }

    /// Remove key
    pub fn remove(&mut self, key: u64) -> HfsResult<bool> {
        let idx = key_index(key, self.level);

        if self.slots[idx].is_empty() {
            return Ok(false);
        }

        self.slots[idx] = RadixSlot::empty();
        self.count = self.count.saturating_sub(1);

        Ok(true)
    }
}

// Verify size
const _: () = assert!(core::mem::size_of::<RadixNodeDisk>() <= 4096);

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_index() {
        // Height 0: bits 0-5
        assert_eq!(key_index(0b000001, 0), 1);
        assert_eq!(key_index(0b111111, 0), 63);

        // Height 1: bits 6-11
        assert_eq!(key_index(0b000001_000000, 1), 1);
    }

    #[test]
    fn test_height_for_key() {
        assert_eq!(height_for_key(0), 0);
        assert_eq!(height_for_key(63), 0);
        assert_eq!(height_for_key(64), 1);
        assert_eq!(height_for_key(4095), 1);
        assert_eq!(height_for_key(4096), 2);
    }

    #[test]
    fn test_tagged_ptr() {
        // Value encoding
        let encoded = encode_value(12345);
        let decoded = decode(encoded);
        assert_eq!(decoded.value(), Some(12345));

        // Empty
        let empty = decode(0);
        assert!(empty.is_empty());
    }

    #[test]
    fn test_radix_slot() {
        let slot = RadixSlot::with_value(100, 6, 5000);

        assert!(!slot.is_empty());
        assert!(slot.has_value());
        assert_eq!(slot.value, 5000);
    }

    #[test]
    fn test_radix_node_disk() {
        let mut node = RadixNodeDisk::new(0, 100);

        assert!(node.validate().is_ok());
        assert_eq!(node.count, 0);

        node.insert(42, 1000).unwrap();
        assert_eq!(node.count, 1);

        let val = node.find(42);
        assert_eq!(val, Some(1000));

        node.remove(42).unwrap();
        assert_eq!(node.count, 0);
        assert!(node.find(42).is_none());
    }

    #[test]
    fn test_radix_tree() {
        let tree = RadixTree::new();

        assert!(tree.is_empty());
        assert_eq!(tree.len(), 0);
        assert_eq!(tree.height(), 0);
    }
}
