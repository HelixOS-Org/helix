// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Getpid (process ID retrieval application interface)

extern crate alloc;
use alloc::collections::BTreeMap;

/// Process identity information
#[derive(Debug, Clone)]
pub struct AppProcessIdentity {
    pub pid: u64,
    pub ppid: u64,
    pub pgid: u64,
    pub sid: u64,
    pub uid: u32,
    pub gid: u32,
    pub euid: u32,
    pub egid: u32,
}

/// ID query type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppIdQuery {
    Pid,
    Ppid,
    Pgid,
    Sid,
    Uid,
    Gid,
    Euid,
    Egid,
    Tid,
}

/// Stats for ID queries
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AppGetpidStats {
    pub total_queries: u64,
    pub pid_queries: u64,
    pub ppid_queries: u64,
    pub uid_queries: u64,
    pub cache_hits: u64,
}

/// Manager for process ID operations
pub struct AppGetpidManager {
    identities: BTreeMap<u64, AppProcessIdentity>,
    stats: AppGetpidStats,
}

impl AppGetpidManager {
    pub fn new() -> Self {
        Self {
            identities: BTreeMap::new(),
            stats: AppGetpidStats {
                total_queries: 0,
                pid_queries: 0,
                ppid_queries: 0,
                uid_queries: 0,
                cache_hits: 0,
            },
        }
    }

    #[inline(always)]
    pub fn register(&mut self, identity: AppProcessIdentity) {
        self.identities.insert(identity.pid, identity);
    }

    pub fn query(&mut self, pid: u64, query: AppIdQuery) -> Option<u64> {
        self.stats.total_queries += 1;
        if let Some(id) = self.identities.get(&pid) {
            self.stats.cache_hits += 1;
            match query {
                AppIdQuery::Pid => { self.stats.pid_queries += 1; Some(id.pid) }
                AppIdQuery::Ppid => { self.stats.ppid_queries += 1; Some(id.ppid) }
                AppIdQuery::Pgid => Some(id.pgid),
                AppIdQuery::Sid => Some(id.sid),
                AppIdQuery::Uid => { self.stats.uid_queries += 1; Some(id.uid as u64) }
                AppIdQuery::Gid => Some(id.gid as u64),
                AppIdQuery::Euid => Some(id.euid as u64),
                AppIdQuery::Egid => Some(id.egid as u64),
                AppIdQuery::Tid => Some(pid),
            }
        } else {
            None
        }
    }

    #[inline]
    pub fn update_pgid(&mut self, pid: u64, pgid: u64) {
        if let Some(id) = self.identities.get_mut(&pid) {
            id.pgid = pgid;
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppGetpidStats {
        &self.stats
    }
}
