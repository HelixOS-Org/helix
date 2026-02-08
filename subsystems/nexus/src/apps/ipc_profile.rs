//! # Application IPC Profiler
//!
//! Per-process inter-process communication profiling:
//! - Pipe/socket/shared memory IPC tracking
//! - IPC throughput and latency measurement
//! - Communication graph construction
//! - Bottleneck detection in IPC chains
//! - Buffer utilization analysis

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// IPC mechanism type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcMechanismApps {
    Pipe,
    UnixSocket,
    TcpSocket,
    UdpSocket,
    SharedMemory,
    MessageQueue,
    Semaphore,
    Signal,
    Futex,
}

/// IPC direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcDirection {
    Send,
    Receive,
    Bidirectional,
}

/// Single IPC channel stats
#[derive(Debug, Clone)]
pub struct IpcChannelProfile {
    pub channel_id: u64,
    pub mechanism: IpcMechanismApps,
    pub local_pid: u64,
    pub remote_pid: u64,
    pub direction: IpcDirection,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub total_latency_ns: u64,
    pub max_latency_ns: u64,
    pub min_latency_ns: u64,
    pub buffer_full_count: u64,
    pub buffer_empty_count: u64,
    pub buffer_size: u64,
    pub created_at: u64,
    pub last_activity: u64,
}

impl IpcChannelProfile {
    pub fn new(channel_id: u64, mechanism: IpcMechanismApps, local: u64, remote: u64) -> Self {
        Self {
            channel_id,
            mechanism,
            local_pid: local,
            remote_pid: remote,
            direction: IpcDirection::Bidirectional,
            messages_sent: 0,
            messages_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            total_latency_ns: 0,
            max_latency_ns: 0,
            min_latency_ns: u64::MAX,
            buffer_full_count: 0,
            buffer_empty_count: 0,
            buffer_size: 0,
            created_at: 0,
            last_activity: 0,
        }
    }

    pub fn throughput_bps(&self, duration_ns: u64) -> f64 {
        if duration_ns == 0 { return 0.0; }
        let total = self.bytes_sent + self.bytes_received;
        total as f64 / (duration_ns as f64 / 1_000_000_000.0)
    }

    pub fn avg_latency_ns(&self) -> u64 {
        let total_msgs = self.messages_sent + self.messages_received;
        if total_msgs == 0 { return 0; }
        self.total_latency_ns / total_msgs
    }

    pub fn avg_message_size(&self) -> u64 {
        let total_msgs = self.messages_sent + self.messages_received;
        if total_msgs == 0 { return 0; }
        (self.bytes_sent + self.bytes_received) / total_msgs
    }

    pub fn buffer_pressure(&self) -> f64 {
        let total = self.buffer_full_count + self.buffer_empty_count;
        if total == 0 { return 0.0; }
        self.buffer_full_count as f64 / total as f64
    }

    pub fn record_send(&mut self, bytes: u64, latency_ns: u64, ts: u64) {
        self.messages_sent += 1;
        self.bytes_sent += bytes;
        self.total_latency_ns += latency_ns;
        if latency_ns > self.max_latency_ns { self.max_latency_ns = latency_ns; }
        if latency_ns < self.min_latency_ns { self.min_latency_ns = latency_ns; }
        self.last_activity = ts;
    }

    pub fn record_receive(&mut self, bytes: u64, latency_ns: u64, ts: u64) {
        self.messages_received += 1;
        self.bytes_received += bytes;
        self.total_latency_ns += latency_ns;
        if latency_ns > self.max_latency_ns { self.max_latency_ns = latency_ns; }
        if latency_ns < self.min_latency_ns { self.min_latency_ns = latency_ns; }
        self.last_activity = ts;
    }
}

/// IPC communication graph edge
#[derive(Debug, Clone)]
pub struct IpcGraphEdge {
    pub from_pid: u64,
    pub to_pid: u64,
    pub mechanism: IpcMechanismApps,
    pub total_bytes: u64,
    pub message_count: u64,
}

/// Per-process IPC profile
#[derive(Debug, Clone)]
pub struct ProcessIpcProfile {
    pub pid: u64,
    pub channels: BTreeMap<u64, IpcChannelProfile>,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub total_messages: u64,
    pub communication_partners: Vec<u64>,
}

impl ProcessIpcProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            channels: BTreeMap::new(),
            total_bytes_sent: 0,
            total_bytes_received: 0,
            total_messages: 0,
            communication_partners: Vec::new(),
        }
    }

    pub fn add_channel(&mut self, channel: IpcChannelProfile) {
        let remote = channel.remote_pid;
        self.channels.insert(channel.channel_id, channel);
        if !self.communication_partners.contains(&remote) {
            self.communication_partners.push(remote);
        }
    }

    pub fn record_send(&mut self, channel_id: u64, bytes: u64, latency_ns: u64, ts: u64) {
        if let Some(ch) = self.channels.get_mut(&channel_id) {
            ch.record_send(bytes, latency_ns, ts);
            self.total_bytes_sent += bytes;
            self.total_messages += 1;
        }
    }

    pub fn record_receive(&mut self, channel_id: u64, bytes: u64, latency_ns: u64, ts: u64) {
        if let Some(ch) = self.channels.get_mut(&channel_id) {
            ch.record_receive(bytes, latency_ns, ts);
            self.total_bytes_received += bytes;
            self.total_messages += 1;
        }
    }

    pub fn bottleneck_channels(&self) -> Vec<u64> {
        self.channels.values()
            .filter(|ch| ch.buffer_pressure() > 0.7)
            .map(|ch| ch.channel_id)
            .collect()
    }

    pub fn busiest_channel(&self) -> Option<u64> {
        self.channels.values()
            .max_by_key(|ch| ch.bytes_sent + ch.bytes_received)
            .map(|ch| ch.channel_id)
    }
}

/// App IPC profiler stats
#[derive(Debug, Clone, Default)]
pub struct AppIpcProfilerStats {
    pub total_processes: usize,
    pub total_channels: usize,
    pub total_bytes_transferred: u64,
    pub total_messages: u64,
    pub bottleneck_channels: usize,
    pub unique_edges: usize,
}

/// Application IPC Profiler
pub struct AppIpcProfiler {
    profiles: BTreeMap<u64, ProcessIpcProfile>,
    graph_edges: Vec<IpcGraphEdge>,
    stats: AppIpcProfilerStats,
}

impl AppIpcProfiler {
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            graph_edges: Vec::new(),
            stats: AppIpcProfilerStats::default(),
        }
    }

    pub fn register_process(&mut self, pid: u64) {
        self.profiles.entry(pid).or_insert_with(|| ProcessIpcProfile::new(pid));
    }

    pub fn add_channel(&mut self, pid: u64, channel: IpcChannelProfile) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.add_channel(channel);
        }
    }

    pub fn record_send(&mut self, pid: u64, channel_id: u64, bytes: u64, latency_ns: u64, ts: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.record_send(channel_id, bytes, latency_ns, ts);
        }
    }

    pub fn record_receive(&mut self, pid: u64, channel_id: u64, bytes: u64, latency_ns: u64, ts: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.record_receive(channel_id, bytes, latency_ns, ts);
        }
    }

    pub fn build_graph(&mut self) {
        self.graph_edges.clear();
        let mut edge_map: BTreeMap<(u64, u64), IpcGraphEdge> = BTreeMap::new();

        for profile in self.profiles.values() {
            for ch in profile.channels.values() {
                let key = if ch.local_pid < ch.remote_pid {
                    (ch.local_pid, ch.remote_pid)
                } else {
                    (ch.remote_pid, ch.local_pid)
                };

                let edge = edge_map.entry(key).or_insert_with(|| IpcGraphEdge {
                    from_pid: key.0,
                    to_pid: key.1,
                    mechanism: ch.mechanism,
                    total_bytes: 0,
                    message_count: 0,
                });
                edge.total_bytes += ch.bytes_sent + ch.bytes_received;
                edge.message_count += ch.messages_sent + ch.messages_received;
            }
        }

        self.graph_edges = edge_map.into_values().collect();
    }

    pub fn recompute(&mut self) {
        self.stats.total_processes = self.profiles.len();
        self.stats.total_channels = self.profiles.values().map(|p| p.channels.len()).sum();
        self.stats.total_bytes_transferred = self.profiles.values()
            .map(|p| p.total_bytes_sent + p.total_bytes_received).sum();
        self.stats.total_messages = self.profiles.values().map(|p| p.total_messages).sum();
        self.stats.bottleneck_channels = self.profiles.values()
            .map(|p| p.bottleneck_channels().len()).sum();
        self.stats.unique_edges = self.graph_edges.len();
    }

    pub fn profile(&self, pid: u64) -> Option<&ProcessIpcProfile> {
        self.profiles.get(&pid)
    }

    pub fn graph(&self) -> &[IpcGraphEdge] {
        &self.graph_edges
    }

    pub fn stats(&self) -> &AppIpcProfilerStats {
        &self.stats
    }

    pub fn remove_process(&mut self, pid: u64) {
        self.profiles.remove(&pid);
        self.recompute();
    }
}
