// SPDX-License-Identifier: GPL-2.0
//! Coop dentry — cooperative dentry cache with shared negative lookups

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Coop dentry state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopDentryState {
    Positive,
    Negative,
    Shared,
    Revalidating,
    Dead,
}

/// Shared dentry entry
#[derive(Debug, Clone)]
pub struct CoopDentryEntry {
    pub name_hash: u64,
    pub parent_hash: u64,
    pub inode: u64,
    pub state: CoopDentryState,
    pub shared_count: u32,
    pub hits: u64,
    pub last_revalidation: u64,
}

impl CoopDentryEntry {
    pub fn new(name: &[u8], parent_hash: u64) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in name { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self { name_hash: h, parent_hash, inode: 0, state: CoopDentryState::Positive, shared_count: 1, hits: 0, last_revalidation: 0 }
    }

    pub fn share(&mut self) { self.shared_count += 1; self.state = CoopDentryState::Shared; }
    pub fn hit(&mut self) { self.hits += 1; }
    pub fn invalidate(&mut self) { self.state = CoopDentryState::Dead; }
    pub fn make_negative(&mut self) { self.state = CoopDentryState::Negative; self.inode = 0; }
}

/// Coop dentry stats
#[derive(Debug, Clone)]
pub struct CoopDentryStats {
    pub total_lookups: u64,
    pub hits: u64,
    pub negative_hits: u64,
    pub shared_lookups: u64,
    pub invalidations: u64,
}

/// Main coop dentry
#[derive(Debug)]
pub struct CoopDentry {
    pub entries: BTreeMap<u64, CoopDentryEntry>,
    pub stats: CoopDentryStats,
}

impl CoopDentry {
    pub fn new() -> Self {
        Self { entries: BTreeMap::new(), stats: CoopDentryStats { total_lookups: 0, hits: 0, negative_hits: 0, shared_lookups: 0, invalidations: 0 } }
    }

    pub fn lookup(&mut self, name_hash: u64) -> Option<&CoopDentryEntry> {
        self.stats.total_lookups += 1;
        if let Some(entry) = self.entries.get_mut(&name_hash) {
            entry.hit();
            match entry.state {
                CoopDentryState::Negative => { self.stats.negative_hits += 1; }
                CoopDentryState::Shared => { self.stats.shared_lookups += 1; self.stats.hits += 1; }
                _ => { self.stats.hits += 1; }
            }
        }
        self.entries.get(&name_hash)
    }

    pub fn insert(&mut self, name: &[u8], parent_hash: u64, inode: u64) {
        let mut entry = CoopDentryEntry::new(name, parent_hash);
        entry.inode = inode;
        self.entries.insert(entry.name_hash, entry);
    }

    pub fn hit_rate(&self) -> f64 {
        if self.stats.total_lookups == 0 { 0.0 } else { self.stats.hits as f64 / self.stats.total_lookups as f64 }
    }
}

// ============================================================================
// Merged from dentry_v2_coop
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopDentryV2Event {
    Lookup,
    Insert,
    Delete,
    Rename,
    Revalidate,
    Invalidate,
    Splice,
    Prune,
}

/// Cooperative dentry cache entry
#[derive(Debug, Clone)]
pub struct CoopDentryV2Entry {
    pub name: String,
    pub inode: u64,
    pub parent_inode: u64,
    pub ref_count: u32,
    pub is_negative: bool,
    pub generation: u64,
    pub last_validated: u64,
}

/// Stats for dentry cooperation
#[derive(Debug, Clone)]
pub struct CoopDentryV2Stats {
    pub total_lookups: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub invalidations: u64,
    pub pruned_entries: u64,
    pub negative_entries: u64,
}

/// Manager for dentry cooperative operations
pub struct CoopDentryV2Manager {
    cache: BTreeMap<u64, CoopDentryV2Entry>,
    name_index: BTreeMap<u64, u64>,
    stats: CoopDentryV2Stats,
    max_cache_size: usize,
}

impl CoopDentryV2Manager {
    pub fn new() -> Self {
        Self {
            cache: BTreeMap::new(),
            name_index: BTreeMap::new(),
            stats: CoopDentryV2Stats {
                total_lookups: 0,
                cache_hits: 0,
                cache_misses: 0,
                invalidations: 0,
                pruned_entries: 0,
                negative_entries: 0,
            },
            max_cache_size: 8192,
        }
    }

    fn hash_name(name: &str, parent: u64) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in name.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h ^= parent;
        h = h.wrapping_mul(0x100000001b3);
        h
    }

    pub fn insert(&mut self, name: &str, inode: u64, parent_inode: u64) {
        let hash = Self::hash_name(name, parent_inode);
        let entry = CoopDentryV2Entry {
            name: String::from(name),
            inode,
            parent_inode,
            ref_count: 1,
            is_negative: false,
            generation: self.cache.len() as u64,
            last_validated: hash & 0xFFFF,
        };
        self.cache.insert(inode, entry);
        self.name_index.insert(hash, inode);
    }

    pub fn lookup(&mut self, name: &str, parent_inode: u64) -> Option<&CoopDentryV2Entry> {
        self.stats.total_lookups += 1;
        let hash = Self::hash_name(name, parent_inode);
        if let Some(&inode) = self.name_index.get(&hash) {
            self.stats.cache_hits += 1;
            self.cache.get(&inode)
        } else {
            self.stats.cache_misses += 1;
            None
        }
    }

    pub fn invalidate(&mut self, inode: u64) -> bool {
        if self.cache.remove(&inode).is_some() {
            self.stats.invalidations += 1;
            true
        } else {
            false
        }
    }

    pub fn prune(&mut self, max_age: u64) -> usize {
        let to_prune: alloc::vec::Vec<u64> = self.cache.iter()
            .filter(|(_, e)| e.ref_count == 0 && e.last_validated < max_age)
            .map(|(&k, _)| k)
            .collect();
        let count = to_prune.len();
        for inode in to_prune {
            self.cache.remove(&inode);
        }
        self.stats.pruned_entries += count as u64;
        count
    }

    pub fn stats(&self) -> &CoopDentryV2Stats {
        &self.stats
    }
}
