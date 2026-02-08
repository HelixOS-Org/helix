// SPDX-License-Identifier: GPL-2.0
//! Coop skip_list — probabilistic skip list for ordered cooperative data structures.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Maximum number of skip list levels
pub const MAX_LEVEL: usize = 32;

/// Skip list node
#[derive(Debug, Clone)]
pub struct SkipNode {
    pub key: u64,
    pub value_size: usize,
    pub value_hash: u64,
    pub level: usize,
    pub inserted_ns: u64,
    pub access_count: u64,
}

impl SkipNode {
    pub fn new(key: u64, value_size: usize, level: usize) -> Self {
        Self {
            key,
            value_size,
            value_hash: 0,
            level,
            inserted_ns: 0,
            access_count: 0,
        }
    }
}

/// Skip list instance — ordered set with O(log n) operations
#[derive(Debug)]
pub struct SkipList {
    pub id: u64,
    pub name: String,
    pub max_level: usize,
    pub current_level: usize,
    nodes: Vec<SkipNode>,
    pub node_count: u64,
    pub total_bytes: u64,
    pub total_inserts: u64,
    pub total_removes: u64,
    pub total_lookups: u64,
    pub total_scans: u64,
    pub lookup_comparisons: u64,
    rng_state: u64,
}

impl SkipList {
    pub fn new(id: u64, name: String) -> Self {
        Self {
            id,
            name,
            max_level: MAX_LEVEL,
            current_level: 1,
            nodes: Vec::new(),
            node_count: 0,
            total_bytes: 0,
            total_inserts: 0,
            total_removes: 0,
            total_lookups: 0,
            total_scans: 0,
            lookup_comparisons: 0,
            rng_state: 0x12345678abcdef01,
        }
    }

    fn random_level(&mut self) -> usize {
        let mut level = 1usize;
        // xorshift64
        loop {
            self.rng_state ^= self.rng_state << 13;
            self.rng_state ^= self.rng_state >> 7;
            self.rng_state ^= self.rng_state << 17;
            if self.rng_state & 1 == 0 && level < self.max_level {
                level += 1;
            } else {
                break;
            }
        }
        level
    }

    pub fn insert(&mut self, key: u64, value_size: usize, now_ns: u64) -> bool {
        // Check for duplicate
        if self.nodes.iter().any(|n| n.key == key) {
            return false;
        }

        let level = self.random_level();
        if level > self.current_level {
            self.current_level = level;
        }

        let mut node = SkipNode::new(key, value_size, level);
        node.inserted_ns = now_ns;

        // Insert in sorted order
        let pos = self.nodes.partition_point(|n| n.key < key);
        self.nodes.insert(pos, node);

        self.node_count += 1;
        self.total_bytes += value_size as u64;
        self.total_inserts += 1;
        true
    }

    pub fn remove(&mut self, key: u64) -> bool {
        if let Some(pos) = self.nodes.iter().position(|n| n.key == key) {
            let node = self.nodes.remove(pos);
            self.node_count = self.node_count.saturating_sub(1);
            self.total_bytes = self.total_bytes.saturating_sub(node.value_size as u64);
            self.total_removes += 1;
            true
        } else {
            false
        }
    }

    pub fn lookup(&mut self, key: u64) -> Option<&SkipNode> {
        self.total_lookups += 1;
        // Binary search (simulating skip list O(log n))
        let mut comparisons = 0u64;
        let result = self.nodes.binary_search_by_key(&key, |n| {
            comparisons += 1;
            n.key
        });
        self.lookup_comparisons += comparisons;

        match result {
            Ok(idx) => {
                self.nodes[idx].access_count += 1;
                Some(&self.nodes[idx])
            }
            Err(_) => None,
        }
    }

    pub fn range_scan(&mut self, start: u64, end: u64) -> Vec<&SkipNode> {
        self.total_scans += 1;
        self.nodes.iter()
            .filter(|n| n.key >= start && n.key <= end)
            .collect()
    }

    pub fn min(&self) -> Option<&SkipNode> {
        self.nodes.first()
    }

    pub fn max(&self) -> Option<&SkipNode> {
        self.nodes.last()
    }

    pub fn floor(&self, key: u64) -> Option<&SkipNode> {
        let pos = self.nodes.partition_point(|n| n.key <= key);
        if pos > 0 { Some(&self.nodes[pos - 1]) } else { None }
    }

    pub fn ceiling(&self, key: u64) -> Option<&SkipNode> {
        let pos = self.nodes.partition_point(|n| n.key < key);
        self.nodes.get(pos)
    }

    pub fn count(&self) -> u64 {
        self.node_count
    }

    pub fn avg_comparisons_per_lookup(&self) -> f64 {
        if self.total_lookups == 0 { return 0.0; }
        self.lookup_comparisons as f64 / self.total_lookups as f64
    }

    pub fn avg_value_size(&self) -> f64 {
        if self.node_count == 0 { return 0.0; }
        self.total_bytes as f64 / self.node_count as f64
    }

    pub fn level_distribution(&self) -> Vec<(usize, u64)> {
        let mut dist = Vec::new();
        for lvl in 1..=self.current_level {
            let count = self.nodes.iter().filter(|n| n.level >= lvl).count() as u64;
            dist.push((lvl, count));
        }
        dist
    }

    pub fn hottest_keys(&self, top: usize) -> Vec<(u64, u64)> {
        let mut v: Vec<(u64, u64)> = self.nodes.iter()
            .map(|n| (n.key, n.access_count))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(top);
        v
    }
}

/// Skip list stats
#[derive(Debug, Clone)]
pub struct SkipListStats {
    pub total_lists: u64,
    pub total_nodes: u64,
    pub total_bytes: u64,
    pub total_inserts: u64,
    pub total_removes: u64,
    pub total_lookups: u64,
}

/// Main skip list manager
pub struct CoopSkipList {
    lists: BTreeMap<u64, SkipList>,
    next_id: u64,
    stats: SkipListStats,
}

impl CoopSkipList {
    pub fn new() -> Self {
        Self {
            lists: BTreeMap::new(),
            next_id: 1,
            stats: SkipListStats {
                total_lists: 0,
                total_nodes: 0,
                total_bytes: 0,
                total_inserts: 0,
                total_removes: 0,
                total_lookups: 0,
            },
        }
    }

    pub fn create_list(&mut self, name: String) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.lists.insert(id, SkipList::new(id, name));
        self.stats.total_lists += 1;
        id
    }

    pub fn insert(&mut self, list_id: u64, key: u64, value_size: usize, now_ns: u64) -> bool {
        if let Some(list) = self.lists.get_mut(&list_id) {
            let ok = list.insert(key, value_size, now_ns);
            if ok {
                self.stats.total_inserts += 1;
                self.stats.total_nodes += 1;
                self.stats.total_bytes += value_size as u64;
            }
            ok
        } else {
            false
        }
    }

    pub fn remove(&mut self, list_id: u64, key: u64) -> bool {
        if let Some(list) = self.lists.get_mut(&list_id) {
            let ok = list.remove(key);
            if ok {
                self.stats.total_removes += 1;
                self.stats.total_nodes = self.stats.total_nodes.saturating_sub(1);
            }
            ok
        } else {
            false
        }
    }

    pub fn lookup(&mut self, list_id: u64, key: u64) -> bool {
        if let Some(list) = self.lists.get_mut(&list_id) {
            self.stats.total_lookups += 1;
            list.lookup(key).is_some()
        } else {
            false
        }
    }

    pub fn largest_lists(&self, top: usize) -> Vec<(u64, u64)> {
        let mut v: Vec<(u64, u64)> = self.lists.iter()
            .map(|(&id, l)| (id, l.node_count))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(top);
        v
    }

    pub fn get_list(&self, id: u64) -> Option<&SkipList> {
        self.lists.get(&id)
    }

    pub fn stats(&self) -> &SkipListStats {
        &self.stats
    }
}
