//! # Apps Process Group Manager
//!
//! Process group and session management:
//! - Session creation and leader tracking
//! - Process group lifecycle (setpgid/getpgid)
//! - Controlling terminal association
//! - Foreground/background group management
//! - Job control signal routing (SIGTSTP/SIGCONT)
//! - Orphaned process group detection

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Session descriptor
#[derive(Debug, Clone)]
pub struct SessionDesc {
    pub session_id: u64,
    pub leader_pid: u64,
    pub controlling_tty: Option<u64>,
    pub foreground_pgid: Option<u64>,
    pub groups: Vec<u64>,
    pub created_ns: u64,
}

impl SessionDesc {
    pub fn new(sid: u64, leader: u64, ts: u64) -> Self {
        Self {
            session_id: sid,
            leader_pid: leader,
            controlling_tty: None,
            foreground_pgid: None,
            groups: Vec::new(),
            created_ns: ts,
        }
    }

    pub fn add_group(&mut self, pgid: u64) {
        if !self.groups.contains(&pgid) { self.groups.push(pgid); }
    }

    pub fn remove_group(&mut self, pgid: u64) {
        self.groups.retain(|&g| g != pgid);
    }

    pub fn has_controlling_tty(&self) -> bool { self.controlling_tty.is_some() }
}

/// Process group descriptor
#[derive(Debug, Clone)]
pub struct ProcessGroup {
    pub pgid: u64,
    pub session_id: u64,
    pub leader_pid: u64,
    pub members: Vec<u64>,
    pub is_foreground: bool,
    pub is_orphaned: bool,
    pub stopped_count: u32,
    pub created_ns: u64,
}

impl ProcessGroup {
    pub fn new(pgid: u64, sid: u64, leader: u64, ts: u64) -> Self {
        Self {
            pgid, session_id: sid, leader_pid: leader,
            members: alloc::vec![leader],
            is_foreground: false, is_orphaned: false,
            stopped_count: 0, created_ns: ts,
        }
    }

    pub fn add_member(&mut self, pid: u64) {
        if !self.members.contains(&pid) { self.members.push(pid); }
    }

    pub fn remove_member(&mut self, pid: u64) {
        self.members.retain(|&m| m != pid);
    }

    pub fn member_count(&self) -> usize { self.members.len() }
    pub fn is_empty(&self) -> bool { self.members.is_empty() }
}

/// Per-process PG/session state
#[derive(Debug, Clone)]
pub struct ProcessPgState {
    pub process_id: u64,
    pub parent_pid: u64,
    pub pgid: u64,
    pub session_id: u64,
    pub is_session_leader: bool,
    pub is_group_leader: bool,
    pub stopped: bool,
}

/// Job control action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobAction {
    Stop,
    Continue,
    Terminate,
    BringToForeground,
    SendToBackground,
}

/// Apps PG manager stats
#[derive(Debug, Clone, Default)]
pub struct AppsPgMgrStats {
    pub total_sessions: usize,
    pub total_groups: usize,
    pub total_processes: usize,
    pub orphaned_groups: usize,
    pub foreground_groups: usize,
}

/// Apps Process Group Manager
pub struct AppsPgMgr {
    sessions: BTreeMap<u64, SessionDesc>,
    groups: BTreeMap<u64, ProcessGroup>,
    processes: BTreeMap<u64, ProcessPgState>,
    stats: AppsPgMgrStats,
}

impl AppsPgMgr {
    pub fn new() -> Self {
        Self {
            sessions: BTreeMap::new(),
            groups: BTreeMap::new(),
            processes: BTreeMap::new(),
            stats: AppsPgMgrStats::default(),
        }
    }

    pub fn setsid(&mut self, pid: u64, ts: u64) -> Option<u64> {
        // Process must not already be a group leader
        if let Some(proc_state) = self.processes.get(&pid) {
            if proc_state.is_group_leader { return None; }
        }

        let sid = pid; // Session ID == PID of leader
        let session = SessionDesc::new(sid, pid, ts);
        self.sessions.insert(sid, session);

        // Create a process group with pgid == pid
        let pg = ProcessGroup::new(pid, sid, pid, ts);
        self.groups.insert(pid, pg);
        if let Some(s) = self.sessions.get_mut(&sid) { s.add_group(pid); }

        // Update process state
        let proc_state = self.processes.entry(pid).or_insert_with(|| {
            ProcessPgState {
                process_id: pid, parent_pid: 0, pgid: pid,
                session_id: sid, is_session_leader: true,
                is_group_leader: true, stopped: false,
            }
        });
        proc_state.session_id = sid;
        proc_state.pgid = pid;
        proc_state.is_session_leader = true;
        proc_state.is_group_leader = true;

        Some(sid)
    }

    pub fn setpgid(&mut self, pid: u64, new_pgid: u64, ts: u64) -> bool {
        let pgid = if new_pgid == 0 { pid } else { new_pgid };

        // Remove from old group
        if let Some(proc_state) = self.processes.get(&pid) {
            let old_pgid = proc_state.pgid;
            if let Some(old_group) = self.groups.get_mut(&old_pgid) {
                old_group.remove_member(pid);
            }
        }

        // Get session
        let sid = self.processes.get(&pid).map(|p| p.session_id).unwrap_or(0);

        // Add to new group (create if needed)
        let group = self.groups.entry(pgid).or_insert_with(|| {
            let g = ProcessGroup::new(pgid, sid, pid, ts);
            if let Some(s) = self.sessions.get_mut(&sid) { s.add_group(pgid); }
            g
        });
        group.add_member(pid);

        // Update process state
        if let Some(proc_state) = self.processes.get_mut(&pid) {
            proc_state.pgid = pgid;
            proc_state.is_group_leader = pgid == pid;
        }

        true
    }

    pub fn register_process(&mut self, pid: u64, parent_pid: u64, pgid: u64, sid: u64) {
        self.processes.insert(pid, ProcessPgState {
            process_id: pid, parent_pid, pgid, session_id: sid,
            is_session_leader: false, is_group_leader: false, stopped: false,
        });
        if let Some(group) = self.groups.get_mut(&pgid) {
            group.add_member(pid);
        }
    }

    pub fn remove_process(&mut self, pid: u64) {
        if let Some(proc_state) = self.processes.remove(&pid) {
            if let Some(group) = self.groups.get_mut(&proc_state.pgid) {
                group.remove_member(pid);
            }
        }
        // Clean up empty groups
        let empty: Vec<u64> = self.groups.iter()
            .filter(|(_, g)| g.is_empty())
            .map(|(&pgid, _)| pgid)
            .collect();
        for pgid in empty {
            if let Some(group) = self.groups.remove(&pgid) {
                if let Some(session) = self.sessions.get_mut(&group.session_id) {
                    session.remove_group(pgid);
                }
            }
        }
    }

    pub fn set_foreground(&mut self, sid: u64, pgid: u64) -> bool {
        if let Some(session) = self.sessions.get_mut(&sid) {
            // Mark old foreground as background
            if let Some(old_pgid) = session.foreground_pgid {
                if let Some(old_group) = self.groups.get_mut(&old_pgid) {
                    old_group.is_foreground = false;
                }
            }
            session.foreground_pgid = Some(pgid);
            if let Some(group) = self.groups.get_mut(&pgid) {
                group.is_foreground = true;
            }
            true
        } else { false }
    }

    pub fn detect_orphaned_groups(&mut self) {
        for group in self.groups.values_mut() {
            // A process group is orphaned if no member has a parent
            // in a different process group within the same session
            let has_outside_parent = group.members.iter().any(|&pid| {
                if let Some(proc_state) = self.processes.get(&pid) {
                    if let Some(parent_state) = self.processes.get(&proc_state.parent_pid) {
                        return parent_state.pgid != group.pgid
                            && parent_state.session_id == group.session_id;
                    }
                }
                false
            });
            group.is_orphaned = !has_outside_parent;
        }
    }

    pub fn recompute(&mut self) {
        self.stats.total_sessions = self.sessions.len();
        self.stats.total_groups = self.groups.len();
        self.stats.total_processes = self.processes.len();
        self.stats.orphaned_groups = self.groups.values().filter(|g| g.is_orphaned).count();
        self.stats.foreground_groups = self.groups.values().filter(|g| g.is_foreground).count();
    }

    pub fn session(&self, sid: u64) -> Option<&SessionDesc> { self.sessions.get(&sid) }
    pub fn group(&self, pgid: u64) -> Option<&ProcessGroup> { self.groups.get(&pgid) }
    pub fn stats(&self) -> &AppsPgMgrStats { &self.stats }
}
