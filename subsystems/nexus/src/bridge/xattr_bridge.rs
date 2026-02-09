//! # Bridge Xattr Bridge
//!
//! Extended attribute syscall bridging:
//! - setxattr/getxattr/listxattr/removexattr translation
//! - Namespace handling (user, trusted, security, system)
//! - ACL attribute management
//! - Security label tracking
//! - Attribute size limits enforcement
//! - Per-inode attribute cache

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

/// Xattr namespace
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum XattrNamespace {
    User,
    Trusted,
    Security,
    System,
    Unknown,
}

impl XattrNamespace {
    #[inline]
    pub fn from_name(name: &str) -> Self {
        if name.starts_with("user.") { Self::User }
        else if name.starts_with("trusted.") { Self::Trusted }
        else if name.starts_with("security.") { Self::Security }
        else if name.starts_with("system.") { Self::System }
        else { Self::Unknown }
    }
}

/// Xattr operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XattrOp {
    Set,
    Get,
    List,
    Remove,
    FSet,
    FGet,
    LSet,
    LGet,
}

/// Set flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XattrSetFlag {
    Create,
    Replace,
    Any,
}

/// Xattr entry
#[derive(Debug, Clone)]
pub struct XattrEntry {
    pub name: String,
    pub namespace: XattrNamespace,
    pub value_size: usize,
    pub value_hash: u64,
    pub set_count: u64,
    pub get_count: u64,
    pub last_modified_ts: u64,
}

impl XattrEntry {
    pub fn new(name: String, size: usize, ts: u64) -> Self {
        let ns = XattrNamespace::from_name(&name);
        let hash = Self::hash_value(&name);
        Self { name, namespace: ns, value_size: size, value_hash: hash, set_count: 1, get_count: 0, last_modified_ts: ts }
    }

    fn hash_value(s: &str) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in s.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }
}

/// Per-inode xattr collection
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct InodeXattrs {
    pub inode: u64,
    pub attrs: BTreeMap<String, XattrEntry>,
    pub total_size: usize,
    pub max_total_size: usize,
    pub acl_access: bool,
    pub acl_default: bool,
    pub security_label: Option<String>,
}

impl InodeXattrs {
    pub fn new(inode: u64) -> Self {
        Self {
            inode, attrs: BTreeMap::new(), total_size: 0,
            max_total_size: 65536, acl_access: false,
            acl_default: false, security_label: None,
        }
    }

    pub fn set(&mut self, name: String, size: usize, ts: u64, flag: XattrSetFlag) -> bool {
        let exists = self.attrs.contains_key(&name);
        match flag {
            XattrSetFlag::Create if exists => return false,
            XattrSetFlag::Replace if !exists => return false,
            _ => {}
        }

        if name == "system.posix_acl_access" { self.acl_access = true; }
        if name == "system.posix_acl_default" { self.acl_default = true; }
        if name.starts_with("security.") { self.security_label = Some(name.clone()); }

        let old_size = self.attrs.get(&name).map(|e| e.value_size).unwrap_or(0);
        if self.total_size - old_size + size > self.max_total_size { return false; }
        self.total_size = self.total_size - old_size + size;

        if let Some(entry) = self.attrs.get_mut(&name) {
            entry.value_size = size;
            entry.set_count += 1;
            entry.last_modified_ts = ts;
        } else {
            self.attrs.insert(name.clone(), XattrEntry::new(name, size, ts));
        }
        true
    }

    #[inline]
    pub fn get(&mut self, name: &str) -> Option<&XattrEntry> {
        if let Some(e) = self.attrs.get_mut(&String::from(name)) {
            e.get_count += 1;
        }
        self.attrs.get(&String::from(name))
    }

    #[inline]
    pub fn remove(&mut self, name: &str) -> bool {
        let key = String::from(name);
        if let Some(e) = self.attrs.remove(&key) {
            self.total_size -= e.value_size;
            if key == "system.posix_acl_access" { self.acl_access = false; }
            if key == "system.posix_acl_default" { self.acl_default = false; }
            true
        } else { false }
    }

    #[inline(always)]
    pub fn list(&self) -> Vec<&str> {
        self.attrs.keys().map(|k| k.as_str()).collect()
    }

    #[inline(always)]
    pub fn count(&self) -> usize { self.attrs.len() }

    #[inline(always)]
    pub fn count_in_ns(&self, ns: XattrNamespace) -> usize {
        self.attrs.values().filter(|e| e.namespace == ns).count()
    }
}

/// Xattr operation record
#[derive(Debug, Clone)]
pub struct XattrOpRecord {
    pub op: XattrOp,
    pub inode: u64,
    pub name: String,
    pub size: usize,
    pub success: bool,
    pub timestamp: u64,
    pub pid: u64,
}

/// Xattr bridge stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct XattrBridgeStats {
    pub inodes_tracked: usize,
    pub total_attrs: usize,
    pub total_size_bytes: usize,
    pub total_sets: u64,
    pub total_gets: u64,
    pub total_removes: u64,
    pub total_lists: u64,
    pub acl_inodes: usize,
    pub security_labeled: usize,
}

/// Bridge xattr manager
#[repr(align(64))]
pub struct BridgeXattrBridge {
    inodes: BTreeMap<u64, InodeXattrs>,
    ops: VecDeque<XattrOpRecord>,
    max_ops: usize,
    stats: XattrBridgeStats,
}

impl BridgeXattrBridge {
    pub fn new() -> Self {
        Self { inodes: BTreeMap::new(), ops: VecDeque::new(), max_ops: 1024, stats: XattrBridgeStats::default() }
    }

    #[inline]
    pub fn setxattr(&mut self, inode: u64, name: String, size: usize, flag: XattrSetFlag, pid: u64, ts: u64) -> bool {
        let ix = self.inodes.entry(inode).or_insert_with(|| InodeXattrs::new(inode));
        let success = ix.set(name.clone(), size, ts, flag);
        self.ops.push_back(XattrOpRecord { op: XattrOp::Set, inode, name, size, success, timestamp: ts, pid });
        if self.ops.len() > self.max_ops { self.ops.pop_front(); }
        success
    }

    #[inline]
    pub fn getxattr(&mut self, inode: u64, name: &str, pid: u64, ts: u64) -> Option<usize> {
        let ix = self.inodes.entry(inode).or_insert_with(|| InodeXattrs::new(inode));
        let result = ix.get(name).map(|e| e.value_size);
        self.ops.push_back(XattrOpRecord { op: XattrOp::Get, inode, name: String::from(name), size: result.unwrap_or(0), success: result.is_some(), timestamp: ts, pid });
        result
    }

    #[inline]
    pub fn removexattr(&mut self, inode: u64, name: &str, pid: u64, ts: u64) -> bool {
        let ix = self.inodes.entry(inode).or_insert_with(|| InodeXattrs::new(inode));
        let success = ix.remove(name);
        self.ops.push_back(XattrOpRecord { op: XattrOp::Remove, inode, name: String::from(name), size: 0, success, timestamp: ts, pid });
        success
    }

    #[inline]
    pub fn listxattr(&mut self, inode: u64) -> Vec<String> {
        if let Some(ix) = self.inodes.get(&inode) {
            ix.list().into_iter().map(|s| String::from(s)).collect()
        } else { Vec::new() }
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.inodes_tracked = self.inodes.len();
        self.stats.total_attrs = self.inodes.values().map(|ix| ix.count()).sum();
        self.stats.total_size_bytes = self.inodes.values().map(|ix| ix.total_size).sum();
        self.stats.total_sets = self.ops.iter().filter(|o| o.op == XattrOp::Set).count() as u64;
        self.stats.total_gets = self.ops.iter().filter(|o| o.op == XattrOp::Get).count() as u64;
        self.stats.total_removes = self.ops.iter().filter(|o| o.op == XattrOp::Remove).count() as u64;
        self.stats.total_lists = self.ops.iter().filter(|o| o.op == XattrOp::List).count() as u64;
        self.stats.acl_inodes = self.inodes.values().filter(|ix| ix.acl_access || ix.acl_default).count();
        self.stats.security_labeled = self.inodes.values().filter(|ix| ix.security_label.is_some()).count();
    }

    #[inline(always)]
    pub fn inode_xattrs(&self, inode: u64) -> Option<&InodeXattrs> { self.inodes.get(&inode) }
    #[inline(always)]
    pub fn stats(&self) -> &XattrBridgeStats { &self.stats }
}

// ============================================================================
// Merged from xattr_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XattrNamespace {
    User,
    Trusted,
    System,
    Security,
    Btrfs,
}

/// Xattr flags
#[derive(Debug, Clone, Copy)]
pub struct XattrFlags {
    pub bits: u32,
}

impl XattrFlags {
    pub const CREATE: u32 = 1;
    pub const REPLACE: u32 = 2;

    pub fn new(bits: u32) -> Self { Self { bits } }
    #[inline(always)]
    pub fn create_only(&self) -> bool { self.bits & Self::CREATE != 0 }
    #[inline(always)]
    pub fn replace_only(&self) -> bool { self.bits & Self::REPLACE != 0 }
}

/// Xattr entry
#[derive(Debug, Clone)]
pub struct XattrEntry {
    pub name: String,
    pub namespace: XattrNamespace,
    pub value: Vec<u8>,
    pub set_at: u64,
    pub access_count: u64,
}

impl XattrEntry {
    pub fn new(name: String, ns: XattrNamespace, value: Vec<u8>, now: u64) -> Self {
        Self { name, namespace: ns, value, set_at: now, access_count: 0 }
    }

    #[inline(always)]
    pub fn size(&self) -> usize { self.name.len() + self.value.len() }

    #[inline(always)]
    pub fn access(&mut self) { self.access_count += 1; }

    #[inline]
    pub fn name_hash(&self) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in self.name.as_bytes() {
            hash ^= *b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }
}

/// Inode xattr store
#[derive(Debug)]
#[repr(align(64))]
pub struct InodeXattrs {
    pub inode_id: u64,
    pub attrs: BTreeMap<u64, XattrEntry>, // keyed by name_hash
    pub total_size: u64,
    pub max_size: u64,
}

impl InodeXattrs {
    pub fn new(inode_id: u64) -> Self {
        Self { inode_id, attrs: BTreeMap::new(), total_size: 0, max_size: 65536 }
    }

    pub fn set(&mut self, entry: XattrEntry, flags: XattrFlags) -> bool {
        let hash = entry.name_hash();
        let exists = self.attrs.contains_key(&hash);
        if flags.create_only() && exists { return false; }
        if flags.replace_only() && !exists { return false; }

        let new_size = entry.size() as u64;
        if exists {
            let old_size = self.attrs.get(&hash).map(|e| e.size() as u64).unwrap_or(0);
            self.total_size = self.total_size.saturating_sub(old_size) + new_size;
        } else {
            if self.total_size + new_size > self.max_size { return false; }
            self.total_size += new_size;
        }
        self.attrs.insert(hash, entry);
        true
    }

    #[inline(always)]
    pub fn get(&mut self, name: &str) -> Option<&Vec<u8>> {
        let hash = Self::hash_name(name);
        self.attrs.get_mut(&hash).map(|e| { e.access(); &e.value })
    }

    #[inline]
    pub fn remove(&mut self, name: &str) -> bool {
        let hash = Self::hash_name(name);
        if let Some(entry) = self.attrs.remove(&hash) {
            self.total_size = self.total_size.saturating_sub(entry.size() as u64);
            true
        } else { false }
    }

    #[inline(always)]
    pub fn list(&self) -> Vec<&String> {
        self.attrs.values().map(|e| &e.name).collect()
    }

    fn hash_name(name: &str) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in name.as_bytes() {
            hash ^= *b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }
}

/// Xattr operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XattrOp {
    Set,
    Get,
    Remove,
    List,
}

/// Bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct XattrV2BridgeStats {
    pub total_inodes_tracked: u32,
    pub total_attrs: u64,
    pub total_bytes: u64,
    pub total_operations: u64,
    pub attrs_by_namespace: BTreeMap<u8, u32>,
}

/// Main xattr v2 bridge
#[repr(align(64))]
pub struct BridgeXattrV2 {
    inodes: BTreeMap<u64, InodeXattrs>,
    total_ops: u64,
}

impl BridgeXattrV2 {
    pub fn new() -> Self { Self { inodes: BTreeMap::new(), total_ops: 0 } }

    #[inline]
    pub fn set_xattr(&mut self, inode: u64, name: String, ns: XattrNamespace, value: Vec<u8>, flags: XattrFlags, now: u64) -> bool {
        let store = self.inodes.entry(inode).or_insert_with(|| InodeXattrs::new(inode));
        let entry = XattrEntry::new(name, ns, value, now);
        self.total_ops += 1;
        store.set(entry, flags)
    }

    #[inline(always)]
    pub fn get_xattr(&mut self, inode: u64, name: &str) -> Option<Vec<u8>> {
        self.total_ops += 1;
        self.inodes.get_mut(&inode)?.get(name).cloned()
    }

    #[inline(always)]
    pub fn remove_xattr(&mut self, inode: u64, name: &str) -> bool {
        self.total_ops += 1;
        self.inodes.get_mut(&inode).map(|s| s.remove(name)).unwrap_or(false)
    }

    #[inline(always)]
    pub fn list_xattrs(&self, inode: u64) -> Vec<String> {
        self.inodes.get(&inode).map(|s| s.list().into_iter().cloned().collect()).unwrap_or_default()
    }

    pub fn stats(&self) -> XattrV2BridgeStats {
        let total_attrs: u64 = self.inodes.values().map(|s| s.attrs.len() as u64).sum();
        let total_bytes: u64 = self.inodes.values().map(|s| s.total_size).sum();
        let mut by_ns = BTreeMap::new();
        for store in self.inodes.values() {
            for attr in store.attrs.values() {
                *by_ns.entry(attr.namespace as u8).or_insert(0u32) += 1;
            }
        }
        XattrV2BridgeStats {
            total_inodes_tracked: self.inodes.len() as u32,
            total_attrs, total_bytes, total_operations: self.total_ops,
            attrs_by_namespace: by_ns,
        }
    }
}

// ============================================================================
// Merged from xattr_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XattrV3Namespace {
    User,
    System,
    Security,
    Trusted,
    Posix,
}

/// Extended attribute entry v3
#[derive(Debug)]
pub struct XattrV3Entry {
    pub name_hash: u64,
    pub namespace: XattrV3Namespace,
    pub value_size: u32,
    pub value_hash: u64,
}

impl XattrV3Entry {
    pub fn new(name_hash: u64, ns: XattrV3Namespace, size: u32, value_hash: u64) -> Self {
        Self { name_hash, namespace: ns, value_size: size, value_hash }
    }
}

/// Inode xattr set v3
#[derive(Debug)]
pub struct InodeXattrsV3 {
    pub inode: u64,
    pub attrs: Vec<XattrV3Entry>,
    pub total_size: u32,
    pub max_size: u32,
    pub get_count: u64,
    pub set_count: u64,
    pub remove_count: u64,
    pub list_count: u64,
}

impl InodeXattrsV3 {
    pub fn new(inode: u64, max_size: u32) -> Self {
        Self { inode, attrs: Vec::new(), total_size: 0, max_size, get_count: 0, set_count: 0, remove_count: 0, list_count: 0 }
    }

    pub fn set(&mut self, name_hash: u64, ns: XattrV3Namespace, size: u32, value_hash: u64) -> bool {
        let new_total = self.total_size + size;
        if new_total > self.max_size { return false; }
        if let Some(existing) = self.attrs.iter_mut().find(|a| a.name_hash == name_hash) {
            self.total_size -= existing.value_size;
            existing.value_size = size;
            existing.value_hash = value_hash;
        } else {
            self.attrs.push(XattrV3Entry::new(name_hash, ns, size, value_hash));
        }
        self.total_size += size;
        self.set_count += 1;
        true
    }

    #[inline(always)]
    pub fn get(&mut self, name_hash: u64) -> Option<u64> {
        self.get_count += 1;
        self.attrs.iter().find(|a| a.name_hash == name_hash).map(|a| a.value_hash)
    }

    #[inline]
    pub fn remove(&mut self, name_hash: u64) -> bool {
        if let Some(idx) = self.attrs.iter().position(|a| a.name_hash == name_hash) {
            self.total_size -= self.attrs[idx].value_size;
            self.attrs.remove(idx);
            self.remove_count += 1;
            true
        } else { false }
    }

    #[inline(always)]
    pub fn list(&mut self) -> Vec<u64> { self.list_count += 1; self.attrs.iter().map(|a| a.name_hash).collect() }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct XattrV3BridgeStats {
    pub total_inodes: u32,
    pub total_attrs: u32,
    pub total_size: u64,
    pub total_ops: u64,
}

/// Main xattr v3 bridge
#[repr(align(64))]
pub struct BridgeXattrV3 {
    inodes: BTreeMap<u64, InodeXattrsV3>,
}

impl BridgeXattrV3 {
    pub fn new() -> Self { Self { inodes: BTreeMap::new() } }

    #[inline(always)]
    pub fn ensure_inode(&mut self, inode: u64, max_size: u32) {
        self.inodes.entry(inode).or_insert_with(|| InodeXattrsV3::new(inode, max_size));
    }

    #[inline(always)]
    pub fn setxattr(&mut self, inode: u64, name_hash: u64, ns: XattrV3Namespace, size: u32, value_hash: u64) -> bool {
        self.ensure_inode(inode, 65536);
        self.inodes.get_mut(&inode).map_or(false, |i| i.set(name_hash, ns, size, value_hash))
    }

    #[inline]
    pub fn stats(&self) -> XattrV3BridgeStats {
        let attrs: u32 = self.inodes.values().map(|i| i.attrs.len() as u32).sum();
        let size: u64 = self.inodes.values().map(|i| i.total_size as u64).sum();
        let ops: u64 = self.inodes.values().map(|i| i.get_count + i.set_count + i.remove_count + i.list_count).sum();
        XattrV3BridgeStats { total_inodes: self.inodes.len() as u32, total_attrs: attrs, total_size: size, total_ops: ops }
    }
}
