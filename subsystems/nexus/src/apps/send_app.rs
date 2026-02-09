// SPDX-License-Identifier: GPL-2.0
//! App send — socket send application interface

extern crate alloc;

/// Send flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendFlag { None, NoSignal, DontRoute, OutOfBand, More }

/// Send request
#[derive(Debug, Clone)]
pub struct SendRequest {
    pub fd: i32,
    pub bytes: u64,
    pub flags: SendFlag,
    pub dest_port: u16,
}

impl SendRequest {
    pub fn new(fd: i32, bytes: u64) -> Self { Self { fd, bytes, flags: SendFlag::None, dest_port: 0 } }
}

/// Send app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SendAppStats { pub total_sends: u64, pub bytes_sent: u64, pub errors: u64, pub partial_sends: u64 }

/// Main app send
#[derive(Debug)]
pub struct AppSend { pub stats: SendAppStats }

impl AppSend {
    pub fn new() -> Self { Self { stats: SendAppStats { total_sends: 0, bytes_sent: 0, errors: 0, partial_sends: 0 } } }
    #[inline]
    pub fn send(&mut self, req: &SendRequest) -> i64 {
        self.stats.total_sends += 1;
        self.stats.bytes_sent += req.bytes;
        req.bytes as i64
    }
}
