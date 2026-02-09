//! # Bridge Futex Proxy
//!
//! Fast userspace mutex (futex) syscall optimization:
//! - Contention tracking per futex address
//! - Wait queue analysis
//! - Spin-vs-sleep decision support
//! - Priority inheritance proxying
//! - Deadlock detection hints

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;

use crate::fast::linear_map::LinearMap;

/// Futex operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FutexOp {
    Wait,
    Wake,
    WaitBitset,
    WakeBitset,
    Requeue,
    CmpRequeue,
    LockPi,
    UnlockPi,
}

/// Contention level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FutexContention {
    /// No contention (single waiter or less)
    None,
    /// Low (2-4 waiters)
    Low,
    /// Medium (5-16 waiters)
    Medium,
    /// High (>16 waiters)
    High,
    /// Convoy (serial wakeup pattern)
    Convoy,
}

/// Futex address entry
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FutexEntry {
    /// Futex address (userspace virtual)
    pub address: u64,
    /// Current waiter count
    pub waiters: u32,
    /// Peak waiter count
    pub peak_waiters: u32,
    /// Total wait operations
    pub total_waits: u64,
    /// Total wake operations
    pub total_wakes: u64,
    /// Total wait time (ns)
    pub total_wait_ns: u64,
    /// Average wait time (EMA, ns)
    pub avg_wait_ema_ns: f64,
    /// Last operation time (ns)
    pub last_op_ns: u64,
    /// Owner PID (if known)
    pub owner_pid: Option<u64>,
    /// Is priority inheritance?
    pub is_pi: bool,
    /// Requeue count
    pub requeues: u64,
}

impl FutexEntry {
    pub fn new(address: u64) -> Self {
        Self {
            address,
            waiters: 0,
            peak_waiters: 0,
            total_waits: 0,
            total_wakes: 0,
            total_wait_ns: 0,
            avg_wait_ema_ns: 0.0,
            last_op_ns: 0,
            owner_pid: None,
            is_pi: false,
            requeues: 0,
        }
    }

    /// Record wait
    #[inline]
    pub fn record_wait(&mut self, now_ns: u64) {
        self.waiters += 1;
        if self.waiters > self.peak_waiters {
            self.peak_waiters = self.waiters;
        }
        self.total_waits += 1;
        self.last_op_ns = now_ns;
    }

    /// Record wake
    #[inline]
    pub fn record_wake(&mut self, count: u32, now_ns: u64) {
        let woken = count.min(self.waiters);
        self.waiters = self.waiters.saturating_sub(woken);
        self.total_wakes += woken as u64;
        self.last_op_ns = now_ns;
    }

    /// Record wait completion (with latency)
    #[inline(always)]
    pub fn record_wait_complete(&mut self, wait_ns: u64) {
        self.total_wait_ns += wait_ns;
        self.avg_wait_ema_ns = 0.8 * self.avg_wait_ema_ns + 0.2 * wait_ns as f64;
    }

    /// Contention level
    pub fn contention(&self) -> FutexContention {
        match self.waiters {
            0..=1 => FutexContention::None,
            2..=4 => FutexContention::Low,
            5..=16 => FutexContention::Medium,
            _ => {
                // Check for convoy pattern
                if self.total_wakes > 10 && self.peak_waiters > 32 {
                    FutexContention::Convoy
                } else {
                    FutexContention::High
                }
            },
        }
    }

    /// Should spin instead of sleep?
    #[inline(always)]
    pub fn should_spin(&self) -> bool {
        // Spin if avg wait is very short (<1µs) and low contention
        self.avg_wait_ema_ns < 1000.0 && self.waiters < 3
    }

    /// Wakeup efficiency (wakes / waits)
    #[inline]
    pub fn wakeup_efficiency(&self) -> f64 {
        if self.total_waits == 0 {
            return 1.0;
        }
        (self.total_wakes as f64 / self.total_waits as f64).min(1.0)
    }
}

/// Potential deadlock
#[derive(Debug, Clone)]
pub struct FutexDeadlockHint {
    /// Addresses involved
    pub addresses: Vec<u64>,
    /// PIDs involved
    pub pids: Vec<u64>,
    /// Detection timestamp
    pub detected_ns: u64,
}

/// Futex proxy stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BridgeFutexStats {
    pub tracked_futexes: usize,
    pub total_waiters: u32,
    pub high_contention: usize,
    pub convoy_detected: usize,
    pub avg_wait_ns: f64,
    pub total_waits: u64,
    pub total_wakes: u64,
}

/// Bridge futex proxy
#[repr(align(64))]
pub struct BridgeFutexProxy {
    /// Tracked futexes (address -> entry)
    futexes: BTreeMap<u64, FutexEntry>,
    /// Wait chains (pid -> futex address being waited on)
    wait_chains: LinearMap<u64, 64>,
    /// Stats
    stats: BridgeFutexStats,
}

impl BridgeFutexProxy {
    pub fn new() -> Self {
        Self {
            futexes: BTreeMap::new(),
            wait_chains: LinearMap::new(),
            stats: BridgeFutexStats::default(),
        }
    }

    /// Record futex wait
    #[inline]
    pub fn record_wait(&mut self, address: u64, pid: u64, now_ns: u64) {
        let entry = self
            .futexes
            .entry(address)
            .or_insert_with(|| FutexEntry::new(address));
        entry.record_wait(now_ns);
        self.wait_chains.insert(pid, address);
        self.update_stats();
    }

    /// Record futex wake
    #[inline]
    pub fn record_wake(&mut self, address: u64, count: u32, now_ns: u64) {
        if let Some(entry) = self.futexes.get_mut(&address) {
            entry.record_wake(count, now_ns);
        }
        // Clean up wait chains for woken PIDs
        self.wait_chains
            .retain(|_, addr| *addr != address || count == 0);
        self.update_stats();
    }

    /// Get spin recommendation
    #[inline]
    pub fn should_spin(&self, address: u64) -> bool {
        self.futexes
            .get(&address)
            .map(|e| e.should_spin())
            .unwrap_or(false)
    }

    /// Get contention level
    #[inline]
    pub fn contention(&self, address: u64) -> FutexContention {
        self.futexes
            .get(&address)
            .map(|e| e.contention())
            .unwrap_or(FutexContention::None)
    }

    /// Detect potential deadlocks (simple cycle detection in wait chains)
    pub fn detect_deadlocks(&self, now_ns: u64) -> Vec<FutexDeadlockHint> {
        let mut hints = Vec::new();
        // Simple: check if any PID waits on a futex owned by another waiting PID
        for (&pid, &addr) in &self.wait_chains {
            if let Some(entry) = self.futexes.get(&addr) {
                if let Some(owner) = entry.owner_pid {
                    if owner != pid {
                        if let Some(&owner_waiting_on) = self.wait_chains.get(owner) {
                            // Owner is also waiting - potential deadlock
                            if let Some(other_entry) = self.futexes.get(&owner_waiting_on) {
                                if other_entry.owner_pid == Some(pid) {
                                    hints.push(FutexDeadlockHint {
                                        addresses: alloc::vec![addr, owner_waiting_on],
                                        pids: alloc::vec![pid, owner],
                                        detected_ns: now_ns,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        hints
    }

    /// Hot futexes (most contended)
    #[inline]
    pub fn hot_futexes(&self, n: usize) -> Vec<(u64, u32)> {
        let mut entries: Vec<(u64, u32)> = self
            .futexes
            .iter()
            .map(|(&addr, e)| (addr, e.waiters))
            .collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1));
        entries.truncate(n);
        entries
    }

    fn update_stats(&mut self) {
        self.stats.tracked_futexes = self.futexes.len();
        self.stats.total_waiters = self.futexes.values().map(|e| e.waiters).sum();
        self.stats.high_contention = self
            .futexes
            .values()
            .filter(|e| {
                matches!(
                    e.contention(),
                    FutexContention::High | FutexContention::Convoy
                )
            })
            .count();
        self.stats.convoy_detected = self
            .futexes
            .values()
            .filter(|e| matches!(e.contention(), FutexContention::Convoy))
            .count();
        if !self.futexes.is_empty() {
            self.stats.avg_wait_ns = self
                .futexes
                .values()
                .map(|e| e.avg_wait_ema_ns)
                .sum::<f64>()
                / self.futexes.len() as f64;
        }
        self.stats.total_waits = self.futexes.values().map(|e| e.total_waits).sum();
        self.stats.total_wakes = self.futexes.values().map(|e| e.total_wakes).sum();
    }

    #[inline(always)]
    pub fn stats(&self) -> &BridgeFutexStats {
        &self.stats
    }
}

// ============================================================================
// Merged from futex_v2
// ============================================================================

/// Waiter priority class
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WaiterPriority {
    Realtime(u8),
    Normal(u8),
    Idle,
}

impl WaiterPriority {
    #[inline]
    pub fn as_value(&self) -> u32 {
        match self {
            Self::Realtime(p) => *p as u32,
            Self::Normal(p) => 100 + *p as u32,
            Self::Idle => 200,
        }
    }
}

/// A waiter in the futex queue
#[derive(Debug, Clone)]
pub struct FutexWaiter {
    pub pid: u64,
    pub tid: u64,
    pub priority: WaiterPriority,
    pub bitset: u32,
    pub enqueue_ns: u64,
    pub deadline_ns: Option<u64>,
    pub boosted: bool,
}

impl FutexWaiter {
    pub fn new(pid: u64, tid: u64, priority: WaiterPriority) -> Self {
        Self {
            pid,
            tid,
            priority,
            bitset: 0xFFFF_FFFF,
            enqueue_ns: 0,
            deadline_ns: None,
            boosted: false,
        }
    }

    #[inline(always)]
    pub fn with_bitset(mut self, bitset: u32) -> Self {
        self.bitset = bitset;
        self
    }

    #[inline(always)]
    pub fn with_deadline(mut self, deadline_ns: u64) -> Self {
        self.deadline_ns = Some(deadline_ns);
        self
    }

    #[inline]
    pub fn is_expired(&self, now_ns: u64) -> bool {
        if let Some(deadline) = self.deadline_ns {
            now_ns >= deadline
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn wait_duration_ns(&self, now_ns: u64) -> u64 {
        now_ns.saturating_sub(self.enqueue_ns)
    }
}

/// Hash bucket for futex addresses
#[derive(Debug)]
pub struct FutexBucket {
    pub address: u64,
    pub waiters: VecDeque<FutexWaiter>,
    pub pi_owner: Option<u64>,
    total_waits: u64,
    total_wakes: u64,
    total_timeouts: u64,
    max_waiters: usize,
}

impl FutexBucket {
    pub fn new(address: u64) -> Self {
        Self {
            address,
            waiters: VecDeque::new(),
            pi_owner: None,
            total_waits: 0,
            total_wakes: 0,
            total_timeouts: 0,
            max_waiters: 0,
        }
    }

    #[inline]
    pub fn enqueue(&mut self, waiter: FutexWaiter) {
        self.total_waits += 1;
        self.waiters.push_back(waiter);
        if self.waiters.len() > self.max_waiters {
            self.max_waiters = self.waiters.len();
        }
        // Sort by priority (lower value = higher priority)
        self.waiters
            .make_contiguous()
            .sort_by_key(|w| w.priority.as_value());
    }

    #[inline]
    pub fn wake_one(&mut self) -> Option<FutexWaiter> {
        if self.waiters.is_empty() {
            return None;
        }
        self.total_wakes += 1;
        self.waiters.pop_front()
    }

    #[inline]
    pub fn wake_n(&mut self, n: usize) -> Vec<FutexWaiter> {
        let count = n.min(self.waiters.len());
        let woken: Vec<FutexWaiter> = self.waiters.drain(..count).collect();
        self.total_wakes += woken.len() as u64;
        woken
    }

    pub fn wake_bitset(&mut self, bitset: u32, max: usize) -> Vec<FutexWaiter> {
        let mut woken = Vec::new();
        let mut i = 0;
        while i < self.waiters.len() && woken.len() < max {
            if self.waiters[i].bitset & bitset != 0 {
                woken.push(self.waiters.remove(i).unwrap());
            } else {
                i += 1;
            }
        }
        self.total_wakes += woken.len() as u64;
        woken
    }

    pub fn expire_waiters(&mut self, now_ns: u64) -> Vec<FutexWaiter> {
        let mut expired = Vec::new();
        let mut i = 0;
        while i < self.waiters.len() {
            if self.waiters[i].is_expired(now_ns) {
                expired.push(self.waiters.remove(i).unwrap());
            } else {
                i += 1;
            }
        }
        self.total_timeouts += expired.len() as u64;
        expired
    }

    #[inline(always)]
    pub fn waiter_count(&self) -> usize {
        self.waiters.len()
    }

    #[inline]
    pub fn contention_score(&self) -> f64 {
        if self.total_waits == 0 {
            return 0.0;
        }
        self.max_waiters as f64 * (self.total_waits as f64 / (self.total_wakes.max(1) as f64))
    }
}

/// Priority inheritance chain entry
#[derive(Debug, Clone)]
pub struct PiChainEntry {
    pub holder_tid: u64,
    pub waiter_tid: u64,
    pub original_priority: WaiterPriority,
    pub boosted_priority: WaiterPriority,
    pub futex_addr: u64,
}

/// Futex v2 waitv entry (wait on multiple)
#[derive(Debug, Clone)]
pub struct WaitvEntry {
    pub address: u64,
    pub expected_val: u32,
    pub flags: u32,
}

/// Futex v2 stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FutexV2Stats {
    pub total_waits: u64,
    pub total_wakes: u64,
    pub total_requeues: u64,
    pub total_timeouts: u64,
    pub pi_boosts: u64,
    pub pi_chains_max_depth: u32,
    pub waitv_operations: u64,
    pub hash_buckets_used: u64,
}

/// Main bridge futex v2 manager
#[repr(align(64))]
pub struct BridgeFutexV2 {
    buckets: BTreeMap<u64, FutexBucket>,
    pi_chains: Vec<PiChainEntry>,
    max_pi_depth: u32,
    num_hash_bits: u32,
    stats: FutexV2Stats,
}

impl BridgeFutexV2 {
    pub fn new() -> Self {
        Self {
            buckets: BTreeMap::new(),
            pi_chains: Vec::new(),
            max_pi_depth: 16,
            num_hash_bits: 10,
            stats: FutexV2Stats {
                total_waits: 0,
                total_wakes: 0,
                total_requeues: 0,
                total_timeouts: 0,
                pi_boosts: 0,
                pi_chains_max_depth: 0,
                waitv_operations: 0,
                hash_buckets_used: 0,
            },
        }
    }

    fn hash_addr(&self, addr: u64) -> u64 {
        // FNV-1a hash truncated to bucket bits
        let mut hash: u64 = 0xcbf29ce484222325;
        let bytes = addr.to_le_bytes();
        for &b in &bytes {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash & ((1u64 << self.num_hash_bits) - 1)
    }

    fn get_or_create_bucket(&mut self, addr: u64) -> &mut FutexBucket {
        let key = self.hash_addr(addr);
        if !self.buckets.contains_key(&key) {
            self.buckets.insert(key, FutexBucket::new(addr));
            self.stats.hash_buckets_used += 1;
        }
        self.buckets.get_mut(&key).unwrap()
    }

    #[inline]
    pub fn futex_wait(&mut self, addr: u64, waiter: FutexWaiter) -> bool {
        let bucket = self.get_or_create_bucket(addr);
        bucket.enqueue(waiter);
        self.stats.total_waits += 1;
        true
    }

    #[inline]
    pub fn futex_wake(&mut self, addr: u64, count: usize) -> Vec<FutexWaiter> {
        let key = self.hash_addr(addr);
        if let Some(bucket) = self.buckets.get_mut(&key) {
            let woken = bucket.wake_n(count);
            self.stats.total_wakes += woken.len() as u64;
            woken
        } else {
            Vec::new()
        }
    }

    #[inline]
    pub fn futex_wake_bitset(&mut self, addr: u64, bitset: u32, max: usize) -> Vec<FutexWaiter> {
        let key = self.hash_addr(addr);
        if let Some(bucket) = self.buckets.get_mut(&key) {
            let woken = bucket.wake_bitset(bitset, max);
            self.stats.total_wakes += woken.len() as u64;
            woken
        } else {
            Vec::new()
        }
    }

    pub fn futex_requeue(
        &mut self,
        from_addr: u64,
        to_addr: u64,
        wake_count: usize,
        requeue_count: usize,
    ) -> (usize, usize) {
        let from_key = self.hash_addr(from_addr);
        let mut woken = Vec::new();
        let mut requeued = Vec::new();

        if let Some(from_bucket) = self.buckets.get_mut(&from_key) {
            woken = from_bucket.wake_n(wake_count);
            let rq_count = requeue_count.min(from_bucket.waiters.len());
            requeued = from_bucket.waiters.drain(..rq_count).collect();
        }

        let w = woken.len();
        let r = requeued.len();

        if !requeued.is_empty() {
            let to_bucket = self.get_or_create_bucket(to_addr);
            for waiter in requeued {
                to_bucket.enqueue(waiter);
            }
        }

        self.stats.total_wakes += w as u64;
        self.stats.total_requeues += r as u64;
        (w, r)
    }

    pub fn futex_lock_pi(&mut self, addr: u64, tid: u64, waiter: FutexWaiter) -> bool {
        let bucket = self.get_or_create_bucket(addr);
        if bucket.pi_owner.is_none() {
            bucket.pi_owner = Some(tid);
            return true;
        }
        // Priority inheritance boost
        let holder_tid = bucket.pi_owner.unwrap();
        if waiter.priority.as_value() < WaiterPriority::Normal(0).as_value() {
            let entry = PiChainEntry {
                holder_tid,
                waiter_tid: tid,
                original_priority: waiter.priority,
                boosted_priority: waiter.priority,
                futex_addr: addr,
            };
            self.pi_chains.push(entry);
            let depth = self
                .pi_chains
                .iter()
                .filter(|c| c.futex_addr == addr)
                .count() as u32;
            if depth > self.stats.pi_chains_max_depth {
                self.stats.pi_chains_max_depth = depth;
            }
            self.stats.pi_boosts += 1;
        }
        bucket.enqueue(waiter);
        self.stats.total_waits += 1;
        false
    }

    pub fn futex_unlock_pi(&mut self, addr: u64, tid: u64) -> Option<FutexWaiter> {
        let key = self.hash_addr(addr);
        if let Some(bucket) = self.buckets.get_mut(&key) {
            if bucket.pi_owner != Some(tid) {
                return None;
            }
            let next = bucket.wake_one();
            if let Some(ref w) = next {
                bucket.pi_owner = Some(w.tid);
            } else {
                bucket.pi_owner = None;
            }
            // Clean up PI chains
            self.pi_chains
                .retain(|c| c.futex_addr != addr || c.holder_tid != tid);
            self.stats.total_wakes += 1;
            next
        } else {
            None
        }
    }

    #[inline]
    pub fn futex_waitv(&mut self, entries: &[WaitvEntry], waiter: FutexWaiter) -> usize {
        let mut enqueued = 0;
        for entry in entries {
            let bucket = self.get_or_create_bucket(entry.address);
            bucket.enqueue(waiter.clone());
            enqueued += 1;
        }
        self.stats.waitv_operations += 1;
        self.stats.total_waits += enqueued as u64;
        enqueued
    }

    #[inline]
    pub fn expire_all(&mut self, now_ns: u64) -> Vec<FutexWaiter> {
        let mut all_expired = Vec::new();
        for bucket in self.buckets.values_mut() {
            let expired = bucket.expire_waiters(now_ns);
            all_expired.extend(expired);
        }
        self.stats.total_timeouts += all_expired.len() as u64;
        all_expired
    }

    #[inline]
    pub fn contention_hotspots(&self, top_n: usize) -> Vec<(u64, f64)> {
        let mut scores: Vec<(u64, f64)> = self
            .buckets
            .iter()
            .map(|(k, b)| (*k, b.contention_score()))
            .filter(|(_, s)| *s > 0.0)
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        scores.truncate(top_n);
        scores
    }

    #[inline(always)]
    pub fn stats(&self) -> &FutexV2Stats {
        &self.stats
    }
}

// ============================================================================
// Merged from futex_v3
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FutexV3Op {
    Wait,
    Wake,
    WaitBitset,
    WakeBitset,
    Requeue,
    CmpRequeue,
    WakeOp,
    LockPi,
    UnlockPi,
    TrylockPi,
    WaitRequeuePi,
}

/// Futex waiter state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FutexV3WaiterState {
    Waiting,
    Woken,
    TimedOut,
    Requeued,
    Interrupted,
}

/// A futex waiter entry
#[derive(Debug, Clone)]
pub struct FutexV3Waiter {
    pub thread_id: u64,
    pub state: FutexV3WaiterState,
    pub bitset: u32,
    pub priority: u32,
    pub wait_start_tick: u64,
    pub timeout_ns: u64,
}

/// A futex key identifying a specific futex
#[derive(Debug, Clone)]
pub struct FutexV3Key {
    pub address: u64,
    pub offset: u32,
    pub is_shared: bool,
    pub owner_pid: u64,
}

/// A futex instance with its waitqueue
#[derive(Debug, Clone)]
pub struct FutexV3Instance {
    pub key: FutexV3Key,
    pub waiters: Vec<FutexV3Waiter>,
    pub pi_owner: Option<u64>,
    pub value: u32,
    pub contention_count: u64,
    pub wake_count: u64,
}

impl FutexV3Instance {
    pub fn new(key: FutexV3Key, value: u32) -> Self {
        Self {
            key,
            waiters: Vec::new(),
            pi_owner: None,
            value,
            contention_count: 0,
            wake_count: 0,
        }
    }

    pub fn add_waiter(
        &mut self,
        thread_id: u64,
        bitset: u32,
        priority: u32,
        tick: u64,
        timeout: u64,
    ) {
        let waiter = FutexV3Waiter {
            thread_id,
            state: FutexV3WaiterState::Waiting,
            bitset,
            priority,
            wait_start_tick: tick,
            timeout_ns: timeout,
        };
        self.waiters.push(waiter);
        self.contention_count += 1;
    }

    pub fn wake(&mut self, count: usize, bitset: u32) -> u64 {
        let mut woken = 0u64;
        for w in self.waiters.iter_mut() {
            if w.state == FutexV3WaiterState::Waiting && (w.bitset & bitset) != 0 {
                w.state = FutexV3WaiterState::Woken;
                woken += 1;
                if woken >= count as u64 {
                    break;
                }
            }
        }
        self.wake_count += woken;
        woken
    }

    pub fn requeue(
        &mut self,
        target: &mut FutexV3Instance,
        wake_count: usize,
        requeue_count: usize,
    ) -> (u64, u64) {
        let mut woken = 0u64;
        let mut requeued = 0u64;
        let mut to_requeue = Vec::new();

        for w in self.waiters.iter_mut() {
            if w.state != FutexV3WaiterState::Waiting {
                continue;
            }
            if (woken as usize) < wake_count {
                w.state = FutexV3WaiterState::Woken;
                woken += 1;
            } else if (requeued as usize) < requeue_count {
                w.state = FutexV3WaiterState::Requeued;
                to_requeue.push(w.clone());
                requeued += 1;
            }
        }

        for mut w in to_requeue {
            w.state = FutexV3WaiterState::Waiting;
            target.waiters.push(w);
        }

        self.waiters
            .retain(|w| w.state == FutexV3WaiterState::Waiting);
        (woken, requeued)
    }

    #[inline(always)]
    pub fn waiting_count(&self) -> usize {
        self.waiters
            .iter()
            .filter(|w| w.state == FutexV3WaiterState::Waiting)
            .count()
    }
}

/// Futex V3 bridge statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FutexV3Stats {
    pub total_waits: u64,
    pub total_wakes: u64,
    pub total_requeues: u64,
    pub pi_operations: u64,
    pub timeouts: u64,
    pub contention_events: u64,
    pub active_futexes: u64,
}

/// Main futex V3 bridge manager
#[derive(Debug)]
#[repr(align(64))]
pub struct BridgeFutexV3 {
    futexes: BTreeMap<u64, FutexV3Instance>,
    stats: FutexV3Stats,
}

impl BridgeFutexV3 {
    pub fn new() -> Self {
        Self {
            futexes: BTreeMap::new(),
            stats: FutexV3Stats {
                total_waits: 0,
                total_wakes: 0,
                total_requeues: 0,
                pi_operations: 0,
                timeouts: 0,
                contention_events: 0,
                active_futexes: 0,
            },
        }
    }

    #[inline]
    pub fn get_or_create(&mut self, addr: u64, value: u32, pid: u64) -> &mut FutexV3Instance {
        if !self.futexes.contains_key(&addr) {
            let key = FutexV3Key {
                address: addr,
                offset: 0,
                is_shared: false,
                owner_pid: pid,
            };
            self.futexes.insert(addr, FutexV3Instance::new(key, value));
            self.stats.active_futexes += 1;
        }
        self.futexes.get_mut(&addr).unwrap()
    }

    #[inline]
    pub fn wait(
        &mut self,
        addr: u64,
        expected: u32,
        tid: u64,
        bitset: u32,
        priority: u32,
        tick: u64,
        timeout: u64,
        pid: u64,
    ) -> bool {
        let futex = self.get_or_create(addr, expected, pid);
        if futex.value != expected {
            return false;
        }
        futex.add_waiter(tid, bitset, priority, tick, timeout);
        self.stats.total_waits += 1;
        true
    }

    #[inline]
    pub fn wake(&mut self, addr: u64, count: usize, bitset: u32) -> u64 {
        if let Some(futex) = self.futexes.get_mut(&addr) {
            let woken = futex.wake(count, bitset);
            self.stats.total_wakes += woken;
            woken
        } else {
            0
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &FutexV3Stats {
        &self.stats
    }
}
