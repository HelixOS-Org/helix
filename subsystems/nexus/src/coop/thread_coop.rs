// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — Thread (cooperative thread management)

extern crate alloc;
use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Thread cooperation level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopThreadLevel {
    Isolated,
    Shared,
    Cooperative,
    FullyMerged,
}

/// Thread group entry
#[derive(Debug, Clone)]
pub struct CoopThreadGroup {
    pub tgid: u64,
    pub leader: u64,
    pub members: Vec<u64>,
    pub level: CoopThreadLevel,
    pub shared_tls_pages: u32,
}

/// Thread cooperation stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CoopThreadStats {
    pub total_groups: u64,
    pub total_threads: u64,
    pub cooperative_groups: u64,
    pub merged_groups: u64,
    pub tls_sharing_events: u64,
    pub group_migrations: u64,
}

/// Manager for cooperative thread operations
pub struct CoopThreadManager {
    groups: BTreeMap<u64, CoopThreadGroup>,
    tid_to_group: LinearMap<u64, 64>,
    stats: CoopThreadStats,
}

impl CoopThreadManager {
    pub fn new() -> Self {
        Self {
            groups: BTreeMap::new(),
            tid_to_group: LinearMap::new(),
            stats: CoopThreadStats {
                total_groups: 0,
                total_threads: 0,
                cooperative_groups: 0,
                merged_groups: 0,
                tls_sharing_events: 0,
                group_migrations: 0,
            },
        }
    }

    pub fn create_group(&mut self, tgid: u64, leader: u64, level: CoopThreadLevel) {
        let group = CoopThreadGroup {
            tgid,
            leader,
            members: Vec::from([leader]),
            level,
            shared_tls_pages: 0,
        };
        self.groups.insert(tgid, group);
        self.tid_to_group.insert(leader, tgid);
        self.stats.total_groups += 1;
        self.stats.total_threads += 1;
        match level {
            CoopThreadLevel::Cooperative => self.stats.cooperative_groups += 1,
            CoopThreadLevel::FullyMerged => self.stats.merged_groups += 1,
            _ => {}
        }
    }

    #[inline]
    pub fn add_thread(&mut self, tgid: u64, tid: u64) -> bool {
        if let Some(group) = self.groups.get_mut(&tgid) {
            group.members.push(tid);
            self.tid_to_group.insert(tid, tgid);
            self.stats.total_threads += 1;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn remove_thread(&mut self, tid: u64) -> bool {
        if let Some(&tgid) = self.tid_to_group.get(tid) {
            if let Some(group) = self.groups.get_mut(&tgid) {
                group.members.retain(|&t| t != tid);
            }
            self.tid_to_group.remove(tid);
            true
        } else {
            false
        }
    }

    pub fn migrate_thread(&mut self, tid: u64, new_tgid: u64) -> bool {
        if let Some(&old_tgid) = self.tid_to_group.get(tid) {
            if let Some(old) = self.groups.get_mut(&old_tgid) {
                old.members.retain(|&t| t != tid);
            }
            if let Some(new) = self.groups.get_mut(&new_tgid) {
                new.members.push(tid);
                self.tid_to_group.insert(tid, new_tgid);
                self.stats.group_migrations += 1;
                return true;
            }
        }
        false
    }

    #[inline(always)]
    pub fn stats(&self) -> &CoopThreadStats {
        &self.stats
    }
}
