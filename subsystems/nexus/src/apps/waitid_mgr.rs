// SPDX-License-Identifier: GPL-2.0
//! Apps waitid_mgr — advanced wait/waitpid/waitid management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Wait target type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitIdType {
    Pid,
    Pgid,
    All,
    PidFd,
}

/// Wait options
#[derive(Debug, Clone, Copy)]
pub struct WaitOptions {
    pub bits: u32,
}

impl WaitOptions {
    pub const NOHANG: u32 = 1;
    pub const UNTRACED: u32 = 2;
    pub const CONTINUED: u32 = 4;
    pub const EXITED: u32 = 8;
    pub const STOPPED: u32 = 16;
    pub const NOWAIT: u32 = 32;
    pub const CLONE: u32 = 0x80000000;

    pub fn new(bits: u32) -> Self { Self { bits } }
    pub fn has(&self, flag: u32) -> bool { self.bits & flag != 0 }
    pub fn is_nohang(&self) -> bool { self.has(Self::NOHANG) }
}

/// Child exit status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    Exited(i32),
    Killed(u32),
    Dumped(u32),
    Stopped(u32),
    Continued,
    Trapped,
}

/// Siginfo for wait
#[derive(Debug, Clone)]
pub struct WaitSiginfo {
    pub pid: u64,
    pub uid: u32,
    pub status: ExitStatus,
    pub utime: u64,
    pub stime: u64,
}

/// Waiter (a process waiting for child events)
#[derive(Debug, Clone)]
pub struct WaiterEntry {
    pub waiter_pid: u64,
    pub id_type: WaitIdType,
    pub target_id: u64,
    pub options: WaitOptions,
    pub enqueued_at: u64,
    pub satisfied: bool,
    pub result: Option<WaitSiginfo>,
}

impl WaiterEntry {
    pub fn new(pid: u64, id_type: WaitIdType, target: u64, opts: WaitOptions, now: u64) -> Self {
        Self {
            waiter_pid: pid, id_type, target_id: target,
            options: opts, enqueued_at: now, satisfied: false, result: None,
        }
    }

    pub fn matches(&self, child_pid: u64, pgid: u64) -> bool {
        match self.id_type {
            WaitIdType::Pid => self.target_id == child_pid,
            WaitIdType::Pgid => self.target_id == pgid,
            WaitIdType::All => true,
            WaitIdType::PidFd => self.target_id == child_pid,
        }
    }

    pub fn satisfy(&mut self, info: WaitSiginfo) {
        self.satisfied = true;
        self.result = Some(info);
    }

    pub fn wait_time(&self, now: u64) -> u64 { now.saturating_sub(self.enqueued_at) }
}

/// Zombie process entry
#[derive(Debug, Clone)]
pub struct ZombieEntry {
    pub pid: u64,
    pub parent_pid: u64,
    pub pgid: u64,
    pub status: ExitStatus,
    pub utime: u64,
    pub stime: u64,
    pub exit_time: u64,
    pub reaped: bool,
}

impl ZombieEntry {
    pub fn new(pid: u64, parent: u64, pgid: u64, status: ExitStatus, now: u64) -> Self {
        Self {
            pid, parent_pid: parent, pgid, status,
            utime: 0, stime: 0, exit_time: now, reaped: false,
        }
    }

    pub fn zombie_time(&self, now: u64) -> u64 { now.saturating_sub(self.exit_time) }
}

/// Stats
#[derive(Debug, Clone)]
pub struct WaitIdMgrStats {
    pub total_waiters: u32,
    pub satisfied_waiters: u32,
    pub total_zombies: u32,
    pub reaped_zombies: u32,
    pub total_wait_events: u64,
    pub avg_wait_ns: u64,
}

/// Main waitid manager
pub struct AppWaitIdMgr {
    waiters: Vec<WaiterEntry>,
    zombies: BTreeMap<u64, ZombieEntry>,
    total_events: u64,
}

impl AppWaitIdMgr {
    pub fn new() -> Self {
        Self { waiters: Vec::new(), zombies: BTreeMap::new(), total_events: 0 }
    }

    pub fn wait(&mut self, pid: u64, id_type: WaitIdType, target: u64, opts: u32, now: u64) -> Option<WaitSiginfo> {
        let options = WaitOptions::new(opts);
        self.total_events += 1;

        // Check for existing zombies first
        let zombie_match = self.zombies.values().find(|z| {
            !z.reaped && z.parent_pid == pid && match id_type {
                WaitIdType::Pid => z.pid == target,
                WaitIdType::Pgid => z.pgid == target,
                WaitIdType::All => true,
                WaitIdType::PidFd => z.pid == target,
            }
        }).map(|z| z.pid);

        if let Some(zombie_pid) = zombie_match {
            if let Some(z) = self.zombies.get_mut(&zombie_pid) {
                if !options.has(WaitOptions::NOWAIT) { z.reaped = true; }
                return Some(WaitSiginfo {
                    pid: z.pid, uid: 0, status: z.status,
                    utime: z.utime, stime: z.stime,
                });
            }
        }

        if !options.is_nohang() {
            self.waiters.push(WaiterEntry::new(pid, id_type, target, options, now));
        }
        None
    }

    pub fn exit_child(&mut self, child_pid: u64, parent: u64, pgid: u64, status: ExitStatus, now: u64) {
        self.zombies.insert(child_pid, ZombieEntry::new(child_pid, parent, pgid, status, now));

        // Wake matching waiters
        for waiter in &mut self.waiters {
            if waiter.waiter_pid == parent && !waiter.satisfied && waiter.matches(child_pid, pgid) {
                waiter.satisfy(WaitSiginfo { pid: child_pid, uid: 0, status, utime: 0, stime: 0 });
            }
        }
    }

    pub fn stats(&self) -> WaitIdMgrStats {
        let satisfied = self.waiters.iter().filter(|w| w.satisfied).count() as u32;
        let reaped = self.zombies.values().filter(|z| z.reaped).count() as u32;
        WaitIdMgrStats {
            total_waiters: self.waiters.len() as u32, satisfied_waiters: satisfied,
            total_zombies: self.zombies.len() as u32, reaped_zombies: reaped,
            total_wait_events: self.total_events, avg_wait_ns: 0,
        }
    }
}
