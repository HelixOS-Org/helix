// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — Wait (cooperative process wait/reap)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Wait cooperation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopWaitMode {
    Blocking,
    NonBlocking,
    Batched,
    EventDriven,
    Speculative,
}

/// Wait cooperation entry
#[derive(Debug, Clone)]
pub struct CoopWaitEntry {
    pub waiter_pid: u64,
    pub target_pid: u64,
    pub mode: CoopWaitMode,
    pub wait_start: u64,
    pub wait_duration_us: u64,
    pub reaped: bool,
}

/// Wait cooperation stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CoopWaitStats {
    pub total_waits: u64,
    pub blocking_waits: u64,
    pub nonblocking_waits: u64,
    pub batched_reaps: u64,
    pub avg_wait_us: u64,
    pub total_reaped: u64,
}

/// Manager for cooperative wait operations
pub struct CoopWaitManager {
    pending: BTreeMap<u64, Vec<CoopWaitEntry>>,
    completed: Vec<CoopWaitEntry>,
    stats: CoopWaitStats,
}

impl CoopWaitManager {
    pub fn new() -> Self {
        Self {
            pending: BTreeMap::new(),
            completed: Vec::new(),
            stats: CoopWaitStats {
                total_waits: 0,
                blocking_waits: 0,
                nonblocking_waits: 0,
                batched_reaps: 0,
                avg_wait_us: 0,
                total_reaped: 0,
            },
        }
    }

    pub fn register_wait(&mut self, waiter: u64, target: u64, mode: CoopWaitMode) {
        self.stats.total_waits += 1;
        match mode {
            CoopWaitMode::Blocking => self.stats.blocking_waits += 1,
            CoopWaitMode::NonBlocking => self.stats.nonblocking_waits += 1,
            _ => {}
        }
        let entry = CoopWaitEntry {
            waiter_pid: waiter,
            target_pid: target,
            mode,
            wait_start: self.stats.total_waits.wrapping_mul(53),
            wait_duration_us: 0,
            reaped: false,
        };
        self.pending.entry(waiter).or_insert_with(Vec::new).push(entry);
    }

    pub fn notify_exit(&mut self, pid: u64) {
        let mut to_complete = Vec::new();
        for (_, entries) in self.pending.iter_mut() {
            entries.retain(|e| {
                if e.target_pid == pid {
                    let mut completed = e.clone();
                    completed.reaped = true;
                    completed.wait_duration_us = 100;
                    to_complete.push(completed);
                    false
                } else {
                    true
                }
            });
        }
        self.stats.total_reaped += to_complete.len() as u64;
        self.completed.extend(to_complete);
    }

    #[inline(always)]
    pub fn pending_count(&self, waiter: u64) -> usize {
        self.pending.get(&waiter).map(|v| v.len()).unwrap_or(0)
    }

    #[inline(always)]
    pub fn stats(&self) -> &CoopWaitStats {
        &self.stats
    }
}
