//! Ftrace buffer and manager.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::callgraph::CallGraph;
use super::entry::TraceEntry;
use super::function::FunctionInfo;
use super::latency::{LatencyRecord, LatencyStats, LatencyType};
use super::tracer::{TracerOptions, TracerType};
use super::types::{CpuId, FuncAddr};

// ============================================================================
// TRACE BUFFER
// ============================================================================

/// Ftrace buffer
#[derive(Debug)]
pub struct TraceBuffer {
    /// Entries
    entries: Vec<TraceEntry>,
    /// Max entries
    max_entries: usize,
    /// Entry count
    entry_count: AtomicU64,
    /// Lost entries
    lost_entries: AtomicU64,
}

impl TraceBuffer {
    /// Create new buffer
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries,
            entry_count: AtomicU64::new(0),
            lost_entries: AtomicU64::new(0),
        }
    }

    /// Add entry
    pub fn add(&mut self, entry: TraceEntry) {
        self.entry_count.fetch_add(1, Ordering::Relaxed);
        if self.entries.len() >= self.max_entries {
            self.lost_entries.fetch_add(1, Ordering::Relaxed);
            self.entries.remove(0);
        }
        self.entries.push(entry);
    }

    /// Get entries
    pub fn entries(&self) -> &[TraceEntry] {
        &self.entries
    }

    /// Clear
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Entry count
    pub fn entry_count(&self) -> u64 {
        self.entry_count.load(Ordering::Relaxed)
    }

    /// Lost entries
    pub fn lost_entries(&self) -> u64 {
        self.lost_entries.load(Ordering::Relaxed)
    }
}

// ============================================================================
// FTRACE MANAGER
// ============================================================================

/// Ftrace manager
pub struct FtraceManager {
    /// Current tracer
    current_tracer: TracerType,
    /// Tracer options
    options: TracerOptions,
    /// Trace buffer per CPU
    pub(crate) buffers: BTreeMap<CpuId, TraceBuffer>,
    /// Function info
    functions: BTreeMap<FuncAddr, FunctionInfo>,
    /// Latency records
    pub(crate) latency_records: Vec<LatencyRecord>,
    /// Max latency records
    max_latency_records: usize,
    /// Call graph
    pub(crate) call_graph: CallGraph,
    /// Latency stats per type
    latency_stats: BTreeMap<LatencyType, LatencyStats>,
    /// Enabled
    #[allow(dead_code)]
    enabled: AtomicBool,
    /// Tracing
    tracing: AtomicBool,
}

impl FtraceManager {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            current_tracer: TracerType::Nop,
            options: TracerOptions::default(),
            buffers: BTreeMap::new(),
            functions: BTreeMap::new(),
            latency_records: Vec::new(),
            max_latency_records: 1000,
            call_graph: CallGraph::new(),
            latency_stats: BTreeMap::new(),
            enabled: AtomicBool::new(true),
            tracing: AtomicBool::new(false),
        }
    }

    /// Set tracer
    pub fn set_tracer(&mut self, tracer: TracerType) {
        self.current_tracer = tracer;
    }

    /// Get current tracer
    pub fn current_tracer(&self) -> TracerType {
        self.current_tracer
    }

    /// Start tracing
    pub fn start(&self) {
        self.tracing.store(true, Ordering::Relaxed);
    }

    /// Stop tracing
    pub fn stop(&self) {
        self.tracing.store(false, Ordering::Relaxed);
    }

    /// Is tracing
    pub fn is_tracing(&self) -> bool {
        self.tracing.load(Ordering::Relaxed)
    }

    /// Add CPU buffer
    pub fn add_cpu_buffer(&mut self, cpu: CpuId, size: usize) {
        self.buffers.insert(cpu, TraceBuffer::new(size));
    }

    /// Record trace entry
    pub fn record_entry(&mut self, cpu: CpuId, entry: TraceEntry) {
        if !self.is_tracing() {
            return;
        }

        // Update function stats
        if let Some(func) = entry.func {
            if let Some(duration) = entry.duration_ns {
                if let Some(info) = self.functions.get(&func) {
                    info.record_hit(duration);
                }
            }
        }

        if let Some(buffer) = self.buffers.get_mut(&cpu) {
            buffer.add(entry);
        }
    }

    /// Record latency
    pub fn record_latency(&mut self, record: LatencyRecord) {
        let latency_type = record.latency_type;

        // Update stats
        match self.latency_stats.get_mut(&latency_type) {
            Some(stats) => stats.record(record.duration_ns),
            None => {
                let mut stats = LatencyStats::new();
                stats.record(record.duration_ns);
                self.latency_stats.insert(latency_type, stats);
            },
        }

        // Store record
        if self.latency_records.len() >= self.max_latency_records {
            self.latency_records.remove(0);
        }
        self.latency_records.push(record);
    }

    /// Register function
    pub fn register_function(&mut self, info: FunctionInfo) {
        self.functions.insert(info.addr, info);
    }

    /// Get function
    pub fn get_function(&self, addr: FuncAddr) -> Option<&FunctionInfo> {
        self.functions.get(&addr)
    }

    /// Get buffer
    pub fn get_buffer(&self, cpu: CpuId) -> Option<&TraceBuffer> {
        self.buffers.get(&cpu)
    }

    /// Get call graph
    pub fn call_graph(&self) -> &CallGraph {
        &self.call_graph
    }

    /// Get latency stats
    pub fn latency_stats(&self, latency_type: LatencyType) -> Option<&LatencyStats> {
        self.latency_stats.get(&latency_type)
    }

    /// Max latency
    pub fn max_latency(&self) -> Option<&LatencyRecord> {
        self.latency_records.iter().max_by_key(|r| r.duration_ns)
    }

    /// Clear all buffers
    pub fn clear(&mut self) {
        for buffer in self.buffers.values_mut() {
            buffer.clear();
        }
        self.latency_records.clear();
        self.latency_stats.clear();
    }

    /// Set options
    pub fn set_options(&mut self, options: TracerOptions) {
        self.options = options;
    }
}

impl Default for FtraceManager {
    fn default() -> Self {
        Self::new()
    }
}
