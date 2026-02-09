//! # Bridge DMA Fence
//!
//! DMA fence/sync object management for device synchronization:
//! - Fence creation and signaling
//! - Timeline fences (sync_file/dma_fence_chain)
//! - Wait/poll operations
//! - Fence dependency tracking
//! - Timeout and deadline management

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Fence state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenceState {
    Unsignaled,
    Signaled,
    Error,
    TimedOut,
}

/// Fence type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenceType {
    Single,
    Array,
    Chain,
    Timeline,
}

/// Fence descriptor
#[derive(Debug, Clone)]
pub struct DmaFence {
    pub id: u64,
    pub context: u64,
    pub seqno: u64,
    pub fence_type: FenceType,
    pub state: FenceState,
    pub create_ts: u64,
    pub signal_ts: u64,
    pub deadline_ts: u64,
    pub error_code: i32,
    pub callbacks: u32,
    pub deps: Vec<u64>,
    pub waiters: Vec<u64>,
}

impl DmaFence {
    pub fn new(id: u64, ctx: u64, seqno: u64, ftype: FenceType, ts: u64) -> Self {
        Self {
            id, context: ctx, seqno, fence_type: ftype,
            state: FenceState::Unsignaled, create_ts: ts, signal_ts: 0,
            deadline_ts: 0, error_code: 0, callbacks: 0,
            deps: Vec::new(), waiters: Vec::new(),
        }
    }

    #[inline(always)]
    pub fn signal(&mut self, ts: u64) { self.state = FenceState::Signaled; self.signal_ts = ts; }
    #[inline(always)]
    pub fn signal_error(&mut self, err: i32, ts: u64) { self.state = FenceState::Error; self.error_code = err; self.signal_ts = ts; }
    #[inline(always)]
    pub fn is_signaled(&self) -> bool { self.state == FenceState::Signaled || self.state == FenceState::Error }
    #[inline(always)]
    pub fn latency_ns(&self) -> u64 { if self.signal_ts > self.create_ts { self.signal_ts - self.create_ts } else { 0 } }
    #[inline(always)]
    pub fn add_dep(&mut self, dep: u64) { if !self.deps.contains(&dep) { self.deps.push(dep); } }
    #[inline(always)]
    pub fn add_waiter(&mut self, waiter: u64) { if !self.waiters.contains(&waiter) { self.waiters.push(waiter); } }
    #[inline(always)]
    pub fn set_deadline(&mut self, ts: u64) { self.deadline_ts = ts; }
    #[inline(always)]
    pub fn is_past_deadline(&self, now: u64) -> bool { self.deadline_ts > 0 && now > self.deadline_ts && !self.is_signaled() }
}

/// Fence context (per-device timeline)
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FenceContext {
    pub id: u64,
    pub last_seqno: u64,
    pub last_signaled_seqno: u64,
    pub pending_count: u32,
    pub total_fences: u64,
    pub total_signaled: u64,
    pub total_errors: u64,
    pub avg_latency_ns: u64,
}

impl FenceContext {
    pub fn new(id: u64) -> Self {
        Self { id, last_seqno: 0, last_signaled_seqno: 0, pending_count: 0, total_fences: 0, total_signaled: 0, total_errors: 0, avg_latency_ns: 0 }
    }

    #[inline(always)]
    pub fn next_seqno(&mut self) -> u64 { self.last_seqno += 1; self.last_seqno }
}

/// Sync file (collection of fences)
#[derive(Debug, Clone)]
pub struct SyncFile {
    pub id: u64,
    pub fence_ids: Vec<u64>,
    pub state: FenceState,
    pub create_ts: u64,
}

impl SyncFile {
    pub fn new(id: u64, fences: Vec<u64>, ts: u64) -> Self {
        Self { id, fence_ids: fences, state: FenceState::Unsignaled, create_ts: ts }
    }
}

/// Wait request
#[derive(Debug, Clone)]
pub struct FenceWaitReq {
    pub fence_id: u64,
    pub waiter_id: u64,
    pub timeout_ns: u64,
    pub start_ts: u64,
    pub interruptible: bool,
}

/// DMA fence stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct DmaFenceStats {
    pub total_fences: u64,
    pub pending_fences: usize,
    pub signaled_fences: u64,
    pub error_fences: u64,
    pub avg_latency_ns: u64,
    pub max_latency_ns: u64,
    pub total_waits: u64,
    pub deadline_misses: u64,
    pub contexts: usize,
}

/// Bridge DMA fence manager
#[repr(align(64))]
pub struct BridgeDmaFence {
    fences: BTreeMap<u64, DmaFence>,
    contexts: BTreeMap<u64, FenceContext>,
    sync_files: BTreeMap<u64, SyncFile>,
    waits: Vec<FenceWaitReq>,
    stats: DmaFenceStats,
    next_fence: u64,
    next_ctx: u64,
    next_sf: u64,
}

impl BridgeDmaFence {
    pub fn new() -> Self {
        Self {
            fences: BTreeMap::new(), contexts: BTreeMap::new(), sync_files: BTreeMap::new(),
            waits: Vec::new(), stats: DmaFenceStats::default(),
            next_fence: 1, next_ctx: 1, next_sf: 1,
        }
    }

    #[inline]
    pub fn create_context(&mut self) -> u64 {
        let id = self.next_ctx; self.next_ctx += 1;
        self.contexts.insert(id, FenceContext::new(id));
        id
    }

    #[inline]
    pub fn create_fence(&mut self, ctx: u64, ts: u64) -> u64 {
        let seqno = self.contexts.get_mut(&ctx).map(|c| c.next_seqno()).unwrap_or(0);
        let id = self.next_fence; self.next_fence += 1;
        let fence = DmaFence::new(id, ctx, seqno, FenceType::Single, ts);
        self.fences.insert(id, fence);
        if let Some(c) = self.contexts.get_mut(&ctx) { c.pending_count += 1; c.total_fences += 1; }
        id
    }

    #[inline]
    pub fn create_timeline_fence(&mut self, ctx: u64, seqno: u64, ts: u64) -> u64 {
        let id = self.next_fence; self.next_fence += 1;
        let fence = DmaFence::new(id, ctx, seqno, FenceType::Timeline, ts);
        self.fences.insert(id, fence);
        if let Some(c) = self.contexts.get_mut(&ctx) { c.pending_count += 1; c.total_fences += 1; }
        id
    }

    pub fn signal_fence(&mut self, id: u64, ts: u64) {
        if let Some(f) = self.fences.get_mut(&id) {
            let ctx = f.context;
            let seqno = f.seqno;
            f.signal(ts);
            if let Some(c) = self.contexts.get_mut(&ctx) {
                c.pending_count = c.pending_count.saturating_sub(1);
                c.total_signaled += 1;
                if seqno > c.last_signaled_seqno { c.last_signaled_seqno = seqno; }
                let lat = f.latency_ns();
                c.avg_latency_ns = (c.avg_latency_ns * (c.total_signaled - 1) + lat) / c.total_signaled;
            }
        }
        self.waits.retain(|w| w.fence_id != id);
    }

    #[inline]
    pub fn signal_error(&mut self, id: u64, err: i32, ts: u64) {
        if let Some(f) = self.fences.get_mut(&id) {
            let ctx = f.context;
            f.signal_error(err, ts);
            if let Some(c) = self.contexts.get_mut(&ctx) { c.pending_count = c.pending_count.saturating_sub(1); c.total_errors += 1; }
        }
    }

    #[inline(always)]
    pub fn add_dependency(&mut self, fence: u64, dep: u64) {
        if let Some(f) = self.fences.get_mut(&fence) { f.add_dep(dep); }
    }

    #[inline(always)]
    pub fn wait_fence(&mut self, fence_id: u64, waiter: u64, timeout: u64, ts: u64) {
        if let Some(f) = self.fences.get_mut(&fence_id) { f.add_waiter(waiter); }
        self.waits.push(FenceWaitReq { fence_id, waiter_id: waiter, timeout_ns: timeout, start_ts: ts, interruptible: true });
    }

    #[inline]
    pub fn create_sync_file(&mut self, fences: Vec<u64>, ts: u64) -> u64 {
        let id = self.next_sf; self.next_sf += 1;
        self.sync_files.insert(id, SyncFile::new(id, fences, ts));
        id
    }

    #[inline(always)]
    pub fn check_deadlines(&mut self, now: u64) -> Vec<u64> {
        self.fences.values().filter(|f| f.is_past_deadline(now)).map(|f| f.id).collect()
    }

    pub fn recompute(&mut self) {
        self.stats.total_fences = self.fences.len() as u64;
        self.stats.pending_fences = self.fences.values().filter(|f| !f.is_signaled()).count();
        self.stats.signaled_fences = self.fences.values().filter(|f| f.state == FenceState::Signaled).count() as u64;
        self.stats.error_fences = self.fences.values().filter(|f| f.state == FenceState::Error).count() as u64;
        let signaled: Vec<u64> = self.fences.values().filter(|f| f.is_signaled()).map(|f| f.latency_ns()).collect();
        if !signaled.is_empty() {
            self.stats.avg_latency_ns = signaled.iter().sum::<u64>() / signaled.len() as u64;
            self.stats.max_latency_ns = signaled.iter().copied().max().unwrap_or(0);
        }
        self.stats.total_waits = self.waits.len() as u64;
        self.stats.contexts = self.contexts.len();
    }

    #[inline(always)]
    pub fn fence(&self, id: u64) -> Option<&DmaFence> { self.fences.get(&id) }
    #[inline(always)]
    pub fn context(&self, id: u64) -> Option<&FenceContext> { self.contexts.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &DmaFenceStats { &self.stats }
}
