//! # Bridge io_uring Bridging
//!
//! io_uring submission/completion ring bridge:
//! - SQ/CQ ring management per process
//! - SQE submission batching and ordering
//! - CQE completion delivery and overflow tracking
//! - Op-code profiling per ring
//! - Linked request chain tracking
//! - Fixed buffer / registered FD management

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// io_uring operation codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoUringOp {
    Nop,
    Readv,
    Writev,
    Fsync,
    ReadFixed,
    WriteFixed,
    PollAdd,
    PollRemove,
    SendMsg,
    RecvMsg,
    Timeout,
    Accept,
    Connect,
    Close,
    Fallocate,
    Openat,
    Statx,
    Read,
    Write,
    Splice,
    ProvideBuffers,
    RemoveBuffers,
    Shutdown,
    Renameat,
    Unlinkat,
    Mkdirat,
    Socket,
    SendZc,
}

/// SQE flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SqeFlags {
    pub fixed_file: bool,
    pub io_drain: bool,
    pub io_link: bool,
    pub io_hardlink: bool,
    pub async_op: bool,
    pub buffer_select: bool,
    pub cqe_skip_success: bool,
}

impl SqeFlags {
    pub fn empty() -> Self {
        Self {
            fixed_file: false, io_drain: false, io_link: false,
            io_hardlink: false, async_op: false, buffer_select: false,
            cqe_skip_success: false,
        }
    }
}

/// Submission Queue Entry
#[derive(Debug, Clone)]
pub struct BridgeSqe {
    pub opcode: IoUringOp,
    pub flags: SqeFlags,
    pub fd: i32,
    pub offset: u64,
    pub addr: u64,
    pub len: u32,
    pub user_data: u64,
    pub buf_group: u16,
    pub personality: u16,
    pub submit_ns: u64,
}

/// Completion Queue Entry
#[derive(Debug, Clone)]
pub struct BridgeCqe {
    pub user_data: u64,
    pub result: i32,
    pub flags: u32,
    pub complete_ns: u64,
    pub latency_ns: u64,
}

/// Per-opcode statistics
#[derive(Debug, Clone, Default)]
pub struct OpStats {
    pub count: u64,
    pub total_latency_ns: u64,
    pub max_latency_ns: u64,
    pub errors: u64,
    pub bytes_transferred: u64,
}

impl OpStats {
    pub fn avg_latency_ns(&self) -> u64 {
        if self.count == 0 { return 0; }
        self.total_latency_ns / self.count
    }

    pub fn record(&mut self, latency: u64, result: i32, bytes: u64) {
        self.count += 1;
        self.total_latency_ns += latency;
        if latency > self.max_latency_ns { self.max_latency_ns = latency; }
        if result < 0 { self.errors += 1; }
        self.bytes_transferred += bytes;
    }
}

/// Registered file descriptor table
#[derive(Debug, Clone)]
pub struct RegisteredFdTable {
    pub fds: Vec<Option<i32>>,
    pub capacity: u32,
    pub used: u32,
}

impl RegisteredFdTable {
    pub fn new(cap: u32) -> Self {
        let mut fds = Vec::with_capacity(cap as usize);
        for _ in 0..cap { fds.push(None); }
        Self { fds, capacity: cap, used: 0 }
    }

    pub fn register(&mut self, slot: u32, fd: i32) -> bool {
        if slot >= self.capacity { return false; }
        let was_none = self.fds[slot as usize].is_none();
        self.fds[slot as usize] = Some(fd);
        if was_none { self.used += 1; }
        true
    }

    pub fn unregister(&mut self, slot: u32) -> bool {
        if slot >= self.capacity { return false; }
        if self.fds[slot as usize].take().is_some() {
            self.used -= 1;
            true
        } else { false }
    }

    pub fn resolve(&self, slot: u32) -> Option<i32> {
        self.fds.get(slot as usize).and_then(|&v| v)
    }
}

/// Fixed buffer registration
#[derive(Debug, Clone)]
pub struct FixedBuffer {
    pub addr: u64,
    pub len: u64,
    pub registered: bool,
}

/// io_uring ring instance
#[derive(Debug, Clone)]
pub struct IoUringInstance {
    pub ring_fd: i32,
    pub owner_pid: u64,
    pub sq_size: u32,
    pub cq_size: u32,
    pub sq_pending: Vec<BridgeSqe>,
    pub cq_ready: Vec<BridgeCqe>,
    pub cq_overflow: u64,
    pub op_stats: BTreeMap<u8, OpStats>,
    pub registered_fds: Option<RegisteredFdTable>,
    pub fixed_buffers: Vec<FixedBuffer>,
    pub total_submitted: u64,
    pub total_completed: u64,
    pub created_ns: u64,
}

impl IoUringInstance {
    pub fn new(ring_fd: i32, pid: u64, sq_sz: u32, cq_sz: u32, ts: u64) -> Self {
        Self {
            ring_fd,
            owner_pid: pid,
            sq_size: sq_sz,
            cq_size: cq_sz,
            sq_pending: Vec::new(),
            cq_ready: Vec::new(),
            cq_overflow: 0,
            op_stats: BTreeMap::new(),
            registered_fds: None,
            fixed_buffers: Vec::new(),
            total_submitted: 0,
            total_completed: 0,
            created_ns: ts,
        }
    }

    pub fn submit(&mut self, sqe: BridgeSqe) -> bool {
        if self.sq_pending.len() >= self.sq_size as usize { return false; }
        self.sq_pending.push(sqe);
        self.total_submitted += 1;
        true
    }

    pub fn complete(&mut self, user_data: u64, result: i32, flags: u32, now: u64) {
        // find submission to calculate latency
        let submit_ns = self.sq_pending.iter()
            .find(|s| s.user_data == user_data)
            .map(|s| s.submit_ns)
            .unwrap_or(now);
        let latency = now.saturating_sub(submit_ns);

        // remove from pending
        if let Some(idx) = self.sq_pending.iter().position(|s| s.user_data == user_data) {
            let sqe = self.sq_pending.remove(idx);
            let op_key = sqe.opcode as u8;
            self.op_stats.entry(op_key)
                .or_insert_with(OpStats::default)
                .record(latency, result, 0);
        }

        let cqe = BridgeCqe { user_data, result, flags, complete_ns: now, latency_ns: latency };
        if self.cq_ready.len() >= self.cq_size as usize {
            self.cq_overflow += 1;
        } else {
            self.cq_ready.push(cqe);
        }
        self.total_completed += 1;
    }

    pub fn reap(&mut self, max: usize) -> Vec<BridgeCqe> {
        let take = max.min(self.cq_ready.len());
        self.cq_ready.drain(..take).collect()
    }

    pub fn register_fds(&mut self, capacity: u32) {
        self.registered_fds = Some(RegisteredFdTable::new(capacity));
    }

    pub fn add_fixed_buffer(&mut self, addr: u64, len: u64) {
        self.fixed_buffers.push(FixedBuffer { addr, len, registered: true });
    }

    pub fn inflight(&self) -> usize { self.sq_pending.len() }
    pub fn completions_available(&self) -> usize { self.cq_ready.len() }
}

/// Global io_uring bridge stats
#[derive(Debug, Clone, Default)]
pub struct BridgeIoUringStats {
    pub total_rings: usize,
    pub total_submitted: u64,
    pub total_completed: u64,
    pub total_overflow: u64,
    pub total_inflight: usize,
}

/// Bridge io_uring Manager
pub struct BridgeIoUringBridge {
    rings: BTreeMap<i32, IoUringInstance>,
    stats: BridgeIoUringStats,
    next_ring_fd: i32,
}

impl BridgeIoUringBridge {
    pub fn new() -> Self {
        Self {
            rings: BTreeMap::new(),
            stats: BridgeIoUringStats::default(),
            next_ring_fd: 2000,
        }
    }

    pub fn setup_ring(&mut self, pid: u64, sq_sz: u32, cq_sz: u32, ts: u64) -> i32 {
        let fd = self.next_ring_fd;
        self.next_ring_fd += 1;
        self.rings.insert(fd, IoUringInstance::new(fd, pid, sq_sz, cq_sz, ts));
        fd
    }

    pub fn submit_sqe(&mut self, ring_fd: i32, sqe: BridgeSqe) -> bool {
        self.rings.get_mut(&ring_fd).map(|r| r.submit(sqe)).unwrap_or(false)
    }

    pub fn complete_sqe(&mut self, ring_fd: i32, user_data: u64, result: i32, flags: u32, now: u64) {
        if let Some(ring) = self.rings.get_mut(&ring_fd) {
            ring.complete(user_data, result, flags, now);
        }
    }

    pub fn reap_cqes(&mut self, ring_fd: i32, max: usize) -> Vec<BridgeCqe> {
        self.rings.get_mut(&ring_fd).map(|r| r.reap(max)).unwrap_or_default()
    }

    pub fn destroy_ring(&mut self, ring_fd: i32) -> bool {
        self.rings.remove(&ring_fd).is_some()
    }

    pub fn recompute(&mut self) {
        self.stats.total_rings = self.rings.len();
        self.stats.total_submitted = self.rings.values().map(|r| r.total_submitted).sum();
        self.stats.total_completed = self.rings.values().map(|r| r.total_completed).sum();
        self.stats.total_overflow = self.rings.values().map(|r| r.cq_overflow).sum();
        self.stats.total_inflight = self.rings.values().map(|r| r.inflight()).sum();
    }

    pub fn ring(&self, fd: i32) -> Option<&IoUringInstance> { self.rings.get(&fd) }
    pub fn stats(&self) -> &BridgeIoUringStats { &self.stats }
}

// ============================================================================
// Merged from io_uring_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoUringV2Op {
    Nop,
    Readv,
    Writev,
    Fsync,
    ReadFixed,
    WriteFixed,
    PollAdd,
    PollRemove,
    Accept,
    AcceptMultishot,
    Recv,
    RecvMultishot,
    Send,
    SendZc,
    Openat,
    Close,
    Statx,
    Splice,
    ProvideBuffers,
    RemoveBuffers,
    Cancel,
    LinkTimeout,
    WaitId,
    Futex,
    Socket,
}

/// Completion flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoUringV2CqeFlag {
    Buffer,
    More,
    Notif,
    SockNonempty,
}

/// Provided buffer group.
#[derive(Debug, Clone)]
pub struct IoUringV2BufGroup {
    pub group_id: u16,
    pub buf_count: u32,
    pub buf_size: u32,
    pub consumed: u32,
    pub replenished: u32,
}

impl IoUringV2BufGroup {
    pub fn new(group_id: u16, buf_count: u32, buf_size: u32) -> Self {
        Self {
            group_id,
            buf_count,
            buf_size,
            consumed: 0,
            replenished: 0,
        }
    }

    pub fn consume(&mut self) -> bool {
        if self.consumed < self.buf_count {
            self.consumed += 1;
            true
        } else {
            false
        }
    }

    pub fn replenish(&mut self, count: u32) {
        self.consumed = self.consumed.saturating_sub(count);
        self.replenished += count;
    }

    pub fn available(&self) -> u32 {
        self.buf_count.saturating_sub(self.consumed)
    }
}

/// A registered file descriptor.
#[derive(Debug, Clone)]
pub struct IoUringV2RegisteredFd {
    pub slot: u32,
    pub fd: i32,
    pub is_direct: bool,
    pub ref_count: u32,
}

/// An io_uring V2 ring instance.
#[derive(Debug, Clone)]
pub struct IoUringV2Ring {
    pub ring_id: u64,
    pub sq_size: u32,
    pub cq_size: u32,
    pub sq_entries_used: u32,
    pub cq_entries_ready: u32,
    pub sq_dropped: u64,
    pub cq_overflow: u64,
    pub multishot_active: u32,
    pub registered_fds: Vec<IoUringV2RegisteredFd>,
    pub buf_groups: BTreeMap<u16, IoUringV2BufGroup>,
    pub total_submitted: u64,
    pub total_completed: u64,
}

impl IoUringV2Ring {
    pub fn new(ring_id: u64, sq_size: u32, cq_size: u32) -> Self {
        Self {
            ring_id,
            sq_size,
            cq_size,
            sq_entries_used: 0,
            cq_entries_ready: 0,
            sq_dropped: 0,
            cq_overflow: 0,
            multishot_active: 0,
            registered_fds: Vec::new(),
            buf_groups: BTreeMap::new(),
            total_submitted: 0,
            total_completed: 0,
        }
    }

    pub fn submit_op(&mut self, op: IoUringV2Op) -> bool {
        if self.sq_entries_used >= self.sq_size {
            self.sq_dropped += 1;
            return false;
        }
        self.sq_entries_used += 1;
        self.total_submitted += 1;
        if matches!(
            op,
            IoUringV2Op::AcceptMultishot | IoUringV2Op::RecvMultishot
        ) {
            self.multishot_active += 1;
        }
        true
    }

    pub fn complete_op(&mut self) {
        if self.sq_entries_used > 0 {
            self.sq_entries_used -= 1;
        }
        self.total_completed += 1;
    }

    pub fn register_fd(&mut self, slot: u32, fd: i32, direct: bool) {
        self.registered_fds.push(IoUringV2RegisteredFd {
            slot,
            fd,
            is_direct: direct,
            ref_count: 1,
        });
    }

    pub fn add_buf_group(&mut self, group_id: u16, count: u32, size: u32) {
        let group = IoUringV2BufGroup::new(group_id, count, size);
        self.buf_groups.insert(group_id, group);
    }
}

/// Statistics for io_uring V2 bridge.
#[derive(Debug, Clone)]
pub struct IoUringV2BridgeStats {
    pub total_rings: u64,
    pub total_submitted: u64,
    pub total_completed: u64,
    pub total_sq_overflow: u64,
    pub total_cq_overflow: u64,
    pub multishot_ops: u64,
    pub registered_fds_total: u64,
    pub buf_groups_total: u64,
    pub zero_copy_sends: u64,
}

/// Main bridge io_uring V2 manager.
pub struct BridgeIoUringV2 {
    pub rings: BTreeMap<u64, IoUringV2Ring>,
    pub next_ring_id: u64,
    pub stats: IoUringV2BridgeStats,
}

impl BridgeIoUringV2 {
    pub fn new() -> Self {
        Self {
            rings: BTreeMap::new(),
            next_ring_id: 1,
            stats: IoUringV2BridgeStats {
                total_rings: 0,
                total_submitted: 0,
                total_completed: 0,
                total_sq_overflow: 0,
                total_cq_overflow: 0,
                multishot_ops: 0,
                registered_fds_total: 0,
                buf_groups_total: 0,
                zero_copy_sends: 0,
            },
        }
    }

    pub fn create_ring(&mut self, sq_size: u32, cq_size: u32) -> u64 {
        let id = self.next_ring_id;
        self.next_ring_id += 1;
        let ring = IoUringV2Ring::new(id, sq_size, cq_size);
        self.rings.insert(id, ring);
        self.stats.total_rings += 1;
        id
    }

    pub fn submit(&mut self, ring_id: u64, op: IoUringV2Op) -> bool {
        if let Some(ring) = self.rings.get_mut(&ring_id) {
            let ok = ring.submit_op(op);
            if ok {
                self.stats.total_submitted += 1;
                if matches!(
                    op,
                    IoUringV2Op::AcceptMultishot | IoUringV2Op::RecvMultishot
                ) {
                    self.stats.multishot_ops += 1;
                }
                if op == IoUringV2Op::SendZc {
                    self.stats.zero_copy_sends += 1;
                }
            }
            ok
        } else {
            false
        }
    }

    pub fn ring_count(&self) -> usize {
        self.rings.len()
    }
}

// ============================================================================
// Merged from io_uring_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoUringV3Op {
    Read,
    Write,
    Readv,
    Writev,
    Fsync,
    PollAdd,
    PollRemove,
    SendMsg,
    RecvMsg,
    Accept,
    Connect,
    Close,
    Splice,
    Provide,
    Cancel,
    Timeout,
    LinkTimeout,
    Statx,
    Fallocate,
    OpenAt,
    MkdirAt,
    Shutdown,
    Socket,
    SendZc,
    WaitId,
}

/// io_uring v3 feature flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoUringV3Feature {
    SingleIssuer,
    RegisteredFds,
    RegisteredBuffers,
    SubmitAll,
    CoopTaskrun,
    TaskrunFlag,
    SqeGroup,
    DeferTaskrun,
    BufRing,
    MultiShot,
}

/// Submission queue entry
#[derive(Debug, Clone)]
pub struct SqeEntry {
    pub op: IoUringV3Op,
    pub flags: u32,
    pub fd: i32,
    pub offset: u64,
    pub len: u32,
    pub buf_group: u16,
    pub personality: u16,
    pub user_data: u64,
    pub linked: bool,
    pub multi_shot: bool,
}

impl SqeEntry {
    pub fn new(op: IoUringV3Op, fd: i32, len: u32) -> Self {
        Self {
            op,
            flags: 0,
            fd,
            offset: 0,
            len,
            buf_group: 0,
            personality: 0,
            user_data: 0,
            linked: false,
            multi_shot: false,
        }
    }
}

/// Completion queue entry
#[derive(Debug, Clone)]
pub struct CqeEntry {
    pub user_data: u64,
    pub result: i32,
    pub flags: u32,
    pub extra1: u64,
    pub extra2: u64,
}

/// Registered buffer pool
#[derive(Debug, Clone)]
pub struct RegisteredBufPool {
    pub pool_id: u16,
    pub buf_count: u32,
    pub buf_size: u32,
    pub allocated: u32,
    pub recycled: u64,
}

impl RegisteredBufPool {
    pub fn new(pool_id: u16, buf_count: u32, buf_size: u32) -> Self {
        Self {
            pool_id,
            buf_count,
            buf_size,
            allocated: 0,
            recycled: 0,
        }
    }

    pub fn alloc(&mut self) -> bool {
        if self.allocated < self.buf_count {
            self.allocated += 1;
            true
        } else {
            false
        }
    }

    pub fn free(&mut self) {
        if self.allocated > 0 {
            self.allocated -= 1;
            self.recycled += 1;
        }
    }

    pub fn utilization_pct(&self) -> f64 {
        if self.buf_count == 0 { 0.0 } else { (self.allocated as f64 / self.buf_count as f64) * 100.0 }
    }

    pub fn total_memory(&self) -> u64 {
        self.buf_count as u64 * self.buf_size as u64
    }
}

/// io_uring ring instance
#[derive(Debug, Clone)]
pub struct IoUringV3Ring {
    pub ring_id: u32,
    pub sq_size: u32,
    pub cq_size: u32,
    pub sq_pending: u32,
    pub cq_pending: u32,
    pub features: u64,
    pub submissions: u64,
    pub completions: u64,
    pub overflow: u64,
    pub sq_drops: u64,
    pub buf_pools: Vec<RegisteredBufPool>,
    pub registered_fds: u32,
    pub op_counts: BTreeMap<u8, u64>,
}

impl IoUringV3Ring {
    pub fn new(ring_id: u32, sq_size: u32, cq_size: u32) -> Self {
        Self {
            ring_id,
            sq_size,
            cq_size,
            sq_pending: 0,
            cq_pending: 0,
            features: 0,
            submissions: 0,
            completions: 0,
            overflow: 0,
            sq_drops: 0,
            buf_pools: Vec::new(),
            registered_fds: 0,
            op_counts: BTreeMap::new(),
        }
    }

    pub fn submit(&mut self, sqe: &SqeEntry) -> bool {
        if self.sq_pending >= self.sq_size {
            self.sq_drops += 1;
            return false;
        }
        self.sq_pending += 1;
        self.submissions += 1;
        *self.op_counts.entry(sqe.op as u8).or_insert(0) += 1;
        true
    }

    pub fn complete(&mut self) -> bool {
        if self.sq_pending == 0 { return false; }
        self.sq_pending -= 1;
        self.cq_pending += 1;
        self.completions += 1;
        true
    }

    pub fn reap(&mut self) -> bool {
        if self.cq_pending > 0 { self.cq_pending -= 1; true } else { false }
    }

    pub fn enable_feature(&mut self, feature: IoUringV3Feature) {
        self.features |= 1u64 << (feature as u64);
    }

    pub fn sq_utilization_pct(&self) -> f64 {
        if self.sq_size == 0 { 0.0 } else { (self.sq_pending as f64 / self.sq_size as f64) * 100.0 }
    }
}

/// io_uring v3 bridge stats
#[derive(Debug, Clone)]
pub struct IoUringV3BridgeStats {
    pub total_rings: u64,
    pub total_submissions: u64,
    pub total_completions: u64,
    pub total_buf_pools: u64,
}

/// Main bridge io_uring v3
#[derive(Debug)]
pub struct BridgeIoUringV3 {
    pub rings: BTreeMap<u32, IoUringV3Ring>,
    pub stats: IoUringV3BridgeStats,
    pub next_ring_id: u32,
}

impl BridgeIoUringV3 {
    pub fn new() -> Self {
        Self {
            rings: BTreeMap::new(),
            stats: IoUringV3BridgeStats {
                total_rings: 0,
                total_submissions: 0,
                total_completions: 0,
                total_buf_pools: 0,
            },
            next_ring_id: 1,
        }
    }

    pub fn create_ring(&mut self, sq_size: u32, cq_size: u32) -> u32 {
        let id = self.next_ring_id;
        self.next_ring_id += 1;
        self.rings.insert(id, IoUringV3Ring::new(id, sq_size, cq_size));
        self.stats.total_rings += 1;
        id
    }

    pub fn submit_to_ring(&mut self, ring_id: u32, sqe: &SqeEntry) -> bool {
        if let Some(ring) = self.rings.get_mut(&ring_id) {
            if ring.submit(sqe) {
                self.stats.total_submissions += 1;
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}
