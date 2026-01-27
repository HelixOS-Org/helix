//! Inode Management
//!
//! In-memory inode cache and management for VFS integration.

use crate::core::error::{HfsError, HfsResult};
use crate::api::FileType;
use super::{ROOT_INO, VfsInode, VfsInodeFlags, Timespec};

// ============================================================================
// Constants
// ============================================================================

/// Inode hash table size
const HASH_SIZE: usize = 4096;

/// Maximum inodes in cache
const CACHE_SIZE: usize = 16384;

// ============================================================================
// Inode Hash
// ============================================================================

/// Hash function for inodes.
#[inline]
fn inode_hash(dev: u64, ino: u64) -> usize {
    let key = dev.wrapping_mul(0x9e3779b97f4a7c15) ^ ino;
    (key as usize) & (HASH_SIZE - 1)
}

// ============================================================================
// Cached Inode
// ============================================================================

/// State of cached inode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum InodeCacheState {
    /// Free slot
    Free = 0,
    /// Valid and in use
    Valid = 1,
    /// Being loaded
    Loading = 2,
    /// Being written
    Writing = 3,
    /// Being deleted
    Deleting = 4,
}

impl Default for InodeCacheState {
    fn default() -> Self {
        Self::Free
    }
}

/// Cached inode entry.
#[derive(Clone, Copy)]
pub struct CachedInode {
    /// Inode data
    pub inode: VfsInode,
    /// Cache state
    pub state: InodeCacheState,
    /// Hash chain next
    pub hash_next: u16,
    /// LRU next
    pub lru_next: u16,
    /// LRU prev
    pub lru_prev: u16,
    /// Cache index
    pub index: u16,
}

impl CachedInode {
    /// Create empty cached inode
    pub const fn empty() -> Self {
        Self {
            inode: VfsInode {
                ino: 0,
                dev: 0,
                ftype: FileType::Unknown,
                mode: 0,
                nlink: 0,
                uid: 0,
                gid: 0,
                rdev: 0,
                size: 0,
                blksize: 4096,
                blocks: 0,
                atime: Timespec { sec: 0, nsec: 0 },
                mtime: Timespec { sec: 0, nsec: 0 },
                ctime: Timespec { sec: 0, nsec: 0 },
                crtime: Timespec { sec: 0, nsec: 0 },
                generation: 0,
                flags: VfsInodeFlags(0),
                refcount: 0,
            },
            state: InodeCacheState::Free,
            hash_next: u16::MAX,
            lru_next: u16::MAX,
            lru_prev: u16::MAX,
            index: 0,
        }
    }
    
    /// Is free
    #[inline]
    pub fn is_free(&self) -> bool {
        self.state == InodeCacheState::Free
    }
    
    /// Is valid
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.state == InodeCacheState::Valid
    }
}

impl Default for CachedInode {
    fn default() -> Self {
        Self::empty()
    }
}

// ============================================================================
// Inode Cache
// ============================================================================

/// Inode cache.
pub struct InodeCache {
    /// Cached inodes
    inodes: [CachedInode; CACHE_SIZE],
    /// Hash table
    hash_table: [u16; HASH_SIZE],
    /// Free list head
    free_head: u16,
    /// LRU list head (most recent)
    lru_head: u16,
    /// LRU list tail (least recent)
    lru_tail: u16,
    /// Number of cached inodes
    count: usize,
    /// Statistics
    stats: InodeCacheStats,
}

/// Inode cache statistics.
#[derive(Clone, Copy, Debug, Default)]
pub struct InodeCacheStats {
    /// Cache hits
    pub hits: u64,
    /// Cache misses
    pub misses: u64,
    /// Evictions
    pub evictions: u64,
    /// Dirty writebacks
    pub writebacks: u64,
}

impl InodeCache {
    /// Create new inode cache
    pub fn new() -> Self {
        let mut cache = Self {
            inodes: [CachedInode::empty(); CACHE_SIZE],
            hash_table: [u16::MAX; HASH_SIZE],
            free_head: 0,
            lru_head: u16::MAX,
            lru_tail: u16::MAX,
            count: 0,
            stats: InodeCacheStats::default(),
        };
        
        // Initialize free list
        for i in 0..CACHE_SIZE - 1 {
            cache.inodes[i].lru_next = (i + 1) as u16;
            cache.inodes[i].index = i as u16;
        }
        cache.inodes[CACHE_SIZE - 1].lru_next = u16::MAX;
        cache.inodes[CACHE_SIZE - 1].index = (CACHE_SIZE - 1) as u16;
        
        cache
    }
    
    /// Lookup inode in cache
    pub fn lookup(&mut self, dev: u64, ino: u64) -> Option<&mut CachedInode> {
        let hash = inode_hash(dev, ino);
        let mut idx = self.hash_table[hash];
        
        while idx != u16::MAX {
            let cached = &self.inodes[idx as usize];
            if cached.inode.dev == dev && cached.inode.ino == ino && cached.is_valid() {
                self.stats.hits += 1;
                
                // Move to front of LRU
                self.lru_touch(idx);
                
                return Some(&mut self.inodes[idx as usize]);
            }
            idx = cached.hash_next;
        }
        
        self.stats.misses += 1;
        None
    }
    
    /// Insert inode into cache
    pub fn insert(&mut self, inode: VfsInode) -> HfsResult<&mut CachedInode> {
        // Check if already cached
        if self.lookup(inode.dev, inode.ino).is_some() {
            // Update existing
            let cached = self.lookup(inode.dev, inode.ino).unwrap();
            cached.inode = inode;
            return Ok(cached);
        }
        
        // Allocate slot
        let idx = self.alloc_slot()?;
        
        // Insert into hash table
        let hash = inode_hash(inode.dev, inode.ino);
        self.inodes[idx].hash_next = self.hash_table[hash];
        self.hash_table[hash] = idx as u16;
        
        // Set inode data
        self.inodes[idx].inode = inode;
        self.inodes[idx].state = InodeCacheState::Valid;
        
        // Add to LRU front
        self.lru_add_front(idx as u16);
        
        self.count += 1;
        
        Ok(&mut self.inodes[idx])
    }
    
    /// Remove inode from cache
    pub fn remove(&mut self, dev: u64, ino: u64) -> Option<VfsInode> {
        let hash = inode_hash(dev, ino);
        let mut prev: u16 = u16::MAX;
        let mut idx = self.hash_table[hash];
        
        while idx != u16::MAX {
            let cached = &self.inodes[idx as usize];
            
            if cached.inode.dev == dev && cached.inode.ino == ino {
                // Remove from hash chain
                if prev == u16::MAX {
                    self.hash_table[hash] = cached.hash_next;
                } else {
                    self.inodes[prev as usize].hash_next = cached.hash_next;
                }
                
                // Remove from LRU
                self.lru_remove(idx);
                
                // Get inode data
                let inode = self.inodes[idx as usize].inode;
                
                // Add to free list
                self.inodes[idx as usize].state = InodeCacheState::Free;
                self.inodes[idx as usize].lru_next = self.free_head;
                self.free_head = idx;
                
                self.count -= 1;
                
                return Some(inode);
            }
            
            prev = idx;
            idx = cached.hash_next;
        }
        
        None
    }
    
    /// Get inode reference by index
    pub fn get(&self, idx: usize) -> Option<&CachedInode> {
        if idx < CACHE_SIZE && self.inodes[idx].is_valid() {
            Some(&self.inodes[idx])
        } else {
            None
        }
    }
    
    /// Get inode reference by index (mutable)
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut CachedInode> {
        if idx < CACHE_SIZE && self.inodes[idx].is_valid() {
            Some(&mut self.inodes[idx])
        } else {
            None
        }
    }
    
    /// Allocate cache slot
    fn alloc_slot(&mut self) -> HfsResult<usize> {
        // Try free list
        if self.free_head != u16::MAX {
            let idx = self.free_head as usize;
            self.free_head = self.inodes[idx].lru_next;
            self.inodes[idx].lru_next = u16::MAX;
            self.inodes[idx].lru_prev = u16::MAX;
            self.inodes[idx].hash_next = u16::MAX;
            return Ok(idx);
        }
        
        // Evict from LRU tail
        if self.lru_tail != u16::MAX {
            let idx = self.lru_tail;
            let cached = &self.inodes[idx as usize];
            
            // Skip if dirty or referenced
            if cached.inode.flags.is_dirty() || cached.inode.refcount > 0 {
                // Try to find a non-dirty entry
                let mut scan = self.lru_tail;
                while scan != u16::MAX {
                    let entry = &self.inodes[scan as usize];
                    if !entry.inode.flags.is_dirty() && entry.inode.refcount == 0 {
                        self.evict(scan as usize);
                        self.stats.evictions += 1;
                        return Ok(scan as usize);
                    }
                    scan = entry.lru_prev;
                }
                
                return Err(HfsError::NoSpace);
            }
            
            self.evict(idx as usize);
            self.stats.evictions += 1;
            return Ok(idx as usize);
        }
        
        Err(HfsError::NoSpace)
    }
    
    /// Evict an entry
    fn evict(&mut self, idx: usize) {
        let cached = &self.inodes[idx];
        let dev = cached.inode.dev;
        let ino = cached.inode.ino;
        
        // Remove from hash
        let hash = inode_hash(dev, ino);
        let mut prev: u16 = u16::MAX;
        let mut cur = self.hash_table[hash];
        
        while cur != u16::MAX {
            if cur as usize == idx {
                if prev == u16::MAX {
                    self.hash_table[hash] = self.inodes[cur as usize].hash_next;
                } else {
                    self.inodes[prev as usize].hash_next = self.inodes[cur as usize].hash_next;
                }
                break;
            }
            prev = cur;
            cur = self.inodes[cur as usize].hash_next;
        }
        
        // Remove from LRU
        self.lru_remove(idx as u16);
        
        // Reset entry
        self.inodes[idx].state = InodeCacheState::Free;
        self.inodes[idx].hash_next = u16::MAX;
        
        self.count -= 1;
    }
    
    /// Add to front of LRU list
    fn lru_add_front(&mut self, idx: u16) {
        self.inodes[idx as usize].lru_next = self.lru_head;
        self.inodes[idx as usize].lru_prev = u16::MAX;
        
        if self.lru_head != u16::MAX {
            self.inodes[self.lru_head as usize].lru_prev = idx;
        }
        
        self.lru_head = idx;
        
        if self.lru_tail == u16::MAX {
            self.lru_tail = idx;
        }
    }
    
    /// Remove from LRU list
    fn lru_remove(&mut self, idx: u16) {
        let entry = &self.inodes[idx as usize];
        let prev = entry.lru_prev;
        let next = entry.lru_next;
        
        if prev != u16::MAX {
            self.inodes[prev as usize].lru_next = next;
        } else {
            self.lru_head = next;
        }
        
        if next != u16::MAX {
            self.inodes[next as usize].lru_prev = prev;
        } else {
            self.lru_tail = prev;
        }
        
        self.inodes[idx as usize].lru_prev = u16::MAX;
        self.inodes[idx as usize].lru_next = u16::MAX;
    }
    
    /// Touch entry (move to front)
    fn lru_touch(&mut self, idx: u16) {
        if idx == self.lru_head {
            return;
        }
        
        self.lru_remove(idx);
        self.lru_add_front(idx);
    }
    
    /// Iterate over dirty inodes
    pub fn dirty_iter(&self) -> impl Iterator<Item = &CachedInode> {
        self.inodes.iter().filter(|i| i.is_valid() && i.inode.flags.is_dirty())
    }
    
    /// Count dirty inodes
    pub fn dirty_count(&self) -> usize {
        self.inodes.iter().filter(|i| i.is_valid() && i.inode.flags.is_dirty()).count()
    }
    
    /// Get cache stats
    pub fn stats(&self) -> &InodeCacheStats {
        &self.stats
    }
    
    /// Get count
    pub fn count(&self) -> usize {
        self.count
    }
}

impl Default for InodeCache {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Inode Reference
// ============================================================================

/// Reference to a cached inode.
pub struct InodeRef<'a> {
    /// Cache reference
    cache: &'a mut InodeCache,
    /// Cache index
    index: usize,
}

impl<'a> InodeRef<'a> {
    /// Create new inode reference
    pub fn new(cache: &'a mut InodeCache, index: usize) -> Option<Self> {
        if cache.get(index).is_some() {
            cache.inodes[index].inode.refcount += 1;
            Some(Self { cache, index })
        } else {
            None
        }
    }
    
    /// Get inode
    pub fn inode(&self) -> &VfsInode {
        &self.cache.inodes[self.index].inode
    }
    
    /// Get mutable inode
    pub fn inode_mut(&mut self) -> &mut VfsInode {
        &mut self.cache.inodes[self.index].inode
    }
}

impl<'a> Drop for InodeRef<'a> {
    fn drop(&mut self) {
        if self.cache.inodes[self.index].inode.refcount > 0 {
            self.cache.inodes[self.index].inode.refcount -= 1;
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_inode_cache() {
        let mut cache = InodeCache::new();
        
        let inode = VfsInode::new_file(100, 0o644);
        cache.insert(inode).unwrap();
        
        assert_eq!(cache.count(), 1);
        
        let cached = cache.lookup(0, 100).unwrap();
        assert_eq!(cached.inode.ino, 100);
    }
    
    #[test]
    fn test_inode_cache_remove() {
        let mut cache = InodeCache::new();
        
        let inode = VfsInode::new_file(100, 0o644);
        cache.insert(inode).unwrap();
        
        let removed = cache.remove(0, 100);
        assert!(removed.is_some());
        assert_eq!(cache.count(), 0);
        
        assert!(cache.lookup(0, 100).is_none());
    }
    
    #[test]
    fn test_inode_cache_lru() {
        let mut cache = InodeCache::new();
        
        // Insert multiple inodes
        for i in 1..=10 {
            let inode = VfsInode::new_file(i, 0o644);
            cache.insert(inode).unwrap();
        }
        
        assert_eq!(cache.count(), 10);
        
        // Access oldest
        cache.lookup(0, 1);
        
        // Inode 1 should now be at front
        assert_eq!(cache.inodes[cache.lru_head as usize].inode.ino, 1);
    }
    
    #[test]
    fn test_inode_dirty() {
        let mut cache = InodeCache::new();
        
        let mut inode = VfsInode::new_file(100, 0o644);
        cache.insert(inode).unwrap();
        
        // Mark dirty
        let cached = cache.lookup(0, 100).unwrap();
        cached.inode.mark_dirty();
        
        assert_eq!(cache.dirty_count(), 1);
    }
}
