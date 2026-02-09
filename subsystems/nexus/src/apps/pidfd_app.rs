// SPDX-License-Identifier: GPL-2.0
//! Apps pidfd_app — pidfd process file descriptor management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Pidfd flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PidfdFlags(pub u32);

impl PidfdFlags {
    pub const NONBLOCK: u32 = 1 << 0;
    pub const THREAD: u32 = 1 << 1;

    pub fn new() -> Self { Self(0) }
    #[inline(always)]
    pub fn set(&mut self, f: u32) { self.0 |= f; }
    #[inline(always)]
    pub fn has(&self, f: u32) -> bool { self.0 & f != 0 }
}

/// Pidfd state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PidfdState {
    Active,
    Exited,
    Zombie,
    Closed,
}

/// Pidfd instance
#[derive(Debug)]
pub struct PidfdInstance {
    pub fd: i32,
    pub target_pid: u64,
    pub flags: PidfdFlags,
    pub state: PidfdState,
    pub created_at: u64,
    pub exit_code: i32,
    pub signal_count: u64,
    pub wait_count: u64,
    pub poll_count: u64,
}

impl PidfdInstance {
    pub fn new(fd: i32, pid: u64, flags: PidfdFlags, now: u64) -> Self {
        Self { fd, target_pid: pid, flags, state: PidfdState::Active, created_at: now, exit_code: 0, signal_count: 0, wait_count: 0, poll_count: 0 }
    }

    #[inline(always)]
    pub fn send_signal(&mut self) { self.signal_count += 1; }
    #[inline(always)]
    pub fn wait(&mut self) { self.wait_count += 1; }
    #[inline(always)]
    pub fn poll(&mut self) { self.poll_count += 1; }
    #[inline(always)]
    pub fn process_exit(&mut self, code: i32) { self.state = PidfdState::Exited; self.exit_code = code; }
    #[inline(always)]
    pub fn close(&mut self) { self.state = PidfdState::Closed; }
    #[inline(always)]
    pub fn is_active(&self) -> bool { self.state == PidfdState::Active }
}

/// Pidfd getfd operation
#[derive(Debug, Clone)]
pub struct PidfdGetfdOp {
    pub source_pid: u64,
    pub source_fd: i32,
    pub target_fd: i32,
    pub flags: u32,
    pub timestamp: u64,
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PidfdAppStats {
    pub total_pidfds: u32,
    pub active: u32,
    pub exited: u32,
    pub total_signals: u64,
    pub total_waits: u64,
    pub total_getfd: u64,
}

/// Main pidfd app
pub struct AppPidfd {
    pidfds: BTreeMap<i32, PidfdInstance>,
    getfd_ops: Vec<PidfdGetfdOp>,
    next_fd: i32,
}

impl AppPidfd {
    pub fn new() -> Self { Self { pidfds: BTreeMap::new(), getfd_ops: Vec::new(), next_fd: 500 } }

    #[inline]
    pub fn open(&mut self, pid: u64, flags: PidfdFlags, now: u64) -> i32 {
        let fd = self.next_fd; self.next_fd += 1;
        self.pidfds.insert(fd, PidfdInstance::new(fd, pid, flags, now));
        fd
    }

    #[inline(always)]
    pub fn send_signal(&mut self, fd: i32) -> bool {
        if let Some(p) = self.pidfds.get_mut(&fd) { if p.is_active() { p.send_signal(); return true; } }
        false
    }

    #[inline]
    pub fn getfd(&mut self, fd: i32, source_fd: i32, now: u64) -> Option<i32> {
        let pidfd = self.pidfds.get(&fd)?;
        if !pidfd.is_active() { return None; }
        let new_fd = self.next_fd; self.next_fd += 1;
        self.getfd_ops.push(PidfdGetfdOp { source_pid: pidfd.target_pid, source_fd, target_fd: new_fd, flags: 0, timestamp: now });
        Some(new_fd)
    }

    #[inline]
    pub fn stats(&self) -> PidfdAppStats {
        let active = self.pidfds.values().filter(|p| p.state == PidfdState::Active).count() as u32;
        let exited = self.pidfds.values().filter(|p| p.state == PidfdState::Exited).count() as u32;
        let sigs: u64 = self.pidfds.values().map(|p| p.signal_count).sum();
        let waits: u64 = self.pidfds.values().map(|p| p.wait_count).sum();
        PidfdAppStats { total_pidfds: self.pidfds.len() as u32, active, exited, total_signals: sigs, total_waits: waits, total_getfd: self.getfd_ops.len() as u64 }
    }
}
