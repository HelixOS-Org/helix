//! QoS Engine
//!
//! Quality of Service management.

use alloc::collections::BTreeMap;

use super::{FlowId, QosClass};

/// Bandwidth allocation
#[derive(Debug, Clone, Copy)]
pub struct BandwidthAllocation {
    /// Minimum bandwidth (bytes/sec)
    pub min_bandwidth: u64,
    /// Maximum bandwidth (bytes/sec)
    pub max_bandwidth: u64,
    /// Current usage
    pub current_usage: u64,
    /// Priority weight
    pub weight: u32,
}

/// Quality of Service engine
pub struct QosEngine {
    /// Flow classifications
    classifications: BTreeMap<FlowId, QosClass>,
    /// Port-based rules
    port_rules: BTreeMap<u16, QosClass>,
    /// Default class
    default_class: QosClass,
    /// Bandwidth allocations per class
    allocations: BTreeMap<QosClass, BandwidthAllocation>,
}

impl QosEngine {
    /// Create new QoS engine
    pub fn new() -> Self {
        let mut allocations = BTreeMap::new();

        // Default allocations
        allocations.insert(QosClass::NetworkControl, BandwidthAllocation {
            min_bandwidth: 10_000_000,
            max_bandwidth: u64::MAX,
            current_usage: 0,
            weight: 100,
        });
        allocations.insert(QosClass::RealTime, BandwidthAllocation {
            min_bandwidth: 5_000_000,
            max_bandwidth: u64::MAX,
            current_usage: 0,
            weight: 80,
        });
        allocations.insert(QosClass::Voice, BandwidthAllocation {
            min_bandwidth: 1_000_000,
            max_bandwidth: 10_000_000,
            current_usage: 0,
            weight: 60,
        });
        allocations.insert(QosClass::Video, BandwidthAllocation {
            min_bandwidth: 1_000_000,
            max_bandwidth: 100_000_000,
            current_usage: 0,
            weight: 40,
        });
        allocations.insert(QosClass::Standard, BandwidthAllocation {
            min_bandwidth: 0,
            max_bandwidth: u64::MAX,
            current_usage: 0,
            weight: 20,
        });
        allocations.insert(QosClass::BestEffort, BandwidthAllocation {
            min_bandwidth: 0,
            max_bandwidth: u64::MAX,
            current_usage: 0,
            weight: 10,
        });
        allocations.insert(QosClass::Background, BandwidthAllocation {
            min_bandwidth: 0,
            max_bandwidth: u64::MAX,
            current_usage: 0,
            weight: 5,
        });

        let mut engine = Self {
            classifications: BTreeMap::new(),
            port_rules: BTreeMap::new(),
            default_class: QosClass::BestEffort,
            allocations,
        };

        // Default port rules
        engine.add_port_rule(22, QosClass::Standard); // SSH
        engine.add_port_rule(80, QosClass::Standard); // HTTP
        engine.add_port_rule(443, QosClass::Standard); // HTTPS
        engine.add_port_rule(53, QosClass::RealTime); // DNS
        engine.add_port_rule(5060, QosClass::Voice); // SIP
        engine.add_port_rule(5061, QosClass::Voice); // SIP-TLS
        engine.add_port_rule(554, QosClass::Video); // RTSP

        engine
    }

    /// Add port-based rule
    pub fn add_port_rule(&mut self, port: u16, class: QosClass) {
        self.port_rules.insert(port, class);
    }

    /// Classify flow
    pub fn classify(&mut self, flow_id: FlowId) -> QosClass {
        // Check cached classification
        if let Some(&class) = self.classifications.get(&flow_id) {
            return class;
        }

        // Check port rules
        let class = self
            .port_rules
            .get(&flow_id.dst_port)
            .or_else(|| self.port_rules.get(&flow_id.src_port))
            .copied()
            .unwrap_or(self.default_class);

        self.classifications.insert(flow_id, class);
        class
    }

    /// Set flow classification
    pub fn set_classification(&mut self, flow_id: FlowId, class: QosClass) {
        self.classifications.insert(flow_id, class);
    }

    /// Get DSCP for flow
    pub fn get_dscp(&mut self, flow_id: FlowId) -> u8 {
        self.classify(flow_id).to_dscp()
    }

    /// Check if flow can send
    pub fn can_send(&self, flow_id: &FlowId, bytes: u64) -> bool {
        let class = self
            .classifications
            .get(flow_id)
            .unwrap_or(&self.default_class);

        if let Some(alloc) = self.allocations.get(class) {
            alloc.current_usage + bytes <= alloc.max_bandwidth
        } else {
            true
        }
    }

    /// Record bandwidth usage
    pub fn record_usage(&mut self, flow_id: &FlowId, bytes: u64) {
        let class = *self
            .classifications
            .get(flow_id)
            .unwrap_or(&self.default_class);

        if let Some(alloc) = self.allocations.get_mut(&class) {
            alloc.current_usage = alloc.current_usage.saturating_add(bytes);
        }
    }

    /// Reset usage counters
    pub fn reset_usage(&mut self) {
        for alloc in self.allocations.values_mut() {
            alloc.current_usage = 0;
        }
    }

    /// Get allocation for class
    pub fn get_allocation(&self, class: QosClass) -> Option<&BandwidthAllocation> {
        self.allocations.get(&class)
    }

    /// Set allocation
    pub fn set_allocation(&mut self, class: QosClass, min: u64, max: u64, weight: u32) {
        self.allocations.insert(class, BandwidthAllocation {
            min_bandwidth: min,
            max_bandwidth: max,
            current_usage: 0,
            weight,
        });
    }
}

impl Default for QosEngine {
    fn default() -> Self {
        Self::new()
    }
}
