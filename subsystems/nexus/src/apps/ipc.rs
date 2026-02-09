//! # Application IPC Tracking
//!
//! Track inter-process communication patterns:
//! - Pipe/socket/shared memory tracking
//! - Message flow analysis
//! - Bottleneck detection
//! - Bandwidth measurement
//! - Latency profiling

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// IPC TYPES
// ============================================================================

/// IPC mechanism type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AppIpcMechanism {
    /// Unix pipe
    Pipe,
    /// Unix domain socket
    UnixSocket,
    /// TCP socket
    TcpSocket,
    /// UDP socket
    UdpSocket,
    /// Shared memory
    SharedMemory,
    /// Message queue
    MessageQueue,
    /// Semaphore
    Semaphore,
    /// Signal
    Signal,
    /// Eventfd
    EventFd,
}

impl AppIpcMechanism {
    /// Typical latency class (ns)
    pub fn typical_latency_ns(&self) -> u64 {
        match self {
            Self::Signal => 500,
            Self::EventFd => 200,
            Self::SharedMemory => 100,
            Self::Semaphore => 300,
            Self::Pipe => 1_000,
            Self::UnixSocket => 2_000,
            Self::MessageQueue => 5_000,
            Self::TcpSocket => 10_000,
            Self::UdpSocket => 8_000,
        }
    }

    /// Max bandwidth class (bytes/sec)
    pub fn typical_bandwidth(&self) -> u64 {
        match self {
            Self::SharedMemory => 10_000_000_000, // 10 GB/s
            Self::Pipe => 1_000_000_000,          // 1 GB/s
            Self::UnixSocket => 800_000_000,
            Self::TcpSocket => 100_000_000,
            Self::UdpSocket => 100_000_000,
            Self::MessageQueue => 500_000_000,
            Self::EventFd => 50_000_000,
            Self::Signal => 1_000_000,
            Self::Semaphore => 10_000_000,
        }
    }
}

/// IPC direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcDirection {
    /// Send
    Send,
    /// Receive
    Recv,
    /// Bidirectional
    Bidir,
}

// ============================================================================
// IPC CHANNEL
// ============================================================================

/// IPC channel identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct IpcChannelId(pub u64);

/// IPC channel
#[derive(Debug, Clone)]
pub struct AppIpcChannel {
    /// Channel id
    pub id: IpcChannelId,
    /// Mechanism
    pub mechanism: AppIpcMechanism,
    /// Source pid
    pub src_pid: u64,
    /// Destination pid
    pub dst_pid: u64,
    /// Messages sent
    pub messages_sent: u64,
    /// Bytes transferred
    pub bytes_transferred: u64,
    /// Total latency ns (for computing average)
    pub total_latency_ns: u64,
    /// Message count for latency
    pub latency_samples: u64,
    /// Created timestamp
    pub created_at: u64,
    /// Last active
    pub last_active: u64,
    /// Is alive
    pub alive: bool,
    /// Errors
    pub error_count: u64,
    /// Would-block events
    pub would_block_count: u64,
}

impl AppIpcChannel {
    pub fn new(
        id: IpcChannelId,
        mechanism: AppIpcMechanism,
        src: u64,
        dst: u64,
        now: u64,
    ) -> Self {
        Self {
            id,
            mechanism,
            src_pid: src,
            dst_pid: dst,
            messages_sent: 0,
            bytes_transferred: 0,
            total_latency_ns: 0,
            latency_samples: 0,
            created_at: now,
            last_active: now,
            alive: true,
            error_count: 0,
            would_block_count: 0,
        }
    }

    /// Record send
    #[inline]
    pub fn record_send(&mut self, bytes: u64, latency_ns: u64, now: u64) {
        self.messages_sent += 1;
        self.bytes_transferred += bytes;
        self.total_latency_ns += latency_ns;
        self.latency_samples += 1;
        self.last_active = now;
    }

    /// Record error
    #[inline(always)]
    pub fn record_error(&mut self) {
        self.error_count += 1;
    }

    /// Record would-block
    #[inline(always)]
    pub fn record_would_block(&mut self) {
        self.would_block_count += 1;
    }

    /// Average latency
    #[inline]
    pub fn avg_latency_ns(&self) -> u64 {
        if self.latency_samples == 0 {
            return 0;
        }
        self.total_latency_ns / self.latency_samples
    }

    /// Bandwidth (bytes/sec)
    pub fn bandwidth_bps(&self, now: u64) -> u64 {
        let elapsed = now.saturating_sub(self.created_at);
        if elapsed == 0 {
            return 0;
        }
        // Convert ns to seconds
        let secs = elapsed / 1_000_000_000;
        if secs == 0 {
            return self.bytes_transferred;
        }
        self.bytes_transferred / secs
    }

    /// Error rate
    #[inline]
    pub fn error_rate(&self) -> f64 {
        if self.messages_sent == 0 {
            return 0.0;
        }
        self.error_count as f64 / self.messages_sent as f64
    }

    /// Is bottleneck? (high would_block ratio)
    #[inline]
    pub fn is_bottleneck(&self) -> bool {
        if self.messages_sent < 10 {
            return false;
        }
        let block_ratio = self.would_block_count as f64 / self.messages_sent as f64;
        block_ratio > 0.3
    }

    /// Close channel
    #[inline(always)]
    pub fn close(&mut self) {
        self.alive = false;
    }
}

// ============================================================================
// IPC GRAPH
// ============================================================================

/// IPC edge for graph analysis
#[derive(Debug, Clone)]
pub struct IpcEdge {
    /// Source
    pub src: u64,
    /// Destination
    pub dst: u64,
    /// Total bytes
    pub bytes: u64,
    /// Message count
    pub messages: u64,
    /// Mechanisms used
    pub mechanisms: Vec<AppIpcMechanism>,
}

/// IPC communication graph
#[derive(Debug)]
pub struct IpcGraph {
    /// Edges keyed by (src, dst) hash
    edges: BTreeMap<u64, IpcEdge>,
    /// Processes involved
    processes: Vec<u64>,
}

impl IpcGraph {
    pub fn new() -> Self {
        Self {
            edges: BTreeMap::new(),
            processes: Vec::new(),
        }
    }

    fn edge_key(src: u64, dst: u64) -> u64 {
        // FNV-1a hash combination
        let mut hash: u64 = 0xcbf29ce484222325;
        hash ^= src;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= dst;
        hash = hash.wrapping_mul(0x100000001b3);
        hash
    }

    /// Record communication
    pub fn record(&mut self, src: u64, dst: u64, bytes: u64, mechanism: AppIpcMechanism) {
        let key = Self::edge_key(src, dst);
        let edge = self.edges.entry(key).or_insert_with(|| IpcEdge {
            src,
            dst,
            bytes: 0,
            messages: 0,
            mechanisms: Vec::new(),
        });
        edge.bytes += bytes;
        edge.messages += 1;
        if !edge.mechanisms.contains(&mechanism) {
            edge.mechanisms.push(mechanism);
        }

        // Track processes
        if !self.processes.contains(&src) {
            self.processes.push(src);
        }
        if !self.processes.contains(&dst) {
            self.processes.push(dst);
        }
    }

    /// Top talkers by bytes
    #[inline]
    pub fn top_talkers(&self, limit: usize) -> Vec<(u64, u64, u64)> {
        let mut edges: Vec<_> = self.edges.values().map(|e| (e.src, e.dst, e.bytes)).collect();
        edges.sort_by(|a, b| b.2.cmp(&a.2));
        edges.truncate(limit);
        edges
    }

    /// Processes with most connections
    #[inline]
    pub fn most_connected(&self) -> Vec<(u64, usize)> {
        let mut counts: LinearMap<usize, 64> = BTreeMap::new();
        for edge in self.edges.values() {
            *counts.entry(edge.src).or_insert(0) += 1;
            *counts.entry(edge.dst).or_insert(0) += 1;
        }
        let mut result: Vec<_> = counts.into_iter().collect();
        result.sort_by(|a, b| b.1.cmp(&a.1));
        result
    }
}

// ============================================================================
// IPC ANALYZER
// ============================================================================

/// IPC stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppIpcStats {
    /// Active channels
    pub active_channels: usize,
    /// Total bytes
    pub total_bytes: u64,
    /// Total messages
    pub total_messages: u64,
    /// Bottleneck channels
    pub bottleneck_count: usize,
}

/// Application IPC analyzer
pub struct AppIpcAnalyzer {
    /// Channels
    channels: BTreeMap<u64, AppIpcChannel>,
    /// Communication graph
    graph: IpcGraph,
    /// Next channel id
    next_id: u64,
    /// Stats
    stats: AppIpcStats,
}

impl AppIpcAnalyzer {
    pub fn new() -> Self {
        Self {
            channels: BTreeMap::new(),
            graph: IpcGraph::new(),
            next_id: 1,
            stats: AppIpcStats::default(),
        }
    }

    /// Create channel
    #[inline]
    pub fn create_channel(
        &mut self,
        mechanism: AppIpcMechanism,
        src: u64,
        dst: u64,
        now: u64,
    ) -> IpcChannelId {
        let id = IpcChannelId(self.next_id);
        self.next_id += 1;
        let channel = AppIpcChannel::new(id, mechanism, src, dst, now);
        self.channels.insert(id.0, channel);
        self.stats.active_channels = self.channels.values().filter(|c| c.alive).count();
        id
    }

    /// Record send on channel
    #[inline]
    pub fn record_send(
        &mut self,
        channel_id: IpcChannelId,
        bytes: u64,
        latency_ns: u64,
        now: u64,
    ) {
        if let Some(ch) = self.channels.get_mut(&channel_id.0) {
            ch.record_send(bytes, latency_ns, now);
            self.graph.record(ch.src_pid, ch.dst_pid, bytes, ch.mechanism);
            self.stats.total_bytes += bytes;
            self.stats.total_messages += 1;
        }
    }

    /// Record error
    #[inline]
    pub fn record_error(&mut self, channel_id: IpcChannelId) {
        if let Some(ch) = self.channels.get_mut(&channel_id.0) {
            ch.record_error();
        }
    }

    /// Close channel
    #[inline]
    pub fn close_channel(&mut self, channel_id: IpcChannelId) {
        if let Some(ch) = self.channels.get_mut(&channel_id.0) {
            ch.close();
            self.stats.active_channels = self.channels.values().filter(|c| c.alive).count();
        }
    }

    /// Get channel
    #[inline(always)]
    pub fn channel(&self, id: IpcChannelId) -> Option<&AppIpcChannel> {
        self.channels.get(&id.0)
    }

    /// Find bottlenecks
    #[inline]
    pub fn bottlenecks(&self) -> Vec<IpcChannelId> {
        self.channels
            .values()
            .filter(|c| c.alive && c.is_bottleneck())
            .map(|c| c.id)
            .collect()
    }

    /// Top talkers
    #[inline(always)]
    pub fn top_talkers(&self, limit: usize) -> Vec<(u64, u64, u64)> {
        self.graph.top_talkers(limit)
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &AppIpcStats {
        &self.stats
    }
}
