// SPDX-License-Identifier: GPL-2.0
//! NEXUS Holistic dentry cache — Directory entry cache with negative lookup
//!
//! Models the kernel dentry cache with LRU reclaim, negative dentry tracking,
//! path component hash lookup, and mount point crossing awareness.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Dentry state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DentryState {
    Positive,
    Negative,
    Disconnected,
    Mounted,
    Reclaiming,
}

/// Dentry type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DentryType {
    Directory,
    RegularFile,
    Symlink,
    Special,
}

/// A cached directory entry.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DentryCacheEntry {
    pub dentry_id: u64,
    pub parent_id: Option<u64>,
    pub inode: Option<u64>,
    pub name_hash: u64,
    pub name: String,
    pub state: DentryState,
    pub dtype: Option<DentryType>,
    pub ref_count: u32,
    pub lru_timestamp: u64,
    pub lookup_count: u64,
}

impl DentryCacheEntry {
    pub fn new(dentry_id: u64, name: String, inode: Option<u64>) -> Self {
        // FNV-1a hash
        let mut h: u64 = 0xcbf29ce484222325;
        for b in name.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        let state = if inode.is_some() {
            DentryState::Positive
        } else {
            DentryState::Negative
        };
        Self {
            dentry_id,
            parent_id: None,
            inode,
            name_hash: h,
            name,
            state,
            dtype: None,
            ref_count: 1,
            lru_timestamp: 0,
            lookup_count: 0,
        }
    }

    #[inline(always)]
    pub fn is_negative(&self) -> bool {
        self.state == DentryState::Negative
    }

    #[inline(always)]
    pub fn touch(&mut self, timestamp: u64) {
        self.lru_timestamp = timestamp;
        self.lookup_count += 1;
    }
}

/// Statistics for dentry cache.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DentryCacheStats {
    pub total_entries: u64,
    pub positive_entries: u64,
    pub negative_entries: u64,
    pub lookups: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub reclaimed: u64,
    pub allocated: u64,
}

/// Main holistic dentry cache manager.
#[repr(align(64))]
pub struct HolisticDentryCache {
    pub entries: BTreeMap<u64, DentryCacheEntry>,
    pub hash_index: BTreeMap<u64, Vec<u64>>, // name_hash → [dentry_ids]
    pub next_id: u64,
    pub max_entries: usize,
    pub stats: DentryCacheStats,
}

impl HolisticDentryCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            hash_index: BTreeMap::new(),
            next_id: 1,
            max_entries,
            stats: DentryCacheStats {
                total_entries: 0,
                positive_entries: 0,
                negative_entries: 0,
                lookups: 0,
                cache_hits: 0,
                cache_misses: 0,
                reclaimed: 0,
                allocated: 0,
            },
        }
    }

    pub fn insert(&mut self, name: String, inode: Option<u64>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let entry = DentryCacheEntry::new(id, name, inode);
        let hash = entry.name_hash;
        if entry.is_negative() {
            self.stats.negative_entries += 1;
        } else {
            self.stats.positive_entries += 1;
        }
        self.hash_index.entry(hash).or_insert_with(Vec::new).push(id);
        self.entries.insert(id, entry);
        self.stats.total_entries += 1;
        self.stats.allocated += 1;
        id
    }

    pub fn lookup(&mut self, name: &str, timestamp: u64) -> Option<&DentryCacheEntry> {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in name.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        self.stats.lookups += 1;
        if let Some(ids) = self.hash_index.get(&h) {
            for &id in ids {
                if let Some(entry) = self.entries.get_mut(&id) {
                    if entry.name == name {
                        entry.touch(timestamp);
                        self.stats.cache_hits += 1;
                        return self.entries.get(&id);
                    }
                }
            }
        }
        self.stats.cache_misses += 1;
        None
    }

    #[inline]
    pub fn hit_rate(&self) -> f64 {
        if self.stats.lookups == 0 {
            return 0.0;
        }
        self.stats.cache_hits as f64 / self.stats.lookups as f64
    }

    #[inline(always)]
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}
