// SPDX-License-Identifier: GPL-2.0
//! Apps io_uring_app — io_uring application interface.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// SQE opcode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoUringOp {
    Nop, Readv, Writev, Fsync, ReadFixed, WriteFixed,
    PollAdd, PollRemove, SyncFileRange, SendMsg, RecvMsg,
    Timeout, TimeoutRemove, Accept, AsyncCancel, LinkTimeout,
    Connect, Fallocate, OpenAt, Close, FilesUpdate,
    Statx, Read, Write, Fadvise, Madvise, Send, Recv,
    OpenAt2, EpollCtl, Splice, ProvideBuffers, RemoveBuffers,
    Tee, Shutdown, Renameat, Unlinkat, Mkdirat, Symlinkat,
    Linkat, MsgRing, Fsetxattr, Setxattr, Fgetxattr, Getxattr,
    Socket, UringCmd, SendZc, SendMsgZc, WaitId,
}

/// SQE flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SqeFlags(pub u32);

impl SqeFlags {
    pub const FIXED_FILE: u32 = 1 << 0;
    pub const IO_DRAIN: u32 = 1 << 1;
    pub const IO_LINK: u32 = 1 << 2;
    pub const IO_HARDLINK: u32 = 1 << 3;
    pub const ASYNC: u32 = 1 << 4;
    pub const BUFFER_SELECT: u32 = 1 << 5;
    pub const CQE_SKIP_SUCCESS: u32 = 1 << 6;

    pub fn new() -> Self { Self(0) }
    pub fn has(&self, f: u32) -> bool { self.0 & f != 0 }
}

/// Submission queue entry
#[derive(Debug)]
pub struct Sqe {
    pub user_data: u64,
    pub op: IoUringOp,
    pub flags: SqeFlags,
    pub fd: i32,
    pub off: u64,
    pub len: u32,
    pub submitted_at: u64,
}

/// Completion queue entry
#[derive(Debug, Clone)]
pub struct Cqe {
    pub user_data: u64,
    pub res: i32,
    pub flags: u32,
    pub completed_at: u64,
}

/// Ring instance
#[derive(Debug)]
pub struct IoUringInstance {
    pub id: u64,
    pub sq_entries: u32,
    pub cq_entries: u32,
    pub pending: Vec<Sqe>,
    pub completions: Vec<Cqe>,
    pub total_submitted: u64,
    pub total_completed: u64,
    pub total_bytes: u64,
    pub sq_polling: bool,
}

impl IoUringInstance {
    pub fn new(id: u64, sq: u32, cq: u32) -> Self {
        Self { id, sq_entries: sq, cq_entries: cq, pending: Vec::new(), completions: Vec::new(), total_submitted: 0, total_completed: 0, total_bytes: 0, sq_polling: false }
    }

    pub fn submit(&mut self, sqe: Sqe) { self.total_submitted += 1; self.pending.push(sqe); }

    pub fn complete(&mut self, user_data: u64, res: i32, now: u64) {
        self.pending.retain(|s| s.user_data != user_data);
        self.total_completed += 1;
        if self.completions.len() >= self.cq_entries as usize { self.completions.drain(..self.cq_entries as usize / 2); }
        self.completions.push(Cqe { user_data, res, flags: 0, completed_at: now });
    }

    pub fn reap(&mut self, max: u32) -> Vec<Cqe> {
        let n = (max as usize).min(self.completions.len());
        self.completions.drain(..n).collect()
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct IoUringAppStats {
    pub total_rings: u32,
    pub total_submitted: u64,
    pub total_completed: u64,
    pub total_pending: u32,
    pub sq_poll_rings: u32,
}

/// Main io_uring app
pub struct AppIoUring {
    rings: BTreeMap<u64, IoUringInstance>,
    next_id: u64,
}

impl AppIoUring {
    pub fn new() -> Self { Self { rings: BTreeMap::new(), next_id: 1 } }

    pub fn setup(&mut self, sq: u32, cq: u32) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.rings.insert(id, IoUringInstance::new(id, sq, cq));
        id
    }

    pub fn submit(&mut self, ring: u64, sqe: Sqe) {
        if let Some(r) = self.rings.get_mut(&ring) { r.submit(sqe); }
    }

    pub fn stats(&self) -> IoUringAppStats {
        let sub: u64 = self.rings.values().map(|r| r.total_submitted).sum();
        let comp: u64 = self.rings.values().map(|r| r.total_completed).sum();
        let pend: u32 = self.rings.values().map(|r| r.pending.len() as u32).sum();
        let poll = self.rings.values().filter(|r| r.sq_polling).count() as u32;
        IoUringAppStats { total_rings: self.rings.len() as u32, total_submitted: sub, total_completed: comp, total_pending: pend, sq_poll_rings: poll }
    }
}
