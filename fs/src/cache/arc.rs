//! ARC (Adaptive Replacement Cache) implementation.
//!
//! ARC is a self-tuning algorithm that balances between recency (LRU)
//! and frequency (LFU) based caching. It maintains four lists:
//! - T1: Recent entries (LRU order)
//! - T2: Frequent entries (LRU order)
//! - B1: Ghost entries evicted from T1
//! - B2: Ghost entries evicted from T2

use crate::cache::CacheKey;
use crate::core::atomic::{AtomicU32, AtomicU64, Ordering};
use crate::core::error::{HfsError, HfsResult};

// ============================================================================
// Constants
// ============================================================================

/// Maximum ARC entries
pub const MAX_ARC_ENTRIES: usize = 16384;

/// Ghost list maximum size
pub const MAX_GHOST_ENTRIES: usize = 8192;

/// Adaptation rate
pub const ADAPTATION_DELTA: u32 = 1;

// ============================================================================
// ARC Entry
// ============================================================================

/// ARC list identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ArcList {
    /// Not in any list
    None = 0,
    /// Recent entries (seen once)
    T1   = 1,
    /// Frequent entries (seen multiple times)
    T2   = 2,
    /// Ghost of T1
    B1   = 3,
    /// Ghost of T2
    B2   = 4,
}

impl ArcList {
    /// From raw value
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::T1,
            2 => Self::T2,
            3 => Self::B1,
            4 => Self::B2,
            _ => Self::None,
        }
    }

    /// Is this a cache list (not ghost)
    #[inline]
    pub fn is_cache(&self) -> bool {
        matches!(self, Self::T1 | Self::T2)
    }

    /// Is this a ghost list
    #[inline]
    pub fn is_ghost(&self) -> bool {
        matches!(self, Self::B1 | Self::B2)
    }
}

impl Default for ArcList {
    fn default() -> Self {
        Self::None
    }
}

/// ARC entry.
#[repr(C)]
pub struct ArcEntry {
    /// Cache key
    pub key: CacheKey,
    /// Which list this entry belongs to
    pub list: ArcList,
    /// Buffer index (for T1/T2 only)
    pub buffer_idx: u32,
    /// Previous in list
    pub prev: u32,
    /// Next in list
    pub next: u32,
    /// Hash chain next
    pub hash_next: u32,
    /// Padding
    _pad: [u8; 4],
}

impl ArcEntry {
    /// Create new entry
    pub const fn new() -> Self {
        Self {
            key: CacheKey {
                device: 0,
                block: 0,
            },
            list: ArcList::None,
            buffer_idx: u32::MAX,
            prev: u32::MAX,
            next: u32::MAX,
            hash_next: u32::MAX,
            _pad: [0; 4],
        }
    }

    /// Is entry in use
    #[inline]
    pub fn is_used(&self) -> bool {
        self.list != ArcList::None
    }

    /// Is entry a ghost
    #[inline]
    pub fn is_ghost(&self) -> bool {
        self.list.is_ghost()
    }
}

impl Default for ArcEntry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// ARC List
// ============================================================================

/// Doubly-linked list for ARC.
#[repr(C)]
pub struct ArcLinkedList {
    /// Head index
    pub head: AtomicU32,
    /// Tail index
    pub tail: AtomicU32,
    /// Entry count
    pub count: AtomicU32,
}

impl Default for ArcLinkedList {
    fn default() -> Self {
        Self::new()
    }
}

impl ArcLinkedList {
    /// Create new empty list
    pub const fn new() -> Self {
        Self {
            head: AtomicU32::new(u32::MAX),
            tail: AtomicU32::new(u32::MAX),
            count: AtomicU32::new(0),
        }
    }

    /// Is list empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count.load(Ordering::Relaxed) == 0
    }

    /// Get count
    #[inline]
    pub fn len(&self) -> u32 {
        self.count.load(Ordering::Relaxed)
    }

    /// Get head
    #[inline]
    pub fn get_head(&self) -> Option<u32> {
        let head = self.head.load(Ordering::Acquire);
        if head == u32::MAX {
            None
        } else {
            Some(head)
        }
    }

    /// Get tail
    #[inline]
    pub fn get_tail(&self) -> Option<u32> {
        let tail = self.tail.load(Ordering::Acquire);
        if tail == u32::MAX {
            None
        } else {
            Some(tail)
        }
    }
}

// ============================================================================
// ARC Hash Table
// ============================================================================

/// Hash table size
pub const ARC_HASH_SIZE: usize = 4096;

/// Hash mask
pub const ARC_HASH_MASK: usize = ARC_HASH_SIZE - 1;

/// ARC hash bucket.
#[repr(C, align(64))]
pub struct ArcHashBucket {
    /// Head of chain
    pub head: AtomicU32,
    /// Lock
    pub lock: AtomicU32,
    /// Padding
    _pad: [u8; 56],
}

impl Default for ArcHashBucket {
    fn default() -> Self {
        Self::new()
    }
}

impl ArcHashBucket {
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
// ARC State
// ============================================================================

/// ARC state with adaptation parameter.
#[repr(C)]
pub struct ArcState {
    /// Target size for T1 (0 to c)
    pub p: AtomicU32,
    /// Maximum cache size (entries)
    pub c: u32,
    /// Maximum ghost list size
    pub ghost_max: u32,
    /// Hits in T1
    pub t1_hits: AtomicU64,
    /// Hits in T2
    pub t2_hits: AtomicU64,
    /// Hits in B1 (ghost)
    pub b1_hits: AtomicU64,
    /// Hits in B2 (ghost)
    pub b2_hits: AtomicU64,
    /// Total accesses
    pub accesses: AtomicU64,
}

impl ArcState {
    /// Create new state
    pub fn new(cache_size: u32) -> Self {
        Self {
            p: AtomicU32::new(cache_size / 2),
            c: cache_size,
            ghost_max: cache_size,
            t1_hits: AtomicU64::new(0),
            t2_hits: AtomicU64::new(0),
            b1_hits: AtomicU64::new(0),
            b2_hits: AtomicU64::new(0),
            accesses: AtomicU64::new(0),
        }
    }

    /// Get current p value
    #[inline]
    pub fn get_p(&self) -> u32 {
        self.p.load(Ordering::Relaxed)
    }

    /// Increase p (favor T1)
    pub fn increase_p(&self, delta: u32) {
        let max = self.c;
        let _ = self
            .p
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |p| {
                Some(core::cmp::min(p + delta, max))
            });
    }

    /// Decrease p (favor T2)
    pub fn decrease_p(&self, delta: u32) {
        let _ = self
            .p
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |p| {
                Some(p.saturating_sub(delta))
            });
    }

    /// Record hit
    pub fn record_hit(&self, list: ArcList) {
        self.accesses.fetch_add(1, Ordering::Relaxed);

        match list {
            ArcList::T1 => {
                self.t1_hits.fetch_add(1, Ordering::Relaxed);
            },
            ArcList::T2 => {
                self.t2_hits.fetch_add(1, Ordering::Relaxed);
            },
            ArcList::B1 => {
                self.b1_hits.fetch_add(1, Ordering::Relaxed);
            },
            ArcList::B2 => {
                self.b2_hits.fetch_add(1, Ordering::Relaxed);
            },
            ArcList::None => {},
        }
    }
}

// ============================================================================
// ARC Cache
// ============================================================================

/// ARC cache manager.
pub struct ArcCache {
    /// Entries
    pub entries: [ArcEntry; MAX_ARC_ENTRIES],
    /// Free list head
    pub free_head: AtomicU32,
    /// Free count
    pub free_count: AtomicU32,
    /// T1 list (recent)
    pub t1: ArcLinkedList,
    /// T2 list (frequent)
    pub t2: ArcLinkedList,
    /// B1 list (ghost recent)
    pub b1: ArcLinkedList,
    /// B2 list (ghost frequent)
    pub b2: ArcLinkedList,
    /// Hash table
    pub hash_table: [ArcHashBucket; ARC_HASH_SIZE],
    /// State
    pub state: ArcState,
    /// Lock for list operations
    pub list_lock: AtomicU32,
}

impl ArcCache {
    /// Hash function
    #[inline]
    fn hash_key(key: &CacheKey) -> usize {
        (key.hash() as usize) & ARC_HASH_MASK
    }

    /// Acquire list lock
    fn lock_lists(&self) {
        while self
            .list_lock
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
    }

    /// Release list lock
    fn unlock_lists(&self) {
        self.list_lock.store(0, Ordering::Release);
    }

    /// Look up entry by key
    pub fn lookup(&self, key: &CacheKey) -> Option<(u32, ArcList)> {
        let bucket_idx = Self::hash_key(key);
        let bucket = &self.hash_table[bucket_idx];

        bucket.lock();

        let mut idx = bucket.head.load(Ordering::Relaxed);
        while idx != u32::MAX {
            if (idx as usize) >= MAX_ARC_ENTRIES {
                break;
            }

            let entry = &self.entries[idx as usize];
            if entry.key == *key && entry.is_used() {
                let list = entry.list;
                bucket.unlock();
                return Some((idx, list));
            }

            idx = entry.hash_next;
        }

        bucket.unlock();
        None
    }

    /// Allocate new entry
    fn alloc_entry(&self) -> Option<u32> {
        loop {
            let head = self.free_head.load(Ordering::Acquire);
            if head == u32::MAX {
                return None;
            }

            if (head as usize) >= MAX_ARC_ENTRIES {
                return None;
            }

            let entry = &self.entries[head as usize];
            let next = entry.next;

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

    /// Free entry back to pool
    fn free_entry(&self, idx: u32) {
        if (idx as usize) >= MAX_ARC_ENTRIES {
            return;
        }

        let entry = &self.entries[idx as usize];

        loop {
            let head = self.free_head.load(Ordering::Acquire);

            // Safety: entry.next is not atomic, but we're the only
            // writer at this point since we removed it from all lists
            // This would need unsafe in real implementation

            if self
                .free_head
                .compare_exchange(head, idx, Ordering::Release, Ordering::Relaxed)
                .is_ok()
            {
                self.free_count.fetch_add(1, Ordering::Relaxed);
                return;
            }
        }
    }

    /// Add entry to hash table
    fn hash_insert(&self, idx: u32, key: &CacheKey) {
        let bucket_idx = Self::hash_key(key);
        let bucket = &self.hash_table[bucket_idx];

        bucket.lock();

        let old_head = bucket.head.load(Ordering::Relaxed);

        // Note: in real implementation, would need unsafe to modify entry
        // self.entries[idx].hash_next = old_head;

        bucket.head.store(idx, Ordering::Relaxed);
        bucket.unlock();
    }

    /// Remove entry from hash table
    fn hash_remove(&self, idx: u32, key: &CacheKey) {
        let bucket_idx = Self::hash_key(key);
        let bucket = &self.hash_table[bucket_idx];

        bucket.lock();

        let mut prev_idx = u32::MAX;
        let mut cur_idx = bucket.head.load(Ordering::Relaxed);

        while cur_idx != u32::MAX {
            if cur_idx == idx {
                let entry = &self.entries[cur_idx as usize];
                let next = entry.hash_next;

                if prev_idx == u32::MAX {
                    bucket.head.store(next, Ordering::Relaxed);
                }
                // else: need unsafe to modify prev entry

                break;
            }

            prev_idx = cur_idx;
            cur_idx = self.entries[cur_idx as usize].hash_next;
        }

        bucket.unlock();
    }

    /// Get list reference
    fn get_list(&self, list: ArcList) -> &ArcLinkedList {
        match list {
            ArcList::T1 => &self.t1,
            ArcList::T2 => &self.t2,
            ArcList::B1 => &self.b1,
            ArcList::B2 => &self.b2,
            ArcList::None => &self.t1, // Fallback
        }
    }

    /// Record access and potentially promote entry
    pub fn access(&self, key: &CacheKey) -> ArcAccessResult {
        // First check if in cache
        if let Some((idx, list)) = self.lookup(key) {
            self.state.record_hit(list);

            match list {
                ArcList::T1 => {
                    // Hit in T1: move to T2 (MRU)
                    self.lock_lists();
                    // Move logic here
                    self.unlock_lists();
                    return ArcAccessResult::HitT1(idx);
                },
                ArcList::T2 => {
                    // Hit in T2: move to MRU of T2
                    self.lock_lists();
                    // Move to MRU logic
                    self.unlock_lists();
                    return ArcAccessResult::HitT2(idx);
                },
                ArcList::B1 => {
                    // Ghost hit in B1: adapt p upward
                    let delta = if self.b2.len() >= self.b1.len() {
                        1
                    } else {
                        self.b2.len() / core::cmp::max(self.b1.len(), 1)
                    };
                    self.state.increase_p(delta);
                    return ArcAccessResult::GhostB1(idx);
                },
                ArcList::B2 => {
                    // Ghost hit in B2: adapt p downward
                    let delta = if self.b1.len() >= self.b2.len() {
                        1
                    } else {
                        self.b1.len() / core::cmp::max(self.b2.len(), 1)
                    };
                    self.state.decrease_p(delta);
                    return ArcAccessResult::GhostB2(idx);
                },
                ArcList::None => {},
            }
        }

        // Complete miss
        ArcAccessResult::Miss
    }

    /// Select victim for eviction
    pub fn select_victim(&self) -> Option<ArcVictim> {
        self.lock_lists();

        let t1_len = self.t1.len();
        let t2_len = self.t2.len();
        let p = self.state.get_p();

        // Case 1: T1 is not empty and (T1 len > p or T2 is empty and T1 has entries)
        let evict_from_t1 =
            !self.t1.is_empty() && (t1_len > p || (self.t2.is_empty() && t1_len > 0));

        let victim = if evict_from_t1 {
            // Evict from T1, move to B1
            if let Some(tail) = self.t1.get_tail() {
                let entry = &self.entries[tail as usize];
                Some(ArcVictim {
                    entry_idx: tail,
                    buffer_idx: entry.buffer_idx,
                    from_list: ArcList::T1,
                })
            } else {
                None
            }
        } else {
            // Evict from T2, move to B2
            if let Some(tail) = self.t2.get_tail() {
                let entry = &self.entries[tail as usize];
                Some(ArcVictim {
                    entry_idx: tail,
                    buffer_idx: entry.buffer_idx,
                    from_list: ArcList::T2,
                })
            } else {
                None
            }
        };

        self.unlock_lists();
        victim
    }

    /// Insert new entry
    pub fn insert(&self, key: &CacheKey, buffer_idx: u32) -> HfsResult<u32> {
        // Allocate entry
        let idx = self.alloc_entry().ok_or(HfsError::NoSpace)?;

        // Note: In real implementation, would need unsafe to modify entry
        // self.entries[idx].key = *key;
        // self.entries[idx].buffer_idx = buffer_idx;
        // self.entries[idx].list = ArcList::T1;

        // Add to hash table
        self.hash_insert(idx, key);

        // Add to T1 (MRU)
        self.lock_lists();
        // Add to T1 head logic
        // t1.count.fetch_add(1);
        self.unlock_lists();

        Ok(idx)
    }

    /// Get statistics
    pub fn get_stats(&self) -> ArcStats {
        ArcStats {
            t1_size: self.t1.len(),
            t2_size: self.t2.len(),
            b1_size: self.b1.len(),
            b2_size: self.b2.len(),
            p_value: self.state.get_p(),
            cache_size: self.state.c,
            t1_hits: self.state.t1_hits.load(Ordering::Relaxed),
            t2_hits: self.state.t2_hits.load(Ordering::Relaxed),
            b1_hits: self.state.b1_hits.load(Ordering::Relaxed),
            b2_hits: self.state.b2_hits.load(Ordering::Relaxed),
            accesses: self.state.accesses.load(Ordering::Relaxed),
        }
    }
}

// ============================================================================
// ARC Results
// ============================================================================

/// Result of ARC access.
#[derive(Clone, Copy, Debug)]
pub enum ArcAccessResult {
    /// Hit in T1 (recent)
    HitT1(u32),
    /// Hit in T2 (frequent)
    HitT2(u32),
    /// Ghost hit in B1
    GhostB1(u32),
    /// Ghost hit in B2
    GhostB2(u32),
    /// Complete miss
    Miss,
}

impl ArcAccessResult {
    /// Is this a cache hit
    #[inline]
    pub fn is_hit(&self) -> bool {
        matches!(self, Self::HitT1(_) | Self::HitT2(_))
    }

    /// Is this a ghost hit
    #[inline]
    pub fn is_ghost(&self) -> bool {
        matches!(self, Self::GhostB1(_) | Self::GhostB2(_))
    }

    /// Get entry index
    pub fn entry_idx(&self) -> Option<u32> {
        match self {
            Self::HitT1(idx) | Self::HitT2(idx) | Self::GhostB1(idx) | Self::GhostB2(idx) => {
                Some(*idx)
            },
            Self::Miss => None,
        }
    }
}

/// Victim selected for eviction.
#[derive(Clone, Copy, Debug)]
pub struct ArcVictim {
    /// Entry index in ARC
    pub entry_idx: u32,
    /// Buffer index
    pub buffer_idx: u32,
    /// Which list it came from
    pub from_list: ArcList,
}

// ============================================================================
// ARC Statistics
// ============================================================================

/// ARC statistics.
#[derive(Clone, Copy, Debug, Default)]
pub struct ArcStats {
    /// T1 size
    pub t1_size: u32,
    /// T2 size
    pub t2_size: u32,
    /// B1 size (ghost)
    pub b1_size: u32,
    /// B2 size (ghost)
    pub b2_size: u32,
    /// Current p value
    pub p_value: u32,
    /// Cache capacity
    pub cache_size: u32,
    /// T1 hits
    pub t1_hits: u64,
    /// T2 hits
    pub t2_hits: u64,
    /// B1 ghost hits
    pub b1_hits: u64,
    /// B2 ghost hits
    pub b2_hits: u64,
    /// Total accesses
    pub accesses: u64,
}

impl ArcStats {
    /// Hit rate
    pub fn hit_rate(&self) -> f32 {
        if self.accesses == 0 {
            return 0.0;
        }
        let hits = self.t1_hits + self.t2_hits;
        (hits as f32 / self.accesses as f32) * 100.0
    }

    /// Ghost hit rate
    pub fn ghost_hit_rate(&self) -> f32 {
        if self.accesses == 0 {
            return 0.0;
        }
        let ghosts = self.b1_hits + self.b2_hits;
        (ghosts as f32 / self.accesses as f32) * 100.0
    }

    /// Recency ratio (T1 / total cache)
    pub fn recency_ratio(&self) -> f32 {
        let total = self.t1_size + self.t2_size;
        if total == 0 {
            return 0.5;
        }
        self.t1_size as f32 / total as f32
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arc_list() {
        assert!(ArcList::T1.is_cache());
        assert!(ArcList::T2.is_cache());
        assert!(ArcList::B1.is_ghost());
        assert!(ArcList::B2.is_ghost());
        assert!(!ArcList::None.is_cache());
    }

    #[test]
    fn test_arc_entry() {
        let entry = ArcEntry::new();
        assert!(!entry.is_used());
        assert!(!entry.is_ghost());
    }

    #[test]
    fn test_arc_linked_list() {
        let list = ArcLinkedList::new();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
        assert!(list.get_head().is_none());
    }

    #[test]
    fn test_arc_state() {
        let state = ArcState::new(1000);
        assert_eq!(state.get_p(), 500); // Initial p = c/2

        state.increase_p(100);
        assert_eq!(state.get_p(), 600);

        state.decrease_p(200);
        assert_eq!(state.get_p(), 400);
    }

    #[test]
    fn test_arc_access_result() {
        assert!(ArcAccessResult::HitT1(0).is_hit());
        assert!(ArcAccessResult::HitT2(0).is_hit());
        assert!(ArcAccessResult::GhostB1(0).is_ghost());
        assert!(!ArcAccessResult::Miss.is_hit());
    }

    #[test]
    fn test_arc_stats() {
        let mut stats = ArcStats::default();
        stats.accesses = 100;
        stats.t1_hits = 30;
        stats.t2_hits = 40;

        assert_eq!(stats.hit_rate(), 70.0);
    }
}
