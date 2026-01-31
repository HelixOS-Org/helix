//! Tracer types and configuration.

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// TRACER TYPES
// ============================================================================

/// Tracer type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TracerType {
    /// No tracer
    Nop,
    /// Function tracer
    Function,
    /// Function graph tracer
    FunctionGraph,
    /// Hardware latency tracer
    HwLat,
    /// IRQ off tracer
    IrqsOff,
    /// Preempt off tracer
    PreemptOff,
    /// IRQ + preempt off tracer
    PreemptirqsOff,
    /// Wakeup tracer
    Wakeup,
    /// Wakeup RT tracer
    WakeupRt,
    /// Wakeup DL tracer
    WakeupDl,
    /// mmio tracer
    Mmiotrace,
    /// Branch tracer
    Branch,
    /// Block tracer
    Blk,
}

impl TracerType {
    /// Get tracer name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Nop => "nop",
            Self::Function => "function",
            Self::FunctionGraph => "function_graph",
            Self::HwLat => "hwlat",
            Self::IrqsOff => "irqsoff",
            Self::PreemptOff => "preemptoff",
            Self::PreemptirqsOff => "preemptirqsoff",
            Self::Wakeup => "wakeup",
            Self::WakeupRt => "wakeup_rt",
            Self::WakeupDl => "wakeup_dl",
            Self::Mmiotrace => "mmiotrace",
            Self::Branch => "branch",
            Self::Blk => "blk",
        }
    }

    /// Is latency tracer
    pub fn is_latency_tracer(&self) -> bool {
        matches!(
            self,
            Self::IrqsOff
                | Self::PreemptOff
                | Self::PreemptirqsOff
                | Self::HwLat
                | Self::Wakeup
                | Self::WakeupRt
                | Self::WakeupDl
        )
    }
}

/// Tracer options
#[derive(Debug, Clone)]
pub struct TracerOptions {
    /// Function filters
    pub func_filter: Vec<String>,
    /// Function no-trace
    pub func_notrace: Vec<String>,
    /// Graph depth
    pub graph_depth: u32,
    /// Graph time threshold (ns)
    pub graph_time_ns: u64,
    /// Include sleep time
    pub include_sleep: bool,
    /// Trace children
    pub trace_children: bool,
    /// Max graph depth
    pub max_depth: u32,
}

impl Default for TracerOptions {
    fn default() -> Self {
        Self {
            func_filter: Vec::new(),
            func_notrace: Vec::new(),
            graph_depth: 0,
            graph_time_ns: 0,
            include_sleep: false,
            trace_children: true,
            max_depth: 16,
        }
    }
}
