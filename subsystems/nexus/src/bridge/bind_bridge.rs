// SPDX-License-Identifier: GPL-2.0
//! Bridge bind — socket bind address bridging

extern crate alloc;

/// Bind address family
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindFamily { Inet4, Inet6, Unix, Netlink, Packet }

/// Bind bridge record
#[derive(Debug, Clone)]
pub struct BindBridgeRecord {
    pub family: BindFamily,
    pub fd: i32,
    pub port: u16,
    pub addr_hash: u64,
    pub reuse_addr: bool,
}

impl BindBridgeRecord {
    pub fn new(family: BindFamily, fd: i32) -> Self { Self { family, fd, port: 0, addr_hash: 0, reuse_addr: false } }
}

/// Bind bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BindBridgeStats { pub total_binds: u64, pub inet_binds: u64, pub unix_binds: u64, pub reuse_binds: u64 }

/// Main bridge bind
#[derive(Debug)]
pub struct BridgeBind { pub stats: BindBridgeStats }

impl BridgeBind {
    pub fn new() -> Self { Self { stats: BindBridgeStats { total_binds: 0, inet_binds: 0, unix_binds: 0, reuse_binds: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &BindBridgeRecord) {
        self.stats.total_binds += 1;
        match rec.family {
            BindFamily::Inet4 | BindFamily::Inet6 => self.stats.inet_binds += 1,
            BindFamily::Unix => self.stats.unix_binds += 1,
            _ => {}
        }
        if rec.reuse_addr { self.stats.reuse_binds += 1; }
    }
}
