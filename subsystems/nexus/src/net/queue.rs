//! Network Queue and Traffic Analysis
//!
//! Queue disciplines and traffic pattern analysis.

// ============================================================================
// QUEUE DISCIPLINE
// ============================================================================

/// Queue discipline type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QdiscType {
    /// Priority fifo fast
    PfifoFast,
    /// Fair queue
    Fq,
    /// Fair queue CoDel
    FqCodel,
    /// Hierarchical token bucket
    Htb,
    /// Token bucket filter
    Tbf,
    /// Stochastic fair queueing
    Sfq,
    /// Multi-queue priority
    Mqprio,
    /// Network emulator
    Netem,
    /// Ingress
    Ingress,
    /// Clsact
    Clsact,
    /// No queue
    Noqueue,
    /// Unknown
    Unknown,
}

impl QdiscType {
    /// Get type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::PfifoFast => "pfifo_fast",
            Self::Fq => "fq",
            Self::FqCodel => "fq_codel",
            Self::Htb => "htb",
            Self::Tbf => "tbf",
            Self::Sfq => "sfq",
            Self::Mqprio => "mqprio",
            Self::Netem => "netem",
            Self::Ingress => "ingress",
            Self::Clsact => "clsact",
            Self::Noqueue => "noqueue",
            Self::Unknown => "unknown",
        }
    }
}

// ============================================================================
// TRAFFIC ANALYSIS
// ============================================================================

/// Traffic pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficPattern {
    /// Mostly receive
    ReceiveHeavy,
    /// Mostly transmit
    TransmitHeavy,
    /// Balanced
    Balanced,
    /// Bursty
    Bursty,
    /// Idle
    Idle,
    /// Unknown
    Unknown,
}

impl TrafficPattern {
    /// Get pattern name
    pub fn name(&self) -> &'static str {
        match self {
            Self::ReceiveHeavy => "rx_heavy",
            Self::TransmitHeavy => "tx_heavy",
            Self::Balanced => "balanced",
            Self::Bursty => "bursty",
            Self::Idle => "idle",
            Self::Unknown => "unknown",
        }
    }
}

/// Traffic analysis
#[derive(Debug, Clone)]
pub struct TrafficAnalysis {
    /// Pattern
    pub pattern: TrafficPattern,
    /// RX rate (bytes/sec)
    pub rx_rate: u64,
    /// TX rate (bytes/sec)
    pub tx_rate: u64,
    /// RX PPS
    pub rx_pps: u64,
    /// TX PPS
    pub tx_pps: u64,
    /// Avg packet size
    pub avg_packet_size: u64,
    /// RX utilization
    pub rx_utilization: f32,
    /// TX utilization
    pub tx_utilization: f32,
}

impl TrafficAnalysis {
    /// Create new analysis
    pub fn new() -> Self {
        Self {
            pattern: TrafficPattern::Unknown,
            rx_rate: 0,
            tx_rate: 0,
            rx_pps: 0,
            tx_pps: 0,
            avg_packet_size: 0,
            rx_utilization: 0.0,
            tx_utilization: 0.0,
        }
    }

    /// Classify traffic
    pub fn classify(&mut self) {
        let total_rate = self.rx_rate + self.tx_rate;

        if total_rate == 0 {
            self.pattern = TrafficPattern::Idle;
            return;
        }

        let rx_ratio = self.rx_rate as f32 / total_rate as f32;

        self.pattern = if rx_ratio > 0.8 {
            TrafficPattern::ReceiveHeavy
        } else if rx_ratio < 0.2 {
            TrafficPattern::TransmitHeavy
        } else {
            TrafficPattern::Balanced
        };
    }
}

impl Default for TrafficAnalysis {
    fn default() -> Self {
        Self::new()
    }
}
