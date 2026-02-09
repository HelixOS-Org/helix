//! # Coop Sharding
//!
//! Cooperative data sharding and partitioning:
//! - Consistent hashing with virtual nodes
//! - Range-based and hash-based partitioning
//! - Shard migration and rebalancing
//! - Hot shard detection and splitting
//! - Shard placement constraints
//! - Load-aware shard assignment

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Sharding strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShardStrategy {
    HashMod,
    ConsistentHash,
    RangeBased,
    RoundRobin,
}

/// Shard state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShardState {
    Active,
    Migrating,
    Splitting,
    Merging,
    Draining,
    Offline,
}

/// Virtual node for consistent hashing
#[derive(Debug, Clone)]
pub struct VirtualNode {
    pub hash_point: u64,
    pub physical_node: u64,
    pub shard_id: u32,
}

/// Shard definition
#[derive(Debug, Clone)]
pub struct Shard {
    pub shard_id: u32,
    pub state: ShardState,
    pub assigned_node: u64,
    pub key_count: u64,
    pub data_bytes: u64,
    pub read_ops: u64,
    pub write_ops: u64,
    pub range_start: Option<u64>,
    pub range_end: Option<u64>,
    pub created_ts: u64,
    pub last_access_ts: u64,
}

impl Shard {
    pub fn new(id: u32, node: u64, ts: u64) -> Self {
        Self {
            shard_id: id, state: ShardState::Active, assigned_node: node,
            key_count: 0, data_bytes: 0, read_ops: 0, write_ops: 0,
            range_start: None, range_end: None, created_ts: ts, last_access_ts: ts,
        }
    }

    #[inline(always)]
    pub fn total_ops(&self) -> u64 { self.read_ops + self.write_ops }

    #[inline]
    pub fn load_score(&self) -> f64 {
        let ops = self.total_ops() as f64;
        let size = self.data_bytes as f64 / (1024.0 * 1024.0);
        ops * 0.7 + size * 0.3
    }

    #[inline]
    pub fn contains_range(&self, key: u64) -> bool {
        match (self.range_start, self.range_end) {
            (Some(s), Some(e)) => key >= s && key < e,
            _ => false,
        }
    }

    #[inline(always)]
    pub fn record_read(&mut self, ts: u64) { self.read_ops += 1; self.last_access_ts = ts; }
    #[inline(always)]
    pub fn record_write(&mut self, bytes: u64, ts: u64) {
        self.write_ops += 1; self.data_bytes += bytes; self.key_count += 1; self.last_access_ts = ts;
    }
}

/// Node info for shard placement
#[derive(Debug, Clone)]
pub struct ShardNode {
    pub node_id: u64,
    pub capacity_score: f64,
    pub shards_assigned: Vec<u32>,
    pub is_available: bool,
    pub zone: String,
}

impl ShardNode {
    pub fn new(id: u64, capacity: f64, zone: String) -> Self {
        Self { node_id: id, capacity_score: capacity, shards_assigned: Vec::new(), is_available: true, zone }
    }

    #[inline]
    pub fn load_per_capacity(&self, shards: &BTreeMap<u32, Shard>) -> f64 {
        if self.capacity_score <= 0.0 { return f64::MAX; }
        let total_load: f64 = self.shards_assigned.iter()
            .filter_map(|sid| shards.get(sid))
            .map(|s| s.load_score())
            .sum();
        total_load / self.capacity_score
    }
}

/// Migration record
#[derive(Debug, Clone)]
pub struct ShardMigration {
    pub migration_id: u64,
    pub shard_id: u32,
    pub from_node: u64,
    pub to_node: u64,
    pub keys_migrated: u64,
    pub bytes_migrated: u64,
    pub started_ts: u64,
    pub completed_ts: Option<u64>,
}

/// Hot shard threshold
#[derive(Debug, Clone)]
pub struct HotShardConfig {
    pub ops_threshold: u64,
    pub load_multiplier: f64,
}

impl Default for HotShardConfig {
    fn default() -> Self { Self { ops_threshold: 10_000, load_multiplier: 3.0 } }
}

/// Sharding stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct ShardingStats {
    pub total_shards: usize,
    pub active_shards: usize,
    pub total_nodes: usize,
    pub total_keys: u64,
    pub total_bytes: u64,
    pub total_migrations: usize,
    pub hot_shards: usize,
    pub avg_load_score: f64,
    pub load_imbalance: f64,
}

/// Cooperative sharding manager
pub struct CoopSharding {
    shards: BTreeMap<u32, Shard>,
    nodes: BTreeMap<u64, ShardNode>,
    ring: Vec<VirtualNode>,
    migrations: Vec<ShardMigration>,
    strategy: ShardStrategy,
    hot_config: HotShardConfig,
    next_shard_id: u32,
    next_migration_id: u64,
    vnodes_per_node: usize,
    stats: ShardingStats,
}

impl CoopSharding {
    pub fn new(strategy: ShardStrategy) -> Self {
        Self {
            shards: BTreeMap::new(), nodes: BTreeMap::new(),
            ring: Vec::new(), migrations: Vec::new(),
            strategy, hot_config: HotShardConfig::default(),
            next_shard_id: 1, next_migration_id: 1,
            vnodes_per_node: 150, stats: ShardingStats::default(),
        }
    }

    fn fnv_hash(key: u64) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in key.to_le_bytes() { hash ^= b as u64; hash = hash.wrapping_mul(0x100000001b3); }
        hash
    }

    #[inline(always)]
    pub fn add_node(&mut self, id: u64, capacity: f64, zone: String) {
        self.nodes.insert(id, ShardNode::new(id, capacity, zone));
        self.rebuild_ring();
    }

    #[inline(always)]
    pub fn remove_node(&mut self, id: u64) {
        self.nodes.remove(&id);
        self.rebuild_ring();
    }

    #[inline]
    pub fn create_shard(&mut self, node_id: u64, ts: u64) -> Option<u32> {
        if !self.nodes.contains_key(&node_id) { return None; }
        let id = self.next_shard_id; self.next_shard_id += 1;
        self.shards.insert(id, Shard::new(id, node_id, ts));
        if let Some(n) = self.nodes.get_mut(&node_id) { n.shards_assigned.push(id); }
        Some(id)
    }

    fn rebuild_ring(&mut self) {
        self.ring.clear();
        for (_, node) in &self.nodes {
            if !node.is_available { continue; }
            for i in 0..self.vnodes_per_node {
                let hash_input = node.node_id.wrapping_mul(1000).wrapping_add(i as u64);
                let hash = Self::fnv_hash(hash_input);
                if let Some(&sid) = node.shards_assigned.first() {
                    self.ring.push(VirtualNode { hash_point: hash, physical_node: node.node_id, shard_id: sid });
                }
            }
        }
        self.ring.sort_by_key(|vn| vn.hash_point);
    }

    pub fn lookup(&self, key: u64) -> Option<u32> {
        match self.strategy {
            ShardStrategy::ConsistentHash => {
                if self.ring.is_empty() { return None; }
                let hash = Self::fnv_hash(key);
                let idx = match self.ring.binary_search_by_key(&hash, |vn| vn.hash_point) {
                    Ok(i) => i,
                    Err(i) => if i >= self.ring.len() { 0 } else { i },
                };
                Some(self.ring[idx].shard_id)
            }
            ShardStrategy::HashMod => {
                if self.shards.is_empty() { return None; }
                let hash = Self::fnv_hash(key);
                let shard_count = self.shards.len() as u64;
                let idx = (hash % shard_count) as u32;
                self.shards.keys().nth(idx as usize).copied()
            }
            ShardStrategy::RangeBased => {
                self.shards.values().find(|s| s.contains_range(key)).map(|s| s.shard_id)
            }
            ShardStrategy::RoundRobin => {
                if self.shards.is_empty() { return None; }
                let idx = key as usize % self.shards.len();
                self.shards.keys().nth(idx).copied()
            }
        }
    }

    #[inline]
    pub fn detect_hot_shards(&self) -> Vec<u32> {
        if self.shards.is_empty() { return Vec::new(); }
        let avg_ops: f64 = self.shards.values().map(|s| s.total_ops() as f64).sum::<f64>() / self.shards.len() as f64;
        self.shards.values()
            .filter(|s| s.total_ops() as f64 > avg_ops * self.hot_config.load_multiplier
                        || s.total_ops() > self.hot_config.ops_threshold)
            .map(|s| s.shard_id)
            .collect()
    }

    pub fn begin_migration(&mut self, shard_id: u32, to_node: u64, ts: u64) -> Option<u64> {
        let shard = self.shards.get_mut(&shard_id)?;
        if shard.state != ShardState::Active { return None; }
        let from_node = shard.assigned_node;
        shard.state = ShardState::Migrating;
        let mid = self.next_migration_id; self.next_migration_id += 1;
        self.migrations.push(ShardMigration {
            migration_id: mid, shard_id, from_node, to_node,
            keys_migrated: 0, bytes_migrated: 0, started_ts: ts, completed_ts: None,
        });
        Some(mid)
    }

    pub fn complete_migration(&mut self, migration_id: u64, ts: u64) {
        if let Some(m) = self.migrations.iter_mut().find(|m| m.migration_id == migration_id) {
            m.completed_ts = Some(ts);
            let shard_id = m.shard_id;
            let from = m.from_node;
            let to = m.to_node;
            if let Some(s) = self.shards.get_mut(&shard_id) {
                s.state = ShardState::Active;
                s.assigned_node = to;
            }
            if let Some(n) = self.nodes.get_mut(&from) { n.shards_assigned.retain(|&s| s != shard_id); }
            if let Some(n) = self.nodes.get_mut(&to) { n.shards_assigned.push(shard_id); }
            self.rebuild_ring();
        }
    }

    pub fn recompute(&mut self) {
        self.stats.total_shards = self.shards.len();
        self.stats.active_shards = self.shards.values().filter(|s| s.state == ShardState::Active).count();
        self.stats.total_nodes = self.nodes.len();
        self.stats.total_keys = self.shards.values().map(|s| s.key_count).sum();
        self.stats.total_bytes = self.shards.values().map(|s| s.data_bytes).sum();
        self.stats.total_migrations = self.migrations.len();
        self.stats.hot_shards = self.detect_hot_shards().len();
        if !self.shards.is_empty() {
            let loads: Vec<f64> = self.shards.values().map(|s| s.load_score()).collect();
            self.stats.avg_load_score = loads.iter().sum::<f64>() / loads.len() as f64;
            let max = loads.iter().fold(0.0f64, |a, &b| if b > a { b } else { a });
            let min = loads.iter().fold(f64::MAX, |a, &b| if b < a { b } else { a });
            self.stats.load_imbalance = if self.stats.avg_load_score > 0.0 { (max - min) / self.stats.avg_load_score } else { 0.0 };
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &ShardingStats { &self.stats }
}
