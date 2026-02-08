// SPDX-License-Identifier: GPL-2.0
//! App bind v2 — advanced socket bind application interface

extern crate alloc;

/// Bind v2 result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindV2Result { Success, AddrInUse, PermDenied, InvalidAddr }

/// Bind v2 request
#[derive(Debug, Clone)]
pub struct BindV2Request {
    pub fd: i32,
    pub port: u16,
    pub addr_hash: u64,
    pub reuse_addr: bool,
    pub reuse_port: bool,
}

impl BindV2Request {
    pub fn new(fd: i32, port: u16) -> Self { Self { fd, port, addr_hash: 0, reuse_addr: false, reuse_port: false } }
}

/// Bind v2 app stats
#[derive(Debug, Clone)]
pub struct BindV2AppStats { pub total_binds: u64, pub successes: u64, pub failures: u64, pub reuses: u64 }

/// Main app bind v2
#[derive(Debug)]
pub struct AppBindV2 { pub stats: BindV2AppStats }

impl AppBindV2 {
    pub fn new() -> Self { Self { stats: BindV2AppStats { total_binds: 0, successes: 0, failures: 0, reuses: 0 } } }
    pub fn bind(&mut self, req: &BindV2Request) -> BindV2Result {
        self.stats.total_binds += 1;
        if req.reuse_addr || req.reuse_port { self.stats.reuses += 1; }
        self.stats.successes += 1;
        BindV2Result::Success
    }
}
