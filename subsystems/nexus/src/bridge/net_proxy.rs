//! # Bridge Network Proxy
//!
//! Network syscall optimization layer:
//! - Socket operation batching
//! - Connection tracking
//! - Send/recv buffer tuning hints
//! - TCP state machine awareness
//! - Syscall coalescing for scatter/gather

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Socket type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeSocketType {
    TcpStream,
    TcpListener,
    UdpSocket,
    UnixStream,
    UnixDgram,
    Raw,
    Netlink,
}

/// Socket state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeSocketState {
    Created,
    Bound,
    Listening,
    Connecting,
    Connected,
    Closing,
    Closed,
}

/// Network syscall type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetSyscallType {
    Socket,
    Bind,
    Listen,
    Accept,
    Connect,
    Send,
    Recv,
    Sendto,
    Recvfrom,
    Sendmsg,
    Recvmsg,
    Shutdown,
    Close,
    Setsockopt,
    Getsockopt,
    Poll,
    Epoll,
    Select,
}

/// Socket tracking entry
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SocketEntry {
    /// File descriptor
    pub fd: u32,
    /// PID
    pub pid: u64,
    /// Socket type
    pub sock_type: BridgeSocketType,
    /// State
    pub state: BridgeSocketState,
    /// Send bytes total
    pub bytes_sent: u64,
    /// Recv bytes total
    pub bytes_recv: u64,
    /// Send operations
    pub send_ops: u64,
    /// Recv operations
    pub recv_ops: u64,
    /// Average send size
    pub avg_send_size: f64,
    /// Average recv size
    pub avg_recv_size: f64,
    /// Send buffer size (bytes)
    pub sndbuf: u32,
    /// Recv buffer size (bytes)
    pub rcvbuf: u32,
    /// Created timestamp
    pub created_ns: u64,
    /// Last activity
    pub last_activity_ns: u64,
}

impl SocketEntry {
    pub fn new(fd: u32, pid: u64, sock_type: BridgeSocketType, now_ns: u64) -> Self {
        Self {
            fd,
            pid,
            sock_type,
            state: BridgeSocketState::Created,
            bytes_sent: 0,
            bytes_recv: 0,
            send_ops: 0,
            recv_ops: 0,
            avg_send_size: 0.0,
            avg_recv_size: 0.0,
            sndbuf: 87380,
            rcvbuf: 87380,
            created_ns: now_ns,
            last_activity_ns: now_ns,
        }
    }

    /// Record send
    #[inline]
    pub fn record_send(&mut self, bytes: u64, now_ns: u64) {
        self.bytes_sent += bytes;
        self.send_ops += 1;
        self.avg_send_size = 0.9 * self.avg_send_size + 0.1 * bytes as f64;
        self.last_activity_ns = now_ns;
    }

    /// Record recv
    #[inline]
    pub fn record_recv(&mut self, bytes: u64, now_ns: u64) {
        self.bytes_recv += bytes;
        self.recv_ops += 1;
        self.avg_recv_size = 0.9 * self.avg_recv_size + 0.1 * bytes as f64;
        self.last_activity_ns = now_ns;
    }

    /// Suggested send buffer size
    #[inline]
    pub fn suggested_sndbuf(&self) -> u32 {
        let avg = self.avg_send_size;
        if avg > 65536.0 {
            262144
        } else if avg > 4096.0 {
            131072
        } else {
            87380
        }
    }

    /// Suggested recv buffer size
    #[inline]
    pub fn suggested_rcvbuf(&self) -> u32 {
        let avg = self.avg_recv_size;
        if avg > 65536.0 {
            262144
        } else if avg > 4096.0 {
            131072
        } else {
            87380
        }
    }

    /// Is idle?
    #[inline(always)]
    pub fn is_idle(&self, now_ns: u64, threshold_ns: u64) -> bool {
        now_ns.saturating_sub(self.last_activity_ns) > threshold_ns
    }

    /// Throughput estimate (bytes/sec based on total)
    #[inline]
    pub fn throughput_estimate(&self, now_ns: u64) -> f64 {
        let elapsed = now_ns.saturating_sub(self.created_ns) as f64 / 1_000_000_000.0;
        if elapsed > 0.0 {
            (self.bytes_sent + self.bytes_recv) as f64 / elapsed
        } else {
            0.0
        }
    }
}

/// Sendmsg coalescing opportunity
#[derive(Debug, Clone)]
pub struct CoalesceOpportunity {
    /// Socket FD
    pub fd: u32,
    /// Number of small sends that could be coalesced
    pub small_sends: u64,
    /// Potential savings (fewer syscalls)
    pub potential_savings: u64,
}

/// Network proxy stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BridgeNetProxyStats {
    pub tracked_sockets: usize,
    pub active_connections: usize,
    pub total_bytes_sent: u64,
    pub total_bytes_recv: u64,
    pub coalesce_opportunities: usize,
    pub buffer_tune_needed: usize,
}

/// Bridge network proxy
#[repr(align(64))]
pub struct BridgeNetProxy {
    /// Sockets (FD -> entry)
    sockets: BTreeMap<u64, SocketEntry>,
    /// Stats
    stats: BridgeNetProxyStats,
}

impl BridgeNetProxy {
    pub fn new() -> Self {
        Self {
            sockets: BTreeMap::new(),
            stats: BridgeNetProxyStats::default(),
        }
    }

    fn socket_key(pid: u64, fd: u32) -> u64 {
        (pid << 32) | fd as u64
    }

    /// Track socket creation
    #[inline]
    pub fn track_socket(&mut self, pid: u64, fd: u32, sock_type: BridgeSocketType, now_ns: u64) {
        let key = Self::socket_key(pid, fd);
        self.sockets.insert(key, SocketEntry::new(fd, pid, sock_type, now_ns));
        self.update_stats();
    }

    /// Update socket state
    #[inline]
    pub fn update_state(&mut self, pid: u64, fd: u32, state: BridgeSocketState) {
        let key = Self::socket_key(pid, fd);
        if let Some(entry) = self.sockets.get_mut(&key) {
            entry.state = state;
        }
        self.update_stats();
    }

    /// Record send
    #[inline]
    pub fn record_send(&mut self, pid: u64, fd: u32, bytes: u64, now_ns: u64) {
        let key = Self::socket_key(pid, fd);
        if let Some(entry) = self.sockets.get_mut(&key) {
            entry.record_send(bytes, now_ns);
        }
        self.update_stats();
    }

    /// Record recv
    #[inline]
    pub fn record_recv(&mut self, pid: u64, fd: u32, bytes: u64, now_ns: u64) {
        let key = Self::socket_key(pid, fd);
        if let Some(entry) = self.sockets.get_mut(&key) {
            entry.record_recv(bytes, now_ns);
        }
        self.update_stats();
    }

    /// Remove socket
    #[inline]
    pub fn remove_socket(&mut self, pid: u64, fd: u32) {
        let key = Self::socket_key(pid, fd);
        self.sockets.remove(&key);
        self.update_stats();
    }

    /// Find coalescing opportunities (many small sends)
    pub fn coalesce_opportunities(&self) -> Vec<CoalesceOpportunity> {
        self.sockets.values()
            .filter(|s| s.send_ops > 10 && s.avg_send_size < 256.0)
            .map(|s| {
                let small = (s.send_ops as f64 * 0.8) as u64;
                CoalesceOpportunity {
                    fd: s.fd,
                    small_sends: small,
                    potential_savings: small / 2,
                }
            })
            .collect()
    }

    /// Sockets needing buffer tuning
    #[inline]
    pub fn buffer_tune_needed(&self) -> Vec<(u32, u32, u32)> {
        self.sockets.values()
            .filter(|s| s.suggested_sndbuf() != s.sndbuf || s.suggested_rcvbuf() != s.rcvbuf)
            .map(|s| (s.fd, s.suggested_sndbuf(), s.suggested_rcvbuf()))
            .collect()
    }

    fn update_stats(&mut self) {
        self.stats.tracked_sockets = self.sockets.len();
        self.stats.active_connections = self.sockets.values()
            .filter(|s| matches!(s.state, BridgeSocketState::Connected | BridgeSocketState::Listening))
            .count();
        self.stats.total_bytes_sent = self.sockets.values().map(|s| s.bytes_sent).sum();
        self.stats.total_bytes_recv = self.sockets.values().map(|s| s.bytes_recv).sum();
        self.stats.coalesce_opportunities = self.coalesce_opportunities().len();
        self.stats.buffer_tune_needed = self.buffer_tune_needed().len();
    }

    #[inline(always)]
    pub fn stats(&self) -> &BridgeNetProxyStats {
        &self.stats
    }
}
