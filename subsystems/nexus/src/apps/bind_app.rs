// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Bind (socket address binding management)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindFamily {
    Inet4,
    Inet6,
    Unix,
    Netlink,
    Vsock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindResult {
    Success,
    AddrInUse,
    AddrNotAvail,
    PermDenied,
    InvalidArg,
    BadFd,
    NoMem,
}

#[derive(Debug, Clone)]
pub struct BindAddress {
    pub family: BindFamily,
    pub addr_hash: u64,
    pub port: u16,
    pub reuse_addr: bool,
    pub reuse_port: bool,
    pub transparent: bool,
}

impl BindAddress {
    pub fn new(family: BindFamily, addr: &[u8], port: u16) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in addr { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self {
            family, addr_hash: h, port,
            reuse_addr: false, reuse_port: false, transparent: false,
        }
    }

    pub fn is_wildcard(&self) -> bool { self.addr_hash == 0 }
    pub fn is_loopback(&self) -> bool { self.port == 0 }
}

#[derive(Debug, Clone)]
pub struct BindRecord {
    pub fd: u64,
    pub pid: u64,
    pub address: BindAddress,
    pub result: BindResult,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct PortAllocation {
    pub port: u16,
    pub family: BindFamily,
    pub owner_fd: u64,
    pub owner_pid: u64,
    pub reuse_count: u32,
}

#[derive(Debug, Clone)]
pub struct BindAppStats {
    pub total_binds: u64,
    pub successful_binds: u64,
    pub addr_in_use_errors: u64,
    pub wildcard_binds: u64,
    pub reuse_port_binds: u64,
}

pub struct AppBind {
    port_map: BTreeMap<u16, Vec<PortAllocation>>,
    records: Vec<BindRecord>,
    stats: BindAppStats,
}

impl AppBind {
    pub fn new() -> Self {
        Self {
            port_map: BTreeMap::new(),
            records: Vec::new(),
            stats: BindAppStats {
                total_binds: 0, successful_binds: 0,
                addr_in_use_errors: 0, wildcard_binds: 0,
                reuse_port_binds: 0,
            },
        }
    }

    pub fn try_bind(&mut self, fd: u64, pid: u64, address: BindAddress, ts: u64) -> BindResult {
        self.stats.total_binds += 1;
        if address.is_wildcard() { self.stats.wildcard_binds += 1; }
        if address.reuse_port { self.stats.reuse_port_binds += 1; }

        let port = address.port;
        let existing = self.port_map.entry(port).or_insert_with(Vec::new);

        if !existing.is_empty() && !address.reuse_port {
            let conflict = existing.iter().any(|a| a.family == address.family && !address.reuse_addr);
            if conflict {
                self.stats.addr_in_use_errors += 1;
                self.records.push(BindRecord { fd, pid, address, result: BindResult::AddrInUse, timestamp: ts });
                return BindResult::AddrInUse;
            }
        }

        existing.push(PortAllocation {
            port, family: address.family,
            owner_fd: fd, owner_pid: pid, reuse_count: 0,
        });
        self.stats.successful_binds += 1;
        let result = BindResult::Success;
        self.records.push(BindRecord { fd, pid, address, result, timestamp: ts });
        result
    }

    pub fn release_port(&mut self, fd: u64, port: u16) {
        if let Some(allocs) = self.port_map.get_mut(&port) {
            allocs.retain(|a| a.owner_fd != fd);
        }
    }

    pub fn stats(&self) -> &BindAppStats { &self.stats }
}

// ============================================================================
// Merged from bind_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindV2Result {
    Success,
    AddrInUse,
    PermDenied,
    InvalidAddr,
}

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
    pub fn new(fd: i32, port: u16) -> Self {
        Self {
            fd,
            port,
            addr_hash: 0,
            reuse_addr: false,
            reuse_port: false,
        }
    }
}

/// Bind v2 app stats
#[derive(Debug, Clone)]
pub struct BindV2AppStats {
    pub total_binds: u64,
    pub successes: u64,
    pub failures: u64,
    pub reuses: u64,
}

/// Main app bind v2
#[derive(Debug)]
pub struct AppBindV2 {
    pub stats: BindV2AppStats,
}

impl AppBindV2 {
    pub fn new() -> Self {
        Self {
            stats: BindV2AppStats {
                total_binds: 0,
                successes: 0,
                failures: 0,
                reuses: 0,
            },
        }
    }
    pub fn bind(&mut self, req: &BindV2Request) -> BindV2Result {
        self.stats.total_binds += 1;
        if req.reuse_addr || req.reuse_port {
            self.stats.reuses += 1;
        }
        self.stats.successes += 1;
        BindV2Result::Success
    }
}
