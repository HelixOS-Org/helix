//! Network Intelligence
//!
//! Central network intelligence coordinator.

use super::{
    BandwidthPredictor, ConnectionPredictor, Direction, FlowId, NetworkAnomaly,
    NetworkAnomalyDetector, QosEngine, TrafficAnalyzer,
};

/// Central network intelligence coordinator
pub struct NetworkIntelligence {
    /// Traffic analyzer
    traffic: TrafficAnalyzer,
    /// Bandwidth predictor
    bandwidth: BandwidthPredictor,
    /// Anomaly detector
    anomaly: NetworkAnomalyDetector,
    /// QoS engine
    qos: QosEngine,
    /// Connection predictor
    connection: ConnectionPredictor,
}

impl NetworkIntelligence {
    /// Create new network intelligence
    pub fn new() -> Self {
        Self {
            traffic: TrafficAnalyzer::default(),
            bandwidth: BandwidthPredictor::default(),
            anomaly: NetworkAnomalyDetector::default(),
            qos: QosEngine::default(),
            connection: ConnectionPredictor::default(),
        }
    }

    /// Process packet
    pub fn process_packet(
        &mut self,
        flow_id: FlowId,
        direction: Direction,
        bytes: u64,
    ) -> Option<NetworkAnomaly> {
        // Record traffic
        self.traffic.record_packet(flow_id, direction, bytes);

        // Record connection
        self.connection
            .record_connection(flow_id.src_ip, flow_id.dst_ip, flow_id.dst_port);

        // QoS classification
        self.qos.classify(flow_id);
        self.qos.record_usage(&flow_id, bytes);

        // Check for anomalies periodically
        if self.traffic.total_packets() % 1000 == 0 {
            let bw = self.traffic.current_bandwidth();
            return self.anomaly.check_traffic(bw);
        }

        None
    }

    /// Sample and update
    pub fn sample(&mut self) {
        self.traffic.sample_bandwidth();

        let bw = self.traffic.current_bandwidth();
        let connections = self.traffic.flow_count() as u32;
        self.bandwidth.record(bw, connections);

        // Update anomaly baseline
        let avg_rtt = self
            .traffic
            .active_flows()
            .filter_map(|f| f.avg_rtt())
            .sum::<f64>()
            / self.traffic.flow_count().max(1) as f64;

        self.anomaly
            .update_baseline(bw, connections as f64, avg_rtt);
    }

    /// Get traffic analyzer
    #[inline(always)]
    pub fn traffic(&self) -> &TrafficAnalyzer {
        &self.traffic
    }

    /// Get mutable traffic analyzer
    #[inline(always)]
    pub fn traffic_mut(&mut self) -> &mut TrafficAnalyzer {
        &mut self.traffic
    }

    /// Get bandwidth predictor
    #[inline(always)]
    pub fn bandwidth(&self) -> &BandwidthPredictor {
        &self.bandwidth
    }

    /// Get anomaly detector
    #[inline(always)]
    pub fn anomaly(&self) -> &NetworkAnomalyDetector {
        &self.anomaly
    }

    /// Get mutable anomaly detector
    #[inline(always)]
    pub fn anomaly_mut(&mut self) -> &mut NetworkAnomalyDetector {
        &mut self.anomaly
    }

    /// Get QoS engine
    #[inline(always)]
    pub fn qos(&self) -> &QosEngine {
        &self.qos
    }

    /// Get mutable QoS engine
    #[inline(always)]
    pub fn qos_mut(&mut self) -> &mut QosEngine {
        &mut self.qos
    }

    /// Get connection predictor
    #[inline(always)]
    pub fn connection(&self) -> &ConnectionPredictor {
        &self.connection
    }
}

impl Default for NetworkIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
