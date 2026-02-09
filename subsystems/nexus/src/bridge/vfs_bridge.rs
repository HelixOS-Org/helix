// SPDX-License-Identifier: GPL-2.0
//! Bridge VFS — virtual filesystem switch bridge with path resolution

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Bridge VFS call type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeVfsCall {
    Open,
    Close,
    Read,
    Write,
    Stat,
    Readdir,
    Mkdir,
    Rmdir,
    Unlink,
    Rename,
    Ioctl,
    Fsync,
    Mmap,
}

/// Bridge VFS result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeVfsResult {
    Success,
    NotFound,
    PermissionDenied,
    Busy,
    Io,
    NoSpace,
    CrossDevice,
    Error,
}

/// VFS bridge call record
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct VfsBridgeRecord {
    pub call_type: BridgeVfsCall,
    pub result: BridgeVfsResult,
    pub path_hash: u64,
    pub fd: i32,
    pub bytes: u64,
    pub latency_ns: u64,
    pub path_components: u32,
}

impl VfsBridgeRecord {
    pub fn new(call_type: BridgeVfsCall, path: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        let comps = path.iter().filter(|&&b| b == b'/').count() as u32;
        Self { call_type, result: BridgeVfsResult::Success, path_hash: h, fd: -1, bytes: 0, latency_ns: 0, path_components: comps }
    }
}

/// VFS bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct VfsBridgeStats {
    pub total_calls: u64,
    pub reads: u64,
    pub writes: u64,
    pub errors: u64,
    pub total_bytes: u64,
    pub total_latency_ns: u64,
}

/// Main bridge VFS
#[derive(Debug)]
pub struct BridgeVfs {
    pub stats: VfsBridgeStats,
}

impl BridgeVfs {
    pub fn new() -> Self {
        Self { stats: VfsBridgeStats { total_calls: 0, reads: 0, writes: 0, errors: 0, total_bytes: 0, total_latency_ns: 0 } }
    }

    #[inline]
    pub fn record(&mut self, rec: &VfsBridgeRecord) {
        self.stats.total_calls += 1;
        self.stats.total_bytes += rec.bytes;
        self.stats.total_latency_ns += rec.latency_ns;
        match rec.call_type {
            BridgeVfsCall::Read => self.stats.reads += 1,
            BridgeVfsCall::Write => self.stats.writes += 1,
            _ => {}
        }
        if rec.result != BridgeVfsResult::Success { self.stats.errors += 1; }
    }

    #[inline(always)]
    pub fn avg_latency_ns(&self) -> u64 {
        if self.stats.total_calls == 0 { 0 } else { self.stats.total_latency_ns / self.stats.total_calls }
    }
}

// ============================================================================
// Merged from vfs_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfsV2Op { Lookup, Create, Unlink, Rename, Mkdir, Rmdir, Symlink }

/// VFS v2 record
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct VfsV2Record {
    pub op: VfsV2Op,
    pub inode: u64,
    pub parent_inode: u64,
    pub name_hash: u64,
    pub latency_ns: u64,
}

impl VfsV2Record {
    pub fn new(op: VfsV2Op) -> Self { Self { op, inode: 0, parent_inode: 0, name_hash: 0, latency_ns: 0 } }
}

/// VFS v2 bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct VfsV2BridgeStats { pub total_ops: u64, pub lookups: u64, pub mutations: u64, pub errors: u64 }

/// Main bridge VFS v2
#[derive(Debug)]
pub struct BridgeVfsV2 { pub stats: VfsV2BridgeStats }

impl BridgeVfsV2 {
    pub fn new() -> Self { Self { stats: VfsV2BridgeStats { total_ops: 0, lookups: 0, mutations: 0, errors: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &VfsV2Record) {
        self.stats.total_ops += 1;
        match rec.op {
            VfsV2Op::Lookup => self.stats.lookups += 1,
            _ => self.stats.mutations += 1,
        }
    }
}
