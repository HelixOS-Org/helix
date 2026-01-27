//! Namespace Management
//!
//! Manages mount namespaces and filesystem visibility.

use crate::core::error::{HfsError, HfsResult};
use super::ROOT_INO;

// ============================================================================
// Namespace Constants
// ============================================================================

/// Maximum mount points per namespace
pub const MAX_MOUNTS_PER_NS: usize = 256;

/// Maximum namespaces
pub const MAX_NAMESPACES: usize = 64;

/// Maximum namespace name length
pub const MAX_NS_NAME: usize = 64;

/// Maximum path length
pub const MAX_PATH: usize = 4096;

// ============================================================================
// Mount Entry
// ============================================================================

/// Mount propagation flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum MountPropagation {
    /// Private mount
    Private = 0,
    /// Shared mount
    Shared = 1,
    /// Slave mount
    Slave = 2,
    /// Unbindable mount
    Unbindable = 3,
}

impl Default for MountPropagation {
    fn default() -> Self {
        Self::Private
    }
}

/// Mount entry flags.
#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub struct MountEntryFlags(pub u32);

impl MountEntryFlags {
    /// Read-only mount
    pub const MNT_RDONLY: u32 = 1 << 0;
    /// Nosuid
    pub const MNT_NOSUID: u32 = 1 << 1;
    /// Nodev
    pub const MNT_NODEV: u32 = 1 << 2;
    /// Noexec
    pub const MNT_NOEXEC: u32 = 1 << 3;
    /// Noatime
    pub const MNT_NOATIME: u32 = 1 << 4;
    /// Relatime
    pub const MNT_RELATIME: u32 = 1 << 5;
    /// Strictatime
    pub const MNT_STRICTATIME: u32 = 1 << 6;
    /// Internal mount
    pub const MNT_INTERNAL: u32 = 1 << 7;
    /// Locked mount (cannot be unmounted by unprivileged user)
    pub const MNT_LOCKED: u32 = 1 << 8;
    
    #[inline]
    pub fn has(&self, flag: u32) -> bool {
        self.0 & flag != 0
    }
    
    #[inline]
    pub fn set(&mut self, flag: u32) {
        self.0 |= flag;
    }
    
    #[inline]
    pub fn clear(&mut self, flag: u32) {
        self.0 &= !flag;
    }
}

/// Mount entry in namespace.
#[derive(Clone, Copy)]
pub struct MountEntry {
    /// Mount ID
    pub mount_id: u32,
    /// Parent mount ID
    pub parent_id: u32,
    /// Device ID
    pub dev: u64,
    /// Root inode
    pub root_ino: u64,
    /// Mount point path
    pub mount_path: [u8; MAX_PATH],
    /// Mount path length
    pub mount_path_len: u16,
    /// Filesystem type
    pub fs_type: [u8; 32],
    /// FS type length
    pub fs_type_len: u8,
    /// Flags
    pub flags: MountEntryFlags,
    /// Propagation type
    pub propagation: MountPropagation,
    /// Superblock index
    pub sb_idx: u8,
    /// Active flag
    pub active: bool,
}

impl MountEntry {
    /// Create new mount entry
    pub fn new(mount_id: u32, dev: u64, sb_idx: u8) -> Self {
        Self {
            mount_id,
            parent_id: 0,
            dev,
            root_ino: ROOT_INO,
            mount_path: [0; MAX_PATH],
            mount_path_len: 0,
            fs_type: [0; 32],
            fs_type_len: 0,
            flags: MountEntryFlags::default(),
            propagation: MountPropagation::default(),
            sb_idx,
            active: false,
        }
    }
    
    /// Set mount path
    pub fn set_path(&mut self, path: &[u8]) {
        let len = core::cmp::min(path.len(), MAX_PATH);
        self.mount_path[..len].copy_from_slice(&path[..len]);
        self.mount_path_len = len as u16;
    }
    
    /// Get mount path
    pub fn path(&self) -> &[u8] {
        &self.mount_path[..self.mount_path_len as usize]
    }
    
    /// Set filesystem type
    pub fn set_fs_type(&mut self, fs_type: &[u8]) {
        let len = core::cmp::min(fs_type.len(), 32);
        self.fs_type[..len].copy_from_slice(&fs_type[..len]);
        self.fs_type_len = len as u8;
    }
    
    /// Get filesystem type
    pub fn fs_type(&self) -> &[u8] {
        &self.fs_type[..self.fs_type_len as usize]
    }
    
    /// Check if path is under this mount
    pub fn contains_path(&self, path: &[u8]) -> bool {
        let mount_path = self.path();
        if path.len() < mount_path.len() {
            return false;
        }
        
        if &path[..mount_path.len()] != mount_path {
            return false;
        }
        
        // Must be exact match or followed by /
        if path.len() == mount_path.len() {
            return true;
        }
        
        path[mount_path.len()] == b'/'
    }
    
    /// Is read-only
    #[inline]
    pub fn is_readonly(&self) -> bool {
        self.flags.has(MountEntryFlags::MNT_RDONLY)
    }
}

impl Default for MountEntry {
    fn default() -> Self {
        Self {
            mount_id: 0,
            parent_id: 0,
            dev: 0,
            root_ino: ROOT_INO,
            mount_path: [0; MAX_PATH],
            mount_path_len: 0,
            fs_type: [0; 32],
            fs_type_len: 0,
            flags: MountEntryFlags::default(),
            propagation: MountPropagation::default(),
            sb_idx: 0,
            active: false,
        }
    }
}

// ============================================================================
// Mount Namespace
// ============================================================================

/// Mount namespace.
pub struct MountNamespace {
    /// Namespace ID
    pub ns_id: u32,
    /// Name
    name: [u8; MAX_NS_NAME],
    /// Name length
    name_len: u8,
    /// Mount entries
    mounts: [MountEntry; MAX_MOUNTS_PER_NS],
    /// Mount count
    mount_count: usize,
    /// Next mount ID
    next_mount_id: u32,
    /// Root mount ID
    root_mount_id: u32,
    /// Owner UID
    pub owner_uid: u32,
    /// Reference count
    pub ref_count: u32,
    /// Active flag
    pub active: bool,
}

impl MountNamespace {
    /// Create new namespace
    pub fn new(ns_id: u32, owner_uid: u32) -> Self {
        Self {
            ns_id,
            name: [0; MAX_NS_NAME],
            name_len: 0,
            mounts: [MountEntry::default(); MAX_MOUNTS_PER_NS],
            mount_count: 0,
            next_mount_id: 1,
            root_mount_id: 0,
            owner_uid,
            ref_count: 1,
            active: true,
        }
    }
    
    /// Set namespace name
    pub fn set_name(&mut self, name: &[u8]) {
        let len = core::cmp::min(name.len(), MAX_NS_NAME);
        self.name[..len].copy_from_slice(&name[..len]);
        self.name_len = len as u8;
    }
    
    /// Get namespace name
    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_len as usize]
    }
    
    /// Add mount entry
    pub fn add_mount(&mut self, mut entry: MountEntry) -> HfsResult<u32> {
        if self.mount_count >= MAX_MOUNTS_PER_NS {
            return Err(HfsError::NoSpace);
        }
        
        // Allocate mount ID
        entry.mount_id = self.next_mount_id;
        self.next_mount_id += 1;
        entry.active = true;
        
        // Find free slot
        for i in 0..MAX_MOUNTS_PER_NS {
            if !self.mounts[i].active {
                self.mounts[i] = entry;
                self.mount_count += 1;
                return Ok(entry.mount_id);
            }
        }
        
        Err(HfsError::NoSpace)
    }
    
    /// Remove mount by ID
    pub fn remove_mount(&mut self, mount_id: u32) -> HfsResult<MountEntry> {
        for i in 0..MAX_MOUNTS_PER_NS {
            if self.mounts[i].active && self.mounts[i].mount_id == mount_id {
                let entry = self.mounts[i];
                self.mounts[i].active = false;
                self.mount_count -= 1;
                return Ok(entry);
            }
        }
        
        Err(HfsError::NotFound)
    }
    
    /// Find mount by path
    pub fn find_mount_by_path(&self, path: &[u8]) -> Option<&MountEntry> {
        let mut best_match: Option<&MountEntry> = None;
        let mut best_len = 0;
        
        for mount in &self.mounts {
            if !mount.active {
                continue;
            }
            
            if mount.contains_path(path) {
                let len = mount.mount_path_len as usize;
                if len > best_len {
                    best_len = len;
                    best_match = Some(mount);
                }
            }
        }
        
        best_match
    }
    
    /// Find mount by ID
    pub fn find_mount(&self, mount_id: u32) -> Option<&MountEntry> {
        self.mounts.iter()
            .find(|m| m.active && m.mount_id == mount_id)
    }
    
    /// Find mutable mount by ID
    pub fn find_mount_mut(&mut self, mount_id: u32) -> Option<&mut MountEntry> {
        self.mounts.iter_mut()
            .find(|m| m.active && m.mount_id == mount_id)
    }
    
    /// Iterate mounts
    pub fn iter_mounts(&self) -> impl Iterator<Item = &MountEntry> {
        self.mounts.iter().filter(|m| m.active)
    }
    
    /// Set root mount
    pub fn set_root(&mut self, mount_id: u32) {
        self.root_mount_id = mount_id;
    }
    
    /// Get root mount
    pub fn root(&self) -> Option<&MountEntry> {
        self.find_mount(self.root_mount_id)
    }
    
    /// Mount count
    pub fn mount_count(&self) -> usize {
        self.mount_count
    }
    
    /// Resolve path to mount and relative path
    pub fn resolve_path<'a, 'b>(&'a self, path: &'b [u8]) -> Option<(&'a MountEntry, &'b [u8])>
    where
        'a: 'b,
    {
        let mount = self.find_mount_by_path(path)?;
        let mount_path_len = mount.mount_path_len as usize;
        
        // Get relative path (skip mount point)
        let relative = if path.len() > mount_path_len {
            &path[mount_path_len..]
        } else {
            // Path equals mount point, return root
            &path[path.len()..]  // Empty slice from path
        };
        
        Some((mount, relative))
    }
}

impl Default for MountNamespace {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

// ============================================================================
// Namespace Manager
// ============================================================================

/// Namespace manager.
pub struct NamespaceManager {
    /// Namespaces
    namespaces: [Option<MountNamespace>; MAX_NAMESPACES],
    /// Count
    count: usize,
    /// Next namespace ID
    next_ns_id: u32,
    /// Initial namespace ID
    init_ns_id: u32,
}

impl NamespaceManager {
    /// Create new namespace manager
    pub const fn new() -> Self {
        const NONE: Option<MountNamespace> = None;
        Self {
            namespaces: [NONE; MAX_NAMESPACES],
            count: 0,
            next_ns_id: 1,
            init_ns_id: 0,
        }
    }
    
    /// Create namespace
    pub fn create(&mut self, owner_uid: u32) -> HfsResult<u32> {
        for i in 0..MAX_NAMESPACES {
            if self.namespaces[i].is_none() {
                let ns_id = self.next_ns_id;
                self.next_ns_id += 1;
                
                self.namespaces[i] = Some(MountNamespace::new(ns_id, owner_uid));
                self.count += 1;
                
                // First namespace is init namespace
                if self.count == 1 {
                    self.init_ns_id = ns_id;
                }
                
                return Ok(ns_id);
            }
        }
        
        Err(HfsError::NoSpace)
    }
    
    /// Destroy namespace
    pub fn destroy(&mut self, ns_id: u32) -> HfsResult<()> {
        // Cannot destroy init namespace
        if ns_id == self.init_ns_id {
            return Err(HfsError::PermissionDenied);
        }
        
        for i in 0..MAX_NAMESPACES {
            if let Some(ns) = &self.namespaces[i] {
                if ns.ns_id == ns_id {
                    if ns.ref_count > 0 {
                        return Err(HfsError::Busy);
                    }
                    
                    self.namespaces[i] = None;
                    self.count -= 1;
                    return Ok(());
                }
            }
        }
        
        Err(HfsError::NotFound)
    }
    
    /// Get namespace
    pub fn get(&self, ns_id: u32) -> Option<&MountNamespace> {
        self.namespaces.iter()
            .flatten()
            .find(|ns| ns.ns_id == ns_id)
    }
    
    /// Get mutable namespace
    pub fn get_mut(&mut self, ns_id: u32) -> Option<&mut MountNamespace> {
        self.namespaces.iter_mut()
            .flatten()
            .find(|ns| ns.ns_id == ns_id)
    }
    
    /// Get init namespace
    pub fn init_ns(&self) -> Option<&MountNamespace> {
        self.get(self.init_ns_id)
    }
    
    /// Get mutable init namespace
    pub fn init_ns_mut(&mut self) -> Option<&mut MountNamespace> {
        self.get_mut(self.init_ns_id)
    }
    
    /// Clone namespace
    pub fn clone_ns(&mut self, src_ns_id: u32, owner_uid: u32) -> HfsResult<u32> {
        // Find source namespace
        let src_ns = self.get(src_ns_id)
            .ok_or(HfsError::NotFound)?;
        
        // Copy mounts (shallow copy, just the entries)
        let mounts = src_ns.mounts;
        let mount_count = src_ns.mount_count;
        let root_mount_id = src_ns.root_mount_id;
        
        // Create new namespace
        let new_ns_id = self.create(owner_uid)?;
        
        // Copy to new namespace
        if let Some(new_ns) = self.get_mut(new_ns_id) {
            new_ns.mounts = mounts;
            new_ns.mount_count = mount_count;
            new_ns.root_mount_id = root_mount_id;
        }
        
        Ok(new_ns_id)
    }
    
    /// Iterate namespaces
    pub fn iter(&self) -> impl Iterator<Item = &MountNamespace> {
        self.namespaces.iter().flatten()
    }
    
    /// Count
    pub fn count(&self) -> usize {
        self.count
    }
}

impl Default for NamespaceManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Path Resolution
// ============================================================================

/// Path resolution result.
#[derive(Clone, Copy)]
pub struct ResolvedPath {
    /// Mount namespace ID
    pub ns_id: u32,
    /// Mount ID
    pub mount_id: u32,
    /// Inode number
    pub ino: u64,
    /// Device ID
    pub dev: u64,
    /// Is symlink
    pub is_symlink: bool,
    /// Path remainder (for symlinks)
    pub remainder_offset: usize,
}

/// Path resolution flags.
#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub struct LookupFlags(pub u32);

impl LookupFlags {
    /// Follow symlinks
    pub const LOOKUP_FOLLOW: u32 = 1 << 0;
    /// Directory expected
    pub const LOOKUP_DIRECTORY: u32 = 1 << 1;
    /// Don't follow last symlink
    pub const LOOKUP_NOFOLLOW: u32 = 1 << 2;
    /// Creating file
    pub const LOOKUP_CREATE: u32 = 1 << 3;
    /// Exclusive create
    pub const LOOKUP_EXCL: u32 = 1 << 4;
    /// Open for read
    pub const LOOKUP_OPEN: u32 = 1 << 5;
    /// Must be empty directory
    pub const LOOKUP_EMPTY: u32 = 1 << 6;
    /// Rename target
    pub const LOOKUP_RENAME_TARGET: u32 = 1 << 7;
    
    #[inline]
    pub fn has(&self, flag: u32) -> bool {
        self.0 & flag != 0
    }
    
    /// Follow symlinks?
    #[inline]
    pub fn follow_symlinks(&self) -> bool {
        self.has(Self::LOOKUP_FOLLOW) && !self.has(Self::LOOKUP_NOFOLLOW)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mount_entry() {
        let mut entry = MountEntry::new(1, 100, 0);
        
        entry.set_path(b"/mnt/data");
        assert_eq!(entry.path(), b"/mnt/data");
        
        entry.set_fs_type(b"helixfs");
        assert_eq!(entry.fs_type(), b"helixfs");
        
        assert!(entry.contains_path(b"/mnt/data"));
        assert!(entry.contains_path(b"/mnt/data/file.txt"));
        assert!(!entry.contains_path(b"/mnt"));
        assert!(!entry.contains_path(b"/mnt/data2"));
    }
    
    #[test]
    fn test_mount_namespace() {
        let mut ns = MountNamespace::new(1, 0);
        
        let mut entry = MountEntry::new(0, 100, 0);
        entry.set_path(b"/");
        
        let mount_id = ns.add_mount(entry).unwrap();
        assert_eq!(ns.mount_count(), 1);
        
        let mount = ns.find_mount(mount_id).unwrap();
        assert_eq!(mount.dev, 100);
        
        ns.remove_mount(mount_id).unwrap();
        assert_eq!(ns.mount_count(), 0);
    }
    
    #[test]
    fn test_namespace_resolve_path() {
        let mut ns = MountNamespace::new(1, 0);
        
        // Root mount
        let mut root = MountEntry::new(0, 100, 0);
        root.set_path(b"/");
        let root_id = ns.add_mount(root).unwrap();
        ns.set_root(root_id);
        
        // Sub mount
        let mut sub = MountEntry::new(0, 200, 1);
        sub.set_path(b"/mnt");
        ns.add_mount(sub).unwrap();
        
        // Resolve /file -> root mount
        let (mount, rel) = ns.resolve_path(b"/file").unwrap();
        assert_eq!(mount.dev, 100);
        assert_eq!(rel, b"/file");
        
        // Resolve /mnt/data -> sub mount
        let (mount, rel) = ns.resolve_path(b"/mnt/data").unwrap();
        assert_eq!(mount.dev, 200);
        assert_eq!(rel, b"/data");
    }
    
    #[test]
    fn test_namespace_manager() {
        let mut mgr = NamespaceManager::new();
        
        let ns1 = mgr.create(1000).unwrap();
        assert_eq!(mgr.count(), 1);
        
        let ns2 = mgr.clone_ns(ns1, 1001).unwrap();
        assert_eq!(mgr.count(), 2);
        
        // Cannot destroy init namespace
        assert!(mgr.destroy(ns1).is_err());
        
        // Can get namespaces
        assert!(mgr.get(ns1).is_some());
        assert!(mgr.get(ns2).is_some());
    }
}
