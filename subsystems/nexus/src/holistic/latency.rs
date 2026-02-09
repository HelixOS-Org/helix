//! # Holistic Latency Analysis
//!
//! End-to-end latency tracking across the entire system:
//! - Request path tracing
//! - Latency breakdown by component
//! - Tail latency analysis (p50, p95, p99, p999)
//! - Latency budget tracking
//! - Bottleneck identification
//! - Latency SLA monitoring

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

// ============================================================================
// LATENCY PATH
// ============================================================================

/// Component in a latency path
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LatencyComponent {
    /// User-space processing
    UserSpace,
    /// Syscall entry overhead
    SyscallEntry,
    /// Scheduler decision
    Scheduler,
    /// Context switch
    ContextSwitch,
    /// Memory allocation
    MemoryAlloc,
    /// Page fault handling
    PageFault,
    /// I/O submission
    IoSubmit,
    /// I/O completion
    IoComplete,
    /// Network stack
    NetworkStack,
    /// IPC delivery
    IpcDelivery,
    /// Lock contention
    LockContention,
    /// Interrupt handling
    InterruptHandling,
    /// Syscall exit overhead
    SyscallExit,
}

/// A span in a latency trace
#[derive(Debug, Clone)]
pub struct LatencySpan {
    /// Component
    pub component: LatencyComponent,
    /// Start timestamp (ns)
    pub start_ns: u64,
    /// End timestamp (ns)
    pub end_ns: u64,
    /// Duration (ns)
    pub duration_ns: u64,
    /// Was on critical path
    pub on_critical_path: bool,
}

/// Complete request trace
#[derive(Debug, Clone)]
pub struct RequestTrace {
    /// Trace ID
    pub id: u64,
    /// Process ID
    pub pid: u64,
    /// Spans
    pub spans: Vec<LatencySpan>,
    /// Total latency (ns)
    pub total_ns: u64,
    /// Critical path latency (ns)
    pub critical_path_ns: u64,
    /// Timestamp
    pub timestamp: u64,
}

impl RequestTrace {
    pub fn new(id: u64, pid: u64, timestamp: u64) -> Self {
        Self {
            id,
            pid,
            spans: Vec::new(),
            total_ns: 0,
            critical_path_ns: 0,
            timestamp,
        }
    }

    /// Add span
    #[inline]
    pub fn add_span(&mut self, span: LatencySpan) {
        self.total_ns += span.duration_ns;
        if span.on_critical_path {
            self.critical_path_ns += span.duration_ns;
        }
        self.spans.push(span);
    }

    /// Breakdown by component (percentage)
    pub fn breakdown(&self) -> BTreeMap<u8, u32> {
        let mut result = BTreeMap::new();
        if self.total_ns == 0 {
            return result;
        }

        for span in &self.spans {
            let entry = result.entry(span.component as u8).or_insert(0u32);
            *entry += (span.duration_ns * 100 / self.total_ns) as u32;
        }
        result
    }

    /// Bottleneck component
    pub fn bottleneck(&self) -> Option<LatencyComponent> {
        let mut component_time: BTreeMap<u8, u64> = BTreeMap::new();
        for span in &self.spans {
            *component_time.entry(span.component as u8).or_insert(0) += span.duration_ns;
        }

        component_time
            .into_iter()
            .max_by_key(|(_, t)| *t)
            .and_then(|(c, _)| {
                // Map back to LatencyComponent
                match c {
                    0 => Some(LatencyComponent::UserSpace),
                    1 => Some(LatencyComponent::SyscallEntry),
                    2 => Some(LatencyComponent::Scheduler),
                    3 => Some(LatencyComponent::ContextSwitch),
                    4 => Some(LatencyComponent::MemoryAlloc),
                    5 => Some(LatencyComponent::PageFault),
                    6 => Some(LatencyComponent::IoSubmit),
                    7 => Some(LatencyComponent::IoComplete),
                    8 => Some(LatencyComponent::NetworkStack),
                    9 => Some(LatencyComponent::IpcDelivery),
                    10 => Some(LatencyComponent::LockContention),
                    11 => Some(LatencyComponent::InterruptHandling),
                    12 => Some(LatencyComponent::SyscallExit),
                    _ => None,
                }
            })
    }
}

// ============================================================================
// PERCENTILE HISTOGRAM
// ============================================================================

/// Latency percentile histogram
#[derive(Debug, Clone)]
pub struct LatencyPercentiles {
    /// Sorted values (ns)
    values: VecDeque<u64>,
    /// Max values
    max_values: usize,
}

impl LatencyPercentiles {
    pub fn new(max_values: usize) -> Self {
        Self {
            values: VecDeque::new(),
            max_values,
        }
    }

    /// Record value
    #[inline]
    pub fn record(&mut self, ns: u64) {
        // Insert sorted
        let pos = self.values.partition_point(|&v| v < ns);
        self.values.insert(pos, ns);
        if self.values.len() > self.max_values {
            self.values.pop_front();
        }
    }

    /// Get percentile
    #[inline]
    pub fn percentile(&self, p: f64) -> u64 {
        if self.values.is_empty() {
            return 0;
        }
        let idx = ((self.values.len() as f64 * p / 100.0) as usize).min(self.values.len() - 1);
        self.values[idx]
    }

    #[inline(always)]
    pub fn p50(&self) -> u64 {
        self.percentile(50.0)
    }
    #[inline(always)]
    pub fn p90(&self) -> u64 {
        self.percentile(90.0)
    }
    #[inline(always)]
    pub fn p95(&self) -> u64 {
        self.percentile(95.0)
    }
    #[inline(always)]
    pub fn p99(&self) -> u64 {
        self.percentile(99.0)
    }
    #[inline(always)]
    pub fn p999(&self) -> u64 {
        self.percentile(99.9)
    }

    #[inline(always)]
    pub fn min(&self) -> u64 {
        self.values.first().copied().unwrap_or(0)
    }
    #[inline(always)]
    pub fn max(&self) -> u64 {
        self.values.back().copied().unwrap_or(0)
    }

    #[inline(always)]
    pub fn count(&self) -> usize {
        self.values.len()
    }

    #[inline]
    pub fn mean(&self) -> u64 {
        if self.values.is_empty() {
            return 0;
        }
        self.values.iter().sum::<u64>() / self.values.len() as u64
    }
}

// ============================================================================
// LATENCY BUDGET
// ============================================================================

/// Latency budget for a request type
#[derive(Debug, Clone)]
pub struct LatencyBudget {
    /// Request type ID
    pub request_type: u64,
    /// Total budget (ns)
    pub total_budget_ns: u64,
    /// Per-component budgets (ns)
    pub component_budgets: BTreeMap<u8, u64>,
    /// Violations
    pub violations: u64,
    /// Checks
    pub checks: u64,
}

impl LatencyBudget {
    pub fn new(request_type: u64, total_ns: u64) -> Self {
        Self {
            request_type,
            total_budget_ns: total_ns,
            component_budgets: BTreeMap::new(),
            violations: 0,
            checks: 0,
        }
    }

    /// Set component budget
    #[inline(always)]
    pub fn set_component_budget(&mut self, component: LatencyComponent, ns: u64) {
        self.component_budgets.insert(component as u8, ns);
    }

    /// Check trace against budget
    #[inline]
    pub fn check(&mut self, trace: &RequestTrace) -> bool {
        self.checks += 1;
        if trace.total_ns > self.total_budget_ns {
            self.violations += 1;
            return false;
        }
        true
    }

    /// Violation rate
    #[inline]
    pub fn violation_rate(&self) -> f64 {
        if self.checks == 0 {
            return 0.0;
        }
        self.violations as f64 / self.checks as f64
    }
}

// ============================================================================
// LATENCY ANALYZER
// ============================================================================

/// Latency analyzer stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct LatencyAnalyzerStats {
    /// Total traces
    pub total_traces: u64,
    /// Active traces
    pub active_traces: usize,
    /// Average latency (ns)
    pub avg_latency_ns: u64,
    /// P99 latency (ns)
    pub p99_latency_ns: u64,
    /// Budget violations
    pub budget_violations: u64,
    /// Bottleneck component
    pub primary_bottleneck: u8,
}

/// Holistic latency analyzer
pub struct HolisticLatencyAnalyzer {
    /// Active traces
    active_traces: BTreeMap<u64, RequestTrace>,
    /// Completed traces (ring buffer)
    completed: VecDeque<RequestTrace>,
    /// Percentile tracker
    percentiles: LatencyPercentiles,
    /// Per-component percentiles
    component_percentiles: BTreeMap<u8, LatencyPercentiles>,
    /// Latency budgets
    budgets: BTreeMap<u64, LatencyBudget>,
    /// Bottleneck counter (component → total ns)
    bottleneck_totals: BTreeMap<u8, u64>,
    /// Next trace ID
    next_id: u64,
    /// Max completed traces
    max_completed: usize,
    /// Stats
    stats: LatencyAnalyzerStats,
}

impl HolisticLatencyAnalyzer {
    pub fn new() -> Self {
        Self {
            active_traces: BTreeMap::new(),
            completed: VecDeque::new(),
            percentiles: LatencyPercentiles::new(10000),
            component_percentiles: BTreeMap::new(),
            budgets: BTreeMap::new(),
            bottleneck_totals: BTreeMap::new(),
            next_id: 1,
            max_completed: 1024,
            stats: LatencyAnalyzerStats::default(),
        }
    }

    /// Start new trace
    #[inline]
    pub fn start_trace(&mut self, pid: u64, timestamp: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.active_traces
            .insert(id, RequestTrace::new(id, pid, timestamp));
        self.stats.active_traces = self.active_traces.len();
        id
    }

    /// Add span to trace
    #[inline]
    pub fn add_span(&mut self, trace_id: u64, span: LatencySpan) {
        if let Some(trace) = self.active_traces.get_mut(&trace_id) {
            trace.add_span(span);
        }
    }

    /// Complete trace
    pub fn complete_trace(&mut self, trace_id: u64) -> Option<u64> {
        let trace = self.active_traces.remove(&trace_id)?;
        let total = trace.total_ns;

        // Record percentile
        self.percentiles.record(total);

        // Per-component
        for span in &trace.spans {
            let perc = self
                .component_percentiles
                .entry(span.component as u8)
                .or_insert_with(|| LatencyPercentiles::new(5000));
            perc.record(span.duration_ns);

            *self
                .bottleneck_totals
                .entry(span.component as u8)
                .or_insert(0) += span.duration_ns;
        }

        // Check budgets
        // (Would check against request type, simplified here)

        self.completed.push_back(trace);
        if self.completed.len() > self.max_completed {
            self.completed.pop_front();
        }

        self.stats.total_traces += 1;
        self.stats.active_traces = self.active_traces.len();
        self.stats.avg_latency_ns = self.percentiles.mean();
        self.stats.p99_latency_ns = self.percentiles.p99();

        // Update primary bottleneck
        if let Some((&comp, _)) = self.bottleneck_totals.iter().max_by_key(|(_, &t)| t) {
            self.stats.primary_bottleneck = comp;
        }

        Some(total)
    }

    /// Set budget
    #[inline(always)]
    pub fn set_budget(&mut self, budget: LatencyBudget) {
        self.budgets.insert(budget.request_type, budget);
    }

    /// Get percentiles
    #[inline(always)]
    pub fn percentiles(&self) -> &LatencyPercentiles {
        &self.percentiles
    }

    /// Get component percentiles
    #[inline(always)]
    pub fn component_percentiles(&self, component: LatencyComponent) -> Option<&LatencyPercentiles> {
        self.component_percentiles.get(&(component as u8))
    }

    /// Get stats
    #[inline(always)]
    pub fn stats(&self) -> &LatencyAnalyzerStats {
        &self.stats
    }
}
