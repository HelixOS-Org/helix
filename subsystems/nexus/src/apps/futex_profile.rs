//! # Application Futex Profiler
//!
//! Per-process futex contention analysis:
//! - Futex wait/wake tracking
//! - Contention hotspot detection
//! - Thundering herd detection
//! - Waiter chain analysis
//! - Priority-inheritance correlation
//! - Spin-then-sleep efficiency analysis

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Futex operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FutexOpType {
    Wait,
    Wake,
    WakeOp,
    Requeue,
    CmpRequeue,
    LockPi,
    UnlockPi,
    WaitBitset,
    WakeBitset,
}

/// Futex contention level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FutexContentionLevel {
    None,
    Low,
    Moderate,
    High,
    Severe,
}

/// Single futex address stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FutexAddrStats {
    pub addr: u64,
    pub wait_count: u64,
    pub wake_count: u64,
    pub requeue_count: u64,
    pub total_wait_ns: u64,
    pub max_wait_ns: u64,
    pub max_waiters: u32,
    pub current_waiters: u32,
    pub thundering_herd_events: u64,
    pub spin_attempts: u64,
    pub spin_successes: u64,
    pub pi_boosts: u64,
}

impl FutexAddrStats {
    pub fn new(addr: u64) -> Self {
        Self {
            addr,
            wait_count: 0,
            wake_count: 0,
            requeue_count: 0,
            total_wait_ns: 0,
            max_wait_ns: 0,
            max_waiters: 0,
            current_waiters: 0,
            thundering_herd_events: 0,
            spin_attempts: 0,
            spin_successes: 0,
            pi_boosts: 0,
        }
    }

    #[inline(always)]
    pub fn avg_wait_ns(&self) -> u64 {
        if self.wait_count == 0 { return 0; }
        self.total_wait_ns / self.wait_count
    }

    #[inline]
    pub fn contention_level(&self) -> FutexContentionLevel {
        if self.wait_count < 10 { return FutexContentionLevel::None; }
        let avg = self.avg_wait_ns();
        if avg < 10_000 { FutexContentionLevel::Low }
        else if avg < 100_000 { FutexContentionLevel::Moderate }
        else if avg < 1_000_000 { FutexContentionLevel::High }
        else { FutexContentionLevel::Severe }
    }

    #[inline(always)]
    pub fn spin_efficiency(&self) -> f64 {
        if self.spin_attempts == 0 { return 0.0; }
        self.spin_successes as f64 / self.spin_attempts as f64
    }

    #[inline]
    pub fn record_wait(&mut self, duration_ns: u64) {
        self.wait_count += 1;
        self.total_wait_ns += duration_ns;
        if duration_ns > self.max_wait_ns { self.max_wait_ns = duration_ns; }
        self.current_waiters += 1;
        if self.current_waiters > self.max_waiters { self.max_waiters = self.current_waiters; }
    }

    #[inline]
    pub fn record_wake(&mut self, count: u32) {
        self.wake_count += 1;
        self.current_waiters = self.current_waiters.saturating_sub(count);
        // Thundering herd: waking many waiters at once
        if count > 4 {
            self.thundering_herd_events += 1;
        }
    }
}

/// Waiter chain link
#[derive(Debug, Clone)]
pub struct WaiterChainLink {
    pub thread_id: u64,
    pub futex_addr: u64,
    pub waiting_for: Option<u64>, // thread holding the lock
    pub depth: u32,
}

/// Per-process futex profile
#[derive(Debug, Clone)]
pub struct ProcessFutexProfile {
    pub pid: u64,
    pub futex_stats: BTreeMap<u64, FutexAddrStats>,
    pub total_waits: u64,
    pub total_wakes: u64,
    pub total_wait_ns: u64,
    pub total_thundering_herds: u64,
    pub waiter_chains: Vec<WaiterChainLink>,
    pub max_chain_depth: u32,
}

impl ProcessFutexProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            futex_stats: BTreeMap::new(),
            total_waits: 0,
            total_wakes: 0,
            total_wait_ns: 0,
            total_thundering_herds: 0,
            waiter_chains: Vec::new(),
            max_chain_depth: 0,
        }
    }

    #[inline]
    pub fn record_wait(&mut self, addr: u64, duration_ns: u64) {
        let stats = self.futex_stats.entry(addr)
            .or_insert_with(|| FutexAddrStats::new(addr));
        stats.record_wait(duration_ns);
        self.total_waits += 1;
        self.total_wait_ns += duration_ns;
    }

    #[inline]
    pub fn record_wake(&mut self, addr: u64, count: u32) {
        let stats = self.futex_stats.entry(addr)
            .or_insert_with(|| FutexAddrStats::new(addr));
        stats.record_wake(count);
        self.total_wakes += 1;
        if count > 4 { self.total_thundering_herds += 1; }
    }

    #[inline]
    pub fn record_spin(&mut self, addr: u64, success: bool) {
        let stats = self.futex_stats.entry(addr)
            .or_insert_with(|| FutexAddrStats::new(addr));
        stats.spin_attempts += 1;
        if success { stats.spin_successes += 1; }
    }

    #[inline]
    pub fn hottest_futexes(&self, n: usize) -> Vec<&FutexAddrStats> {
        let mut sorted: Vec<_> = self.futex_stats.values().collect();
        sorted.sort_by(|a, b| b.total_wait_ns.cmp(&a.total_wait_ns));
        sorted.truncate(n);
        sorted
    }

    #[inline]
    pub fn severe_contention_addrs(&self) -> Vec<u64> {
        self.futex_stats.values()
            .filter(|s| s.contention_level() == FutexContentionLevel::Severe)
            .map(|s| s.addr)
            .collect()
    }

    #[inline]
    pub fn update_chain(&mut self, chain: Vec<WaiterChainLink>) {
        let depth = chain.iter().map(|l| l.depth).max().unwrap_or(0);
        if depth > self.max_chain_depth { self.max_chain_depth = depth; }
        self.waiter_chains = chain;
    }
}

/// App futex profiler stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppFutexProfilerStats {
    pub total_processes: usize,
    pub total_futexes: usize,
    pub total_waits: u64,
    pub total_wakes: u64,
    pub total_wait_ns: u64,
    pub severe_contention_count: usize,
    pub thundering_herd_count: u64,
    pub max_chain_depth: u32,
}

/// Application Futex Profiler
pub struct AppFutexProfiler {
    profiles: BTreeMap<u64, ProcessFutexProfile>,
    stats: AppFutexProfilerStats,
}

impl AppFutexProfiler {
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            stats: AppFutexProfilerStats::default(),
        }
    }

    #[inline(always)]
    pub fn register_process(&mut self, pid: u64) {
        self.profiles.entry(pid).or_insert_with(|| ProcessFutexProfile::new(pid));
    }

    #[inline]
    pub fn record_wait(&mut self, pid: u64, addr: u64, duration_ns: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.record_wait(addr, duration_ns);
        }
    }

    #[inline]
    pub fn record_wake(&mut self, pid: u64, addr: u64, count: u32) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.record_wake(addr, count);
        }
    }

    #[inline]
    pub fn record_spin(&mut self, pid: u64, addr: u64, success: bool) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.record_spin(addr, success);
        }
    }

    pub fn recompute(&mut self) {
        self.stats.total_processes = self.profiles.len();
        self.stats.total_futexes = self.profiles.values().map(|p| p.futex_stats.len()).sum();
        self.stats.total_waits = self.profiles.values().map(|p| p.total_waits).sum();
        self.stats.total_wakes = self.profiles.values().map(|p| p.total_wakes).sum();
        self.stats.total_wait_ns = self.profiles.values().map(|p| p.total_wait_ns).sum();
        self.stats.thundering_herd_count = self.profiles.values()
            .map(|p| p.total_thundering_herds).sum();
        self.stats.severe_contention_count = self.profiles.values()
            .flat_map(|p| p.futex_stats.values())
            .filter(|s| s.contention_level() == FutexContentionLevel::Severe)
            .count();
        self.stats.max_chain_depth = self.profiles.values()
            .map(|p| p.max_chain_depth)
            .max()
            .unwrap_or(0);
    }

    #[inline(always)]
    pub fn profile(&self, pid: u64) -> Option<&ProcessFutexProfile> {
        self.profiles.get(&pid)
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppFutexProfilerStats {
        &self.stats
    }

    #[inline(always)]
    pub fn remove_process(&mut self, pid: u64) {
        self.profiles.remove(&pid);
        self.recompute();
    }
}
