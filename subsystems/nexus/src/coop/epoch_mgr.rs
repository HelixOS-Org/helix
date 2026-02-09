//! # Coop Epoch Manager
//!
//! Epoch-based coordination and reclamation:
//! - Global epoch counter management
//! - Per-participant epoch pinning
//! - Grace period detection for safe reclamation
//! - Epoch advancement with quiescent state tracking
//! - Deferred cleanup queue
//! - Concurrent epoch observation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Epoch value
pub type Epoch = u64;

/// Participant state in epoch protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticipantState {
    Active,
    Pinned,
    Quiescent,
    Offline,
}

/// Epoch participant
#[derive(Debug, Clone)]
pub struct EpochParticipant {
    pub id: u64,
    pub state: ParticipantState,
    pub pinned_epoch: Epoch,
    pub local_epoch: Epoch,
    pub enter_count: u64,
    pub exit_count: u64,
    pub last_activity_ts: u64,
}

impl EpochParticipant {
    pub fn new(id: u64, epoch: Epoch, ts: u64) -> Self {
        Self {
            id, state: ParticipantState::Quiescent,
            pinned_epoch: epoch, local_epoch: epoch,
            enter_count: 0, exit_count: 0, last_activity_ts: ts,
        }
    }

    #[inline]
    pub fn enter(&mut self, global_epoch: Epoch, ts: u64) {
        self.state = ParticipantState::Pinned;
        self.local_epoch = global_epoch;
        self.pinned_epoch = global_epoch;
        self.enter_count += 1;
        self.last_activity_ts = ts;
    }

    #[inline]
    pub fn exit(&mut self, ts: u64) {
        self.state = ParticipantState::Quiescent;
        self.exit_count += 1;
        self.last_activity_ts = ts;
    }

    #[inline(always)]
    pub fn is_pinned(&self) -> bool { self.state == ParticipantState::Pinned }
    #[inline(always)]
    pub fn is_quiescent(&self) -> bool { self.state == ParticipantState::Quiescent }
}

/// Deferred cleanup entry
#[derive(Debug, Clone)]
pub struct DeferredCleanup {
    pub id: u64,
    pub retire_epoch: Epoch,
    pub cleanup_type: CleanupType,
    pub data_id: u64,
    pub deferred_ts: u64,
}

/// Cleanup type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupType {
    FreeMemory,
    CloseHandle,
    RemoveEntry,
    ReleaseResource,
    Custom,
}

/// Epoch advancement result
#[derive(Debug, Clone, Copy)]
pub struct AdvanceResult {
    pub new_epoch: Epoch,
    pub old_epoch: Epoch,
    pub reclaimable_count: usize,
    pub all_quiescent: bool,
}

/// Epoch manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct EpochMgrStats {
    pub current_epoch: Epoch,
    pub participants: usize,
    pub active_participants: usize,
    pub pinned_participants: usize,
    pub deferred_cleanups: usize,
    pub total_advances: u64,
    pub total_reclaimed: u64,
    pub min_pinned_epoch: Epoch,
}

/// Cooperative epoch manager
pub struct CoopEpochMgr {
    global_epoch: Epoch,
    participants: BTreeMap<u64, EpochParticipant>,
    deferred: Vec<DeferredCleanup>,
    next_cleanup_id: u64,
    max_deferred: usize,
    stats: EpochMgrStats,
}

impl CoopEpochMgr {
    pub fn new() -> Self {
        Self {
            global_epoch: 1, participants: BTreeMap::new(),
            deferred: Vec::new(), next_cleanup_id: 1,
            max_deferred: 4096, stats: EpochMgrStats::default(),
        }
    }

    #[inline(always)]
    pub fn register(&mut self, id: u64, ts: u64) {
        self.participants.insert(id, EpochParticipant::new(id, self.global_epoch, ts));
    }

    #[inline(always)]
    pub fn unregister(&mut self, id: u64) {
        if let Some(p) = self.participants.get_mut(&id) { p.state = ParticipantState::Offline; }
    }

    #[inline]
    pub fn pin(&mut self, participant_id: u64, ts: u64) {
        if let Some(p) = self.participants.get_mut(&participant_id) {
            p.enter(self.global_epoch, ts);
        }
    }

    #[inline]
    pub fn unpin(&mut self, participant_id: u64, ts: u64) {
        if let Some(p) = self.participants.get_mut(&participant_id) {
            p.exit(ts);
        }
    }

    #[inline]
    pub fn defer_cleanup(&mut self, cleanup_type: CleanupType, data_id: u64, ts: u64) -> u64 {
        let id = self.next_cleanup_id;
        self.next_cleanup_id += 1;
        self.deferred.push(DeferredCleanup {
            id, retire_epoch: self.global_epoch, cleanup_type, data_id, deferred_ts: ts,
        });
        id
    }

    pub fn try_advance(&mut self) -> Option<AdvanceResult> {
        // Can advance if all participants have observed current epoch
        let all_caught_up = self.participants.values()
            .filter(|p| p.state != ParticipantState::Offline)
            .all(|p| p.is_quiescent() || p.local_epoch >= self.global_epoch);

        if !all_caught_up { return None; }

        let old = self.global_epoch;
        self.global_epoch += 1;
        self.stats.total_advances += 1;

        let reclaimable = self.collect_reclaimable();
        Some(AdvanceResult {
            new_epoch: self.global_epoch,
            old_epoch: old,
            reclaimable_count: reclaimable,
            all_quiescent: self.participants.values().all(|p| p.is_quiescent() || p.state == ParticipantState::Offline),
        })
    }

    fn collect_reclaimable(&mut self) -> usize {
        let min_pinned = self.min_pinned_epoch();
        let before = self.deferred.len();
        self.deferred.retain(|d| d.retire_epoch >= min_pinned);
        let reclaimed = before - self.deferred.len();
        self.stats.total_reclaimed += reclaimed as u64;
        reclaimed
    }

    fn min_pinned_epoch(&self) -> Epoch {
        self.participants.values()
            .filter(|p| p.is_pinned())
            .map(|p| p.pinned_epoch)
            .min()
            .unwrap_or(self.global_epoch)
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.current_epoch = self.global_epoch;
        self.stats.participants = self.participants.len();
        self.stats.active_participants = self.participants.values().filter(|p| p.state != ParticipantState::Offline).count();
        self.stats.pinned_participants = self.participants.values().filter(|p| p.is_pinned()).count();
        self.stats.deferred_cleanups = self.deferred.len();
        self.stats.min_pinned_epoch = self.min_pinned_epoch();
    }

    #[inline(always)]
    pub fn global_epoch(&self) -> Epoch { self.global_epoch }
    #[inline(always)]
    pub fn participant(&self, id: u64) -> Option<&EpochParticipant> { self.participants.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &EpochMgrStats { &self.stats }
}

// ============================================================================
// Merged from epoch_mgr_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpochV2 {
    Zero,
    One,
    Two,
}

impl EpochV2 {
    #[inline(always)]
    pub fn next(self) -> Self { match self { Self::Zero => Self::One, Self::One => Self::Two, Self::Two => Self::Zero } }
    #[inline(always)]
    pub fn as_u8(self) -> u8 { match self { Self::Zero => 0, Self::One => 1, Self::Two => 2 } }
}

/// Thread epoch state
#[derive(Debug)]
pub struct EpochV2Thread {
    pub tid: u64,
    pub local_epoch: EpochV2,
    pub active: bool,
    pub pin_count: u64,
    pub gc_count: u64,
}

impl EpochV2Thread {
    pub fn new(tid: u64) -> Self { Self { tid, local_epoch: EpochV2::Zero, active: false, pin_count: 0, gc_count: 0 } }
    #[inline(always)]
    pub fn pin(&mut self, global: EpochV2) { self.active = true; self.local_epoch = global; self.pin_count += 1; }
    #[inline(always)]
    pub fn unpin(&mut self) { self.active = false; }
    #[inline(always)]
    pub fn is_pinned(&self) -> bool { self.active }
}

/// Garbage entry
#[derive(Debug)]
pub struct GarbageEntry {
    pub addr: u64,
    pub size: u64,
    pub epoch: EpochV2,
    pub queued_at: u64,
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EpochV2MgrStats {
    pub global_epoch: u8,
    pub total_threads: u32,
    pub active_threads: u32,
    pub total_pins: u64,
    pub garbage_pending: u32,
    pub garbage_bytes: u64,
    pub total_reclaimed: u64,
}

/// Main epoch manager v2
pub struct CoopEpochMgrV2 {
    global_epoch: EpochV2,
    threads: BTreeMap<u64, EpochV2Thread>,
    garbage: [Vec<GarbageEntry>; 3],
    total_reclaimed: u64,
    advance_count: u64,
}

impl CoopEpochMgrV2 {
    pub fn new() -> Self {
        Self { global_epoch: EpochV2::Zero, threads: BTreeMap::new(), garbage: [Vec::new(), Vec::new(), Vec::new()], total_reclaimed: 0, advance_count: 0 }
    }

    #[inline(always)]
    pub fn register(&mut self, tid: u64) { self.threads.insert(tid, EpochV2Thread::new(tid)); }

    #[inline(always)]
    pub fn pin(&mut self, tid: u64) {
        let epoch = self.global_epoch;
        if let Some(t) = self.threads.get_mut(&tid) { t.pin(epoch); }
    }

    #[inline(always)]
    pub fn unpin(&mut self, tid: u64) {
        if let Some(t) = self.threads.get_mut(&tid) { t.unpin(); }
    }

    #[inline(always)]
    pub fn retire(&mut self, addr: u64, size: u64, now: u64) {
        let idx = self.global_epoch.as_u8() as usize;
        self.garbage[idx].push(GarbageEntry { addr, size, epoch: self.global_epoch, queued_at: now });
    }

    pub fn try_advance(&mut self) -> bool {
        let all_caught_up = self.threads.values()
            .filter(|t| t.active)
            .all(|t| t.local_epoch == self.global_epoch);
        if all_caught_up {
            let safe_epoch = self.global_epoch.next().next();
            let idx = safe_epoch.as_u8() as usize;
            let reclaimed: u64 = self.garbage[idx].iter().map(|g| g.size).sum();
            self.total_reclaimed += reclaimed;
            self.garbage[idx].clear();
            self.global_epoch = self.global_epoch.next();
            self.advance_count += 1;
            true
        } else { false }
    }

    #[inline]
    pub fn stats(&self) -> EpochV2MgrStats {
        let active = self.threads.values().filter(|t| t.active).count() as u32;
        let pins: u64 = self.threads.values().map(|t| t.pin_count).sum();
        let pending: u32 = self.garbage.iter().map(|g| g.len() as u32).sum();
        let bytes: u64 = self.garbage.iter().flat_map(|g| g.iter()).map(|e| e.size).sum();
        EpochV2MgrStats { global_epoch: self.global_epoch.as_u8(), total_threads: self.threads.len() as u32, active_threads: active, total_pins: pins, garbage_pending: pending, garbage_bytes: bytes, total_reclaimed: self.total_reclaimed }
    }
}

// ============================================================================
// Merged from epoch_mgr_v3
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpochV3State {
    Active,
    Quiescent,
    Offline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpochV3GcPolicy {
    Eager,
    Lazy,
    Batched(u32),
    Threshold(u64),
}

#[derive(Debug, Clone)]
pub struct EpochV3Garbage {
    pub addr: u64,
    pub size: u32,
    pub retire_epoch: u64,
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EpochV3ThreadState {
    pub thread_id: u32,
    pub local_epoch: u64,
    pub state: EpochV3State,
    pub pin_count: u64,
    pub unpin_count: u64,
    pub gc_count: u64,
    pub garbage: Vec<EpochV3Garbage>,
}

impl EpochV3ThreadState {
    pub fn new(thread_id: u32) -> Self {
        Self {
            thread_id, local_epoch: 0,
            state: EpochV3State::Offline,
            pin_count: 0, unpin_count: 0,
            gc_count: 0, garbage: Vec::new(),
        }
    }

    #[inline]
    pub fn pin(&mut self, global_epoch: u64) {
        self.local_epoch = global_epoch;
        self.state = EpochV3State::Active;
        self.pin_count += 1;
    }

    #[inline(always)]
    pub fn unpin(&mut self) {
        self.state = EpochV3State::Quiescent;
        self.unpin_count += 1;
    }

    #[inline(always)]
    pub fn defer_free(&mut self, addr: u64, size: u32, epoch: u64) {
        self.garbage.push(EpochV3Garbage { addr, size, retire_epoch: epoch });
    }

    #[inline]
    pub fn collect(&mut self, safe_epoch: u64) -> (u64, u64) {
        let before = self.garbage.len();
        self.garbage.retain(|g| g.retire_epoch >= safe_epoch);
        let collected = (before - self.garbage.len()) as u64;
        let bytes: u64 = collected * 64; // approximate
        self.gc_count += 1;
        (collected, bytes)
    }

    #[inline(always)]
    pub fn pending_garbage(&self) -> usize { self.garbage.len() }
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EpochV3Stats {
    pub global_epoch: u64,
    pub total_threads: u32,
    pub total_pins: u64,
    pub total_unpins: u64,
    pub total_collected: u64,
    pub total_deferred: u64,
    pub pending_garbage: u64,
}

pub struct CoopEpochMgrV3 {
    threads: BTreeMap<u32, EpochV3ThreadState>,
    global_epoch: AtomicU64,
    gc_policy: EpochV3GcPolicy,
    stats: EpochV3Stats,
}

impl CoopEpochMgrV3 {
    pub fn new(policy: EpochV3GcPolicy) -> Self {
        Self {
            threads: BTreeMap::new(),
            global_epoch: AtomicU64::new(0),
            gc_policy: policy,
            stats: EpochV3Stats {
                global_epoch: 0, total_threads: 0,
                total_pins: 0, total_unpins: 0,
                total_collected: 0, total_deferred: 0,
                pending_garbage: 0,
            },
        }
    }

    #[inline(always)]
    pub fn register_thread(&mut self, id: u32) {
        self.threads.insert(id, EpochV3ThreadState::new(id));
        self.stats.total_threads += 1;
    }

    #[inline]
    pub fn pin(&mut self, thread_id: u32) {
        let epoch = self.global_epoch.load(Ordering::Acquire);
        if let Some(t) = self.threads.get_mut(&thread_id) {
            t.pin(epoch);
            self.stats.total_pins += 1;
        }
    }

    #[inline]
    pub fn unpin(&mut self, thread_id: u32) {
        if let Some(t) = self.threads.get_mut(&thread_id) {
            t.unpin();
            self.stats.total_unpins += 1;
        }
    }

    #[inline]
    pub fn try_advance(&mut self) -> bool {
        let current = self.global_epoch.load(Ordering::Acquire);
        let all_quiescent = self.threads.values()
            .all(|t| t.state == EpochV3State::Quiescent || t.state == EpochV3State::Offline
                 || t.local_epoch == current);
        if all_quiescent {
            self.global_epoch.fetch_add(1, Ordering::Release);
            self.stats.global_epoch = current + 1;
            true
        } else { false }
    }

    #[inline]
    pub fn collect_garbage(&mut self) -> u64 {
        let safe = self.global_epoch.load(Ordering::Acquire).saturating_sub(2);
        let mut total = 0u64;
        for t in self.threads.values_mut() {
            let (count, _) = t.collect(safe);
            total += count;
        }
        self.stats.total_collected += total;
        total
    }

    #[inline(always)]
    pub fn stats(&self) -> &EpochV3Stats { &self.stats }
}
