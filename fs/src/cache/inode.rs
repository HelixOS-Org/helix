//! Inode cache implementation.
//!
//! Caches inode metadata for fast access without disk I/O.

use crate::cache::{CacheFlags, CacheHandle, CacheState};
use crate::core::atomic::{AtomicU32, AtomicU64, Ordering};
use crate::core::error::{HfsError, HfsResult};

// ============================================================================
// Constants
// ============================================================================

/// Maximum cached inodes
pub const MAX_CACHED_INODES: usize = 65536;

/// Inode hash table size
pub const INODE_HASH_SIZE: usize = 8192;

/// Inode hash mask
pub const INODE_HASH_MASK: usize = INODE_HASH_SIZE - 1;

/// Unused inode number
pub const INODE_UNUSED: u64 = 0;

// ============================================================================
// Inode Type
// ============================================================================

/// Inode type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum InodeType {
    /// Unknown/invalid
    Unknown   = 0,
    /// Regular file
    File      = 1,
    /// Directory
    Directory = 2,
    /// Symbolic link
    Symlink   = 3,
    /// Block device
    BlockDev  = 4,
    /// Character device
    CharDev   = 5,
    /// FIFO/pipe
    Fifo      = 6,
    /// Socket
    Socket    = 7,
}

impl InodeType {
    /// From raw value
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::File,
            2 => Self::Directory,
            3 => Self::Symlink,
            4 => Self::BlockDev,
            5 => Self::CharDev,
            6 => Self::Fifo,
            7 => Self::Socket,
            _ => Self::Unknown,
        }
    }

    /// Is regular file
    #[inline]
    pub fn is_file(&self) -> bool {
        *self == Self::File
    }

    /// Is directory
    #[inline]
    pub fn is_dir(&self) -> bool {
        *self == Self::Directory
    }

    /// Is symlink
    #[inline]
    pub fn is_symlink(&self) -> bool {
        *self == Self::Symlink
    }
}

impl Default for InodeType {
    fn default() -> Self {
        Self::Unknown
    }
}

// ============================================================================
// Inode Flags
// ============================================================================

/// Inode flags.
#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub struct InodeFlags(pub u32);

impl InodeFlags {
    /// Inode is dirty
    pub const DIRTY: u32 = 1 << 0;
    /// Inode data is dirty
    pub const DATA_DIRTY: u32 = 1 << 1;
    /// Inode is new (not on disk)
    pub const NEW: u32 = 1 << 2;
    /// Inode is being deleted
    pub const DELETING: u32 = 1 << 3;
    /// Inode is locked
    pub const LOCKED: u32 = 1 << 4;
    /// Inode has pending I/O
    pub const IO_PENDING: u32 = 1 << 5;
    /// Inode is pinned
    pub const PINNED: u32 = 1 << 6;
    /// Inode is immutable
    pub const IMMUTABLE: u32 = 1 << 7;
    /// Inode is append-only
    pub const APPEND_ONLY: u32 = 1 << 8;
    /// Inode has extended attributes
    pub const HAS_XATTR: u32 = 1 << 9;
    /// Inode uses inline data
    pub const INLINE_DATA: u32 = 1 << 10;
    /// Inode has been sync'd
    pub const SYNCED: u32 = 1 << 11;

    /// Create empty
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Check flag
    #[inline]
    pub fn has(&self, flag: u32) -> bool {
        self.0 & flag != 0
    }

    /// Set flag
    #[inline]
    pub fn set(&mut self, flag: u32) {
        self.0 |= flag;
    }

    /// Clear flag
    #[inline]
    pub fn clear(&mut self, flag: u32) {
        self.0 &= !flag;
    }

    /// Is dirty
    #[inline]
    pub fn is_dirty(&self) -> bool {
        self.has(Self::DIRTY)
    }

    /// Is pinned
    #[inline]
    pub fn is_pinned(&self) -> bool {
        self.has(Self::PINNED)
    }
}

// ============================================================================
// Cached Inode Data
// ============================================================================

/// Cached inode metadata.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CachedInodeData {
    /// Inode number
    pub ino: u64,
    /// Inode type
    pub itype: InodeType,
    /// File mode (permissions)
    pub mode: u16,
    /// Link count
    pub nlink: u32,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
    /// File size
    pub size: u64,
    /// Block count
    pub blocks: u64,
    /// Access time (ns)
    pub atime: u64,
    /// Modification time (ns)
    pub mtime: u64,
    /// Change time (ns)
    pub ctime: u64,
    /// Creation time (ns)
    pub crtime: u64,
    /// Flags
    pub flags: InodeFlags,
    /// Generation
    pub generation: u32,
    /// Extended attribute block
    pub xattr_block: u64,
    /// Parent inode (for directory traversal)
    pub parent: u64,
}

impl CachedInodeData {
    /// Create new empty data
    pub const fn new() -> Self {
        Self {
            ino: 0,
            itype: InodeType::Unknown,
            mode: 0,
            nlink: 0,
            uid: 0,
            gid: 0,
            size: 0,
            blocks: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            crtime: 0,
            flags: InodeFlags(0),
            generation: 0,
            xattr_block: 0,
            parent: 0,
        }
    }

    /// Is valid
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.ino != 0 && self.itype != InodeType::Unknown
    }

    /// Is directory
    #[inline]
    pub fn is_dir(&self) -> bool {
        self.itype.is_dir()
    }

    /// Is regular file
    #[inline]
    pub fn is_file(&self) -> bool {
        self.itype.is_file()
    }

    /// Is symlink
    #[inline]
    pub fn is_symlink(&self) -> bool {
        self.itype.is_symlink()
    }
}

impl Default for CachedInodeData {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Inode Cache Entry
// ============================================================================

/// Inode cache entry.
#[repr(C)]
pub struct InodeCacheEntry {
    /// Inode data
    pub data: CachedInodeData,
    /// State
    pub state: AtomicU32,
    /// Cache flags
    pub cache_flags: AtomicU32,
    /// Reference count
    pub refcount: AtomicU32,
    /// Cache generation
    pub cache_gen: AtomicU32,
    /// Hash chain next
    pub hash_next: AtomicU32,
    /// LRU prev
    pub lru_prev: AtomicU32,
    /// LRU next
    pub lru_next: AtomicU32,
    /// Last access time
    pub last_access: AtomicU64,
    /// Dirty time (when marked dirty)
    pub dirty_time: AtomicU64,
}

impl InodeCacheEntry {
    /// Create new entry
    pub fn new() -> Self {
        Self {
            data: CachedInodeData::new(),
            state: AtomicU32::new(CacheState::Free as u32),
            cache_flags: AtomicU32::new(0),
            refcount: AtomicU32::new(0),
            cache_gen: AtomicU32::new(0),
            hash_next: AtomicU32::new(u32::MAX),
            lru_prev: AtomicU32::new(u32::MAX),
            lru_next: AtomicU32::new(u32::MAX),
            last_access: AtomicU64::new(0),
            dirty_time: AtomicU64::new(0),
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

    /// Get cache flags
    #[inline]
    pub fn get_cache_flags(&self) -> CacheFlags {
        CacheFlags(self.cache_flags.load(Ordering::Relaxed) as u16)
    }

    /// Set cache flag
    #[inline]
    pub fn set_cache_flag(&self, flag: u16) {
        self.cache_flags.fetch_or(flag as u32, Ordering::Relaxed);
    }

    /// Clear cache flag
    #[inline]
    pub fn clear_cache_flag(&self, flag: u16) {
        self.cache_flags
            .fetch_and(!(flag as u32), Ordering::Relaxed);
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
        self.get_state() == CacheState::Dirty || self.data.flags.is_dirty()
    }

    /// Is pinned
    #[inline]
    pub fn is_pinned(&self) -> bool {
        self.get_cache_flags().is_pinned()
    }

    /// Can evict
    pub fn can_evict(&self) -> bool {
        self.get_refcount() == 0 && !self.is_pinned() && self.get_state().is_usable()
    }
}

impl Default for InodeCacheEntry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Inode Hash Bucket
// ============================================================================

/// Inode hash bucket.
#[repr(C, align(64))]
pub struct InodeHashBucket {
    /// Head of chain
    pub head: AtomicU32,
    /// Lock
    pub lock: AtomicU32,
    /// Padding
    _pad: [u8; 56],
}

impl InodeHashBucket {
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
        self.lock
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
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
// Inode Cache
// ============================================================================

/// Inode cache manager.
pub struct InodeCache {
    /// Cached entries
    pub entries: [InodeCacheEntry; MAX_CACHED_INODES],
    /// Hash table
    pub hash_table: [InodeHashBucket; INODE_HASH_SIZE],
    /// Free list head
    pub free_head: AtomicU32,
    /// Free count
    pub free_count: AtomicU32,
    /// LRU head
    pub lru_head: AtomicU32,
    /// LRU tail
    pub lru_tail: AtomicU32,
    /// Active count
    pub active_count: AtomicU32,
    /// Dirty count
    pub dirty_count: AtomicU32,
    /// Clock hand
    pub clock_hand: AtomicU32,
    /// Lookups
    pub lookups: AtomicU64,
    /// Hits
    pub hits: AtomicU64,
}

impl InodeCache {
    /// Hash function for inode number
    #[inline]
    fn hash_ino(ino: u64) -> usize {
        // Mix bits of inode number
        let h = ino.wrapping_mul(0x517cc1b727220a95);
        (h as usize) & INODE_HASH_MASK
    }

    /// Lookup inode by number
    pub fn lookup(&self, ino: u64) -> Option<CacheHandle> {
        self.lookups.fetch_add(1, Ordering::Relaxed);

        let bucket_idx = Self::hash_ino(ino);
        let bucket = &self.hash_table[bucket_idx];

        bucket.lock();

        let mut idx = bucket.head.load(Ordering::Relaxed);
        while idx != u32::MAX {
            if (idx as usize) >= MAX_CACHED_INODES {
                break;
            }

            let entry = &self.entries[idx as usize];
            if entry.data.ino == ino && !entry.is_free() {
                entry.acquire();
                entry.set_cache_flag(CacheFlags::ACCESSED);
                bucket.unlock();

                self.hits.fetch_add(1, Ordering::Relaxed);

                return Some(CacheHandle::new(
                    idx,
                    entry.cache_gen.load(Ordering::Relaxed),
                ));
            }

            idx = entry.hash_next.load(Ordering::Relaxed);
        }

        bucket.unlock();
        None
    }

    /// Find or create inode entry
    pub fn find_or_create(&self, ino: u64) -> HfsResult<(CacheHandle, bool)> {
        // First try lookup
        if let Some(handle) = self.lookup(ino) {
            return Ok((handle, true));
        }

        // Need to allocate
        let idx = self.alloc_entry().ok_or(HfsError::NoSpace)?;
        let entry = &self.entries[idx as usize];

        // Note: would need unsafe to modify data.ino
        entry.set_state(CacheState::Reading);
        entry.acquire();

        // Add to hash
        self.hash_insert(idx, ino);

        let handle = CacheHandle::new(idx, entry.cache_gen.load(Ordering::Relaxed));
        Ok((handle, false))
    }

    /// Allocate entry
    fn alloc_entry(&self) -> Option<u32> {
        // Try free list
        loop {
            let head = self.free_head.load(Ordering::Acquire);
            if head == u32::MAX {
                break;
            }

            if (head as usize) >= MAX_CACHED_INODES {
                break;
            }

            let entry = &self.entries[head as usize];
            let next = entry.lru_next.load(Ordering::Relaxed);

            if self
                .free_head
                .compare_exchange(head, next, Ordering::Release, Ordering::Relaxed)
                .is_ok()
            {
                self.free_count.fetch_sub(1, Ordering::Relaxed);
                entry.cache_gen.fetch_add(1, Ordering::Relaxed);
                return Some(head);
            }
        }

        // Try eviction
        self.evict_entry()
    }

    /// Evict entry using clock algorithm
    fn evict_entry(&self) -> Option<u32> {
        let count = MAX_CACHED_INODES;

        for _ in 0..(count * 2) {
            let hand = self.clock_hand.fetch_add(1, Ordering::Relaxed) as usize % count;
            let entry = &self.entries[hand];

            if !entry.can_evict() {
                continue;
            }

            // Check accessed
            if entry.get_cache_flags().has(CacheFlags::ACCESSED) {
                entry.clear_cache_flag(CacheFlags::ACCESSED);
                continue;
            }

            // Try to acquire
            if entry
                .refcount
                .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                let ino = entry.data.ino;

                // Remove from hash
                self.hash_remove(hand as u32, ino);

                if entry.is_dirty() {
                    self.dirty_count.fetch_sub(1, Ordering::Relaxed);
                }

                entry.cache_gen.fetch_add(1, Ordering::Relaxed);
                self.active_count.fetch_sub(1, Ordering::Relaxed);

                return Some(hand as u32);
            }
        }

        None
    }

    /// Insert into hash table
    fn hash_insert(&self, idx: u32, ino: u64) {
        let bucket_idx = Self::hash_ino(ino);
        let bucket = &self.hash_table[bucket_idx];
        let entry = &self.entries[idx as usize];

        bucket.lock();

        let old_head = bucket.head.load(Ordering::Relaxed);
        entry.hash_next.store(old_head, Ordering::Relaxed);
        bucket.head.store(idx, Ordering::Release);

        bucket.unlock();

        self.active_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Remove from hash table
    fn hash_remove(&self, idx: u32, ino: u64) -> bool {
        let bucket_idx = Self::hash_ino(ino);
        let bucket = &self.hash_table[bucket_idx];

        bucket.lock();

        let mut prev_idx = u32::MAX;
        let mut cur_idx = bucket.head.load(Ordering::Relaxed);

        while cur_idx != u32::MAX {
            if cur_idx == idx {
                let entry = &self.entries[cur_idx as usize];
                let next = entry.hash_next.load(Ordering::Relaxed);

                if prev_idx == u32::MAX {
                    bucket.head.store(next, Ordering::Relaxed);
                } else {
                    self.entries[prev_idx as usize]
                        .hash_next
                        .store(next, Ordering::Relaxed);
                }

                bucket.unlock();
                return true;
            }

            prev_idx = cur_idx;
            cur_idx = self.entries[cur_idx as usize]
                .hash_next
                .load(Ordering::Relaxed);
        }

        bucket.unlock();
        false
    }

    /// Get inode data
    pub fn get_data(&self, handle: CacheHandle) -> Option<&CachedInodeData> {
        if !handle.is_valid() || (handle.index as usize) >= MAX_CACHED_INODES {
            return None;
        }

        let entry = &self.entries[handle.index as usize];

        if entry.cache_gen.load(Ordering::Relaxed) != handle.generation {
            return None;
        }

        Some(&entry.data)
    }

    /// Mark inode dirty
    pub fn mark_dirty(&self, handle: CacheHandle) -> HfsResult<()> {
        if !handle.is_valid() || (handle.index as usize) >= MAX_CACHED_INODES {
            return Err(HfsError::InvalidHandle);
        }

        let entry = &self.entries[handle.index as usize];

        if entry.cache_gen.load(Ordering::Relaxed) != handle.generation {
            return Err(HfsError::InvalidHandle);
        }

        if entry.get_state() != CacheState::Dirty {
            entry.set_state(CacheState::Dirty);
            self.dirty_count.fetch_add(1, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Release inode
    pub fn release(&self, handle: CacheHandle) {
        if !handle.is_valid() || (handle.index as usize) >= MAX_CACHED_INODES {
            return;
        }

        self.entries[handle.index as usize].release();
    }

    /// Get hit rate
    pub fn hit_rate(&self) -> f32 {
        let lookups = self.lookups.load(Ordering::Relaxed);
        if lookups == 0 {
            return 0.0;
        }
        let hits = self.hits.load(Ordering::Relaxed);
        (hits as f32 / lookups as f32) * 100.0
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
}

// ============================================================================
// Inode Reference
// ============================================================================

/// RAII inode reference.
pub struct InodeRef<'a> {
    /// Cache
    cache: &'a InodeCache,
    /// Handle
    handle: CacheHandle,
    /// Entry
    entry: &'a InodeCacheEntry,
}

impl<'a> InodeRef<'a> {
    /// Create new reference
    pub fn new(cache: &'a InodeCache, handle: CacheHandle) -> Option<Self> {
        if !handle.is_valid() || (handle.index as usize) >= MAX_CACHED_INODES {
            return None;
        }

        let entry = &cache.entries[handle.index as usize];

        if entry.cache_gen.load(Ordering::Relaxed) != handle.generation {
            return None;
        }

        Some(Self {
            cache,
            handle,
            entry,
        })
    }

    /// Get inode number
    #[inline]
    pub fn ino(&self) -> u64 {
        self.entry.data.ino
    }

    /// Get inode type
    #[inline]
    pub fn itype(&self) -> InodeType {
        self.entry.data.itype
    }

    /// Get size
    #[inline]
    pub fn size(&self) -> u64 {
        self.entry.data.size
    }

    /// Get data
    #[inline]
    pub fn data(&self) -> &CachedInodeData {
        &self.entry.data
    }

    /// Is directory
    #[inline]
    pub fn is_dir(&self) -> bool {
        self.entry.data.is_dir()
    }

    /// Is file
    #[inline]
    pub fn is_file(&self) -> bool {
        self.entry.data.is_file()
    }

    /// Mark dirty
    pub fn mark_dirty(&self) -> HfsResult<()> {
        self.cache.mark_dirty(self.handle)
    }
}

impl<'a> Drop for InodeRef<'a> {
    fn drop(&mut self) {
        self.cache.release(self.handle);
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inode_type() {
        assert!(InodeType::File.is_file());
        assert!(InodeType::Directory.is_dir());
        assert!(InodeType::Symlink.is_symlink());
        assert!(!InodeType::File.is_dir());
    }

    #[test]
    fn test_inode_flags() {
        let mut flags = InodeFlags::empty();
        assert!(!flags.is_dirty());

        flags.set(InodeFlags::DIRTY);
        assert!(flags.is_dirty());

        flags.clear(InodeFlags::DIRTY);
        assert!(!flags.is_dirty());
    }

    #[test]
    fn test_cached_inode_data() {
        let data = CachedInodeData::new();
        assert!(!data.is_valid());

        let mut data = CachedInodeData::new();
        data.ino = 1;
        data.itype = InodeType::File;
        assert!(data.is_valid());
        assert!(data.is_file());
    }

    #[test]
    fn test_inode_cache_entry() {
        let entry = InodeCacheEntry::new();
        assert!(entry.is_free());
        assert!(!entry.is_dirty());

        entry.set_state(CacheState::Clean);
        assert!(!entry.is_free());

        entry.acquire();
        assert_eq!(entry.get_refcount(), 1);
    }
}
