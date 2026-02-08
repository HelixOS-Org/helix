// SPDX-License-Identifier: GPL-2.0
//! Holistic ACL — POSIX ACL analysis and optimization

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// ACL entry type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AclEntryType {
    UserObj,
    User,
    GroupObj,
    Group,
    Mask,
    Other,
}

/// ACL permission
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AclPerm {
    Read,
    Write,
    Execute,
    ReadWrite,
    ReadExecute,
    All,
    None,
}

/// ACL entry
#[derive(Debug, Clone)]
pub struct HolisticAclEntry {
    pub entry_type: AclEntryType,
    pub qualifier: u32,
    pub perm: AclPerm,
}

impl HolisticAclEntry {
    pub fn new(entry_type: AclEntryType, qualifier: u32, perm: AclPerm) -> Self {
        Self {
            entry_type,
            qualifier,
            perm,
        }
    }

    pub fn is_named(&self) -> bool {
        matches!(self.entry_type, AclEntryType::User | AclEntryType::Group)
    }
}

/// Inode ACL set
#[derive(Debug, Clone)]
pub struct InodeAcl {
    pub inode: u64,
    pub access_acl: Vec<HolisticAclEntry>,
    pub default_acl: Vec<HolisticAclEntry>,
    pub check_count: u64,
}

impl InodeAcl {
    pub fn new(inode: u64) -> Self {
        Self {
            inode,
            access_acl: Vec::new(),
            default_acl: Vec::new(),
            check_count: 0,
        }
    }

    pub fn add_access(&mut self, entry: HolisticAclEntry) {
        self.access_acl.push(entry);
    }
    pub fn add_default(&mut self, entry: HolisticAclEntry) {
        self.default_acl.push(entry);
    }
    pub fn check(&mut self) {
        self.check_count += 1;
    }
    pub fn has_named_entries(&self) -> bool {
        self.access_acl.iter().any(|e| e.is_named())
    }
    pub fn complexity(&self) -> usize {
        self.access_acl.len() + self.default_acl.len()
    }
}

/// Holistic ACL stats
#[derive(Debug, Clone)]
pub struct HolisticAclStats {
    pub total_inodes_with_acl: u64,
    pub total_checks: u64,
    pub named_entry_count: u64,
    pub max_complexity: usize,
}

/// Main holistic ACL
#[derive(Debug)]
pub struct HolisticAcl {
    pub acls: BTreeMap<u64, InodeAcl>,
    pub stats: HolisticAclStats,
}

impl HolisticAcl {
    pub fn new() -> Self {
        Self {
            acls: BTreeMap::new(),
            stats: HolisticAclStats {
                total_inodes_with_acl: 0,
                total_checks: 0,
                named_entry_count: 0,
                max_complexity: 0,
            },
        }
    }

    pub fn set_acl(&mut self, inode: u64, acl: InodeAcl) {
        let c = acl.complexity();
        if c > self.stats.max_complexity {
            self.stats.max_complexity = c;
        }
        if !self.acls.contains_key(&inode) {
            self.stats.total_inodes_with_acl += 1;
        }
        self.acls.insert(inode, acl);
    }

    pub fn check(&mut self, inode: u64) {
        self.stats.total_checks += 1;
        if let Some(acl) = self.acls.get_mut(&inode) {
            acl.check();
        }
    }
}
