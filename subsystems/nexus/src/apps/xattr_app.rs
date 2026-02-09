// SPDX-License-Identifier: GPL-2.0
//! App xattr — extended attributes application tracking

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Xattr operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XattrOp {
    Getxattr,
    Lgetxattr,
    Fgetxattr,
    Setxattr,
    Lsetxattr,
    Fsetxattr,
    Removexattr,
    Lremovexattr,
    Fremovexattr,
    Listxattr,
    Llistxattr,
    Flistxattr,
}

/// Xattr result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XattrResult {
    Success,
    NotFound,
    NoData,
    Range,
    PermissionDenied,
    NotSupported,
    Error,
}

/// Xattr namespace
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XattrNs {
    User,
    Trusted,
    Security,
    System,
    Unknown,
}

/// Xattr record
#[derive(Debug, Clone)]
pub struct XattrRecord {
    pub op: XattrOp,
    pub result: XattrResult,
    pub ns: XattrNs,
    pub name_hash: u64,
    pub path_hash: u64,
    pub value_size: u32,
}

impl XattrRecord {
    pub fn new(op: XattrOp, name: &[u8], path: &[u8]) -> Self {
        let hash = |d: &[u8]| -> u64 {
            let mut h: u64 = 0xcbf29ce484222325;
            for b in d {
                h ^= *b as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
            h
        };
        Self {
            op,
            result: XattrResult::Success,
            ns: XattrNs::User,
            name_hash: hash(name),
            path_hash: hash(path),
            value_size: 0,
        }
    }

    #[inline]
    pub fn is_write(&self) -> bool {
        matches!(
            self.op,
            XattrOp::Setxattr
                | XattrOp::Lsetxattr
                | XattrOp::Fsetxattr
                | XattrOp::Removexattr
                | XattrOp::Lremovexattr
                | XattrOp::Fremovexattr
        )
    }

    #[inline(always)]
    pub fn is_security(&self) -> bool {
        self.ns == XattrNs::Security
    }
}

/// Xattr app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct XattrAppStats {
    pub total_ops: u64,
    pub gets: u64,
    pub sets: u64,
    pub removes: u64,
    pub lists: u64,
    pub errors: u64,
}

/// Main app xattr
#[derive(Debug)]
pub struct AppXattr {
    pub stats: XattrAppStats,
}

impl AppXattr {
    pub fn new() -> Self {
        Self {
            stats: XattrAppStats {
                total_ops: 0,
                gets: 0,
                sets: 0,
                removes: 0,
                lists: 0,
                errors: 0,
            },
        }
    }

    pub fn record(&mut self, rec: &XattrRecord) {
        self.stats.total_ops += 1;
        match rec.op {
            XattrOp::Getxattr | XattrOp::Lgetxattr | XattrOp::Fgetxattr => self.stats.gets += 1,
            XattrOp::Setxattr | XattrOp::Lsetxattr | XattrOp::Fsetxattr => self.stats.sets += 1,
            XattrOp::Removexattr | XattrOp::Lremovexattr | XattrOp::Fremovexattr => {
                self.stats.removes += 1
            },
            XattrOp::Listxattr | XattrOp::Llistxattr | XattrOp::Flistxattr => self.stats.lists += 1,
        }
        if rec.result != XattrResult::Success {
            self.stats.errors += 1;
        }
    }
}
