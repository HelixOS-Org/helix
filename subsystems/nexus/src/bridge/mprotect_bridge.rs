// SPDX-License-Identifier: GPL-2.0
//! NEXUS Bridge — Mprotect (memory protection bridge)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Protection level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeMprotectPerm { None, ReadOnly, ReadWrite, ReadExec, ReadWriteExec }

/// Mprotect record
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BridgeMprotectRecord { pub addr: u64, pub length: u64, pub old_perm: BridgeMprotectPerm, pub new_perm: BridgeMprotectPerm }

/// Mprotect stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BridgeMprotectStats { pub total_ops: u64, pub escalations: u64, pub restrictions: u64, pub wx_attempts: u64 }

/// Manager for mprotect bridge
#[repr(align(64))]
pub struct BridgeMprotectManager {
    current_perms: BTreeMap<u64, BridgeMprotectPerm>,
    history: Vec<BridgeMprotectRecord>,
    stats: BridgeMprotectStats,
}

impl BridgeMprotectManager {
    pub fn new() -> Self {
        Self { current_perms: BTreeMap::new(), history: Vec::new(), stats: BridgeMprotectStats { total_ops: 0, escalations: 0, restrictions: 0, wx_attempts: 0 } }
    }

    #[inline]
    pub fn protect(&mut self, addr: u64, length: u64, new_perm: BridgeMprotectPerm) -> bool {
        self.stats.total_ops += 1;
        if matches!(new_perm, BridgeMprotectPerm::ReadWriteExec) { self.stats.wx_attempts += 1; }
        let old = self.current_perms.get(&addr).cloned().unwrap_or(BridgeMprotectPerm::None);
        let record = BridgeMprotectRecord { addr, length, old_perm: old, new_perm };
        self.history.push(record);
        self.current_perms.insert(addr, new_perm);
        true
    }

    #[inline(always)]
    pub fn get_perm(&self, addr: u64) -> BridgeMprotectPerm { self.current_perms.get(&addr).cloned().unwrap_or(BridgeMprotectPerm::None) }
    #[inline(always)]
    pub fn stats(&self) -> &BridgeMprotectStats { &self.stats }
}
