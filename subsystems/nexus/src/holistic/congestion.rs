//! # Holistic Congestion Manager
//!
//! System-wide congestion detection and management:
//! - Multi-resource congestion scoring
//! - Backpressure propagation
//! - Admission control
//! - Congestion avoidance algorithms
//! - Flow control coordination

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// CONGESTION TYPES
// ============================================================================

/// Congestion resource
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CongestionResource {
    /// CPU run queue
    CpuRunQueue,
    /// Memory allocation
    MemoryAlloc,
    /// Disk I/O queue
    DiskIo,
    /// Network transmit queue
    NetworkTx,
    /// Network receive queue
    NetworkRx,
    /// IPC channel
    IpcChannel,
    /// Lock contention
    LockContention,
}

/// Congestion level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CongestionLevel {
    /// No congestion
    None,
    /// Light congestion
    Light,
    /// Moderate congestion
    Moderate,
    /// Severe congestion
    Severe,
    /// Collapse (total saturation)
    Collapse,
}

impl CongestionLevel {
    /// From utilization ratio
    pub fn from_utilization(util: f64) -> Self {
        if util < 0.5 {
            CongestionLevel::None
        } else if util < 0.7 {
            CongestionLevel::Light
        } else if util < 0.85 {
            CongestionLevel::Moderate
        } else if util < 0.95 {
            CongestionLevel::Severe
        } else {
            CongestionLevel::Collapse
        }
    }

    /// Numeric severity
    #[inline]
    pub fn severity(&self) -> u32 {
        match self {
            CongestionLevel::None => 0,
            CongestionLevel::Light => 25,
            CongestionLevel::Moderate => 50,
            CongestionLevel::Severe => 75,
            CongestionLevel::Collapse => 100,
        }
    }
}

/// Backpressure signal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackpressureAction {
    /// No action needed
    NoAction,
    /// Slow down producers
    SlowDown,
    /// Pause producers
    Pause,
    /// Drop lowest priority
    DropLow,
    /// Emergency shed load
    ShedLoad,
}

// ============================================================================
// CONGESTION WINDOW (CWND) MANAGEMENT
// ============================================================================

/// Congestion window state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CwndState {
    /// Slow start
    SlowStart,
    /// Congestion avoidance
    CongestionAvoidance,
    /// Fast recovery
    FastRecovery,
}

/// Per-resource congestion window
#[derive(Debug, Clone)]
pub struct CongestionWindow {
    /// Window size (units depend on resource)
    pub cwnd: u64,
    /// Slow start threshold
    pub ssthresh: u64,
    /// State
    pub state: CwndState,
    /// Max observed window
    pub max_cwnd: u64,
    /// Min window
    pub min_cwnd: u64,
    /// Consecutive successes
    successes: u64,
    /// Consecutive losses
    losses: u64,
}

impl CongestionWindow {
    pub fn new(initial: u64, max: u64) -> Self {
        Self {
            cwnd: initial,
            ssthresh: max / 2,
            state: CwndState::SlowStart,
            max_cwnd: max,
            min_cwnd: 1,
            successes: 0,
            losses: 0,
        }
    }

    /// Record successful operation
    pub fn on_success(&mut self) {
        self.successes += 1;
        self.losses = 0;
        match self.state {
            CwndState::SlowStart => {
                self.cwnd = (self.cwnd * 2).min(self.max_cwnd);
                if self.cwnd >= self.ssthresh {
                    self.state = CwndState::CongestionAvoidance;
                }
            }
            CwndState::CongestionAvoidance => {
                // Additive increase
                self.cwnd = (self.cwnd + 1).min(self.max_cwnd);
            }
            CwndState::FastRecovery => {
                self.cwnd = self.ssthresh;
                self.state = CwndState::CongestionAvoidance;
            }
        }
    }

    /// Record congestion/loss event
    pub fn on_congestion(&mut self) {
        self.losses += 1;
        self.successes = 0;
        // Multiplicative decrease
        self.ssthresh = (self.cwnd / 2).max(self.min_cwnd);
        if self.losses >= 3 {
            self.state = CwndState::FastRecovery;
            self.cwnd = self.ssthresh + 3;
        } else {
            self.cwnd = self.ssthresh;
            self.state = CwndState::CongestionAvoidance;
        }
    }

    /// Available capacity
    #[inline(always)]
    pub fn available(&self) -> u64 {
        self.cwnd
    }

    /// Utilization
    #[inline]
    pub fn utilization(&self) -> f64 {
        if self.max_cwnd == 0 {
            return 0.0;
        }
        self.cwnd as f64 / self.max_cwnd as f64
    }
}

// ============================================================================
// RESOURCE CONGESTION TRACKER
// ============================================================================

/// Per-resource congestion data
#[derive(Debug)]
pub struct ResourceCongestion {
    /// Resource type
    pub resource: CongestionResource,
    /// Queue depth (current)
    pub queue_depth: u64,
    /// Queue capacity
    pub queue_capacity: u64,
    /// Service rate (ops/sec)
    pub service_rate: f64,
    /// Arrival rate (ops/sec)
    pub arrival_rate: f64,
    /// Congestion window
    pub cwnd: CongestionWindow,
    /// Level
    pub level: CongestionLevel,
    /// Backpressure active
    pub backpressure: bool,
    /// EMA latency (ns)
    pub avg_latency_ns: f64,
}

impl ResourceCongestion {
    pub fn new(resource: CongestionResource, capacity: u64) -> Self {
        Self {
            resource,
            queue_depth: 0,
            queue_capacity: capacity,
            service_rate: 0.0,
            arrival_rate: 0.0,
            cwnd: CongestionWindow::new(capacity / 4, capacity),
            level: CongestionLevel::None,
            backpressure: false,
            avg_latency_ns: 0.0,
        }
    }

    /// Update queue state
    #[inline]
    pub fn update_queue(&mut self, depth: u64) {
        self.queue_depth = depth;
        let util = if self.queue_capacity > 0 {
            depth as f64 / self.queue_capacity as f64
        } else {
            0.0
        };
        self.level = CongestionLevel::from_utilization(util);
    }

    /// Update rates
    #[inline(always)]
    pub fn update_rates(&mut self, arrival: f64, service: f64) {
        self.arrival_rate = arrival;
        self.service_rate = service;
    }

    /// Update latency (EMA alpha=0.1)
    #[inline(always)]
    pub fn update_latency(&mut self, latency_ns: u64) {
        let alpha = 0.1;
        self.avg_latency_ns = alpha * latency_ns as f64 + (1.0 - alpha) * self.avg_latency_ns;
    }

    /// Traffic intensity (rho = arrival / service)
    #[inline]
    pub fn traffic_intensity(&self) -> f64 {
        if self.service_rate <= 0.0 {
            return f64::MAX;
        }
        self.arrival_rate / self.service_rate
    }

    /// Determine backpressure action
    #[inline]
    pub fn backpressure_action(&self) -> BackpressureAction {
        match self.level {
            CongestionLevel::None => BackpressureAction::NoAction,
            CongestionLevel::Light => BackpressureAction::NoAction,
            CongestionLevel::Moderate => BackpressureAction::SlowDown,
            CongestionLevel::Severe => BackpressureAction::Pause,
            CongestionLevel::Collapse => BackpressureAction::ShedLoad,
        }
    }
}

// ============================================================================
// CONGESTION ENGINE
// ============================================================================

/// Congestion stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticCongestionStats {
    /// Resources tracked
    pub resources_tracked: usize,
    /// Congested resources
    pub congested_count: usize,
    /// Backpressure events
    pub backpressure_events: u64,
    /// Total load shed
    pub load_shed_count: u64,
}

/// Holistic congestion manager
pub struct HolisticCongestionEngine {
    /// Per-resource congestion
    resources: BTreeMap<u8, ResourceCongestion>,
    /// Global backpressure level
    pub global_level: CongestionLevel,
    /// Stats
    stats: HolisticCongestionStats,
}

impl HolisticCongestionEngine {
    pub fn new() -> Self {
        Self {
            resources: BTreeMap::new(),
            global_level: CongestionLevel::None,
            stats: HolisticCongestionStats::default(),
        }
    }

    /// Register resource
    #[inline]
    pub fn register(&mut self, resource: CongestionResource, capacity: u64) {
        let key = resource as u8;
        self.resources.insert(key, ResourceCongestion::new(resource, capacity));
        self.update_stats();
    }

    /// Update queue depth
    pub fn update_queue(&mut self, resource: CongestionResource, depth: u64) {
        let key = resource as u8;
        if let Some(rc) = self.resources.get_mut(&key) {
            rc.update_queue(depth);
            let action = rc.backpressure_action();
            if action != BackpressureAction::NoAction {
                rc.backpressure = true;
                self.stats.backpressure_events += 1;
            } else {
                rc.backpressure = false;
            }
        }
        self.update_global();
    }

    /// Record successful operation
    #[inline]
    pub fn on_success(&mut self, resource: CongestionResource) {
        let key = resource as u8;
        if let Some(rc) = self.resources.get_mut(&key) {
            rc.cwnd.on_success();
        }
    }

    /// Record congestion event
    #[inline]
    pub fn on_congestion(&mut self, resource: CongestionResource) {
        let key = resource as u8;
        if let Some(rc) = self.resources.get_mut(&key) {
            rc.cwnd.on_congestion();
        }
    }

    /// Get congested resources
    #[inline]
    pub fn congested_resources(&self) -> Vec<CongestionResource> {
        self.resources
            .values()
            .filter(|r| r.level >= CongestionLevel::Moderate)
            .map(|r| r.resource)
            .collect()
    }

    /// Global backpressure action
    #[inline]
    pub fn global_action(&self) -> BackpressureAction {
        match self.global_level {
            CongestionLevel::None => BackpressureAction::NoAction,
            CongestionLevel::Light => BackpressureAction::NoAction,
            CongestionLevel::Moderate => BackpressureAction::SlowDown,
            CongestionLevel::Severe => BackpressureAction::Pause,
            CongestionLevel::Collapse => BackpressureAction::ShedLoad,
        }
    }

    fn update_global(&mut self) {
        // Global = worst of any resource
        let worst = self.resources.values().map(|r| r.level).max();
        self.global_level = worst.unwrap_or(CongestionLevel::None);
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.resources_tracked = self.resources.len();
        self.stats.congested_count = self.resources
            .values()
            .filter(|r| r.level >= CongestionLevel::Moderate)
            .count();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticCongestionStats {
        &self.stats
    }
}
