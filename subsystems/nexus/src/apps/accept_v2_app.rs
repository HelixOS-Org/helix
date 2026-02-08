// SPDX-License-Identifier: GPL-2.0
//! App accept v2 — advanced socket accept application interface

extern crate alloc;

/// Accept v2 flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptV2Flag { None, NonBlock, CloseOnExec }

/// Accept v2 request
#[derive(Debug, Clone)]
pub struct AcceptV2Request {
    pub listen_fd: i32,
    pub flags: AcceptV2Flag,
}

impl AcceptV2Request {
    pub fn new(listen_fd: i32) -> Self { Self { listen_fd, flags: AcceptV2Flag::None } }
}

/// Accept v2 app stats
#[derive(Debug, Clone)]
pub struct AcceptV2AppStats { pub total_accepts: u64, pub successful: u64, pub would_block: u64, pub errors: u64 }

/// Main app accept v2
#[derive(Debug)]
pub struct AppAcceptV2 { pub stats: AcceptV2AppStats }

impl AppAcceptV2 {
    pub fn new() -> Self { Self { stats: AcceptV2AppStats { total_accepts: 0, successful: 0, would_block: 0, errors: 0 } } }
    pub fn accept(&mut self, req: &AcceptV2Request) -> i32 {
        self.stats.total_accepts += 1;
        self.stats.successful += 1;
        self.stats.total_accepts as i32
    }
}
