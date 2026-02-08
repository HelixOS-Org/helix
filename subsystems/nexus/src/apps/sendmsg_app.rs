// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Sendmsg (message-based send operations)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendmsgFlag {
    DontRoute,
    DontWait,
    Eor,
    Oob,
    NoSignal,
    More,
    Confirm,
    ZeroCopy,
    Fastopen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendmsgCmsgType {
    ScmRights,
    ScmCredentials,
    ScmTimestamp,
    IpTtl,
    IpTos,
    Ipv6TClass,
    TxTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendmsgResult {
    Success(u64),
    WouldBlock,
    MsgSize,
    ConnReset,
    Pipe,
    NoMem,
    NotConn,
    Again,
}

#[derive(Debug, Clone)]
pub struct SendmsgRecord {
    pub fd: u64,
    pub bytes: u64,
    pub iov_count: u32,
    pub flags: u32,
    pub has_cmsg: bool,
    pub has_dest_addr: bool,
    pub result: SendmsgResult,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct SocketSendState {
    pub fd: u64,
    pub total_bytes: u64,
    pub total_calls: u64,
    pub total_iov_entries: u64,
    pub zerocopy_calls: u64,
    pub zerocopy_bytes: u64,
    pub failed_calls: u64,
    pub max_msg_size: u64,
}

impl SocketSendState {
    pub fn new(fd: u64) -> Self {
        Self {
            fd, total_bytes: 0, total_calls: 0,
            total_iov_entries: 0, zerocopy_calls: 0,
            zerocopy_bytes: 0, failed_calls: 0,
            max_msg_size: 0,
        }
    }

    pub fn record(&mut self, bytes: u64, iov: u32, zerocopy: bool) {
        self.total_bytes += bytes;
        self.total_calls += 1;
        self.total_iov_entries += iov as u64;
        if zerocopy {
            self.zerocopy_calls += 1;
            self.zerocopy_bytes += bytes;
        }
        if bytes > self.max_msg_size { self.max_msg_size = bytes; }
    }

    pub fn avg_msg_size(&self) -> u64 {
        if self.total_calls == 0 { 0 } else { self.total_bytes / self.total_calls }
    }

    pub fn avg_iov_per_call(&self) -> u64 {
        if self.total_calls == 0 { 0 } else { self.total_iov_entries / self.total_calls }
    }

    pub fn zerocopy_pct(&self) -> u64 {
        if self.total_calls == 0 { 0 } else { (self.zerocopy_calls * 100) / self.total_calls }
    }
}

#[derive(Debug, Clone)]
pub struct SendmsgAppStats {
    pub total_sends: u64,
    pub total_bytes: u64,
    pub total_zerocopy: u64,
    pub total_failures: u64,
}

pub struct AppSendmsg {
    sockets: BTreeMap<u64, SocketSendState>,
    stats: SendmsgAppStats,
}

impl AppSendmsg {
    pub fn new() -> Self {
        Self {
            sockets: BTreeMap::new(),
            stats: SendmsgAppStats {
                total_sends: 0, total_bytes: 0,
                total_zerocopy: 0, total_failures: 0,
            },
        }
    }

    pub fn register_socket(&mut self, fd: u64) {
        self.sockets.insert(fd, SocketSendState::new(fd));
    }

    pub fn record_send(&mut self, fd: u64, bytes: u64, iov: u32, zerocopy: bool) {
        if let Some(s) = self.sockets.get_mut(&fd) {
            s.record(bytes, iov, zerocopy);
            self.stats.total_sends += 1;
            self.stats.total_bytes += bytes;
            if zerocopy { self.stats.total_zerocopy += 1; }
        }
    }

    pub fn stats(&self) -> &SendmsgAppStats { &self.stats }
}
