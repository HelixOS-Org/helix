// SPDX-License-Identifier: GPL-2.0
//! Apps fcntl_app — file control operations application layer.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Fcntl command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FcntlCmd {
    DupFd,
    DupFdCloseOnExec,
    GetFd,
    SetFd,
    GetFl,
    SetFl,
    GetLk,
    SetLk,
    SetLkW,
    GetOwn,
    SetOwn,
    GetOwnEx,
    SetOwnEx,
    GetSig,
    SetSig,
    SetLease,
    GetLease,
    AddSeals,
    GetSeals,
}

/// File seal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSeal {
    Seal,
    Shrink,
    Grow,
    Write,
    FutureWrite,
}

/// Fcntl operation record
#[derive(Debug)]
pub struct FcntlOp {
    pub fd: u64,
    pub cmd: FcntlCmd,
    pub arg: u64,
    pub result: i64,
    pub timestamp: u64,
}

/// FD flags tracker
#[derive(Debug)]
pub struct FdFlagsTracker {
    pub fd: u64,
    pub close_on_exec: bool,
    pub status_flags: u32,
    pub seals: u32,
    pub lease_type: i32,
    pub owner_pid: i64,
    pub sig_num: i32,
}

impl FdFlagsTracker {
    pub fn new(fd: u64) -> Self {
        Self { fd, close_on_exec: false, status_flags: 0, seals: 0, lease_type: -1, owner_pid: 0, sig_num: 0 }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct FcntlAppStats {
    pub total_fds_tracked: u32,
    pub total_operations: u64,
    pub dup_count: u64,
    pub seal_count: u64,
    pub lock_count: u64,
}

/// Main app fcntl
pub struct AppFcntl {
    trackers: BTreeMap<u64, FdFlagsTracker>,
    ops: Vec<FcntlOp>,
}

impl AppFcntl {
    pub fn new() -> Self { Self { trackers: BTreeMap::new(), ops: Vec::new() } }

    pub fn track_fd(&mut self, fd: u64) { self.trackers.insert(fd, FdFlagsTracker::new(fd)); }

    pub fn record_op(&mut self, op: FcntlOp) {
        let fd = op.fd;
        let cmd = op.cmd;
        match cmd {
            FcntlCmd::SetFd => { if let Some(t) = self.trackers.get_mut(&fd) { t.close_on_exec = op.arg != 0; } }
            FcntlCmd::SetFl => { if let Some(t) = self.trackers.get_mut(&fd) { t.status_flags = op.arg as u32; } }
            FcntlCmd::SetOwn => { if let Some(t) = self.trackers.get_mut(&fd) { t.owner_pid = op.arg as i64; } }
            FcntlCmd::SetSig => { if let Some(t) = self.trackers.get_mut(&fd) { t.sig_num = op.arg as i32; } }
            FcntlCmd::AddSeals => { if let Some(t) = self.trackers.get_mut(&fd) { t.seals |= op.arg as u32; } }
            _ => {}
        }
        self.ops.push(op);
    }

    pub fn untrack(&mut self, fd: u64) { self.trackers.remove(&fd); }

    pub fn stats(&self) -> FcntlAppStats {
        let dups = self.ops.iter().filter(|o| matches!(o.cmd, FcntlCmd::DupFd | FcntlCmd::DupFdCloseOnExec)).count() as u64;
        let seals = self.ops.iter().filter(|o| matches!(o.cmd, FcntlCmd::AddSeals | FcntlCmd::GetSeals)).count() as u64;
        let locks = self.ops.iter().filter(|o| matches!(o.cmd, FcntlCmd::GetLk | FcntlCmd::SetLk | FcntlCmd::SetLkW)).count() as u64;
        FcntlAppStats { total_fds_tracked: self.trackers.len() as u32, total_operations: self.ops.len() as u64, dup_count: dups, seal_count: seals, lock_count: locks }
    }
}

// ============================================================================
// Merged from fcntl_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FcntlV2Cmd {
    DupFd,
    DupFdCloexec,
    GetFd,
    SetFd,
    GetFl,
    SetFl,
    GetLk,
    SetLk,
    SetLkw,
    OfdGetLk,
    OfdSetLk,
    OfdSetLkw,
    SetLease,
    GetLease,
    SetPipeSize,
    GetPipeSize,
    AddSeals,
    GetSeals,
    GetSig,
    SetSig,
    SetOwn,
    GetOwn,
}

/// Lock type for advisory locking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FcntlV2LockType {
    ReadLock,
    WriteLock,
    Unlock,
}

/// Lease type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FcntlV2LeaseType {
    ReadLease,
    WriteLease,
    NoLease,
}

/// An advisory file lock.
#[derive(Debug, Clone)]
pub struct FcntlV2Lock {
    pub lock_id: u64,
    pub fd: i32,
    pub pid: u64,
    pub lock_type: FcntlV2LockType,
    pub start: u64,
    pub length: u64,
    pub whence: i32,
    pub is_ofd: bool,
}

impl FcntlV2Lock {
    pub fn new(lock_id: u64, fd: i32, pid: u64, lock_type: FcntlV2LockType) -> Self {
        Self {
            lock_id,
            fd,
            pid,
            lock_type,
            start: 0,
            length: 0,
            whence: 0,
            is_ofd: false,
        }
    }

    pub fn conflicts_with(&self, other: &FcntlV2Lock) -> bool {
        if self.fd != other.fd {
            return false;
        }
        if self.lock_type == FcntlV2LockType::ReadLock
            && other.lock_type == FcntlV2LockType::ReadLock
        {
            return false;
        }
        // Range overlap
        let self_end = if self.length == 0 {
            u64::MAX
        } else {
            self.start + self.length
        };
        let other_end = if other.length == 0 {
            u64::MAX
        } else {
            other.start + other.length
        };
        self.start < other_end && other.start < self_end
    }
}

/// A file lease.
#[derive(Debug, Clone)]
pub struct FcntlV2Lease {
    pub fd: i32,
    pub pid: u64,
    pub lease_type: FcntlV2LeaseType,
    pub break_requested: bool,
}

impl FcntlV2Lease {
    pub fn new(fd: i32, pid: u64, lease_type: FcntlV2LeaseType) -> Self {
        Self {
            fd,
            pid,
            lease_type,
            break_requested: false,
        }
    }
}

/// Per-process fcntl state.
#[derive(Debug, Clone)]
pub struct ProcessFcntlV2State {
    pub pid: u64,
    pub locks: Vec<FcntlV2Lock>,
    pub leases: Vec<FcntlV2Lease>,
    pub fcntl_calls: u64,
    pub lock_conflicts: u64,
}

impl ProcessFcntlV2State {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            locks: Vec::new(),
            leases: Vec::new(),
            fcntl_calls: 0,
            lock_conflicts: 0,
        }
    }
}

/// Statistics for fcntl V2 app.
#[derive(Debug, Clone)]
pub struct FcntlV2AppStats {
    pub total_calls: u64,
    pub lock_operations: u64,
    pub lease_operations: u64,
    pub ofd_locks: u64,
    pub lock_conflicts: u64,
    pub lease_breaks: u64,
    pub seal_operations: u64,
}

/// Main apps fcntl V2 manager.
pub struct AppFcntlV2 {
    pub processes: BTreeMap<u64, ProcessFcntlV2State>,
    pub next_lock_id: u64,
    pub stats: FcntlV2AppStats,
}

impl AppFcntlV2 {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            next_lock_id: 1,
            stats: FcntlV2AppStats {
                total_calls: 0,
                lock_operations: 0,
                lease_operations: 0,
                ofd_locks: 0,
                lock_conflicts: 0,
                lease_breaks: 0,
                seal_operations: 0,
            },
        }
    }

    pub fn record_lock(
        &mut self,
        pid: u64,
        fd: i32,
        lock_type: FcntlV2LockType,
        start: u64,
        length: u64,
        is_ofd: bool,
    ) -> u64 {
        let id = self.next_lock_id;
        self.next_lock_id += 1;
        let mut lock = FcntlV2Lock::new(id, fd, pid, lock_type);
        lock.start = start;
        lock.length = length;
        lock.is_ofd = is_ofd;
        let state = self.processes.entry(pid).or_insert_with(|| ProcessFcntlV2State::new(pid));
        state.locks.push(lock);
        state.fcntl_calls += 1;
        self.stats.total_calls += 1;
        self.stats.lock_operations += 1;
        if is_ofd {
            self.stats.ofd_locks += 1;
        }
        id
    }

    pub fn record_lease(
        &mut self,
        pid: u64,
        fd: i32,
        lease_type: FcntlV2LeaseType,
    ) {
        let state = self.processes.entry(pid).or_insert_with(|| ProcessFcntlV2State::new(pid));
        let lease = FcntlV2Lease::new(fd, pid, lease_type);
        state.leases.push(lease);
        state.fcntl_calls += 1;
        self.stats.total_calls += 1;
        self.stats.lease_operations += 1;
    }

    pub fn process_count(&self) -> usize {
        self.processes.len()
    }
}
