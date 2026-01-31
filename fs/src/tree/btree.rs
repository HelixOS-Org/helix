//! B+tree implementation.
//!
//! A B+tree optimized for filesystem metadata:
//! - All values stored in leaves
//! - Leaves linked for range scans
//! - Copy-on-Write friendly

use crate::core::types::*;
use crate::core::error::{HfsError, HfsResult};
use super::{TreeConfig, TreeHeader, TreeStats, SearchResult};
use super::node::*;

// ============================================================================
// B+Tree Path Entry
// ============================================================================

/// Entry in tree traversal path (for insert/delete)
#[derive(Clone, Copy, Debug)]
pub struct PathEntry {
    /// Block number of node
    pub block: BlockNum,
    /// Index in parent (for internal nodes)
    pub index: u16,
    /// Level in tree
    pub level: u8,
    /// Was node modified
    pub dirty: bool,
}

impl PathEntry {
    /// Create new path entry
    pub fn new(block: BlockNum, index: u16, level: u8) -> Self {
        Self {
            block,
            index,
            level,
            dirty: false,
        }
    }
}

/// Maximum tree depth
pub const MAX_TREE_DEPTH: usize = 16;

/// Tree traversal path
pub struct TreePath {
    /// Path entries from root to leaf
    entries: [PathEntry; MAX_TREE_DEPTH],
    /// Current depth
    depth: usize,
}

impl TreePath {
    /// Create empty path
    pub const fn new() -> Self {
        Self {
            entries: [PathEntry {
                block: BlockNum::new(0),
                index: 0,
                level: 0,
                dirty: false,
            }; MAX_TREE_DEPTH],
            depth: 0,
        }
    }
    
    /// Clear path
    pub fn clear(&mut self) {
        self.depth = 0;
    }
    
    /// Push entry
    pub fn push(&mut self, entry: PathEntry) -> HfsResult<()> {
        if self.depth >= MAX_TREE_DEPTH {
            return Err(HfsError::BtreeTooDeep);
        }
        self.entries[self.depth] = entry;
        self.depth += 1;
        Ok(())
    }
    
    /// Pop entry
    pub fn pop(&mut self) -> Option<PathEntry> {
        if self.depth == 0 {
            None
        } else {
            self.depth -= 1;
            Some(self.entries[self.depth])
        }
    }
    
    /// Get current depth
    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }
    
    /// Get entry at index
    pub fn get(&self, index: usize) -> Option<&PathEntry> {
        if index < self.depth {
            Some(&self.entries[index])
        } else {
            None
        }
    }
    
    /// Get mutable entry at index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut PathEntry> {
        if index < self.depth {
            Some(&mut self.entries[index])
        } else {
            None
        }
    }
    
    /// Get leaf entry
    pub fn leaf(&self) -> Option<&PathEntry> {
        if self.depth == 0 {
            None
        } else {
            Some(&self.entries[self.depth - 1])
        }
    }
    
    /// Get root entry
    pub fn root(&self) -> Option<&PathEntry> {
        if self.depth == 0 {
            None
        } else {
            Some(&self.entries[0])
        }
    }
    
    /// Iterator over path
    pub fn iter(&self) -> impl Iterator<Item = &PathEntry> {
        self.entries[..self.depth].iter()
    }
}

impl Default for TreePath {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// B+Tree State
// ============================================================================

/// B+tree state and metadata.
pub struct BPlusTree {
    /// Tree configuration
    config: TreeConfig,
    /// Tree header
    header: TreeHeader,
    /// Statistics
    stats: TreeStats,
    /// Is tree dirty
    dirty: bool,
}

impl BPlusTree {
    /// Create new empty tree
    pub fn new(config: TreeConfig) -> Self {
        Self {
            header: TreeHeader::new(&config),
            config,
            stats: TreeStats::new(),
            dirty: true,
        }
    }
    
    /// Get configuration
    #[inline]
    pub fn config(&self) -> &TreeConfig {
        &self.config
    }
    
    /// Get header
    #[inline]
    pub fn header(&self) -> &TreeHeader {
        &self.header
    }
    
    /// Get mutable header
    #[inline]
    pub fn header_mut(&mut self) -> &mut TreeHeader {
        self.dirty = true;
        &mut self.header
    }
    
    /// Get stats
    #[inline]
    pub fn stats(&self) -> &TreeStats {
        &self.stats
    }
    
    /// Get mutable stats
    #[inline]
    pub fn stats_mut(&mut self) -> &mut TreeStats {
        &mut self.stats
    }
    
    /// Check if tree is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.header.item_count == 0
    }
    
    /// Get item count
    #[inline]
    pub fn len(&self) -> u64 {
        self.header.item_count
    }
    
    /// Get tree depth
    #[inline]
    pub fn depth(&self) -> u8 {
        self.header.depth
    }
    
    /// Get root block
    #[inline]
    pub fn root_block(&self) -> BlockNum {
        BlockNum::new(self.header.root_block)
    }
    
    /// Set root block
    pub fn set_root(&mut self, block: BlockNum, depth: u8) {
        self.header.root_block = block.get();
        self.header.depth = depth;
        self.dirty = true;
    }
    
    /// Increment item count
    pub fn inc_count(&mut self) {
        self.header.item_count += 1;
        self.stats.items += 1;
        self.stats.inserts += 1;
        self.dirty = true;
    }
    
    /// Decrement item count
    pub fn dec_count(&mut self) {
        self.header.item_count = self.header.item_count.saturating_sub(1);
        self.stats.items = self.stats.items.saturating_sub(1);
        self.stats.deletes += 1;
        self.dirty = true;
    }
    
    /// Increment node count
    pub fn inc_nodes(&mut self) {
        self.header.node_count += 1;
        self.stats.nodes += 1;
        self.dirty = true;
    }
    
    /// Decrement node count
    pub fn dec_nodes(&mut self) {
        self.header.node_count = self.header.node_count.saturating_sub(1);
        self.stats.nodes = self.stats.nodes.saturating_sub(1);
        self.dirty = true;
    }
    
    /// Record split
    pub fn record_split(&mut self) {
        self.stats.splits += 1;
    }
    
    /// Record merge
    pub fn record_merge(&mut self) {
        self.stats.merges += 1;
    }
    
    /// Record lookup
    pub fn record_lookup(&mut self, cache_hit: bool) {
        self.stats.lookups += 1;
        if cache_hit {
            self.stats.cache_hits += 1;
        } else {
            self.stats.cache_misses += 1;
        }
    }
    
    /// Increment generation
    pub fn inc_generation(&mut self) {
        self.header.generation += 1;
        self.dirty = true;
    }
    
    /// Check if dirty
    #[inline]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    
    /// Mark clean
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }
}

// ============================================================================
// B+Tree Operations (for u64 keys)
// ============================================================================

/// B+tree operations for u64 keys with 24-byte values.
pub struct BTreeOps64_24;

impl BTreeOps64_24 {
    /// Search for key in leaf node
    pub fn search_leaf(node: &LeafNode8_24, key: u64) -> SearchResult {
        node.search(key)
    }
    
    /// Search for child in internal node
    pub fn search_internal(node: &InternalNode64, key: u64) -> Option<u64> {
        node.find_child(key)
    }
    
    /// Check if node needs split
    pub fn needs_split_leaf(node: &LeafNode8_24) -> bool {
        node.count() >= MAX_LEAF_ENTRIES_8_24 - 1
    }
    
    /// Check if node needs split
    pub fn needs_split_internal(node: &InternalNode64) -> bool {
        node.count() >= MAX_INTERNAL_ENTRIES_64 - 1
    }
    
    /// Check if node needs merge (less than half full)
    pub fn needs_merge_leaf(node: &LeafNode8_24) -> bool {
        node.count() < MAX_LEAF_ENTRIES_8_24 / 4
    }
    
    /// Check if node needs merge
    pub fn needs_merge_internal(node: &InternalNode64) -> bool {
        node.count() < MAX_INTERNAL_ENTRIES_64 / 4
    }
    
    /// Calculate fill percentage
    pub fn fill_percent_leaf(node: &LeafNode8_24) -> u8 {
        ((node.count() * 100) / MAX_LEAF_ENTRIES_8_24) as u8
    }
    
    /// Calculate fill percentage
    pub fn fill_percent_internal(node: &InternalNode64) -> u8 {
        ((node.count() * 100) / MAX_INTERNAL_ENTRIES_64) as u8
    }
}

// ============================================================================
// Insert Result
// ============================================================================

/// Result of insert operation
#[derive(Clone, Copy, Debug)]
pub enum InsertResult {
    /// Inserted successfully
    Inserted,
    /// Updated existing key
    Updated,
    /// Node was split, propagate up
    Split {
        /// New node block
        new_block: BlockNum,
        /// Split key
        split_key: u64,
    },
    /// Tree needs new root
    NewRoot {
        /// Left child
        left: BlockNum,
        /// Right child
        right: BlockNum,
        /// Split key
        split_key: u64,
    },
}

impl InsertResult {
    /// Check if split occurred
    pub fn is_split(&self) -> bool {
        matches!(self, Self::Split { .. } | Self::NewRoot { .. })
    }
}

// ============================================================================
// Delete Result
// ============================================================================

/// Result of delete operation
#[derive(Clone, Copy, Debug)]
pub enum DeleteResult {
    /// Deleted successfully
    Deleted,
    /// Key not found
    NotFound,
    /// Node needs merge/rebalance
    NeedsMerge,
    /// Node was merged, propagate up
    Merged {
        /// Block that was freed
        freed_block: BlockNum,
    },
    /// Keys were redistributed
    Redistributed,
}

// ============================================================================
// Range Query
// ============================================================================

/// Range query bounds
#[derive(Clone, Copy, Debug)]
pub struct KeyRange {
    /// Start key (inclusive)
    pub start: u64,
    /// End key (exclusive)
    pub end: u64,
    /// Maximum items to return
    pub limit: usize,
}

impl KeyRange {
    /// Create new range
    pub const fn new(start: u64, end: u64) -> Self {
        Self {
            start,
            end,
            limit: usize::MAX,
        }
    }
    
    /// Full range
    pub const fn all() -> Self {
        Self::new(0, u64::MAX)
    }
    
    /// Single key
    pub const fn single(key: u64) -> Self {
        Self::new(key, key + 1)
    }
    
    /// With limit
    pub const fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
    
    /// Check if key is in range
    #[inline]
    pub fn contains(&self, key: u64) -> bool {
        key >= self.start && key < self.end
    }
    
    /// Check if range is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

// ============================================================================
// Iterator State
// ============================================================================

/// State for iterating over tree
#[derive(Clone, Copy)]
pub struct IterState {
    /// Current leaf block
    pub leaf_block: BlockNum,
    /// Current index in leaf
    pub index: u16,
    /// Remaining items to return
    pub remaining: usize,
    /// Key range
    pub range: KeyRange,
}

impl IterState {
    /// Create new iterator state
    pub fn new(leaf_block: BlockNum, index: u16, range: KeyRange) -> Self {
        Self {
            leaf_block,
            index,
            remaining: range.limit,
            range,
        }
    }
    
    /// Check if exhausted
    #[inline]
    pub fn is_exhausted(&self) -> bool {
        self.remaining == 0 || self.leaf_block.is_zero()
    }
    
    /// Advance to next item
    pub fn advance(&mut self, leaf_count: u16, next_leaf: BlockNum) {
        if self.remaining > 0 {
            self.remaining -= 1;
        }
        
        self.index += 1;
        if self.index >= leaf_count {
            self.leaf_block = next_leaf;
            self.index = 0;
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
    fn test_tree_path() {
        let mut path = TreePath::new();
        
        path.push(PathEntry::new(BlockNum::new(100), 0, 2)).unwrap();
        path.push(PathEntry::new(BlockNum::new(200), 5, 1)).unwrap();
        path.push(PathEntry::new(BlockNum::new(300), 3, 0)).unwrap();
        
        assert_eq!(path.depth(), 3);
        assert_eq!(path.root().unwrap().block.get(), 100);
        assert_eq!(path.leaf().unwrap().block.get(), 300);
        
        let popped = path.pop().unwrap();
        assert_eq!(popped.block.get(), 300);
        assert_eq!(path.depth(), 2);
    }
    
    #[test]
    fn test_bplus_tree() {
        let config = TreeConfig::inode();
        let mut tree = BPlusTree::new(config);
        
        assert!(tree.is_empty());
        assert_eq!(tree.len(), 0);
        
        tree.inc_count();
        tree.inc_count();
        
        assert!(!tree.is_empty());
        assert_eq!(tree.len(), 2);
        assert_eq!(tree.stats().inserts, 2);
        
        tree.dec_count();
        assert_eq!(tree.len(), 1);
        assert_eq!(tree.stats().deletes, 1);
    }
    
    #[test]
    fn test_key_range() {
        let range = KeyRange::new(10, 100);
        
        assert!(range.contains(10));
        assert!(range.contains(50));
        assert!(range.contains(99));
        assert!(!range.contains(100));
        assert!(!range.contains(9));
        
        let limited = range.with_limit(5);
        assert_eq!(limited.limit, 5);
    }
    
    #[test]
    fn test_insert_result() {
        let result = InsertResult::Split {
            new_block: BlockNum::new(100),
            split_key: 50,
        };
        
        assert!(result.is_split());
        
        let result2 = InsertResult::Inserted;
        assert!(!result2.is_split());
    }
    
    #[test]
    fn test_iter_state() {
        let range = KeyRange::new(0, 100).with_limit(10);
        let mut state = IterState::new(BlockNum::new(100), 0, range);
        
        assert!(!state.is_exhausted());
        assert_eq!(state.remaining, 10);
        
        state.advance(5, BlockNum::new(200));
        assert_eq!(state.remaining, 9);
        assert_eq!(state.index, 1);
        
        // Advance past leaf end
        state.index = 4;
        state.advance(5, BlockNum::new(300));
        assert_eq!(state.leaf_block.get(), 300);
        assert_eq!(state.index, 0);
    }
}
