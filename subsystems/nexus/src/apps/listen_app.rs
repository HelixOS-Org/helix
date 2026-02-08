// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Listen (connection backlog management)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListenState {
    Active,
    Full,
    Overflow,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListenSynState {
    Received,
    AckPending,
    Completed,
    Dropped,
    TimedOut,
}

#[derive(Debug, Clone)]
pub struct ListenBacklog {
    pub fd: u64,
    pub max_backlog: u32,
    pub current_pending: u32,
    pub syn_queue_len: u32,
    pub accept_queue_len: u32,
    pub total_accepted: u64,
    pub total_dropped: u64,
    pub total_overflows: u64,
    pub state: ListenState,
}

impl ListenBacklog {
    pub fn new(fd: u64, max_backlog: u32) -> Self {
        Self {
            fd, max_backlog,
            current_pending: 0, syn_queue_len: 0,
            accept_queue_len: 0, total_accepted: 0,
            total_dropped: 0, total_overflows: 0,
            state: ListenState::Active,
        }
    }

    pub fn incoming_syn(&mut self) -> bool {
        if self.syn_queue_len >= self.max_backlog {
            self.total_overflows += 1;
            self.state = ListenState::Overflow;
            return false;
        }
        self.syn_queue_len += 1;
        self.current_pending += 1;
        true
    }

    pub fn syn_completed(&mut self) {
        if self.syn_queue_len > 0 { self.syn_queue_len -= 1; }
        self.accept_queue_len += 1;
    }

    pub fn accept_connection(&mut self) -> bool {
        if self.accept_queue_len == 0 { return false; }
        self.accept_queue_len -= 1;
        if self.current_pending > 0 { self.current_pending -= 1; }
        self.total_accepted += 1;
        if self.state == ListenState::Full || self.state == ListenState::Overflow {
            self.state = ListenState::Active;
        }
        true
    }

    pub fn utilization_pct(&self) -> u64 {
        if self.max_backlog == 0 { 0 }
        else { (self.current_pending as u64 * 100) / self.max_backlog as u64 }
    }

    pub fn drop_rate(&self) -> u64 {
        let total = self.total_accepted + self.total_dropped;
        if total == 0 { 0 } else { (self.total_dropped * 100) / total }
    }
}

#[derive(Debug, Clone)]
pub struct ListenAppStats {
    pub total_listeners: u64,
    pub total_accepted: u64,
    pub total_dropped: u64,
    pub total_overflows: u64,
    pub avg_backlog_util: u64,
}

pub struct AppListen {
    backlogs: BTreeMap<u64, ListenBacklog>,
    stats: ListenAppStats,
}

impl AppListen {
    pub fn new() -> Self {
        Self {
            backlogs: BTreeMap::new(),
            stats: ListenAppStats {
                total_listeners: 0, total_accepted: 0,
                total_dropped: 0, total_overflows: 0,
                avg_backlog_util: 0,
            },
        }
    }

    pub fn start_listen(&mut self, fd: u64, backlog: u32) {
        self.backlogs.insert(fd, ListenBacklog::new(fd, backlog));
        self.stats.total_listeners += 1;
    }

    pub fn incoming_connection(&mut self, fd: u64) -> bool {
        if let Some(bl) = self.backlogs.get_mut(&fd) {
            return bl.incoming_syn();
        }
        false
    }

    pub fn accept(&mut self, fd: u64) -> bool {
        if let Some(bl) = self.backlogs.get_mut(&fd) {
            if bl.accept_connection() {
                self.stats.total_accepted += 1;
                return true;
            }
        }
        false
    }

    pub fn stats(&self) -> &ListenAppStats { &self.stats }
}

// ============================================================================
// Merged from listen_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListenV2Result { Success, NotBound, InvalidFd, PermDenied }

/// Listen v2 request
#[derive(Debug, Clone)]
pub struct ListenV2Request {
    pub fd: i32,
    pub backlog: u32,
    pub fast_open: bool,
}

impl ListenV2Request {
    pub fn new(fd: i32, backlog: u32) -> Self { Self { fd, backlog, fast_open: false } }
}

/// Listen v2 app stats
#[derive(Debug, Clone)]
pub struct ListenV2AppStats { pub total_listens: u64, pub active: u32, pub fast_opens: u64, pub failures: u64 }

/// Main app listen v2
#[derive(Debug)]
pub struct AppListenV2 { pub stats: ListenV2AppStats }

impl AppListenV2 {
    pub fn new() -> Self { Self { stats: ListenV2AppStats { total_listens: 0, active: 0, fast_opens: 0, failures: 0 } } }
    pub fn listen(&mut self, req: &ListenV2Request) -> ListenV2Result {
        self.stats.total_listens += 1;
        self.stats.active += 1;
        if req.fast_open { self.stats.fast_opens += 1; }
        ListenV2Result::Success
    }
}
