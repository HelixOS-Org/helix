//! # App Network Stack Profiler
//!
//! Network stack profiling per application:
//! - TCP state machine tracking
//! - Retransmission analysis
//! - Connection latency tracking
//! - Socket buffer profiling
//! - Bandwidth utilization per flow

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// TCP TYPES
// ============================================================================

/// TCP state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    /// Listening
    Listen,
    /// SYN sent
    SynSent,
    /// SYN received
    SynReceived,
    /// Established
    Established,
    /// FIN wait 1
    FinWait1,
    /// FIN wait 2
    FinWait2,
    /// Close wait
    CloseWait,
    /// Last ACK
    LastAck,
    /// Time wait
    TimeWait,
    /// Closed
    Closed,
}

/// Protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetProtocol {
    /// TCP
    Tcp,
    /// UDP
    Udp,
    /// Unix domain socket
    Unix,
    /// Raw socket
    Raw,
}

/// Connection direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnDirection {
    /// Incoming
    Inbound,
    /// Outgoing
    Outbound,
    /// Loopback
    Loopback,
}

// ============================================================================
// CONNECTION PROFILE
// ============================================================================

/// Connection profile
#[derive(Debug, Clone)]
pub struct ConnectionProfile {
    /// Connection ID (FNV-1a of 5-tuple)
    pub conn_id: u64,
    /// Protocol
    pub protocol: NetProtocol,
    /// Direction
    pub direction: ConnDirection,
    /// TCP state
    pub tcp_state: TcpState,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_recv: u64,
    /// Packets sent
    pub packets_sent: u64,
    /// Packets received
    pub packets_recv: u64,
    /// Retransmissions
    pub retransmits: u64,
    /// Duplicate ACKs
    pub dup_acks: u64,
    /// RTT EMA (ns)
    pub rtt_ema_ns: f64,
    /// RTT min (ns)
    pub rtt_min_ns: u64,
    /// RTT max (ns)
    pub rtt_max_ns: u64,
    /// Congestion window
    pub cwnd: u32,
    /// Send buffer size
    pub send_buf: u32,
    /// Recv buffer size
    pub recv_buf: u32,
    /// Connection established (ns)
    pub established_ns: u64,
    /// Last activity (ns)
    pub last_activity_ns: u64,
}

impl ConnectionProfile {
    pub fn new(conn_id: u64, protocol: NetProtocol, direction: ConnDirection, now: u64) -> Self {
        Self {
            conn_id,
            protocol,
            direction,
            tcp_state: TcpState::Closed,
            bytes_sent: 0,
            bytes_recv: 0,
            packets_sent: 0,
            packets_recv: 0,
            retransmits: 0,
            dup_acks: 0,
            rtt_ema_ns: 0.0,
            rtt_min_ns: u64::MAX,
            rtt_max_ns: 0,
            cwnd: 10,
            send_buf: 65536,
            recv_buf: 65536,
            established_ns: now,
            last_activity_ns: now,
        }
    }

    /// Record send
    pub fn record_send(&mut self, bytes: u64, now: u64) {
        self.bytes_sent += bytes;
        self.packets_sent += 1;
        self.last_activity_ns = now;
    }

    /// Record recv
    pub fn record_recv(&mut self, bytes: u64, now: u64) {
        self.bytes_recv += bytes;
        self.packets_recv += 1;
        self.last_activity_ns = now;
    }

    /// Record RTT sample
    pub fn record_rtt(&mut self, rtt_ns: u64) {
        self.rtt_ema_ns = 0.875 * self.rtt_ema_ns + 0.125 * rtt_ns as f64;
        if rtt_ns < self.rtt_min_ns {
            self.rtt_min_ns = rtt_ns;
        }
        if rtt_ns > self.rtt_max_ns {
            self.rtt_max_ns = rtt_ns;
        }
    }

    /// Record retransmit
    pub fn record_retransmit(&mut self) {
        self.retransmits += 1;
    }

    /// Retransmit rate
    pub fn retransmit_rate(&self) -> f64 {
        if self.packets_sent == 0 {
            return 0.0;
        }
        self.retransmits as f64 / self.packets_sent as f64
    }

    /// Throughput (bytes/sec)
    pub fn send_throughput(&self, now: u64) -> f64 {
        let elapsed = now.saturating_sub(self.established_ns);
        if elapsed == 0 {
            return 0.0;
        }
        self.bytes_sent as f64 / (elapsed as f64 / 1_000_000_000.0)
    }

    /// Recv throughput
    pub fn recv_throughput(&self, now: u64) -> f64 {
        let elapsed = now.saturating_sub(self.established_ns);
        if elapsed == 0 {
            return 0.0;
        }
        self.bytes_recv as f64 / (elapsed as f64 / 1_000_000_000.0)
    }

    /// Is idle
    pub fn is_idle(&self, now: u64, timeout_ns: u64) -> bool {
        now.saturating_sub(self.last_activity_ns) > timeout_ns
    }
}

// ============================================================================
// SOCKET BUFFER PROFILING
// ============================================================================

/// Socket buffer stats
#[derive(Debug, Clone, Default)]
pub struct SocketBufferStats {
    /// Send buffer full events
    pub send_full_events: u64,
    /// Recv buffer full events
    pub recv_full_events: u64,
    /// Send buffer avg utilization
    pub send_utilization: f64,
    /// Recv buffer avg utilization
    pub recv_utilization: f64,
    /// Buffer resizes
    pub resizes: u64,
}

// ============================================================================
// PER-PROCESS NETWORK
// ============================================================================

/// Per-process network profile
#[derive(Debug)]
pub struct ProcessNetProfile {
    /// PID
    pub pid: u64,
    /// Connections
    connections: BTreeMap<u64, ConnectionProfile>,
    /// Socket buffer stats
    pub buffer_stats: SocketBufferStats,
    /// Total bytes sent
    pub total_sent: u64,
    /// Total bytes received
    pub total_recv: u64,
    /// Active connections
    pub active_connections: usize,
}

impl ProcessNetProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            connections: BTreeMap::new(),
            buffer_stats: SocketBufferStats::default(),
            total_sent: 0,
            total_recv: 0,
            active_connections: 0,
        }
    }

    /// Add/get connection
    pub fn connection(&mut self, conn_id: u64, protocol: NetProtocol, direction: ConnDirection, now: u64) -> &mut ConnectionProfile {
        self.connections.entry(conn_id)
            .or_insert_with(|| ConnectionProfile::new(conn_id, protocol, direction, now))
    }

    /// Get connection
    pub fn get_connection(&self, conn_id: u64) -> Option<&ConnectionProfile> {
        self.connections.get(&conn_id)
    }

    /// Remove connection
    pub fn remove_connection(&mut self, conn_id: u64) {
        self.connections.remove(&conn_id);
    }

    /// Cleanup idle connections
    pub fn cleanup_idle(&mut self, now: u64, timeout_ns: u64) {
        self.connections.retain(|_, c| !c.is_idle(now, timeout_ns));
        self.active_connections = self.connections.len();
    }

    /// Update aggregates
    pub fn update_aggregates(&mut self) {
        self.total_sent = self.connections.values().map(|c| c.bytes_sent).sum();
        self.total_recv = self.connections.values().map(|c| c.bytes_recv).sum();
        self.active_connections = self.connections.values()
            .filter(|c| c.tcp_state == TcpState::Established)
            .count();
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Network profiler stats
#[derive(Debug, Clone, Default)]
pub struct AppNetProfilerStats {
    /// Tracked processes
    pub tracked_processes: usize,
    /// Total connections
    pub total_connections: usize,
    /// Total bytes transferred
    pub total_bytes: u64,
    /// High retransmit connections
    pub high_retransmit: usize,
}

/// App network stack profiler
pub struct AppNetStackProfiler {
    /// Per-process profiles
    processes: BTreeMap<u64, ProcessNetProfile>,
    /// Stats
    stats: AppNetProfilerStats,
}

impl AppNetStackProfiler {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            stats: AppNetProfilerStats::default(),
        }
    }

    /// Get/create process profile
    pub fn process(&mut self, pid: u64) -> &mut ProcessNetProfile {
        self.processes.entry(pid).or_insert_with(|| ProcessNetProfile::new(pid))
    }

    /// Record send
    pub fn record_send(&mut self, pid: u64, conn_id: u64, bytes: u64, now: u64) {
        let proc = self.processes.entry(pid).or_insert_with(|| ProcessNetProfile::new(pid));
        let conn = proc.connection(conn_id, NetProtocol::Tcp, ConnDirection::Outbound, now);
        conn.record_send(bytes, now);
        self.update_stats();
    }

    /// Record recv
    pub fn record_recv(&mut self, pid: u64, conn_id: u64, bytes: u64, now: u64) {
        let proc = self.processes.entry(pid).or_insert_with(|| ProcessNetProfile::new(pid));
        let conn = proc.connection(conn_id, NetProtocol::Tcp, ConnDirection::Inbound, now);
        conn.record_recv(bytes, now);
        self.update_stats();
    }

    /// Remove process
    pub fn remove_process(&mut self, pid: u64) {
        self.processes.remove(&pid);
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.total_connections = self.processes.values()
            .map(|p| p.connections.len())
            .sum();
        self.stats.total_bytes = self.processes.values()
            .map(|p| p.connections.values().map(|c| c.bytes_sent + c.bytes_recv).sum::<u64>())
            .sum();
        self.stats.high_retransmit = self.processes.values()
            .flat_map(|p| p.connections.values())
            .filter(|c| c.retransmit_rate() > 0.05)
            .count();
    }

    /// Stats
    pub fn stats(&self) -> &AppNetProfilerStats {
        &self.stats
    }
}
