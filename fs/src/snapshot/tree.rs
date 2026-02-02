//! Snapshot tree management.
//!
//! Manages the hierarchical structure of snapshots
//! and provides traversal operations.

use crate::core::error::{HfsError, HfsResult};
use crate::snapshot::core::SnapshotPath;
use crate::snapshot::{
    SnapshotId, SnapshotState, INVALID_SNAPSHOT_ID, MAX_SNAPSHOT_DEPTH, ROOT_SNAPSHOT_ID,
};

// ============================================================================
// Snapshot Tree Node
// ============================================================================

/// In-memory snapshot tree node.
#[derive(Clone, Copy, Debug)]
pub struct SnapshotTreeNode {
    /// Snapshot ID
    pub id: SnapshotId,
    /// Parent ID
    pub parent: SnapshotId,
    /// First child ID
    pub first_child: SnapshotId,
    /// Next sibling ID
    pub next_sibling: SnapshotId,
    /// Previous sibling ID
    pub prev_sibling: SnapshotId,
    /// Depth in tree
    pub depth: u16,
    /// Child count
    pub child_count: u16,
    /// State
    pub state: SnapshotState,
    /// Flags
    pub flags: u8,
}

impl SnapshotTreeNode {
    /// Create new node
    pub fn new(id: SnapshotId, parent: SnapshotId, depth: u16) -> Self {
        Self {
            id,
            parent,
            first_child: INVALID_SNAPSHOT_ID,
            next_sibling: INVALID_SNAPSHOT_ID,
            prev_sibling: INVALID_SNAPSHOT_ID,
            depth,
            child_count: 0,
            state: SnapshotState::Active,
            flags: 0,
        }
    }

    /// Create root node
    pub fn root() -> Self {
        Self::new(ROOT_SNAPSHOT_ID, INVALID_SNAPSHOT_ID, 0)
    }

    /// Check if has children
    #[inline]
    pub fn has_children(&self) -> bool {
        self.first_child != INVALID_SNAPSHOT_ID
    }

    /// Check if has siblings
    #[inline]
    pub fn has_next(&self) -> bool {
        self.next_sibling != INVALID_SNAPSHOT_ID
    }

    /// Check if is leaf
    #[inline]
    pub fn is_leaf(&self) -> bool {
        !self.has_children()
    }

    /// Check if is root
    #[inline]
    pub fn is_root(&self) -> bool {
        self.parent == INVALID_SNAPSHOT_ID
    }
}

impl Default for SnapshotTreeNode {
    fn default() -> Self {
        Self::new(INVALID_SNAPSHOT_ID, INVALID_SNAPSHOT_ID, 0)
    }
}

// ============================================================================
// Snapshot Tree
// ============================================================================

/// Maximum nodes in memory
const MAX_TREE_NODES: usize = 256;

/// Snapshot tree structure.
pub struct SnapshotTree {
    /// Nodes
    pub nodes: [SnapshotTreeNode; MAX_TREE_NODES],
    /// Node count
    pub count: usize,
    /// Root node index
    pub root_index: usize,
    /// Maximum depth seen
    pub max_depth: u16,
}

impl SnapshotTree {
    /// Create new tree with root
    pub fn new() -> Self {
        let mut tree = Self {
            nodes: [SnapshotTreeNode::default(); MAX_TREE_NODES],
            count: 0,
            root_index: 0,
            max_depth: 0,
        };

        // Add root node
        tree.nodes[0] = SnapshotTreeNode::root();
        tree.count = 1;

        tree
    }

    /// Find node by ID
    pub fn find(&self, id: SnapshotId) -> Option<usize> {
        for i in 0..self.count {
            if self.nodes[i].id == id {
                return Some(i);
            }
        }
        None
    }

    /// Get node by ID
    pub fn get(&self, id: SnapshotId) -> Option<&SnapshotTreeNode> {
        self.find(id).map(|i| &self.nodes[i])
    }

    /// Get node by ID (mutable)
    pub fn get_mut(&mut self, id: SnapshotId) -> Option<&mut SnapshotTreeNode> {
        if let Some(i) = self.find(id) {
            Some(&mut self.nodes[i])
        } else {
            None
        }
    }

    /// Add child to parent
    pub fn add_child(&mut self, parent_id: SnapshotId, child_id: SnapshotId) -> HfsResult<usize> {
        if self.count >= MAX_TREE_NODES {
            return Err(HfsError::OutOfMemory);
        }

        let parent_idx = self.find(parent_id).ok_or(HfsError::SnapshotNotFound)?;
        let parent_depth = self.nodes[parent_idx].depth;

        if parent_depth + 1 >= MAX_SNAPSHOT_DEPTH as u16 {
            return Err(HfsError::SnapshotDepthExceeded);
        }

        // Create new node
        let child_idx = self.count;
        let new_depth = parent_depth + 1;
        self.nodes[child_idx] = SnapshotTreeNode::new(child_id, parent_id, new_depth);

        // Link to parent's children
        if self.nodes[parent_idx].first_child == INVALID_SNAPSHOT_ID {
            self.nodes[parent_idx].first_child = child_id;
        } else {
            // Find last sibling
            let mut curr_id = self.nodes[parent_idx].first_child;
            let mut last_idx = self.find(curr_id).unwrap();

            while self.nodes[last_idx].next_sibling != INVALID_SNAPSHOT_ID {
                curr_id = self.nodes[last_idx].next_sibling;
                last_idx = self.find(curr_id).unwrap();
            }

            // Link
            self.nodes[last_idx].next_sibling = child_id;
            self.nodes[child_idx].prev_sibling = self.nodes[last_idx].id;
        }

        self.nodes[parent_idx].child_count += 1;
        self.count += 1;

        if new_depth > self.max_depth {
            self.max_depth = new_depth;
        }

        Ok(child_idx)
    }

    /// Remove node (must be leaf)
    pub fn remove(&mut self, id: SnapshotId) -> HfsResult<()> {
        let idx = self.find(id).ok_or(HfsError::SnapshotNotFound)?;

        if self.nodes[idx].has_children() {
            return Err(HfsError::SnapshotHasChildren);
        }

        if self.nodes[idx].is_root() {
            return Err(HfsError::CannotDeleteRoot);
        }

        // Unlink from siblings
        let prev = self.nodes[idx].prev_sibling;
        let next = self.nodes[idx].next_sibling;

        if prev != INVALID_SNAPSHOT_ID {
            if let Some(prev_idx) = self.find(prev) {
                self.nodes[prev_idx].next_sibling = next;
            }
        }

        if next != INVALID_SNAPSHOT_ID {
            if let Some(next_idx) = self.find(next) {
                self.nodes[next_idx].prev_sibling = prev;
            }
        }

        // Update parent
        let parent_id = self.nodes[idx].parent;
        if let Some(parent_idx) = self.find(parent_id) {
            if self.nodes[parent_idx].first_child == id {
                self.nodes[parent_idx].first_child = next;
            }
            self.nodes[parent_idx].child_count =
                self.nodes[parent_idx].child_count.saturating_sub(1);
        }

        // Swap with last and remove
        self.nodes[idx] = self.nodes[self.count - 1];
        self.nodes[self.count - 1] = SnapshotTreeNode::default();
        self.count -= 1;

        Ok(())
    }

    /// Get path to root from snapshot
    pub fn path_to_root(&self, id: SnapshotId) -> SnapshotPath {
        let mut path = SnapshotPath::empty();

        let mut curr_id = id;
        while curr_id != INVALID_SNAPSHOT_ID {
            if path.push(curr_id).is_err() {
                break;
            }

            if let Some(node) = self.get(curr_id) {
                curr_id = node.parent;
            } else {
                break;
            }
        }

        path
    }

    /// Find common ancestor of two snapshots
    pub fn common_ancestor(&self, a: SnapshotId, b: SnapshotId) -> Option<SnapshotId> {
        let path_a = self.path_to_root(a);
        let path_b = self.path_to_root(b);

        // Find first common ID
        for id_a in path_a.as_slice() {
            for id_b in path_b.as_slice() {
                if id_a == id_b {
                    return Some(*id_a);
                }
            }
        }

        None
    }

    /// Get depth of snapshot
    pub fn depth(&self, id: SnapshotId) -> Option<u16> {
        self.get(id).map(|n| n.depth)
    }

    /// Count children (direct)
    pub fn child_count(&self, id: SnapshotId) -> usize {
        self.get(id).map(|n| n.child_count as usize).unwrap_or(0)
    }

    /// Count all descendants
    pub fn descendant_count(&self, id: SnapshotId) -> usize {
        let mut count = 0;
        let mut stack = [INVALID_SNAPSHOT_ID; 64];
        let mut stack_len = 0;

        // Push first child
        if let Some(node) = self.get(id) {
            if node.first_child != INVALID_SNAPSHOT_ID {
                stack[0] = node.first_child;
                stack_len = 1;
            }
        }

        while stack_len > 0 {
            stack_len -= 1;
            let curr_id = stack[stack_len];
            count += 1;

            if let Some(node) = self.get(curr_id) {
                // Push children
                if node.first_child != INVALID_SNAPSHOT_ID && stack_len < 63 {
                    stack[stack_len] = node.first_child;
                    stack_len += 1;
                }

                // Push siblings
                if node.next_sibling != INVALID_SNAPSHOT_ID && stack_len < 63 {
                    stack[stack_len] = node.next_sibling;
                    stack_len += 1;
                }
            }
        }

        count
    }
}

impl Default for SnapshotTree {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tree Traversal
// ============================================================================

/// Tree traversal order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraversalOrder {
    /// Pre-order (parent before children)
    PreOrder,
    /// Post-order (children before parent)
    PostOrder,
    /// Breadth-first (level by level)
    BreadthFirst,
}

/// Tree traversal state.
pub struct TreeTraversal {
    /// Order
    pub order: TraversalOrder,
    /// Stack for DFS
    pub stack: [SnapshotId; 64],
    /// Stack length
    pub stack_len: usize,
    /// Queue for BFS
    pub queue: [SnapshotId; 64],
    /// Queue head
    pub queue_head: usize,
    /// Queue tail
    pub queue_tail: usize,
    /// Visited count
    pub visited: usize,
}

impl TreeTraversal {
    /// Create pre-order traversal starting at node
    pub fn pre_order(start: SnapshotId) -> Self {
        let mut t = Self {
            order: TraversalOrder::PreOrder,
            stack: [INVALID_SNAPSHOT_ID; 64],
            stack_len: 1,
            queue: [INVALID_SNAPSHOT_ID; 64],
            queue_head: 0,
            queue_tail: 0,
            visited: 0,
        };
        t.stack[0] = start;
        t
    }

    /// Create breadth-first traversal
    pub fn breadth_first(start: SnapshotId) -> Self {
        let mut t = Self {
            order: TraversalOrder::BreadthFirst,
            stack: [INVALID_SNAPSHOT_ID; 64],
            stack_len: 0,
            queue: [INVALID_SNAPSHOT_ID; 64],
            queue_head: 0,
            queue_tail: 1,
            visited: 0,
        };
        t.queue[0] = start;
        t
    }

    /// Check if done
    #[inline]
    pub fn is_done(&self) -> bool {
        match self.order {
            TraversalOrder::PreOrder | TraversalOrder::PostOrder => self.stack_len == 0,
            TraversalOrder::BreadthFirst => self.queue_head == self.queue_tail,
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
    fn test_snapshot_tree_node() {
        let node = SnapshotTreeNode::root();

        assert!(node.is_root());
        assert!(node.is_leaf());
        assert!(!node.has_children());
    }

    #[test]
    fn test_snapshot_tree() {
        let mut tree = SnapshotTree::new();

        // Should have root
        assert_eq!(tree.count, 1);
        assert!(tree.get(ROOT_SNAPSHOT_ID).is_some());

        // Add children
        tree.add_child(ROOT_SNAPSHOT_ID, 2).unwrap();
        tree.add_child(ROOT_SNAPSHOT_ID, 3).unwrap();
        tree.add_child(2, 4).unwrap();

        assert_eq!(tree.count, 4);
        assert_eq!(tree.child_count(ROOT_SNAPSHOT_ID), 2);
        assert_eq!(tree.depth(4), Some(2));
        assert_eq!(tree.max_depth, 2);
    }

    #[test]
    fn test_path_to_root() {
        let mut tree = SnapshotTree::new();

        tree.add_child(ROOT_SNAPSHOT_ID, 2).unwrap();
        tree.add_child(2, 3).unwrap();
        tree.add_child(3, 4).unwrap();

        let path = tree.path_to_root(4);
        assert_eq!(path.as_slice(), &[4, 3, 2, ROOT_SNAPSHOT_ID]);
    }

    #[test]
    fn test_common_ancestor() {
        let mut tree = SnapshotTree::new();

        //        1 (root)
        //       / \
        //      2   3
        //     / \
        //    4   5

        tree.add_child(ROOT_SNAPSHOT_ID, 2).unwrap();
        tree.add_child(ROOT_SNAPSHOT_ID, 3).unwrap();
        tree.add_child(2, 4).unwrap();
        tree.add_child(2, 5).unwrap();

        assert_eq!(tree.common_ancestor(4, 5), Some(2));
        assert_eq!(tree.common_ancestor(4, 3), Some(ROOT_SNAPSHOT_ID));
        assert_eq!(tree.common_ancestor(2, 3), Some(ROOT_SNAPSHOT_ID));
    }

    #[test]
    fn test_remove_node() {
        let mut tree = SnapshotTree::new();

        tree.add_child(ROOT_SNAPSHOT_ID, 2).unwrap();
        tree.add_child(ROOT_SNAPSHOT_ID, 3).unwrap();

        // Can't remove root
        assert!(tree.remove(ROOT_SNAPSHOT_ID).is_err());

        // Can remove leaf
        tree.remove(3).unwrap();
        assert_eq!(tree.count, 2);
        assert!(tree.get(3).is_none());
    }

    #[test]
    fn test_descendant_count() {
        let mut tree = SnapshotTree::new();

        tree.add_child(ROOT_SNAPSHOT_ID, 2).unwrap();
        tree.add_child(ROOT_SNAPSHOT_ID, 3).unwrap();
        tree.add_child(2, 4).unwrap();
        tree.add_child(2, 5).unwrap();

        assert_eq!(tree.descendant_count(ROOT_SNAPSHOT_ID), 4);
        assert_eq!(tree.descendant_count(2), 2);
        assert_eq!(tree.descendant_count(3), 0);
    }
}
