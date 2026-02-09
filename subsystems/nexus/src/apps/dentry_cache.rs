// SPDX-License-Identifier: GPL-2.0
//! Apps dentry cache — directory entry caching and path lookup optimization.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

/// Dentry state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DentryState {
    /// Valid and in use
    Active,
    /// Valid but not referenced
    Unused,
    /// Negative dentry (name doesn't exist)
    Negative,
    /// Being deleted
    Dying,
}

/// A directory entry
#[derive(Debug, Clone)]
pub struct Dentry {
    pub inode: u64,
    pub parent_inode: u64,
    pub name: String,
    pub state: DentryState,
    pub ref_count: u32,
    pub mount_point: bool,
    pub lookup_count: u64,
    pub last_access_ns: u64,
    hash: u64,
}

impl Dentry {
    pub fn new(inode: u64, parent_inode: u64, name: String) -> Self {
        let hash = Self::compute_hash(parent_inode, &name);
        Self {
            inode,
            parent_inode,
            name,
            state: DentryState::Active,
            ref_count: 1,
            mount_point: false,
            lookup_count: 0,
            last_access_ns: 0,
            hash,
        }
    }

    pub fn negative(parent_inode: u64, name: String) -> Self {
        let hash = Self::compute_hash(parent_inode, &name);
        Self {
            inode: 0,
            parent_inode,
            name,
            state: DentryState::Negative,
            ref_count: 0,
            mount_point: false,
            lookup_count: 0,
            last_access_ns: 0,
            hash,
        }
    }

    fn compute_hash(parent: u64, name: &str) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for &b in &parent.to_le_bytes() {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        for &b in name.as_bytes() {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    #[inline(always)]
    pub fn is_active(&self) -> bool {
        self.state == DentryState::Active
    }

    #[inline(always)]
    pub fn is_negative(&self) -> bool {
        self.state == DentryState::Negative
    }

    #[inline]
    pub fn acquire(&mut self) {
        self.ref_count = self.ref_count.saturating_add(1);
        self.state = DentryState::Active;
        self.lookup_count += 1;
    }

    #[inline]
    pub fn release(&mut self) {
        self.ref_count = self.ref_count.saturating_sub(1);
        if self.ref_count == 0 && self.state == DentryState::Active {
            self.state = DentryState::Unused;
        }
    }
}

/// Path component for multi-level lookup
#[derive(Debug, Clone)]
pub struct PathLookup {
    pub components: Vec<String>,
    pub resolved_inodes: Vec<u64>,
    pub cache_hits: u32,
    pub cache_misses: u32,
    pub complete: bool,
}

impl PathLookup {
    pub fn from_path(path: &str) -> Self {
        let components: Vec<String> = path
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| String::from(s))
            .collect();
        Self {
            components,
            resolved_inodes: Vec::new(),
            cache_hits: 0,
            cache_misses: 0,
            complete: false,
        }
    }

    #[inline]
    pub fn hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 { return 0.0; }
        self.cache_hits as f64 / total as f64
    }

    #[inline(always)]
    pub fn remaining_components(&self) -> usize {
        self.components.len() - self.resolved_inodes.len()
    }
}

/// LRU list for dcache eviction
#[derive(Debug)]
pub struct DcacheLru {
    entries: VecDeque<u64>,
    max_size: usize,
}

impl DcacheLru {
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            max_size,
        }
    }

    #[inline(always)]
    pub fn touch(&mut self, hash: u64) {
        self.entries.retain(|&h| h != hash);
        self.entries.push_back(hash);
    }

    #[inline]
    pub fn evict_oldest(&mut self) -> Option<u64> {
        if self.entries.is_empty() {
            None
        } else {
            self.entries.pop_front()
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[inline(always)]
    pub fn is_full(&self) -> bool {
        self.entries.len() >= self.max_size
    }
}

/// Dentry cache stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DentryCacheStats {
    pub total_entries: u64,
    pub active_entries: u64,
    pub negative_entries: u64,
    pub lookups: u64,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub invalidations: u64,
}

impl DentryCacheStats {
    #[inline]
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { return 0.0; }
        self.hits as f64 / total as f64
    }
}

/// Main apps dentry cache
#[repr(align(64))]
pub struct AppDentryCache {
    cache: BTreeMap<u64, Dentry>,
    lru: DcacheLru,
    root_inode: u64,
    max_entries: usize,
    max_negative: usize,
    negative_count: usize,
    stats: DentryCacheStats,
}

impl AppDentryCache {
    pub fn new(root_inode: u64, max_entries: usize) -> Self {
        Self {
            cache: BTreeMap::new(),
            lru: DcacheLru::new(max_entries),
            root_inode,
            max_entries,
            max_negative: max_entries / 4,
            negative_count: 0,
            stats: DentryCacheStats {
                total_entries: 0,
                active_entries: 0,
                negative_entries: 0,
                lookups: 0,
                hits: 0,
                misses: 0,
                evictions: 0,
                invalidations: 0,
            },
        }
    }

    pub fn insert(&mut self, dentry: Dentry) {
        let hash = dentry.hash;
        let is_negative = dentry.is_negative();

        if is_negative {
            if self.negative_count >= self.max_negative {
                return;
            }
            self.negative_count += 1;
            self.stats.negative_entries += 1;
        }

        // Evict if at capacity
        while self.cache.len() >= self.max_entries {
            if let Some(evict_hash) = self.lru.evict_oldest() {
                if let Some(evicted) = self.cache.remove(&evict_hash) {
                    if evicted.is_negative() {
                        self.negative_count = self.negative_count.saturating_sub(1);
                    }
                    self.stats.evictions += 1;
                }
            } else {
                break;
            }
        }

        self.cache.insert(hash, dentry);
        self.lru.touch(hash);
        self.stats.total_entries += 1;
        self.stats.active_entries = self.cache.len() as u64;
    }

    pub fn lookup(&mut self, parent_inode: u64, name: &str) -> Option<&Dentry> {
        self.stats.lookups += 1;
        let hash = Dentry::compute_hash(parent_inode, name);
        if let Some(dentry) = self.cache.get(&hash) {
            if dentry.parent_inode == parent_inode && dentry.name == name {
                self.stats.hits += 1;
                self.lru.touch(hash);
                return Some(dentry);
            }
        }
        self.stats.misses += 1;
        None
    }

    pub fn lookup_path(&mut self, path: &str) -> PathLookup {
        let mut lookup = PathLookup::from_path(path);
        let mut current_inode = self.root_inode;

        for component in &lookup.components {
            if let Some(dentry) = self.lookup(current_inode, component) {
                if dentry.is_negative() {
                    lookup.cache_hits += 1;
                    break; // Path doesn't exist
                }
                current_inode = dentry.inode;
                lookup.resolved_inodes.push(current_inode);
                lookup.cache_hits += 1;
            } else {
                lookup.cache_misses += 1;
                break;
            }
        }

        lookup.complete = lookup.resolved_inodes.len() == lookup.components.len();
        lookup
    }

    pub fn invalidate(&mut self, parent_inode: u64, name: &str) -> bool {
        let hash = Dentry::compute_hash(parent_inode, name);
        if let Some(dentry) = self.cache.remove(&hash) {
            if dentry.is_negative() {
                self.negative_count = self.negative_count.saturating_sub(1);
            }
            self.stats.invalidations += 1;
            self.stats.active_entries = self.cache.len() as u64;
            true
        } else {
            false
        }
    }

    pub fn invalidate_subtree(&mut self, inode: u64) -> u32 {
        let to_remove: Vec<u64> = self.cache.iter()
            .filter(|(_, d)| d.parent_inode == inode)
            .map(|(h, _)| *h)
            .collect();
        let count = to_remove.len() as u32;
        for hash in to_remove {
            if let Some(d) = self.cache.remove(&hash) {
                if d.is_negative() {
                    self.negative_count = self.negative_count.saturating_sub(1);
                }
            }
        }
        self.stats.invalidations += count as u64;
        self.stats.active_entries = self.cache.len() as u64;
        count
    }

    pub fn shrink(&mut self, target: usize) -> u32 {
        let mut evicted = 0u32;
        while self.cache.len() > target {
            if let Some(hash) = self.lru.evict_oldest() {
                if let Some(d) = self.cache.remove(&hash) {
                    if d.is_negative() {
                        self.negative_count = self.negative_count.saturating_sub(1);
                    }
                    evicted += 1;
                }
            } else {
                break;
            }
        }
        self.stats.evictions += evicted as u64;
        self.stats.active_entries = self.cache.len() as u64;
        evicted
    }

    #[inline(always)]
    pub fn stats(&self) -> &DentryCacheStats {
        &self.stats
    }
}
