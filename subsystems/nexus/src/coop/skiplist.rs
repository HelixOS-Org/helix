// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — Skiplist (concurrent probabilistic skip list)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkiplistOp {
    Insert,
    Remove,
    Find,
    RangeQuery,
}

#[derive(Debug, Clone)]
pub struct SkiplistNode {
    pub key: u64,
    pub value: u64,
    pub height: u32,
    pub forward: Vec<Option<u64>>,
    pub marked: bool,
    pub fully_linked: bool,
}

impl SkiplistNode {
    pub fn new(key: u64, value: u64, height: u32) -> Self {
        let forward = (0..height).map(|_| None).collect();
        Self { key, value, height, forward, marked: false, fully_linked: false }
    }

    pub fn link(&mut self) { self.fully_linked = true; }
    pub fn mark_for_removal(&mut self) { self.marked = true; }
}

#[derive(Debug, Clone)]
pub struct SkiplistLevel {
    pub level: u32,
    pub node_count: u64,
    pub traversals: u64,
}

#[derive(Debug, Clone)]
pub struct SkiplistConfig {
    pub max_height: u32,
    pub probability: u32, // out of 100
    pub seed: u64,
}

impl SkiplistConfig {
    pub fn default_config() -> Self {
        Self { max_height: 32, probability: 50, seed: 0x12345678 }
    }

    pub fn random_height(&self, mut seed: u64) -> u32 {
        let mut height = 1u32;
        loop {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            if (seed % 100) >= self.probability as u64 || height >= self.max_height {
                break;
            }
            height += 1;
        }
        height
    }
}

#[derive(Debug, Clone)]
pub struct SkiplistStats {
    pub total_nodes: u64,
    pub max_height_used: u32,
    pub total_inserts: u64,
    pub total_removes: u64,
    pub total_finds: u64,
    pub total_traversals: u64,
    pub avg_search_length: u64,
}

pub struct CoopSkiplist {
    nodes: BTreeMap<u64, SkiplistNode>,
    config: SkiplistConfig,
    current_height: u32,
    rng_state: u64,
    stats: SkiplistStats,
}

impl CoopSkiplist {
    pub fn new(config: SkiplistConfig) -> Self {
        let seed = config.seed;
        Self {
            nodes: BTreeMap::new(),
            config,
            current_height: 1,
            rng_state: seed,
            stats: SkiplistStats {
                total_nodes: 0, max_height_used: 1,
                total_inserts: 0, total_removes: 0,
                total_finds: 0, total_traversals: 0,
                avg_search_length: 0,
            },
        }
    }

    pub fn insert(&mut self, key: u64, value: u64) -> bool {
        if self.nodes.contains_key(&key) { return false; }
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        let height = self.config.random_height(self.rng_state);
        let node = SkiplistNode::new(key, value, height);
        self.nodes.insert(key, node);
        if height > self.current_height {
            self.current_height = height;
            self.stats.max_height_used = height;
        }
        self.stats.total_nodes += 1;
        self.stats.total_inserts += 1;
        true
    }

    pub fn find(&mut self, key: u64) -> Option<u64> {
        self.stats.total_finds += 1;
        self.nodes.get(&key).filter(|n| !n.marked).map(|n| n.value)
    }

    pub fn remove(&mut self, key: u64) -> bool {
        if let Some(node) = self.nodes.get_mut(&key) {
            node.mark_for_removal();
            self.stats.total_removes += 1;
            true
        } else { false }
    }

    pub fn stats(&self) -> &SkiplistStats { &self.stats }
}
