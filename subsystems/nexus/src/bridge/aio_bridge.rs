//! # Bridge AIO Bridge
//!
//! Asynchronous I/O syscall bridging:
//! - AIO context management (io_setup/io_destroy)
//! - IO control block (IOCB) tracking
//! - Completion event monitoring
//! - Outstanding request tracking per context
//! - AIO ring buffer management
//! - Performance and latency statistics

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// AIO operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AioOp {
    Read,
    Write,
    Fsync,
    Fdsync,
    Poll,
    ReadV,
    WriteV,
}

/// AIO request state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AioState {
    Submitted,
    Running,
    Completed,
    Cancelled,
    Error,
}

/// AIO control block
#[derive(Debug, Clone)]
pub struct AioIocb {
    pub iocb_id: u64,
    pub ctx_id: u64,
    pub op: AioOp,
    pub state: AioState,
    pub fd: i32,
    pub offset: u64,
    pub nbytes: u64,
    pub result: i64,
    pub submit_ts: u64,
    pub complete_ts: u64,
    pub priority: i32,
}

impl AioIocb {
    pub fn new(id: u64, ctx: u64, op: AioOp, fd: i32, offset: u64, nbytes: u64, ts: u64) -> Self {
        Self {
            iocb_id: id, ctx_id: ctx, op, state: AioState::Submitted,
            fd, offset, nbytes, result: 0, submit_ts: ts,
            complete_ts: 0, priority: 0,
        }
    }

    #[inline]
    pub fn complete(&mut self, result: i64, ts: u64) {
        self.state = if result >= 0 { AioState::Completed } else { AioState::Error };
        self.result = result;
        self.complete_ts = ts;
    }

    #[inline(always)]
    pub fn cancel(&mut self, ts: u64) {
        self.state = AioState::Cancelled;
        self.complete_ts = ts;
    }

    #[inline(always)]
    pub fn latency_ns(&self) -> u64 {
        if self.complete_ts > self.submit_ts { self.complete_ts - self.submit_ts } else { 0 }
    }

    #[inline(always)]
    pub fn is_pending(&self) -> bool {
        matches!(self.state, AioState::Submitted | AioState::Running)
    }
}

/// AIO context
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AioContext {
    pub ctx_id: u64,
    pub owner_pid: u64,
    pub max_events: u32,
    pub outstanding: u32,
    pub total_submitted: u64,
    pub total_completed: u64,
    pub total_cancelled: u64,
    pub total_errors: u64,
    pub created_ts: u64,
    pub destroyed: bool,
}

impl AioContext {
    pub fn new(id: u64, owner: u64, max_events: u32, ts: u64) -> Self {
        Self {
            ctx_id: id, owner_pid: owner, max_events, outstanding: 0,
            total_submitted: 0, total_completed: 0, total_cancelled: 0,
            total_errors: 0, created_ts: ts, destroyed: false,
        }
    }

    #[inline]
    pub fn submit(&mut self) -> bool {
        if self.outstanding >= self.max_events { return false; }
        self.outstanding += 1;
        self.total_submitted += 1;
        true
    }

    #[inline(always)]
    pub fn complete(&mut self, success: bool) {
        self.outstanding = self.outstanding.saturating_sub(1);
        if success { self.total_completed += 1; } else { self.total_errors += 1; }
    }

    #[inline(always)]
    pub fn cancel_one(&mut self) {
        self.outstanding = self.outstanding.saturating_sub(1);
        self.total_cancelled += 1;
    }

    #[inline(always)]
    pub fn utilization(&self) -> f64 {
        if self.max_events == 0 { 0.0 } else { self.outstanding as f64 / self.max_events as f64 }
    }
}

/// Completion event
#[derive(Debug, Clone)]
pub struct AioEvent {
    pub iocb_id: u64,
    pub result: i64,
    pub result2: i64,
    pub timestamp: u64,
}

/// AIO bridge stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AioBridgeStats {
    pub total_contexts: usize,
    pub active_contexts: usize,
    pub total_outstanding: u32,
    pub total_submitted: u64,
    pub total_completed: u64,
    pub total_errors: u64,
    pub avg_latency_ns: u64,
    pub read_ops: u64,
    pub write_ops: u64,
}

/// Bridge AIO manager
#[repr(align(64))]
pub struct BridgeAioBridge {
    contexts: BTreeMap<u64, AioContext>,
    iocbs: BTreeMap<u64, AioIocb>,
    events: VecDeque<AioEvent>,
    max_events: usize,
    next_ctx_id: u64,
    next_iocb_id: u64,
    stats: AioBridgeStats,
}

impl BridgeAioBridge {
    pub fn new() -> Self {
        Self {
            contexts: BTreeMap::new(), iocbs: BTreeMap::new(),
            events: VecDeque::new(), max_events: 2048,
            next_ctx_id: 1, next_iocb_id: 1,
            stats: AioBridgeStats::default(),
        }
    }

    #[inline]
    pub fn io_setup(&mut self, owner: u64, max_events: u32, ts: u64) -> u64 {
        let id = self.next_ctx_id;
        self.next_ctx_id += 1;
        self.contexts.insert(id, AioContext::new(id, owner, max_events, ts));
        id
    }

    #[inline(always)]
    pub fn io_destroy(&mut self, ctx_id: u64) {
        if let Some(c) = self.contexts.get_mut(&ctx_id) { c.destroyed = true; }
    }

    #[inline]
    pub fn io_submit(&mut self, ctx_id: u64, op: AioOp, fd: i32, offset: u64, nbytes: u64, ts: u64) -> Option<u64> {
        if let Some(c) = self.contexts.get_mut(&ctx_id) {
            if !c.submit() { return None; }
            let id = self.next_iocb_id;
            self.next_iocb_id += 1;
            self.iocbs.insert(id, AioIocb::new(id, ctx_id, op, fd, offset, nbytes, ts));
            Some(id)
        } else { None }
    }

    #[inline]
    pub fn io_complete(&mut self, iocb_id: u64, result: i64, ts: u64) {
        if let Some(iocb) = self.iocbs.get_mut(&iocb_id) {
            let ctx_id = iocb.ctx_id;
            iocb.complete(result, ts);
            self.events.push_back(AioEvent { iocb_id, result, result2: 0, timestamp: ts });
            if let Some(c) = self.contexts.get_mut(&ctx_id) {
                c.complete(result >= 0);
            }
        }
        if self.events.len() > self.max_events { self.events.pop_front(); }
    }

    #[inline]
    pub fn io_cancel(&mut self, iocb_id: u64, ts: u64) {
        if let Some(iocb) = self.iocbs.get_mut(&iocb_id) {
            let ctx_id = iocb.ctx_id;
            iocb.cancel(ts);
            if let Some(c) = self.contexts.get_mut(&ctx_id) { c.cancel_one(); }
        }
    }

    #[inline]
    pub fn io_getevents(&self, ctx_id: u64) -> Vec<&AioEvent> {
        self.events.iter().filter(|e| {
            self.iocbs.get(&e.iocb_id).map(|i| i.ctx_id == ctx_id).unwrap_or(false)
        }).collect()
    }

    pub fn recompute(&mut self) {
        self.stats.total_contexts = self.contexts.len();
        self.stats.active_contexts = self.contexts.values().filter(|c| !c.destroyed).count();
        self.stats.total_outstanding = self.contexts.values().map(|c| c.outstanding).sum();
        self.stats.total_submitted = self.contexts.values().map(|c| c.total_submitted).sum();
        self.stats.total_completed = self.contexts.values().map(|c| c.total_completed).sum();
        self.stats.total_errors = self.contexts.values().map(|c| c.total_errors).sum();
        let lats: Vec<u64> = self.iocbs.values().filter(|i| !i.is_pending()).map(|i| i.latency_ns()).collect();
        self.stats.avg_latency_ns = if lats.is_empty() { 0 } else { lats.iter().sum::<u64>() / lats.len() as u64 };
        self.stats.read_ops = self.iocbs.values().filter(|i| matches!(i.op, AioOp::Read | AioOp::ReadV)).count() as u64;
        self.stats.write_ops = self.iocbs.values().filter(|i| matches!(i.op, AioOp::Write | AioOp::WriteV)).count() as u64;
    }

    #[inline(always)]
    pub fn context(&self, id: u64) -> Option<&AioContext> { self.contexts.get(&id) }
    #[inline(always)]
    pub fn iocb(&self, id: u64) -> Option<&AioIocb> { self.iocbs.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &AioBridgeStats { &self.stats }
}

// ============================================================================
// Merged from aio_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AioV2Op {
    Read,
    Write,
    Fsync,
    FDataSync,
    Poll,
    Noop,
    ReadV,
    WriteV,
}

/// Completion mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AioV2CompletionMode {
    Polling,
    Interrupt,
    Eventfd,
    Signal,
}

/// IO context state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AioCtxState {
    Active,
    Draining,
    Destroyed,
}

/// IO control block
#[derive(Debug, Clone)]
pub struct Iocb {
    pub id: u64,
    pub op: AioV2Op,
    pub fd: i32,
    pub offset: u64,
    pub length: u64,
    pub priority: i16,
    pub submitted_at: u64,
    pub completed_at: u64,
    pub result: i64,
    pub in_flight: bool,
}

impl Iocb {
    pub fn new(id: u64, op: AioV2Op, fd: i32, offset: u64, length: u64, now: u64) -> Self {
        Self { id, op, fd, offset, length, priority: 0, submitted_at: now, completed_at: 0, result: 0, in_flight: true }
    }

    #[inline]
    pub fn complete(&mut self, result: i64, now: u64) {
        self.result = result;
        self.completed_at = now;
        self.in_flight = false;
    }

    #[inline(always)]
    pub fn latency_ns(&self) -> u64 {
        if self.completed_at > 0 { self.completed_at - self.submitted_at } else { 0 }
    }
}

/// AIO context
#[derive(Debug)]
#[repr(align(64))]
pub struct AioContext {
    pub id: u64,
    pub state: AioCtxState,
    pub max_events: u32,
    pub iocbs: Vec<Iocb>,
    pub in_flight: u32,
    pub total_submitted: u64,
    pub total_completed: u64,
    pub total_bytes: u64,
    pub completion_mode: AioV2CompletionMode,
}

impl AioContext {
    pub fn new(id: u64, max_events: u32) -> Self {
        Self {
            id, state: AioCtxState::Active, max_events, iocbs: Vec::new(),
            in_flight: 0, total_submitted: 0, total_completed: 0,
            total_bytes: 0, completion_mode: AioV2CompletionMode::Interrupt,
        }
    }

    #[inline]
    pub fn submit(&mut self, iocb: Iocb) -> bool {
        if self.in_flight >= self.max_events { return false; }
        self.total_bytes += iocb.length;
        self.in_flight += 1;
        self.total_submitted += 1;
        self.iocbs.push(iocb);
        true
    }

    #[inline]
    pub fn complete(&mut self, iocb_id: u64, result: i64, now: u64) {
        if let Some(iocb) = self.iocbs.iter_mut().find(|i| i.id == iocb_id && i.in_flight) {
            iocb.complete(result, now);
            self.in_flight -= 1;
            self.total_completed += 1;
        }
    }

    #[inline(always)]
    pub fn throughput_mbps(&self, elapsed_ns: u64) -> f64 {
        if elapsed_ns == 0 { return 0.0; }
        (self.total_bytes as f64 / (1024.0 * 1024.0)) / (elapsed_ns as f64 / 1_000_000_000.0)
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AioV2BridgeStats {
    pub total_contexts: u32,
    pub total_in_flight: u32,
    pub total_submitted: u64,
    pub total_completed: u64,
    pub total_bytes: u64,
    pub avg_latency_ns: u64,
}

/// Main AIO v2 bridge
#[repr(align(64))]
pub struct BridgeAioV2 {
    contexts: BTreeMap<u64, AioContext>,
    next_id: u64,
    next_iocb_id: u64,
}

impl BridgeAioV2 {
    pub fn new() -> Self { Self { contexts: BTreeMap::new(), next_id: 1, next_iocb_id: 1 } }

    #[inline]
    pub fn create_context(&mut self, max_events: u32) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.contexts.insert(id, AioContext::new(id, max_events));
        id
    }

    #[inline]
    pub fn submit(&mut self, ctx: u64, op: AioV2Op, fd: i32, offset: u64, length: u64, now: u64) -> Option<u64> {
        let iocb_id = self.next_iocb_id; self.next_iocb_id += 1;
        let iocb = Iocb::new(iocb_id, op, fd, offset, length, now);
        if self.contexts.get_mut(&ctx)?.submit(iocb) { Some(iocb_id) } else { None }
    }

    #[inline]
    pub fn stats(&self) -> AioV2BridgeStats {
        let in_flight: u32 = self.contexts.values().map(|c| c.in_flight).sum();
        let submitted: u64 = self.contexts.values().map(|c| c.total_submitted).sum();
        let completed: u64 = self.contexts.values().map(|c| c.total_completed).sum();
        let bytes: u64 = self.contexts.values().map(|c| c.total_bytes).sum();
        let lats: Vec<u64> = self.contexts.values().flat_map(|c| &c.iocbs).filter(|i| !i.in_flight).map(|i| i.latency_ns()).collect();
        let avg = if lats.is_empty() { 0 } else { lats.iter().sum::<u64>() / lats.len() as u64 };
        AioV2BridgeStats { total_contexts: self.contexts.len() as u32, total_in_flight: in_flight, total_submitted: submitted, total_completed: completed, total_bytes: bytes, avg_latency_ns: avg }
    }
}

// ============================================================================
// Merged from aio_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AioV3OpType {
    PRead,
    PWrite,
    Fsync,
    Fdsync,
    Poll,
    PReadV,
    PWriteV,
    NoOp,
}

/// AIO v3 submission
#[derive(Debug)]
pub struct AioV3Submission {
    pub id: u64,
    pub op: AioV3OpType,
    pub fd: u64,
    pub offset: u64,
    pub nbytes: u64,
    pub priority: i16,
    pub submit_time: u64,
}

/// AIO v3 completion
#[derive(Debug)]
pub struct AioV3Completion {
    pub id: u64,
    pub result: i64,
    pub result2: i64,
    pub complete_time: u64,
}

/// AIO v3 ring
#[derive(Debug)]
pub struct AioV3Ring {
    pub ctx_id: u64,
    pub max_events: u32,
    pub submissions: Vec<AioV3Submission>,
    pub completions: Vec<AioV3Completion>,
    pub total_submitted: u64,
    pub total_completed: u64,
}

impl AioV3Ring {
    pub fn new(ctx_id: u64, max: u32) -> Self {
        Self { ctx_id, max_events: max, submissions: Vec::new(), completions: Vec::new(), total_submitted: 0, total_completed: 0 }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AioV3BridgeStats {
    pub total_rings: u32,
    pub total_submitted: u64,
    pub total_completed: u64,
    pub pending: u32,
}

/// Main bridge AIO v3
#[repr(align(64))]
pub struct BridgeAioV3 {
    rings: BTreeMap<u64, AioV3Ring>,
}

impl BridgeAioV3 {
    pub fn new() -> Self { Self { rings: BTreeMap::new() } }

    #[inline(always)]
    pub fn create_ring(&mut self, ctx_id: u64, max: u32) { self.rings.insert(ctx_id, AioV3Ring::new(ctx_id, max)); }

    #[inline(always)]
    pub fn submit(&mut self, ctx: u64, sub: AioV3Submission) {
        if let Some(r) = self.rings.get_mut(&ctx) { r.total_submitted += 1; r.submissions.push(sub); }
    }

    #[inline]
    pub fn complete(&mut self, ctx: u64, comp: AioV3Completion) {
        if let Some(r) = self.rings.get_mut(&ctx) {
            r.total_completed += 1;
            r.submissions.retain(|s| s.id != comp.id);
            r.completions.push(comp);
        }
    }

    #[inline]
    pub fn stats(&self) -> AioV3BridgeStats {
        let submitted: u64 = self.rings.values().map(|r| r.total_submitted).sum();
        let completed: u64 = self.rings.values().map(|r| r.total_completed).sum();
        let pending: u32 = self.rings.values().map(|r| r.submissions.len() as u32).sum();
        AioV3BridgeStats { total_rings: self.rings.len() as u32, total_submitted: submitted, total_completed: completed, pending }
    }
}

// ============================================================================
// Merged from aio_v4_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AioV4Op {
    Read,
    Write,
    Fsync,
    Fdatasync,
    ReadFixed,
    WriteFixed,
    PollAdd,
    PollRemove,
    Nop,
    Cancel,
    Timeout,
    LinkTimeout,
    Accept,
    Connect,
    Openat,
    Close,
    Statx,
    Splice,
    ProvideBuffers,
    RemoveBuffers,
}

/// Completion entry state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AioV4CqeState {
    Pending,
    Completed,
    Error,
    Cancelled,
    Overflow,
}

/// Submission queue entry.
#[derive(Debug, Clone)]
pub struct AioV4Sqe {
    pub sqe_id: u64,
    pub opcode: AioV4Op,
    pub fd: i32,
    pub offset: u64,
    pub length: u32,
    pub flags: u32,
    pub user_data: u64,
    pub buf_index: Option<u32>,
    pub ioprio: u16,
    pub link_next: bool,
}

impl AioV4Sqe {
    pub fn new(sqe_id: u64, opcode: AioV4Op, fd: i32) -> Self {
        Self {
            sqe_id,
            opcode,
            fd,
            offset: 0,
            length: 0,
            flags: 0,
            user_data: 0,
            buf_index: None,
            ioprio: 0,
            link_next: false,
        }
    }
}

/// Completion queue entry.
#[derive(Debug, Clone)]
pub struct AioV4Cqe {
    pub user_data: u64,
    pub result: i64,
    pub flags: u32,
    pub state: AioV4CqeState,
}

impl AioV4Cqe {
    #[inline]
    pub fn success(user_data: u64, result: i64) -> Self {
        Self {
            user_data,
            result,
            flags: 0,
            state: AioV4CqeState::Completed,
        }
    }

    #[inline]
    pub fn error(user_data: u64, errno: i64) -> Self {
        Self {
            user_data,
            result: -errno,
            flags: 0,
            state: AioV4CqeState::Error,
        }
    }
}

/// Ring buffer descriptor.
#[derive(Debug, Clone)]
pub struct AioV4Ring {
    pub ring_id: u64,
    pub sq_entries: u32,
    pub cq_entries: u32,
    pub sq_head: u32,
    pub sq_tail: u32,
    pub cq_head: u32,
    pub cq_tail: u32,
    pub sq_dropped: u64,
    pub cq_overflow: u64,
    pub sq_pending: Vec<AioV4Sqe>,
    pub cq_ready: Vec<AioV4Cqe>,
}

impl AioV4Ring {
    pub fn new(ring_id: u64, sq_size: u32, cq_size: u32) -> Self {
        Self {
            ring_id,
            sq_entries: sq_size,
            cq_entries: cq_size,
            sq_head: 0,
            sq_tail: 0,
            cq_head: 0,
            cq_tail: 0,
            sq_dropped: 0,
            cq_overflow: 0,
            sq_pending: Vec::new(),
            cq_ready: Vec::new(),
        }
    }

    #[inline]
    pub fn submit(&mut self, sqe: AioV4Sqe) -> bool {
        if self.sq_pending.len() as u32 >= self.sq_entries {
            self.sq_dropped += 1;
            return false;
        }
        self.sq_pending.push(sqe);
        self.sq_tail = self.sq_tail.wrapping_add(1);
        true
    }

    #[inline]
    pub fn complete(&mut self, cqe: AioV4Cqe) -> bool {
        if self.cq_ready.len() as u32 >= self.cq_entries {
            self.cq_overflow += 1;
            return false;
        }
        self.cq_ready.push(cqe);
        self.cq_tail = self.cq_tail.wrapping_add(1);
        true
    }

    #[inline]
    pub fn harvest_completions(&mut self) -> Vec<AioV4Cqe> {
        let ready = core::mem::take(&mut self.cq_ready);
        self.cq_head = self.cq_tail;
        ready
    }

    #[inline(always)]
    pub fn sq_pending_count(&self) -> usize {
        self.sq_pending.len()
    }

    #[inline(always)]
    pub fn cq_ready_count(&self) -> usize {
        self.cq_ready.len()
    }
}

/// Fixed buffer registration.
#[derive(Debug, Clone)]
pub struct AioV4FixedBuf {
    pub index: u32,
    pub addr: u64,
    pub length: u64,
    pub ref_count: u32,
}

/// Statistics for AIO V4 bridge.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AioV4BridgeStats {
    pub total_rings: u64,
    pub total_sqes_submitted: u64,
    pub total_cqes_completed: u64,
    pub total_errors: u64,
    pub sq_overflow_count: u64,
    pub cq_overflow_count: u64,
    pub fixed_bufs_registered: u64,
    pub batched_submissions: u64,
}

/// Main bridge AIO V4 manager.
#[repr(align(64))]
pub struct BridgeAioV4 {
    pub rings: BTreeMap<u64, AioV4Ring>,
    pub fixed_bufs: BTreeMap<u32, AioV4FixedBuf>,
    pub next_ring_id: u64,
    pub next_sqe_id: u64,
    pub stats: AioV4BridgeStats,
}

impl BridgeAioV4 {
    pub fn new() -> Self {
        Self {
            rings: BTreeMap::new(),
            fixed_bufs: BTreeMap::new(),
            next_ring_id: 1,
            next_sqe_id: 1,
            stats: AioV4BridgeStats {
                total_rings: 0,
                total_sqes_submitted: 0,
                total_cqes_completed: 0,
                total_errors: 0,
                sq_overflow_count: 0,
                cq_overflow_count: 0,
                fixed_bufs_registered: 0,
                batched_submissions: 0,
            },
        }
    }

    #[inline]
    pub fn create_ring(&mut self, sq_size: u32, cq_size: u32) -> u64 {
        let id = self.next_ring_id;
        self.next_ring_id += 1;
        let ring = AioV4Ring::new(id, sq_size, cq_size);
        self.rings.insert(id, ring);
        self.stats.total_rings += 1;
        id
    }

    pub fn submit_to_ring(&mut self, ring_id: u64, opcode: AioV4Op, fd: i32) -> Option<u64> {
        let sqe_id = self.next_sqe_id;
        self.next_sqe_id += 1;
        let sqe = AioV4Sqe::new(sqe_id, opcode, fd);
        if let Some(ring) = self.rings.get_mut(&ring_id) {
            if ring.submit(sqe) {
                self.stats.total_sqes_submitted += 1;
                return Some(sqe_id);
            }
        }
        None
    }

    #[inline]
    pub fn register_fixed_buf(&mut self, index: u32, addr: u64, length: u64) {
        let buf = AioV4FixedBuf {
            index,
            addr,
            length,
            ref_count: 0,
        };
        self.fixed_bufs.insert(index, buf);
        self.stats.fixed_bufs_registered += 1;
    }

    #[inline(always)]
    pub fn ring_count(&self) -> usize {
        self.rings.len()
    }
}

// ============================================================================
// Merged from aio_v5_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AioV5OpType {
    Read,
    Write,
    Fsync,
    FDsync,
    Nop,
    Poll,
    Readv,
    Writev,
    Fadvise,
    Allocate,
}

/// AIO v5 state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AioV5State {
    Queued,
    Submitted,
    InProgress,
    Completed,
    Cancelled,
    Error,
    Retrying,
}

/// AIO v5 priority
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AioV5Priority {
    Background,
    Idle,
    Low,
    Normal,
    High,
    Realtime,
    Deadline,
}

/// AIO v5 control block
#[derive(Debug, Clone)]
pub struct AioV5Cb {
    pub cb_id: u64,
    pub op: AioV5OpType,
    pub state: AioV5State,
    pub priority: AioV5Priority,
    pub fd: i32,
    pub offset: u64,
    pub len: u64,
    pub result: i64,
    pub submit_ns: u64,
    pub complete_ns: u64,
    pub retries: u32,
    pub max_retries: u32,
    pub deadline_ns: u64,
}

impl AioV5Cb {
    pub fn new(cb_id: u64, op: AioV5OpType, fd: i32, offset: u64, len: u64) -> Self {
        Self {
            cb_id,
            op,
            state: AioV5State::Queued,
            priority: AioV5Priority::Normal,
            fd,
            offset,
            len,
            result: 0,
            submit_ns: 0,
            complete_ns: 0,
            retries: 0,
            max_retries: 3,
            deadline_ns: 0,
        }
    }

    #[inline(always)]
    pub fn submit(&mut self, ts_ns: u64) {
        self.state = AioV5State::Submitted;
        self.submit_ns = ts_ns;
    }

    #[inline]
    pub fn complete(&mut self, result: i64, ts_ns: u64) {
        self.state = AioV5State::Completed;
        self.result = result;
        self.complete_ns = ts_ns;
    }

    #[inline]
    pub fn retry(&mut self) -> bool {
        if self.retries < self.max_retries {
            self.retries += 1;
            self.state = AioV5State::Retrying;
            true
        } else {
            self.state = AioV5State::Error;
            false
        }
    }

    #[inline(always)]
    pub fn latency_ns(&self) -> u64 {
        self.complete_ns.saturating_sub(self.submit_ns)
    }

    #[inline(always)]
    pub fn missed_deadline(&self) -> bool {
        self.deadline_ns > 0 && self.complete_ns > self.deadline_ns
    }
}

/// AIO v5 completion ring
#[derive(Debug, Clone)]
pub struct AioV5Ring {
    pub ring_id: u32,
    pub capacity: u32,
    pub head: u32,
    pub tail: u32,
    pub completed: u64,
    pub overflows: u64,
    pub batch_completions: u64,
}

impl AioV5Ring {
    pub fn new(ring_id: u32, capacity: u32) -> Self {
        Self { ring_id, capacity, head: 0, tail: 0, completed: 0, overflows: 0, batch_completions: 0 }
    }

    #[inline]
    pub fn push(&mut self) -> bool {
        let next = (self.tail + 1) % self.capacity;
        if next == self.head { self.overflows += 1; return false; }
        self.tail = next;
        self.completed += 1;
        true
    }

    #[inline]
    pub fn pop(&mut self) -> bool {
        if self.head == self.tail { return false; }
        self.head = (self.head + 1) % self.capacity;
        true
    }

    #[inline(always)]
    pub fn pending(&self) -> u32 {
        if self.tail >= self.head { self.tail - self.head } else { self.capacity - self.head + self.tail }
    }
}

/// AIO v5 context
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AioV5Context {
    pub ctx_id: u32,
    pub max_events: u32,
    pub pending: u32,
    pub ring: AioV5Ring,
    pub completed: u64,
    pub total_bytes: u64,
    pub total_latency_ns: u64,
    pub deadline_misses: u64,
}

impl AioV5Context {
    pub fn new(ctx_id: u32, max_events: u32) -> Self {
        Self {
            ctx_id,
            max_events,
            pending: 0,
            ring: AioV5Ring::new(ctx_id, max_events),
            completed: 0,
            total_bytes: 0,
            total_latency_ns: 0,
            deadline_misses: 0,
        }
    }

    #[inline]
    pub fn submit(&mut self) -> bool {
        if self.pending >= self.max_events { return false; }
        self.pending += 1;
        true
    }

    #[inline]
    pub fn record_complete(&mut self, bytes: u64, latency_ns: u64, missed_deadline: bool) {
        self.pending = self.pending.saturating_sub(1);
        self.completed += 1;
        self.total_bytes += bytes;
        self.total_latency_ns += latency_ns;
        if missed_deadline { self.deadline_misses += 1; }
        self.ring.push();
    }

    #[inline(always)]
    pub fn avg_latency_ns(&self) -> u64 {
        if self.completed == 0 { 0 } else { self.total_latency_ns / self.completed }
    }

    #[inline(always)]
    pub fn deadline_miss_rate(&self) -> f64 {
        if self.completed == 0 { 0.0 } else { self.deadline_misses as f64 / self.completed as f64 }
    }
}

/// AIO v5 bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AioV5BridgeStats {
    pub total_contexts: u64,
    pub total_submitted: u64,
    pub total_completed: u64,
    pub total_bytes: u64,
    pub total_retries: u64,
}

/// Main bridge AIO v5
#[derive(Debug)]
#[repr(align(64))]
pub struct BridgeAioV5 {
    pub contexts: BTreeMap<u32, AioV5Context>,
    pub stats: AioV5BridgeStats,
    pub next_ctx_id: u32,
}

impl BridgeAioV5 {
    pub fn new() -> Self {
        Self {
            contexts: BTreeMap::new(),
            stats: AioV5BridgeStats {
                total_contexts: 0,
                total_submitted: 0,
                total_completed: 0,
                total_bytes: 0,
                total_retries: 0,
            },
            next_ctx_id: 1,
        }
    }

    #[inline]
    pub fn create_context(&mut self, max_events: u32) -> u32 {
        let id = self.next_ctx_id;
        self.next_ctx_id += 1;
        self.contexts.insert(id, AioV5Context::new(id, max_events));
        self.stats.total_contexts += 1;
        id
    }

    pub fn submit_io(&mut self, ctx_id: u32) -> bool {
        if let Some(ctx) = self.contexts.get_mut(&ctx_id) {
            if ctx.submit() {
                self.stats.total_submitted += 1;
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}
