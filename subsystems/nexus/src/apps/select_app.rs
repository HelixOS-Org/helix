// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Select App (synchronous I/O multiplexing)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Select fd set type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectFdSet {
    Read,
    Write,
    Except,
}

/// A file descriptor in a select set
#[derive(Debug, Clone)]
pub struct SelectFdEntry {
    pub fd: u64,
    pub sets: u32,
    pub ready_mask: u32,
    pub check_count: u64,
    pub ready_count: u64,
}

/// A select call instance
#[derive(Debug, Clone)]
pub struct SelectCall {
    pub id: u64,
    pub nfds: u32,
    pub fds: Vec<SelectFdEntry>,
    pub timeout_us: u64,
    pub started_tick: u64,
    pub completed: bool,
    pub ready_fds: u32,
}

impl SelectCall {
    pub fn new(id: u64, nfds: u32, timeout_us: u64, tick: u64) -> Self {
        Self {
            id,
            nfds,
            fds: Vec::new(),
            timeout_us,
            started_tick: tick,
            completed: false,
            ready_fds: 0,
        }
    }

    #[inline]
    pub fn add_fd(&mut self, fd: u64, sets: u32) {
        self.fds.push(SelectFdEntry {
            fd,
            sets,
            ready_mask: 0,
            check_count: 0,
            ready_count: 0,
        });
    }

    pub fn mark_ready(&mut self, fd: u64, ready_sets: u32) -> bool {
        for entry in self.fds.iter_mut() {
            if entry.fd == fd {
                entry.ready_mask |= ready_sets & entry.sets;
                if entry.ready_mask != 0 {
                    entry.ready_count += 1;
                    self.ready_fds += 1;
                    return true;
                }
            }
        }
        false
    }

    #[inline(always)]
    pub fn complete(&mut self) {
        self.completed = true;
    }
}

/// Statistics for select app
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SelectAppStats {
    pub total_calls: u64,
    pub total_fds_monitored: u64,
    pub total_ready: u64,
    pub timeouts: u64,
    pub max_nfds_seen: u32,
    pub avg_fds_per_call: u64,
}

/// Main select app manager
#[derive(Debug)]
pub struct AppSelect {
    calls: BTreeMap<u64, SelectCall>,
    next_id: u64,
    stats: SelectAppStats,
}

impl AppSelect {
    pub fn new() -> Self {
        Self {
            calls: BTreeMap::new(),
            next_id: 1,
            stats: SelectAppStats {
                total_calls: 0,
                total_fds_monitored: 0,
                total_ready: 0,
                timeouts: 0,
                max_nfds_seen: 0,
                avg_fds_per_call: 0,
            },
        }
    }

    #[inline]
    pub fn begin_select(&mut self, nfds: u32, timeout_us: u64, tick: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.calls.insert(id, SelectCall::new(id, nfds, timeout_us, tick));
        self.stats.total_calls += 1;
        if nfds > self.stats.max_nfds_seen {
            self.stats.max_nfds_seen = nfds;
        }
        id
    }

    #[inline]
    pub fn add_fd(&mut self, call_id: u64, fd: u64, sets: u32) -> bool {
        if let Some(call) = self.calls.get_mut(&call_id) {
            call.add_fd(fd, sets);
            self.stats.total_fds_monitored += 1;
            true
        } else {
            false
        }
    }

    pub fn complete_select(&mut self, call_id: u64) -> Option<u32> {
        if let Some(call) = self.calls.get_mut(&call_id) {
            call.complete();
            let ready = call.ready_fds;
            self.stats.total_ready += ready as u64;
            if ready == 0 {
                self.stats.timeouts += 1;
            }
            Some(ready)
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &SelectAppStats {
        &self.stats
    }
}
