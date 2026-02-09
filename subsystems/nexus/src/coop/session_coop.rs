// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — Session (cooperative session management)

extern crate alloc;
use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Session cooperation state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopSessionState {
    Active,
    Detached,
    Closing,
    Orphan,
}

/// Session entry
#[derive(Debug, Clone)]
pub struct CoopSessionEntry {
    pub sid: u64,
    pub leader: u64,
    pub state: CoopSessionState,
    pub controlling_tty: Option<u64>,
    pub process_groups: Vec<u64>,
    pub foreground_pgid: u64,
}

/// Session cooperation stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CoopSessionStats {
    pub total_sessions: u64,
    pub active_sessions: u64,
    pub detached: u64,
    pub tty_assignments: u64,
    pub hangups_sent: u64,
}

/// Manager for cooperative session operations
pub struct CoopSessionManager {
    sessions: BTreeMap<u64, CoopSessionEntry>,
    pid_to_session: LinearMap<u64, 64>,
    stats: CoopSessionStats,
}

impl CoopSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: BTreeMap::new(),
            pid_to_session: LinearMap::new(),
            stats: CoopSessionStats {
                total_sessions: 0,
                active_sessions: 0,
                detached: 0,
                tty_assignments: 0,
                hangups_sent: 0,
            },
        }
    }

    pub fn create_session(&mut self, leader: u64) -> u64 {
        let sid = leader;
        let entry = CoopSessionEntry {
            sid,
            leader,
            state: CoopSessionState::Active,
            controlling_tty: None,
            process_groups: Vec::from([leader]),
            foreground_pgid: leader,
        };
        self.sessions.insert(sid, entry);
        self.pid_to_session.insert(leader, sid);
        self.stats.total_sessions += 1;
        self.stats.active_sessions += 1;
        sid
    }

    #[inline]
    pub fn assign_tty(&mut self, sid: u64, tty: u64) -> bool {
        if let Some(session) = self.sessions.get_mut(&sid) {
            session.controlling_tty = Some(tty);
            self.stats.tty_assignments += 1;
            true
        } else {
            false
        }
    }

    pub fn hangup(&mut self, sid: u64) -> usize {
        if let Some(session) = self.sessions.get_mut(&sid) {
            session.controlling_tty = None;
            session.state = CoopSessionState::Detached;
            self.stats.hangups_sent += 1;
            self.stats.detached += 1;
            self.stats.active_sessions = self.stats.active_sessions.saturating_sub(1);
            session.process_groups.len()
        } else {
            0
        }
    }

    #[inline]
    pub fn set_foreground_pgid(&mut self, sid: u64, pgid: u64) -> bool {
        if let Some(session) = self.sessions.get_mut(&sid) {
            session.foreground_pgid = pgid;
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &CoopSessionStats {
        &self.stats
    }
}
