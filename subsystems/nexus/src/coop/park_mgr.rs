// SPDX-License-Identifier: GPL-2.0
//! Coop park_mgr — thread parking manager.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Park state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParkState {
    Running,
    Parked,
    Unparked,
    TimedOut,
}

/// Parked thread
#[derive(Debug)]
pub struct ParkedThread {
    pub tid: u64,
    pub state: ParkState,
    pub park_count: u64,
    pub unpark_count: u64,
    pub total_parked_ns: u64,
    pub last_park_start: u64,
    pub timeout_count: u64,
}

impl ParkedThread {
    pub fn new(tid: u64) -> Self {
        Self { tid, state: ParkState::Running, park_count: 0, unpark_count: 0, total_parked_ns: 0, last_park_start: 0, timeout_count: 0 }
    }

    pub fn park(&mut self, now: u64) { self.state = ParkState::Parked; self.park_count += 1; self.last_park_start = now; }

    pub fn unpark(&mut self, now: u64) {
        if self.state == ParkState::Parked {
            self.total_parked_ns += now - self.last_park_start;
            self.unpark_count += 1;
        }
        self.state = ParkState::Unparked;
    }

    pub fn avg_park_time(&self) -> u64 { if self.unpark_count == 0 { 0 } else { self.total_parked_ns / self.unpark_count } }
}

/// Stats
#[derive(Debug, Clone)]
pub struct ParkMgrStats {
    pub total_threads: u32,
    pub currently_parked: u32,
    pub total_parks: u64,
    pub total_unparks: u64,
    pub avg_park_time_ns: u64,
}

/// Main park manager
pub struct CoopParkMgr {
    threads: BTreeMap<u64, ParkedThread>,
}

impl CoopParkMgr {
    pub fn new() -> Self { Self { threads: BTreeMap::new() } }
    pub fn register(&mut self, tid: u64) { self.threads.insert(tid, ParkedThread::new(tid)); }
    pub fn unregister(&mut self, tid: u64) { self.threads.remove(&tid); }

    pub fn park(&mut self, tid: u64, now: u64) {
        if let Some(t) = self.threads.get_mut(&tid) { t.park(now); }
    }

    pub fn unpark(&mut self, tid: u64, now: u64) {
        if let Some(t) = self.threads.get_mut(&tid) { t.unpark(now); }
    }

    pub fn stats(&self) -> ParkMgrStats {
        let parked = self.threads.values().filter(|t| t.state == ParkState::Parked).count() as u32;
        let parks: u64 = self.threads.values().map(|t| t.park_count).sum();
        let unparks: u64 = self.threads.values().map(|t| t.unpark_count).sum();
        let times: Vec<u64> = self.threads.values().filter(|t| t.unpark_count > 0).map(|t| t.avg_park_time()).collect();
        let avg = if times.is_empty() { 0 } else { times.iter().sum::<u64>() / times.len() as u64 };
        ParkMgrStats { total_threads: self.threads.len() as u32, currently_parked: parked, total_parks: parks, total_unparks: unparks, avg_park_time_ns: avg }
    }
}
