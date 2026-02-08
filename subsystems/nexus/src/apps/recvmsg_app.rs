// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Recvmsg (message-based receive operations)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecvmsgFlag {
    Peek,
    WaitAll,
    Trunc,
    DontWait,
    ErrQueue,
    CmsgCloexec,
    NoSignal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecvmsgResult {
    Success(u64),
    WouldBlock,
    ConnReset,
    NotConn,
    Again,
    Truncated,
    Eof,
}

#[derive(Debug, Clone)]
pub struct RecvmsgAncillary {
    pub cmsg_type: u32,
    pub cmsg_level: u32,
    pub data_len: u32,
}

#[derive(Debug, Clone)]
pub struct RecvmsgRecord {
    pub fd: u64,
    pub bytes: u64,
    pub iov_count: u32,
    pub flags: u32,
    pub ancillaries: Vec<RecvmsgAncillary>,
    pub result: RecvmsgResult,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct SocketRecvState {
    pub fd: u64,
    pub total_bytes: u64,
    pub total_calls: u64,
    pub total_iov_entries: u64,
    pub peek_calls: u64,
    pub truncated_count: u64,
    pub eof_count: u64,
    pub failed_calls: u64,
    pub max_msg_size: u64,
    pub ancillary_count: u64,
}

impl SocketRecvState {
    pub fn new(fd: u64) -> Self {
        Self {
            fd, total_bytes: 0, total_calls: 0,
            total_iov_entries: 0, peek_calls: 0,
            truncated_count: 0, eof_count: 0,
            failed_calls: 0, max_msg_size: 0,
            ancillary_count: 0,
        }
    }

    pub fn record_recv(&mut self, bytes: u64, iov: u32, truncated: bool, peek: bool) {
        self.total_bytes += bytes;
        self.total_calls += 1;
        self.total_iov_entries += iov as u64;
        if truncated { self.truncated_count += 1; }
        if peek { self.peek_calls += 1; }
        if bytes > self.max_msg_size { self.max_msg_size = bytes; }
    }

    pub fn avg_msg_size(&self) -> u64 {
        if self.total_calls == 0 { 0 } else { self.total_bytes / self.total_calls }
    }

    pub fn truncation_rate(&self) -> u64 {
        if self.total_calls == 0 { 0 } else { (self.truncated_count * 100) / self.total_calls }
    }
}

#[derive(Debug, Clone)]
pub struct RecvmsgAppStats {
    pub total_recvs: u64,
    pub total_bytes: u64,
    pub total_truncated: u64,
    pub total_eof: u64,
    pub total_failures: u64,
}

pub struct AppRecvmsg {
    sockets: BTreeMap<u64, SocketRecvState>,
    stats: RecvmsgAppStats,
}

impl AppRecvmsg {
    pub fn new() -> Self {
        Self {
            sockets: BTreeMap::new(),
            stats: RecvmsgAppStats {
                total_recvs: 0, total_bytes: 0,
                total_truncated: 0, total_eof: 0,
                total_failures: 0,
            },
        }
    }

    pub fn register_socket(&mut self, fd: u64) {
        self.sockets.insert(fd, SocketRecvState::new(fd));
    }

    pub fn record_recv(&mut self, fd: u64, bytes: u64, iov: u32, truncated: bool) {
        if let Some(s) = self.sockets.get_mut(&fd) {
            s.record_recv(bytes, iov, truncated, false);
            self.stats.total_recvs += 1;
            self.stats.total_bytes += bytes;
            if truncated { self.stats.total_truncated += 1; }
        }
    }

    pub fn stats(&self) -> &RecvmsgAppStats { &self.stats }
}
