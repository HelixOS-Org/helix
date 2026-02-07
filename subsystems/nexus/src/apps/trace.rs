//! # Application Trace Profiler
//!
//! Application execution tracing and call graph analysis:
//! - Function call tracing
//! - Call graph construction
//! - Hot path detection
//! - Flame graph data collection
//! - Execution time attribution

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// TRACE TYPES
// ============================================================================

/// Trace event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppTraceEventType {
    /// Function entry
    FunctionEntry,
    /// Function exit
    FunctionExit,
    /// Syscall entry
    SyscallEntry,
    /// Syscall exit
    SyscallExit,
    /// Context switch out
    ContextSwitchOut,
    /// Context switch in
    ContextSwitchIn,
    /// Page fault
    PageFault,
    /// Lock acquire
    LockAcquire,
    /// Lock release
    LockRelease,
}

/// A trace event
#[derive(Debug, Clone)]
pub struct AppTraceEvent {
    /// Event type
    pub event_type: AppTraceEventType,
    /// Instruction pointer
    pub ip: u64,
    /// Timestamp (ns)
    pub timestamp: u64,
    /// Thread id
    pub tid: u64,
    /// Stack depth
    pub depth: u32,
    /// Associated value
    pub value: u64,
}

// ============================================================================
// CALL GRAPH NODE
// ============================================================================

/// Call graph node
#[derive(Debug, Clone)]
pub struct CallNode {
    /// Function address
    pub address: u64,
    /// Self time (ns)
    pub self_time_ns: u64,
    /// Total time (ns, including children)
    pub total_time_ns: u64,
    /// Call count
    pub call_count: u64,
    /// Children (callee address -> call count)
    pub children: BTreeMap<u64, u64>,
    /// Last entry timestamp
    last_entry: u64,
}

impl CallNode {
    pub fn new(address: u64) -> Self {
        Self {
            address,
            self_time_ns: 0,
            total_time_ns: 0,
            call_count: 0,
            children: BTreeMap::new(),
            last_entry: 0,
        }
    }

    /// Record entry
    pub fn enter(&mut self, now: u64) {
        self.call_count += 1;
        self.last_entry = now;
    }

    /// Record exit
    pub fn exit(&mut self, now: u64) {
        let duration = now.saturating_sub(self.last_entry);
        self.total_time_ns += duration;
    }

    /// Record call to child
    pub fn call_child(&mut self, child_addr: u64) {
        *self.children.entry(child_addr).or_insert(0) += 1;
    }

    /// Average time per call
    pub fn avg_time_ns(&self) -> f64 {
        if self.call_count == 0 {
            return 0.0;
        }
        self.total_time_ns as f64 / self.call_count as f64
    }

    /// Children sorted by call count
    pub fn top_callees(&self, n: usize) -> Vec<(u64, u64)> {
        let mut callees: Vec<(u64, u64)> = self.children.iter().map(|(&a, &c)| (a, c)).collect();
        callees.sort_by(|a, b| b.1.cmp(&a.1));
        callees.truncate(n);
        callees
    }
}

// ============================================================================
// CALL GRAPH
// ============================================================================

/// Call graph for a thread
#[derive(Debug)]
pub struct AppCallGraph {
    /// Nodes
    nodes: BTreeMap<u64, CallNode>,
    /// Current call stack
    stack: Vec<u64>,
    /// Root node address
    pub root: u64,
}

impl AppCallGraph {
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            stack: Vec::new(),
            root: 0,
        }
    }

    /// Process function entry
    pub fn on_entry(&mut self, addr: u64, now: u64) {
        if self.stack.is_empty() {
            self.root = addr;
        }

        // Record call from parent to child
        if let Some(&parent) = self.stack.last() {
            if let Some(parent_node) = self.nodes.get_mut(&parent) {
                parent_node.call_child(addr);
            }
        }

        let node = self.nodes.entry(addr).or_insert_with(|| CallNode::new(addr));
        node.enter(now);
        self.stack.push(addr);
    }

    /// Process function exit
    pub fn on_exit(&mut self, addr: u64, now: u64) {
        if let Some(node) = self.nodes.get_mut(&addr) {
            node.exit(now);
        }
        if self.stack.last() == Some(&addr) {
            self.stack.pop();
        }
    }

    /// Get hot functions (top N by total time)
    pub fn hot_functions(&self, n: usize) -> Vec<&CallNode> {
        let mut sorted: Vec<&CallNode> = self.nodes.values().collect();
        sorted.sort_by(|a, b| b.total_time_ns.cmp(&a.total_time_ns));
        sorted.truncate(n);
        sorted
    }

    /// Most called functions
    pub fn most_called(&self, n: usize) -> Vec<&CallNode> {
        let mut sorted: Vec<&CallNode> = self.nodes.values().collect();
        sorted.sort_by(|a, b| b.call_count.cmp(&a.call_count));
        sorted.truncate(n);
        sorted
    }

    /// Node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Current stack depth
    pub fn depth(&self) -> usize {
        self.stack.len()
    }
}

// ============================================================================
// FLAME GRAPH DATA
// ============================================================================

/// Flame graph stack entry
#[derive(Debug, Clone)]
pub struct FlameStack {
    /// Stack (bottom to top function addresses)
    pub stack: Vec<u64>,
    /// Sample count
    pub samples: u64,
}

/// Flame graph collector
#[derive(Debug)]
pub struct FlameGraphCollector {
    /// Stacks
    stacks: BTreeMap<u64, FlameStack>,
    /// Total samples
    pub total_samples: u64,
}

impl FlameGraphCollector {
    pub fn new() -> Self {
        Self {
            stacks: BTreeMap::new(),
            total_samples: 0,
        }
    }

    fn stack_key(stack: &[u64]) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for &addr in stack {
            hash ^= addr;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    /// Record a stack sample
    pub fn record(&mut self, stack: Vec<u64>) {
        let key = Self::stack_key(&stack);
        self.total_samples += 1;
        if let Some(entry) = self.stacks.get_mut(&key) {
            entry.samples += 1;
        } else {
            self.stacks.insert(key, FlameStack { stack, samples: 1 });
        }
    }

    /// Top stacks by sample count
    pub fn top_stacks(&self, n: usize) -> Vec<&FlameStack> {
        let mut sorted: Vec<&FlameStack> = self.stacks.values().collect();
        sorted.sort_by(|a, b| b.samples.cmp(&a.samples));
        sorted.truncate(n);
        sorted
    }

    /// Unique stack count
    pub fn unique_stacks(&self) -> usize {
        self.stacks.len()
    }
}

// ============================================================================
// TRACE ENGINE
// ============================================================================

/// Trace stats
#[derive(Debug, Clone, Default)]
pub struct AppTraceStats {
    /// Processes traced
    pub traced_processes: usize,
    /// Total events
    pub total_events: u64,
    /// Total functions discovered
    pub total_functions: u64,
}

/// App trace profiler
pub struct AppTraceProfiler {
    /// Call graphs per (pid, tid) hash
    graphs: BTreeMap<u64, AppCallGraph>,
    /// Flame graph collector
    pub flames: FlameGraphCollector,
    /// Stats
    stats: AppTraceStats,
}

impl AppTraceProfiler {
    pub fn new() -> Self {
        Self {
            graphs: BTreeMap::new(),
            flames: FlameGraphCollector::new(),
            stats: AppTraceStats::default(),
        }
    }

    fn thread_key(pid: u64, tid: u64) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        hash ^= pid;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= tid;
        hash = hash.wrapping_mul(0x100000001b3);
        hash
    }

    /// Process trace event
    pub fn process_event(&mut self, pid: u64, event: AppTraceEvent) {
        let key = Self::thread_key(pid, event.tid);
        let graph = self.graphs.entry(key).or_insert_with(AppCallGraph::new);

        match event.event_type {
            AppTraceEventType::FunctionEntry => {
                graph.on_entry(event.ip, event.timestamp);
            }
            AppTraceEventType::FunctionExit => {
                graph.on_exit(event.ip, event.timestamp);
            }
            _ => {}
        }
        self.stats.total_events += 1;
        self.update_stats();
    }

    /// Get call graph for thread
    pub fn graph(&self, pid: u64, tid: u64) -> Option<&AppCallGraph> {
        let key = Self::thread_key(pid, tid);
        self.graphs.get(&key)
    }

    /// Remove process
    pub fn remove_process(&mut self, pid: u64) {
        // Remove all graphs that belong to this process
        // Since keys are hashed, we'd need to track pid->keys mapping
        // For simplicity, this is a placeholder
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.traced_processes = self.graphs.len();
        self.stats.total_functions = self.graphs.values().map(|g| g.node_count() as u64).sum();
    }

    /// Stats
    pub fn stats(&self) -> &AppTraceStats {
        &self.stats
    }
}
