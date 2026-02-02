//! Directory Entry Cache (Dentry Cache)
//!
//! Caches directory entries for fast path lookups.

use crate::api::FileType;
use crate::core::error::{HfsError, HfsResult};

/// Maximum name length for dentry
const MAX_NAME_LEN: usize = 255;

// ============================================================================
// Constants
// ============================================================================

/// Dentry hash table size
const HASH_SIZE: usize = 8192;

/// Maximum cached dentries
const CACHE_SIZE: usize = 32768;

/// Negative dentry timeout (seconds)
const NEGATIVE_TIMEOUT: u64 = 30;

// ============================================================================
// Dentry State
// ============================================================================

/// Dentry state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DentryState {
    /// Free slot
    Free      = 0,
    /// Valid positive dentry
    Valid     = 1,
    /// Negative dentry (name doesn't exist)
    Negative  = 2,
    /// Being resolved
    Resolving = 3,
}

impl Default for DentryState {
    fn default() -> Self {
        Self::Free
    }
}

// ============================================================================
// Dentry Flags
// ============================================================================

/// Dentry flags.
#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub struct DentryFlags(pub u16);

impl DentryFlags {
    /// Dentry is a mount point
    pub const D_MOUNTED: u16 = 1 << 0;
    /// Dentry is referenced
    pub const D_REFERENCED: u16 = 1 << 1;
    /// Dentry is a directory
    pub const D_DIRECTORY: u16 = 1 << 2;
    /// Dentry is a symlink
    pub const D_SYMLINK: u16 = 1 << 3;
    /// Dentry is automount trigger
    pub const D_AUTOMOUNT: u16 = 1 << 4;

    /// Check flag
    #[inline]
    pub fn has(&self, flag: u16) -> bool {
        self.0 & flag != 0
    }

    /// Set flag
    #[inline]
    pub fn set(&mut self, flag: u16) {
        self.0 |= flag;
    }

    /// Clear flag
    #[inline]
    pub fn clear(&mut self, flag: u16) {
        self.0 &= !flag;
    }
}

// ============================================================================
// Dentry
// ============================================================================

/// Directory entry in cache.
#[derive(Clone, Copy)]
pub struct Dentry {
    /// Parent inode number
    pub parent_ino: u64,
    /// This entry's inode number (0 for negative)
    pub ino: u64,
    /// File type
    pub ftype: FileType,
    /// Name length
    pub name_len: u8,
    /// Name data
    pub name: [u8; MAX_NAME_LEN],
    /// State
    pub state: DentryState,
    /// Flags
    pub flags: DentryFlags,
    /// Reference count
    pub refcount: u16,
    /// Hash chain next
    pub hash_next: u16,
    /// LRU next
    pub lru_next: u16,
    /// LRU prev
    pub lru_prev: u16,
    /// Creation timestamp
    pub timestamp: u64,
}

impl Dentry {
    /// Create empty dentry
    pub const fn empty() -> Self {
        Self {
            parent_ino: 0,
            ino: 0,
            ftype: FileType::Unknown,
            name_len: 0,
            name: [0; MAX_NAME_LEN],
            state: DentryState::Free,
            flags: DentryFlags(0),
            refcount: 0,
            hash_next: u16::MAX,
            lru_next: u16::MAX,
            lru_prev: u16::MAX,
            timestamp: 0,
        }
    }

    /// Create positive dentry
    pub fn new(parent_ino: u64, name: &[u8], ino: u64, ftype: FileType) -> Self {
        let mut dentry = Self::empty();
        dentry.parent_ino = parent_ino;
        dentry.ino = ino;
        dentry.ftype = ftype;
        dentry.state = DentryState::Valid;

        let len = core::cmp::min(name.len(), MAX_NAME_LEN);
        dentry.name[..len].copy_from_slice(&name[..len]);
        dentry.name_len = len as u8;

        if ftype == FileType::Directory {
            dentry.flags.set(DentryFlags::D_DIRECTORY);
        } else if ftype == FileType::Symlink {
            dentry.flags.set(DentryFlags::D_SYMLINK);
        }

        dentry
    }

    /// Create negative dentry
    pub fn new_negative(parent_ino: u64, name: &[u8]) -> Self {
        let mut dentry = Self::empty();
        dentry.parent_ino = parent_ino;
        dentry.ino = 0;
        dentry.state = DentryState::Negative;

        let len = core::cmp::min(name.len(), MAX_NAME_LEN);
        dentry.name[..len].copy_from_slice(&name[..len]);
        dentry.name_len = len as u8;

        dentry
    }

    /// Get name as slice
    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_len as usize]
    }

    /// Is free
    #[inline]
    pub fn is_free(&self) -> bool {
        self.state == DentryState::Free
    }

    /// Is valid positive dentry
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.state == DentryState::Valid
    }

    /// Is negative dentry
    #[inline]
    pub fn is_negative(&self) -> bool {
        self.state == DentryState::Negative
    }

    /// Is directory
    #[inline]
    pub fn is_dir(&self) -> bool {
        self.ftype == FileType::Directory
    }

    /// Is mount point
    #[inline]
    pub fn is_mounted(&self) -> bool {
        self.flags.has(DentryFlags::D_MOUNTED)
    }

    /// Match name
    pub fn matches(&self, parent: u64, name: &[u8]) -> bool {
        self.parent_ino == parent && self.name() == name
    }
}

impl Default for Dentry {
    fn default() -> Self {
        Self::empty()
    }
}

// ============================================================================
// Dentry Hash
// ============================================================================

/// Hash function for dentries.
fn dentry_hash(parent: u64, name: &[u8]) -> usize {
    let mut hash = parent.wrapping_mul(0x9e3779b97f4a7c15);

    for &b in name {
        hash = hash.wrapping_mul(31).wrapping_add(b as u64);
    }

    (hash as usize) & (HASH_SIZE - 1)
}

// ============================================================================
// Dentry Cache
// ============================================================================

/// Dentry cache.
pub struct DentryCache {
    /// Cached dentries
    dentries: [Dentry; CACHE_SIZE],
    /// Hash table
    hash_table: [u16; HASH_SIZE],
    /// Free list head
    free_head: u16,
    /// LRU head
    lru_head: u16,
    /// LRU tail
    lru_tail: u16,
    /// Number of cached dentries
    count: usize,
    /// Statistics
    stats: DentryCacheStats,
}

/// Dentry cache statistics.
#[derive(Clone, Copy, Debug, Default)]
pub struct DentryCacheStats {
    /// Cache hits
    pub hits: u64,
    /// Cache misses
    pub misses: u64,
    /// Negative hits
    pub neg_hits: u64,
    /// Evictions
    pub evictions: u64,
}

impl DentryCache {
    /// Create new dentry cache
    pub fn new() -> Self {
        let mut cache = Self {
            dentries: [Dentry::empty(); CACHE_SIZE],
            hash_table: [u16::MAX; HASH_SIZE],
            free_head: 0,
            lru_head: u16::MAX,
            lru_tail: u16::MAX,
            count: 0,
            stats: DentryCacheStats::default(),
        };

        // Initialize free list
        for i in 0..CACHE_SIZE - 1 {
            cache.dentries[i].lru_next = (i + 1) as u16;
        }
        cache.dentries[CACHE_SIZE - 1].lru_next = u16::MAX;

        cache
    }

    /// Lookup dentry
    pub fn lookup(&mut self, parent: u64, name: &[u8]) -> Option<&Dentry> {
        let hash = dentry_hash(parent, name);
        let mut idx = self.hash_table[hash];

        while idx != u16::MAX {
            let dentry = &self.dentries[idx as usize];

            if dentry.matches(parent, name) {
                if dentry.is_valid() {
                    self.stats.hits += 1;
                    self.lru_touch(idx);
                    return Some(&self.dentries[idx as usize]);
                } else if dentry.is_negative() {
                    self.stats.neg_hits += 1;
                    return Some(&self.dentries[idx as usize]);
                }
            }

            idx = dentry.hash_next;
        }

        self.stats.misses += 1;
        None
    }

    /// Insert positive dentry
    pub fn insert(&mut self, parent: u64, name: &[u8], ino: u64, ftype: FileType) -> HfsResult<()> {
        let dentry = Dentry::new(parent, name, ino, ftype);
        self.insert_dentry(dentry)
    }

    /// Insert negative dentry
    pub fn insert_negative(&mut self, parent: u64, name: &[u8]) -> HfsResult<()> {
        let dentry = Dentry::new_negative(parent, name);
        self.insert_dentry(dentry)
    }

    /// Insert dentry
    fn insert_dentry(&mut self, dentry: Dentry) -> HfsResult<()> {
        // Remove existing if any
        self.remove(dentry.parent_ino, dentry.name());

        // Allocate slot
        let idx = self.alloc_slot()?;

        // Insert into hash table
        let hash = dentry_hash(dentry.parent_ino, dentry.name());
        self.dentries[idx].hash_next = self.hash_table[hash];
        self.hash_table[hash] = idx as u16;

        // Set dentry data
        self.dentries[idx] = dentry;
        self.dentries[idx].hash_next = self.hash_table[hash];

        // Add to LRU
        self.lru_add_front(idx as u16);

        self.count += 1;

        Ok(())
    }

    /// Remove dentry
    pub fn remove(&mut self, parent: u64, name: &[u8]) -> bool {
        let hash = dentry_hash(parent, name);
        let mut prev: u16 = u16::MAX;
        let mut idx = self.hash_table[hash];

        while idx != u16::MAX {
            let dentry = &self.dentries[idx as usize];

            if dentry.matches(parent, name) {
                // Remove from hash chain
                if prev == u16::MAX {
                    self.hash_table[hash] = dentry.hash_next;
                } else {
                    self.dentries[prev as usize].hash_next = dentry.hash_next;
                }

                // Remove from LRU
                self.lru_remove(idx);

                // Add to free list
                self.dentries[idx as usize] = Dentry::empty();
                self.dentries[idx as usize].lru_next = self.free_head;
                self.free_head = idx;

                self.count -= 1;

                return true;
            }

            prev = idx;
            idx = dentry.hash_next;
        }

        false
    }

    /// Invalidate all dentries for a directory
    pub fn invalidate_dir(&mut self, dir_ino: u64) {
        // Collect names first to avoid borrow issues
        let mut to_remove = [(0u64, [0u8; MAX_NAME_LEN], 0usize); 64]; // Batch size
        let mut count = 0;

        for i in 0..CACHE_SIZE {
            if !self.dentries[i].is_free() && self.dentries[i].parent_ino == dir_ino {
                if count < to_remove.len() {
                    let name_len = self.dentries[i].name_len as usize;
                    to_remove[count].0 = dir_ino;
                    to_remove[count].2 = name_len;
                    to_remove[count].1[..name_len]
                        .copy_from_slice(&self.dentries[i].name[..name_len]);
                    count += 1;
                }
            }
        }

        // Remove collected entries
        for j in 0..count {
            let (parent, name, len) = &to_remove[j];
            self.remove(*parent, &name[..*len]);
        }
    }

    /// Invalidate dentry by inode
    pub fn invalidate_ino(&mut self, ino: u64) {
        for i in 0..CACHE_SIZE {
            if !self.dentries[i].is_free() && self.dentries[i].ino == ino {
                let parent = self.dentries[i].parent_ino;
                let mut name = [0u8; MAX_NAME_LEN];
                let name_len = self.dentries[i].name_len as usize;
                name[..name_len].copy_from_slice(&self.dentries[i].name[..name_len]);
                self.remove(parent, &name[..name_len]);
            }
        }
    }

    /// Allocate cache slot
    fn alloc_slot(&mut self) -> HfsResult<usize> {
        // Try free list
        if self.free_head != u16::MAX {
            let idx = self.free_head as usize;
            self.free_head = self.dentries[idx].lru_next;
            return Ok(idx);
        }

        // Evict from LRU tail
        if self.lru_tail != u16::MAX {
            let idx = self.lru_tail as usize;
            let dentry = &self.dentries[idx];

            if dentry.refcount == 0 {
                let parent = dentry.parent_ino;
                let mut name = [0u8; MAX_NAME_LEN];
                let name_len = dentry.name_len as usize;
                name[..name_len].copy_from_slice(&dentry.name[..name_len]);

                self.remove(parent, &name[..name_len]);
                self.stats.evictions += 1;
                return Ok(idx);
            }

            // Find unreferenced entry
            let mut scan = self.lru_tail;
            while scan != u16::MAX {
                if self.dentries[scan as usize].refcount == 0 {
                    let idx = scan as usize;
                    let dentry = &self.dentries[idx];
                    let parent = dentry.parent_ino;
                    let mut name = [0u8; MAX_NAME_LEN];
                    let name_len = dentry.name_len as usize;
                    name[..name_len].copy_from_slice(&dentry.name[..name_len]);

                    self.remove(parent, &name[..name_len]);
                    self.stats.evictions += 1;
                    return Ok(idx);
                }
                scan = self.dentries[scan as usize].lru_prev;
            }
        }

        Err(HfsError::NoSpace)
    }

    /// Add to LRU front
    fn lru_add_front(&mut self, idx: u16) {
        self.dentries[idx as usize].lru_next = self.lru_head;
        self.dentries[idx as usize].lru_prev = u16::MAX;

        if self.lru_head != u16::MAX {
            self.dentries[self.lru_head as usize].lru_prev = idx;
        }

        self.lru_head = idx;

        if self.lru_tail == u16::MAX {
            self.lru_tail = idx;
        }
    }

    /// Remove from LRU
    fn lru_remove(&mut self, idx: u16) {
        let prev = self.dentries[idx as usize].lru_prev;
        let next = self.dentries[idx as usize].lru_next;

        if prev != u16::MAX {
            self.dentries[prev as usize].lru_next = next;
        } else {
            self.lru_head = next;
        }

        if next != u16::MAX {
            self.dentries[next as usize].lru_prev = prev;
        } else {
            self.lru_tail = prev;
        }
    }

    /// Touch LRU entry
    fn lru_touch(&mut self, idx: u16) {
        if idx == self.lru_head {
            return;
        }

        self.lru_remove(idx);
        self.lru_add_front(idx);
    }

    /// Get statistics
    pub fn stats(&self) -> &DentryCacheStats {
        &self.stats
    }

    /// Get count
    pub fn count(&self) -> usize {
        self.count
    }
}

impl Default for DentryCache {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Path Resolution
// ============================================================================

/// Path component iterator.
pub struct PathComponents<'a> {
    /// Remaining path
    path: &'a [u8],
}

impl<'a> PathComponents<'a> {
    /// Create new path components iterator
    pub fn new(path: &'a [u8]) -> Self {
        // Skip leading slashes
        let path = path
            .iter()
            .position(|&b| b != b'/')
            .map(|i| &path[i..])
            .unwrap_or(&[]);

        Self { path }
    }
}

impl<'a> Iterator for PathComponents<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.path.is_empty() {
            return None;
        }

        // Find next slash
        let end = self
            .path
            .iter()
            .position(|&b| b == b'/')
            .unwrap_or(self.path.len());

        let component = &self.path[..end];

        // Skip component and any following slashes
        self.path = &self.path[end..];
        while !self.path.is_empty() && self.path[0] == b'/' {
            self.path = &self.path[1..];
        }

        if component.is_empty() {
            self.next()
        } else {
            Some(component)
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
    fn test_dentry() {
        let dentry = Dentry::new(1, b"test.txt", 100, FileType::Regular);

        assert_eq!(dentry.parent_ino, 1);
        assert_eq!(dentry.ino, 100);
        assert_eq!(dentry.name(), b"test.txt");
        assert!(dentry.is_valid());
    }

    #[test]
    fn test_negative_dentry() {
        let dentry = Dentry::new_negative(1, b"noexist");

        assert!(dentry.is_negative());
        assert_eq!(dentry.ino, 0);
    }

    #[test]
    fn test_dentry_cache() {
        let mut cache = DentryCache::new();

        cache
            .insert(1, b"file1.txt", 100, FileType::Regular)
            .unwrap();
        cache
            .insert(1, b"file2.txt", 101, FileType::Regular)
            .unwrap();

        assert_eq!(cache.count(), 2);

        let dentry = cache.lookup(1, b"file1.txt").unwrap();
        assert_eq!(dentry.ino, 100);

        assert!(cache.lookup(1, b"noexist").is_none());
    }

    #[test]
    fn test_dentry_remove() {
        let mut cache = DentryCache::new();

        cache
            .insert(1, b"file.txt", 100, FileType::Regular)
            .unwrap();
        assert!(cache.remove(1, b"file.txt"));
        assert!(cache.lookup(1, b"file.txt").is_none());
    }

    #[test]
    fn test_path_components() {
        let components: Vec<_> = PathComponents::new(b"/home/user/file.txt").collect();
        assert_eq!(components, vec![
            b"home".as_slice(),
            b"user".as_slice(),
            b"file.txt".as_slice()
        ]);

        let components: Vec<_> = PathComponents::new(b"relative/path").collect();
        assert_eq!(components, vec![b"relative".as_slice(), b"path".as_slice()]);

        let components: Vec<_> = PathComponents::new(b"///multiple///slashes///").collect();
        assert_eq!(components, vec![
            b"multiple".as_slice(),
            b"slashes".as_slice()
        ]);
    }
}
