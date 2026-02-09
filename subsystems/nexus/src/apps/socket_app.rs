// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Socket App (application-level socket management)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Socket domain
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppSocketDomain {
    Unix,
    Inet,
    Inet6,
    Netlink,
    Packet,
}

/// Socket type for apps layer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppSocketType {
    Stream,
    Dgram,
    Raw,
    SeqPacket,
}

/// Socket state at app level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppSocketState {
    Created,
    Bound,
    Listening,
    Connected,
    Accepted,
    Closing,
    Closed,
    Error,
}

/// A socket tracked at the app layer
#[derive(Debug, Clone)]
pub struct AppSocketEntry {
    pub fd: u64,
    pub pid: u64,
    pub domain: AppSocketDomain,
    pub sock_type: AppSocketType,
    pub state: AppSocketState,
    pub local_port: u16,
    pub remote_port: u16,
    pub bytes_sent: u64,
    pub bytes_recv: u64,
    pub send_calls: u64,
    pub recv_calls: u64,
    pub accept_count: u64,
    pub error_count: u64,
    pub created_tick: u64,
}

impl AppSocketEntry {
    pub fn new(fd: u64, pid: u64, domain: AppSocketDomain, sock_type: AppSocketType, tick: u64) -> Self {
        Self {
            fd, pid, domain, sock_type,
            state: AppSocketState::Created,
            local_port: 0, remote_port: 0,
            bytes_sent: 0, bytes_recv: 0,
            send_calls: 0, recv_calls: 0,
            accept_count: 0, error_count: 0,
            created_tick: tick,
        }
    }

    #[inline(always)]
    pub fn bind(&mut self, port: u16) {
        self.local_port = port;
        self.state = AppSocketState::Bound;
    }

    #[inline(always)]
    pub fn connect(&mut self, port: u16) {
        self.remote_port = port;
        self.state = AppSocketState::Connected;
    }

    #[inline(always)]
    pub fn send(&mut self, bytes: u64) {
        self.bytes_sent += bytes;
        self.send_calls += 1;
    }

    #[inline(always)]
    pub fn recv(&mut self, bytes: u64) {
        self.bytes_recv += bytes;
        self.recv_calls += 1;
    }
}

/// Statistics for socket app
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SocketAppStats {
    pub sockets_created: u64,
    pub sockets_closed: u64,
    pub connections: u64,
    pub binds: u64,
    pub accepts: u64,
    pub total_sent: u64,
    pub total_recv: u64,
    pub errors: u64,
}

/// Main socket app manager
#[derive(Debug)]
pub struct AppSocket {
    sockets: BTreeMap<u64, AppSocketEntry>,
    pid_sockets: BTreeMap<u64, Vec<u64>>,
    stats: SocketAppStats,
}

impl AppSocket {
    pub fn new() -> Self {
        Self {
            sockets: BTreeMap::new(),
            pid_sockets: BTreeMap::new(),
            stats: SocketAppStats {
                sockets_created: 0, sockets_closed: 0,
                connections: 0, binds: 0, accepts: 0,
                total_sent: 0, total_recv: 0, errors: 0,
            },
        }
    }

    #[inline]
    pub fn create_socket(&mut self, fd: u64, pid: u64, domain: AppSocketDomain, sock_type: AppSocketType, tick: u64) {
        self.sockets.insert(fd, AppSocketEntry::new(fd, pid, domain, sock_type, tick));
        self.pid_sockets.entry(pid).or_insert_with(Vec::new).push(fd);
        self.stats.sockets_created += 1;
    }

    #[inline]
    pub fn bind(&mut self, fd: u64, port: u16) -> bool {
        if let Some(sock) = self.sockets.get_mut(&fd) {
            sock.bind(port);
            self.stats.binds += 1;
            true
        } else { false }
    }

    #[inline]
    pub fn connect(&mut self, fd: u64, port: u16) -> bool {
        if let Some(sock) = self.sockets.get_mut(&fd) {
            sock.connect(port);
            self.stats.connections += 1;
            true
        } else { false }
    }

    #[inline]
    pub fn send(&mut self, fd: u64, bytes: u64) -> bool {
        if let Some(sock) = self.sockets.get_mut(&fd) {
            sock.send(bytes);
            self.stats.total_sent += bytes;
            true
        } else { false }
    }

    #[inline]
    pub fn recv(&mut self, fd: u64, bytes: u64) -> bool {
        if let Some(sock) = self.sockets.get_mut(&fd) {
            sock.recv(bytes);
            self.stats.total_recv += bytes;
            true
        } else { false }
    }

    #[inline]
    pub fn close_socket(&mut self, fd: u64) -> bool {
        if let Some(sock) = self.sockets.remove(&fd) {
            if let Some(fds) = self.pid_sockets.get_mut(&sock.pid) {
                fds.retain(|f| *f != fd);
            }
            self.stats.sockets_closed += 1;
            true
        } else { false }
    }

    #[inline(always)]
    pub fn stats(&self) -> &SocketAppStats {
        &self.stats
    }
}

// ============================================================================
// Merged from socket_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketV2Domain {
    Unix,
    Inet,
    Inet6,
    Netlink,
    Packet,
    Vsock,
    Bluetooth,
    Can,
    Xdp,
    Tipc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketV2Type {
    Stream,
    Dgram,
    Raw,
    Seqpacket,
    Rdm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketV2State {
    Unbound,
    Bound,
    Listening,
    Connecting,
    Connected,
    Closing,
    Closed,
    Error,
}

#[derive(Debug, Clone)]
pub struct SocketV2Instance {
    pub fd: u64,
    pub domain: SocketV2Domain,
    pub sock_type: SocketV2Type,
    pub protocol: u32,
    pub state: SocketV2State,
    pub nonblocking: bool,
    pub cloexec: bool,
    pub local_addr_hash: u64,
    pub remote_addr_hash: u64,
    pub send_buf_size: u32,
    pub recv_buf_size: u32,
    pub bytes_sent: u64,
    pub bytes_recv: u64,
    pub send_calls: u64,
    pub recv_calls: u64,
}

impl SocketV2Instance {
    pub fn new(fd: u64, domain: SocketV2Domain, sock_type: SocketV2Type, protocol: u32) -> Self {
        Self {
            fd, domain, sock_type, protocol,
            state: SocketV2State::Unbound,
            nonblocking: false, cloexec: false,
            local_addr_hash: 0, remote_addr_hash: 0,
            send_buf_size: 65536, recv_buf_size: 65536,
            bytes_sent: 0, bytes_recv: 0,
            send_calls: 0, recv_calls: 0,
        }
    }

    #[inline]
    pub fn bind(&mut self, addr: &[u8]) {
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in addr { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
        self.local_addr_hash = h;
        self.state = SocketV2State::Bound;
    }

    #[inline]
    pub fn connect(&mut self, addr: &[u8]) {
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in addr { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
        self.remote_addr_hash = h;
        self.state = SocketV2State::Connected;
    }

    #[inline(always)]
    pub fn send(&mut self, bytes: u64) {
        self.bytes_sent += bytes;
        self.send_calls += 1;
    }

    #[inline(always)]
    pub fn recv(&mut self, bytes: u64) {
        self.bytes_recv += bytes;
        self.recv_calls += 1;
    }

    #[inline(always)]
    pub fn avg_send_size(&self) -> u64 {
        if self.send_calls == 0 { 0 } else { self.bytes_sent / self.send_calls }
    }

    #[inline(always)]
    pub fn avg_recv_size(&self) -> u64 {
        if self.recv_calls == 0 { 0 } else { self.bytes_recv / self.recv_calls }
    }
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SocketV2AppStats {
    pub total_sockets: u64,
    pub active_connections: u64,
    pub total_bytes_sent: u64,
    pub total_bytes_recv: u64,
    pub total_errors: u64,
}

pub struct AppSocketV2 {
    sockets: BTreeMap<u64, SocketV2Instance>,
    stats: SocketV2AppStats,
}

impl AppSocketV2 {
    pub fn new() -> Self {
        Self {
            sockets: BTreeMap::new(),
            stats: SocketV2AppStats {
                total_sockets: 0, active_connections: 0,
                total_bytes_sent: 0, total_bytes_recv: 0,
                total_errors: 0,
            },
        }
    }

    #[inline(always)]
    pub fn create_socket(&mut self, fd: u64, domain: SocketV2Domain, sock_type: SocketV2Type, proto: u32) {
        self.sockets.insert(fd, SocketV2Instance::new(fd, domain, sock_type, proto));
        self.stats.total_sockets += 1;
    }

    #[inline]
    pub fn close_socket(&mut self, fd: u64) {
        if let Some(s) = self.sockets.get_mut(&fd) {
            s.state = SocketV2State::Closed;
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &SocketV2AppStats { &self.stats }
}

// ============================================================================
// Merged from socket_v3_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketV3Domain { Inet4, Inet6, Unix, Netlink, Packet, Vsock }

/// Socket v3 type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketV3Type { Stream, Dgram, Raw, Seqpacket }

/// Socket v3 request
#[derive(Debug, Clone)]
pub struct SocketV3Request {
    pub domain: SocketV3Domain,
    pub sock_type: SocketV3Type,
    pub protocol: u16,
    pub nonblock: bool,
    pub cloexec: bool,
}

impl SocketV3Request {
    pub fn new(domain: SocketV3Domain, sock_type: SocketV3Type) -> Self {
        Self { domain, sock_type, protocol: 0, nonblock: false, cloexec: false }
    }
}

/// Socket v3 app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SocketV3AppStats { pub total_creates: u64, pub streams: u64, pub dgrams: u64, pub failures: u64 }

/// Main app socket v3
#[derive(Debug)]
pub struct AppSocketV3 { pub stats: SocketV3AppStats }

impl AppSocketV3 {
    pub fn new() -> Self { Self { stats: SocketV3AppStats { total_creates: 0, streams: 0, dgrams: 0, failures: 0 } } }
    #[inline]
    pub fn request(&mut self, req: &SocketV3Request) -> i32 {
        self.stats.total_creates += 1;
        match req.sock_type {
            SocketV3Type::Stream | SocketV3Type::Seqpacket => self.stats.streams += 1,
            SocketV3Type::Dgram | SocketV3Type::Raw => self.stats.dgrams += 1,
        }
        self.stats.total_creates as i32
    }
}
