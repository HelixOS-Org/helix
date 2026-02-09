// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Fork (process forking application interface)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Fork mode for application layer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppForkMode {
    Standard,
    Vfork,
    PosixSpawn,
    CopyOnWrite,
}

/// Fork request from application
#[derive(Debug, Clone)]
pub struct AppForkRequest {
    pub parent_pid: u64,
    pub mode: AppForkMode,
    pub inherit_fds: bool,
    pub inherit_signals: bool,
    pub share_memory: bool,
    pub timestamp: u64,
}

/// Fork result returned to application
#[derive(Debug, Clone)]
pub struct AppForkResult {
    pub child_pid: u64,
    pub parent_pid: u64,
    pub cow_pages: u64,
    pub latency_us: u64,
    pub success: bool,
}

/// Stats for fork operations
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AppForkStats {
    pub total_forks: u64,
    pub successful: u64,
    pub failed: u64,
    pub vforks: u64,
    pub avg_cow_pages: u64,
    pub avg_latency_us: u64,
}

/// Manager for fork application operations
pub struct AppForkManager {
    active_children: BTreeMap<u64, Vec<u64>>,
    results: Vec<AppForkResult>,
    next_pid: u64,
    stats: AppForkStats,
}

impl AppForkManager {
    pub fn new() -> Self {
        Self {
            active_children: BTreeMap::new(),
            results: Vec::new(),
            next_pid: 5000,
            stats: AppForkStats {
                total_forks: 0,
                successful: 0,
                failed: 0,
                vforks: 0,
                avg_cow_pages: 0,
                avg_latency_us: 0,
            },
        }
    }

    pub fn fork(&mut self, parent_pid: u64, mode: AppForkMode) -> AppForkResult {
        let child_pid = self.next_pid;
        self.next_pid += 1;
        let cow = match mode {
            AppForkMode::Vfork => {
                self.stats.vforks += 1;
                0
            },
            AppForkMode::CopyOnWrite => 512,
            _ => 256,
        };
        let result = AppForkResult {
            child_pid,
            parent_pid,
            cow_pages: cow,
            latency_us: if cow == 0 { 30 } else { 150 },
            success: true,
        };
        self.active_children
            .entry(parent_pid)
            .or_insert_with(Vec::new)
            .push(child_pid);
        self.results.push(result.clone());
        self.stats.total_forks += 1;
        self.stats.successful += 1;
        result
    }

    #[inline]
    pub fn reap_child(&mut self, parent_pid: u64, child_pid: u64) -> bool {
        if let Some(children) = self.active_children.get_mut(&parent_pid) {
            let before = children.len();
            children.retain(|&c| c != child_pid);
            children.len() < before
        } else {
            false
        }
    }

    #[inline]
    pub fn children_of(&self, parent: u64) -> usize {
        self.active_children
            .get(&parent)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppForkStats {
        &self.stats
    }
}
