// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — PGID (cooperative process group management)

extern crate alloc;
use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Process group cooperation state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopPgidState {
    Active,
    Orphaned,
    Stopped,
    Foreground,
    Background,
}

/// Process group entry
#[derive(Debug, Clone)]
pub struct CoopPgidEntry {
    pub pgid: u64,
    pub leader: u64,
    pub state: CoopPgidState,
    pub members: Vec<u64>,
    pub session_id: u64,
}

/// PGID cooperation stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CoopPgidStats {
    pub total_groups: u64,
    pub orphaned_groups: u64,
    pub fg_switches: u64,
    pub group_signals: u64,
    pub member_migrations: u64,
}

/// Manager for cooperative PGID operations
pub struct CoopPgidManager {
    groups: BTreeMap<u64, CoopPgidEntry>,
    pid_to_pgid: LinearMap<u64, 64>,
    stats: CoopPgidStats,
}

impl CoopPgidManager {
    pub fn new() -> Self {
        Self {
            groups: BTreeMap::new(),
            pid_to_pgid: LinearMap::new(),
            stats: CoopPgidStats {
                total_groups: 0,
                orphaned_groups: 0,
                fg_switches: 0,
                group_signals: 0,
                member_migrations: 0,
            },
        }
    }

    pub fn create_group(&mut self, pgid: u64, leader: u64, session: u64) {
        let entry = CoopPgidEntry {
            pgid,
            leader,
            state: CoopPgidState::Active,
            members: Vec::from([leader]),
            session_id: session,
        };
        self.groups.insert(pgid, entry);
        self.pid_to_pgid.insert(leader, pgid);
        self.stats.total_groups += 1;
    }

    #[inline]
    pub fn join_group(&mut self, pid: u64, pgid: u64) -> bool {
        if let Some(group) = self.groups.get_mut(&pgid) {
            group.members.push(pid);
            self.pid_to_pgid.insert(pid, pgid);
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn set_foreground(&mut self, pgid: u64) -> bool {
        if let Some(group) = self.groups.get_mut(&pgid) {
            group.state = CoopPgidState::Foreground;
            self.stats.fg_switches += 1;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn orphan_group(&mut self, pgid: u64) {
        if let Some(group) = self.groups.get_mut(&pgid) {
            group.state = CoopPgidState::Orphaned;
            self.stats.orphaned_groups += 1;
        }
    }

    #[inline]
    pub fn signal_group(&mut self, pgid: u64) -> usize {
        if let Some(group) = self.groups.get(&pgid) {
            self.stats.group_signals += 1;
            group.members.len()
        } else {
            0
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &CoopPgidStats {
        &self.stats
    }
}
