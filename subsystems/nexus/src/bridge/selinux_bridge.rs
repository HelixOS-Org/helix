// SPDX-License-Identifier: GPL-2.0
//! Bridge SELinux — SELinux access vector cache bridge

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// SELinux operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelinuxOp {
    AccessCheck,
    Transition,
    LoadPolicy,
    SetEnforce,
    SetBool,
    GetContext,
    SetContext,
    ComputeAv,
}

/// SELinux result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelinuxResult {
    Allowed,
    Denied,
    Audited,
    PolicyLoaded,
    Error,
}

/// SELinux record
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SelinuxRecord {
    pub op: SelinuxOp,
    pub result: SelinuxResult,
    pub source_ctx_hash: u64,
    pub target_ctx_hash: u64,
    pub tclass: u32,
    pub permission: u32,
    pub latency_ns: u64,
}

impl SelinuxRecord {
    pub fn new(op: SelinuxOp, source: &[u8], target: &[u8]) -> Self {
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
            result: SelinuxResult::Allowed,
            source_ctx_hash: hash(source),
            target_ctx_hash: hash(target),
            tclass: 0,
            permission: 0,
            latency_ns: 0,
        }
    }
}

/// AVC cache entry
#[derive(Debug, Clone)]
pub struct AvcEntry {
    pub source_hash: u64,
    pub target_hash: u64,
    pub tclass: u32,
    pub allowed: u32,
    pub denied: u32,
    pub hits: u64,
}

impl AvcEntry {
    pub fn new(src: u64, tgt: u64, tclass: u32) -> Self {
        Self {
            source_hash: src,
            target_hash: tgt,
            tclass,
            allowed: 0,
            denied: 0,
            hits: 0,
        }
    }
    #[inline(always)]
    pub fn hit(&mut self) {
        self.hits += 1;
    }
}

/// SELinux bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SelinuxBridgeStats {
    pub total_ops: u64,
    pub access_checks: u64,
    pub denials: u64,
    pub avc_hits: u64,
    pub avc_misses: u64,
}

/// Main bridge SELinux
#[derive(Debug)]
pub struct BridgeSelinux {
    pub stats: SelinuxBridgeStats,
}

impl BridgeSelinux {
    pub fn new() -> Self {
        Self {
            stats: SelinuxBridgeStats {
                total_ops: 0,
                access_checks: 0,
                denials: 0,
                avc_hits: 0,
                avc_misses: 0,
            },
        }
    }

    #[inline]
    pub fn record(&mut self, rec: &SelinuxRecord) {
        self.stats.total_ops += 1;
        if matches!(rec.op, SelinuxOp::AccessCheck | SelinuxOp::ComputeAv) {
            self.stats.access_checks += 1;
        }
        if rec.result == SelinuxResult::Denied {
            self.stats.denials += 1;
        }
    }
}
