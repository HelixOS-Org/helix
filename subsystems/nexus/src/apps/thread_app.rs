// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Thread (thread management application interface)

extern crate alloc;
use alloc::collections::BTreeMap;

/// Thread state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppThreadState {
    Created,
    Running,
    Blocked,
    Sleeping,
    Waiting,
    Stopped,
    Terminated,
}

/// Thread attributes
#[derive(Debug, Clone)]
pub struct AppThreadAttr {
    pub stack_size: usize,
    pub guard_size: usize,
    pub detached: bool,
    pub priority: i32,
    pub affinity_mask: u64,
}

/// Thread entry
#[derive(Debug, Clone)]
pub struct AppThreadEntry {
    pub tid: u64,
    pub pid: u64,
    pub state: AppThreadState,
    pub attr: AppThreadAttr,
    pub cpu_time_us: u64,
    pub creation_time: u64,
}

/// Stats for thread operations
#[derive(Debug, Clone)]
pub struct AppThreadStats {
    pub total_created: u64,
    pub active_threads: u64,
    pub peak_threads: u64,
    pub joins: u64,
    pub detaches: u64,
    pub cancellations: u64,
}

/// Manager for thread application operations
pub struct AppThreadManager {
    threads: BTreeMap<u64, AppThreadEntry>,
    next_tid: u64,
    stats: AppThreadStats,
}

impl AppThreadManager {
    pub fn new() -> Self {
        Self {
            threads: BTreeMap::new(),
            next_tid: 20000,
            stats: AppThreadStats {
                total_created: 0,
                active_threads: 0,
                peak_threads: 0,
                joins: 0,
                detaches: 0,
                cancellations: 0,
            },
        }
    }

    pub fn create(&mut self, pid: u64, stack_size: usize, detached: bool) -> u64 {
        let tid = self.next_tid;
        self.next_tid += 1;
        let attr = AppThreadAttr {
            stack_size,
            guard_size: 4096,
            detached,
            priority: 0,
            affinity_mask: 0xFFFFFFFF,
        };
        let entry = AppThreadEntry {
            tid,
            pid,
            state: AppThreadState::Created,
            attr,
            cpu_time_us: 0,
            creation_time: tid.wrapping_mul(43),
        };
        self.threads.insert(tid, entry);
        self.stats.total_created += 1;
        self.stats.active_threads += 1;
        if detached {
            self.stats.detaches += 1;
        }
        if self.stats.active_threads > self.stats.peak_threads {
            self.stats.peak_threads = self.stats.active_threads;
        }
        tid
    }

    pub fn join(&mut self, tid: u64) -> bool {
        if let Some(t) = self.threads.get(&tid) {
            if t.attr.detached {
                return false;
            }
        }
        if self.threads.remove(&tid).is_some() {
            self.stats.joins += 1;
            self.stats.active_threads = self.stats.active_threads.saturating_sub(1);
            true
        } else {
            false
        }
    }

    pub fn cancel(&mut self, tid: u64) -> bool {
        if self.threads.remove(&tid).is_some() {
            self.stats.cancellations += 1;
            self.stats.active_threads = self.stats.active_threads.saturating_sub(1);
            true
        } else {
            false
        }
    }

    pub fn set_state(&mut self, tid: u64, state: AppThreadState) {
        if let Some(t) = self.threads.get_mut(&tid) {
            t.state = state;
        }
    }

    pub fn stats(&self) -> &AppThreadStats {
        &self.stats
    }
}
