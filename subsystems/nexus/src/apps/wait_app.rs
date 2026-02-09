// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Wait (process wait/reap application interface)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Wait target
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppWaitTarget {
    AnyChild,
    Pid(u64),
    ProcessGroup(u64),
    AnyInGroup,
}

/// Wait options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppWaitOption {
    Block,
    NoHang,
    Untraced,
    Continued,
}

/// Child exit information
#[derive(Debug, Clone)]
pub struct AppChildStatus {
    pub pid: u64,
    pub exit_code: i32,
    pub signaled: bool,
    pub signal: u32,
    pub core_dumped: bool,
    pub user_time_us: u64,
    pub sys_time_us: u64,
}

/// Stats for wait operations
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AppWaitStats {
    pub total_waits: u64,
    pub successful_reaps: u64,
    pub nohang_empty: u64,
    pub zombies_reaped: u64,
    pub avg_wait_us: u64,
}

/// Manager for wait application operations
pub struct AppWaitManager {
    zombie_queue: BTreeMap<u64, AppChildStatus>,
    wait_history: Vec<(u64, AppWaitTarget)>,
    stats: AppWaitStats,
}

impl AppWaitManager {
    pub fn new() -> Self {
        Self {
            zombie_queue: BTreeMap::new(),
            wait_history: Vec::new(),
            stats: AppWaitStats {
                total_waits: 0,
                successful_reaps: 0,
                nohang_empty: 0,
                zombies_reaped: 0,
                avg_wait_us: 0,
            },
        }
    }

    #[inline(always)]
    pub fn report_exit(&mut self, status: AppChildStatus) {
        self.zombie_queue.insert(status.pid, status);
    }

    pub fn wait(&mut self, parent: u64, target: AppWaitTarget, option: AppWaitOption) -> Option<AppChildStatus> {
        self.stats.total_waits += 1;
        self.wait_history.push((parent, target));
        match target {
            AppWaitTarget::Pid(pid) => {
                if let Some(status) = self.zombie_queue.remove(&pid) {
                    self.stats.successful_reaps += 1;
                    self.stats.zombies_reaped += 1;
                    Some(status)
                } else {
                    if matches!(option, AppWaitOption::NoHang) {
                        self.stats.nohang_empty += 1;
                    }
                    None
                }
            }
            AppWaitTarget::AnyChild => {
                if let Some((&pid, _)) = self.zombie_queue.iter().next() {
                    let status = self.zombie_queue.remove(&pid).unwrap();
                    self.stats.successful_reaps += 1;
                    self.stats.zombies_reaped += 1;
                    Some(status)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    #[inline(always)]
    pub fn zombie_count(&self) -> usize {
        self.zombie_queue.len()
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppWaitStats {
        &self.stats
    }
}
