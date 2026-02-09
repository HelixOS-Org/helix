// SPDX-License-Identifier: GPL-2.0
//! App mq_open — POSIX message queue open interface

extern crate alloc;

/// Mq open flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MqOpenFlag { ReadOnly, WriteOnly, ReadWrite, Create, Exclusive, Nonblock }

/// Mq open result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MqOpenResult { Success, PermissionDenied, Exists, NotFound, Error }

/// Mq open record
#[derive(Debug, Clone)]
pub struct MqOpenRecord {
    pub result: MqOpenResult,
    pub name_hash: u64,
    pub flags: u32,
    pub maxmsg: u32,
    pub msgsize: u32,
    pub mqd: i32,
}

impl MqOpenRecord {
    pub fn new(name: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in name { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self { result: MqOpenResult::Success, name_hash: h, flags: 0, maxmsg: 10, msgsize: 8192, mqd: -1 }
    }
}

/// Mq open app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MqOpenAppStats { pub total_ops: u64, pub opened: u64, pub created: u64, pub errors: u64 }

/// Main app mq_open
#[derive(Debug)]
pub struct AppMqOpen { pub stats: MqOpenAppStats }

impl AppMqOpen {
    pub fn new() -> Self { Self { stats: MqOpenAppStats { total_ops: 0, opened: 0, created: 0, errors: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &MqOpenRecord) {
        self.stats.total_ops += 1;
        if rec.result == MqOpenResult::Success { self.stats.opened += 1; }
        if rec.result != MqOpenResult::Success { self.stats.errors += 1; }
    }
}
