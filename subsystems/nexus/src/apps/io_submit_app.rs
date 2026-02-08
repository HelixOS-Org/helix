// SPDX-License-Identifier: GPL-2.0
//! Apps io_submit_app — AIO io_submit application layer.

extern crate alloc;

use alloc::collections::BTreeMap;

/// IO submit operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoSubmitOp {
    Read,
    Write,
    Fsync,
    Fdsync,
    Poll,
    Noop,
}

/// IO submit state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoSubmitState {
    Pending,
    InFlight,
    Completed,
    Cancelled,
    Error,
}

/// IO control block
#[derive(Debug)]
pub struct IoCb {
    pub id: u64,
    pub fd: u64,
    pub op: IoSubmitOp,
    pub offset: u64,
    pub nbytes: u64,
    pub state: IoSubmitState,
    pub result: i64,
    pub submit_time: u64,
    pub complete_time: u64,
}

impl IoCb {
    pub fn new(id: u64, fd: u64, op: IoSubmitOp, offset: u64, nbytes: u64, now: u64) -> Self {
        Self { id, fd, op, offset, nbytes, state: IoSubmitState::Pending, result: 0, submit_time: now, complete_time: 0 }
    }

    pub fn latency_ns(&self) -> u64 {
        if self.complete_time > self.submit_time { self.complete_time - self.submit_time } else { 0 }
    }
}

/// AIO context
#[derive(Debug)]
pub struct AioContext {
    pub ctx_id: u64,
    pub max_events: u32,
    pub pending: u32,
    pub completed: u32,
    pub total_submitted: u64,
    pub total_completed: u64,
}

impl AioContext {
    pub fn new(id: u64, max: u32) -> Self {
        Self { ctx_id: id, max_events: max, pending: 0, completed: 0, total_submitted: 0, total_completed: 0 }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct IoSubmitAppStats {
    pub contexts: u32,
    pub total_submitted: u64,
    pub total_completed: u64,
    pub total_pending: u32,
}

/// Main app io_submit
pub struct AppIoSubmit {
    contexts: BTreeMap<u64, AioContext>,
    iocbs: BTreeMap<u64, IoCb>,
    next_id: u64,
}

impl AppIoSubmit {
    pub fn new() -> Self { Self { contexts: BTreeMap::new(), iocbs: BTreeMap::new(), next_id: 1 } }

    pub fn setup(&mut self, max: u32) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.contexts.insert(id, AioContext::new(id, max));
        id
    }

    pub fn submit(&mut self, ctx: u64, fd: u64, op: IoSubmitOp, offset: u64, nbytes: u64, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        if let Some(c) = self.contexts.get_mut(&ctx) {
            c.pending += 1; c.total_submitted += 1;
            self.iocbs.insert(id, IoCb::new(id, fd, op, offset, nbytes, now));
        }
        id
    }

    pub fn complete(&mut self, iocb_id: u64, result: i64, now: u64) {
        if let Some(cb) = self.iocbs.get_mut(&iocb_id) {
            cb.state = IoSubmitState::Completed;
            cb.result = result;
            cb.complete_time = now;
        }
    }

    pub fn destroy(&mut self, ctx: u64) { self.contexts.remove(&ctx); }

    pub fn stats(&self) -> IoSubmitAppStats {
        let sub: u64 = self.contexts.values().map(|c| c.total_submitted).sum();
        let comp: u64 = self.contexts.values().map(|c| c.total_completed).sum();
        let pend: u32 = self.contexts.values().map(|c| c.pending).sum();
        IoSubmitAppStats { contexts: self.contexts.len() as u32, total_submitted: sub, total_completed: comp, total_pending: pend }
    }
}
