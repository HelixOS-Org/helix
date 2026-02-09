// SPDX-License-Identifier: GPL-2.0
//! NEXUS Bridge — Fork (process forking bridge)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Fork type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeForkType {
    Fork,
    Vfork,
    PosixSpawn,
    ForkExec,
}

/// Fork entry tracking
#[derive(Debug, Clone)]
pub struct BridgeForkEntry {
    pub parent_pid: u64,
    pub child_pid: u64,
    pub fork_type: BridgeForkType,
    pub cow_pages: u64,
    pub shared_pages: u64,
    pub timestamp: u64,
}

/// Stats for fork operations
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BridgeForkStats {
    pub total_forks: u64,
    pub vforks: u64,
    pub cow_faults: u64,
    pub avg_fork_us: u64,
    pub peak_children: u64,
}

/// Manager for fork bridge operations
#[repr(align(64))]
pub struct BridgeForkManager {
    children: BTreeMap<u64, BridgeForkEntry>,
    parent_map: BTreeMap<u64, Vec<u64>>,
    next_pid: u64,
    stats: BridgeForkStats,
}

impl BridgeForkManager {
    pub fn new() -> Self {
        Self {
            children: BTreeMap::new(),
            parent_map: BTreeMap::new(),
            next_pid: 2000,
            stats: BridgeForkStats {
                total_forks: 0,
                vforks: 0,
                cow_faults: 0,
                avg_fork_us: 0,
                peak_children: 0,
            },
        }
    }

    pub fn fork(&mut self, parent_pid: u64, fork_type: BridgeForkType, pages: u64) -> u64 {
        let child_pid = self.next_pid;
        self.next_pid += 1;
        let entry = BridgeForkEntry {
            parent_pid,
            child_pid,
            fork_type,
            cow_pages: pages,
            shared_pages: pages,
            timestamp: child_pid.wrapping_mul(31),
        };
        self.children.insert(child_pid, entry);
        self.parent_map.entry(parent_pid).or_insert_with(Vec::new).push(child_pid);
        self.stats.total_forks += 1;
        if matches!(fork_type, BridgeForkType::Vfork) {
            self.stats.vforks += 1;
        }
        let active = self.children.len() as u64;
        if active > self.stats.peak_children {
            self.stats.peak_children = active;
        }
        child_pid
    }

    #[inline]
    pub fn exit_child(&mut self, child_pid: u64) -> bool {
        if let Some(entry) = self.children.remove(&child_pid) {
            if let Some(list) = self.parent_map.get_mut(&entry.parent_pid) {
                list.retain(|&p| p != child_pid);
            }
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn children_of(&self, parent: u64) -> usize {
        self.parent_map.get(&parent).map(|v| v.len()).unwrap_or(0)
    }

    #[inline(always)]
    pub fn stats(&self) -> &BridgeForkStats {
        &self.stats
    }
}
