//! Buffer cache implementation.
//!
//! Fixed-size block buffer cache with hash table lookup
//! and LRU/CLOCK eviction.

use crate::cache::{CacheFlags, CacheHandle, CacheKey, CacheState, CacheStats};
use crate::core::atomic::{AtomicU32, AtomicU64, Ordering};
use crate::core::error::{HfsError, HfsResult};

// ============================================================================
// Constants
// ============================================================================

/// Maximum buffers in cache
pub const MAX_BUFFERS: usize = 65536;

/// Hash table size (power of 2)
pub const HASH_TABLE_SIZE: usize = 16384;

/// Hash table mask
pub const HASH_MASK: usize = HASH_TABLE_SIZE - 1;

/// Free list batch size
pub const FREE_LIST_BATCH: usize = 64;

/// Clock hand increment
pub const CLOCK_INCREMENT: u32 = 64;

// ============================================================================
// Buffer Header
// ============================================================================

/// Buffer header (metadata).
#[repr(C)]
pub struct BufferHeader {
    /// Cache key
    pub key: CacheKey,
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
    /// Access time
    pub access_time: AtomicU64,
    /// Modification time
    pub mod_time: AtomicU64,
    /// Write time (when marked dirty)
    pub write_time: AtomicU64,
    /// Data offset in buffer pool
    pub data_offset: u32,
    /// Data size
    pub data_size: u32,
}

impl BufferHeader {
    /// Create new header
    pub fn new(index: u32, block_size: u32) -> Self {
        Self {
            key: CacheKey::new(0, 0),
            state: AtomicU32::new(CacheState::Free as u32),
            flags: AtomicU32::new(0),
            refcount: AtomicU32::new(0),
            generation: AtomicU32::new(0),
            hash_next: AtomicU32::new(u32::MAX),
            lru_prev: AtomicU32::new(u32::MAX),
            lru_next: AtomicU32::new(u32::MAX),
            access_time: AtomicU64::new(0),
            mod_time: AtomicU64::new(0),
            write_time: AtomicU64::new(0),
            data_offset: index * block_size,
            data_size: block_size,
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

    /// Increment refcount
    #[inline]
    pub fn acquire(&self) -> u32 {
        self.refcount.fetch_add(1, Ordering::Acquire)
    }

    /// Decrement refcount
    #[inline]
    pub fn release(&self) -> u32 {
        self.refcount.fetch_sub(1, Ordering::Release)
    }

    /// Get refcount
    #[inline]
    pub fn get_refcount(&self) -> u32 {
        self.refcount.load(Ordering::Relaxed)
    }

    /// Is buffer free
    #[inline]
    pub fn is_free(&self) -> bool {
        self.get_state() == CacheState::Free
    }

    /// Is buffer dirty
    #[inline]
    pub fn is_dirty(&self) -> bool {
        self.get_state() == CacheState::Dirty
    }

    /// Is buffer pinned
    #[inline]
    pub fn is_pinned(&self) -> bool {
        self.get_flags().is_pinned()
    }

    /// Can evict buffer
    pub fn can_evict(&self) -> bool {
        let state = self.get_state();
        let refcount = self.get_refcount();
        let flags = self.get_flags();

        refcount == 0
            && !flags.is_pinned()
            && matches!(state, CacheState::Clean | CacheState::Dirty)
    }
}

// ============================================================================
// Hash Bucket
// ============================================================================

/// Hash bucket for buffer lookup.
#[repr(C, align(64))]
pub struct HashBucket {
    /// Head of chain
    pub head: AtomicU32,
    /// Lock
    pub lock: AtomicU32,
    /// Padding for cache line
    _pad: [u8; 56],
}

impl HashBucket {
    /// Create new bucket
    pub const fn new() -> Self {
        Self {
            head: AtomicU32::new(u32::MAX),
            lock: AtomicU32::new(0),
            _pad: [0; 56],
        }
    }

    /// Try to acquire lock
    pub fn try_lock(&self) -> bool {
        self.lock
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    }

    /// Spin lock
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
// Buffer Cache
// ============================================================================

/// Buffer cache manager.
pub struct BufferCache {
    /// Block size
    pub block_size: u32,
    /// Number of buffers
    pub buffer_count: u32,
    /// Hash table (fixed size)
    pub hash_table: [HashBucket; HASH_TABLE_SIZE],
    /// Buffer headers (fixed size)
    pub headers: [BufferHeader; MAX_BUFFERS],
    /// Free list head
    pub free_head: AtomicU32,
    /// Free list count
    pub free_count: AtomicU32,
    /// LRU head
    pub lru_head: AtomicU32,
    /// LRU tail
    pub lru_tail: AtomicU32,
    /// Clock hand
    pub clock_hand: AtomicU32,
    /// Dirty count
    pub dirty_count: AtomicU32,
    /// Statistics
    pub stats: CacheStats,
}

impl BufferCache {
    /// Hash function for key
    #[inline]
    fn hash_key(key: &CacheKey) -> usize {
        (key.hash() as usize) & HASH_MASK
    }

    /// Lookup buffer by key
    pub fn lookup(&self, key: &CacheKey) -> Option<CacheHandle> {
        let bucket_idx = Self::hash_key(key);
        let bucket = &self.hash_table[bucket_idx];

        bucket.lock();

        let mut idx = bucket.head.load(Ordering::Relaxed);
        while idx != u32::MAX {
            if (idx as usize) >= MAX_BUFFERS {
                break;
            }

            let header = &self.headers[idx as usize];
            if header.key == *key && !header.is_free() {
                header.acquire();
                header.set_flag(CacheFlags::ACCESSED);
                bucket.unlock();

                return Some(CacheHandle::new(
                    idx,
                    header.generation.load(Ordering::Relaxed),
                ));
            }

            idx = header.hash_next.load(Ordering::Relaxed);
        }

        bucket.unlock();
        None
    }

    /// Get buffer from free list
    fn get_free_buffer(&self) -> Option<u32> {
        loop {
            let head = self.free_head.load(Ordering::Acquire);
            if head == u32::MAX {
                return None;
            }

            if (head as usize) >= MAX_BUFFERS {
                return None;
            }

            let header = &self.headers[head as usize];
            let next = header.lru_next.load(Ordering::Relaxed);

            if self
                .free_head
                .compare_exchange(head, next, Ordering::Release, Ordering::Relaxed)
                .is_ok()
            {
                self.free_count.fetch_sub(1, Ordering::Relaxed);
                return Some(head);
            }
        }
    }

    /// Evict buffer using CLOCK algorithm
    pub fn evict_clock(&self) -> Option<u32> {
        let count = self.buffer_count as usize;
        if count == 0 {
            return None;
        }

        for _ in 0..(count * 2) {
            let hand = self.clock_hand.fetch_add(1, Ordering::Relaxed) as usize % count;
            let header = &self.headers[hand];

            if !header.can_evict() {
                continue;
            }

            // Check accessed bit
            let flags = header.get_flags();
            if flags.has(CacheFlags::ACCESSED) {
                header.clear_flag(CacheFlags::ACCESSED);
                continue;
            }

            // Try to acquire for eviction
            if header
                .refcount
                .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                // Double-check state
                if header.can_evict() || header.get_refcount() == 1 {
                    header.set_state(CacheState::Evicting);
                    return Some(hand as u32);
                }
                header.release();
            }
        }

        None
    }

    /// Insert buffer into hash table
    pub fn hash_insert(&self, idx: u32, key: &CacheKey) {
        let bucket_idx = Self::hash_key(key);
        let bucket = &self.hash_table[bucket_idx];
        let header = &self.headers[idx as usize];

        bucket.lock();

        let old_head = bucket.head.load(Ordering::Relaxed);
        header.hash_next.store(old_head, Ordering::Relaxed);
        bucket.head.store(idx, Ordering::Relaxed);

        bucket.unlock();
    }

    /// Remove buffer from hash table
    pub fn hash_remove(&self, idx: u32, key: &CacheKey) -> bool {
        let bucket_idx = Self::hash_key(key);
        let bucket = &self.hash_table[bucket_idx];

        bucket.lock();

        let mut prev_idx = u32::MAX;
        let mut cur_idx = bucket.head.load(Ordering::Relaxed);

        while cur_idx != u32::MAX {
            if cur_idx == idx {
                let header = &self.headers[cur_idx as usize];
                let next = header.hash_next.load(Ordering::Relaxed);

                if prev_idx == u32::MAX {
                    bucket.head.store(next, Ordering::Relaxed);
                } else {
                    self.headers[prev_idx as usize]
                        .hash_next
                        .store(next, Ordering::Relaxed);
                }

                bucket.unlock();
                return true;
            }

            prev_idx = cur_idx;
            cur_idx = self.headers[cur_idx as usize]
                .hash_next
                .load(Ordering::Relaxed);
        }

        bucket.unlock();
        false
    }

    /// Mark buffer as dirty
    pub fn mark_dirty(&self, handle: CacheHandle) -> HfsResult<()> {
        if !handle.is_valid() || (handle.index as usize) >= MAX_BUFFERS {
            return Err(HfsError::InvalidHandle);
        }

        let header = &self.headers[handle.index as usize];

        if header.generation.load(Ordering::Relaxed) != handle.generation {
            return Err(HfsError::InvalidHandle);
        }

        let old_state = header.get_state();
        if old_state != CacheState::Dirty {
            header.set_state(CacheState::Dirty);
            self.dirty_count.fetch_add(1, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Release buffer reference
    pub fn release(&self, handle: CacheHandle) {
        if !handle.is_valid() || (handle.index as usize) >= MAX_BUFFERS {
            return;
        }

        let header = &self.headers[handle.index as usize];
        header.release();
    }

    /// Get dirty buffer for writeback
    pub fn get_dirty_buffer(&self) -> Option<CacheHandle> {
        let count = self.buffer_count as usize;
        let start = self.clock_hand.load(Ordering::Relaxed) as usize;

        for i in 0..count {
            let idx = (start + i) % count;
            let header = &self.headers[idx];

            if header.is_dirty()
                && !header.is_pinned()
                && header
                    .refcount
                    .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
                    .is_ok()
            {
                if header.is_dirty() {
                    header.set_state(CacheState::Writing);
                    return Some(CacheHandle::new(
                        idx as u32,
                        header.generation.load(Ordering::Relaxed),
                    ));
                }
                header.release();
            }
        }

        None
    }

    /// Complete writeback
    pub fn complete_writeback(&self, handle: CacheHandle) {
        if !handle.is_valid() || (handle.index as usize) >= MAX_BUFFERS {
            return;
        }

        let header = &self.headers[handle.index as usize];

        if header.get_state() == CacheState::Writing {
            header.set_state(CacheState::Clean);
            self.dirty_count.fetch_sub(1, Ordering::Relaxed);
        }

        header.release();
    }

    /// Get dirty count
    #[inline]
    pub fn get_dirty_count(&self) -> u32 {
        self.dirty_count.load(Ordering::Relaxed)
    }

    /// Get free count
    #[inline]
    pub fn get_free_count(&self) -> u32 {
        self.free_count.load(Ordering::Relaxed)
    }

    /// Check if writeback needed
    pub fn needs_writeback(&self, max_dirty_ratio: u32) -> bool {
        if self.buffer_count == 0 {
            return false;
        }

        let dirty = self.get_dirty_count();
        let ratio = (dirty * 100) / self.buffer_count;
        ratio >= max_dirty_ratio
    }
}

// ============================================================================
// Buffer Reference
// ============================================================================

/// RAII buffer reference.
pub struct BufferRef<'a> {
    /// Cache
    pub cache: &'a BufferCache,
    /// Handle
    pub handle: CacheHandle,
    /// Header
    pub header: &'a BufferHeader,
}

impl<'a> BufferRef<'a> {
    /// Create new reference
    pub fn new(cache: &'a BufferCache, handle: CacheHandle) -> Option<Self> {
        if !handle.is_valid() || (handle.index as usize) >= MAX_BUFFERS {
            return None;
        }

        let header = &cache.headers[handle.index as usize];

        if header.generation.load(Ordering::Relaxed) != handle.generation {
            return None;
        }

        Some(Self {
            cache,
            handle,
            header,
        })
    }

    /// Get key
    #[inline]
    pub fn key(&self) -> &CacheKey {
        &self.header.key
    }

    /// Get state
    #[inline]
    pub fn state(&self) -> CacheState {
        self.header.get_state()
    }

    /// Is dirty
    #[inline]
    pub fn is_dirty(&self) -> bool {
        self.header.is_dirty()
    }

    /// Mark dirty
    pub fn mark_dirty(&self) -> HfsResult<()> {
        self.cache.mark_dirty(self.handle)
    }

    /// Get data offset
    #[inline]
    pub fn data_offset(&self) -> u32 {
        self.header.data_offset
    }

    /// Get data size
    #[inline]
    pub fn data_size(&self) -> u32 {
        self.header.data_size
    }
}

impl<'a> Drop for BufferRef<'a> {
    fn drop(&mut self) {
        self.cache.release(self.handle);
    }
}

// ============================================================================
// Dirty Buffer Iterator
// ============================================================================

/// Iterator over dirty buffers.
pub struct DirtyIterator<'a> {
    /// Cache
    cache: &'a BufferCache,
    /// Current index
    index: u32,
    /// Maximum to return
    max_count: u32,
    /// Returned count
    count: u32,
}

impl<'a> DirtyIterator<'a> {
    /// Create new iterator
    pub fn new(cache: &'a BufferCache, max_count: u32) -> Self {
        Self {
            cache,
            index: 0,
            max_count,
            count: 0,
        }
    }
}

impl<'a> Iterator for DirtyIterator<'a> {
    type Item = CacheHandle;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count >= self.max_count {
            return None;
        }

        while (self.index as usize) < MAX_BUFFERS && self.index < self.cache.buffer_count {
            let header = &self.cache.headers[self.index as usize];
            self.index += 1;

            if header.is_dirty() && !header.is_pinned() {
                // Try to acquire
                if header
                    .refcount
                    .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
                    .is_ok()
                {
                    if header.is_dirty() {
                        self.count += 1;
                        return Some(CacheHandle::new(
                            self.index - 1,
                            header.generation.load(Ordering::Relaxed),
                        ));
                    }
                    header.release();
                }
            }
        }

        None
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_header() {
        let header = BufferHeader::new(0, 4096);

        assert!(header.is_free());
        assert!(!header.is_dirty());

        header.set_state(CacheState::Dirty);
        assert!(header.is_dirty());

        header.acquire();
        assert_eq!(header.get_refcount(), 1);

        header.release();
        assert_eq!(header.get_refcount(), 0);
    }

    #[test]
    fn test_hash_bucket() {
        let bucket = HashBucket::new();

        assert!(bucket.try_lock());
        assert!(!bucket.try_lock()); // Already locked
        bucket.unlock();
        assert!(bucket.try_lock()); // Now available
    }

    #[test]
    fn test_cache_handle() {
        let handle = CacheHandle::new(42, 1);
        assert!(handle.is_valid());
        assert_eq!(handle.index, 42);
        assert_eq!(handle.generation, 1);
    }
}
