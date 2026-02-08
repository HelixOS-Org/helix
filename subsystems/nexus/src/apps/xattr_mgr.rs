// SPDX-License-Identifier: GPL-2.0
//! Apps xattr_mgr — extended attribute management and security label tracking per application.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Extended attribute namespace
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XattrNamespace {
    /// user.*
    User,
    /// trusted.*
    Trusted,
    /// security.*
    Security,
    /// system.*
    System,
    /// selinux label
    Selinux,
    /// smack label
    Smack,
    /// capability bits
    Capability,
    /// posix ACL
    PosixAcl,
}

impl XattrNamespace {
    pub fn requires_privilege(&self) -> bool {
        matches!(self, Self::Trusted | Self::Security | Self::Selinux | Self::Smack | Self::Capability)
    }
}

/// Xattr operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XattrOp {
    Get,
    Set,
    List,
    Remove,
}

/// A stored extended attribute
#[derive(Debug, Clone)]
pub struct XattrEntry {
    pub name: String,
    pub namespace: XattrNamespace,
    pub value_size: usize,
    pub value_hash: u64,
    pub set_timestamp_ns: u64,
    pub access_count: u64,
}

impl XattrEntry {
    pub fn new(name: String, namespace: XattrNamespace, value_size: usize, value_hash: u64) -> Self {
        Self {
            name,
            namespace,
            value_size,
            value_hash,
            set_timestamp_ns: 0,
            access_count: 0,
        }
    }

    pub fn is_security_label(&self) -> bool {
        matches!(self.namespace, XattrNamespace::Security | XattrNamespace::Selinux | XattrNamespace::Smack)
    }
}

/// Inode xattr storage
#[derive(Debug)]
pub struct InodeXattrs {
    pub inode: u64,
    attrs: BTreeMap<String, XattrEntry>,
    pub total_value_bytes: usize,
}

impl InodeXattrs {
    pub fn new(inode: u64) -> Self {
        Self {
            inode,
            attrs: BTreeMap::new(),
            total_value_bytes: 0,
        }
    }

    pub fn set(&mut self, entry: XattrEntry) {
        let old_size = self.attrs.get(&entry.name).map(|e| e.value_size).unwrap_or(0);
        self.total_value_bytes = self.total_value_bytes.saturating_sub(old_size) + entry.value_size;
        self.attrs.insert(entry.name.clone(), entry);
    }

    pub fn get(&mut self, name: &str) -> Option<&XattrEntry> {
        if let Some(entry) = self.attrs.get_mut(name) {
            entry.access_count += 1;
        }
        self.attrs.get(name)
    }

    pub fn remove(&mut self, name: &str) -> bool {
        if let Some(entry) = self.attrs.remove(name) {
            self.total_value_bytes = self.total_value_bytes.saturating_sub(entry.value_size);
            true
        } else {
            false
        }
    }

    pub fn list(&self) -> Vec<&str> {
        self.attrs.keys().map(|s| s.as_str()).collect()
    }

    pub fn count(&self) -> usize {
        self.attrs.len()
    }

    pub fn has_security_labels(&self) -> bool {
        self.attrs.values().any(|e| e.is_security_label())
    }

    pub fn security_labels(&self) -> Vec<&XattrEntry> {
        self.attrs.values().filter(|e| e.is_security_label()).collect()
    }

    pub fn by_namespace(&self, ns: XattrNamespace) -> Vec<&XattrEntry> {
        self.attrs.values().filter(|e| e.namespace == ns).collect()
    }
}

/// Per-app xattr access tracking
#[derive(Debug)]
pub struct AppXattrProfile {
    pub pid: u64,
    pub get_count: u64,
    pub set_count: u64,
    pub list_count: u64,
    pub remove_count: u64,
    pub denied_count: u64,
    pub namespaces_accessed: Vec<XattrNamespace>,
    pub security_label_changes: u64,
}

impl AppXattrProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            get_count: 0,
            set_count: 0,
            list_count: 0,
            remove_count: 0,
            denied_count: 0,
            namespaces_accessed: Vec::new(),
            security_label_changes: 0,
        }
    }

    pub fn record_op(&mut self, op: XattrOp, ns: XattrNamespace) {
        match op {
            XattrOp::Get => self.get_count += 1,
            XattrOp::Set => {
                self.set_count += 1;
                if matches!(ns, XattrNamespace::Security | XattrNamespace::Selinux | XattrNamespace::Smack) {
                    self.security_label_changes += 1;
                }
            }
            XattrOp::List => self.list_count += 1,
            XattrOp::Remove => self.remove_count += 1,
        }
        if !self.namespaces_accessed.contains(&ns) {
            self.namespaces_accessed.push(ns);
        }
    }

    pub fn record_denied(&mut self) {
        self.denied_count += 1;
    }

    pub fn total_ops(&self) -> u64 {
        self.get_count + self.set_count + self.list_count + self.remove_count
    }

    pub fn write_ratio(&self) -> f64 {
        let total = self.total_ops();
        if total == 0 { return 0.0; }
        (self.set_count + self.remove_count) as f64 / total as f64
    }

    pub fn denial_rate(&self) -> f64 {
        let total = self.total_ops() + self.denied_count;
        if total == 0 { return 0.0; }
        self.denied_count as f64 / total as f64
    }

    pub fn accesses_privileged(&self) -> bool {
        self.namespaces_accessed.iter().any(|ns| ns.requires_privilege())
    }
}

/// Xattr manager stats
#[derive(Debug, Clone)]
pub struct XattrMgrStats {
    pub total_inodes_tracked: u64,
    pub total_xattrs: u64,
    pub total_value_bytes: u64,
    pub total_gets: u64,
    pub total_sets: u64,
    pub total_removes: u64,
    pub total_denials: u64,
    pub security_label_count: u64,
}

/// Main xattr manager
pub struct AppXattrMgr {
    inodes: BTreeMap<u64, InodeXattrs>,
    profiles: BTreeMap<u64, AppXattrProfile>,
    stats: XattrMgrStats,
}

impl AppXattrMgr {
    pub fn new() -> Self {
        Self {
            inodes: BTreeMap::new(),
            profiles: BTreeMap::new(),
            stats: XattrMgrStats {
                total_inodes_tracked: 0,
                total_xattrs: 0,
                total_value_bytes: 0,
                total_gets: 0,
                total_sets: 0,
                total_removes: 0,
                total_denials: 0,
                security_label_count: 0,
            },
        }
    }

    pub fn register_app(&mut self, pid: u64) {
        self.profiles.insert(pid, AppXattrProfile::new(pid));
    }

    fn ensure_inode(&mut self, inode: u64) {
        if !self.inodes.contains_key(&inode) {
            self.inodes.insert(inode, InodeXattrs::new(inode));
            self.stats.total_inodes_tracked += 1;
        }
    }

    pub fn setxattr(&mut self, pid: u64, inode: u64, name: String, ns: XattrNamespace, value_size: usize, value_hash: u64, timestamp_ns: u64) -> bool {
        // Permission check
        if ns.requires_privilege() {
            // For now: simplified — track denial
            // In real kernel: check capabilities
        }

        if let Some(prof) = self.profiles.get_mut(&pid) {
            prof.record_op(XattrOp::Set, ns);
        }

        self.ensure_inode(inode);
        let inode_xattrs = self.inodes.get_mut(&inode).unwrap();
        let is_new = !inode_xattrs.attrs.contains_key(&name);
        let mut entry = XattrEntry::new(name, ns, value_size, value_hash);
        entry.set_timestamp_ns = timestamp_ns;
        let is_sec = entry.is_security_label();
        inode_xattrs.set(entry);

        self.stats.total_sets += 1;
        if is_new {
            self.stats.total_xattrs += 1;
            if is_sec {
                self.stats.security_label_count += 1;
            }
        }
        self.stats.total_value_bytes = self.inodes.values().map(|i| i.total_value_bytes as u64).sum();
        true
    }

    pub fn getxattr(&mut self, pid: u64, inode: u64, name: &str) -> Option<u64> {
        if let Some(prof) = self.profiles.get_mut(&pid) {
            let ns = self.inodes.get(&inode)
                .and_then(|i| i.attrs.get(name))
                .map(|e| e.namespace)
                .unwrap_or(XattrNamespace::User);
            prof.record_op(XattrOp::Get, ns);
        }
        self.stats.total_gets += 1;
        if let Some(inode_xattrs) = self.inodes.get_mut(&inode) {
            inode_xattrs.get(name).map(|e| e.value_hash)
        } else {
            None
        }
    }

    pub fn removexattr(&mut self, pid: u64, inode: u64, name: &str) -> bool {
        if let Some(prof) = self.profiles.get_mut(&pid) {
            prof.record_op(XattrOp::Remove, XattrNamespace::User);
        }
        self.stats.total_removes += 1;
        if let Some(inode_xattrs) = self.inodes.get_mut(&inode) {
            if inode_xattrs.remove(name) {
                self.stats.total_xattrs = self.stats.total_xattrs.saturating_sub(1);
                self.stats.total_value_bytes = self.inodes.values().map(|i| i.total_value_bytes as u64).sum();
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn listxattr(&mut self, pid: u64, inode: u64) -> Vec<String> {
        if let Some(prof) = self.profiles.get_mut(&pid) {
            prof.record_op(XattrOp::List, XattrNamespace::User);
        }
        if let Some(inode_xattrs) = self.inodes.get(&inode) {
            inode_xattrs.list().iter().map(|s| String::from(*s)).collect()
        } else {
            Vec::new()
        }
    }

    pub fn deny_access(&mut self, pid: u64) {
        if let Some(prof) = self.profiles.get_mut(&pid) {
            prof.record_denied();
        }
        self.stats.total_denials += 1;
    }

    pub fn inodes_with_security_labels(&self) -> Vec<u64> {
        self.inodes.iter()
            .filter(|(_, ix)| ix.has_security_labels())
            .map(|(&ino, _)| ino)
            .collect()
    }

    pub fn heaviest_inodes(&self, top: usize) -> Vec<(u64, usize)> {
        let mut v: Vec<(u64, usize)> = self.inodes.iter()
            .map(|(&ino, ix)| (ino, ix.total_value_bytes))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(top);
        v
    }

    pub fn most_active_apps(&self, top: usize) -> Vec<(u64, u64)> {
        let mut v: Vec<(u64, u64)> = self.profiles.iter()
            .map(|(&pid, p)| (pid, p.total_ops()))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(top);
        v
    }

    pub fn get_inode_xattrs(&self, inode: u64) -> Option<&InodeXattrs> {
        self.inodes.get(&inode)
    }

    pub fn get_profile(&self, pid: u64) -> Option<&AppXattrProfile> {
        self.profiles.get(&pid)
    }

    pub fn stats(&self) -> &XattrMgrStats {
        &self.stats
    }
}
