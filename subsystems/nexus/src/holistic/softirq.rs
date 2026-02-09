// SPDX-License-Identifier: GPL-2.0
//! Holistic softirq — software interrupt handling management.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Softirq vector
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoftirqVec {
    HiTasklet,
    Timer,
    NetTx,
    NetRx,
    Block,
    IrqPoll,
    Tasklet,
    Sched,
    HrTimer,
    Rcu,
}

/// Softirq entry
#[derive(Debug)]
pub struct SoftirqEntry {
    pub vec: SoftirqVec,
    pub cpu: u32,
    pub pending: bool,
    pub total_raised: u64,
    pub total_handled: u64,
    pub total_ns: u64,
    pub max_ns: u64,
    pub ksoftirqd_wakeups: u64,
}

impl SoftirqEntry {
    pub fn new(vec: SoftirqVec, cpu: u32) -> Self {
        Self { vec, cpu, pending: false, total_raised: 0, total_handled: 0, total_ns: 0, max_ns: 0, ksoftirqd_wakeups: 0 }
    }

    #[inline(always)]
    pub fn raise(&mut self) {
        self.pending = true;
        self.total_raised += 1;
    }

    #[inline]
    pub fn handle(&mut self, ns: u64) {
        self.pending = false;
        self.total_handled += 1;
        self.total_ns += ns;
        if ns > self.max_ns { self.max_ns = ns; }
    }

    #[inline(always)]
    pub fn avg_ns(&self) -> u64 {
        if self.total_handled == 0 { 0 } else { self.total_ns / self.total_handled }
    }
}

fn softirq_key(vec: SoftirqVec, cpu: u32) -> u64 {
    ((vec as u64) << 32) | cpu as u64
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SoftirqStats {
    pub total_entries: u32,
    pub total_raised: u64,
    pub total_handled: u64,
    pub pending_count: u32,
    pub avg_handler_ns: u64,
    pub ksoftirqd_wakeups: u64,
}

/// Main holistic softirq manager
pub struct HolisticSoftirq {
    entries: BTreeMap<u64, SoftirqEntry>,
}

impl HolisticSoftirq {
    pub fn new() -> Self { Self { entries: BTreeMap::new() } }

    #[inline(always)]
    pub fn register(&mut self, vec: SoftirqVec, cpu: u32) {
        let key = softirq_key(vec, cpu);
        self.entries.insert(key, SoftirqEntry::new(vec, cpu));
    }

    #[inline(always)]
    pub fn raise(&mut self, vec: SoftirqVec, cpu: u32) {
        let key = softirq_key(vec, cpu);
        if let Some(e) = self.entries.get_mut(&key) { e.raise(); }
    }

    #[inline(always)]
    pub fn handle(&mut self, vec: SoftirqVec, cpu: u32, ns: u64) {
        let key = softirq_key(vec, cpu);
        if let Some(e) = self.entries.get_mut(&key) { e.handle(ns); }
    }

    #[inline(always)]
    pub fn record_ksoftirqd(&mut self, vec: SoftirqVec, cpu: u32) {
        let key = softirq_key(vec, cpu);
        if let Some(e) = self.entries.get_mut(&key) { e.ksoftirqd_wakeups += 1; }
    }

    #[inline]
    pub fn stats(&self) -> SoftirqStats {
        let raised: u64 = self.entries.values().map(|e| e.total_raised).sum();
        let handled: u64 = self.entries.values().map(|e| e.total_handled).sum();
        let pending = self.entries.values().filter(|e| e.pending).count() as u32;
        let ns: u64 = self.entries.values().map(|e| e.total_ns).sum();
        let wakeups: u64 = self.entries.values().map(|e| e.ksoftirqd_wakeups).sum();
        let avg = if handled == 0 { 0 } else { ns / handled };
        SoftirqStats { total_entries: self.entries.len() as u32, total_raised: raised, total_handled: handled, pending_count: pending, avg_handler_ns: avg, ksoftirqd_wakeups: wakeups }
    }
}
