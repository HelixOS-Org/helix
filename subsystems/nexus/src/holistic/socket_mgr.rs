// SPDX-License-Identifier: GPL-2.0
//! Holistic socket manager — unified socket layer for all protocol families

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;

/// Socket domain
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketDomain {
    AfInet,
    AfInet6,
    AfUnix,
    AfNetlink,
    AfPacket,
    AfVsock,
    AfBluetooth,
    AfCan,
    AfTipc,
    AfXdp,
}

/// Socket type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketType {
    Stream,
    Dgram,
    Raw,
    Seqpacket,
    Rdm,
    Packet,
}

/// Socket state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketMgrState {
    Unconnected,
    Binding,
    Bound,
    Listening,
    Connecting,
    Connected,
    Disconnecting,
    Closed,
}

/// Socket shutdown mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketShutMode {
    Read,
    Write,
    Both,
}

/// Socket buffer
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SocketBuffer {
    pub capacity: u32,
    pub used: u32,
    pub low_watermark: u32,
    pub high_watermark: u32,
    pub total_bytes: u64,
    pub total_ops: u64,
}

impl SocketBuffer {
    pub fn new(capacity: u32) -> Self {
        Self {
            capacity,
            used: 0,
            low_watermark: 1,
            high_watermark: capacity,
            total_bytes: 0,
            total_ops: 0,
        }
    }

    #[inline]
    pub fn write(&mut self, bytes: u32) -> u32 {
        let avail = self.capacity - self.used;
        let written = if bytes > avail { avail } else { bytes };
        self.used += written;
        self.total_bytes += written as u64;
        self.total_ops += 1;
        written
    }

    #[inline]
    pub fn read(&mut self, bytes: u32) -> u32 {
        let readable = if bytes > self.used { self.used } else { bytes };
        self.used -= readable;
        self.total_ops += 1;
        readable
    }

    #[inline]
    pub fn utilization_pct(&self) -> f64 {
        if self.capacity == 0 {
            return 0.0;
        }
        (self.used as f64 / self.capacity as f64) * 100.0
    }

    #[inline(always)]
    pub fn is_readable(&self) -> bool {
        self.used >= self.low_watermark
    }

    #[inline(always)]
    pub fn is_writable(&self) -> bool {
        self.used < self.high_watermark
    }
}

/// Managed socket instance
#[derive(Debug, Clone)]
pub struct ManagedSocket {
    pub fd: u64,
    pub domain: SocketDomain,
    pub sock_type: SocketType,
    pub state: SocketMgrState,
    pub protocol: u16,
    pub send_buf: SocketBuffer,
    pub recv_buf: SocketBuffer,
    pub local_addr_hash: u64,
    pub remote_addr_hash: u64,
    pub backlog: u32,
    pub flags: u32,
    pub created_ns: u64,
    pub last_activity_ns: u64,
    pub send_timeout_ms: u32,
    pub recv_timeout_ms: u32,
    pub error_count: u32,
}

impl ManagedSocket {
    pub fn new(fd: u64, domain: SocketDomain, sock_type: SocketType) -> Self {
        Self {
            fd,
            domain,
            sock_type,
            state: SocketMgrState::Unconnected,
            protocol: 0,
            send_buf: SocketBuffer::new(212992),
            recv_buf: SocketBuffer::new(212992),
            local_addr_hash: 0,
            remote_addr_hash: 0,
            backlog: 0,
            flags: 0,
            created_ns: 0,
            last_activity_ns: 0,
            send_timeout_ms: 0,
            recv_timeout_ms: 0,
            error_count: 0,
        }
    }

    #[inline]
    pub fn bind(&mut self, addr: &[u8]) {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in addr {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        self.local_addr_hash = h;
        self.state = SocketMgrState::Bound;
    }

    #[inline]
    pub fn connect(&mut self, addr: &[u8]) {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in addr {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        self.remote_addr_hash = h;
        self.state = SocketMgrState::Connected;
    }

    #[inline(always)]
    pub fn listen(&mut self, backlog: u32) {
        self.backlog = backlog;
        self.state = SocketMgrState::Listening;
    }

    #[inline(always)]
    pub fn close(&mut self) {
        self.state = SocketMgrState::Closed;
    }

    #[inline(always)]
    pub fn idle_ms(&self, now_ns: u64) -> u64 {
        (now_ns.saturating_sub(self.last_activity_ns)) / 1_000_000
    }
}

/// Socket manager stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SocketMgrStats {
    pub total_created: u64,
    pub active_sockets: u64,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub total_errors: u64,
}

/// Main holistic socket manager
#[derive(Debug)]
pub struct HolisticSocketMgr {
    pub sockets: BTreeMap<u64, ManagedSocket>,
    pub domain_counts: BTreeMap<u8, u64>,
    pub stats: SocketMgrStats,
    pub next_fd: u64,
    pub max_sockets: u32,
}

impl HolisticSocketMgr {
    pub fn new(max_sockets: u32) -> Self {
        Self {
            sockets: BTreeMap::new(),
            domain_counts: BTreeMap::new(),
            stats: SocketMgrStats {
                total_created: 0,
                active_sockets: 0,
                total_bytes_sent: 0,
                total_bytes_received: 0,
                total_errors: 0,
            },
            next_fd: 3,
            max_sockets,
        }
    }

    pub fn create_socket(&mut self, domain: SocketDomain, sock_type: SocketType) -> Option<u64> {
        if self.stats.active_sockets >= self.max_sockets as u64 {
            return None;
        }
        let fd = self.next_fd;
        self.next_fd += 1;
        self.sockets.insert(fd, ManagedSocket::new(fd, domain, sock_type));
        self.stats.total_created += 1;
        self.stats.active_sockets += 1;
        let dk = domain as u8;
        *self.domain_counts.entry(dk).or_insert(0) += 1;
        Some(fd)
    }

    #[inline]
    pub fn close_socket(&mut self, fd: u64) -> bool {
        if let Some(sock) = self.sockets.get_mut(&fd) {
            sock.close();
            self.stats.active_sockets = self.stats.active_sockets.saturating_sub(1);
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn get_socket(&self, fd: u64) -> Option<&ManagedSocket> {
        self.sockets.get(&fd)
    }

    #[inline]
    pub fn active_by_domain(&self, domain: SocketDomain) -> u64 {
        self.sockets.values()
            .filter(|s| s.domain == domain && s.state != SocketMgrState::Closed)
            .count() as u64
    }
}
