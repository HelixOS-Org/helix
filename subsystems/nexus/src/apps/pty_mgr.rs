//! # Apps PTY Manager
//!
//! PTY session management for application tracking:
//! - Master/slave pair allocation
//! - Session leader and controlling terminal tracking
//! - Window size change propagation
//! - PTY I/O statistics
//! - Job control state tracking
//! - Orphaned process group detection

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// PTY state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyState {
    Allocated,
    Active,
    Hung,
    Closed,
}

/// Window size
#[derive(Debug, Clone, Copy)]
pub struct PtyWinSize {
    pub rows: u16,
    pub cols: u16,
    pub xpixel: u16,
    pub ypixel: u16,
}

impl PtyWinSize {
    pub fn new(rows: u16, cols: u16) -> Self {
        Self { rows, cols, xpixel: 0, ypixel: 0 }
    }
}

/// PTY pair
#[derive(Debug, Clone)]
pub struct PtyPair {
    pub id: u64,
    pub master_fd: i32,
    pub slave_fd: i32,
    pub slave_name: String,
    pub state: PtyState,
    pub winsize: PtyWinSize,
    pub session_leader: u64,
    pub foreground_pgrp: u64,
    pub controlling_pid: u64,
    pub bytes_written: u64,
    pub bytes_read: u64,
    pub write_ops: u64,
    pub read_ops: u64,
    pub created_ts: u64,
    pub last_io_ts: u64,
    pub member_pids: Vec<u64>,
}

impl PtyPair {
    pub fn new(id: u64, master: i32, slave: i32, name: String, ts: u64) -> Self {
        Self {
            id, master_fd: master, slave_fd: slave, slave_name: name,
            state: PtyState::Allocated, winsize: PtyWinSize::new(24, 80),
            session_leader: 0, foreground_pgrp: 0, controlling_pid: 0,
            bytes_written: 0, bytes_read: 0, write_ops: 0, read_ops: 0,
            created_ts: ts, last_io_ts: 0, member_pids: Vec::new(),
        }
    }

    pub fn activate(&mut self, session_leader: u64) {
        self.state = PtyState::Active;
        self.session_leader = session_leader;
        self.controlling_pid = session_leader;
    }

    pub fn set_foreground(&mut self, pgrp: u64) { self.foreground_pgrp = pgrp; }
    pub fn set_winsize(&mut self, ws: PtyWinSize) { self.winsize = ws; }

    pub fn record_write(&mut self, bytes: usize, ts: u64) {
        self.bytes_written += bytes as u64;
        self.write_ops += 1;
        self.last_io_ts = ts;
    }

    pub fn record_read(&mut self, bytes: usize, ts: u64) {
        self.bytes_read += bytes as u64;
        self.read_ops += 1;
        self.last_io_ts = ts;
    }

    pub fn add_member(&mut self, pid: u64) {
        if !self.member_pids.contains(&pid) { self.member_pids.push(pid); }
    }

    pub fn remove_member(&mut self, pid: u64) {
        self.member_pids.retain(|&p| p != pid);
    }

    pub fn hangup(&mut self) { self.state = PtyState::Hung; }

    pub fn throughput_bps(&self, elapsed_ns: u64) -> f64 {
        if elapsed_ns == 0 { return 0.0; }
        let total = self.bytes_written + self.bytes_read;
        (total as f64 * 1_000_000_000.0) / elapsed_ns as f64
    }

    pub fn is_idle(&self, now: u64, threshold_ns: u64) -> bool {
        if self.last_io_ts == 0 { return now - self.created_ts > threshold_ns; }
        now - self.last_io_ts > threshold_ns
    }
}

/// Job control state
#[derive(Debug, Clone)]
pub struct JobControlState {
    pub session_id: u64,
    pub process_groups: BTreeMap<u64, Vec<u64>>,
    pub stopped_groups: Vec<u64>,
    pub orphaned_groups: Vec<u64>,
}

impl JobControlState {
    pub fn new(sid: u64) -> Self {
        Self { session_id: sid, process_groups: BTreeMap::new(), stopped_groups: Vec::new(), orphaned_groups: Vec::new() }
    }

    pub fn add_to_group(&mut self, pgrp: u64, pid: u64) {
        self.process_groups.entry(pgrp).or_insert_with(Vec::new).push(pid);
    }

    pub fn remove_pid(&mut self, pid: u64) {
        for group in self.process_groups.values_mut() { group.retain(|&p| p != pid); }
        self.process_groups.retain(|_, g| !g.is_empty());
    }

    pub fn mark_stopped(&mut self, pgrp: u64) {
        if !self.stopped_groups.contains(&pgrp) { self.stopped_groups.push(pgrp); }
    }

    pub fn mark_continued(&mut self, pgrp: u64) { self.stopped_groups.retain(|&g| g != pgrp); }

    pub fn detect_orphaned(&mut self) -> Vec<u64> {
        // simplified: groups with no parent in session
        let all_pids: Vec<u64> = self.process_groups.values().flat_map(|g| g.iter().copied()).collect();
        let mut orphaned = Vec::new();
        for (&pgrp, members) in &self.process_groups {
            if members.iter().all(|&p| p == pgrp || !all_pids.contains(&p)) {
                if !orphaned.contains(&pgrp) { orphaned.push(pgrp); }
            }
        }
        self.orphaned_groups = orphaned.clone();
        orphaned
    }
}

/// PTY manager stats
#[derive(Debug, Clone, Default)]
pub struct PtyMgrStats {
    pub total_ptys: usize,
    pub active_ptys: usize,
    pub hung_ptys: usize,
    pub total_bytes_written: u64,
    pub total_bytes_read: u64,
    pub total_sessions: usize,
    pub idle_ptys: usize,
}

/// Apps PTY manager
pub struct AppsPtyMgr {
    ptys: BTreeMap<u64, PtyPair>,
    sessions: BTreeMap<u64, JobControlState>,
    next_id: u64,
    idle_threshold_ns: u64,
    stats: PtyMgrStats,
}

impl AppsPtyMgr {
    pub fn new() -> Self {
        Self {
            ptys: BTreeMap::new(), sessions: BTreeMap::new(),
            next_id: 1, idle_threshold_ns: 60_000_000_000,
            stats: PtyMgrStats::default(),
        }
    }

    pub fn allocate(&mut self, master: i32, slave: i32, name: String, ts: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.ptys.insert(id, PtyPair::new(id, master, slave, name, ts));
        id
    }

    pub fn activate(&mut self, pty_id: u64, session_leader: u64) {
        if let Some(pty) = self.ptys.get_mut(&pty_id) {
            pty.activate(session_leader);
            self.sessions.entry(session_leader).or_insert_with(|| JobControlState::new(session_leader));
        }
    }

    pub fn record_write(&mut self, pty_id: u64, bytes: usize, ts: u64) {
        if let Some(pty) = self.ptys.get_mut(&pty_id) { pty.record_write(bytes, ts); }
    }

    pub fn record_read(&mut self, pty_id: u64, bytes: usize, ts: u64) {
        if let Some(pty) = self.ptys.get_mut(&pty_id) { pty.record_read(bytes, ts); }
    }

    pub fn set_winsize(&mut self, pty_id: u64, ws: PtyWinSize) {
        if let Some(pty) = self.ptys.get_mut(&pty_id) { pty.set_winsize(ws); }
    }

    pub fn hangup(&mut self, pty_id: u64) {
        if let Some(pty) = self.ptys.get_mut(&pty_id) { pty.hangup(); }
    }

    pub fn close(&mut self, pty_id: u64) { self.ptys.remove(&pty_id); }

    pub fn recompute(&mut self, now: u64) {
        self.stats.total_ptys = self.ptys.len();
        self.stats.active_ptys = self.ptys.values().filter(|p| p.state == PtyState::Active).count();
        self.stats.hung_ptys = self.ptys.values().filter(|p| p.state == PtyState::Hung).count();
        self.stats.total_bytes_written = self.ptys.values().map(|p| p.bytes_written).sum();
        self.stats.total_bytes_read = self.ptys.values().map(|p| p.bytes_read).sum();
        self.stats.total_sessions = self.sessions.len();
        self.stats.idle_ptys = self.ptys.values().filter(|p| p.is_idle(now, self.idle_threshold_ns)).count();
    }

    pub fn pty(&self, id: u64) -> Option<&PtyPair> { self.ptys.get(&id) }
    pub fn stats(&self) -> &PtyMgrStats { &self.stats }
}
