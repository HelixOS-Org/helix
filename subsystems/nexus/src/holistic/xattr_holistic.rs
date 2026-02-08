// SPDX-License-Identifier: GPL-2.0
//! Holistic xattr — extended attribute usage analysis

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Holistic xattr namespace
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HolisticXattrNs {
    User,
    Trusted,
    Security,
    System,
    Posix,
}

/// Xattr usage pattern
#[derive(Debug, Clone)]
pub struct XattrUsagePattern {
    pub inode: u64,
    pub ns: HolisticXattrNs,
    pub name_hash: u64,
    pub get_count: u64,
    pub set_count: u64,
    pub total_value_bytes: u64,
    pub max_value_size: u32,
}

impl XattrUsagePattern {
    pub fn new(inode: u64, ns: HolisticXattrNs, name: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in name {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Self {
            inode,
            ns,
            name_hash: h,
            get_count: 0,
            set_count: 0,
            total_value_bytes: 0,
            max_value_size: 0,
        }
    }

    pub fn record_get(&mut self) {
        self.get_count += 1;
    }
    pub fn record_set(&mut self, size: u32) {
        self.set_count += 1;
        self.total_value_bytes += size as u64;
        if size > self.max_value_size {
            self.max_value_size = size;
        }
    }
}

/// Holistic xattr stats
#[derive(Debug, Clone)]
pub struct HolisticXattrStats {
    pub total_ops: u64,
    pub by_ns: BTreeMap<HolisticXattrNs, u64>,
    pub security_ops: u64,
    pub total_value_bytes: u64,
}

/// Main holistic xattr
#[derive(Debug)]
pub struct HolisticXattr {
    pub patterns: BTreeMap<u64, XattrUsagePattern>,
    pub stats: HolisticXattrStats,
}

impl HolisticXattr {
    pub fn new() -> Self {
        Self {
            patterns: BTreeMap::new(),
            stats: HolisticXattrStats {
                total_ops: 0,
                by_ns: BTreeMap::new(),
                security_ops: 0,
                total_value_bytes: 0,
            },
        }
    }

    pub fn record_get(&mut self, inode: u64, ns: HolisticXattrNs, name: &[u8]) {
        self.stats.total_ops += 1;
        *self.stats.by_ns.entry(ns).or_insert(0) += 1;
        if ns == HolisticXattrNs::Security {
            self.stats.security_ops += 1;
        }
        let mut h: u64 = 0xcbf29ce484222325;
        for b in name {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        let key = inode ^ h;
        self.patterns
            .entry(key)
            .or_insert_with(|| XattrUsagePattern::new(inode, ns, name))
            .record_get();
    }

    pub fn record_set(&mut self, inode: u64, ns: HolisticXattrNs, name: &[u8], size: u32) {
        self.stats.total_ops += 1;
        self.stats.total_value_bytes += size as u64;
        *self.stats.by_ns.entry(ns).or_insert(0) += 1;
        if ns == HolisticXattrNs::Security {
            self.stats.security_ops += 1;
        }
        let mut h: u64 = 0xcbf29ce484222325;
        for b in name {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        let key = inode ^ h;
        self.patterns
            .entry(key)
            .or_insert_with(|| XattrUsagePattern::new(inode, ns, name))
            .record_set(size);
    }
}
