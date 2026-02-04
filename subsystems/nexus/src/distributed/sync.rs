//! # State Synchronization
//!
//! Year 3 EVOLUTION - Q4 - State synchronization across distributed nodes

#![allow(dead_code)]
#![allow(clippy::excessive_nesting)]
#![allow(clippy::redundant_closure)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{Epoch, NodeId};

// ============================================================================
// SYNC TYPES
// ============================================================================

/// Sync session ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SyncSessionId(pub u64);

/// Vector clock
#[derive(Debug, Clone, Default)]
pub struct VectorClock {
    /// Clocks per node
    clocks: BTreeMap<NodeId, u64>,
}

static SYNC_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

impl SyncSessionId {
    pub fn generate() -> Self {
        Self(SYNC_SESSION_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

impl VectorClock {
    /// Create new vector clock
    pub fn new() -> Self {
        Self {
            clocks: BTreeMap::new(),
        }
    }

    /// Increment clock for node
    pub fn increment(&mut self, node: NodeId) {
        *self.clocks.entry(node).or_insert(0) += 1;
    }

    /// Get clock for node
    pub fn get(&self, node: NodeId) -> u64 {
        self.clocks.get(&node).copied().unwrap_or(0)
    }

    /// Merge with another clock (take max)
    pub fn merge(&mut self, other: &VectorClock) {
        for (&node, &time) in &other.clocks {
            let current = self.clocks.entry(node).or_insert(0);
            *current = (*current).max(time);
        }
    }

    /// Check if this clock happens before other
    pub fn happens_before(&self, other: &VectorClock) -> bool {
        let mut at_least_one_less = false;

        for (&node, &time) in &self.clocks {
            let other_time = other.get(node);
            if time > other_time {
                return false;
            }
            if time < other_time {
                at_least_one_less = true;
            }
        }

        // Check other's clocks for nodes we don't have
        for (&node, &time) in &other.clocks {
            if !self.clocks.contains_key(&node) && time > 0 {
                at_least_one_less = true;
            }
        }

        at_least_one_less
    }

    /// Check if clocks are concurrent
    pub fn concurrent(&self, other: &VectorClock) -> bool {
        !self.happens_before(other) && !other.happens_before(self)
    }
}

// ============================================================================
// STATE VERSION
// ============================================================================

/// State version
#[derive(Debug, Clone)]
pub struct StateVersion {
    /// Version number
    pub version: u64,
    /// Epoch
    pub epoch: Epoch,
    /// Hash
    pub hash: u64,
    /// Vector clock
    pub clock: VectorClock,
    /// Last modified
    pub last_modified: u64,
}

/// State item
#[derive(Debug, Clone)]
pub struct StateItem {
    /// Key
    pub key: String,
    /// Value
    pub value: Vec<u8>,
    /// Version
    pub version: StateVersion,
    /// Tombstone (deleted)
    pub tombstone: bool,
}

/// State snapshot
#[derive(Debug, Clone)]
pub struct StateSnapshot {
    /// Epoch
    pub epoch: Epoch,
    /// Items
    pub items: Vec<StateItem>,
    /// Hash
    pub hash: u64,
    /// Timestamp
    pub timestamp: u64,
}

// ============================================================================
// SYNC PROTOCOL
// ============================================================================

/// Sync mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    /// Full sync
    Full,
    /// Incremental
    Incremental,
    /// Snapshot
    Snapshot,
    /// Anti-entropy
    AntiEntropy,
    /// Merkle tree
    MerkleTree,
}

/// Sync request
#[derive(Debug, Clone)]
pub struct SyncRequest {
    /// Session ID
    pub session_id: SyncSessionId,
    /// Requester
    pub requester: NodeId,
    /// Mode
    pub mode: SyncMode,
    /// From epoch
    pub from_epoch: Epoch,
    /// Vector clock
    pub clock: VectorClock,
    /// Keys to sync (None = all)
    pub keys: Option<Vec<String>>,
}

/// Sync response
#[derive(Debug, Clone)]
pub struct SyncResponse {
    /// Session ID
    pub session_id: SyncSessionId,
    /// Responder
    pub responder: NodeId,
    /// Items
    pub items: Vec<StateItem>,
    /// Current epoch
    pub current_epoch: Epoch,
    /// Current clock
    pub clock: VectorClock,
    /// Has more
    pub has_more: bool,
}

/// Sync delta
#[derive(Debug, Clone)]
pub struct SyncDelta {
    /// Items to add/update
    pub upsert: Vec<StateItem>,
    /// Keys to delete
    pub delete: Vec<String>,
    /// New epoch
    pub epoch: Epoch,
}

// ============================================================================
// MERKLE TREE
// ============================================================================

/// Merkle tree for efficient sync
pub struct MerkleTree {
    /// Nodes
    nodes: Vec<MerkleNode>,
    /// Leaf count
    leaf_count: usize,
}

/// Merkle node
#[derive(Debug, Clone)]
pub struct MerkleNode {
    /// Hash
    pub hash: u64,
    /// Children (if internal)
    pub children: Option<(usize, usize)>,
    /// Key (if leaf)
    pub key: Option<String>,
}

impl MerkleTree {
    /// Create from items
    pub fn from_items(items: &[StateItem]) -> Self {
        if items.is_empty() {
            return Self {
                nodes: Vec::new(),
                leaf_count: 0,
            };
        }

        let mut nodes = Vec::new();
        let hashes: Vec<u64> = items.iter().map(|item| Self::hash_item(item)).collect();

        // Create leaf nodes
        for (i, item) in items.iter().enumerate() {
            nodes.push(MerkleNode {
                hash: hashes[i],
                children: None,
                key: Some(item.key.clone()),
            });
        }

        let leaf_count = nodes.len();

        // Build tree bottom-up
        let mut current_level_start = 0;
        let mut current_level_size = leaf_count;

        while current_level_size > 1 {
            let next_level_start = nodes.len();
            let mut i = 0;

            while i < current_level_size {
                let left = current_level_start + i;
                let right = if i + 1 < current_level_size {
                    current_level_start + i + 1
                } else {
                    left // Duplicate for odd count
                };

                let combined_hash = Self::combine_hashes(nodes[left].hash, nodes[right].hash);

                nodes.push(MerkleNode {
                    hash: combined_hash,
                    children: Some((left, right)),
                    key: None,
                });

                i += 2;
            }

            current_level_start = next_level_start;
            current_level_size = (current_level_size + 1) / 2;
        }

        Self { nodes, leaf_count }
    }

    /// Get root hash
    pub fn root_hash(&self) -> u64 {
        self.nodes.last().map(|n| n.hash).unwrap_or(0)
    }

    /// Find differences
    pub fn diff(&self, other: &MerkleTree) -> Vec<String> {
        let mut diff_keys = Vec::new();

        if self.nodes.is_empty() || other.nodes.is_empty() {
            // All keys different
            for node in &self.nodes {
                if let Some(key) = &node.key {
                    diff_keys.push(key.clone());
                }
            }
            return diff_keys;
        }

        // Compare trees
        self.diff_recursive(
            self.nodes.len() - 1,
            other,
            other.nodes.len() - 1,
            &mut diff_keys,
        );

        diff_keys
    }

    fn diff_recursive(
        &self,
        self_idx: usize,
        other: &MerkleTree,
        other_idx: usize,
        diff: &mut Vec<String>,
    ) {
        if self_idx >= self.nodes.len() || other_idx >= other.nodes.len() {
            return;
        }

        let self_node = &self.nodes[self_idx];
        let other_node = &other.nodes[other_idx];

        if self_node.hash == other_node.hash {
            return; // Subtrees are identical
        }

        match (&self_node.children, &other_node.children) {
            (Some((l1, r1)), Some((l2, r2))) => {
                // Both internal nodes, recurse
                self.diff_recursive(*l1, other, *l2, diff);
                self.diff_recursive(*r1, other, *r2, diff);
            },
            (None, None) => {
                // Both leaves
                if let Some(key) = &self_node.key {
                    diff.push(key.clone());
                }
            },
            _ => {
                // Structure mismatch, add all leaves from self
                self.collect_leaves(self_idx, diff);
            },
        }
    }

    fn collect_leaves(&self, idx: usize, keys: &mut Vec<String>) {
        if idx >= self.nodes.len() {
            return;
        }

        let node = &self.nodes[idx];
        match &node.children {
            Some((left, right)) => {
                self.collect_leaves(*left, keys);
                self.collect_leaves(*right, keys);
            },
            None => {
                if let Some(key) = &node.key {
                    keys.push(key.clone());
                }
            },
        }
    }

    fn hash_item(item: &StateItem) -> u64 {
        let mut hash = 0u64;
        for b in item.key.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(b as u64);
        }
        for b in &item.value {
            hash = hash.wrapping_mul(31).wrapping_add(*b as u64);
        }
        hash = hash.wrapping_mul(31).wrapping_add(item.version.version);
        hash
    }

    fn combine_hashes(a: u64, b: u64) -> u64 {
        a.wrapping_mul(0x517cc1b727220a95).wrapping_add(b)
    }
}

// ============================================================================
// SYNC ENGINE
// ============================================================================

/// Sync engine
pub struct SyncEngine {
    /// Local node ID
    node_id: NodeId,
    /// State
    state: BTreeMap<String, StateItem>,
    /// Vector clock
    clock: VectorClock,
    /// Current epoch
    epoch: Epoch,
    /// Merkle tree
    merkle: Option<MerkleTree>,
    /// Pending syncs
    pending: BTreeMap<SyncSessionId, SyncSession>,
    /// Configuration
    config: SyncConfig,
    /// Statistics
    stats: SyncStats,
}

/// Sync session
#[derive(Debug)]
pub struct SyncSession {
    /// Session ID
    pub id: SyncSessionId,
    /// Peer
    pub peer: NodeId,
    /// Mode
    pub mode: SyncMode,
    /// State
    pub state: SyncSessionState,
    /// Items received
    pub items_received: usize,
    /// Started at
    pub started_at: u64,
}

/// Sync session state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncSessionState {
    /// Initiated
    Initiated,
    /// Receiving
    Receiving,
    /// Sending
    Sending,
    /// Complete
    Complete,
    /// Failed
    Failed,
}

/// Sync configuration
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Default mode
    pub default_mode: SyncMode,
    /// Batch size
    pub batch_size: usize,
    /// Sync interval (ms)
    pub sync_interval: u64,
    /// Timeout (ms)
    pub timeout: u64,
    /// Use merkle tree
    pub use_merkle: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            default_mode: SyncMode::Incremental,
            batch_size: 100,
            sync_interval: 5000,
            timeout: 30000,
            use_merkle: true,
        }
    }
}

/// Sync statistics
#[derive(Debug, Clone, Default)]
pub struct SyncStats {
    /// Syncs initiated
    pub syncs_initiated: u64,
    /// Syncs completed
    pub syncs_completed: u64,
    /// Syncs failed
    pub syncs_failed: u64,
    /// Items synced
    pub items_synced: u64,
    /// Bytes transferred
    pub bytes_transferred: u64,
}

impl SyncEngine {
    /// Create new sync engine
    pub fn new(node_id: NodeId, config: SyncConfig) -> Self {
        let mut clock = VectorClock::new();
        clock.increment(node_id);

        Self {
            node_id,
            state: BTreeMap::new(),
            clock,
            epoch: Epoch(0),
            merkle: None,
            pending: BTreeMap::new(),
            config,
            stats: SyncStats::default(),
        }
    }

    /// Put item
    pub fn put(&mut self, key: String, value: Vec<u8>) {
        self.clock.increment(self.node_id);

        let hash = Self::hash_value(&value);
        let item = StateItem {
            key: key.clone(),
            value,
            version: StateVersion {
                version: self.clock.get(self.node_id),
                epoch: self.epoch,
                hash,
                clock: self.clock.clone(),
                last_modified: 0,
            },
            tombstone: false,
        };

        self.state.insert(key, item);
        self.merkle = None; // Invalidate
    }

    /// Get item
    pub fn get(&self, key: &str) -> Option<&StateItem> {
        self.state.get(key).filter(|i| !i.tombstone)
    }

    /// Delete item
    pub fn delete(&mut self, key: &str) {
        if let Some(item) = self.state.get_mut(key) {
            self.clock.increment(self.node_id);
            item.tombstone = true;
            item.version.version = self.clock.get(self.node_id);
            item.version.clock = self.clock.clone();
            self.merkle = None;
        }
    }

    /// Initiate sync
    pub fn initiate_sync(&mut self, peer: NodeId, mode: SyncMode) -> SyncRequest {
        let session_id = SyncSessionId::generate();

        self.pending.insert(session_id, SyncSession {
            id: session_id,
            peer,
            mode,
            state: SyncSessionState::Initiated,
            items_received: 0,
            started_at: 0,
        });

        self.stats.syncs_initiated += 1;

        SyncRequest {
            session_id,
            requester: self.node_id,
            mode,
            from_epoch: self.epoch,
            clock: self.clock.clone(),
            keys: None,
        }
    }

    /// Handle sync request
    pub fn handle_request(&mut self, request: &SyncRequest) -> SyncResponse {
        let items: Vec<StateItem> = match request.mode {
            SyncMode::Full => self.state.values().cloned().collect(),
            SyncMode::Incremental => self
                .state
                .values()
                .filter(|item| {
                    item.version.clock.happens_before(&request.clock)
                        || item.version.clock.concurrent(&request.clock)
                })
                .cloned()
                .collect(),
            SyncMode::MerkleTree => {
                // Use Merkle diff
                let their_tree = MerkleTree::from_items(&[]);
                let our_tree = self.get_or_build_merkle();
                let diff_keys = our_tree.diff(&their_tree);

                diff_keys
                    .iter()
                    .filter_map(|k| self.state.get(k))
                    .cloned()
                    .collect()
            },
            _ => self.state.values().cloned().collect(),
        };

        self.stats.items_synced += items.len() as u64;

        SyncResponse {
            session_id: request.session_id,
            responder: self.node_id,
            items,
            current_epoch: self.epoch,
            clock: self.clock.clone(),
            has_more: false,
        }
    }

    /// Apply sync response
    pub fn apply_response(&mut self, response: &SyncResponse) -> SyncDelta {
        let mut upsert = Vec::new();
        let mut delete = Vec::new();

        for item in &response.items {
            match self.state.get(&item.key) {
                Some(existing) => {
                    // Compare versions
                    if item.version.clock.happens_before(&existing.version.clock) {
                        continue; // Our version is newer
                    }
                    if existing.version.clock.happens_before(&item.version.clock) {
                        // Their version is newer
                        if item.tombstone {
                            delete.push(item.key.clone());
                        } else {
                            upsert.push(item.clone());
                        }
                    } else if item.version.clock.concurrent(&existing.version.clock) {
                        // Conflict resolution: last writer wins (by version number)
                        if item.version.version > existing.version.version {
                            if item.tombstone {
                                delete.push(item.key.clone());
                            } else {
                                upsert.push(item.clone());
                            }
                        }
                    }
                },
                None => {
                    if !item.tombstone {
                        upsert.push(item.clone());
                    }
                },
            }
        }

        // Apply changes
        for item in &upsert {
            self.state.insert(item.key.clone(), item.clone());
        }
        for key in &delete {
            if let Some(item) = self.state.get_mut(key) {
                item.tombstone = true;
            }
        }

        // Merge clock
        self.clock.merge(&response.clock);
        self.merkle = None;

        // Update session
        if let Some(session) = self.pending.get_mut(&response.session_id) {
            session.items_received += response.items.len();
            if !response.has_more {
                session.state = SyncSessionState::Complete;
                self.stats.syncs_completed += 1;
            }
        }

        SyncDelta {
            upsert,
            delete,
            epoch: response.current_epoch,
        }
    }

    /// Get or build Merkle tree
    fn get_or_build_merkle(&mut self) -> &MerkleTree {
        if self.merkle.is_none() {
            let items: Vec<StateItem> = self.state.values().cloned().collect();
            self.merkle = Some(MerkleTree::from_items(&items));
        }
        self.merkle.as_ref().unwrap()
    }

    fn hash_value(value: &[u8]) -> u64 {
        let mut hash = 0u64;
        for b in value {
            hash = hash.wrapping_mul(31).wrapping_add(*b as u64);
        }
        hash
    }

    /// Get statistics
    pub fn stats(&self) -> &SyncStats {
        &self.stats
    }

    /// Get current epoch
    pub fn epoch(&self) -> Epoch {
        self.epoch
    }

    /// Get vector clock
    pub fn clock(&self) -> &VectorClock {
        &self.clock
    }
}

impl Default for SyncEngine {
    fn default() -> Self {
        Self::new(NodeId(0), SyncConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_clock() {
        let mut a = VectorClock::new();
        let mut b = VectorClock::new();

        a.increment(NodeId(1));
        a.increment(NodeId(1));

        b.increment(NodeId(2));

        assert!(!a.happens_before(&b));
        assert!(!b.happens_before(&a));
        assert!(a.concurrent(&b));

        b.merge(&a);
        assert!(a.happens_before(&b) || !a.concurrent(&b));
    }

    #[test]
    fn test_merkle_tree() {
        let items = vec![
            StateItem {
                key: String::from("a"),
                value: vec![1, 2, 3],
                version: StateVersion {
                    version: 1,
                    epoch: Epoch(0),
                    hash: 0,
                    clock: VectorClock::new(),
                    last_modified: 0,
                },
                tombstone: false,
            },
            StateItem {
                key: String::from("b"),
                value: vec![4, 5, 6],
                version: StateVersion {
                    version: 1,
                    epoch: Epoch(0),
                    hash: 0,
                    clock: VectorClock::new(),
                    last_modified: 0,
                },
                tombstone: false,
            },
        ];

        let tree = MerkleTree::from_items(&items);
        assert!(tree.root_hash() != 0);
    }

    #[test]
    fn test_sync_engine() {
        let mut engine = SyncEngine::new(NodeId(1), SyncConfig::default());

        engine.put(String::from("key1"), vec![1, 2, 3]);
        engine.put(String::from("key2"), vec![4, 5, 6]);

        assert!(engine.get("key1").is_some());
        assert!(engine.get("key2").is_some());

        engine.delete("key1");
        assert!(engine.get("key1").is_none());
    }
}
