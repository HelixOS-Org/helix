// SPDX-License-Identifier: GPL-2.0
//! Apps clone3_app — clone3 system call implementation.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Clone flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clone3Flags(pub u64);

impl Clone3Flags {
    pub const NEWTIME: u64 = 1 << 0;
    pub const VM: u64 = 1 << 1;
    pub const FS: u64 = 1 << 2;
    pub const FILES: u64 = 1 << 3;
    pub const SIGHAND: u64 = 1 << 4;
    pub const PIDFD: u64 = 1 << 5;
    pub const PTRACE: u64 = 1 << 6;
    pub const VFORK: u64 = 1 << 7;
    pub const PARENT: u64 = 1 << 8;
    pub const THREAD: u64 = 1 << 9;
    pub const NEWNS: u64 = 1 << 10;
    pub const SYSVSEM: u64 = 1 << 11;
    pub const SETTLS: u64 = 1 << 12;
    pub const NEWCGROUP: u64 = 1 << 13;
    pub const NEWUTS: u64 = 1 << 14;
    pub const NEWIPC: u64 = 1 << 15;
    pub const NEWUSER: u64 = 1 << 16;
    pub const NEWPID: u64 = 1 << 17;
    pub const NEWNET: u64 = 1 << 18;
    pub const IO: u64 = 1 << 19;
    pub const INTO_CGROUP: u64 = 1 << 20;

    pub fn new() -> Self { Self(0) }
    pub fn set(&mut self, f: u64) { self.0 |= f; }
    pub fn has(&self, f: u64) -> bool { self.0 & f != 0 }
    pub fn is_thread(&self) -> bool { self.has(Self::THREAD) }
    pub fn creates_ns(&self) -> bool { self.0 & 0x1FC00 != 0 }
}

/// Clone3 args
#[derive(Debug, Clone)]
pub struct Clone3Args {
    pub flags: Clone3Flags,
    pub pidfd: i32,
    pub child_tid: u64,
    pub parent_tid: u64,
    pub exit_signal: i32,
    pub stack: u64,
    pub stack_size: u64,
    pub tls: u64,
    pub set_tid: Vec<u64>,
    pub cgroup: u64,
}

/// Clone result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clone3Result {
    Success { child_pid: u64, pidfd: i32 },
    TooManyProcesses,
    InvalidFlags,
    NoMemory,
    PermissionDenied,
}

/// Clone event
#[derive(Debug)]
pub struct Clone3Event {
    pub id: u64,
    pub parent_pid: u64,
    pub child_pid: u64,
    pub flags: Clone3Flags,
    pub result: Clone3Result,
    pub duration_ns: u64,
    pub timestamp: u64,
}

/// Stats
#[derive(Debug, Clone)]
pub struct Clone3AppStats {
    pub total_clones: u64,
    pub successful: u64,
    pub failed: u64,
    pub thread_creates: u64,
    pub ns_creates: u64,
    pub avg_duration_ns: u64,
}

/// Main clone3 app
pub struct AppClone3 {
    events: Vec<Clone3Event>,
    next_id: u64,
    max_events: usize,
}

impl AppClone3 {
    pub fn new() -> Self { Self { events: Vec::new(), next_id: 1, max_events: 8192 } }

    pub fn clone3(&mut self, parent: u64, child: u64, args: &Clone3Args, result: Clone3Result, duration: u64, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        if self.events.len() >= self.max_events { self.events.drain(..self.max_events / 2); }
        self.events.push(Clone3Event { id, parent_pid: parent, child_pid: child, flags: args.flags, result, duration_ns: duration, timestamp: now });
        id
    }

    pub fn stats(&self) -> Clone3AppStats {
        let success = self.events.iter().filter(|e| matches!(e.result, Clone3Result::Success { .. })).count() as u64;
        let failed = self.events.len() as u64 - success;
        let threads = self.events.iter().filter(|e| e.flags.is_thread()).count() as u64;
        let ns = self.events.iter().filter(|e| e.flags.creates_ns()).count() as u64;
        let durs: Vec<u64> = self.events.iter().map(|e| e.duration_ns).collect();
        let avg = if durs.is_empty() { 0 } else { durs.iter().sum::<u64>() / durs.len() as u64 };
        Clone3AppStats { total_clones: self.events.len() as u64, successful: success, failed, thread_creates: threads, ns_creates: ns, avg_duration_ns: avg }
    }
}
