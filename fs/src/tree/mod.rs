//! B-tree and metadata engine for HelixFS.
//!
//! This module provides:
//! - B+tree for ordered key-value storage
//! - Radix tree for fast integer-key lookup
//! - Directory entries management
//! - Metadata indexing

pub mod btree;
pub mod radix;
pub mod node;
pub mod cursor;
pub mod dir;

pub use btree::*;
pub use radix::*;
pub use node::*;
pub use cursor::*;
pub use dir::*;

use crate::core::error::{HfsError, HfsResult};

// ============================================================================
// Tree Configuration
// ============================================================================

/// Tree type identifier
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum TreeType {
    /// Inode tree (key = inode number)
    Inode = 1,
    /// Directory tree (key = name hash)
    Directory = 2,
    /// Extent tree (key = logical block)
    Extent = 3,
    /// Extended attribute tree
    Xattr = 4,
    /// Free space tree
    FreeSpace = 5,
    /// Snapshot tree
    Snapshot = 6,
    /// Quota tree
    Quota = 7,
    /// Generic key-value tree
    KeyValue = 8,
}

impl TreeType {
    /// From byte
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::Inode),
            2 => Some(Self::Directory),
            3 => Some(Self::Extent),
            4 => Some(Self::Xattr),
            5 => Some(Self::FreeSpace),
            6 => Some(Self::Snapshot),
            7 => Some(Self::Quota),
            8 => Some(Self::KeyValue),
            _ => None,
        }
    }
}

/// Tree configuration
#[derive(Clone, Copy, Debug)]
pub struct TreeConfig {
    /// Tree type
    pub tree_type: TreeType,
    /// Key size in bytes (0 = variable)
    pub key_size: u16,
    /// Value size in bytes (0 = variable)
    pub value_size: u16,
    /// Maximum tree depth
    pub max_depth: u8,
    /// Minimum fill percentage (for rebalancing)
    pub min_fill: u8,
    /// Allow duplicate keys
    pub allow_duplicates: bool,
    /// Store values in leaves only (B+tree)
    pub leaf_values_only: bool,
}

impl TreeConfig {
    /// Default config for inode tree
    pub const fn inode() -> Self {
        Self {
            tree_type: TreeType::Inode,
            key_size: 8,   // u64 inode number
            value_size: 0, // Variable (inode data)
            max_depth: 8,
            min_fill: 40,
            allow_duplicates: false,
            leaf_values_only: true,
        }
    }
    
    /// Default config for directory tree
    pub const fn directory() -> Self {
        Self {
            tree_type: TreeType::Directory,
            key_size: 8,   // u64 name hash
            value_size: 0, // Variable (dir entry)
            max_depth: 16,
            min_fill: 40,
            allow_duplicates: true, // Hash collisions
            leaf_values_only: true,
        }
    }
    
    /// Default config for extent tree
    pub const fn extent() -> Self {
        Self {
            tree_type: TreeType::Extent,
            key_size: 8,   // u64 logical block
            value_size: 24, // Fixed extent entry
            max_depth: 8,
            min_fill: 50,
            allow_duplicates: false,
            leaf_values_only: true,
        }
    }
}

// ============================================================================
// Tree Header (on-disk)
// ============================================================================

/// On-disk tree header.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct TreeHeader {
    /// Magic number
    pub magic: u32,
    /// Tree type
    pub tree_type: u8,
    /// Key size (0 = variable)
    pub key_size: u8,
    /// Value size (0 = variable)
    pub value_size: u8,
    /// Current tree depth
    pub depth: u8,
    /// Root node block
    pub root_block: u64,
    /// Total items in tree
    pub item_count: u64,
    /// Total nodes in tree
    pub node_count: u64,
    /// Generation number
    pub generation: u64,
    /// Flags
    pub flags: u32,
    /// Checksum
    pub checksum: u32,
}

impl TreeHeader {
    /// Size in bytes
    pub const SIZE: usize = 48;
    
    /// Magic number
    pub const MAGIC: u32 = 0x48545245; // "HTRE"
    
    /// Create new header
    pub fn new(config: &TreeConfig) -> Self {
        Self {
            magic: Self::MAGIC,
            tree_type: config.tree_type as u8,
            key_size: config.key_size as u8,
            value_size: config.value_size as u8,
            depth: 0,
            root_block: 0,
            item_count: 0,
            node_count: 0,
            generation: 0,
            flags: 0,
            checksum: 0,
        }
    }
    
    /// Validate header
    pub fn validate(&self) -> HfsResult<()> {
        if self.magic != Self::MAGIC {
            return Err(HfsError::BtreeCorruption);
        }
        if TreeType::from_u8(self.tree_type).is_none() {
            return Err(HfsError::BtreeCorruption);
        }
        Ok(())
    }
}

// ============================================================================
// Key Types
// ============================================================================

/// Maximum key size
pub const MAX_KEY_SIZE: usize = 256;

/// Fixed-size key
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Key64(pub u64);

impl Key64 {
    #[inline]
    pub const fn new(v: u64) -> Self {
        Self(v)
    }
    
    #[inline]
    pub fn to_bytes(self) -> [u8; 8] {
        self.0.to_be_bytes()
    }
    
    #[inline]
    pub fn from_bytes(bytes: &[u8; 8]) -> Self {
        Self(u64::from_be_bytes(*bytes))
    }
}

/// Variable-size key (for names, etc.)
pub struct VarKey {
    /// Key data
    data: [u8; MAX_KEY_SIZE],
    /// Actual length
    len: u16,
}

impl VarKey {
    /// Create empty key
    pub const fn new() -> Self {
        Self {
            data: [0; MAX_KEY_SIZE],
            len: 0,
        }
    }
    
    /// Create from bytes
    pub fn from_bytes(bytes: &[u8]) -> HfsResult<Self> {
        if bytes.len() > MAX_KEY_SIZE {
            return Err(HfsError::NameTooLong);
        }
        
        let mut key = Self::new();
        key.data[..bytes.len()].copy_from_slice(bytes);
        key.len = bytes.len() as u16;
        Ok(key)
    }
    
    /// Get bytes
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.data[..self.len as usize]
    }
    
    /// Get length
    #[inline]
    pub fn len(&self) -> usize {
        self.len as usize
    }
    
    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Default for VarKey {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for VarKey {
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl Eq for VarKey {}

impl PartialOrd for VarKey {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VarKey {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.as_bytes().cmp(other.as_bytes())
    }
}

// ============================================================================
// Search Result
// ============================================================================

/// Result of tree search
#[derive(Clone, Copy, Debug)]
pub enum SearchResult {
    /// Exact key found at index
    Found(usize),
    /// Key not found, would be inserted at index
    NotFound(usize),
}

impl SearchResult {
    /// Check if found
    #[inline]
    pub fn is_found(&self) -> bool {
        matches!(self, Self::Found(_))
    }
    
    /// Get index
    #[inline]
    pub fn index(&self) -> usize {
        match self {
            Self::Found(i) | Self::NotFound(i) => *i,
        }
    }
}

// ============================================================================
// Tree Operations
// ============================================================================

/// Tree operation type (for logging)
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum TreeOp {
    /// Insert new item
    Insert = 1,
    /// Update existing item
    Update = 2,
    /// Delete item
    Delete = 3,
    /// Split node
    Split = 4,
    /// Merge nodes
    Merge = 5,
    /// Rebalance nodes
    Rebalance = 6,
    /// Create new tree
    Create = 7,
    /// Delete tree
    Drop = 8,
}

/// Tree operation log entry
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct TreeOpLog {
    /// Operation type
    pub op: u8,
    /// Tree type
    pub tree_type: u8,
    /// Padding
    pub _pad: [u8; 2],
    /// Key (for fixed-size keys)
    pub key: u64,
    /// Node block number
    pub node_block: u64,
    /// Transaction ID
    pub txn_id: u64,
}

// ============================================================================
// Tree Statistics
// ============================================================================

/// Tree runtime statistics
#[derive(Clone, Copy, Debug, Default)]
pub struct TreeStats {
    /// Total lookups
    pub lookups: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Total inserts
    pub inserts: u64,
    /// Total deletes
    pub deletes: u64,
    /// Node splits
    pub splits: u64,
    /// Node merges
    pub merges: u64,
    /// Tree depth
    pub depth: u32,
    /// Total nodes
    pub nodes: u64,
    /// Total items
    pub items: u64,
}

impl TreeStats {
    /// Create new stats
    pub const fn new() -> Self {
        Self {
            lookups: 0,
            cache_hits: 0,
            cache_misses: 0,
            inserts: 0,
            deletes: 0,
            splits: 0,
            merges: 0,
            depth: 0,
            nodes: 0,
            items: 0,
        }
    }
    
    /// Cache hit ratio
    pub fn cache_hit_ratio(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }
    
    /// Average items per node
    pub fn avg_items_per_node(&self) -> f64 {
        if self.nodes == 0 {
            0.0
        } else {
            self.items as f64 / self.nodes as f64
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
    fn test_tree_config() {
        let config = TreeConfig::inode();
        assert_eq!(config.tree_type, TreeType::Inode);
        assert_eq!(config.key_size, 8);
        assert!(!config.allow_duplicates);
    }
    
    #[test]
    fn test_key64() {
        let key = Key64::new(12345);
        let bytes = key.to_bytes();
        let restored = Key64::from_bytes(&bytes);
        assert_eq!(key.0, restored.0);
    }
    
    #[test]
    fn test_var_key() {
        let key = VarKey::from_bytes(b"test_key").unwrap();
        assert_eq!(key.len(), 8);
        assert_eq!(key.as_bytes(), b"test_key");
    }
    
    #[test]
    fn test_search_result() {
        let found = SearchResult::Found(5);
        assert!(found.is_found());
        assert_eq!(found.index(), 5);
        
        let not_found = SearchResult::NotFound(3);
        assert!(!not_found.is_found());
        assert_eq!(not_found.index(), 3);
    }
}
