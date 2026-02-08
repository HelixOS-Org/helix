//! # Bridge Socket Manager
//!
//! Socket syscall proxying bridge:
//! - Socket lifecycle (create, bind, listen, accept, close)
//! - Connection state machine (TCP-like)
//! - Per-socket buffer management
//! - Sendmsg/recvmsg scatter-gather tracking
//! - Socket option management
//! - Network namespace binding

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Socket domain
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketDomain {
    Unix,
    Inet,
    Inet6,
    Netlink,
    Packet,
    Vsock,
}

/// Socket type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketType {
    Stream,
    Dgram,
    SeqPacket,
    Raw,
}

/// Socket state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketState {
    Unbound,
    Bound,
    Listening,
    Connecting,
    Connected,
    Closing,
    Closed,
    Error,
}

/// Socket address
#[derive(Debug, Clone)]
pub struct SockAddr {
    pub domain: SocketDomain,
    pub port: u16,
    pub addr_bytes: [u8; 16],
    pub path_hash: u64,
}

impl SockAddr {
    pub fn inet4(addr: [u8; 4], port: u16) -> Self {
        let mut bytes = [0u8; 16];
        bytes[..4].copy_from_slice(&addr);
        Self { domain: SocketDomain::Inet, port, addr_bytes: bytes, path_hash: 0 }
    }

    pub fn inet6(addr: [u8; 16], port: u16) -> Self {
        Self { domain: SocketDomain::Inet6, port, addr_bytes: addr, path_hash: 0 }
    }

    pub fn unix(path_hash: u64) -> Self {
        Self { domain: SocketDomain::Unix, port: 0, addr_bytes: [0; 16], path_hash }
    }
}

/// Socket buffer stats
#[derive(Debug, Clone, Copy, Default)]
pub struct SockBufStats {
    pub send_buf_size: u64,
    pub recv_buf_size: u64,
    pub send_buf_used: u64,
    pub recv_buf_used: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub send_calls: u64,
    pub recv_calls: u64,
}

impl SockBufStats {
    pub fn send_buf_util(&self) -> f64 {
        if self.send_buf_size == 0 { return 0.0; }
        self.send_buf_used as f64 / self.send_buf_size as f64
    }
    pub fn recv_buf_util(&self) -> f64 {
        if self.recv_buf_size == 0 { return 0.0; }
        self.recv_buf_used as f64 / self.recv_buf_size as f64
    }
}

/// Socket options
#[derive(Debug, Clone, Copy)]
pub struct SockOptions {
    pub reuse_addr: bool,
    pub reuse_port: bool,
    pub keepalive: bool,
    pub keepalive_secs: u32,
    pub no_delay: bool,
    pub linger: Option<u32>,
    pub nonblocking: bool,
    pub broadcast: bool,
    pub recv_timeout_ms: u32,
    pub send_timeout_ms: u32,
}

impl SockOptions {
    pub fn default_stream() -> Self {
        Self {
            reuse_addr: false, reuse_port: false,
            keepalive: false, keepalive_secs: 7200,
            no_delay: false, linger: None,
            nonblocking: false, broadcast: false,
            recv_timeout_ms: 0, send_timeout_ms: 0,
        }
    }
}

/// Connection info (for connected sockets)
#[derive(Debug, Clone)]
pub struct ConnInfo {
    pub local: SockAddr,
    pub remote: SockAddr,
    pub connected_ns: u64,
    pub rtt_us: u32,
    pub retransmits: u32,
    pub congestion_window: u32,
    pub slow_start_threshold: u32,
}

/// Socket descriptor
#[derive(Debug, Clone)]
pub struct BridgeSocket {
    pub fd: i32,
    pub owner_pid: u64,
    pub domain: SocketDomain,
    pub sock_type: SocketType,
    pub state: SocketState,
    pub local_addr: Option<SockAddr>,
    pub conn: Option<ConnInfo>,
    pub buf_stats: SockBufStats,
    pub options: SockOptions,
    pub backlog: u32,
    pub accept_count: u64,
    pub error_count: u64,
    pub created_ns: u64,
    pub namespace_id: u64,
}

impl BridgeSocket {
    pub fn new(fd: i32, pid: u64, domain: SocketDomain, stype: SocketType, ts: u64) -> Self {
        Self {
            fd, owner_pid: pid, domain, sock_type: stype,
            state: SocketState::Unbound,
            local_addr: None, conn: None,
            buf_stats: SockBufStats { send_buf_size: 212992, recv_buf_size: 212992, ..Default::default() },
            options: SockOptions::default_stream(),
            backlog: 0, accept_count: 0, error_count: 0,
            created_ns: ts, namespace_id: 0,
        }
    }

    pub fn bind(&mut self, addr: SockAddr) -> bool {
        if self.state != SocketState::Unbound { return false; }
        self.local_addr = Some(addr);
        self.state = SocketState::Bound;
        true
    }

    pub fn listen(&mut self, backlog: u32) -> bool {
        if self.state != SocketState::Bound { return false; }
        self.backlog = backlog;
        self.state = SocketState::Listening;
        true
    }

    pub fn connect(&mut self, remote: SockAddr, ts: u64) -> bool {
        if self.state != SocketState::Unbound && self.state != SocketState::Bound { return false; }
        let local = self.local_addr.clone().unwrap_or(SockAddr::inet4([0; 4], 0));
        self.conn = Some(ConnInfo {
            local, remote, connected_ns: ts,
            rtt_us: 0, retransmits: 0,
            congestion_window: 10, slow_start_threshold: 65535,
        });
        self.state = SocketState::Connected;
        true
    }

    pub fn record_send(&mut self, bytes: u64) {
        self.buf_stats.bytes_sent += bytes;
        self.buf_stats.send_calls += 1;
    }

    pub fn record_recv(&mut self, bytes: u64) {
        self.buf_stats.bytes_received += bytes;
        self.buf_stats.recv_calls += 1;
    }

    pub fn close(&mut self) {
        self.state = SocketState::Closed;
    }

    pub fn is_listening(&self) -> bool { self.state == SocketState::Listening }
    pub fn is_connected(&self) -> bool { self.state == SocketState::Connected }
}

/// Bridge socket manager stats
#[derive(Debug, Clone, Default)]
pub struct BridgeSocketStats {
    pub total_sockets: usize,
    pub listening_count: usize,
    pub connected_count: usize,
    pub total_bytes_sent: u64,
    pub total_bytes_recv: u64,
    pub total_errors: u64,
}

/// Bridge Socket Manager
pub struct BridgeSocketBridge {
    sockets: BTreeMap<i32, BridgeSocket>,
    process_sockets: BTreeMap<u64, Vec<i32>>,
    stats: BridgeSocketStats,
    next_fd: i32,
}

impl BridgeSocketBridge {
    pub fn new() -> Self {
        Self {
            sockets: BTreeMap::new(),
            process_sockets: BTreeMap::new(),
            stats: BridgeSocketStats::default(),
            next_fd: 3000,
        }
    }

    pub fn socket_create(&mut self, pid: u64, domain: SocketDomain, stype: SocketType, ts: u64) -> i32 {
        let fd = self.next_fd;
        self.next_fd += 1;
        self.sockets.insert(fd, BridgeSocket::new(fd, pid, domain, stype, ts));
        self.process_sockets.entry(pid).or_insert_with(Vec::new).push(fd);
        fd
    }

    pub fn socket_bind(&mut self, fd: i32, addr: SockAddr) -> bool {
        self.sockets.get_mut(&fd).map(|s| s.bind(addr)).unwrap_or(false)
    }

    pub fn socket_listen(&mut self, fd: i32, backlog: u32) -> bool {
        self.sockets.get_mut(&fd).map(|s| s.listen(backlog)).unwrap_or(false)
    }

    pub fn socket_connect(&mut self, fd: i32, remote: SockAddr, ts: u64) -> bool {
        self.sockets.get_mut(&fd).map(|s| s.connect(remote, ts)).unwrap_or(false)
    }

    pub fn socket_send(&mut self, fd: i32, bytes: u64) {
        if let Some(s) = self.sockets.get_mut(&fd) { s.record_send(bytes); }
    }

    pub fn socket_recv(&mut self, fd: i32, bytes: u64) {
        if let Some(s) = self.sockets.get_mut(&fd) { s.record_recv(bytes); }
    }

    pub fn socket_close(&mut self, fd: i32) {
        if let Some(s) = self.sockets.get_mut(&fd) {
            let pid = s.owner_pid;
            s.close();
            if let Some(fds) = self.process_sockets.get_mut(&pid) {
                fds.retain(|&f| f != fd);
            }
        }
    }

    pub fn socket_accept(&mut self, listen_fd: i32, remote: SockAddr, pid: u64, ts: u64) -> Option<i32> {
        if let Some(listener) = self.sockets.get_mut(&listen_fd) {
            if !listener.is_listening() { return None; }
            listener.accept_count += 1;
        } else { return None; }

        let new_fd = self.next_fd;
        self.next_fd += 1;
        let domain = self.sockets.get(&listen_fd).map(|s| s.domain).unwrap_or(SocketDomain::Inet);
        let stype = self.sockets.get(&listen_fd).map(|s| s.sock_type).unwrap_or(SocketType::Stream);
        let mut new_sock = BridgeSocket::new(new_fd, pid, domain, stype, ts);
        let local = self.sockets.get(&listen_fd).and_then(|s| s.local_addr.clone())
            .unwrap_or(SockAddr::inet4([0; 4], 0));
        new_sock.conn = Some(ConnInfo {
            local, remote, connected_ns: ts,
            rtt_us: 0, retransmits: 0,
            congestion_window: 10, slow_start_threshold: 65535,
        });
        new_sock.state = SocketState::Connected;
        self.sockets.insert(new_fd, new_sock);
        self.process_sockets.entry(pid).or_insert_with(Vec::new).push(new_fd);
        Some(new_fd)
    }

    pub fn recompute(&mut self) {
        self.stats.total_sockets = self.sockets.len();
        self.stats.listening_count = self.sockets.values().filter(|s| s.is_listening()).count();
        self.stats.connected_count = self.sockets.values().filter(|s| s.is_connected()).count();
        self.stats.total_bytes_sent = self.sockets.values().map(|s| s.buf_stats.bytes_sent).sum();
        self.stats.total_bytes_recv = self.sockets.values().map(|s| s.buf_stats.bytes_received).sum();
        self.stats.total_errors = self.sockets.values().map(|s| s.error_count).sum();
    }

    pub fn socket(&self, fd: i32) -> Option<&BridgeSocket> { self.sockets.get(&fd) }
    pub fn stats(&self) -> &BridgeSocketStats { &self.stats }
}

// ============================================================================
// Merged from socket_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketV2Family {
    Unix,
    Inet,
    Inet6,
    Netlink,
    Packet,
    Vsock,
    Bluetooth,
    Can,
    Tipc,
}

/// Socket type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketV2Type {
    Stream,
    Dgram,
    Raw,
    Rdm,
    SeqPacket,
    Dccp,
}

/// Socket state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketV2State {
    Unbound,
    Bound,
    Listening,
    Connecting,
    Connected,
    Closing,
    Closed,
    TimeWait,
}

/// Socket options
#[derive(Debug, Clone)]
pub struct SocketV2Options {
    pub reuse_addr: bool,
    pub reuse_port: bool,
    pub keep_alive: bool,
    pub no_delay: bool,
    pub broadcast: bool,
    pub linger_secs: Option<u32>,
    pub recv_buf_size: usize,
    pub send_buf_size: usize,
    pub recv_timeout_ms: u64,
    pub send_timeout_ms: u64,
}

impl SocketV2Options {
    pub fn default_opts() -> Self {
        Self {
            reuse_addr: false,
            reuse_port: false,
            keep_alive: false,
            no_delay: false,
            broadcast: false,
            linger_secs: None,
            recv_buf_size: 65536,
            send_buf_size: 65536,
            recv_timeout_ms: 0,
            send_timeout_ms: 0,
        }
    }
}

/// A socket instance
#[derive(Debug, Clone)]
pub struct SocketV2Instance {
    pub fd: u64,
    pub family: SocketV2Family,
    pub sock_type: SocketV2Type,
    pub state: SocketV2State,
    pub options: SocketV2Options,
    pub local_port: u16,
    pub remote_port: u16,
    pub local_addr: u64,
    pub remote_addr: u64,
    pub backlog: u32,
    pub bytes_sent: u64,
    pub bytes_recv: u64,
    pub packets_sent: u64,
    pub packets_recv: u64,
    pub errors: u64,
    pub created_tick: u64,
}

impl SocketV2Instance {
    pub fn new(fd: u64, family: SocketV2Family, sock_type: SocketV2Type, tick: u64) -> Self {
        Self {
            fd,
            family,
            sock_type,
            state: SocketV2State::Unbound,
            options: SocketV2Options::default_opts(),
            local_port: 0,
            remote_port: 0,
            local_addr: 0,
            remote_addr: 0,
            backlog: 0,
            bytes_sent: 0,
            bytes_recv: 0,
            packets_sent: 0,
            packets_recv: 0,
            errors: 0,
            created_tick: tick,
        }
    }

    pub fn bind(&mut self, addr: u64, port: u16) {
        self.local_addr = addr;
        self.local_port = port;
        self.state = SocketV2State::Bound;
    }

    pub fn listen(&mut self, backlog: u32) {
        self.backlog = backlog;
        self.state = SocketV2State::Listening;
    }

    pub fn connect(&mut self, addr: u64, port: u16) {
        self.remote_addr = addr;
        self.remote_port = port;
        self.state = SocketV2State::Connected;
    }

    pub fn send(&mut self, bytes: u64) {
        self.bytes_sent += bytes;
        self.packets_sent += 1;
    }

    pub fn recv(&mut self, bytes: u64) {
        self.bytes_recv += bytes;
        self.packets_recv += 1;
    }

    pub fn close(&mut self) {
        self.state = SocketV2State::Closed;
    }
}

/// Statistics for socket V2 bridge
#[derive(Debug, Clone)]
pub struct SocketV2BridgeStats {
    pub sockets_created: u64,
    pub sockets_closed: u64,
    pub binds: u64,
    pub listens: u64,
    pub connects: u64,
    pub accepts: u64,
    pub total_bytes_sent: u64,
    pub total_bytes_recv: u64,
    pub errors: u64,
}

/// Main socket V2 bridge manager
#[derive(Debug)]
pub struct BridgeSocketV2 {
    sockets: BTreeMap<u64, SocketV2Instance>,
    next_fd: u64,
    stats: SocketV2BridgeStats,
}

impl BridgeSocketV2 {
    pub fn new() -> Self {
        Self {
            sockets: BTreeMap::new(),
            next_fd: 3,
            stats: SocketV2BridgeStats {
                sockets_created: 0,
                sockets_closed: 0,
                binds: 0,
                listens: 0,
                connects: 0,
                accepts: 0,
                total_bytes_sent: 0,
                total_bytes_recv: 0,
                errors: 0,
            },
        }
    }

    pub fn create_socket(&mut self, family: SocketV2Family, sock_type: SocketV2Type, tick: u64) -> u64 {
        let fd = self.next_fd;
        self.next_fd += 1;
        self.sockets.insert(fd, SocketV2Instance::new(fd, family, sock_type, tick));
        self.stats.sockets_created += 1;
        fd
    }

    pub fn bind(&mut self, fd: u64, addr: u64, port: u16) -> bool {
        if let Some(sock) = self.sockets.get_mut(&fd) {
            sock.bind(addr, port);
            self.stats.binds += 1;
            true
        } else {
            false
        }
    }

    pub fn listen(&mut self, fd: u64, backlog: u32) -> bool {
        if let Some(sock) = self.sockets.get_mut(&fd) {
            sock.listen(backlog);
            self.stats.listens += 1;
            true
        } else {
            false
        }
    }

    pub fn connect(&mut self, fd: u64, addr: u64, port: u16) -> bool {
        if let Some(sock) = self.sockets.get_mut(&fd) {
            sock.connect(addr, port);
            self.stats.connects += 1;
            true
        } else {
            false
        }
    }

    pub fn send(&mut self, fd: u64, bytes: u64) -> bool {
        if let Some(sock) = self.sockets.get_mut(&fd) {
            sock.send(bytes);
            self.stats.total_bytes_sent += bytes;
            true
        } else {
            false
        }
    }

    pub fn recv(&mut self, fd: u64, bytes: u64) -> bool {
        if let Some(sock) = self.sockets.get_mut(&fd) {
            sock.recv(bytes);
            self.stats.total_bytes_recv += bytes;
            true
        } else {
            false
        }
    }

    pub fn stats(&self) -> &SocketV2BridgeStats {
        &self.stats
    }
}

// ============================================================================
// Merged from socket_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketV3Event { Create, Bind, Listen, Accept, Connect, Shutdown, Close }

/// Socket bridge v3 record
#[derive(Debug, Clone)]
pub struct SocketV3Record {
    pub event: SocketV3Event,
    pub domain: u16,
    pub sock_type: u16,
    pub protocol: u16,
    pub fd: i32,
}

impl SocketV3Record {
    pub fn new(event: SocketV3Event, fd: i32) -> Self { Self { event, domain: 0, sock_type: 0, protocol: 0, fd } }
}

/// Socket bridge v3 stats
#[derive(Debug, Clone)]
pub struct SocketV3BridgeStats {
    pub total_events: u64,
    pub creates: u64,
    pub connections: u64,
    pub shutdowns: u64,
}

/// Main bridge socket v3
#[derive(Debug)]
pub struct BridgeSocketV3 {
    pub stats: SocketV3BridgeStats,
    pub active_fds: Vec<i32>,
}

impl BridgeSocketV3 {
    pub fn new() -> Self {
        Self { stats: SocketV3BridgeStats { total_events: 0, creates: 0, connections: 0, shutdowns: 0 }, active_fds: Vec::new() }
    }
    pub fn record(&mut self, rec: &SocketV3Record) {
        self.stats.total_events += 1;
        match rec.event {
            SocketV3Event::Create => { self.stats.creates += 1; self.active_fds.push(rec.fd); }
            SocketV3Event::Connect | SocketV3Event::Accept => self.stats.connections += 1,
            SocketV3Event::Shutdown | SocketV3Event::Close => {
                self.stats.shutdowns += 1;
                self.active_fds.retain(|&f| f != rec.fd);
            }
            _ => {}
        }
    }
}
