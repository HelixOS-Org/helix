// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Setsid (session management application interface)

extern crate alloc;
use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Session entry
#[derive(Debug, Clone)]
pub struct AppSessionEntry {
    pub sid: u64,
    pub leader_pid: u64,
    pub controlling_tty: Option<u64>,
    pub foreground_pgid: u64,
    pub member_count: u32,
    pub creation_time: u64,
}

/// Session operation result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppSetsidResult {
    Success,
    AlreadyLeader,
    PermissionDenied,
    InvalidPid,
}

/// Stats for session operations
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AppSetsidStats {
    pub total_ops: u64,
    pub sessions_created: u64,
    pub tty_attachments: u64,
    pub fg_changes: u64,
    pub errors: u64,
}

/// Manager for session application operations
pub struct AppSetsidManager {
    sessions: BTreeMap<u64, AppSessionEntry>,
    pid_to_sid: LinearMap<u64, 64>,
    stats: AppSetsidStats,
}

impl AppSetsidManager {
    pub fn new() -> Self {
        Self {
            sessions: BTreeMap::new(),
            pid_to_sid: LinearMap::new(),
            stats: AppSetsidStats {
                total_ops: 0,
                sessions_created: 0,
                tty_attachments: 0,
                fg_changes: 0,
                errors: 0,
            },
        }
    }

    pub fn setsid(&mut self, pid: u64) -> AppSetsidResult {
        self.stats.total_ops += 1;
        if self.sessions.contains_key(&pid) {
            self.stats.errors += 1;
            return AppSetsidResult::AlreadyLeader;
        }
        let entry = AppSessionEntry {
            sid: pid,
            leader_pid: pid,
            controlling_tty: None,
            foreground_pgid: pid,
            member_count: 1,
            creation_time: pid.wrapping_mul(47),
        };
        self.sessions.insert(pid, entry);
        self.pid_to_sid.insert(pid, pid);
        self.stats.sessions_created += 1;
        AppSetsidResult::Success
    }

    #[inline(always)]
    pub fn getsid(&self, pid: u64) -> Option<u64> {
        self.pid_to_sid.get(pid).cloned()
    }

    #[inline]
    pub fn set_controlling_tty(&mut self, sid: u64, tty: u64) -> bool {
        if let Some(session) = self.sessions.get_mut(&sid) {
            session.controlling_tty = Some(tty);
            self.stats.tty_attachments += 1;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn set_foreground(&mut self, sid: u64, pgid: u64) -> bool {
        if let Some(session) = self.sessions.get_mut(&sid) {
            session.foreground_pgid = pgid;
            self.stats.fg_changes += 1;
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppSetsidStats {
        &self.stats
    }
}
