// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Setpgid (process group management application interface)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Process group operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppPgidOp {
    SetPgid,
    GetPgid,
    SetSid,
    GetSid,
}

/// Process group entry
#[derive(Debug, Clone)]
pub struct AppPgidEntry {
    pub pid: u64,
    pub pgid: u64,
    pub sid: u64,
    pub is_leader: bool,
}

/// Stats for pgid operations
#[derive(Debug, Clone)]
pub struct AppPgidStats {
    pub total_ops: u64,
    pub setpgid_calls: u64,
    pub getpgid_calls: u64,
    pub groups_created: u64,
    pub sessions_created: u64,
}

/// Manager for pgid application operations
pub struct AppSetpgidManager {
    processes: BTreeMap<u64, AppPgidEntry>,
    groups: BTreeMap<u64, Vec<u64>>,
    stats: AppPgidStats,
}

impl AppSetpgidManager {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            groups: BTreeMap::new(),
            stats: AppPgidStats {
                total_ops: 0,
                setpgid_calls: 0,
                getpgid_calls: 0,
                groups_created: 0,
                sessions_created: 0,
            },
        }
    }

    pub fn register_process(&mut self, pid: u64, pgid: u64, sid: u64) {
        let entry = AppPgidEntry {
            pid,
            pgid,
            sid,
            is_leader: pid == pgid,
        };
        self.processes.insert(pid, entry);
        self.groups.entry(pgid).or_insert_with(Vec::new).push(pid);
    }

    pub fn setpgid(&mut self, pid: u64, new_pgid: u64) -> bool {
        self.stats.total_ops += 1;
        self.stats.setpgid_calls += 1;
        if let Some(entry) = self.processes.get_mut(&pid) {
            let old_pgid = entry.pgid;
            if let Some(group) = self.groups.get_mut(&old_pgid) {
                group.retain(|&p| p != pid);
            }
            entry.pgid = new_pgid;
            entry.is_leader = pid == new_pgid;
            if !self.groups.contains_key(&new_pgid) {
                self.stats.groups_created += 1;
            }
            self.groups.entry(new_pgid).or_insert_with(Vec::new).push(pid);
            true
        } else {
            false
        }
    }

    pub fn getpgid(&mut self, pid: u64) -> Option<u64> {
        self.stats.total_ops += 1;
        self.stats.getpgid_calls += 1;
        self.processes.get(&pid).map(|e| e.pgid)
    }

    pub fn group_members(&self, pgid: u64) -> usize {
        self.groups.get(&pgid).map(|v| v.len()).unwrap_or(0)
    }

    pub fn stats(&self) -> &AppPgidStats {
        &self.stats
    }
}
