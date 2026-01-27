//! Page cache implementation.
//!
//! Page cache for file data with read-ahead and
//! write coalescing support.

use crate::core::types::*;
use crate::core::error::{HfsError, HfsResult};
use crate::core::atomic::{AtomicU32, AtomicU64, Ordering};
use crate::cache::{CacheState, CacheFlags, CacheHandle, CacheStats};

// ============================================================================
// Constants
// ============================================================================

/// Page size (4 KB)
pub const PAGE_SIZE: usize = 4096;

/// Maximum pages in cache
pub const MAX_PAGES: usize = 262144; // 1 GB at 4KB pages

/// Page hash table size
pub const PAGE_HASH_SIZE: usize = 16384;

/// Page hash mask
pub const PAGE_HASH_MASK: usize = PAGE_HASH_SIZE - 1;

/// Readahead window (pages)
pub const READAHEAD_WINDOW: u32 = 32;

/// Maximum readahead pages
pub const MAX_READAHEAD: u32 = 128;

/// Minimum sequential accesses to trigger readahead
pub const READAHEAD_THRESHOLD: u32 = 2;

// ============================================================================
// Page Key
// ============================================================================

/// Key for page identification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PageKey {
    /// Inode number
    pub ino: u64,
    /// Page index (offset / PAGE_SIZE)
    pub index: u64,
}

impl PageKey {
    /// Create new key
    pub const fn new(ino: u64, index: u64) -> Self {
        Self { ino, index }
    }
    
    /// From file offset
    pub fn from_offset(ino: u64, offset: u64) -> Self {
        Self {
            ino,
            index: offset / PAGE_SIZE as u64,
        }
    }
    
    /// Hash for lookup
    pub fn hash(&self) -> u64 {
        // FNV-1a
        let mut h: u64 = 0xcbf29ce484222325;
        for b in self.ino.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        for b in self.index.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }
}

// ============================================================================
// Page Descriptor
// ============================================================================

/// Page descriptor.
#[repr(C)]
pub struct PageDescriptor {
    /// Page key
    pub key: PageKey,
    /// State
    pub state: AtomicU32,
    /// Flags
    pub flags: AtomicU32,
    /// Reference count
    pub refcount: AtomicU32,
    /// Generation
    pub generation: AtomicU32,
    /// Hash chain next
    pub hash_next: AtomicU32,
    /// LRU prev
    pub lru_prev: AtomicU32,
    /// LRU next
    pub lru_next: AtomicU32,
    /// Page frame number (index in page pool)
    pub pfn: u32,
    /// Access time
    pub atime: AtomicU64,
    /// Modification time
    pub mtime: AtomicU64,
    /// Private data
    pub private: AtomicU64,
}

impl PageDescriptor {
    /// Create new descriptor
    pub fn new(pfn: u32) -> Self {
        Self {
            key: PageKey::new(0, 0),
            state: AtomicU32::new(CacheState::Free as u32),
            flags: AtomicU32::new(0),
            refcount: AtomicU32::new(0),
            generation: AtomicU32::new(0),
            hash_next: AtomicU32::new(u32::MAX),
            lru_prev: AtomicU32::new(u32::MAX),
            lru_next: AtomicU32::new(u32::MAX),
            pfn,
            atime: AtomicU64::new(0),
            mtime: AtomicU64::new(0),
            private: AtomicU64::new(0),
        }
    }
    
    /// Get state
    #[inline]
    pub fn get_state(&self) -> CacheState {
        CacheState::from_raw(self.state.load(Ordering::Acquire) as u8)
    }
    
    /// Set state
    #[inline]
    pub fn set_state(&self, state: CacheState) {
        self.state.store(state as u32, Ordering::Release);
    }
    
    /// Get flags
    #[inline]
    pub fn get_flags(&self) -> CacheFlags {
        CacheFlags(self.flags.load(Ordering::Relaxed) as u16)
    }
    
    /// Set flag
    #[inline]
    pub fn set_flag(&self, flag: u16) {
        self.flags.fetch_or(flag as u32, Ordering::Relaxed);
    }
    
    /// Clear flag
    #[inline]
    pub fn clear_flag(&self, flag: u16) {
        self.flags.fetch_and(!(flag as u32), Ordering::Relaxed);
    }
    
    /// Acquire reference
    #[inline]
    pub fn acquire(&self) -> u32 {
        self.refcount.fetch_add(1, Ordering::Acquire)
    }
    
    /// Release reference
    #[inline]
    pub fn release(&self) -> u32 {
        self.refcount.fetch_sub(1, Ordering::Release)
    }
    
    /// Get refcount
    #[inline]
    pub fn get_refcount(&self) -> u32 {
        self.refcount.load(Ordering::Relaxed)
    }
    
    /// Is free
    #[inline]
    pub fn is_free(&self) -> bool {
        self.get_state() == CacheState::Free
    }
    
    /// Is dirty
    #[inline]
    pub fn is_dirty(&self) -> bool {
        self.get_state() == CacheState::Dirty
    }
    
    /// Is pinned
    #[inline]
    pub fn is_pinned(&self) -> bool {
        self.get_flags().is_pinned()
    }
    
    /// Can evict
    pub fn can_evict(&self) -> bool {
        self.get_refcount() == 0 && 
        !self.is_pinned() &&
        self.get_state().is_usable()
    }
}

// ============================================================================
// Page Hash Bucket
// ============================================================================

/// Page hash bucket.
#[repr(C, align(64))]
pub struct PageHashBucket {
    /// Head of chain
    pub head: AtomicU32,
    /// Lock
    pub lock: AtomicU32,
    /// Padding
    _pad: [u8; 56],
}

impl PageHashBucket {
    /// Create new bucket
    pub const fn new() -> Self {
        Self {
            head: AtomicU32::new(u32::MAX),
            lock: AtomicU32::new(0),
            _pad: [0; 56],
        }
    }
    
    /// Try lock
    pub fn try_lock(&self) -> bool {
        self.lock.compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed).is_ok()
    }
    
    /// Lock
    pub fn lock(&self) {
        while !self.try_lock() {
            core::hint::spin_loop();
        }
    }
    
    /// Unlock
    pub fn unlock(&self) {
        self.lock.store(0, Ordering::Release);
    }
}

// ============================================================================
// Readahead State
// ============================================================================

/// Readahead state for an inode.
#[derive(Clone, Copy, Debug)]
pub struct ReadaheadState {
    /// Inode
    pub ino: u64,
    /// Start of current readahead window
    pub start: u64,
    /// Size of readahead (pages)
    pub size: u32,
    /// Previous request start
    pub prev_start: u64,
    /// Sequential access count
    pub seq_count: u32,
    /// Async readahead trigger point
    pub async_size: u32,
}

impl ReadaheadState {
    /// Create new state
    pub const fn new(ino: u64) -> Self {
        Self {
            ino,
            start: 0,
            size: READAHEAD_WINDOW,
            prev_start: 0,
            seq_count: 0,
            async_size: 0,
        }
    }
    
    /// Update on access
    pub fn on_access(&mut self, page_index: u64) {
        if page_index == self.prev_start + 1 {
            // Sequential access
            self.seq_count = self.seq_count.saturating_add(1);
            
            // Increase readahead if consistently sequential
            if self.seq_count >= READAHEAD_THRESHOLD {
                self.size = core::cmp::min(self.size * 2, MAX_READAHEAD);
            }
        } else if page_index != self.prev_start {
            // Non-sequential, reset
            self.seq_count = 0;
            self.size = READAHEAD_WINDOW;
        }
        
        self.prev_start = page_index;
    }
    
    /// Should trigger async readahead
    pub fn should_async_readahead(&self, page_index: u64) -> bool {
        if self.async_size == 0 {
            return false;
        }
        
        // Trigger when entering async zone
        page_index >= self.start + (self.size as u64 - self.async_size as u64)
    }
    
    /// Get readahead range
    pub fn get_readahead_range(&self, page_index: u64) -> (u64, u32) {
        let start = page_index + 1;
        (start, self.size)
    }
}

// ============================================================================
// Page Range
// ============================================================================

/// Range of pages.
#[derive(Clone, Copy, Debug)]
pub struct PageRange {
    /// Start index
    pub start: u64,
    /// End index (exclusive)
    pub end: u64,
}

impl PageRange {
    /// Create new range
    pub const fn new(start: u64, end: u64) -> Self {
        Self { start, end }
    }
    
    /// From byte range
    pub fn from_bytes(offset: u64, len: u64) -> Self {
        let start = offset / PAGE_SIZE as u64;
        let end = (offset + len + PAGE_SIZE as u64 - 1) / PAGE_SIZE as u64;
        Self { start, end }
    }
    
    /// Number of pages
    #[inline]
    pub fn count(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }
    
    /// Is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }
    
    /// Contains index
    #[inline]
    pub fn contains(&self, index: u64) -> bool {
        index >= self.start && index < self.end
    }
    
    /// Iterator
    pub fn iter(&self) -> PageRangeIter {
        PageRangeIter {
            current: self.start,
            end: self.end,
        }
    }
}

/// Iterator over page range.
pub struct PageRangeIter {
    current: u64,
    end: u64,
}

impl Iterator for PageRangeIter {
    type Item = u64;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.end {
            let idx = self.current;
            self.current += 1;
            Some(idx)
        } else {
            None
        }
    }
}

// ============================================================================
// Page Cache
// ============================================================================

/// Page cache manager.
pub struct PageCache {
    /// Page descriptors
    pub pages: [PageDescriptor; MAX_PAGES],
    /// Hash table
    pub hash_table: [PageHashBucket; PAGE_HASH_SIZE],
    /// Free list head
    pub free_head: AtomicU32,
    /// Free count
    pub free_count: AtomicU32,
    /// LRU head
    pub lru_head: AtomicU32,
    /// LRU tail
    pub lru_tail: AtomicU32,
    /// Active pages
    pub active_count: AtomicU32,
    /// Dirty pages
    pub dirty_count: AtomicU32,
    /// Clock hand
    pub clock_hand: AtomicU32,
    /// Statistics
    pub stats: CacheStats,
}

impl PageCache {
    /// Hash function
    #[inline]
    fn hash_key(key: &PageKey) -> usize {
        (key.hash() as usize) & PAGE_HASH_MASK
    }
    
    /// Lookup page
    pub fn lookup(&self, key: &PageKey) -> Option<CacheHandle> {
        let bucket_idx = Self::hash_key(key);
        let bucket = &self.hash_table[bucket_idx];
        
        bucket.lock();
        
        let mut idx = bucket.head.load(Ordering::Relaxed);
        while idx != u32::MAX {
            if (idx as usize) >= MAX_PAGES {
                break;
            }
            
            let page = &self.pages[idx as usize];
            if page.key == *key && !page.is_free() {
                page.acquire();
                page.set_flag(CacheFlags::ACCESSED);
                bucket.unlock();
                
                return Some(CacheHandle::new(
                    idx,
                    page.generation.load(Ordering::Relaxed),
                ));
            }
            
            idx = page.hash_next.load(Ordering::Relaxed);
        }
        
        bucket.unlock();
        None
    }
    
    /// Find or create page
    pub fn find_or_create(&self, key: &PageKey) -> HfsResult<(CacheHandle, bool)> {
        // First try lookup
        if let Some(handle) = self.lookup(key) {
            return Ok((handle, true));
        }
        
        // Need to allocate
        let idx = self.alloc_page().ok_or(HfsError::NoSpace)?;
        let page = &self.pages[idx as usize];
        
        // Initialize (note: would need unsafe for key modification)
        page.set_state(CacheState::Reading);
        page.acquire();
        
        // Add to hash
        self.hash_insert(idx, key);
        
        let handle = CacheHandle::new(idx, page.generation.load(Ordering::Relaxed));
        Ok((handle, false))
    }
    
    /// Allocate page from free list
    fn alloc_page(&self) -> Option<u32> {
        // Try free list first
        loop {
            let head = self.free_head.load(Ordering::Acquire);
            if head == u32::MAX {
                break;
            }
            
            if (head as usize) >= MAX_PAGES {
                break;
            }
            
            let page = &self.pages[head as usize];
            let next = page.lru_next.load(Ordering::Relaxed);
            
            if self.free_head.compare_exchange(
                head, next,
                Ordering::Release,
                Ordering::Relaxed
            ).is_ok() {
                self.free_count.fetch_sub(1, Ordering::Relaxed);
                page.generation.fetch_add(1, Ordering::Relaxed);
                return Some(head);
            }
        }
        
        // Try eviction
        self.evict_page()
    }
    
    /// Evict page using clock algorithm
    fn evict_page(&self) -> Option<u32> {
        let count = MAX_PAGES;
        
        for _ in 0..(count * 2) {
            let hand = self.clock_hand.fetch_add(1, Ordering::Relaxed) as usize % count;
            let page = &self.pages[hand];
            
            if !page.can_evict() {
                continue;
            }
            
            // Check accessed flag
            if page.get_flags().has(CacheFlags::ACCESSED) {
                page.clear_flag(CacheFlags::ACCESSED);
                continue;
            }
            
            // Try to acquire
            if page.refcount.compare_exchange(
                0, 1,
                Ordering::Acquire,
                Ordering::Relaxed
            ).is_ok() {
                // Remove from hash
                self.hash_remove(hand as u32, &page.key);
                
                // If dirty, would need to write back first
                if page.is_dirty() {
                    self.dirty_count.fetch_sub(1, Ordering::Relaxed);
                }
                
                page.generation.fetch_add(1, Ordering::Relaxed);
                self.active_count.fetch_sub(1, Ordering::Relaxed);
                
                return Some(hand as u32);
            }
        }
        
        None
    }
    
    /// Insert into hash table
    fn hash_insert(&self, idx: u32, key: &PageKey) {
        let bucket_idx = Self::hash_key(key);
        let bucket = &self.hash_table[bucket_idx];
        let page = &self.pages[idx as usize];
        
        bucket.lock();
        
        let old_head = bucket.head.load(Ordering::Relaxed);
        page.hash_next.store(old_head, Ordering::Relaxed);
        bucket.head.store(idx, Ordering::Release);
        
        bucket.unlock();
        
        self.active_count.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Remove from hash table
    fn hash_remove(&self, idx: u32, key: &PageKey) -> bool {
        let bucket_idx = Self::hash_key(key);
        let bucket = &self.hash_table[bucket_idx];
        
        bucket.lock();
        
        let mut prev_idx = u32::MAX;
        let mut cur_idx = bucket.head.load(Ordering::Relaxed);
        
        while cur_idx != u32::MAX {
            if cur_idx == idx {
                let page = &self.pages[cur_idx as usize];
                let next = page.hash_next.load(Ordering::Relaxed);
                
                if prev_idx == u32::MAX {
                    bucket.head.store(next, Ordering::Relaxed);
                } else {
                    self.pages[prev_idx as usize].hash_next.store(next, Ordering::Relaxed);
                }
                
                bucket.unlock();
                return true;
            }
            
            prev_idx = cur_idx;
            cur_idx = self.pages[cur_idx as usize].hash_next.load(Ordering::Relaxed);
        }
        
        bucket.unlock();
        false
    }
    
    /// Mark page dirty
    pub fn mark_dirty(&self, handle: CacheHandle) -> HfsResult<()> {
        if !handle.is_valid() || (handle.index as usize) >= MAX_PAGES {
            return Err(HfsError::InvalidHandle);
        }
        
        let page = &self.pages[handle.index as usize];
        
        if page.generation.load(Ordering::Relaxed) != handle.generation {
            return Err(HfsError::InvalidHandle);
        }
        
        if page.get_state() != CacheState::Dirty {
            page.set_state(CacheState::Dirty);
            self.dirty_count.fetch_add(1, Ordering::Relaxed);
        }
        
        Ok(())
    }
    
    /// Complete page read
    pub fn complete_read(&self, handle: CacheHandle) {
        if !handle.is_valid() || (handle.index as usize) >= MAX_PAGES {
            return;
        }
        
        let page = &self.pages[handle.index as usize];
        
        if page.get_state() == CacheState::Reading {
            page.set_state(CacheState::Clean);
        }
    }
    
    /// Release page
    pub fn release(&self, handle: CacheHandle) {
        if !handle.is_valid() || (handle.index as usize) >= MAX_PAGES {
            return;
        }
        
        self.pages[handle.index as usize].release();
    }
    
    /// Invalidate pages for inode
    pub fn invalidate_inode(&self, ino: u64) -> u32 {
        let mut count = 0;
        
        for i in 0..MAX_PAGES {
            let page = &self.pages[i];
            
            if page.key.ino == ino && !page.is_free() {
                if page.refcount.compare_exchange(
                    0, 1,
                    Ordering::Acquire,
                    Ordering::Relaxed
                ).is_ok() {
                    self.hash_remove(i as u32, &page.key);
                    
                    if page.is_dirty() {
                        self.dirty_count.fetch_sub(1, Ordering::Relaxed);
                    }
                    
                    page.set_state(CacheState::Free);
                    page.release();
                    self.active_count.fetch_sub(1, Ordering::Relaxed);
                    count += 1;
                }
            }
        }
        
        count
    }
    
    /// Get dirty count
    #[inline]
    pub fn get_dirty_count(&self) -> u32 {
        self.dirty_count.load(Ordering::Relaxed)
    }
    
    /// Get active count
    #[inline]
    pub fn get_active_count(&self) -> u32 {
        self.active_count.load(Ordering::Relaxed)
    }
    
    /// Get free count
    #[inline]
    pub fn get_free_count(&self) -> u32 {
        self.free_count.load(Ordering::Relaxed)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_page_key() {
        let key1 = PageKey::new(1, 100);
        let key2 = PageKey::from_offset(1, 409600); // 100 * 4096
        
        assert_eq!(key1, key2);
        assert_ne!(key1.hash(), PageKey::new(2, 100).hash());
    }
    
    #[test]
    fn test_page_descriptor() {
        let page = PageDescriptor::new(0);
        
        assert!(page.is_free());
        assert!(!page.is_dirty());
        
        page.set_state(CacheState::Clean);
        assert!(!page.is_free());
        
        page.acquire();
        assert_eq!(page.get_refcount(), 1);
        
        page.release();
        assert_eq!(page.get_refcount(), 0);
    }
    
    #[test]
    fn test_readahead_state() {
        let mut ra = ReadaheadState::new(1);
        
        // Simulate sequential access
        ra.on_access(0);
        ra.on_access(1);
        ra.on_access(2);
        
        assert!(ra.seq_count >= READAHEAD_THRESHOLD);
        assert!(ra.size > READAHEAD_WINDOW);
    }
    
    #[test]
    fn test_page_range() {
        let range = PageRange::from_bytes(0, 8192);
        assert_eq!(range.start, 0);
        assert_eq!(range.end, 2);
        assert_eq!(range.count(), 2);
        
        assert!(range.contains(0));
        assert!(range.contains(1));
        assert!(!range.contains(2));
    }
    
    #[test]
    fn test_page_range_iter() {
        let range = PageRange::new(5, 8);
        let pages: Vec<_> = range.iter().collect();
        
        assert_eq!(pages, vec![5, 6, 7]);
    }
}
