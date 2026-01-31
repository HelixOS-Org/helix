//! B+tree node implementation.
//!
//! Provides internal and leaf node structures for the B+tree.

use crate::core::error::{HfsError, HfsResult};
use crate::core::hash::Crc32c;
use super::{TreeType, SearchResult};
use core::mem::size_of;

// ============================================================================
// Constants
// ============================================================================

/// Node size (one block)
pub const NODE_SIZE: usize = 4096;

/// Node header size
pub const NODE_HEADER_SIZE: usize = 64;

/// Maximum data area size
pub const NODE_DATA_SIZE: usize = NODE_SIZE - NODE_HEADER_SIZE;

/// Node magic number
pub const NODE_MAGIC: u32 = 0x484E4F44; // "HNOD"

/// Node type
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum NodeType {
    /// Internal node (keys + child pointers)
    Internal = 1,
    /// Leaf node (keys + values)
    Leaf = 2,
    /// Overflow node (for large values)
    Overflow = 3,
}

impl NodeType {
    /// From byte
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::Internal),
            2 => Some(Self::Leaf),
            3 => Some(Self::Overflow),
            _ => None,
        }
    }
}

// ============================================================================
// Node Header
// ============================================================================

/// On-disk node header.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct NodeHeader {
    /// Magic number
    pub magic: u32,
    /// Node type
    pub node_type: u8,
    /// Level (0 = leaf)
    pub level: u8,
    /// Number of items
    pub item_count: u16,
    /// Tree type
    pub tree_type: u8,
    /// Flags
    pub flags: u8,
    /// Free space in data area
    pub free_space: u16,
    /// Owner inode (for extent trees)
    pub owner: u64,
    /// This node's block number
    pub block_num: u64,
    /// Parent node block (0 if root)
    pub parent_block: u64,
    /// Left sibling block
    pub left_sibling: u64,
    /// Right sibling block
    pub right_sibling: u64,
    /// Generation
    pub generation: u64,
    /// Checksum
    pub checksum: u32,
}

impl NodeHeader {
    /// Create new header
    pub fn new(node_type: NodeType, level: u8, tree_type: TreeType) -> Self {
        Self {
            magic: NODE_MAGIC,
            node_type: node_type as u8,
            level,
            item_count: 0,
            tree_type: tree_type as u8,
            flags: 0,
            free_space: NODE_DATA_SIZE as u16,
            owner: 0,
            block_num: 0,
            parent_block: 0,
            left_sibling: 0,
            right_sibling: 0,
            generation: 0,
            checksum: 0,
        }
    }
    
    /// Check if leaf node
    #[inline]
    pub fn is_leaf(&self) -> bool {
        self.node_type == NodeType::Leaf as u8
    }
    
    /// Check if internal node
    #[inline]
    pub fn is_internal(&self) -> bool {
        self.node_type == NodeType::Internal as u8
    }
    
    /// Check if root
    #[inline]
    pub fn is_root(&self) -> bool {
        self.parent_block == 0
    }
    
    /// Validate header
    pub fn validate(&self) -> HfsResult<()> {
        if self.magic != NODE_MAGIC {
            return Err(HfsError::BtreeCorruption);
        }
        if NodeType::from_u8(self.node_type).is_none() {
            return Err(HfsError::BtreeCorruption);
        }
        Ok(())
    }
}

// Verify size
const _: () = assert!(size_of::<NodeHeader>() == NODE_HEADER_SIZE);

// ============================================================================
// Item Pointer (for variable-size items)
// ============================================================================

/// Points to an item within the node's data area.
#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct ItemPtr {
    /// Offset from start of data area
    pub offset: u16,
    /// Size of item
    pub size: u16,
}

impl ItemPtr {
    /// Size in bytes
    pub const SIZE: usize = 4;
    
    /// Create new pointer
    #[inline]
    pub const fn new(offset: u16, size: u16) -> Self {
        Self { offset, size }
    }
    
    /// Check if valid
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.size > 0 && (self.offset as usize + self.size as usize) <= NODE_DATA_SIZE
    }
}

// ============================================================================
// Fixed-Key Internal Node Entry
// ============================================================================

/// Entry in internal node (for u64 keys).
#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct InternalEntry64 {
    /// Key
    pub key: u64,
    /// Child block number
    pub child: u64,
}

impl InternalEntry64 {
    /// Size in bytes
    pub const SIZE: usize = 16;
    
    /// Create new entry
    #[inline]
    pub const fn new(key: u64, child: u64) -> Self {
        Self { key, child }
    }
}

/// Maximum internal entries per node (for u64 keys)
pub const MAX_INTERNAL_ENTRIES_64: usize = NODE_DATA_SIZE / InternalEntry64::SIZE;

// ============================================================================
// Fixed-Key Leaf Node Entry
// ============================================================================

/// Entry in leaf node (for u64 keys with fixed-size value).
#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct LeafEntry64<V: Copy> {
    /// Key
    pub key: u64,
    /// Value
    pub value: V,
}

impl<V: Copy> LeafEntry64<V> {
    /// Size in bytes
    pub const fn size() -> usize {
        8 + size_of::<V>()
    }
}

// ============================================================================
// Internal Node (u64 keys)
// ============================================================================

/// Internal node with u64 keys.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct InternalNode64 {
    /// Header
    pub header: NodeHeader,
    /// Entries
    pub entries: [InternalEntry64; MAX_INTERNAL_ENTRIES_64],
}

impl InternalNode64 {
    /// Create new internal node
    pub fn new(level: u8, tree_type: TreeType, block_num: u64) -> Self {
        let mut header = NodeHeader::new(NodeType::Internal, level, tree_type);
        header.block_num = block_num;
        
        Self {
            header,
            entries: [InternalEntry64::new(0, 0); MAX_INTERNAL_ENTRIES_64],
        }
    }
    
    /// Get entry count
    #[inline]
    pub fn count(&self) -> usize {
        self.header.item_count as usize
    }
    
    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }
    
    /// Check if full
    #[inline]
    pub fn is_full(&self) -> bool {
        self.count() >= MAX_INTERNAL_ENTRIES_64
    }
    
    /// Get entries slice
    pub fn entries(&self) -> &[InternalEntry64] {
        &self.entries[..self.count()]
    }
    
    /// Binary search for key
    pub fn search(&self, key: u64) -> SearchResult {
        let entries = self.entries();
        let mut left = 0;
        let mut right = entries.len();
        
        while left < right {
            let mid = left + (right - left) / 2;
            if entries[mid].key == key {
                return SearchResult::Found(mid);
            } else if entries[mid].key < key {
                left = mid + 1;
            } else {
                right = mid;
            }
        }
        
        SearchResult::NotFound(left)
    }
    
    /// Find child for key
    pub fn find_child(&self, key: u64) -> Option<u64> {
        let entries = self.entries();
        if entries.is_empty() {
            return None;
        }
        
        match self.search(key) {
            SearchResult::Found(i) => Some(entries[i].child),
            SearchResult::NotFound(i) => {
                if i == 0 {
                    Some(entries[0].child)
                } else {
                    Some(entries[i - 1].child)
                }
            }
        }
    }
    
    /// Insert entry (must have space)
    pub fn insert(&mut self, key: u64, child: u64) -> HfsResult<()> {
        if self.is_full() {
            return Err(HfsError::BtreeFull);
        }
        
        let pos = match self.search(key) {
            SearchResult::Found(i) => i,
            SearchResult::NotFound(i) => i,
        };
        
        let count = self.count();
        
        // Shift entries
        for i in (pos..count).rev() {
            self.entries[i + 1] = self.entries[i];
        }
        
        // Insert
        self.entries[pos] = InternalEntry64::new(key, child);
        self.header.item_count += 1;
        
        Ok(())
    }
    
    /// Remove entry at index
    pub fn remove(&mut self, index: usize) -> HfsResult<InternalEntry64> {
        let count = self.count();
        if index >= count {
            return Err(HfsError::BtreeKeyNotFound);
        }
        
        let entry = self.entries[index];
        
        // Shift entries
        for i in index..count - 1 {
            self.entries[i] = self.entries[i + 1];
        }
        
        self.header.item_count -= 1;
        Ok(entry)
    }
    
    /// Get first key
    pub fn first_key(&self) -> Option<u64> {
        if self.is_empty() {
            None
        } else {
            Some(self.entries[0].key)
        }
    }
    
    /// Get last key
    pub fn last_key(&self) -> Option<u64> {
        let count = self.count();
        if count == 0 {
            None
        } else {
            Some(self.entries[count - 1].key)
        }
    }
    
    /// Split node, returns new node and split key
    pub fn split(&mut self, new_block: u64) -> (Self, u64) {
        let count = self.count();
        let mid = count / 2;
        
        let mut new_node = Self::new(
            self.header.level,
            TreeType::from_u8(self.header.tree_type).unwrap_or(TreeType::KeyValue),
            new_block,
        );
        new_node.header.generation = self.header.generation;
        
        // Copy second half to new node
        for i in mid..count {
            new_node.entries[i - mid] = self.entries[i];
        }
        new_node.header.item_count = (count - mid) as u16;
        
        // Update this node
        let split_key = self.entries[mid].key;
        self.header.item_count = mid as u16;
        
        // Update sibling links
        new_node.header.left_sibling = self.header.block_num;
        new_node.header.right_sibling = self.header.right_sibling;
        self.header.right_sibling = new_block;
        
        (new_node, split_key)
    }
    
    /// Calculate checksum
    pub fn calculate_checksum(&self) -> u32 {
        let bytes = unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                NODE_SIZE - 8,
            )
        };
        Crc32c::hash(bytes)
    }
    
    /// Update checksum
    pub fn update_checksum(&mut self) {
        self.header.checksum = self.calculate_checksum();
    }
}

// ============================================================================
// Leaf Node with Fixed-Size Values
// ============================================================================

/// Maximum leaf entries for 8-byte key + 24-byte value
pub const MAX_LEAF_ENTRIES_8_24: usize = NODE_DATA_SIZE / 32;

/// Leaf entry with 24-byte value
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct LeafEntry8_24 {
    /// Key
    pub key: u64,
    /// Value (24 bytes)
    pub value: [u8; 24],
}

impl LeafEntry8_24 {
    /// Size
    pub const SIZE: usize = 32;
    
    /// Create new
    pub fn new(key: u64, value: &[u8]) -> Self {
        let mut entry = Self {
            key,
            value: [0; 24],
        };
        let len = value.len().min(24);
        entry.value[..len].copy_from_slice(&value[..len]);
        entry
    }
}

/// Leaf node with 8-byte keys and 24-byte values
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct LeafNode8_24 {
    /// Header
    pub header: NodeHeader,
    /// Entries
    pub entries: [LeafEntry8_24; MAX_LEAF_ENTRIES_8_24],
}

impl LeafNode8_24 {
    /// Create new leaf node
    pub fn new(tree_type: TreeType, block_num: u64) -> Self {
        let mut header = NodeHeader::new(NodeType::Leaf, 0, tree_type);
        header.block_num = block_num;
        
        Self {
            header,
            entries: [LeafEntry8_24 { key: 0, value: [0; 24] }; MAX_LEAF_ENTRIES_8_24],
        }
    }
    
    /// Get entry count
    #[inline]
    pub fn count(&self) -> usize {
        self.header.item_count as usize
    }
    
    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }
    
    /// Check if full
    #[inline]
    pub fn is_full(&self) -> bool {
        self.count() >= MAX_LEAF_ENTRIES_8_24
    }
    
    /// Get entries
    pub fn entries(&self) -> &[LeafEntry8_24] {
        &self.entries[..self.count()]
    }
    
    /// Binary search
    pub fn search(&self, key: u64) -> SearchResult {
        let entries = self.entries();
        let mut left = 0;
        let mut right = entries.len();
        
        while left < right {
            let mid = left + (right - left) / 2;
            if entries[mid].key == key {
                return SearchResult::Found(mid);
            } else if entries[mid].key < key {
                left = mid + 1;
            } else {
                right = mid;
            }
        }
        
        SearchResult::NotFound(left)
    }
    
    /// Get value for key
    pub fn get(&self, key: u64) -> Option<&[u8; 24]> {
        match self.search(key) {
            SearchResult::Found(i) => Some(&self.entries[i].value),
            SearchResult::NotFound(_) => None,
        }
    }
    
    /// Insert entry
    pub fn insert(&mut self, key: u64, value: &[u8]) -> HfsResult<()> {
        if self.is_full() {
            return Err(HfsError::BtreeFull);
        }
        
        let pos = match self.search(key) {
            SearchResult::Found(i) => {
                // Update existing
                self.entries[i] = LeafEntry8_24::new(key, value);
                return Ok(());
            }
            SearchResult::NotFound(i) => i,
        };
        
        let count = self.count();
        
        // Shift entries
        for i in (pos..count).rev() {
            self.entries[i + 1] = self.entries[i];
        }
        
        // Insert
        self.entries[pos] = LeafEntry8_24::new(key, value);
        self.header.item_count += 1;
        
        Ok(())
    }
    
    /// Remove entry at index
    pub fn remove(&mut self, index: usize) -> HfsResult<LeafEntry8_24> {
        let count = self.count();
        if index >= count {
            return Err(HfsError::BtreeKeyNotFound);
        }
        
        let entry = self.entries[index];
        
        // Shift entries
        for i in index..count - 1 {
            self.entries[i] = self.entries[i + 1];
        }
        
        self.header.item_count -= 1;
        Ok(entry)
    }
    
    /// Delete by key
    pub fn delete(&mut self, key: u64) -> HfsResult<()> {
        match self.search(key) {
            SearchResult::Found(i) => {
                self.remove(i)?;
                Ok(())
            }
            SearchResult::NotFound(_) => Err(HfsError::BtreeKeyNotFound),
        }
    }
    
    /// First key
    pub fn first_key(&self) -> Option<u64> {
        if self.is_empty() {
            None
        } else {
            Some(self.entries[0].key)
        }
    }
    
    /// Split node
    pub fn split(&mut self, new_block: u64) -> (Self, u64) {
        let count = self.count();
        let mid = count / 2;
        
        let mut new_node = Self::new(
            TreeType::from_u8(self.header.tree_type).unwrap_or(TreeType::KeyValue),
            new_block,
        );
        new_node.header.generation = self.header.generation;
        
        // Copy second half
        for i in mid..count {
            new_node.entries[i - mid] = self.entries[i];
        }
        new_node.header.item_count = (count - mid) as u16;
        
        // Update this node
        let split_key = new_node.entries[0].key;
        self.header.item_count = mid as u16;
        
        // Update sibling links
        new_node.header.left_sibling = self.header.block_num;
        new_node.header.right_sibling = self.header.right_sibling;
        self.header.right_sibling = new_block;
        
        (new_node, split_key)
    }
}

// ============================================================================
// Generic Node Buffer
// ============================================================================

/// Node buffer for reading/writing any node type
#[repr(C, align(4096))]
pub struct NodeBuffer {
    /// Raw bytes
    data: [u8; NODE_SIZE],
}

impl NodeBuffer {
    /// Create new zeroed buffer
    pub const fn new() -> Self {
        Self {
            data: [0; NODE_SIZE],
        }
    }
    
    /// Get header reference
    pub fn header(&self) -> &NodeHeader {
        unsafe { &*(self.data.as_ptr() as *const NodeHeader) }
    }
    
    /// Get mutable header
    pub fn header_mut(&mut self) -> &mut NodeHeader {
        unsafe { &mut *(self.data.as_mut_ptr() as *mut NodeHeader) }
    }
    
    /// Get as internal node
    pub fn as_internal64(&self) -> &InternalNode64 {
        unsafe { &*(self.data.as_ptr() as *const InternalNode64) }
    }
    
    /// Get as mutable internal node
    pub fn as_internal64_mut(&mut self) -> &mut InternalNode64 {
        unsafe { &mut *(self.data.as_mut_ptr() as *mut InternalNode64) }
    }
    
    /// Get as leaf node
    pub fn as_leaf8_24(&self) -> &LeafNode8_24 {
        unsafe { &*(self.data.as_ptr() as *const LeafNode8_24) }
    }
    
    /// Get as mutable leaf node
    pub fn as_leaf8_24_mut(&mut self) -> &mut LeafNode8_24 {
        unsafe { &mut *(self.data.as_mut_ptr() as *mut LeafNode8_24) }
    }
    
    /// Get raw bytes
    pub fn as_bytes(&self) -> &[u8; NODE_SIZE] {
        &self.data
    }
    
    /// Get mutable bytes
    pub fn as_bytes_mut(&mut self) -> &mut [u8; NODE_SIZE] {
        &mut self.data
    }
    
    /// Validate buffer as node
    pub fn validate(&self) -> HfsResult<()> {
        self.header().validate()
    }
}

impl Default for NodeBuffer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_node_header() {
        let header = NodeHeader::new(NodeType::Leaf, 0, TreeType::Inode);
        
        assert!(header.is_leaf());
        assert!(!header.is_internal());
        assert_eq!(header.item_count, 0);
        assert!(header.validate().is_ok());
    }
    
    #[test]
    fn test_internal_node() {
        let mut node = InternalNode64::new(1, TreeType::Inode, 100);
        
        assert!(node.is_empty());
        
        node.insert(50, 1000).unwrap();
        node.insert(100, 2000).unwrap();
        node.insert(25, 500).unwrap();
        
        assert_eq!(node.count(), 3);
        
        // Should be sorted
        let entries = node.entries();
        assert_eq!(entries[0].key, 25);
        assert_eq!(entries[1].key, 50);
        assert_eq!(entries[2].key, 100);
        
        // Find child
        assert_eq!(node.find_child(75), Some(1000));
        assert_eq!(node.find_child(200), Some(2000));
    }
    
    #[test]
    fn test_leaf_node() {
        let mut node = LeafNode8_24::new(TreeType::Inode, 100);
        
        node.insert(50, b"value50value50value50xxx").unwrap();
        node.insert(100, b"value100").unwrap();
        node.insert(25, b"value25").unwrap();
        
        assert_eq!(node.count(), 3);
        
        // Get value
        let val = node.get(50).unwrap();
        assert_eq!(&val[..8], b"value50v");
        
        // Delete
        node.delete(50).unwrap();
        assert_eq!(node.count(), 2);
        assert!(node.get(50).is_none());
    }
    
    #[test]
    fn test_node_split() {
        let mut node = InternalNode64::new(1, TreeType::Inode, 100);
        
        // Fill node
        for i in 0..50 {
            node.insert(i * 10, i * 1000).unwrap();
        }
        
        let (new_node, split_key) = node.split(200);
        
        // Check split
        assert!(node.count() > 0);
        assert!(new_node.count() > 0);
        assert_eq!(node.count() + new_node.count(), 50);
        
        // Keys should be partitioned
        assert!(node.last_key().unwrap() < split_key);
        assert!(new_node.first_key().unwrap() >= split_key);
    }
}
