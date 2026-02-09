//! Network Anomaly Detector
//!
//! Detects network anomalies and attacks.

use alloc::format;
use alloc::string::String;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::AtomicU64;

use super::FlowId;
use crate::core::NexusTimestamp;

/// Network anomaly
#[derive(Debug, Clone)]
pub struct NetworkAnomaly {
    /// Anomaly type
    pub anomaly_type: NetworkAnomalyType,
    /// Severity (0.0 - 1.0)
    pub severity: f64,
    /// Detection timestamp
    pub detected_at: NexusTimestamp,
    /// Description
    pub description: String,
    /// Affected flow (if specific)
    pub flow_id: Option<FlowId>,
}

/// Types of network anomalies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkAnomalyType {
    /// Traffic spike
    TrafficSpike,
    /// Traffic drop
    TrafficDrop,
    /// Latency spike
    LatencySpike,
    /// High packet loss
    PacketLoss,
    /// Port scan detected
    PortScan,
    /// DDoS pattern
    DdosPattern,
    /// Connection flood
    ConnectionFlood,
    /// Unusual protocol
    UnusualProtocol,
    /// Data exfiltration
    DataExfiltration,
    /// Malformed packets
    MalformedPackets,
}

/// Detects network anomalies
pub struct NetworkAnomalyDetector {
    /// Baseline bandwidth
    baseline_bandwidth: f64,
    /// Baseline connections
    baseline_connections: f64,
    /// Baseline RTT
    baseline_rtt: f64,
    /// Anomaly threshold (sigma)
    threshold: f64,
    /// Detection history
    detections: VecDeque<NetworkAnomaly>,
    /// Maximum detections to keep
    max_detections: usize,
    /// Total packets analyzed
    packets_analyzed: AtomicU64,
}

impl NetworkAnomalyDetector {
    /// Create new detector
    pub fn new() -> Self {
        Self {
            baseline_bandwidth: 0.0,
            baseline_connections: 0.0,
            baseline_rtt: 0.0,
            threshold: 3.0,
            detections: VecDeque::new(),
            max_detections: 1000,
            packets_analyzed: AtomicU64::new(0),
        }
    }

    /// Update baseline
    #[inline]
    pub fn update_baseline(&mut self, bandwidth: f64, connections: f64, rtt: f64) {
        // Exponential moving average
        let alpha = 0.1;
        self.baseline_bandwidth = alpha * bandwidth + (1.0 - alpha) * self.baseline_bandwidth;
        self.baseline_connections = alpha * connections + (1.0 - alpha) * self.baseline_connections;
        self.baseline_rtt = alpha * rtt + (1.0 - alpha) * self.baseline_rtt;
    }

    /// Check for traffic anomalies
    pub fn check_traffic(&mut self, current_bandwidth: f64) -> Option<NetworkAnomaly> {
        if self.baseline_bandwidth == 0.0 {
            return None;
        }

        let ratio = current_bandwidth / self.baseline_bandwidth;

        // Traffic spike
        if ratio > self.threshold {
            let severity = ((ratio - 1.0) / 10.0).min(1.0);
            return Some(self.record_anomaly(
                NetworkAnomalyType::TrafficSpike,
                severity,
                format!("Traffic spike: {:.1}x baseline", ratio),
                None,
            ));
        }

        // Traffic drop
        if ratio < 1.0 / self.threshold && current_bandwidth > 0.0 {
            let severity = ((1.0 / ratio - 1.0) / 10.0).min(1.0);
            return Some(self.record_anomaly(
                NetworkAnomalyType::TrafficDrop,
                severity,
                format!("Traffic drop: {:.1}% of baseline", ratio * 100.0),
                None,
            ));
        }

        None
    }

    /// Check for latency anomalies
    pub fn check_latency(&mut self, current_rtt: f64) -> Option<NetworkAnomaly> {
        if self.baseline_rtt == 0.0 {
            return None;
        }

        let ratio = current_rtt / self.baseline_rtt;

        if ratio > self.threshold {
            let severity = ((ratio - 1.0) / 5.0).min(1.0);
            return Some(self.record_anomaly(
                NetworkAnomalyType::LatencySpike,
                severity,
                format!("Latency spike: {:.1}x baseline", ratio),
                None,
            ));
        }

        None
    }

    /// Check for connection flood
    pub fn check_connection_flood(&mut self, new_connections: u32) -> Option<NetworkAnomaly> {
        let threshold = (self.baseline_connections * self.threshold) as u32;

        if new_connections > threshold && threshold > 0 {
            let severity = (new_connections as f64 / threshold as f64 / 5.0).min(1.0);
            return Some(self.record_anomaly(
                NetworkAnomalyType::ConnectionFlood,
                severity,
                format!(
                    "{} new connections (threshold: {})",
                    new_connections, threshold
                ),
                None,
            ));
        }

        None
    }

    /// Check for port scan
    pub fn check_port_scan(
        &mut self,
        flow_id: FlowId,
        unique_ports: u32,
    ) -> Option<NetworkAnomaly> {
        // If single source hitting many ports
        if unique_ports > 100 {
            let severity = (unique_ports as f64 / 1000.0).min(1.0);
            return Some(self.record_anomaly(
                NetworkAnomalyType::PortScan,
                severity,
                format!("Port scan detected: {} unique ports", unique_ports),
                Some(flow_id),
            ));
        }

        None
    }

    /// Record anomaly
    fn record_anomaly(
        &mut self,
        anomaly_type: NetworkAnomalyType,
        severity: f64,
        description: String,
        flow_id: Option<FlowId>,
    ) -> NetworkAnomaly {
        let anomaly = NetworkAnomaly {
            anomaly_type,
            severity,
            detected_at: NexusTimestamp::now(),
            description,
            flow_id,
        };

        self.detections.push_back(anomaly.clone());
        if self.detections.len() > self.max_detections {
            self.detections.pop_front();
        }

        anomaly
    }

    /// Get recent detections
    #[inline(always)]
    pub fn recent_detections(&self, n: usize) -> &[NetworkAnomaly] {
        let start = self.detections.len().saturating_sub(n);
        &self.detections[start..]
    }

    /// Get detections by type
    #[inline]
    pub fn detections_by_type(&self, anomaly_type: NetworkAnomalyType) -> Vec<&NetworkAnomaly> {
        self.detections
            .iter()
            .filter(|a| a.anomaly_type == anomaly_type)
            .collect()
    }

    /// Set threshold
    #[inline(always)]
    pub fn set_threshold(&mut self, threshold: f64) {
        self.threshold = threshold.max(1.5);
    }

    /// Clear detections
    #[inline(always)]
    pub fn clear(&mut self) {
        self.detections.clear();
    }
}

impl Default for NetworkAnomalyDetector {
    fn default() -> Self {
        Self::new()
    }
}
