//! Network Flow
//!
//! Network flow identification and statistics.

use alloc::collections::VecDeque;
use alloc::vec::Vec;

use super::{ConnectionState, QosClass};
use crate::core::NexusTimestamp;
use crate::math::F64Ext;

/// Network flow identifier (5-tuple)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FlowId {
    /// Source IP (simplified to u32)
    pub src_ip: u32,
    /// Destination IP
    pub dst_ip: u32,
    /// Source port
    pub src_port: u16,
    /// Destination port
    pub dst_port: u16,
    /// Protocol
    pub protocol: u8,
}

impl FlowId {
    /// Create new flow ID
    pub fn new(src_ip: u32, dst_ip: u32, src_port: u16, dst_port: u16, protocol: u8) -> Self {
        Self {
            src_ip,
            dst_ip,
            src_port,
            dst_port,
            protocol,
        }
    }

    /// Get reverse flow
    #[inline]
    pub fn reverse(&self) -> Self {
        Self {
            src_ip: self.dst_ip,
            dst_ip: self.src_ip,
            src_port: self.dst_port,
            dst_port: self.src_port,
            protocol: self.protocol,
        }
    }

    /// Create bidirectional key (ordered)
    #[inline]
    pub fn bidirectional(&self) -> Self {
        let reverse = self.reverse();
        if (self.src_ip, self.src_port) < (reverse.src_ip, reverse.src_port) {
            *self
        } else {
            reverse
        }
    }
}

/// Network flow statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FlowStats {
    /// Flow ID
    pub flow_id: FlowId,
    /// Packets sent
    pub packets_sent: u64,
    /// Packets received
    pub packets_received: u64,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// First packet time
    pub first_seen: NexusTimestamp,
    /// Last packet time
    pub last_seen: NexusTimestamp,
    /// Connection state
    pub state: ConnectionState,
    /// QoS class
    pub qos_class: QosClass,
    /// Round-trip time samples (ns)
    pub rtt_samples: VecDeque<u64>,
    /// Retransmission count
    pub retransmissions: u32,
}

impl FlowStats {
    /// Create new flow stats
    pub fn new(flow_id: FlowId) -> Self {
        let now = NexusTimestamp::now();
        Self {
            flow_id,
            packets_sent: 0,
            packets_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            first_seen: now,
            last_seen: now,
            state: ConnectionState::New,
            qos_class: QosClass::BestEffort,
            rtt_samples: VecDeque::new(),
            retransmissions: 0,
        }
    }

    /// Record outbound packet
    #[inline]
    pub fn record_send(&mut self, bytes: u64) {
        self.packets_sent += 1;
        self.bytes_sent += bytes;
        self.last_seen = NexusTimestamp::now();
    }

    /// Record inbound packet
    #[inline]
    pub fn record_recv(&mut self, bytes: u64) {
        self.packets_received += 1;
        self.bytes_received += bytes;
        self.last_seen = NexusTimestamp::now();
    }

    /// Record RTT sample
    #[inline]
    pub fn record_rtt(&mut self, rtt_ns: u64) {
        self.rtt_samples.push_back(rtt_ns);
        if self.rtt_samples.len() > 100 {
            self.rtt_samples.pop_front();
        }
    }

    /// Get average RTT
    #[inline]
    pub fn avg_rtt(&self) -> Option<f64> {
        if self.rtt_samples.is_empty() {
            None
        } else {
            Some(self.rtt_samples.iter().sum::<u64>() as f64 / self.rtt_samples.len() as f64)
        }
    }

    /// Get minimum RTT
    #[inline(always)]
    pub fn min_rtt(&self) -> Option<u64> {
        self.rtt_samples.iter().copied().min()
    }

    /// Get jitter (RTT variance)
    pub fn jitter(&self) -> Option<f64> {
        if self.rtt_samples.len() < 2 {
            return None;
        }

        let avg = self.avg_rtt()?;
        let variance = self
            .rtt_samples
            .iter()
            .map(|&r| (r as f64 - avg).powi(2))
            .sum::<f64>()
            / self.rtt_samples.len() as f64;

        Some(variance.sqrt())
    }

    /// Calculate throughput in bytes per second
    #[inline]
    pub fn throughput(&self) -> f64 {
        let duration = self.last_seen.duration_since(self.first_seen);
        if duration == 0 {
            return 0.0;
        }

        // Assume ticks are nanoseconds
        let total_bytes = self.bytes_sent + self.bytes_received;
        total_bytes as f64 * 1_000_000_000.0 / duration as f64
    }

    /// Is flow idle?
    #[inline(always)]
    pub fn is_idle(&self, timeout_ticks: u64) -> bool {
        NexusTimestamp::now().duration_since(self.last_seen) > timeout_ticks
    }

    /// Get loss rate
    #[inline]
    pub fn loss_rate(&self) -> f64 {
        if self.packets_sent == 0 {
            return 0.0;
        }
        self.retransmissions as f64 / self.packets_sent as f64
    }
}
