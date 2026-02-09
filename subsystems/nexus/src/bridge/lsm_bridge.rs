// SPDX-License-Identifier: GPL-2.0
//! Bridge LSM — Linux Security Module framework bridge

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// LSM hook category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LsmHookCategory {
    File,
    Inode,
    Task,
    Socket,
    Ipc,
    Msg,
    Key,
    Bpf,
    Perf,
}

/// LSM decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LsmDecision {
    Allow,
    Deny,
    Audit,
    Default,
}

/// LSM hook record
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct LsmHookRecord {
    pub category: LsmHookCategory,
    pub hook_hash: u64,
    pub decision: LsmDecision,
    pub pid: u32,
    pub uid: u32,
    pub latency_ns: u64,
}

impl LsmHookRecord {
    pub fn new(category: LsmHookCategory, hook_name: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in hook_name {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Self {
            category,
            hook_hash: h,
            decision: LsmDecision::Allow,
            pid: 0,
            uid: 0,
            latency_ns: 0,
        }
    }
}

/// LSM bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct LsmBridgeStats {
    pub total_hooks: u64,
    pub allows: u64,
    pub denies: u64,
    pub audits: u64,
    pub total_latency_ns: u64,
}

/// Main bridge LSM
#[derive(Debug)]
pub struct BridgeLsm {
    pub stats: LsmBridgeStats,
}

impl BridgeLsm {
    pub fn new() -> Self {
        Self {
            stats: LsmBridgeStats {
                total_hooks: 0,
                allows: 0,
                denies: 0,
                audits: 0,
                total_latency_ns: 0,
            },
        }
    }

    #[inline]
    pub fn record(&mut self, rec: &LsmHookRecord) {
        self.stats.total_hooks += 1;
        self.stats.total_latency_ns += rec.latency_ns;
        match rec.decision {
            LsmDecision::Allow | LsmDecision::Default => self.stats.allows += 1,
            LsmDecision::Deny => self.stats.denies += 1,
            LsmDecision::Audit => self.stats.audits += 1,
        }
    }

    #[inline]
    pub fn deny_rate(&self) -> f64 {
        if self.stats.total_hooks == 0 {
            0.0
        } else {
            self.stats.denies as f64 / self.stats.total_hooks as f64
        }
    }
}
