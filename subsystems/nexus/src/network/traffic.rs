//! Traffic Analyzer
//!
//! Analyzes network traffic patterns.

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{ConnectionState, Direction, FlowId, FlowStats};
use crate::core::NexusTimestamp;

/// Analyzes network traffic patterns
pub struct TrafficAnalyzer {
    /// Active flows
    flows: BTreeMap<FlowId, FlowStats>,
    /// Flow timeout in ticks
    flow_timeout: u64,
    /// Bandwidth samples
    bandwidth_history: VecDeque<(NexusTimestamp, u64)>,
    /// Maximum history size
    max_history: usize,
    /// Total bytes processed
    total_bytes: AtomicU64,
    /// Total packets processed
    total_packets: AtomicU64,
}

impl TrafficAnalyzer {
    /// Create new traffic analyzer
    pub fn new(flow_timeout: u64) -> Self {
        Self {
            flows: BTreeMap::new(),
            flow_timeout,
            bandwidth_history: VecDeque::new(),
            max_history: 3600, // 1 hour at 1 sample/second
            total_bytes: AtomicU64::new(0),
            total_packets: AtomicU64::new(0),
        }
    }

    /// Record packet
    pub fn record_packet(&mut self, flow_id: FlowId, direction: Direction, bytes: u64) {
        self.total_packets.fetch_add(1, Ordering::Relaxed);
        self.total_bytes.fetch_add(bytes, Ordering::Relaxed);

        let stats = self
            .flows
            .entry(flow_id)
            .or_insert_with(|| FlowStats::new(flow_id));

        match direction {
            Direction::Outbound => stats.record_send(bytes),
            Direction::Inbound => stats.record_recv(bytes),
            Direction::Both => {
                stats.record_send(bytes / 2);
                stats.record_recv(bytes / 2);
            },
        }
    }

    /// Record RTT for flow
    #[inline]
    pub fn record_rtt(&mut self, flow_id: FlowId, rtt_ns: u64) {
        if let Some(stats) = self.flows.get_mut(&flow_id) {
            stats.record_rtt(rtt_ns);
        }
    }

    /// Set connection state
    #[inline]
    pub fn set_state(&mut self, flow_id: FlowId, state: ConnectionState) {
        if let Some(stats) = self.flows.get_mut(&flow_id) {
            stats.state = state;
        }
    }

    /// Record retransmission
    #[inline]
    pub fn record_retransmission(&mut self, flow_id: FlowId) {
        if let Some(stats) = self.flows.get_mut(&flow_id) {
            stats.retransmissions += 1;
        }
    }

    /// Get flow stats
    #[inline(always)]
    pub fn get_flow(&self, flow_id: &FlowId) -> Option<&FlowStats> {
        self.flows.get(flow_id)
    }

    /// Get all active flows
    #[inline]
    pub fn active_flows(&self) -> impl Iterator<Item = &FlowStats> {
        self.flows
            .values()
            .filter(|f| !f.is_idle(self.flow_timeout))
    }

    /// Cleanup idle flows
    #[inline(always)]
    pub fn cleanup_idle(&mut self) {
        let timeout = self.flow_timeout;
        self.flows.retain(|_, stats| !stats.is_idle(timeout));
    }

    /// Record bandwidth sample
    #[inline]
    pub fn sample_bandwidth(&mut self) {
        let now = NexusTimestamp::now();
        let bytes = self.total_bytes.load(Ordering::Relaxed);

        self.bandwidth_history.push_back((now, bytes));

        if self.bandwidth_history.len() > self.max_history {
            self.bandwidth_history.pop_front();
        }
    }

    /// Get current bandwidth (bytes/sec)
    pub fn current_bandwidth(&self) -> f64 {
        if self.bandwidth_history.len() < 2 {
            return 0.0;
        }

        let len = self.bandwidth_history.len();
        let (t1, b1) = self.bandwidth_history[len - 2];
        let (t2, b2) = self.bandwidth_history[len - 1];

        let duration = t2.duration_since(t1);
        if duration == 0 {
            return 0.0;
        }

        let bytes = b2.saturating_sub(b1);
        bytes as f64 * 1_000_000_000.0 / duration as f64
    }

    /// Get total statistics
    #[inline(always)]
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes.load(Ordering::Relaxed)
    }

    /// Get total packets
    #[inline(always)]
    pub fn total_packets(&self) -> u64 {
        self.total_packets.load(Ordering::Relaxed)
    }

    /// Get flow count
    #[inline(always)]
    pub fn flow_count(&self) -> usize {
        self.flows.len()
    }

    /// Get top flows by bytes
    #[inline]
    pub fn top_flows_by_bytes(&self, n: usize) -> Vec<&FlowStats> {
        let mut flows: Vec<_> = self.flows.values().collect();
        flows.sort_by(|a, b| {
            let a_bytes = a.bytes_sent + a.bytes_received;
            let b_bytes = b.bytes_sent + b.bytes_received;
            b_bytes.cmp(&a_bytes)
        });
        flows.truncate(n);
        flows
    }
}

impl Default for TrafficAnalyzer {
    fn default() -> Self {
        Self::new(60_000_000_000) // 60 seconds
    }
}
