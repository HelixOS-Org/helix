//! # Coop Fence Protocol
//!
//! Memory fence and barrier cooperative protocol:
//! - Cooperative memory ordering
//! - Epoch-based reclamation coordination
//! - Read-copy-update (RCU) grace period management
//! - Quiescent state tracking
//! - Barrier synchronization with cooperative signaling

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Fence type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenceType {
    /// Read fence
    Read,
    /// Write fence
    Write,
    /// Full memory fence
    Full,
    /// Acquire semantics
    Acquire,
    /// Release semantics
    Release,
    /// Sequential consistency
    SeqCst,
}

/// Epoch state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpochState {
    /// Active in current epoch
    Active,
    /// In quiescent state (not accessing shared data)
    Quiescent,
    /// Pinned (must not advance epoch)
    Pinned,
    /// Offline (not participating)
    Offline,
}

/// Per-thread epoch info
#[derive(Debug, Clone)]
pub struct ThreadEpochInfo {
    pub tid: u64,
    pub current_epoch: u64,
    pub state: EpochState,
    pub pin_count: u32,
    pub quiescent_count: u64,
    pub last_quiescent_ns: u64,
    pub fence_count: u64,
}

impl ThreadEpochInfo {
    pub fn new(tid: u64) -> Self {
        Self {
            tid,
            current_epoch: 0,
            state: EpochState::Active,
            pin_count: 0,
            quiescent_count: 0,
            last_quiescent_ns: 0,
            fence_count: 0,
        }
    }

    pub fn enter_quiescent(&mut self, now_ns: u64) {
        if self.pin_count == 0 {
            self.state = EpochState::Quiescent;
            self.quiescent_count += 1;
            self.last_quiescent_ns = now_ns;
        }
    }

    pub fn exit_quiescent(&mut self) {
        self.state = EpochState::Active;
    }

    pub fn pin(&mut self) {
        self.pin_count += 1;
        self.state = EpochState::Pinned;
    }

    pub fn unpin(&mut self) {
        if self.pin_count > 0 {
            self.pin_count -= 1;
        }
        if self.pin_count == 0 {
            self.state = EpochState::Active;
        }
    }

    pub fn record_fence(&mut self) {
        self.fence_count += 1;
    }
}

/// RCU grace period
#[derive(Debug, Clone)]
pub struct GracePeriod {
    pub gp_id: u64,
    pub start_epoch: u64,
    pub end_epoch: u64,
    pub started_ns: u64,
    pub completed_ns: Option<u64>,
    /// Deferred callbacks count
    pub callbacks_pending: u32,
    pub callbacks_completed: u32,
}

impl GracePeriod {
    pub fn new(gp_id: u64, epoch: u64, now_ns: u64) -> Self {
        Self {
            gp_id,
            start_epoch: epoch,
            end_epoch: epoch + 1,
            started_ns: now_ns,
            completed_ns: None,
            callbacks_pending: 0,
            callbacks_completed: 0,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.completed_ns.is_some()
    }

    pub fn complete(&mut self, now_ns: u64) {
        self.completed_ns = Some(now_ns);
    }

    pub fn duration_ns(&self) -> u64 {
        match self.completed_ns {
            Some(end) => end - self.started_ns,
            None => 0,
        }
    }
}

/// Barrier group
#[derive(Debug)]
pub struct BarrierGroup {
    pub barrier_id: u64,
    pub expected_count: u32,
    pub arrived_count: u32,
    pub participants: Vec<u64>,
    pub arrived: Vec<u64>,
    pub created_ns: u64,
    pub completed_ns: Option<u64>,
    pub generation: u64,
}

impl BarrierGroup {
    pub fn new(id: u64, expected: u32, now_ns: u64) -> Self {
        Self {
            barrier_id: id,
            expected_count: expected,
            arrived_count: 0,
            participants: Vec::new(),
            arrived: Vec::new(),
            created_ns: now_ns,
            completed_ns: None,
            generation: 0,
        }
    }

    pub fn arrive(&mut self, tid: u64, now_ns: u64) -> bool {
        if !self.arrived.contains(&tid) {
            self.arrived.push(tid);
            self.arrived_count += 1;
        }
        if self.arrived_count >= self.expected_count {
            self.completed_ns = Some(now_ns);
            self.generation += 1;
            true
        } else {
            false
        }
    }

    pub fn reset(&mut self) {
        self.arrived.clear();
        self.arrived_count = 0;
        self.completed_ns = None;
    }

    pub fn is_complete(&self) -> bool {
        self.arrived_count >= self.expected_count
    }

    pub fn waiting_count(&self) -> u32 {
        self.expected_count - self.arrived_count
    }
}

/// Fence protocol stats
#[derive(Debug, Clone, Default)]
pub struct CoopFenceProtoStats {
    pub tracked_threads: usize,
    pub current_epoch: u64,
    pub active_grace_periods: usize,
    pub completed_grace_periods: u64,
    pub active_barriers: usize,
    pub total_fences: u64,
    pub avg_gp_duration_ns: f64,
}

/// Coop Fence Protocol
pub struct CoopFenceProtocol {
    threads: BTreeMap<u64, ThreadEpochInfo>,
    current_epoch: u64,
    grace_periods: Vec<GracePeriod>,
    barriers: BTreeMap<u64, BarrierGroup>,
    stats: CoopFenceProtoStats,
    next_gp_id: u64,
    completed_gps: u64,
    total_gp_duration: u64,
}

impl CoopFenceProtocol {
    pub fn new() -> Self {
        Self {
            threads: BTreeMap::new(),
            current_epoch: 0,
            grace_periods: Vec::new(),
            barriers: BTreeMap::new(),
            stats: CoopFenceProtoStats::default(),
            next_gp_id: 1,
            completed_gps: 0,
            total_gp_duration: 0,
        }
    }

    pub fn register_thread(&mut self, tid: u64) {
        self.threads
            .entry(tid)
            .or_insert_with(|| ThreadEpochInfo::new(tid));
    }

    pub fn record_fence(&mut self, tid: u64, _fence_type: FenceType) {
        if let Some(thread) = self.threads.get_mut(&tid) {
            thread.record_fence();
        }
    }

    pub fn enter_quiescent(&mut self, tid: u64, now_ns: u64) {
        if let Some(thread) = self.threads.get_mut(&tid) {
            thread.enter_quiescent(now_ns);
        }
        self.try_advance_epoch(now_ns);
    }

    pub fn exit_quiescent(&mut self, tid: u64) {
        if let Some(thread) = self.threads.get_mut(&tid) {
            thread.exit_quiescent();
        }
    }

    pub fn pin(&mut self, tid: u64) {
        if let Some(thread) = self.threads.get_mut(&tid) {
            thread.pin();
        }
    }

    pub fn unpin(&mut self, tid: u64) {
        if let Some(thread) = self.threads.get_mut(&tid) {
            thread.unpin();
        }
    }

    /// Try to advance the global epoch
    fn try_advance_epoch(&mut self, now_ns: u64) {
        let all_quiescent = self
            .threads
            .values()
            .all(|t| matches!(t.state, EpochState::Quiescent | EpochState::Offline));

        if all_quiescent && !self.threads.is_empty() {
            self.current_epoch += 1;
            for thread in self.threads.values_mut() {
                thread.current_epoch = self.current_epoch;
            }
            // Complete any grace periods waiting for this epoch
            for gp in self.grace_periods.iter_mut() {
                if !gp.is_complete() && self.current_epoch >= gp.end_epoch {
                    gp.complete(now_ns);
                    self.completed_gps += 1;
                    self.total_gp_duration += gp.duration_ns();
                }
            }
        }
        self.update_stats();
    }

    /// Start a new grace period
    pub fn start_grace_period(&mut self, now_ns: u64) -> u64 {
        let id = self.next_gp_id;
        self.next_gp_id += 1;
        self.grace_periods
            .push(GracePeriod::new(id, self.current_epoch, now_ns));
        self.update_stats();
        id
    }

    /// Create a barrier group
    pub fn create_barrier(&mut self, barrier_id: u64, expected: u32, now_ns: u64) {
        self.barriers
            .insert(barrier_id, BarrierGroup::new(barrier_id, expected, now_ns));
    }

    /// Arrive at barrier
    pub fn barrier_arrive(&mut self, barrier_id: u64, tid: u64, now_ns: u64) -> bool {
        if let Some(barrier) = self.barriers.get_mut(&barrier_id) {
            barrier.arrive(tid, now_ns)
        } else {
            false
        }
    }

    fn update_stats(&mut self) {
        self.stats.tracked_threads = self.threads.len();
        self.stats.current_epoch = self.current_epoch;
        self.stats.active_grace_periods = self
            .grace_periods
            .iter()
            .filter(|gp| !gp.is_complete())
            .count();
        self.stats.completed_grace_periods = self.completed_gps;
        self.stats.active_barriers = self.barriers.values().filter(|b| !b.is_complete()).count();
        self.stats.total_fences = self.threads.values().map(|t| t.fence_count).sum();
        if self.completed_gps > 0 {
            self.stats.avg_gp_duration_ns =
                self.total_gp_duration as f64 / self.completed_gps as f64;
        }
    }

    pub fn stats(&self) -> &CoopFenceProtoStats {
        &self.stats
    }
}
