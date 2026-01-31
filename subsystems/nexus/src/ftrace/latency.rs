//! Latency tracing.

use alloc::string::String;
use alloc::vec::Vec;

use super::types::{CpuId, FuncAddr, Pid};

// ============================================================================
// LATENCY TRACING
// ============================================================================

/// Latency type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LatencyType {
    /// IRQ disabled latency
    IrqOff,
    /// Preemption disabled latency
    PreemptOff,
    /// IRQ + preemption disabled
    IrqPreemptOff,
    /// Wakeup latency
    Wakeup,
    /// Hardware latency
    Hardware,
}

impl LatencyType {
    /// Get type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::IrqOff => "irq_off",
            Self::PreemptOff => "preempt_off",
            Self::IrqPreemptOff => "irq_preempt_off",
            Self::Wakeup => "wakeup",
            Self::Hardware => "hardware",
        }
    }
}

/// Latency record
#[derive(Debug, Clone)]
pub struct LatencyRecord {
    /// Latency type
    pub latency_type: LatencyType,
    /// Duration (ns)
    pub duration_ns: u64,
    /// Start timestamp
    pub start_ts: u64,
    /// End timestamp
    pub end_ts: u64,
    /// CPU
    pub cpu: CpuId,
    /// PID
    pub pid: Pid,
    /// Start function
    pub start_func: Option<String>,
    /// End function
    pub end_func: Option<String>,
    /// Backtrace
    pub backtrace: Vec<FuncAddr>,
}

impl LatencyRecord {
    /// Create new record
    pub fn new(
        latency_type: LatencyType,
        duration_ns: u64,
        start_ts: u64,
        end_ts: u64,
        cpu: CpuId,
        pid: Pid,
    ) -> Self {
        Self {
            latency_type,
            duration_ns,
            start_ts,
            end_ts,
            cpu,
            pid,
            start_func: None,
            end_func: None,
            backtrace: Vec::new(),
        }
    }

    /// Duration in microseconds
    pub fn duration_us(&self) -> u64 {
        self.duration_ns / 1000
    }

    /// Duration in milliseconds
    pub fn duration_ms(&self) -> f64 {
        self.duration_ns as f64 / 1_000_000.0
    }
}

/// Latency stats
#[derive(Debug, Clone, Default)]
pub struct LatencyStats {
    /// Count
    pub count: u64,
    /// Total (ns)
    pub total_ns: u64,
    /// Min (ns)
    pub min_ns: u64,
    /// Max (ns)
    pub max_ns: u64,
}

impl LatencyStats {
    /// Create new stats
    pub fn new() -> Self {
        Self {
            count: 0,
            total_ns: 0,
            min_ns: u64::MAX,
            max_ns: 0,
        }
    }

    /// Record latency
    pub fn record(&mut self, latency_ns: u64) {
        self.count += 1;
        self.total_ns += latency_ns;
        if latency_ns < self.min_ns {
            self.min_ns = latency_ns;
        }
        if latency_ns > self.max_ns {
            self.max_ns = latency_ns;
        }
    }

    /// Average latency
    pub fn avg_ns(&self) -> u64 {
        if self.count == 0 {
            return 0;
        }
        self.total_ns / self.count
    }
}
