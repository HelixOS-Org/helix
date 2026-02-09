//! # Cooperative Barrier Synchronization
//!
//! Advanced barrier primitives for cooperative scheduling:
//! - Named barriers with dynamic participant sets
//! - Phase barriers with rolling progression
//! - Timeout-aware barriers
//! - Hierarchical barriers (team → global)
//! - Barrier analytics and profiling
//! - Fuzzy barriers (partial synchronization)

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// BARRIER TYPES
// ============================================================================

/// Barrier type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarrierType {
    /// All must arrive before any proceeds
    Full,
    /// N of M must arrive
    Quorum,
    /// Rolling: once arrived, can proceed after next wave
    Rolling,
    /// Fuzzy: partial sync within window
    Fuzzy,
    /// Hierarchical: local then global
    Hierarchical,
}

/// Barrier state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarrierState {
    /// Open for arrivals
    Open,
    /// All arrived, releasing
    Releasing,
    /// Completed
    Complete,
    /// Timed out
    TimedOut,
    /// Cancelled
    Cancelled,
}

// ============================================================================
// BARRIER DEFINITION
// ============================================================================

/// Barrier configuration
#[derive(Debug, Clone)]
pub struct BarrierConfig {
    /// Barrier type
    pub barrier_type: BarrierType,
    /// Expected participants
    pub expected: u32,
    /// Quorum count (for Quorum type)
    pub quorum: u32,
    /// Timeout (ms, 0 = infinite)
    pub timeout_ms: u64,
    /// Auto-reset after release
    pub auto_reset: bool,
    /// Fuzzy window (ms)
    pub fuzzy_window_ms: u64,
}

impl BarrierConfig {
    #[inline]
    pub fn full(expected: u32) -> Self {
        Self {
            barrier_type: BarrierType::Full,
            expected,
            quorum: expected,
            timeout_ms: 0,
            auto_reset: false,
            fuzzy_window_ms: 0,
        }
    }

    #[inline]
    pub fn quorum(expected: u32, quorum: u32) -> Self {
        Self {
            barrier_type: BarrierType::Quorum,
            expected,
            quorum: quorum.min(expected),
            timeout_ms: 0,
            auto_reset: false,
            fuzzy_window_ms: 0,
        }
    }

    #[inline(always)]
    pub fn with_timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    #[inline(always)]
    pub fn with_auto_reset(mut self) -> Self {
        self.auto_reset = true;
        self
    }
}

// ============================================================================
// BARRIER PARTICIPANT
// ============================================================================

/// Participant in a barrier
#[derive(Debug, Clone)]
pub struct BarrierParticipant {
    /// Process ID
    pub pid: u64,
    /// Arrival timestamp (0 = not arrived)
    pub arrived_at: u64,
    /// Has been released
    pub released: bool,
    /// Wait time (ms)
    pub wait_time_ms: u64,
}

// ============================================================================
// BARRIER INSTANCE
// ============================================================================

/// Active barrier
#[derive(Debug, Clone)]
pub struct BarrierInstance {
    /// Barrier ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Configuration
    pub config: BarrierConfig,
    /// State
    pub state: BarrierState,
    /// Participants
    pub participants: BTreeMap<u64, BarrierParticipant>,
    /// Arrived count
    pub arrived_count: u32,
    /// Phase (for rolling barriers)
    pub phase: u64,
    /// Created timestamp
    pub created_at: u64,
    /// Released timestamp
    pub released_at: u64,
}

impl BarrierInstance {
    pub fn new(id: u64, name: String, config: BarrierConfig, now: u64) -> Self {
        Self {
            id,
            name,
            config,
            state: BarrierState::Open,
            participants: BTreeMap::new(),
            arrived_count: 0,
            phase: 0,
            created_at: now,
            released_at: 0,
        }
    }

    /// Register participant
    #[inline]
    pub fn register(&mut self, pid: u64) {
        self.participants.insert(pid, BarrierParticipant {
            pid,
            arrived_at: 0,
            released: false,
            wait_time_ms: 0,
        });
    }

    /// Arrive at barrier
    pub fn arrive(&mut self, pid: u64, now: u64) -> bool {
        if self.state != BarrierState::Open {
            return false;
        }

        if let Some(participant) = self.participants.get_mut(&pid) {
            if participant.arrived_at == 0 {
                participant.arrived_at = now;
                self.arrived_count += 1;
            }
        }

        // Check if barrier should release
        self.check_release(now)
    }

    /// Check release conditions
    fn check_release(&mut self, now: u64) -> bool {
        let should_release = match self.config.barrier_type {
            BarrierType::Full => self.arrived_count >= self.config.expected,
            BarrierType::Quorum => self.arrived_count >= self.config.quorum,
            BarrierType::Rolling => self.arrived_count >= self.config.expected,
            BarrierType::Fuzzy => {
                // Release if quorum within fuzzy window
                if self.arrived_count >= self.config.quorum {
                    true
                } else {
                    let first_arrival = self
                        .participants
                        .values()
                        .filter(|p| p.arrived_at > 0)
                        .map(|p| p.arrived_at)
                        .min()
                        .unwrap_or(now);
                    now.saturating_sub(first_arrival) >= self.config.fuzzy_window_ms
                        && self.arrived_count > 0
                }
            },
            BarrierType::Hierarchical => self.arrived_count >= self.config.expected,
        };

        if should_release {
            self.release(now);
        }

        should_release
    }

    /// Release all participants
    fn release(&mut self, now: u64) {
        self.state = BarrierState::Releasing;
        self.released_at = now;

        for participant in self.participants.values_mut() {
            if participant.arrived_at > 0 {
                participant.wait_time_ms = now.saturating_sub(participant.arrived_at);
                participant.released = true;
            }
        }

        self.state = BarrierState::Complete;

        if self.config.auto_reset {
            self.reset(now);
        }
    }

    /// Reset for next phase
    #[inline]
    pub fn reset(&mut self, _now: u64) {
        self.state = BarrierState::Open;
        self.arrived_count = 0;
        self.phase += 1;

        for participant in self.participants.values_mut() {
            participant.arrived_at = 0;
            participant.released = false;
            participant.wait_time_ms = 0;
        }
    }

    /// Check timeout
    #[inline]
    pub fn check_timeout(&mut self, now: u64) -> bool {
        if self.config.timeout_ms > 0
            && self.state == BarrierState::Open
            && now.saturating_sub(self.created_at) >= self.config.timeout_ms
        {
            self.state = BarrierState::TimedOut;
            true
        } else {
            false
        }
    }

    /// Average wait time
    #[inline]
    pub fn avg_wait_ms(&self) -> u64 {
        let completed: Vec<&BarrierParticipant> =
            self.participants.values().filter(|p| p.released).collect();
        if completed.is_empty() {
            return 0;
        }
        completed.iter().map(|p| p.wait_time_ms).sum::<u64>() / completed.len() as u64
    }

    /// Max wait time
    #[inline]
    pub fn max_wait_ms(&self) -> u64 {
        self.participants
            .values()
            .filter(|p| p.released)
            .map(|p| p.wait_time_ms)
            .max()
            .unwrap_or(0)
    }
}

// ============================================================================
// BARRIER ANALYTICS
// ============================================================================

/// Barrier performance summary
#[derive(Debug, Clone)]
pub struct BarrierSummary {
    /// Barrier ID
    pub id: u64,
    /// Barrier name
    pub name: String,
    /// Total phases completed
    pub phases_completed: u64,
    /// Average wait time (ms)
    pub avg_wait_ms: u64,
    /// Max wait time (ms)
    pub max_wait_ms: u64,
    /// Timeout count
    pub timeouts: u64,
    /// Average arrival spread (first to last, ms)
    pub avg_spread_ms: u64,
}

// ============================================================================
// BARRIER MANAGER
// ============================================================================

/// Barrier manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BarrierManagerStats {
    /// Active barriers
    pub active: usize,
    /// Completed barriers
    pub completed: u64,
    /// Timed out barriers
    pub timed_out: u64,
    /// Total phases
    pub total_phases: u64,
}

/// Cooperative barrier manager
pub struct CoopBarrierManager {
    /// Active barriers
    barriers: BTreeMap<u64, BarrierInstance>,
    /// Next barrier ID
    next_id: u64,
    /// Summaries for completed barriers
    summaries: VecDeque<BarrierSummary>,
    /// Stats
    stats: BarrierManagerStats,
    /// Max summaries
    max_summaries: usize,
}

impl CoopBarrierManager {
    pub fn new() -> Self {
        Self {
            barriers: BTreeMap::new(),
            next_id: 1,
            summaries: VecDeque::new(),
            stats: BarrierManagerStats::default(),
            max_summaries: 256,
        }
    }

    /// Create barrier
    #[inline]
    pub fn create(&mut self, name: String, config: BarrierConfig, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.barriers
            .insert(id, BarrierInstance::new(id, name, config, now));
        self.stats.active = self.barriers.len();
        id
    }

    /// Register participant
    #[inline]
    pub fn register(&mut self, barrier_id: u64, pid: u64) -> bool {
        if let Some(barrier) = self.barriers.get_mut(&barrier_id) {
            barrier.register(pid);
            true
        } else {
            false
        }
    }

    /// Arrive at barrier
    #[inline]
    pub fn arrive(&mut self, barrier_id: u64, pid: u64, now: u64) -> bool {
        if let Some(barrier) = self.barriers.get_mut(&barrier_id) {
            barrier.arrive(pid, now)
        } else {
            false
        }
    }

    /// Check timeouts
    #[inline]
    pub fn check_timeouts(&mut self, now: u64) -> Vec<u64> {
        let mut timed_out = Vec::new();
        for (id, barrier) in &mut self.barriers {
            if barrier.check_timeout(now) {
                timed_out.push(*id);
                self.stats.timed_out += 1;
            }
        }
        timed_out
    }

    /// Cleanup completed barriers
    pub fn cleanup_completed(&mut self) {
        let completed_ids: Vec<u64> = self
            .barriers
            .iter()
            .filter(|(_, b)| {
                matches!(
                    b.state,
                    BarrierState::Complete | BarrierState::Cancelled | BarrierState::TimedOut
                ) && !b.config.auto_reset
            })
            .map(|(id, _)| *id)
            .collect();

        for id in completed_ids {
            if let Some(barrier) = self.barriers.remove(&id) {
                self.summaries.push_back(BarrierSummary {
                    id: barrier.id,
                    name: barrier.name,
                    phases_completed: barrier.phase,
                    avg_wait_ms: barrier.avg_wait_ms(),
                    max_wait_ms: barrier.max_wait_ms(),
                    timeouts: if barrier.state == BarrierState::TimedOut {
                        1
                    } else {
                        0
                    },
                    avg_spread_ms: 0,
                });

                if self.summaries.len() > self.max_summaries {
                    self.summaries.pop_front();
                }

                self.stats.completed += 1;
            }
        }

        self.stats.active = self.barriers.len();
    }

    /// Get barrier
    #[inline(always)]
    pub fn barrier(&self, id: u64) -> Option<&BarrierInstance> {
        self.barriers.get(&id)
    }

    /// Get stats
    #[inline(always)]
    pub fn stats(&self) -> &BarrierManagerStats {
        &self.stats
    }

    /// Cancel barrier
    #[inline]
    pub fn cancel(&mut self, id: u64) {
        if let Some(barrier) = self.barriers.get_mut(&id) {
            barrier.state = BarrierState::Cancelled;
        }
    }
}

// ============================================================================
// Merged from barrier_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarrierType {
    /// All-or-nothing: all participants must arrive
    FullSync,
    /// Quorum-based: majority must arrive
    Quorum,
    /// Threshold: specific count must arrive
    Threshold,
    /// Phased: multiple sequential barrier rounds
    Phased,
    /// Tree-based: hierarchical barrier reduction
    Tree,
    /// Fuzzy: eventual arrival with timeout
    Fuzzy,
}

/// Participant arrival state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrivalState {
    Waiting,
    Arrived,
    TimedOut,
    Failed,
    Released,
}

/// A barrier participant
#[derive(Debug, Clone)]
pub struct BarrierParticipant {
    pub pid: u64,
    pub state: ArrivalState,
    pub arrival_ns: u64,
    pub wait_ns: u64,
    pub phase: u32,
    pub sense: bool,
}

impl BarrierParticipant {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            state: ArrivalState::Waiting,
            arrival_ns: 0,
            wait_ns: 0,
            phase: 0,
            sense: false,
        }
    }

    #[inline(always)]
    pub fn arrive(&mut self, now_ns: u64) {
        self.state = ArrivalState::Arrived;
        self.arrival_ns = now_ns;
    }

    #[inline]
    pub fn release(&mut self, now_ns: u64) {
        self.wait_ns = now_ns.saturating_sub(self.arrival_ns);
        self.state = ArrivalState::Released;
        self.phase += 1;
        self.sense = !self.sense;
    }

    #[inline(always)]
    pub fn timeout(&mut self) {
        self.state = ArrivalState::TimedOut;
    }

    #[inline]
    pub fn reset(&mut self) {
        self.state = ArrivalState::Waiting;
        self.arrival_ns = 0;
        self.wait_ns = 0;
    }
}

/// A barrier instance
#[derive(Debug)]
pub struct BarrierInstance {
    pub id: u64,
    pub name: String,
    pub barrier_type: BarrierType,
    pub expected: u32,
    pub threshold: u32,
    pub current_phase: u32,
    pub timeout_ns: u64,
    participants: BTreeMap<u64, BarrierParticipant>,
    pub completions: u64,
    pub timeouts: u64,
    pub total_wait_ns: u64,
    pub max_wait_ns: u64,
}

impl BarrierInstance {
    pub fn new(id: u64, name: String, barrier_type: BarrierType, expected: u32) -> Self {
        let threshold = match barrier_type {
            BarrierType::FullSync => expected,
            BarrierType::Quorum => (expected / 2) + 1,
            BarrierType::Threshold => expected, // overridden via set_threshold
            BarrierType::Phased | BarrierType::Tree | BarrierType::Fuzzy => expected,
        };
        Self {
            id,
            name,
            barrier_type,
            expected,
            threshold,
            current_phase: 0,
            timeout_ns: 10_000_000_000, // 10s default
            participants: BTreeMap::new(),
            completions: 0,
            timeouts: 0,
            total_wait_ns: 0,
            max_wait_ns: 0,
        }
    }

    #[inline(always)]
    pub fn set_threshold(&mut self, threshold: u32) {
        self.threshold = threshold;
    }

    #[inline(always)]
    pub fn add_participant(&mut self, pid: u64) {
        self.participants.insert(pid, BarrierParticipant::new(pid));
    }

    #[inline(always)]
    pub fn remove_participant(&mut self, pid: u64) {
        self.participants.remove(&pid);
    }

    #[inline]
    pub fn arrive(&mut self, pid: u64, now_ns: u64) -> bool {
        if let Some(p) = self.participants.get_mut(&pid) {
            p.arrive(now_ns);
            self.check_release(now_ns)
        } else {
            false
        }
    }

    fn arrived_count(&self) -> u32 {
        self.participants
            .values()
            .filter(|p| p.state == ArrivalState::Arrived)
            .count() as u32
    }

    fn check_release(&mut self, now_ns: u64) -> bool {
        if self.arrived_count() >= self.threshold {
            self.release_all(now_ns);
            true
        } else {
            false
        }
    }

    fn release_all(&mut self, now_ns: u64) {
        for p in self.participants.values_mut() {
            if p.state == ArrivalState::Arrived {
                p.release(now_ns);
                self.total_wait_ns += p.wait_ns;
                if p.wait_ns > self.max_wait_ns {
                    self.max_wait_ns = p.wait_ns;
                }
            }
        }
        self.completions += 1;
        self.current_phase += 1;
        // Reset waiting states for next phase
        for p in self.participants.values_mut() {
            if p.state == ArrivalState::Released {
                p.reset();
            }
        }
    }

    pub fn expire_timeouts(&mut self, now_ns: u64) -> u32 {
        let mut expired = 0u32;
        for p in self.participants.values_mut() {
            if p.state == ArrivalState::Waiting && p.arrival_ns > 0 {
                if now_ns.saturating_sub(p.arrival_ns) > self.timeout_ns {
                    p.timeout();
                    expired += 1;
                }
            }
        }
        self.timeouts += expired as u64;
        expired
    }

    #[inline(always)]
    pub fn participant_count(&self) -> usize {
        self.participants.len()
    }

    #[inline]
    pub fn avg_wait_ns(&self) -> f64 {
        if self.completions == 0 {
            return 0.0;
        }
        let total_participants = self.completions as f64 * self.threshold as f64;
        if total_participants == 0.0 {
            return 0.0;
        }
        self.total_wait_ns as f64 / total_participants
    }

    #[inline]
    pub fn completion_rate(&self) -> f64 {
        let total = self.completions + self.timeouts;
        if total == 0 {
            return 0.0;
        }
        self.completions as f64 / total as f64
    }

    #[inline]
    pub fn stragglers(&self) -> Vec<u64> {
        self.participants
            .iter()
            .filter(|(_, p)| p.state == ArrivalState::Waiting)
            .map(|(&pid, _)| pid)
            .collect()
    }
}

/// Barrier stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BarrierV2Stats {
    pub total_barriers: u64,
    pub total_completions: u64,
    pub total_timeouts: u64,
    pub total_participants: u64,
    pub avg_completion_ns: f64,
    pub max_wait_ns: u64,
}

/// Main barrier v2 manager
pub struct CoopBarrierV2 {
    barriers: BTreeMap<u64, BarrierInstance>,
    next_id: u64,
    stats: BarrierV2Stats,
}

impl CoopBarrierV2 {
    pub fn new() -> Self {
        Self {
            barriers: BTreeMap::new(),
            next_id: 1,
            stats: BarrierV2Stats {
                total_barriers: 0,
                total_completions: 0,
                total_timeouts: 0,
                total_participants: 0,
                avg_completion_ns: 0.0,
                max_wait_ns: 0,
            },
        }
    }

    #[inline]
    pub fn create_barrier(
        &mut self,
        name: String,
        barrier_type: BarrierType,
        expected: u32,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.barriers
            .insert(id, BarrierInstance::new(id, name, barrier_type, expected));
        self.stats.total_barriers += 1;
        id
    }

    #[inline]
    pub fn add_participant(&mut self, barrier_id: u64, pid: u64) {
        if let Some(b) = self.barriers.get_mut(&barrier_id) {
            b.add_participant(pid);
            self.stats.total_participants += 1;
        }
    }

    pub fn arrive(&mut self, barrier_id: u64, pid: u64, now_ns: u64) -> bool {
        if let Some(b) = self.barriers.get_mut(&barrier_id) {
            let released = b.arrive(pid, now_ns);
            if released {
                self.stats.total_completions += 1;
                if b.max_wait_ns > self.stats.max_wait_ns {
                    self.stats.max_wait_ns = b.max_wait_ns;
                }
            }
            released
        } else {
            false
        }
    }

    #[inline]
    pub fn expire_all(&mut self, now_ns: u64) -> u32 {
        let mut total = 0u32;
        for b in self.barriers.values_mut() {
            total += b.expire_timeouts(now_ns);
        }
        self.stats.total_timeouts += total as u64;
        total
    }

    #[inline(always)]
    pub fn destroy_barrier(&mut self, id: u64) -> bool {
        self.barriers.remove(&id).is_some()
    }

    #[inline(always)]
    pub fn get_barrier(&self, id: u64) -> Option<&BarrierInstance> {
        self.barriers.get(&id)
    }

    #[inline]
    pub fn slowest_barriers(&self, top: usize) -> Vec<(u64, f64)> {
        let mut v: Vec<(u64, f64)> = self
            .barriers
            .iter()
            .filter(|(_, b)| b.completions > 0)
            .map(|(&id, b)| (id, b.avg_wait_ns()))
            .collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        v.truncate(top);
        v
    }

    #[inline(always)]
    pub fn stats(&self) -> &BarrierV2Stats {
        &self.stats
    }
}
