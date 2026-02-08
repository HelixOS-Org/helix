// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — Page Cache (cooperative page cache management)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Page cache entry state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopPageState {
    Clean,
    Dirty,
    Writeback,
    Locked,
    Uptodate,
    Error,
    Referenced,
}

/// Cooperative page cache entry
#[derive(Debug, Clone)]
pub struct CoopPageEntry {
    pub page_id: u64,
    pub inode: u64,
    pub offset: u64,
    pub state: CoopPageState,
    pub ref_count: u32,
    pub access_count: u64,
    pub last_access: u64,
}

/// Page cache eviction policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopEvictionPolicy {
    LRU,
    LFU,
    Clock,
    FIFO,
    Random,
}

/// Stats for page cache cooperation
#[derive(Debug, Clone)]
pub struct CoopPageCacheStats {
    pub total_pages: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub evictions: u64,
    pub dirty_pages: u64,
    pub hit_ratio_percent: u64,
}

/// Manager for page cache cooperative operations
pub struct CoopPageCacheManager {
    pages: BTreeMap<u64, CoopPageEntry>,
    inode_pages: BTreeMap<u64, Vec<u64>>,
    next_page: u64,
    max_pages: usize,
    policy: CoopEvictionPolicy,
    stats: CoopPageCacheStats,
}

impl CoopPageCacheManager {
    pub fn new(max_pages: usize) -> Self {
        Self {
            pages: BTreeMap::new(),
            inode_pages: BTreeMap::new(),
            next_page: 1,
            max_pages,
            policy: CoopEvictionPolicy::LRU,
            stats: CoopPageCacheStats {
                total_pages: 0,
                cache_hits: 0,
                cache_misses: 0,
                evictions: 0,
                dirty_pages: 0,
                hit_ratio_percent: 0,
            },
        }
    }

    pub fn lookup(&mut self, inode: u64, offset: u64) -> Option<&CoopPageEntry> {
        let key = inode.wrapping_mul(0x100000001b3) ^ offset;
        if let Some(page) = self.pages.get(&key) {
            self.stats.cache_hits += 1;
            Some(page)
        } else {
            self.stats.cache_misses += 1;
            None
        }
    }

    pub fn insert(&mut self, inode: u64, offset: u64) -> u64 {
        if self.pages.len() >= self.max_pages {
            self.evict_one();
        }
        let key = inode.wrapping_mul(0x100000001b3) ^ offset;
        let id = self.next_page;
        self.next_page += 1;
        let entry = CoopPageEntry {
            page_id: id,
            inode,
            offset,
            state: CoopPageState::Uptodate,
            ref_count: 1,
            access_count: 1,
            last_access: id,
        };
        self.pages.insert(key, entry);
        self.inode_pages.entry(inode).or_insert_with(Vec::new).push(key);
        self.stats.total_pages += 1;
        id
    }

    fn evict_one(&mut self) {
        if let Some((&key, _)) = self.pages.iter().next() {
            if let Some(entry) = self.pages.remove(&key) {
                if let Some(list) = self.inode_pages.get_mut(&entry.inode) {
                    list.retain(|&k| k != key);
                }
                self.stats.evictions += 1;
            }
        }
    }

    pub fn mark_dirty(&mut self, inode: u64, offset: u64) {
        let key = inode.wrapping_mul(0x100000001b3) ^ offset;
        if let Some(page) = self.pages.get_mut(&key) {
            if page.state != CoopPageState::Dirty {
                page.state = CoopPageState::Dirty;
                self.stats.dirty_pages += 1;
            }
        }
    }

    pub fn invalidate_inode(&mut self, inode: u64) -> usize {
        if let Some(keys) = self.inode_pages.remove(&inode) {
            let count = keys.len();
            for key in keys {
                self.pages.remove(&key);
            }
            count
        } else {
            0
        }
    }

    pub fn set_policy(&mut self, policy: CoopEvictionPolicy) {
        self.policy = policy;
    }

    pub fn stats(&self) -> &CoopPageCacheStats {
        &self.stats
    }
}
