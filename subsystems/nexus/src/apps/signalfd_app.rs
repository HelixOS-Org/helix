// SPDX-License-Identifier: GPL-2.0
//! Apps signalfd_app — signal file descriptor management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Signal number
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalNum {
    Sighup = 1, Sigint = 2, Sigquit = 3, Sigill = 4,
    Sigtrap = 5, Sigabrt = 6, Sigbus = 7, Sigfpe = 8,
    Sigkill = 9, Sigusr1 = 10, Sigsegv = 11, Sigusr2 = 12,
    Sigpipe = 13, Sigalrm = 14, Sigterm = 15, Sigstkflt = 16,
    Sigchld = 17, Sigcont = 18, Sigstop = 19, Sigtstp = 20,
    Sigttin = 21, Sigttou = 22, Sigurg = 23, Sigxcpu = 24,
    Sigxfsz = 25, Sigvtalrm = 26, Sigprof = 27, Sigwinch = 28,
    Sigio = 29, Sigpwr = 30, Sigsys = 31,
}

/// Signal mask (64-bit)
#[derive(Debug, Clone, Copy)]
pub struct SigMask(pub u64);

impl SigMask {
    #[inline(always)]
    pub fn empty() -> Self { Self(0) }
    #[inline(always)]
    pub fn full() -> Self { Self(u64::MAX) }
    #[inline(always)]
    pub fn add(&mut self, sig: u32) { if sig > 0 && sig <= 64 { self.0 |= 1u64 << (sig - 1); } }
    #[inline(always)]
    pub fn remove(&mut self, sig: u32) { if sig > 0 && sig <= 64 { self.0 &= !(1u64 << (sig - 1)); } }
    #[inline(always)]
    pub fn contains(&self, sig: u32) -> bool { sig > 0 && sig <= 64 && self.0 & (1u64 << (sig - 1)) != 0 }
}

/// Signalfd info
#[derive(Debug, Clone)]
pub struct SignalfdInfo {
    pub signo: u32,
    pub errno: i32,
    pub code: i32,
    pub pid: u32,
    pub uid: u32,
    pub timestamp: u64,
}

/// Signalfd instance
#[derive(Debug)]
pub struct SignalfdInstance {
    pub id: u64,
    pub mask: SigMask,
    pub owner_pid: u64,
    pub pending: VecDeque<SignalfdInfo>,
    pub read_count: u64,
    pub max_pending: usize,
}

impl SignalfdInstance {
    pub fn new(id: u64, mask: SigMask, pid: u64) -> Self {
        Self { id, mask, owner_pid: pid, pending: VecDeque::new(), read_count: 0, max_pending: 256 }
    }

    #[inline]
    pub fn deliver(&mut self, info: SignalfdInfo) {
        if !self.mask.contains(info.signo) { return; }
        if self.pending.len() >= self.max_pending { self.pending.pop_front(); }
        self.pending.push_back(info);
    }

    #[inline]
    pub fn read(&mut self) -> Option<SignalfdInfo> {
        if self.pending.is_empty() { return None; }
        self.read_count += 1;
        self.pending.pop_front()
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SignalfdAppStats {
    pub total_instances: u32,
    pub total_pending: u32,
    pub total_reads: u64,
    pub total_delivered: u64,
}

/// Main signalfd app
pub struct AppSignalfd {
    instances: BTreeMap<u64, SignalfdInstance>,
    next_id: u64,
    total_delivered: u64,
}

impl AppSignalfd {
    pub fn new() -> Self { Self { instances: BTreeMap::new(), next_id: 1, total_delivered: 0 } }

    #[inline]
    pub fn create(&mut self, mask: SigMask, pid: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.instances.insert(id, SignalfdInstance::new(id, mask, pid));
        id
    }

    #[inline(always)]
    pub fn close(&mut self, id: u64) { self.instances.remove(&id); }

    #[inline(always)]
    pub fn deliver(&mut self, id: u64, info: SignalfdInfo) {
        if let Some(inst) = self.instances.get_mut(&id) { inst.deliver(info); self.total_delivered += 1; }
    }

    #[inline]
    pub fn stats(&self) -> SignalfdAppStats {
        let pending: u32 = self.instances.values().map(|i| i.pending.len() as u32).sum();
        let reads: u64 = self.instances.values().map(|i| i.read_count).sum();
        SignalfdAppStats { total_instances: self.instances.len() as u32, total_pending: pending, total_reads: reads, total_delivered: self.total_delivered }
    }
}

// ============================================================================
// Merged from signalfd_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalV2Num {
    Hup,
    Int,
    Quit,
    Ill,
    Trap,
    Abrt,
    Bus,
    Fpe,
    Kill,
    Usr1,
    Segv,
    Usr2,
    Pipe,
    Alrm,
    Term,
    Chld,
    Cont,
    Stop,
    Tstp,
    Ttin,
    Ttou,
    Urg,
    Xcpu,
    Xfsz,
    Vtalrm,
    Prof,
    Winch,
    Io,
    Pwr,
    Sys,
}

/// Signalfd v2 instance
#[derive(Debug)]
pub struct SignalfdV2Instance {
    pub fd: u64,
    pub mask: u64,
    pub flags: u32,
    pub pending_count: u32,
    pub total_signals: u64,
    pub total_reads: u64,
    pub created_at: u64,
}

impl SignalfdV2Instance {
    pub fn new(fd: u64, mask: u64, flags: u32, now: u64) -> Self {
        Self { fd, mask, flags, pending_count: 0, total_signals: 0, total_reads: 0, created_at: now }
    }

    #[inline(always)]
    pub fn deliver(&mut self) { self.pending_count += 1; self.total_signals += 1; }
    #[inline(always)]
    pub fn read(&mut self) -> u32 { let c = self.pending_count; self.pending_count = 0; self.total_reads += 1; c }
}

/// Signal info v2
#[derive(Debug)]
pub struct SignalInfoV2 {
    pub signo: SignalV2Num,
    pub sender_pid: u64,
    pub sender_uid: u32,
    pub code: i32,
    pub timestamp: u64,
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SignalfdV2AppStats {
    pub total_instances: u32,
    pub total_signals: u64,
    pub total_reads: u64,
    pub total_pending: u32,
}

/// Main app signalfd v2
pub struct AppSignalfdV2 {
    instances: BTreeMap<u64, SignalfdV2Instance>,
    next_fd: u64,
}

impl AppSignalfdV2 {
    pub fn new() -> Self { Self { instances: BTreeMap::new(), next_fd: 1 } }

    #[inline]
    pub fn create(&mut self, mask: u64, flags: u32, now: u64) -> u64 {
        let fd = self.next_fd; self.next_fd += 1;
        self.instances.insert(fd, SignalfdV2Instance::new(fd, mask, flags, now));
        fd
    }

    #[inline(always)]
    pub fn deliver(&mut self, fd: u64) {
        if let Some(inst) = self.instances.get_mut(&fd) { inst.deliver(); }
    }

    #[inline(always)]
    pub fn read(&mut self, fd: u64) -> u32 {
        if let Some(inst) = self.instances.get_mut(&fd) { inst.read() } else { 0 }
    }

    #[inline(always)]
    pub fn close(&mut self, fd: u64) { self.instances.remove(&fd); }

    #[inline]
    pub fn stats(&self) -> SignalfdV2AppStats {
        let sigs: u64 = self.instances.values().map(|i| i.total_signals).sum();
        let reads: u64 = self.instances.values().map(|i| i.total_reads).sum();
        let pending: u32 = self.instances.values().map(|i| i.pending_count).sum();
        SignalfdV2AppStats { total_instances: self.instances.len() as u32, total_signals: sigs, total_reads: reads, total_pending: pending }
    }
}
