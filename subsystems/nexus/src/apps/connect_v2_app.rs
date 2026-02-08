// SPDX-License-Identifier: GPL-2.0
//! App connect v2 — advanced connection application interface

extern crate alloc;

/// Connect v2 result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectV2Result { Connected, InProgress, Refused, TimedOut, Unreachable }

/// Connect v2 request
#[derive(Debug, Clone)]
pub struct ConnectV2Request {
    pub fd: i32,
    pub addr_hash: u64,
    pub port: u16,
    pub timeout_ms: u32,
}

impl ConnectV2Request {
    pub fn new(fd: i32, port: u16) -> Self { Self { fd, addr_hash: 0, port, timeout_ms: 30000 } }
}

/// Connect v2 app stats
#[derive(Debug, Clone)]
pub struct ConnectV2AppStats { pub total_connects: u64, pub connected: u64, pub refused: u64, pub timeouts: u64 }

/// Main app connect v2
#[derive(Debug)]
pub struct AppConnectV2 { pub stats: ConnectV2AppStats }

impl AppConnectV2 {
    pub fn new() -> Self { Self { stats: ConnectV2AppStats { total_connects: 0, connected: 0, refused: 0, timeouts: 0 } } }
    pub fn connect(&mut self, req: &ConnectV2Request) -> ConnectV2Result {
        self.stats.total_connects += 1;
        self.stats.connected += 1;
        ConnectV2Result::Connected
    }
}
