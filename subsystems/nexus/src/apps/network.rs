//! # Application Network Behavior Analysis
//!
//! Understanding and optimizing application network patterns:
//! - Connection pattern analysis
//! - Protocol detection and optimization
//! - Bandwidth estimation per application
//! - Socket pooling recommendations
//! - Network QoS classification
//! - Latency-sensitive flow detection

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// NETWORK PATTERN
// ============================================================================

/// Network pattern type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AppNetworkPattern {
    /// Client-server request/response
    RequestResponse,
    /// Long-lived streaming
    Streaming,
    /// Short-lived connections
    ShortLived,
    /// Peer-to-peer
    PeerToPeer,
    /// Multicast/broadcast
    Multicast,
    /// Mostly idle with bursts
    Bursty,
    /// Persistent connections with keep-alive
    Persistent,
    /// DNS-heavy (many lookups)
    DnsHeavy,
    /// No network activity
    None,
}

/// Protocol type detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DetectedProtocol {
    Http,
    Https,
    WebSocket,
    Dns,
    Ssh,
    Ftp,
    Smtp,
    Imap,
    Grpc,
    Custom,
    Unknown,
}

// ============================================================================
// CONNECTION TRACKING
// ============================================================================

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnState {
    Connecting,
    Established,
    Closing,
    Closed,
    TimedOut,
    Error,
}

/// Tracked connection
#[derive(Debug, Clone)]
pub struct TrackedConnection {
    /// Connection ID
    pub id: u64,
    /// Process ID
    pub pid: u64,
    /// Remote address hash
    pub remote_hash: u64,
    /// Remote port
    pub remote_port: u16,
    /// Local port
    pub local_port: u16,
    /// State
    pub state: ConnState,
    /// Protocol
    pub protocol: DetectedProtocol,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Packets sent
    pub packets_sent: u64,
    /// Packets received
    pub packets_received: u64,
    /// Created timestamp
    pub created_at: u64,
    /// Last activity
    pub last_activity: u64,
    /// Latency samples (RTT microseconds)
    rtt_samples: Vec<u32>,
}

impl TrackedConnection {
    pub fn new(
        id: u64,
        pid: u64,
        remote_hash: u64,
        remote_port: u16,
        local_port: u16,
        timestamp: u64,
    ) -> Self {
        Self {
            id,
            pid,
            remote_hash,
            remote_port,
            local_port,
            state: ConnState::Connecting,
            protocol: DetectedProtocol::Unknown,
            bytes_sent: 0,
            bytes_received: 0,
            packets_sent: 0,
            packets_received: 0,
            created_at: timestamp,
            last_activity: timestamp,
            rtt_samples: Vec::new(),
        }
    }

    /// Record RTT sample
    pub fn record_rtt(&mut self, rtt_us: u32) {
        self.rtt_samples.push(rtt_us);
        if self.rtt_samples.len() > 100 {
            self.rtt_samples.remove(0);
        }
    }

    /// Average RTT
    pub fn avg_rtt_us(&self) -> u32 {
        if self.rtt_samples.is_empty() {
            return 0;
        }
        let sum: u64 = self.rtt_samples.iter().map(|&r| r as u64).sum();
        (sum / self.rtt_samples.len() as u64) as u32
    }

    /// Duration (ms)
    pub fn duration_ms(&self) -> u64 {
        self.last_activity.saturating_sub(self.created_at)
    }

    /// Throughput (bytes/sec)
    pub fn throughput(&self) -> u64 {
        let dur_ms = self.duration_ms();
        if dur_ms == 0 {
            return 0;
        }
        (self.bytes_sent + self.bytes_received) * 1000 / dur_ms
    }
}

// ============================================================================
// PROCESS NETWORK PROFILE
// ============================================================================

/// Per-process network profile
#[derive(Debug, Clone)]
pub struct ProcessNetworkProfile {
    /// Process ID
    pub pid: u64,
    /// Detected pattern
    pub pattern: AppNetworkPattern,
    /// Active connections
    pub active_connections: u32,
    /// Total connections ever
    pub total_connections: u64,
    /// Total bytes sent
    pub total_bytes_sent: u64,
    /// Total bytes received
    pub total_bytes_received: u64,
    /// Avg connection duration (ms)
    pub avg_conn_duration_ms: u64,
    /// Protocol distribution
    pub protocol_dist: BTreeMap<u8, u32>,
    /// Is latency sensitive
    pub latency_sensitive: bool,
    /// Recommended socket pool size
    pub recommended_pool_size: u32,
    /// QoS class
    pub qos_class: NetworkQosClass,
}

/// Network QoS classification for app
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkQosClass {
    /// Real-time (VoIP, gaming)
    Realtime,
    /// Interactive (web browsing, SSH)
    Interactive,
    /// Bulk transfer (downloads, backups)
    Bulk,
    /// Background (updates, telemetry)
    Background,
    /// Best effort
    BestEffort,
}

// ============================================================================
// SOCKET POOL ADVISOR
// ============================================================================

/// Socket pooling recommendation
#[derive(Debug, Clone)]
pub struct PoolRecommendation {
    /// Process ID
    pub pid: u64,
    /// Remote endpoint hash
    pub endpoint_hash: u64,
    /// Recommended pool size
    pub pool_size: u32,
    /// Keep-alive interval (seconds)
    pub keepalive_secs: u32,
    /// Max idle time (seconds)
    pub max_idle_secs: u32,
    /// Reason
    pub reason: PoolReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolReason {
    /// Frequent reconnections detected
    FrequentReconnect,
    /// High connection setup overhead
    HighSetupCost,
    /// Pattern suggests connection reuse
    PatternBased,
    /// Explicit app request
    AppRequested,
}

// ============================================================================
// NETWORK ANALYZER
// ============================================================================

/// Application network analyzer
pub struct AppNetworkAnalyzer {
    /// Active connections per process
    connections: BTreeMap<u64, Vec<TrackedConnection>>,
    /// Process profiles
    profiles: BTreeMap<u64, ProcessNetworkProfile>,
    /// Next connection ID
    next_conn_id: u64,
    /// Pool recommendations
    pool_recommendations: Vec<PoolRecommendation>,
    /// Total connections tracked
    pub total_connections: u64,
}

impl AppNetworkAnalyzer {
    pub fn new() -> Self {
        Self {
            connections: BTreeMap::new(),
            profiles: BTreeMap::new(),
            next_conn_id: 1,
            pool_recommendations: Vec::new(),
            total_connections: 0,
        }
    }

    /// Track new connection
    pub fn track_connection(
        &mut self,
        pid: u64,
        remote_hash: u64,
        remote_port: u16,
        local_port: u16,
        timestamp: u64,
    ) -> u64 {
        let id = self.next_conn_id;
        self.next_conn_id += 1;
        self.total_connections += 1;

        let conn = TrackedConnection::new(id, pid, remote_hash, remote_port, local_port, timestamp);

        self.connections
            .entry(pid)
            .or_insert_with(Vec::new)
            .push(conn);

        id
    }

    /// Update connection
    pub fn update_connection(
        &mut self,
        pid: u64,
        conn_id: u64,
        bytes_sent: u64,
        bytes_recv: u64,
        timestamp: u64,
    ) {
        if let Some(conns) = self.connections.get_mut(&pid) {
            if let Some(conn) = conns.iter_mut().find(|c| c.id == conn_id) {
                conn.bytes_sent += bytes_sent;
                conn.bytes_received += bytes_recv;
                conn.packets_sent += if bytes_sent > 0 { 1 } else { 0 };
                conn.packets_received += if bytes_recv > 0 { 1 } else { 0 };
                conn.last_activity = timestamp;
            }
        }
    }

    /// Close connection
    pub fn close_connection(&mut self, pid: u64, conn_id: u64) {
        if let Some(conns) = self.connections.get_mut(&pid) {
            if let Some(conn) = conns.iter_mut().find(|c| c.id == conn_id) {
                conn.state = ConnState::Closed;
            }
        }
    }

    /// Analyze process network behavior
    pub fn analyze_process(&mut self, pid: u64) -> Option<&ProcessNetworkProfile> {
        let conns = self.connections.get(&pid)?;

        let active = conns
            .iter()
            .filter(|c| c.state == ConnState::Established)
            .count() as u32;
        let total = conns.len() as u64;

        let total_sent: u64 = conns.iter().map(|c| c.bytes_sent).sum();
        let total_recv: u64 = conns.iter().map(|c| c.bytes_received).sum();

        let avg_duration = if !conns.is_empty() {
            conns.iter().map(|c| c.duration_ms()).sum::<u64>() / conns.len() as u64
        } else {
            0
        };

        // Detect pattern
        let pattern = if conns.is_empty() {
            AppNetworkPattern::None
        } else if avg_duration < 100 {
            AppNetworkPattern::ShortLived
        } else if avg_duration > 30_000 && active > 0 {
            AppNetworkPattern::Streaming
        } else if conns
            .iter()
            .any(|c| c.avg_rtt_us() > 0 && c.avg_rtt_us() < 1000)
        {
            AppNetworkPattern::RequestResponse
        } else {
            AppNetworkPattern::Persistent
        };

        // Protocol distribution
        let mut proto_dist = BTreeMap::new();
        for conn in conns {
            *proto_dist.entry(conn.protocol as u8).or_insert(0u32) += 1;
        }

        // Latency sensitivity
        let latency_sensitive = conns
            .iter()
            .any(|c| c.avg_rtt_us() > 0 && c.avg_rtt_us() < 5000);

        let qos = if latency_sensitive {
            NetworkQosClass::Realtime
        } else if pattern == AppNetworkPattern::Streaming {
            NetworkQosClass::Bulk
        } else {
            NetworkQosClass::BestEffort
        };

        let pool_size = match pattern {
            AppNetworkPattern::ShortLived => (active + 2).min(20),
            AppNetworkPattern::RequestResponse => (active + 1).min(10),
            AppNetworkPattern::Persistent => active.min(5),
            _ => 0,
        };

        self.profiles.insert(pid, ProcessNetworkProfile {
            pid,
            pattern,
            active_connections: active,
            total_connections: total,
            total_bytes_sent: total_sent,
            total_bytes_received: total_recv,
            avg_conn_duration_ms: avg_duration,
            protocol_dist: proto_dist,
            latency_sensitive,
            recommended_pool_size: pool_size,
            qos_class: qos,
        });

        self.profiles.get(&pid)
    }

    /// Get profile
    pub fn profile(&self, pid: u64) -> Option<&ProcessNetworkProfile> {
        self.profiles.get(&pid)
    }

    /// Cleanup closed connections
    pub fn cleanup(&mut self, max_age_ms: u64, current_time: u64) {
        for conns in self.connections.values_mut() {
            conns.retain(|c| {
                c.state != ConnState::Closed
                    || current_time.saturating_sub(c.last_activity) < max_age_ms
            });
        }
    }
}
