// SPDX-License-Identifier: GPL-2.0
//! Coop split_lock — split lock detection and mitigation.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Split lock action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitLockAction {
    Warn,
    Throttle,
    Kill,
    Emulate,
    Ignore,
}

/// Lock alignment issue
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignmentIssue {
    CacheSplit,
    PageSplit,
    Misaligned,
    UnalignedAtomic,
}

/// Split lock event
#[derive(Debug, Clone)]
pub struct SplitLockEvent {
    pub tid: u64,
    pub addr: u64,
    pub size: u32,
    pub issue: AlignmentIssue,
    pub ip: u64,
    pub timestamp: u64,
    pub bus_lock_cycles: u64,
}

impl SplitLockEvent {
    pub fn new(tid: u64, addr: u64, size: u32, issue: AlignmentIssue, ip: u64, now: u64) -> Self {
        Self { tid, addr, size, issue, ip, timestamp: now, bus_lock_cycles: 0 }
    }

    #[inline]
    pub fn is_cacheline_split(&self) -> bool {
        let cl_start = self.addr & !63;
        let end = self.addr + self.size as u64;
        end > cl_start + 64
    }
}

/// Per-thread tracking
#[derive(Debug)]
#[repr(align(64))]
pub struct ThreadSplitLockState {
    pub tid: u64,
    pub events: u64,
    pub throttle_count: u64,
    pub last_event: u64,
    pub hotspot_ips: LinearMap<u64, 64>,
}

impl ThreadSplitLockState {
    pub fn new(tid: u64) -> Self {
        Self { tid, events: 0, throttle_count: 0, last_event: 0, hotspot_ips: LinearMap::new() }
    }

    #[inline]
    pub fn record(&mut self, ip: u64, now: u64) {
        self.events += 1;
        self.last_event = now;
        self.hotspot_ips.add(ip, 1);
    }

    #[inline(always)]
    pub fn top_hotspot(&self) -> Option<(u64, u64)> {
        self.hotspot_ips.iter().max_by_key(|&(_, &count)| count).map(|(&ip, &count)| (ip, count))
    }

    #[inline(always)]
    pub fn event_rate(&self, window_ns: u64, now: u64) -> f64 {
        if now.saturating_sub(self.last_event) > window_ns { return 0.0; }
        self.events as f64
    }
}

/// Global policy
#[derive(Debug, Clone)]
pub struct SplitLockPolicy {
    pub action: SplitLockAction,
    pub threshold_per_sec: u64,
    pub throttle_ns: u64,
    pub exempt_tids: Vec<u64>,
}

impl SplitLockPolicy {
    #[inline(always)]
    pub fn default_policy() -> Self {
        Self { action: SplitLockAction::Throttle, threshold_per_sec: 100, throttle_ns: 1_000_000, exempt_tids: Vec::new() }
    }

    #[inline(always)]
    pub fn is_exempt(&self, tid: u64) -> bool { self.exempt_tids.contains(&tid) }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SplitLockStats {
    pub total_events: u64,
    pub threads_affected: u32,
    pub throttle_count: u64,
    pub kill_count: u64,
    pub cacheline_splits: u64,
    pub page_splits: u64,
}

/// Main split lock manager
pub struct CoopSplitLock {
    threads: BTreeMap<u64, ThreadSplitLockState>,
    events: Vec<SplitLockEvent>,
    policy: SplitLockPolicy,
    max_events: usize,
    kill_count: u64,
}

impl CoopSplitLock {
    pub fn new() -> Self {
        Self {
            threads: BTreeMap::new(), events: Vec::new(),
            policy: SplitLockPolicy::default_policy(), max_events: 4096, kill_count: 0,
        }
    }

    #[inline(always)]
    pub fn set_policy(&mut self, policy: SplitLockPolicy) { self.policy = policy; }

    pub fn report(&mut self, event: SplitLockEvent) -> SplitLockAction {
        let tid = event.tid;
        let ip = event.ip;
        let now = event.timestamp;

        let state = self.threads.entry(tid).or_insert_with(|| ThreadSplitLockState::new(tid));
        state.record(ip, now);

        if self.events.len() >= self.max_events { self.events.drain(..self.max_events / 4); }
        self.events.push(event);

        if self.policy.is_exempt(tid) { return SplitLockAction::Ignore; }

        match self.policy.action {
            SplitLockAction::Kill if state.events > self.policy.threshold_per_sec * 10 => {
                self.kill_count += 1;
                SplitLockAction::Kill
            }
            SplitLockAction::Throttle if state.events > self.policy.threshold_per_sec => {
                state.throttle_count += 1;
                SplitLockAction::Throttle
            }
            _ => self.policy.action,
        }
    }

    #[inline]
    pub fn stats(&self) -> SplitLockStats {
        let total: u64 = self.threads.values().map(|t| t.events).sum();
        let throttled: u64 = self.threads.values().map(|t| t.throttle_count).sum();
        let cl = self.events.iter().filter(|e| e.issue == AlignmentIssue::CacheSplit).count() as u64;
        let pg = self.events.iter().filter(|e| e.issue == AlignmentIssue::PageSplit).count() as u64;
        SplitLockStats {
            total_events: total, threads_affected: self.threads.len() as u32,
            throttle_count: throttled, kill_count: self.kill_count,
            cacheline_splits: cl, page_splits: pg,
        }
    }
}
