// SPDX-License-Identifier: GPL-2.0
//! Coop convoy — lock convoy detection and mitigation.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Convoy severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvoySeverity {
    None,
    Mild,
    Moderate,
    Severe,
    Critical,
}

/// Convoy detector entry
#[derive(Debug)]
pub struct ConvoyEntry {
    pub lock_id: u64,
    pub waiter_count: u32,
    pub max_wait_ns: u64,
    pub total_wait_ns: u64,
    pub acquire_count: u64,
    pub convoy_detected: bool,
    pub severity: ConvoySeverity,
}

impl ConvoyEntry {
    pub fn new(id: u64) -> Self {
        Self { lock_id: id, waiter_count: 0, max_wait_ns: 0, total_wait_ns: 0, acquire_count: 0, convoy_detected: false, severity: ConvoySeverity::None }
    }

    #[inline]
    pub fn record_acquire(&mut self, wait_ns: u64) {
        self.acquire_count += 1;
        self.total_wait_ns += wait_ns;
        if wait_ns > self.max_wait_ns { self.max_wait_ns = wait_ns; }
    }

    pub fn check_convoy(&mut self, threshold_waiters: u32, threshold_ns: u64) {
        if self.waiter_count >= threshold_waiters && self.max_wait_ns >= threshold_ns {
            self.convoy_detected = true;
            self.severity = if self.waiter_count >= threshold_waiters * 4 { ConvoySeverity::Critical }
                else if self.waiter_count >= threshold_waiters * 3 { ConvoySeverity::Severe }
                else if self.waiter_count >= threshold_waiters * 2 { ConvoySeverity::Moderate }
                else { ConvoySeverity::Mild };
        } else {
            self.convoy_detected = false;
            self.severity = ConvoySeverity::None;
        }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ConvoyStats {
    pub monitored_locks: u32,
    pub convoys_detected: u32,
    pub critical_convoys: u32,
    pub total_wait_ns: u64,
}

/// Main coop convoy detector
pub struct CoopConvoy {
    locks: BTreeMap<u64, ConvoyEntry>,
    waiter_threshold: u32,
    wait_threshold_ns: u64,
}

impl CoopConvoy {
    pub fn new(waiter_thresh: u32, wait_thresh: u64) -> Self {
        Self { locks: BTreeMap::new(), waiter_threshold: waiter_thresh, wait_threshold_ns: wait_thresh }
    }

    #[inline(always)]
    pub fn monitor(&mut self, lock_id: u64) { self.locks.insert(lock_id, ConvoyEntry::new(lock_id)); }

    #[inline(always)]
    pub fn record_acquire(&mut self, lock_id: u64, wait_ns: u64) {
        if let Some(e) = self.locks.get_mut(&lock_id) { e.record_acquire(wait_ns); }
    }

    #[inline(always)]
    pub fn update_waiters(&mut self, lock_id: u64, count: u32) {
        if let Some(e) = self.locks.get_mut(&lock_id) { e.waiter_count = count; }
    }

    #[inline]
    pub fn check_all(&mut self) {
        let wt = self.waiter_threshold;
        let nt = self.wait_threshold_ns;
        for e in self.locks.values_mut() { e.check_convoy(wt, nt); }
    }

    #[inline]
    pub fn stats(&self) -> ConvoyStats {
        let det = self.locks.values().filter(|e| e.convoy_detected).count() as u32;
        let crit = self.locks.values().filter(|e| e.severity == ConvoySeverity::Critical).count() as u32;
        let wait: u64 = self.locks.values().map(|e| e.total_wait_ns).sum();
        ConvoyStats { monitored_locks: self.locks.len() as u32, convoys_detected: det, critical_convoys: crit, total_wait_ns: wait }
    }
}
