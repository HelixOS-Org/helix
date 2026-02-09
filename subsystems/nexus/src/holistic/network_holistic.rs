//! # Holistic Network Analysis
//!
//! System-wide network traffic analysis and optimization:
//! - Cross-process traffic patterns
//! - Protocol distribution
//! - Bandwidth allocation
//! - Network topology awareness
//! - Connection pooling optimization
//! - Congestion detection

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

// ============================================================================
// NETWORK TOPOLOGY
// ============================================================================

/// Network interface type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HolisticInterfaceType {
    /// Loopback
    Loopback,
    /// Ethernet
    Ethernet,
    /// WiFi
    WiFi,
    /// Virtual bridge
    VirtualBridge,
    /// VPN tunnel
    VpnTunnel,
    /// Container veth
    ContainerVeth,
    /// RDMA
    Rdma,
}

/// Network interface profile
#[derive(Debug, Clone)]
pub struct InterfaceProfile {
    /// Interface index
    pub index: u32,
    /// Interface type
    pub iface_type: HolisticInterfaceType,
    /// Bandwidth capacity (bytes/s)
    pub bandwidth_bps: u64,
    /// Current utilization (0.0-1.0)
    pub utilization: f64,
    /// Bytes sent
    pub tx_bytes: u64,
    /// Bytes received
    pub rx_bytes: u64,
    /// Packets sent
    pub tx_packets: u64,
    /// Packets received
    pub rx_packets: u64,
    /// Errors
    pub errors: u64,
    /// Drops
    pub drops: u64,
}

impl InterfaceProfile {
    pub fn new(index: u32, iface_type: HolisticInterfaceType, bandwidth_bps: u64) -> Self {
        Self {
            index,
            iface_type,
            bandwidth_bps,
            utilization: 0.0,
            tx_bytes: 0,
            rx_bytes: 0,
            tx_packets: 0,
            rx_packets: 0,
            errors: 0,
            drops: 0,
        }
    }

    /// Record TX
    #[inline(always)]
    pub fn record_tx(&mut self, bytes: u64, packets: u64) {
        self.tx_bytes += bytes;
        self.tx_packets += packets;
    }

    /// Record RX
    #[inline(always)]
    pub fn record_rx(&mut self, bytes: u64, packets: u64) {
        self.rx_bytes += bytes;
        self.rx_packets += packets;
    }

    /// Update utilization for time period
    #[inline]
    pub fn update_utilization(&mut self, bytes_in_period: u64, period_secs: f64) {
        if self.bandwidth_bps > 0 && period_secs > 0.0 {
            let throughput = bytes_in_period as f64 / period_secs;
            self.utilization = throughput / self.bandwidth_bps as f64;
            if self.utilization > 1.0 {
                self.utilization = 1.0;
            }
        }
    }

    /// Total bytes
    #[inline(always)]
    pub fn total_bytes(&self) -> u64 {
        self.tx_bytes + self.rx_bytes
    }

    /// Error rate
    #[inline]
    pub fn error_rate(&self) -> f64 {
        let total = self.tx_packets + self.rx_packets;
        if total == 0 {
            return 0.0;
        }
        self.errors as f64 / total as f64
    }

    /// Drop rate
    #[inline]
    pub fn drop_rate(&self) -> f64 {
        let total = self.tx_packets + self.rx_packets;
        if total == 0 {
            return 0.0;
        }
        self.drops as f64 / total as f64
    }
}

// ============================================================================
// TRAFFIC FLOW
// ============================================================================

/// Protocol type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HolisticProtocol {
    /// TCP
    Tcp,
    /// UDP
    Udp,
    /// ICMP
    Icmp,
    /// IPC (local)
    Ipc,
    /// RDMA
    Rdma,
    /// Custom
    Custom(u16),
}

/// Traffic flow direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowDirection {
    /// Ingress
    Ingress,
    /// Egress
    Egress,
    /// Local (within system)
    Local,
}

/// A network flow
#[derive(Debug, Clone)]
pub struct NetworkFlow {
    /// Source process
    pub src_pid: u64,
    /// Destination process (0 = external)
    pub dst_pid: u64,
    /// Protocol
    pub protocol: HolisticProtocol,
    /// Direction
    pub direction: FlowDirection,
    /// Bytes transferred
    pub bytes: u64,
    /// Packets
    pub packets: u64,
    /// Start time
    pub start_time: u64,
    /// Last activity time
    pub last_activity: u64,
    /// Retransmits
    pub retransmits: u64,
    /// Average RTT (us)
    pub avg_rtt_us: u64,
}

impl NetworkFlow {
    pub fn new(
        src_pid: u64,
        dst_pid: u64,
        protocol: HolisticProtocol,
        direction: FlowDirection,
        now: u64,
    ) -> Self {
        Self {
            src_pid,
            dst_pid,
            protocol,
            direction,
            bytes: 0,
            packets: 0,
            start_time: now,
            last_activity: now,
            retransmits: 0,
            avg_rtt_us: 0,
        }
    }

    /// Record activity
    #[inline]
    pub fn record(&mut self, bytes: u64, packets: u64, now: u64) {
        self.bytes += bytes;
        self.packets += packets;
        self.last_activity = now;
    }

    /// Average packet size
    #[inline]
    pub fn avg_packet_size(&self) -> u64 {
        if self.packets == 0 {
            return 0;
        }
        self.bytes / self.packets
    }

    /// Retransmit rate
    #[inline]
    pub fn retransmit_rate(&self) -> f64 {
        if self.packets == 0 {
            return 0.0;
        }
        self.retransmits as f64 / self.packets as f64
    }

    /// Is idle
    #[inline(always)]
    pub fn is_idle(&self, now: u64, timeout_ns: u64) -> bool {
        now.saturating_sub(self.last_activity) > timeout_ns
    }
}

// ============================================================================
// CONGESTION DETECTION
// ============================================================================

/// Congestion state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CongestionState {
    /// No congestion
    Clear,
    /// Light congestion
    Light,
    /// Moderate congestion
    Moderate,
    /// Severe congestion
    Severe,
    /// Collapse
    Collapse,
}

/// Congestion detector
#[derive(Debug, Clone)]
pub struct CongestionDetector {
    /// Queue depth samples
    queue_samples: VecDeque<u64>,
    /// Drop rate samples
    drop_samples: VecDeque<f64>,
    /// RTT samples (us)
    rtt_samples: VecDeque<u64>,
    /// Baseline RTT (us)
    baseline_rtt_us: u64,
    /// Max samples
    max_samples: usize,
    /// Current state
    pub state: CongestionState,
}

impl CongestionDetector {
    pub fn new(baseline_rtt_us: u64) -> Self {
        Self {
            queue_samples: VecDeque::new(),
            drop_samples: VecDeque::new(),
            rtt_samples: VecDeque::new(),
            baseline_rtt_us,
            max_samples: 100,
            state: CongestionState::Clear,
        }
    }

    /// Record sample
    pub fn record(&mut self, queue_depth: u64, drop_rate: f64, rtt_us: u64) {
        self.queue_samples.push_back(queue_depth);
        if self.queue_samples.len() > self.max_samples {
            self.queue_samples.pop_front();
        }
        self.drop_samples.push_back(drop_rate);
        if self.drop_samples.len() > self.max_samples {
            self.drop_samples.pop_front();
        }
        self.rtt_samples.push_back(rtt_us);
        if self.rtt_samples.len() > self.max_samples {
            self.rtt_samples.pop_front();
        }
        self.evaluate();
    }

    /// Evaluate congestion
    fn evaluate(&mut self) {
        let avg_rtt = if self.rtt_samples.is_empty() {
            0
        } else {
            self.rtt_samples.iter().sum::<u64>() / self.rtt_samples.len() as u64
        };

        let avg_drop = if self.drop_samples.is_empty() {
            0.0
        } else {
            self.drop_samples.iter().sum::<f64>() / self.drop_samples.len() as f64
        };

        let rtt_ratio = if self.baseline_rtt_us > 0 {
            avg_rtt as f64 / self.baseline_rtt_us as f64
        } else {
            1.0
        };

        self.state = if avg_drop > 0.1 || rtt_ratio > 10.0 {
            CongestionState::Collapse
        } else if avg_drop > 0.05 || rtt_ratio > 5.0 {
            CongestionState::Severe
        } else if avg_drop > 0.01 || rtt_ratio > 2.0 {
            CongestionState::Moderate
        } else if avg_drop > 0.001 || rtt_ratio > 1.5 {
            CongestionState::Light
        } else {
            CongestionState::Clear
        };
    }
}

// ============================================================================
// BANDWIDTH ALLOCATOR
// ============================================================================

/// Bandwidth allocation class
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BandwidthClass {
    /// Best effort
    BestEffort,
    /// Assured forwarding
    Assured,
    /// Expedited forwarding (low latency)
    Expedited,
    /// Network control
    Control,
}

/// Bandwidth allocation
#[derive(Debug, Clone)]
pub struct BandwidthAllocation {
    /// Process
    pub pid: u64,
    /// Class
    pub class: BandwidthClass,
    /// Guaranteed bandwidth (bytes/s)
    pub guaranteed_bps: u64,
    /// Maximum bandwidth (bytes/s)
    pub max_bps: u64,
    /// Current usage (bytes/s)
    pub current_bps: u64,
    /// Borrowed from others
    pub borrowed_bps: u64,
}

impl BandwidthAllocation {
    pub fn new(pid: u64, class: BandwidthClass, guaranteed: u64, max: u64) -> Self {
        Self {
            pid,
            class,
            guaranteed_bps: guaranteed,
            max_bps: max,
            current_bps: 0,
            borrowed_bps: 0,
        }
    }

    /// Spare bandwidth
    #[inline(always)]
    pub fn spare(&self) -> u64 {
        self.guaranteed_bps.saturating_sub(self.current_bps)
    }

    /// Is over guarantee
    #[inline(always)]
    pub fn is_over_guarantee(&self) -> bool {
        self.current_bps > self.guaranteed_bps
    }

    /// Utilization
    #[inline]
    pub fn utilization(&self) -> f64 {
        if self.max_bps == 0 {
            return 0.0;
        }
        self.current_bps as f64 / self.max_bps as f64
    }
}

// ============================================================================
// NETWORK ANALYZER
// ============================================================================

/// Network analysis stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticNetworkStats {
    /// Total flows
    pub total_flows: u64,
    /// Active flows
    pub active_flows: usize,
    /// Total bytes
    pub total_bytes: u64,
    /// Local traffic ratio
    pub local_ratio: f64,
    /// Congestion events
    pub congestion_events: u64,
}

/// Holistic network analyzer
pub struct HolisticNetworkAnalyzer {
    /// Interfaces
    interfaces: BTreeMap<u32, InterfaceProfile>,
    /// Active flows (flow_id → flow)
    flows: BTreeMap<u64, NetworkFlow>,
    /// Bandwidth allocations
    allocations: BTreeMap<u64, BandwidthAllocation>,
    /// Congestion detector per interface
    congestion: BTreeMap<u32, CongestionDetector>,
    /// Protocol distribution (protocol → byte count)
    protocol_dist: BTreeMap<u8, u64>,
    /// Next flow ID
    next_flow_id: u64,
    /// Stats
    stats: HolisticNetworkStats,
}

impl HolisticNetworkAnalyzer {
    pub fn new() -> Self {
        Self {
            interfaces: BTreeMap::new(),
            flows: BTreeMap::new(),
            allocations: BTreeMap::new(),
            congestion: BTreeMap::new(),
            protocol_dist: BTreeMap::new(),
            next_flow_id: 1,
            stats: HolisticNetworkStats::default(),
        }
    }

    /// Register interface
    #[inline]
    pub fn register_interface(&mut self, profile: InterfaceProfile) {
        let idx = profile.index;
        let baseline = if profile.bandwidth_bps > 0 {
            100 // 100us baseline for wired
        } else {
            1000
        };
        self.congestion
            .insert(idx, CongestionDetector::new(baseline));
        self.interfaces.insert(idx, profile);
    }

    /// Create flow
    #[inline]
    pub fn create_flow(
        &mut self,
        src_pid: u64,
        dst_pid: u64,
        protocol: HolisticProtocol,
        direction: FlowDirection,
        now: u64,
    ) -> u64 {
        let id = self.next_flow_id;
        self.next_flow_id += 1;
        self.flows
            .insert(id, NetworkFlow::new(src_pid, dst_pid, protocol, direction, now));
        self.stats.total_flows += 1;
        self.stats.active_flows = self.flows.len();
        id
    }

    /// Record flow activity
    pub fn record_flow(&mut self, flow_id: u64, bytes: u64, packets: u64, now: u64) {
        if let Some(flow) = self.flows.get_mut(&flow_id) {
            let proto_key = match flow.protocol {
                HolisticProtocol::Tcp => 0u8,
                HolisticProtocol::Udp => 1,
                HolisticProtocol::Icmp => 2,
                HolisticProtocol::Ipc => 3,
                HolisticProtocol::Rdma => 4,
                HolisticProtocol::Custom(p) => (5 + p % 250) as u8,
            };
            *self.protocol_dist.entry(proto_key).or_insert(0) += bytes;
            flow.record(bytes, packets, now);
            self.stats.total_bytes += bytes;
        }
    }

    /// Set bandwidth allocation
    #[inline(always)]
    pub fn set_allocation(&mut self, alloc: BandwidthAllocation) {
        self.allocations.insert(alloc.pid, alloc);
    }

    /// Cleanup idle flows
    #[inline(always)]
    pub fn cleanup_idle(&mut self, now: u64, timeout_ns: u64) {
        self.flows.retain(|_, f| !f.is_idle(now, timeout_ns));
        self.stats.active_flows = self.flows.len();
    }

    /// Compute local traffic ratio
    pub fn compute_local_ratio(&mut self) {
        let total: u64 = self.flows.values().map(|f| f.bytes).sum();
        let local: u64 = self
            .flows
            .values()
            .filter(|f| f.direction == FlowDirection::Local)
            .map(|f| f.bytes)
            .sum();
        self.stats.local_ratio = if total > 0 {
            local as f64 / total as f64
        } else {
            0.0
        };
    }

    /// Get interface
    #[inline(always)]
    pub fn interface(&self, index: u32) -> Option<&InterfaceProfile> {
        self.interfaces.get(&index)
    }

    /// Get flow
    #[inline(always)]
    pub fn flow(&self, id: u64) -> Option<&NetworkFlow> {
        self.flows.get(&id)
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticNetworkStats {
        &self.stats
    }
}

// ============================================================================
// Merged from network_v2_holistic
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetLayerV2 { Link, Network, Transport, Application }

/// Network health
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetHealthV2 { Healthy, Warning, Critical, Down }

/// Network v2 holistic record
#[derive(Debug, Clone)]
pub struct NetworkV2HolisticRecord {
    pub layer: NetLayerV2,
    pub health: NetHealthV2,
    pub packets_sec: u64,
    pub errors_sec: u64,
}

impl NetworkV2HolisticRecord {
    pub fn new(layer: NetLayerV2, health: NetHealthV2) -> Self { Self { layer, health, packets_sec: 0, errors_sec: 0 } }
}

/// Network v2 holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct NetworkV2HolisticStats { pub total_samples: u64, pub warnings: u64, pub criticals: u64, pub peak_pps: u64 }

/// Main holistic network v2
#[derive(Debug)]
pub struct HolisticNetworkV2 { pub stats: NetworkV2HolisticStats }

impl HolisticNetworkV2 {
    pub fn new() -> Self { Self { stats: NetworkV2HolisticStats { total_samples: 0, warnings: 0, criticals: 0, peak_pps: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &NetworkV2HolisticRecord) {
        self.stats.total_samples += 1;
        match rec.health {
            NetHealthV2::Warning => self.stats.warnings += 1,
            NetHealthV2::Critical | NetHealthV2::Down => self.stats.criticals += 1,
            _ => {}
        }
        if rec.packets_sec > self.stats.peak_pps { self.stats.peak_pps = rec.packets_sec; }
    }
}
