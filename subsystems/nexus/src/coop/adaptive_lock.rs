// SPDX-License-Identifier: GPL-2.0
//! Coop adaptive_lock — adaptive lock that switches between spin and sleep.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Lock strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockStrategy {
    SpinOnly,
    SleepOnly,
    Adaptive,
    Hybrid,
}

/// Lock contention level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentionLevel {
    None,
    Low,
    Medium,
    High,
    Extreme,
}

/// Adaptive lock state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdaptiveLockState {
    Unlocked,
    SpinLocked,
    SleepLocked,
    Upgrading,
}

/// Spin parameters
#[derive(Debug, Clone, Copy)]
pub struct SpinParams {
    pub max_spins: u32,
    pub backoff_base: u32,
    pub backoff_max: u32,
    pub yield_after: u32,
}

impl SpinParams {
    pub fn default_params() -> Self {
        Self { max_spins: 1000, backoff_base: 1, backoff_max: 128, yield_after: 500 }
    }

    pub fn aggressive() -> Self {
        Self { max_spins: 5000, backoff_base: 1, backoff_max: 64, yield_after: 2000 }
    }

    pub fn conservative() -> Self {
        Self { max_spins: 100, backoff_base: 4, backoff_max: 256, yield_after: 50 }
    }
}

/// Lock instance
#[derive(Debug)]
pub struct AdaptiveLockInstance {
    pub id: u64,
    pub state: AdaptiveLockState,
    pub strategy: LockStrategy,
    pub spin_params: SpinParams,
    pub owner: Option<u32>,
    pub waiters: Vec<u32>,
    pub acquire_count: u64,
    pub spin_count: u64,
    pub sleep_count: u64,
    pub contention_events: u64,
    pub total_hold_ns: u64,
    pub total_wait_ns: u64,
    pub max_hold_ns: u64,
    pub max_wait_ns: u64,
    pub last_acquire: u64,
    pub adapt_threshold: u32,
}

impl AdaptiveLockInstance {
    pub fn new(id: u64, strategy: LockStrategy) -> Self {
        Self {
            id, state: AdaptiveLockState::Unlocked, strategy,
            spin_params: SpinParams::default_params(),
            owner: None, waiters: Vec::new(),
            acquire_count: 0, spin_count: 0, sleep_count: 0,
            contention_events: 0, total_hold_ns: 0, total_wait_ns: 0,
            max_hold_ns: 0, max_wait_ns: 0, last_acquire: 0,
            adapt_threshold: 500,
        }
    }

    pub fn try_acquire(&mut self, tid: u32, now: u64) -> bool {
        if self.state == AdaptiveLockState::Unlocked {
            self.state = AdaptiveLockState::SpinLocked;
            self.owner = Some(tid);
            self.acquire_count += 1;
            self.last_acquire = now;
            return true;
        }
        self.contention_events += 1;
        false
    }

    pub fn release(&mut self, now: u64) -> Option<u32> {
        let hold_time = now.saturating_sub(self.last_acquire);
        self.total_hold_ns += hold_time;
        if hold_time > self.max_hold_ns { self.max_hold_ns = hold_time; }
        self.owner = None;
        self.state = AdaptiveLockState::Unlocked;

        // Wake first waiter
        if !self.waiters.is_empty() {
            Some(self.waiters.remove(0))
        } else { None }
    }

    pub fn add_waiter(&mut self, tid: u32) {
        if !self.waiters.contains(&tid) {
            self.waiters.push(tid);
        }
    }

    pub fn should_spin(&self) -> bool {
        match self.strategy {
            LockStrategy::SpinOnly => true,
            LockStrategy::SleepOnly => false,
            LockStrategy::Adaptive | LockStrategy::Hybrid => {
                self.contention_events < self.adapt_threshold as u64
            }
        }
    }

    pub fn contention_level(&self) -> ContentionLevel {
        if self.contention_events == 0 { return ContentionLevel::None; }
        let rate = if self.acquire_count > 0 {
            self.contention_events as f64 / self.acquire_count as f64
        } else { 0.0 };
        if rate < 0.05 { ContentionLevel::Low }
        else if rate < 0.20 { ContentionLevel::Medium }
        else if rate < 0.50 { ContentionLevel::High }
        else { ContentionLevel::Extreme }
    }

    pub fn avg_hold_ns(&self) -> u64 {
        if self.acquire_count == 0 { 0 } else { self.total_hold_ns / self.acquire_count }
    }

    pub fn avg_wait_ns(&self) -> u64 {
        let total_waits = self.spin_count + self.sleep_count;
        if total_waits == 0 { 0 } else { self.total_wait_ns / total_waits }
    }

    pub fn adapt_strategy(&mut self) {
        if self.strategy != LockStrategy::Adaptive { return; }
        let level = self.contention_level();
        match level {
            ContentionLevel::None | ContentionLevel::Low => {
                self.spin_params = SpinParams::aggressive();
            }
            ContentionLevel::Medium => {
                self.spin_params = SpinParams::default_params();
            }
            ContentionLevel::High | ContentionLevel::Extreme => {
                self.spin_params = SpinParams::conservative();
            }
        }
    }
}

/// Adaptive lock stats
#[derive(Debug, Clone)]
pub struct AdaptiveLockStats {
    pub total_locks: u32,
    pub total_acquires: u64,
    pub total_contentions: u64,
    pub spin_acquires: u64,
    pub sleep_acquires: u64,
    pub avg_hold_ns: u64,
}

/// Main adaptive lock manager
pub struct CoopAdaptiveLock {
    locks: BTreeMap<u64, AdaptiveLockInstance>,
    next_id: u64,
    total_acquires: u64,
    total_contentions: u64,
}

impl CoopAdaptiveLock {
    pub fn new() -> Self {
        Self { locks: BTreeMap::new(), next_id: 1, total_acquires: 0, total_contentions: 0 }
    }

    pub fn create_lock(&mut self, strategy: LockStrategy) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.locks.insert(id, AdaptiveLockInstance::new(id, strategy));
        id
    }

    pub fn try_acquire(&mut self, lock_id: u64, tid: u32, now: u64) -> bool {
        if let Some(lock) = self.locks.get_mut(&lock_id) {
            let acquired = lock.try_acquire(tid, now);
            if acquired { self.total_acquires += 1; }
            else { self.total_contentions += 1; }
            acquired
        } else { false }
    }

    pub fn release(&mut self, lock_id: u64, now: u64) -> Option<u32> {
        self.locks.get_mut(&lock_id)?.release(now)
    }

    pub fn destroy_lock(&mut self, lock_id: u64) -> bool {
        self.locks.remove(&lock_id).is_some()
    }

    pub fn hottest_locks(&self, n: usize) -> Vec<(u64, u64)> {
        let mut v: Vec<_> = self.locks.iter()
            .map(|(&id, l)| (id, l.contention_events))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(n);
        v
    }

    pub fn stats(&self) -> AdaptiveLockStats {
        let total_hold: u64 = self.locks.values().map(|l| l.total_hold_ns).sum();
        let total_acq: u64 = self.locks.values().map(|l| l.acquire_count).sum();
        AdaptiveLockStats {
            total_locks: self.locks.len() as u32,
            total_acquires: self.total_acquires,
            total_contentions: self.total_contentions,
            spin_acquires: self.locks.values().map(|l| l.spin_count).sum(),
            sleep_acquires: self.locks.values().map(|l| l.sleep_count).sum(),
            avg_hold_ns: if total_acq > 0 { total_hold / total_acq } else { 0 },
        }
    }
}

// ============================================================================
// Merged from adaptive_lock_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockStrategyV2 {
    SpinOnly,
    YieldOnly,
    SpinThenYield,
    BackoffSpin,
    Adaptive,
}

/// Lock contention level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentionLevelV2 {
    None,
    Low,
    Medium,
    High,
    Extreme,
}

/// Adaptive lock entry v2
#[derive(Debug)]
pub struct AdaptiveLockEntryV2 {
    pub id: u64,
    pub strategy: LockStrategyV2,
    pub contention: ContentionLevelV2,
    pub spin_count: u64,
    pub yield_count: u64,
    pub acquire_count: u64,
    pub total_wait_ns: u64,
    pub max_wait_ns: u64,
    pub spin_threshold: u32,
    pub adapt_interval: u32,
}

impl AdaptiveLockEntryV2 {
    pub fn new(id: u64) -> Self {
        Self { id, strategy: LockStrategyV2::Adaptive, contention: ContentionLevelV2::None, spin_count: 0, yield_count: 0, acquire_count: 0, total_wait_ns: 0, max_wait_ns: 0, spin_threshold: 100, adapt_interval: 64 }
    }

    pub fn record_acquire(&mut self, wait_ns: u64, spins: u64, yields: u64) {
        self.acquire_count += 1;
        self.spin_count += spins;
        self.yield_count += yields;
        self.total_wait_ns += wait_ns;
        if wait_ns > self.max_wait_ns { self.max_wait_ns = wait_ns; }
        if self.acquire_count % self.adapt_interval as u64 == 0 { self.adapt(); }
    }

    fn adapt(&mut self) {
        let avg = if self.acquire_count == 0 { 0 } else { self.total_wait_ns / self.acquire_count };
        if avg < 100 { self.contention = ContentionLevelV2::None; self.strategy = LockStrategyV2::SpinOnly; }
        else if avg < 1000 { self.contention = ContentionLevelV2::Low; self.strategy = LockStrategyV2::SpinThenYield; }
        else if avg < 10000 { self.contention = ContentionLevelV2::Medium; self.strategy = LockStrategyV2::Adaptive; }
        else if avg < 100000 { self.contention = ContentionLevelV2::High; self.strategy = LockStrategyV2::YieldOnly; }
        else { self.contention = ContentionLevelV2::Extreme; self.strategy = LockStrategyV2::BackoffSpin; }
    }

    pub fn avg_wait_ns(&self) -> u64 {
        if self.acquire_count == 0 { 0 } else { self.total_wait_ns / self.acquire_count }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct AdaptiveLockV2Stats {
    pub total_locks: u32,
    pub total_acquires: u64,
    pub avg_wait_ns: u64,
    pub high_contention: u32,
}

/// Main coop adaptive lock v2
pub struct CoopAdaptiveLockV2 {
    locks: BTreeMap<u64, AdaptiveLockEntryV2>,
}

impl CoopAdaptiveLockV2 {
    pub fn new() -> Self { Self { locks: BTreeMap::new() } }

    pub fn register(&mut self, id: u64) { self.locks.insert(id, AdaptiveLockEntryV2::new(id)); }

    pub fn record_acquire(&mut self, id: u64, wait_ns: u64, spins: u64, yields: u64) {
        if let Some(l) = self.locks.get_mut(&id) { l.record_acquire(wait_ns, spins, yields); }
    }

    pub fn unregister(&mut self, id: u64) { self.locks.remove(&id); }

    pub fn stats(&self) -> AdaptiveLockV2Stats {
        let acqs: u64 = self.locks.values().map(|l| l.acquire_count).sum();
        let wait: u64 = self.locks.values().map(|l| l.total_wait_ns).sum();
        let high = self.locks.values().filter(|l| matches!(l.contention, ContentionLevelV2::High | ContentionLevelV2::Extreme)).count() as u32;
        let avg = if acqs == 0 { 0 } else { wait / acqs };
        AdaptiveLockV2Stats { total_locks: self.locks.len() as u32, total_acquires: acqs, avg_wait_ns: avg, high_contention: high }
    }
}
