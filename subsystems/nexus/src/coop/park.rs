// SPDX-License-Identifier: GPL-2.0
//! Coop park — thread parking mechanism for blocking waits.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Park state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParkState {
    Running,
    Parked,
    Notified,
    TimedOut,
}

/// Parked thread entry
#[derive(Debug)]
pub struct ParkedThread {
    pub tid: u64,
    pub state: ParkState,
    pub parked_at: u64,
    pub unparked_at: u64,
    pub timeout_ns: u64,
    pub park_count: u64,
    pub total_park_ns: u64,
}

impl ParkedThread {
    pub fn new(tid: u64) -> Self {
        Self { tid, state: ParkState::Running, parked_at: 0, unparked_at: 0, timeout_ns: 0, park_count: 0, total_park_ns: 0 }
    }

    pub fn park(&mut self, now: u64, timeout: u64) {
        self.state = ParkState::Parked;
        self.parked_at = now;
        self.timeout_ns = timeout;
        self.park_count += 1;
    }

    pub fn unpark(&mut self, now: u64) {
        if self.state == ParkState::Parked {
            self.state = ParkState::Notified;
            self.unparked_at = now;
            self.total_park_ns += now - self.parked_at;
        }
    }

    pub fn check_timeout(&mut self, now: u64) -> bool {
        if self.state == ParkState::Parked && self.timeout_ns > 0 && now - self.parked_at >= self.timeout_ns {
            self.state = ParkState::TimedOut;
            self.unparked_at = now;
            self.total_park_ns += now - self.parked_at;
            true
        } else { false }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct ParkStats {
    pub tracked_threads: u32,
    pub parked_count: u32,
    pub total_parks: u64,
    pub total_park_ns: u64,
}

/// Main coop park manager
pub struct CoopPark {
    threads: BTreeMap<u64, ParkedThread>,
}

impl CoopPark {
    pub fn new() -> Self { Self { threads: BTreeMap::new() } }
    pub fn register(&mut self, tid: u64) { self.threads.insert(tid, ParkedThread::new(tid)); }

    pub fn park(&mut self, tid: u64, now: u64, timeout: u64) {
        if let Some(t) = self.threads.get_mut(&tid) { t.park(now, timeout); }
    }

    pub fn unpark(&mut self, tid: u64, now: u64) {
        if let Some(t) = self.threads.get_mut(&tid) { t.unpark(now); }
    }

    pub fn tick(&mut self, now: u64) {
        for t in self.threads.values_mut() { t.check_timeout(now); }
    }

    pub fn unregister(&mut self, tid: u64) { self.threads.remove(&tid); }

    pub fn stats(&self) -> ParkStats {
        let parked = self.threads.values().filter(|t| t.state == ParkState::Parked).count() as u32;
        let parks: u64 = self.threads.values().map(|t| t.park_count).sum();
        let ns: u64 = self.threads.values().map(|t| t.total_park_ns).sum();
        ParkStats { tracked_threads: self.threads.len() as u32, parked_count: parked, total_parks: parks, total_park_ns: ns }
    }
}
