//! # Coop Merkle Tree
//!
//! Merkle tree for efficient state verification:
//! - Hash-based integrity verification
//! - Efficient diff computation between trees
//! - Proof generation and verification
//! - Incremental tree updates
//! - Range proof support
//! - State synchronization via Merkle proofs

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Node type in Merkle tree
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MerkleNodeType {
    Leaf,
    Internal,
    Root,
}

/// Merkle tree node
#[derive(Debug, Clone)]
pub struct MerkleNode {
    pub id: u64,
    pub node_type: MerkleNodeType,
    pub hash: u64,
    pub left_child: Option<u64>,
    pub right_child: Option<u64>,
    pub parent: Option<u64>,
    pub depth: u32,
    pub key: Option<u64>,
    pub value_hash: Option<u64>,
    pub subtree_count: u32,
}

impl MerkleNode {
    #[inline]
    pub fn leaf(id: u64, key: u64, value_hash: u64, depth: u32) -> Self {
        let hash = Self::compute_leaf_hash(key, value_hash);
        Self {
            id, node_type: MerkleNodeType::Leaf, hash, left_child: None,
            right_child: None, parent: None, depth, key: Some(key),
            value_hash: Some(value_hash), subtree_count: 1,
        }
    }

    #[inline]
    pub fn internal(id: u64, left_hash: u64, right_hash: u64, depth: u32) -> Self {
        let hash = Self::compute_internal_hash(left_hash, right_hash);
        Self {
            id, node_type: MerkleNodeType::Internal, hash,
            left_child: None, right_child: None, parent: None,
            depth, key: None, value_hash: None, subtree_count: 0,
        }
    }

    fn compute_leaf_hash(key: u64, value: u64) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in &key.to_le_bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
        for &b in &value.to_le_bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
        h
    }

    fn compute_internal_hash(left: u64, right: u64) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in &left.to_le_bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
        for &b in &right.to_le_bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
        h
    }

    #[inline]
    pub fn rehash_leaf(&mut self) {
        if let (Some(k), Some(v)) = (self.key, self.value_hash) {
            self.hash = Self::compute_leaf_hash(k, v);
        }
    }
}

/// Merkle proof step
#[derive(Debug, Clone)]
pub struct ProofStep {
    pub hash: u64,
    pub is_left: bool,
}

/// Merkle proof
#[derive(Debug, Clone)]
pub struct MerkleProof {
    pub key: u64,
    pub value_hash: u64,
    pub steps: Vec<ProofStep>,
    pub root_hash: u64,
}

impl MerkleProof {
    #[inline]
    pub fn verify(&self) -> bool {
        let mut current = MerkleNode::compute_leaf_hash(self.key, self.value_hash);
        for step in &self.steps {
            if step.is_left {
                current = MerkleNode::compute_internal_hash(step.hash, current);
            } else {
                current = MerkleNode::compute_internal_hash(current, step.hash);
            }
        }
        current == self.root_hash
    }
}

/// Diff entry between two trees
#[derive(Debug, Clone)]
pub struct MerkleDiff {
    pub key: u64,
    pub diff_type: DiffType,
    pub local_hash: Option<u64>,
    pub remote_hash: Option<u64>,
}

/// Diff type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffType {
    Added,
    Removed,
    Modified,
}

/// Merkle tree stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct MerkleTreeStats {
    pub total_nodes: usize,
    pub leaf_nodes: usize,
    pub internal_nodes: usize,
    pub tree_depth: u32,
    pub root_hash: u64,
    pub proofs_generated: u64,
    pub proofs_verified: u64,
    pub diffs_computed: u64,
}

/// Coop Merkle tree
pub struct CoopMerkleTree {
    nodes: BTreeMap<u64, MerkleNode>,
    leaves: LinearMap<u64, 64>,  // key -> node_id
    root_id: Option<u64>,
    stats: MerkleTreeStats,
    next_id: u64,
}

impl CoopMerkleTree {
    pub fn new() -> Self {
        Self { nodes: BTreeMap::new(), leaves: BTreeMap::new(), root_id: None, stats: MerkleTreeStats::default(), next_id: 1 }
    }

    #[inline]
    pub fn insert(&mut self, key: u64, value_hash: u64) {
        let node_id = self.next_id; self.next_id += 1;
        let node = MerkleNode::leaf(node_id, key, value_hash, 0);
        self.nodes.insert(node_id, node);
        self.leaves.insert(key, node_id);
        self.rebuild();
    }

    #[inline]
    pub fn update(&mut self, key: u64, new_value_hash: u64) {
        if let Some(&node_id) = self.leaves.get(key) {
            if let Some(node) = self.nodes.get_mut(&node_id) {
                node.value_hash = Some(new_value_hash);
                node.rehash_leaf();
            }
            self.rebuild();
        }
    }

    #[inline]
    pub fn remove(&mut self, key: u64) {
        if let Some(node_id) = self.leaves.remove(key) {
            self.nodes.remove(&node_id);
            self.rebuild();
        }
    }

    fn rebuild(&mut self) {
        // Remove all internal nodes
        let leaf_ids: Vec<u64> = self.leaves.values().copied().collect();
        self.nodes.retain(|id, _| leaf_ids.contains(id));

        if leaf_ids.is_empty() { self.root_id = None; return; }
        if leaf_ids.len() == 1 { self.root_id = Some(leaf_ids[0]); return; }

        let mut current_level: Vec<u64> = leaf_ids;
        let mut depth = 0u32;

        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            let mut i = 0;
            while i < current_level.len() {
                let left = current_level[i];
                let left_hash = self.nodes.get(&left).map(|n| n.hash).unwrap_or(0);
                if i + 1 < current_level.len() {
                    let right = current_level[i + 1];
                    let right_hash = self.nodes.get(&right).map(|n| n.hash).unwrap_or(0);
                    let parent_id = self.next_id; self.next_id += 1;
                    let mut parent = MerkleNode::internal(parent_id, left_hash, right_hash, depth + 1);
                    parent.left_child = Some(left);
                    parent.right_child = Some(right);
                    parent.subtree_count = self.nodes.get(&left).map(|n| n.subtree_count).unwrap_or(0)
                        + self.nodes.get(&right).map(|n| n.subtree_count).unwrap_or(0);
                    self.nodes.insert(parent_id, parent);
                    if let Some(n) = self.nodes.get_mut(&left) { n.parent = Some(parent_id); }
                    if let Some(n) = self.nodes.get_mut(&right) { n.parent = Some(parent_id); }
                    next_level.push(parent_id);
                    i += 2;
                } else {
                    // Odd node, promote directly
                    next_level.push(left);
                    i += 1;
                }
            }
            current_level = next_level;
            depth += 1;
        }
        self.root_id = current_level.first().copied();
    }

    #[inline(always)]
    pub fn root_hash(&self) -> u64 {
        self.root_id.and_then(|id| self.nodes.get(&id)).map(|n| n.hash).unwrap_or(0)
    }

    pub fn generate_proof(&mut self, key: u64) -> Option<MerkleProof> {
        let &node_id = self.leaves.get(key)?;
        let node = self.nodes.get(&node_id)?;
        let value_hash = node.value_hash?;
        let mut steps = Vec::new();
        let mut current = node_id;

        while let Some(parent_id) = self.nodes.get(&current).and_then(|n| n.parent) {
            let parent = self.nodes.get(&parent_id)?;
            if parent.left_child == Some(current) {
                if let Some(right) = parent.right_child {
                    let rh = self.nodes.get(&right).map(|n| n.hash).unwrap_or(0);
                    steps.push(ProofStep { hash: rh, is_left: false });
                }
            } else {
                if let Some(left) = parent.left_child {
                    let lh = self.nodes.get(&left).map(|n| n.hash).unwrap_or(0);
                    steps.push(ProofStep { hash: lh, is_left: true });
                }
            }
            current = parent_id;
        }

        self.stats.proofs_generated += 1;
        Some(MerkleProof { key, value_hash, steps, root_hash: self.root_hash() })
    }

    #[inline(always)]
    pub fn verify_proof(&mut self, proof: &MerkleProof) -> bool {
        self.stats.proofs_verified += 1;
        proof.verify()
    }

    pub fn diff(&mut self, other_leaves: &BTreeMap<u64, u64>) -> Vec<MerkleDiff> {
        self.stats.diffs_computed += 1;
        let mut diffs = Vec::new();

        for (&key, &local_node_id) in &self.leaves {
            let local_hash = self.nodes.get(&local_node_id).and_then(|n| n.value_hash);
            if let Some(&remote_hash) = other_leaves.get(&key) {
                if local_hash != Some(remote_hash) {
                    diffs.push(MerkleDiff { key, diff_type: DiffType::Modified, local_hash, remote_hash: Some(remote_hash) });
                }
            } else {
                diffs.push(MerkleDiff { key, diff_type: DiffType::Removed, local_hash, remote_hash: None });
            }
        }
        for (&key, &hash) in other_leaves {
            if !self.leaves.contains_key(key) {
                diffs.push(MerkleDiff { key, diff_type: DiffType::Added, local_hash: None, remote_hash: Some(hash) });
            }
        }
        diffs
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_nodes = self.nodes.len();
        self.stats.leaf_nodes = self.leaves.len();
        self.stats.internal_nodes = self.stats.total_nodes - self.stats.leaf_nodes;
        self.stats.tree_depth = self.nodes.values().map(|n| n.depth).max().unwrap_or(0);
        self.stats.root_hash = self.root_hash();
    }

    #[inline(always)]
    pub fn stats(&self) -> &MerkleTreeStats { &self.stats }
}
