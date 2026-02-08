// SPDX-License-Identifier: GPL-2.0
//! NEXUS Bridge — Wait (process wait/reap bridge)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Wait target type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeWaitTarget {
    AnyChild,
    SpecificPid(u64),
    ProcessGroup(u64),
    AnyInGroup,
}

/// Wait options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeWaitOption {
    NoHang,
    Untraced,
    Continued,
    CloneWait,
}

/// Exit status representation
#[derive(Debug, Clone)]
pub struct BridgeExitStatus {
    pub pid: u64,
    pub exit_code: i32,
    pub signaled: bool,
    pub signal_number: u32,
    pub core_dumped: bool,
    pub rusage_utime: u64,
    pub rusage_stime: u64,
}

/// Stats for wait operations
#[derive(Debug, Clone)]
pub struct BridgeWaitStats {
    pub total_waits: u64,
    pub successful_reaps: u64,
    pub nohang_returns: u64,
    pub zombies_reaped: u64,
    pub orphans_detected: u64,
}

/// Manager for wait bridge operations
pub struct BridgeWaitManager {
    zombies: BTreeMap<u64, BridgeExitStatus>,
    waiting: BTreeMap<u64, BridgeWaitTarget>,
    stats: BridgeWaitStats,
}

impl BridgeWaitManager {
    pub fn new() -> Self {
        Self {
            zombies: BTreeMap::new(),
            waiting: BTreeMap::new(),
            stats: BridgeWaitStats {
                total_waits: 0,
                successful_reaps: 0,
                nohang_returns: 0,
                zombies_reaped: 0,
                orphans_detected: 0,
            },
        }
    }

    pub fn add_zombie(&mut self, status: BridgeExitStatus) {
        self.zombies.insert(status.pid, status);
    }

    pub fn waitpid(&mut self, parent: u64, target: BridgeWaitTarget) -> Option<BridgeExitStatus> {
        self.stats.total_waits += 1;
        match target {
            BridgeWaitTarget::SpecificPid(pid) => {
                if let Some(status) = self.zombies.remove(&pid) {
                    self.stats.successful_reaps += 1;
                    self.stats.zombies_reaped += 1;
                    Some(status)
                } else {
                    None
                }
            }
            BridgeWaitTarget::AnyChild => {
                let first_zombie = self.zombies.keys().next().cloned();
                if let Some(pid) = first_zombie {
                    let status = self.zombies.remove(&pid).unwrap();
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

    pub fn zombie_count(&self) -> usize {
        self.zombies.len()
    }

    pub fn stats(&self) -> &BridgeWaitStats {
        &self.stats
    }
}
