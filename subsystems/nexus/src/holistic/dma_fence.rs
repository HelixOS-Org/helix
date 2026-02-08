// SPDX-License-Identifier: GPL-2.0
//! Holistic dma_fence — DMA fence synchronization primitives.

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
    SyncFile,
    Timeline,
    Binary,
    Chain,
}

/// DMA fence
#[derive(Debug, Clone)]
pub struct DmaFence {
    pub id: u64,
    pub context: u64,
    pub seqno: u64,
    pub fence_type: FenceType,
    pub state: FenceState,
    pub created_at: u64,
    pub signaled_at: u64,
    pub deadline_ns: u64,
    pub waiters: Vec<u64>,
    pub error_code: i32,
}

impl DmaFence {
    pub fn new(id: u64, context: u64, seqno: u64, ftype: FenceType, now: u64) -> Self {
        Self {
            id, context, seqno, fence_type: ftype, state: FenceState::Unsignaled,
            created_at: now, signaled_at: 0, deadline_ns: 0, waiters: Vec::new(),
            error_code: 0,
        }
    }

    pub fn signal(&mut self, now: u64) {
        self.state = FenceState::Signaled;
        self.signaled_at = now;
    }

    pub fn error(&mut self, code: i32, now: u64) {
        self.state = FenceState::Error;
        self.error_code = code;
        self.signaled_at = now;
    }

    pub fn add_waiter(&mut self, tid: u64) { self.waiters.push(tid); }

    pub fn is_signaled(&self) -> bool { self.state == FenceState::Signaled }

    pub fn latency_ns(&self) -> u64 {
        if self.signaled_at > 0 { self.signaled_at - self.created_at } else { 0 }
    }

    pub fn check_timeout(&mut self, now: u64) -> bool {
        if self.deadline_ns > 0 && now > self.deadline_ns && self.state == FenceState::Unsignaled {
            self.state = FenceState::TimedOut;
            self.signaled_at = now;
            true
        } else { false }
    }
}

/// Timeline fence (multi-point)
#[derive(Debug)]
pub struct TimelineFence {
    pub id: u64,
    pub context: u64,
    pub current_value: u64,
    pub fences: Vec<DmaFence>,
}

impl TimelineFence {
    pub fn new(id: u64, context: u64) -> Self {
        Self { id, context, current_value: 0, fences: Vec::new() }
    }

    pub fn advance(&mut self, value: u64, now: u64) {
        self.current_value = value;
        for f in &mut self.fences {
            if f.seqno <= value && f.state == FenceState::Unsignaled {
                f.signal(now);
            }
        }
    }

    pub fn add_point(&mut self, seqno: u64, now: u64) -> u64 {
        let fence_id = self.fences.len() as u64;
        self.fences.push(DmaFence::new(fence_id, self.context, seqno, FenceType::Timeline, now));
        fence_id
    }
}

/// Sync file (container of fences)
#[derive(Debug)]
pub struct SyncFile {
    pub id: u64,
    pub fence_ids: Vec<u64>,
    pub name_hash: u64,
}

/// Stats
#[derive(Debug, Clone)]
pub struct DmaFenceStats {
    pub total_fences: u32,
    pub signaled: u32,
    pub unsignaled: u32,
    pub errors: u32,
    pub total_timelines: u32,
    pub avg_latency_ns: u64,
    pub total_waiters: u32,
}

/// Main DMA fence manager
pub struct HolisticDmaFence {
    fences: BTreeMap<u64, DmaFence>,
    timelines: BTreeMap<u64, TimelineFence>,
    sync_files: BTreeMap<u64, SyncFile>,
    next_id: u64,
}

impl HolisticDmaFence {
    pub fn new() -> Self {
        Self { fences: BTreeMap::new(), timelines: BTreeMap::new(), sync_files: BTreeMap::new(), next_id: 1 }
    }

    pub fn create_fence(&mut self, context: u64, seqno: u64, ftype: FenceType, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.fences.insert(id, DmaFence::new(id, context, seqno, ftype, now));
        id
    }

    pub fn signal_fence(&mut self, id: u64, now: u64) {
        if let Some(f) = self.fences.get_mut(&id) { f.signal(now); }
    }

    pub fn create_timeline(&mut self, context: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.timelines.insert(id, TimelineFence::new(id, context));
        id
    }

    pub fn advance_timeline(&mut self, id: u64, value: u64, now: u64) {
        if let Some(t) = self.timelines.get_mut(&id) { t.advance(value, now); }
    }

    pub fn stats(&self) -> DmaFenceStats {
        let signaled = self.fences.values().filter(|f| f.state == FenceState::Signaled).count() as u32;
        let unsignaled = self.fences.values().filter(|f| f.state == FenceState::Unsignaled).count() as u32;
        let errors = self.fences.values().filter(|f| f.state == FenceState::Error).count() as u32;
        let lats: Vec<u64> = self.fences.values().filter(|f| f.is_signaled()).map(|f| f.latency_ns()).collect();
        let avg = if lats.is_empty() { 0 } else { lats.iter().sum::<u64>() / lats.len() as u64 };
        let waiters: u32 = self.fences.values().map(|f| f.waiters.len() as u32).sum();
        DmaFenceStats {
            total_fences: self.fences.len() as u32, signaled, unsignaled, errors,
            total_timelines: self.timelines.len() as u32, avg_latency_ns: avg, total_waiters: waiters,
        }
    }
}
