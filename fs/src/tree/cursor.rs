//! Tree cursor for iteration and navigation.
//!
//! Provides efficient stateful iteration over B+tree entries,
//! supporting both forward and backward traversal.

use crate::core::error::{HfsError, HfsResult};
use crate::core::types::*;

// ============================================================================
// Cursor Position
// ============================================================================

/// Cursor position within a leaf node.
#[derive(Clone, Copy, Debug)]
pub struct LeafPosition {
    /// Leaf node block number
    pub block: BlockNum,
    /// Index within leaf
    pub index: u16,
    /// Key at this position
    pub key: u64,
}

impl LeafPosition {
    /// Create position at start of leaf
    pub fn at_start(block: BlockNum, key: u64) -> Self {
        Self {
            block,
            index: 0,
            key,
        }
    }

    /// Create position at end of leaf
    pub fn at_end(block: BlockNum, last_index: u16, key: u64) -> Self {
        Self {
            block,
            index: last_index,
            key,
        }
    }

    /// Create position at specific index
    pub fn at_index(block: BlockNum, index: u16, key: u64) -> Self {
        Self { block, index, key }
    }
}

/// Cursor state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorState {
    /// Not positioned
    Invalid,
    /// At valid entry
    Valid,
    /// Before first entry
    BeforeFirst,
    /// After last entry
    AfterLast,
    /// At end (iteration complete)
    End,
    /// Tree modified, cursor invalidated
    Stale,
}

impl CursorState {
    /// Check if cursor is valid for reading
    #[inline]
    pub fn is_valid(&self) -> bool {
        *self == Self::Valid
    }

    /// Check if cursor is at end
    #[inline]
    pub fn is_end(&self) -> bool {
        matches!(self, Self::End | Self::AfterLast)
    }

    /// Check if cursor is stale
    #[inline]
    pub fn is_stale(&self) -> bool {
        *self == Self::Stale
    }
}

// ============================================================================
// Cursor Item
// ============================================================================

/// Item returned by cursor.
#[derive(Clone, Copy, Debug)]
pub struct CursorItem {
    /// Key
    pub key: u64,
    /// Value (interpretation depends on tree type)
    pub value: u64,
    /// Offset within value (for inline data)
    pub offset: u32,
    /// Size of inline data (if any)
    pub size: u32,
    /// Flags
    pub flags: u8,
}

impl CursorItem {
    /// Create item
    pub const fn new(key: u64, value: u64) -> Self {
        Self {
            key,
            value,
            offset: 0,
            size: 0,
            flags: 0,
        }
    }

    /// Create item with inline data
    pub const fn with_inline(key: u64, offset: u32, size: u32) -> Self {
        Self {
            key,
            value: 0,
            offset,
            size,
            flags: 1,
        }
    }

    /// Check if has inline data
    #[inline]
    pub fn has_inline(&self) -> bool {
        (self.flags & 1) != 0
    }
}

// ============================================================================
// Tree Cursor
// ============================================================================

/// Stateful cursor for B+tree traversal.
///
/// Provides efficient iteration by maintaining position
/// across multiple operations.
pub struct TreeCursor {
    /// Tree root block
    tree_root: BlockNum,
    /// Tree generation (for staleness detection)
    tree_gen: u64,
    /// Current state
    state: CursorState,
    /// Current leaf position
    position: Option<LeafPosition>,
    /// Path from root to current leaf
    path: CursorPath,
    /// Iteration direction
    direction: CursorDirection,
    /// Flags
    flags: CursorFlags,
}

/// Path from root to cursor position.
#[derive(Clone, Debug)]
pub struct CursorPath {
    /// Path entries (from root to leaf)
    entries: [CursorPathEntry; MAX_TREE_DEPTH],
    /// Current depth
    depth: u8,
}

impl CursorPath {
    /// Maximum tree depth
    const MAX_DEPTH: usize = MAX_TREE_DEPTH;

    /// Create empty path
    pub const fn empty() -> Self {
        const EMPTY_ENTRY: CursorPathEntry = CursorPathEntry::empty();
        Self {
            entries: [EMPTY_ENTRY; MAX_TREE_DEPTH],
            depth: 0,
        }
    }

    /// Push entry onto path
    pub fn push(&mut self, entry: CursorPathEntry) -> HfsResult<()> {
        if self.depth as usize >= Self::MAX_DEPTH {
            return Err(HfsError::BtreeCorruption);
        }

        self.entries[self.depth as usize] = entry;
        self.depth += 1;
        Ok(())
    }

    /// Pop entry from path
    pub fn pop(&mut self) -> Option<CursorPathEntry> {
        if self.depth == 0 {
            return None;
        }

        self.depth -= 1;
        Some(self.entries[self.depth as usize])
    }

    /// Peek at top of path
    pub fn peek(&self) -> Option<&CursorPathEntry> {
        if self.depth == 0 {
            return None;
        }
        Some(&self.entries[self.depth as usize - 1])
    }

    /// Clear path
    pub fn clear(&mut self) {
        self.depth = 0;
    }

    /// Get depth
    #[inline]
    pub fn depth(&self) -> u8 {
        self.depth
    }

    /// Get entry at level
    pub fn get(&self, level: u8) -> Option<&CursorPathEntry> {
        if level >= self.depth {
            return None;
        }
        Some(&self.entries[level as usize])
    }
}

/// Maximum tree depth
const MAX_TREE_DEPTH: usize = 16;

/// Entry in cursor path.
#[derive(Clone, Copy, Debug)]
pub struct CursorPathEntry {
    /// Block number of node
    pub block: BlockNum,
    /// Index within node
    pub index: u16,
    /// Count of items in node
    pub count: u16,
    /// Node flags
    pub flags: u8,
    /// Padding
    pub _pad: [u8; 3],
}

impl CursorPathEntry {
    /// Create empty entry
    pub const fn empty() -> Self {
        Self {
            block: BlockNum::NULL,
            index: 0,
            count: 0,
            flags: 0,
            _pad: [0; 3],
        }
    }

    /// Create entry
    pub fn new(block: BlockNum, index: u16, count: u16) -> Self {
        Self {
            block,
            index,
            count,
            flags: 0,
            _pad: [0; 3],
        }
    }

    /// Check if at last entry
    #[inline]
    pub fn is_at_end(&self) -> bool {
        self.index >= self.count
    }

    /// Check if at first entry
    #[inline]
    pub fn is_at_start(&self) -> bool {
        self.index == 0
    }
}

/// Cursor direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorDirection {
    /// Forward iteration
    Forward,
    /// Backward iteration
    Backward,
}

/// Cursor flags.
#[derive(Clone, Copy, Debug, Default)]
pub struct CursorFlags {
    /// Skip deleted entries
    pub skip_deleted: bool,
    /// Include internal nodes in iteration
    pub include_internal: bool,
    /// Stop at key boundary
    pub key_bounded: bool,
    /// Reading only (allows optimizations)
    pub read_only: bool,
}

impl TreeCursor {
    /// Create new cursor for tree.
    pub fn new(tree_root: BlockNum, tree_gen: u64) -> Self {
        Self {
            tree_root,
            tree_gen,
            state: CursorState::Invalid,
            position: None,
            path: CursorPath::empty(),
            direction: CursorDirection::Forward,
            flags: CursorFlags::default(),
        }
    }

    /// Create read-only cursor.
    pub fn read_only(tree_root: BlockNum, tree_gen: u64) -> Self {
        let mut cursor = Self::new(tree_root, tree_gen);
        cursor.flags.read_only = true;
        cursor
    }

    /// Get current state
    #[inline]
    pub fn state(&self) -> CursorState {
        self.state
    }

    /// Check if cursor is valid
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.state.is_valid()
    }

    /// Get current position
    #[inline]
    pub fn position(&self) -> Option<LeafPosition> {
        self.position
    }

    /// Set direction
    pub fn set_direction(&mut self, direction: CursorDirection) {
        self.direction = direction;
    }

    /// Get direction
    #[inline]
    pub fn direction(&self) -> CursorDirection {
        self.direction
    }

    /// Invalidate cursor (tree modified)
    pub fn invalidate(&mut self) {
        self.state = CursorState::Stale;
    }

    /// Reset cursor
    pub fn reset(&mut self) {
        self.state = CursorState::Invalid;
        self.position = None;
        self.path.clear();
    }
}

// ============================================================================
// Cursor Operations Interface
// ============================================================================

/// Operations for cursor.
pub trait CursorOps {
    /// Seek to first entry.
    fn seek_first(&mut self, cursor: &mut TreeCursor) -> HfsResult<bool>;

    /// Seek to last entry.
    fn seek_last(&mut self, cursor: &mut TreeCursor) -> HfsResult<bool>;

    /// Seek to specific key.
    fn seek(&mut self, cursor: &mut TreeCursor, key: u64) -> HfsResult<bool>;

    /// Seek to key or next greater.
    fn seek_ge(&mut self, cursor: &mut TreeCursor, key: u64) -> HfsResult<bool>;

    /// Seek to key or next smaller.
    fn seek_le(&mut self, cursor: &mut TreeCursor, key: u64) -> HfsResult<bool>;

    /// Move to next entry.
    fn next(&mut self, cursor: &mut TreeCursor) -> HfsResult<bool>;

    /// Move to previous entry.
    fn prev(&mut self, cursor: &mut TreeCursor) -> HfsResult<bool>;

    /// Get current item.
    fn current(&self, cursor: &TreeCursor) -> HfsResult<CursorItem>;

    /// Check if cursor is valid.
    fn valid(&self, cursor: &TreeCursor) -> bool;
}

// ============================================================================
// Range Cursor
// ============================================================================

/// Cursor with key range bounds.
pub struct RangeCursor {
    /// Inner cursor
    pub cursor: TreeCursor,
    /// Lower bound (inclusive)
    pub start: Option<u64>,
    /// Upper bound (exclusive)
    pub end: Option<u64>,
    /// Items yielded
    pub count: u64,
    /// Maximum items to yield (0 = unlimited)
    pub limit: u64,
}

impl RangeCursor {
    /// Create unbounded range cursor
    pub fn unbounded(tree_root: BlockNum, tree_gen: u64) -> Self {
        Self {
            cursor: TreeCursor::read_only(tree_root, tree_gen),
            start: None,
            end: None,
            count: 0,
            limit: 0,
        }
    }

    /// Create bounded range cursor
    pub fn bounded(tree_root: BlockNum, tree_gen: u64, start: u64, end: u64) -> Self {
        Self {
            cursor: TreeCursor::read_only(tree_root, tree_gen),
            start: Some(start),
            end: Some(end),
            count: 0,
            limit: 0,
        }
    }

    /// Set limit
    pub fn with_limit(mut self, limit: u64) -> Self {
        self.limit = limit;
        self
    }

    /// Check if key is in range
    #[inline]
    pub fn in_range(&self, key: u64) -> bool {
        if let Some(start) = self.start {
            if key < start {
                return false;
            }
        }
        if let Some(end) = self.end {
            if key >= end {
                return false;
            }
        }
        true
    }

    /// Check if limit reached
    #[inline]
    pub fn limit_reached(&self) -> bool {
        self.limit > 0 && self.count >= self.limit
    }
}

// ============================================================================
// Prefix Cursor
// ============================================================================

/// Cursor that iterates keys with common prefix.
pub struct PrefixCursor {
    /// Inner cursor
    pub cursor: TreeCursor,
    /// Key prefix
    pub prefix: u64,
    /// Prefix mask
    pub mask: u64,
    /// Items yielded
    pub count: u64,
}

impl PrefixCursor {
    /// Create prefix cursor
    pub fn new(tree_root: BlockNum, tree_gen: u64, prefix: u64, prefix_bits: u8) -> Self {
        let mask = if prefix_bits >= 64 {
            u64::MAX
        } else {
            !((1u64 << (64 - prefix_bits)) - 1)
        };

        Self {
            cursor: TreeCursor::read_only(tree_root, tree_gen),
            prefix,
            mask,
            count: 0,
        }
    }

    /// Check if key matches prefix
    #[inline]
    pub fn matches(&self, key: u64) -> bool {
        (key & self.mask) == (self.prefix & self.mask)
    }

    /// Get start key for prefix
    #[inline]
    pub fn start_key(&self) -> u64 {
        self.prefix & self.mask
    }

    /// Get end key for prefix
    #[inline]
    pub fn end_key(&self) -> u64 {
        (self.prefix | !self.mask).saturating_add(1)
    }
}

// ============================================================================
// Cursor Statistics
// ============================================================================

/// Statistics for cursor operations.
#[derive(Clone, Copy, Debug, Default)]
pub struct CursorStats {
    /// Number of seek operations
    pub seeks: u64,
    /// Number of next operations
    pub nexts: u64,
    /// Number of prev operations
    pub prevs: u64,
    /// Number of items read
    pub items_read: u64,
    /// Number of leaf nodes visited
    pub leaves_visited: u64,
    /// Number of internal nodes visited
    pub internals_visited: u64,
    /// Number of leaf hops (moving to sibling leaf)
    pub leaf_hops: u64,
}

impl CursorStats {
    /// Create new stats
    pub const fn new() -> Self {
        Self {
            seeks: 0,
            nexts: 0,
            prevs: 0,
            items_read: 0,
            leaves_visited: 0,
            internals_visited: 0,
            leaf_hops: 0,
        }
    }

    /// Merge with another stats
    pub fn merge(&mut self, other: &Self) {
        self.seeks += other.seeks;
        self.nexts += other.nexts;
        self.prevs += other.prevs;
        self.items_read += other.items_read;
        self.leaves_visited += other.leaves_visited;
        self.internals_visited += other.internals_visited;
        self.leaf_hops += other.leaf_hops;
    }

    /// Calculate efficiency (items per node visited)
    pub fn efficiency(&self) -> f32 {
        let nodes = self.leaves_visited + self.internals_visited;
        if nodes == 0 {
            return 0.0;
        }
        self.items_read as f32 / nodes as f32
    }
}

// ============================================================================
// Multi-Cursor
// ============================================================================

/// Cursor over multiple trees (for merge operations).
#[derive(Debug)]
pub struct MultiCursor {
    /// Source tree roots
    pub sources: MultiCursorSources,
    /// Current minimum key
    pub current_key: Option<u64>,
    /// Merge direction
    pub direction: CursorDirection,
    /// Stats
    pub stats: CursorStats,
}

/// Sources for multi-cursor.
#[derive(Debug)]
pub struct MultiCursorSources {
    /// Tree roots
    pub roots: [Option<BlockNum>; 8],
    /// Tree generations
    pub generations: [u64; 8],
    /// Number of active sources
    pub count: usize,
}

impl MultiCursorSources {
    /// Maximum sources
    pub const MAX_SOURCES: usize = 8;

    /// Create empty sources
    pub const fn empty() -> Self {
        Self {
            roots: [None; 8],
            generations: [0; 8],
            count: 0,
        }
    }

    /// Add source tree
    pub fn add(&mut self, root: BlockNum, generation: u64) -> HfsResult<usize> {
        if self.count >= Self::MAX_SOURCES {
            return Err(HfsError::OutOfMemory);
        }

        let idx = self.count;
        self.roots[idx] = Some(root);
        self.generations[idx] = generation;
        self.count += 1;

        Ok(idx)
    }

    /// Get source at index
    pub fn get(&self, index: usize) -> Option<(BlockNum, u64)> {
        if index >= self.count {
            return None;
        }
        self.roots[index].map(|r| (r, self.generations[index]))
    }
}

impl MultiCursor {
    /// Create new multi-cursor
    pub fn new(direction: CursorDirection) -> Self {
        Self {
            sources: MultiCursorSources::empty(),
            current_key: None,
            direction,
            stats: CursorStats::new(),
        }
    }

    /// Add source tree
    pub fn add_source(&mut self, root: BlockNum, generation: u64) -> HfsResult<usize> {
        self.sources.add(root, generation)
    }

    /// Get number of sources
    #[inline]
    pub fn source_count(&self) -> usize {
        self.sources.count
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_state() {
        assert!(CursorState::Valid.is_valid());
        assert!(!CursorState::Invalid.is_valid());
        assert!(CursorState::End.is_end());
        assert!(CursorState::AfterLast.is_end());
        assert!(CursorState::Stale.is_stale());
    }

    #[test]
    fn test_cursor_item() {
        let item = CursorItem::new(100, 200);
        assert_eq!(item.key, 100);
        assert_eq!(item.value, 200);
        assert!(!item.has_inline());

        let inline = CursorItem::with_inline(100, 50, 25);
        assert!(inline.has_inline());
        assert_eq!(inline.offset, 50);
        assert_eq!(inline.size, 25);
    }

    #[test]
    fn test_tree_cursor() {
        let cursor = TreeCursor::new(100, 1);

        assert!(!cursor.is_valid());
        assert_eq!(cursor.state(), CursorState::Invalid);
        assert!(cursor.position().is_none());
    }

    #[test]
    fn test_cursor_path() {
        let mut path = CursorPath::empty();
        assert_eq!(path.depth(), 0);

        path.push(CursorPathEntry::new(100, 5, 10)).unwrap();
        assert_eq!(path.depth(), 1);
        assert_eq!(path.peek().unwrap().block, 100);

        let entry = path.pop().unwrap();
        assert_eq!(entry.block, 100);
        assert_eq!(path.depth(), 0);
    }

    #[test]
    fn test_range_cursor() {
        let cursor = RangeCursor::bounded(100, 1, 50, 150);

        assert!(cursor.in_range(50));
        assert!(cursor.in_range(100));
        assert!(!cursor.in_range(49));
        assert!(!cursor.in_range(150));
    }

    #[test]
    fn test_prefix_cursor() {
        let cursor = PrefixCursor::new(100, 1, 0xFF00_0000_0000_0000, 8);

        assert!(cursor.matches(0xFF00_0000_0000_0000));
        assert!(cursor.matches(0xFF12_3456_789A_BCDE));
        assert!(!cursor.matches(0xFE00_0000_0000_0000));
    }

    #[test]
    fn test_cursor_stats() {
        let mut stats = CursorStats::new();
        stats.seeks = 5;
        stats.items_read = 100;
        stats.leaves_visited = 10;

        let eff = stats.efficiency();
        assert_eq!(eff, 10.0);
    }

    #[test]
    fn test_multi_cursor() {
        let mut cursor = MultiCursor::new(CursorDirection::Forward);

        cursor.add_source(100, 1).unwrap();
        cursor.add_source(200, 1).unwrap();

        assert_eq!(cursor.source_count(), 2);
    }
}
