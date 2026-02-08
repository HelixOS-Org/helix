// SPDX-License-Identifier: GPL-2.0
//! Apps waitid_app — advanced process waiting (waitid).

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Wait ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitIdType {
    Pid,
    Pgid,
    All,
    PidFd,
}

/// Wait options
#[derive(Debug, Clone, Copy)]
pub struct WaitIdOptions(pub u32);

impl WaitIdOptions {
    pub const WNOHANG: u32 = 1;
    pub const WUNTRACED: u32 = 2;
    pub const WSTOPPED: u32 = 2;
    pub const WEXITED: u32 = 4;
    pub const WCONTINUED: u32 = 8;
    pub const WNOWAIT: u32 = 16;
    pub fn has(&self, f: u32) -> bool { self.0 & f != 0 }
}

/// Child status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildStatus {
    Exited(i32),
    Killed(u32),
    Stopped(u32),
    Continued,
    Trapped,
}

/// Siginfo for waitid
#[derive(Debug, Clone)]
pub struct WaitIdSiginfo {
    pub pid: u64,
    pub uid: u32,
    pub status: ChildStatus,
    pub code: i32,
    pub timestamp: u64,
}

/// Process wait state
#[derive(Debug)]
pub struct ProcessWaitState {
    pub pid: u64,
    pub children: Vec<u64>,
    pub wait_count: u64,
    pub collected_count: u64,
    pub nohang_count: u64,
    pub zombie_children: u32,
}

impl ProcessWaitState {
    pub fn new(pid: u64) -> Self { Self { pid, children: Vec::new(), wait_count: 0, collected_count: 0, nohang_count: 0, zombie_children: 0 } }
    pub fn add_child(&mut self, child: u64) { self.children.push(child); }
}

/// Stats
#[derive(Debug, Clone)]
pub struct WaitIdAppStats {
    pub tracked_processes: u32,
    pub total_waits: u64,
    pub total_collected: u64,
    pub total_zombies: u32,
    pub nohang_calls: u64,
}

/// Main waitid app
pub struct AppWaitId {
    processes: BTreeMap<u64, ProcessWaitState>,
    events: Vec<WaitIdSiginfo>,
    max_events: usize,
}

impl AppWaitId {
    pub fn new() -> Self { Self { processes: BTreeMap::new(), events: Vec::new(), max_events: 4096 } }
    pub fn register(&mut self, pid: u64) { self.processes.insert(pid, ProcessWaitState::new(pid)); }

    pub fn waitid(&mut self, pid: u64, id_type: WaitIdType, options: WaitIdOptions) {
        if let Some(p) = self.processes.get_mut(&pid) {
            p.wait_count += 1;
            if options.has(WaitIdOptions::WNOHANG) { p.nohang_count += 1; }
        }
    }

    pub fn collect(&mut self, parent: u64, info: WaitIdSiginfo) {
        if let Some(p) = self.processes.get_mut(&parent) { p.collected_count += 1; }
        if self.events.len() >= self.max_events { self.events.drain(..self.max_events / 2); }
        self.events.push(info);
    }

    pub fn stats(&self) -> WaitIdAppStats {
        let waits: u64 = self.processes.values().map(|p| p.wait_count).sum();
        let collected: u64 = self.processes.values().map(|p| p.collected_count).sum();
        let zombies: u32 = self.processes.values().map(|p| p.zombie_children).sum();
        let nohang: u64 = self.processes.values().map(|p| p.nohang_count).sum();
        WaitIdAppStats { tracked_processes: self.processes.len() as u32, total_waits: waits, total_collected: collected, total_zombies: zombies, nohang_calls: nohang }
    }
}
