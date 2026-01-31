//! Trace entries.

use alloc::string::String;

use super::types::{CpuId, FuncAddr, Pid};

// ============================================================================
// TRACE ENTRIES
// ============================================================================

/// Trace entry type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceEntryType {
    /// Function entry
    FuncEntry,
    /// Function exit
    FuncExit,
    /// Graph entry
    GraphEntry,
    /// Graph exit
    GraphExit,
    /// Wakeup
    Wakeup,
    /// Context switch
    ContextSwitch,
    /// IRQ entry
    IrqEntry,
    /// IRQ exit
    IrqExit,
    /// Softirq entry
    SoftirqEntry,
    /// Softirq exit
    SoftirqExit,
    /// Printk
    Print,
    /// User marker
    UserMarker,
}

impl TraceEntryType {
    /// Get type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::FuncEntry => "funcentry",
            Self::FuncExit => "funcexit",
            Self::GraphEntry => "funcgraph_entry",
            Self::GraphExit => "funcgraph_exit",
            Self::Wakeup => "wakeup",
            Self::ContextSwitch => "sched_switch",
            Self::IrqEntry => "irq_entry",
            Self::IrqExit => "irq_exit",
            Self::SoftirqEntry => "softirq_entry",
            Self::SoftirqExit => "softirq_exit",
            Self::Print => "print",
            Self::UserMarker => "user_marker",
        }
    }
}

/// Trace entry
#[derive(Debug, Clone)]
pub struct TraceEntry {
    /// Timestamp (ns)
    pub timestamp: u64,
    /// Entry type
    pub entry_type: TraceEntryType,
    /// CPU
    pub cpu: CpuId,
    /// PID
    pub pid: Pid,
    /// Function address
    pub func: Option<FuncAddr>,
    /// Function name
    pub func_name: Option<String>,
    /// Duration (for exit entries)
    pub duration_ns: Option<u64>,
    /// Depth (for graph entries)
    pub depth: Option<u32>,
    /// Extra data
    pub extra: Option<String>,
}

impl TraceEntry {
    /// Create function entry
    pub fn func_entry(timestamp: u64, cpu: CpuId, pid: Pid, func: FuncAddr, name: String) -> Self {
        Self {
            timestamp,
            entry_type: TraceEntryType::FuncEntry,
            cpu,
            pid,
            func: Some(func),
            func_name: Some(name),
            duration_ns: None,
            depth: None,
            extra: None,
        }
    }

    /// Create function exit
    pub fn func_exit(
        timestamp: u64,
        cpu: CpuId,
        pid: Pid,
        func: FuncAddr,
        name: String,
        duration_ns: u64,
    ) -> Self {
        Self {
            timestamp,
            entry_type: TraceEntryType::FuncExit,
            cpu,
            pid,
            func: Some(func),
            func_name: Some(name),
            duration_ns: Some(duration_ns),
            depth: None,
            extra: None,
        }
    }

    /// Create graph entry
    pub fn graph_entry(
        timestamp: u64,
        cpu: CpuId,
        pid: Pid,
        func: FuncAddr,
        name: String,
        depth: u32,
    ) -> Self {
        Self {
            timestamp,
            entry_type: TraceEntryType::GraphEntry,
            cpu,
            pid,
            func: Some(func),
            func_name: Some(name),
            duration_ns: None,
            depth: Some(depth),
            extra: None,
        }
    }

    /// Create graph exit
    pub fn graph_exit(
        timestamp: u64,
        cpu: CpuId,
        pid: Pid,
        func: FuncAddr,
        name: String,
        depth: u32,
        duration_ns: u64,
    ) -> Self {
        Self {
            timestamp,
            entry_type: TraceEntryType::GraphExit,
            cpu,
            pid,
            func: Some(func),
            func_name: Some(name),
            duration_ns: Some(duration_ns),
            depth: Some(depth),
            extra: None,
        }
    }
}
