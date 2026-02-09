//! # Data Sharding
//!
//! Year 3 EVOLUTION - Q4 - Partition and shard evolved code across nodes

#![allow(dead_code)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::Ordering as CmpOrdering;
use core::sync::atomic::{AtomicU64, Ordering};

use super::NodeId;

// ============================================================================
// SHARDING TYPES
// ============================================================================

/// Shard ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShardId(pub u64);

/// Partition ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartitionId(pub u64);

static SHARD_COUNTER: AtomicU64 = AtomicU64::new(1);
static PARTITION_COUNTER: AtomicU64 = AtomicU64::new(1);

impl ShardId {
    #[inline(always)]
    pub fn generate() -> Self {
        Self(SHARD_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

impl PartitionId {
    #[inline(always)]
    pub fn generate() -> Self {
        Self(PARTITION_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Shard
#[derive(Debug, Clone)]
pub struct Shard {
    /// Shard ID
    pub id: ShardId,
    /// Partition range (start, end)
    pub range: (u64, u64),
    /// Primary node
    pub primary: NodeId,
    /// Replicas
    pub replicas: VecDeque<NodeId>,
    /// Status
    pub status: ShardStatus,
    /// Items count
    pub items_count: u64,
    /// Size (bytes)
    pub size: usize,
}

/// Shard status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShardStatus {
    /// Active
    Active,
    /// Splitting
    Splitting,
    /// Merging
    Merging,
    /// Migrating
    Migrating,
    /// Recovering
    Recovering,
    /// Inactive
    Inactive,
}

/// Partition
#[derive(Debug, Clone)]
pub struct Partition {
    /// Partition ID
    pub id: PartitionId,
    /// Key
    pub key: String,
    /// Shard ID
    pub shard_id: ShardId,
    /// Hash
    pub hash: u64,
}

// ============================================================================
// SHARDING STRATEGY
// ============================================================================

/// Sharding strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShardingStrategy {
    /// Hash-based
    Hash,
    /// Range-based
    Range,
    /// Consistent hashing
    ConsistentHash,
    /// Directory-based
    Directory,
}

/// Hash function
pub trait HashFunction: Send + Sync {
    /// Hash key
    fn hash(&self, key: &str) -> u64;

    /// Name
    fn name(&self) -> &str;
}

/// Default hash function (FNV-1a)
pub struct FnvHash;

impl HashFunction for FnvHash {
    fn hash(&self, key: &str) -> u64 {
        const FNV_OFFSET: u64 = 0xcbf29ce484222325;
        const FNV_PRIME: u64 = 0x100000001b3;

        let mut hash = FNV_OFFSET;
        for byte in key.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    }

    fn name(&self) -> &str {
        "FNV-1a"
    }
}

// ============================================================================
// CONSISTENT HASHING
// ============================================================================

/// Consistent hash ring
pub struct ConsistentHashRing {
    /// Ring (hash -> node)
    ring: BTreeMap<u64, NodeId>,
    /// Virtual nodes per node
    virtual_nodes: u32,
    /// Hash function
    hash_fn: Box<dyn HashFunction>,
}

impl ConsistentHashRing {
    /// Create new ring
    pub fn new(virtual_nodes: u32) -> Self {
        Self {
            ring: BTreeMap::new(),
            virtual_nodes,
            hash_fn: Box::new(FnvHash),
        }
    }

    /// Add node
    #[inline]
    pub fn add_node(&mut self, node: NodeId) {
        for i in 0..self.virtual_nodes {
            let key = format!("{}:{}", node.0, i);
            let hash = self.hash_fn.hash(&key);
            self.ring.insert(hash, node);
        }
    }

    /// Remove node
    pub fn remove_node(&mut self, node: NodeId) {
        let keys_to_remove: Vec<_> = self
            .ring
            .iter()
            .filter(|&(_, n)| *n == node)
            .map(|(&k, _)| k)
            .collect();

        for key in keys_to_remove {
            self.ring.remove(&key);
        }
    }

    /// Get node for key
    pub fn get_node(&self, key: &str) -> Option<NodeId> {
        if self.ring.is_empty() {
            return None;
        }

        let hash = self.hash_fn.hash(key);

        // Find first node with hash >= key hash
        self.ring.range(hash..).next().map(|(_, &n)| n).or_else(|| {
            // Wrap around to first node
            self.ring.values().next().copied()
        })
    }

    /// Get N nodes for key (for replication)
    pub fn get_nodes(&self, key: &str, count: usize) -> Vec<NodeId> {
        if self.ring.is_empty() {
            return Vec::new();
        }

        let hash = self.hash_fn.hash(key);
        let mut nodes = Vec::new();
        let mut seen = Vec::new();

        // Collect unique nodes
        for (_, &node) in self.ring.range(hash..).chain(self.ring.iter()) {
            if !seen.contains(&node) {
                seen.push(node);
                nodes.push(node);
                if nodes.len() >= count {
                    break;
                }
            }
        }

        nodes
    }

    /// Get all nodes
    #[inline]
    pub fn nodes(&self) -> Vec<NodeId> {
        let mut nodes: Vec<NodeId> = self.ring.values().copied().collect();
        nodes.sort();
        nodes.dedup();
        nodes
    }
}

impl Default for ConsistentHashRing {
    fn default() -> Self {
        Self::new(150)
    }
}

// ============================================================================
// SHARD MANAGER
// ============================================================================

/// Shard manager
pub struct ShardManager {
    /// Local node ID
    node_id: NodeId,
    /// Shards
    shards: BTreeMap<ShardId, Shard>,
    /// Partitions
    partitions: BTreeMap<String, Partition>,
    /// Consistent hash ring
    ring: ConsistentHashRing,
    /// Strategy
    strategy: ShardingStrategy,
    /// Configuration
    config: ShardConfig,
    /// Statistics
    stats: ShardStats,
}

/// Shard configuration
#[derive(Debug, Clone)]
pub struct ShardConfig {
    /// Number of shards
    pub num_shards: usize,
    /// Replication factor
    pub replication_factor: usize,
    /// Max shard size
    pub max_shard_size: usize,
    /// Auto-rebalance
    pub auto_rebalance: bool,
    /// Split threshold
    pub split_threshold: usize,
    /// Merge threshold
    pub merge_threshold: usize,
}

impl Default for ShardConfig {
    fn default() -> Self {
        Self {
            num_shards: 16,
            replication_factor: 3,
            max_shard_size: 1024 * 1024 * 1024, // 1GB
            auto_rebalance: true,
            split_threshold: 100000,
            merge_threshold: 1000,
        }
    }
}

/// Shard statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct ShardStats {
    /// Total shards
    pub total_shards: u64,
    /// Active shards
    pub active_shards: u64,
    /// Total items
    pub total_items: u64,
    /// Splits performed
    pub splits: u64,
    /// Merges performed
    pub merges: u64,
    /// Migrations performed
    pub migrations: u64,
}

impl ShardManager {
    /// Create new shard manager
    pub fn new(node_id: NodeId, config: ShardConfig) -> Self {
        Self {
            node_id,
            shards: BTreeMap::new(),
            partitions: BTreeMap::new(),
            ring: ConsistentHashRing::new(150),
            strategy: ShardingStrategy::ConsistentHash,
            config,
            stats: ShardStats::default(),
        }
    }

    /// Initialize shards
    pub fn initialize(&mut self, nodes: &[NodeId]) {
        // Add nodes to ring
        for &node in nodes {
            self.ring.add_node(node);
        }

        // Create initial shards
        let range_size = u64::MAX / self.config.num_shards as u64;

        for i in 0..self.config.num_shards {
            let start = i as u64 * range_size;
            let end = if i == self.config.num_shards - 1 {
                u64::MAX
            } else {
                (i as u64 + 1) * range_size - 1
            };

            // Select nodes for this shard
            let primary = nodes[i % nodes.len()];
            let mut replicas = Vec::new();
            for j in 1..self.config.replication_factor.min(nodes.len()) {
                replicas.push(nodes[(i + j) % nodes.len()]);
            }

            let shard = Shard {
                id: ShardId::generate(),
                range: (start, end),
                primary,
                replicas,
                status: ShardStatus::Active,
                items_count: 0,
                size: 0,
            };

            self.shards.insert(shard.id, shard);
            self.stats.total_shards += 1;
            self.stats.active_shards += 1;
        }
    }

    /// Get shard for key
    pub fn get_shard(&self, key: &str) -> Option<&Shard> {
        match self.strategy {
            ShardingStrategy::ConsistentHash => {
                let node = self.ring.get_node(key)?;
                self.shards.values().find(|s| s.primary == node)
            },
            ShardingStrategy::Hash => {
                let hash = FnvHash.hash(key);
                self.shards
                    .values()
                    .find(|s| hash >= s.range.0 && hash <= s.range.1)
            },
            ShardingStrategy::Range => {
                let hash = FnvHash.hash(key);
                self.shards
                    .values()
                    .find(|s| hash >= s.range.0 && hash <= s.range.1)
            },
            ShardingStrategy::Directory => self
                .partitions
                .get(key)
                .and_then(|p| self.shards.get(&p.shard_id)),
        }
    }

    /// Put key
    pub fn put(&mut self, key: String, size: usize) -> Option<ShardId> {
        let hash = FnvHash.hash(&key);

        // Find shard
        let shard_id = self
            .shards
            .values()
            .find(|s| hash >= s.range.0 && hash <= s.range.1)
            .map(|s| s.id)?;

        // Update shard stats
        if let Some(shard) = self.shards.get_mut(&shard_id) {
            shard.items_count += 1;
            shard.size += size;
            self.stats.total_items += 1;

            // Check split threshold
            if self.config.auto_rebalance && shard.items_count > self.config.split_threshold as u64
            {
                // Would trigger split
            }
        }

        // Create partition entry
        self.partitions.insert(key.clone(), Partition {
            id: PartitionId::generate(),
            key,
            shard_id,
            hash,
        });

        Some(shard_id)
    }

    /// Split shard
    pub fn split(&mut self, shard_id: ShardId) -> Result<(ShardId, ShardId), ShardError> {
        let shard = self.shards.get_mut(&shard_id).ok_or(ShardError::NotFound)?;

        if shard.status != ShardStatus::Active {
            return Err(ShardError::InvalidState);
        }

        shard.status = ShardStatus::Splitting;

        // Calculate split point
        let (start, end) = shard.range;
        let mid = start + (end - start) / 2;

        // Create two new shards
        let new_shard1 = Shard {
            id: ShardId::generate(),
            range: (start, mid),
            primary: shard.primary,
            replicas: shard.replicas.clone(),
            status: ShardStatus::Active,
            items_count: shard.items_count / 2,
            size: shard.size / 2,
        };

        let new_shard2 = Shard {
            id: ShardId::generate(),
            range: (mid + 1, end),
            primary: shard.primary,
            replicas: shard.replicas.clone(),
            status: ShardStatus::Active,
            items_count: shard.items_count / 2,
            size: shard.size / 2,
        };

        let id1 = new_shard1.id;
        let id2 = new_shard2.id;

        self.shards.insert(id1, new_shard1);
        self.shards.insert(id2, new_shard2);

        // Remove old shard
        self.shards.remove(&shard_id);

        // Update partitions
        for partition in self.partitions.values_mut() {
            if partition.shard_id == shard_id {
                if partition.hash <= mid {
                    partition.shard_id = id1;
                } else {
                    partition.shard_id = id2;
                }
            }
        }

        self.stats.splits += 1;
        self.stats.total_shards += 1;

        Ok((id1, id2))
    }

    /// Merge shards
    pub fn merge(&mut self, shard1_id: ShardId, shard2_id: ShardId) -> Result<ShardId, ShardError> {
        let shard1 = self
            .shards
            .get(&shard1_id)
            .ok_or(ShardError::NotFound)?
            .clone();
        let shard2 = self
            .shards
            .get(&shard2_id)
            .ok_or(ShardError::NotFound)?
            .clone();

        // Check if adjacent
        if shard1.range.1 + 1 != shard2.range.0 && shard2.range.1 + 1 != shard1.range.0 {
            return Err(ShardError::NotAdjacent);
        }

        // Create merged shard
        let new_range = (
            shard1.range.0.min(shard2.range.0),
            shard1.range.1.max(shard2.range.1),
        );

        let new_shard = Shard {
            id: ShardId::generate(),
            range: new_range,
            primary: shard1.primary,
            replicas: shard1.replicas.clone(),
            status: ShardStatus::Active,
            items_count: shard1.items_count + shard2.items_count,
            size: shard1.size + shard2.size,
        };

        let new_id = new_shard.id;
        self.shards.insert(new_id, new_shard);

        // Remove old shards
        self.shards.remove(&shard1_id);
        self.shards.remove(&shard2_id);

        // Update partitions
        for partition in self.partitions.values_mut() {
            if partition.shard_id == shard1_id || partition.shard_id == shard2_id {
                partition.shard_id = new_id;
            }
        }

        self.stats.merges += 1;
        self.stats.total_shards -= 1;

        Ok(new_id)
    }

    /// Rebalance shards
    pub fn rebalance(&mut self, nodes: &[NodeId]) {
        let shard_count = self.shards.len();
        let node_count = nodes.len();

        if node_count == 0 || shard_count == 0 {
            return;
        }

        let target_per_node = shard_count.div_ceil(node_count);
        let mut node_shards: BTreeMap<NodeId, Vec<ShardId>> = BTreeMap::new();

        // Count shards per node
        for shard in self.shards.values() {
            node_shards.entry(shard.primary).or_default().push(shard.id);
        }

        // Find overloaded and underloaded nodes
        let mut overloaded: Vec<(NodeId, Vec<ShardId>)> = Vec::new();
        let mut underloaded: Vec<NodeId> = Vec::new();

        for &node in nodes {
            let count = node_shards.get(&node).map(|v| v.len()).unwrap_or(0);
            match count.cmp(&target_per_node) {
                CmpOrdering::Greater => {
                    if let Some(shards) = node_shards.get(&node) {
                        let excess: Vec<_> = shards.iter().skip(target_per_node).copied().collect();
                        overloaded.push((node, excess));
                    }
                },
                CmpOrdering::Less => {
                    underloaded.push(node);
                },
                CmpOrdering::Equal => {},
            }
        }

        // Redistribute
        for (_, excess_shards) in overloaded {
            for shard_id in excess_shards {
                if let Some(target_node) = underloaded.pop() {
                    if let Some(shard) = self.shards.get_mut(&shard_id) {
                        shard.primary = target_node;
                        shard.status = ShardStatus::Migrating;
                        self.stats.migrations += 1;
                    }
                }
            }
        }
    }

    /// Add node
    #[inline(always)]
    pub fn add_node(&mut self, node: NodeId) {
        self.ring.add_node(node);
    }

    /// Remove node
    pub fn remove_node(&mut self, node: NodeId) {
        self.ring.remove_node(node);

        // Reassign shards from removed node
        let affected: Vec<ShardId> = self
            .shards
            .values()
            .filter(|s| s.primary == node)
            .map(|s| s.id)
            .collect();

        for shard_id in affected {
            if let Some(shard) = self.shards.get_mut(&shard_id) {
                // Promote first replica
                if let Some(new_primary) = shard.replicas.first().copied() {
                    shard.primary = new_primary;
                    shard.replicas.pop_front().unwrap();
                }
            }
        }
    }

    /// Get shard
    #[inline(always)]
    pub fn get(&self, id: ShardId) -> Option<&Shard> {
        self.shards.get(&id)
    }

    /// Get all shards
    #[inline(always)]
    pub fn all_shards(&self) -> impl Iterator<Item = &Shard> {
        self.shards.values()
    }

    /// Get local shards
    #[inline]
    pub fn local_shards(&self) -> impl Iterator<Item = &Shard> {
        self.shards
            .values()
            .filter(move |s| s.primary == self.node_id)
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &ShardStats {
        &self.stats
    }
}

impl Default for ShardManager {
    fn default() -> Self {
        Self::new(NodeId(0), ShardConfig::default())
    }
}

/// Shard error
#[derive(Debug)]
pub enum ShardError {
    /// Not found
    NotFound,
    /// Invalid state
    InvalidState,
    /// Shards not adjacent
    NotAdjacent,
    /// Capacity exceeded
    CapacityExceeded,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn test_consistent_hash_ring() {
        let mut ring = ConsistentHashRing::new(100);

        ring.add_node(NodeId(1));
        ring.add_node(NodeId(2));
        ring.add_node(NodeId(3));

        let node = ring.get_node("test-key");
        assert!(node.is_some());

        // Same key should always return same node
        let node2 = ring.get_node("test-key");
        assert_eq!(node, node2);
    }

    #[test]
    fn test_consistent_hash_distribution() {
        let mut ring = ConsistentHashRing::new(150);

        for i in 1..=5 {
            ring.add_node(NodeId(i));
        }

        let mut distribution: BTreeMap<NodeId, u32> = BTreeMap::new();

        for i in 0..1000 {
            let key = format!("key-{}", i);
            if let Some(node) = ring.get_node(&key) {
                *distribution.entry(node).or_insert(0) += 1;
            }
        }

        // Should have roughly even distribution
        for (_, count) in &distribution {
            assert!(*count > 100 && *count < 300);
        }
    }

    #[test]
    fn test_shard_manager() {
        let mut manager = ShardManager::new(NodeId(1), ShardConfig {
            num_shards: 4,
            ..Default::default()
        });

        let nodes = vec![NodeId(1), NodeId(2), NodeId(3)];
        manager.initialize(&nodes);

        assert_eq!(manager.stats.total_shards, 4);

        // Put some keys
        manager.put(String::from("key1"), 100);
        manager.put(String::from("key2"), 100);

        assert_eq!(manager.stats.total_items, 2);
    }

    #[test]
    fn test_shard_split() {
        let mut manager = ShardManager::new(NodeId(1), ShardConfig {
            num_shards: 1,
            ..Default::default()
        });

        manager.initialize(&[NodeId(1)]);

        let shard_id = *manager.shards.keys().next().unwrap();
        let result = manager.split(shard_id);

        assert!(result.is_ok());
        assert_eq!(manager.shards.len(), 2);
    }

    #[test]
    fn test_fnv_hash() {
        let hash = FnvHash;

        let h1 = hash.hash("hello");
        let h2 = hash.hash("world");
        let h3 = hash.hash("hello");

        assert_ne!(h1, h2);
        assert_eq!(h1, h3);
    }
}
