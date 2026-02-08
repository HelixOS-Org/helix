// SPDX-License-Identifier: GPL-2.0
//! NEXUS Bridge — Pthread (POSIX thread bridge)

extern crate alloc;
use alloc::collections::BTreeMap;

/// Thread state representation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgePthreadState {
    Created,
    Running,
    Blocked,
    Sleeping,
    Stopped,
    Zombie,
    Dead,
}

/// Thread attributes
#[derive(Debug, Clone)]
pub struct BridgePthreadAttr {
    pub stack_size: usize,
    pub guard_size: usize,
    pub detached: bool,
    pub priority: i32,
    pub policy: u32,
    pub cpu_affinity: u64,
}

/// Thread entry
#[derive(Debug, Clone)]
pub struct BridgePthreadEntry {
    pub tid: u64,
    pub pid: u64,
    pub state: BridgePthreadState,
    pub attr: BridgePthreadAttr,
    pub join_count: u32,
    pub creation_time: u64,
}

/// Stats for pthread operations
#[derive(Debug, Clone)]
pub struct BridgePthreadStats {
    pub total_created: u64,
    pub active_threads: u64,
    pub peak_threads: u64,
    pub joins_completed: u64,
    pub detached_threads: u64,
}

/// Manager for pthread bridge operations
pub struct BridgePthreadManager {
    threads: BTreeMap<u64, BridgePthreadEntry>,
    next_tid: u64,
    stats: BridgePthreadStats,
}

impl BridgePthreadManager {
    pub fn new() -> Self {
        Self {
            threads: BTreeMap::new(),
            next_tid: 10000,
            stats: BridgePthreadStats {
                total_created: 0,
                active_threads: 0,
                peak_threads: 0,
                joins_completed: 0,
                detached_threads: 0,
            },
        }
    }

    pub fn create_thread(&mut self, pid: u64, stack_size: usize, detached: bool) -> u64 {
        let tid = self.next_tid;
        self.next_tid += 1;
        let attr = BridgePthreadAttr {
            stack_size,
            guard_size: 4096,
            detached,
            priority: 0,
            policy: 0,
            cpu_affinity: 0xFFFFFFFF,
        };
        let entry = BridgePthreadEntry {
            tid,
            pid,
            state: BridgePthreadState::Created,
            attr,
            join_count: 0,
            creation_time: tid.wrapping_mul(43),
        };
        self.threads.insert(tid, entry);
        self.stats.total_created += 1;
        self.stats.active_threads += 1;
        if detached { self.stats.detached_threads += 1; }
        if self.stats.active_threads > self.stats.peak_threads {
            self.stats.peak_threads = self.stats.active_threads;
        }
        tid
    }

    pub fn join_thread(&mut self, tid: u64) -> bool {
        if let Some(entry) = self.threads.get_mut(&tid) {
            if entry.attr.detached { return false; }
            entry.state = BridgePthreadState::Dead;
            self.stats.joins_completed += 1;
            self.stats.active_threads = self.stats.active_threads.saturating_sub(1);
            true
        } else {
            false
        }
    }

    pub fn set_state(&mut self, tid: u64, state: BridgePthreadState) {
        if let Some(entry) = self.threads.get_mut(&tid) {
            entry.state = state;
        }
    }

    pub fn stats(&self) -> &BridgePthreadStats {
        &self.stats
    }
}
