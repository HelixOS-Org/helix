// SPDX-License-Identifier: GPL-2.0
//! App recv — socket receive application interface

extern crate alloc;

/// Recv flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecvFlag { None, Peek, WaitAll, DontWait, Truncate }

/// Recv request
#[derive(Debug, Clone)]
pub struct RecvRequest {
    pub fd: i32,
    pub max_bytes: u64,
    pub flags: RecvFlag,
}

impl RecvRequest {
    pub fn new(fd: i32, max_bytes: u64) -> Self { Self { fd, max_bytes, flags: RecvFlag::None } }
}

/// Recv app stats
#[derive(Debug, Clone)]
pub struct RecvAppStats { pub total_recvs: u64, pub bytes_received: u64, pub errors: u64, pub truncated: u64 }

/// Main app recv
#[derive(Debug)]
pub struct AppRecv { pub stats: RecvAppStats }

impl AppRecv {
    pub fn new() -> Self { Self { stats: RecvAppStats { total_recvs: 0, bytes_received: 0, errors: 0, truncated: 0 } } }
    pub fn recv(&mut self, req: &RecvRequest) -> i64 {
        self.stats.total_recvs += 1;
        self.stats.bytes_received += req.max_bytes;
        req.max_bytes as i64
    }
}
