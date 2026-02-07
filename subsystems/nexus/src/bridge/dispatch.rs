//! # Bridge Dispatch Optimizer
//!
//! Fast-path syscall dispatch optimization:
//! - Direct dispatch table construction
//! - Hot-path identification and specialization
//! - Inline handler caching
//! - Dispatch latency tracking
//! - Speculative handler pre-selection

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// DISPATCH TYPES
// ============================================================================

/// Handler type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandlerType {
    /// Fast inline handler
    FastInline,
    /// Standard handler
    Standard,
    /// Slow path (validation required)
    SlowPath,
    /// Emulated handler (compatibility)
    Emulated,
    /// Batched handler
    Batched,
    /// Redirected to another handler
    Redirected,
}

/// Dispatch decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchDecision {
    /// Execute immediately
    Immediate,
    /// Queue for batch processing
    QueueBatch,
    /// Redirect to alternate handler
    Redirect,
    /// Deny (no handler)
    Deny,
    /// Defer (resource busy)
    Defer,
}

/// Handler state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandlerState {
    /// Ready to serve
    Ready,
    /// Warming up (JIT)
    Warming,
    /// Overloaded
    Overloaded,
    /// Disabled
    Disabled,
}

// ============================================================================
// HANDLER ENTRY
// ============================================================================

/// Dispatch table handler entry
#[derive(Debug, Clone)]
pub struct HandlerEntry {
    /// Syscall number
    pub syscall_nr: u32,
    /// Handler type
    pub handler_type: HandlerType,
    /// State
    pub state: HandlerState,
    /// Invocation count
    pub invocations: u64,
    /// Total latency (ns)
    pub total_latency_ns: u64,
    /// Min latency (ns)
    pub min_latency_ns: u64,
    /// Max latency (ns)
    pub max_latency_ns: u64,
    /// Error count
    pub errors: u64,
    /// Is hot path
    pub is_hot: bool,
    /// Redirect target (if any)
    pub redirect_to: Option<u32>,
}

impl HandlerEntry {
    pub fn new(syscall_nr: u32, handler_type: HandlerType) -> Self {
        Self {
            syscall_nr,
            handler_type,
            state: HandlerState::Ready,
            invocations: 0,
            total_latency_ns: 0,
            min_latency_ns: u64::MAX,
            max_latency_ns: 0,
            errors: 0,
            is_hot: false,
            redirect_to: None,
        }
    }

    /// Record invocation
    pub fn record(&mut self, latency_ns: u64, success: bool) {
        self.invocations += 1;
        self.total_latency_ns += latency_ns;
        if latency_ns < self.min_latency_ns {
            self.min_latency_ns = latency_ns;
        }
        if latency_ns > self.max_latency_ns {
            self.max_latency_ns = latency_ns;
        }
        if !success {
            self.errors += 1;
        }
    }

    /// Average latency
    pub fn avg_latency_ns(&self) -> u64 {
        if self.invocations == 0 {
            return 0;
        }
        self.total_latency_ns / self.invocations
    }

    /// Error rate
    pub fn error_rate(&self) -> f64 {
        if self.invocations == 0 {
            return 0.0;
        }
        self.errors as f64 / self.invocations as f64
    }

    /// Throughput (invocations per second given total time)
    pub fn throughput(&self) -> f64 {
        if self.total_latency_ns == 0 {
            return 0.0;
        }
        self.invocations as f64 / (self.total_latency_ns as f64 / 1_000_000_000.0)
    }
}

// ============================================================================
// DISPATCH TABLE
// ============================================================================

/// Hot path threshold
const HOT_PATH_THRESHOLD: u64 = 1000;

/// Dispatch table
#[derive(Debug)]
pub struct DispatchTable {
    /// Handlers keyed by syscall number
    handlers: BTreeMap<u32, HandlerEntry>,
    /// Hot path cache (sorted by frequency)
    hot_cache: Vec<u32>,
    /// Hot cache capacity
    hot_cache_capacity: usize,
}

impl DispatchTable {
    pub fn new(hot_cache_capacity: usize) -> Self {
        Self {
            handlers: BTreeMap::new(),
            hot_cache: Vec::new(),
            hot_cache_capacity,
        }
    }

    /// Register handler
    pub fn register(&mut self, syscall_nr: u32, handler_type: HandlerType) {
        self.handlers.insert(syscall_nr, HandlerEntry::new(syscall_nr, handler_type));
    }

    /// Lookup handler
    pub fn lookup(&self, syscall_nr: u32) -> Option<&HandlerEntry> {
        self.handlers.get(&syscall_nr)
    }

    /// Dispatch decision
    pub fn decide(&self, syscall_nr: u32) -> DispatchDecision {
        match self.handlers.get(&syscall_nr) {
            None => DispatchDecision::Deny,
            Some(h) => match h.state {
                HandlerState::Disabled => DispatchDecision::Deny,
                HandlerState::Overloaded => DispatchDecision::Defer,
                _ => {
                    if let Some(target) = h.redirect_to {
                        DispatchDecision::Redirect
                    } else if h.handler_type == HandlerType::Batched {
                        DispatchDecision::QueueBatch
                    } else {
                        DispatchDecision::Immediate
                    }
                }
            }
        }
    }

    /// Record invocation
    pub fn record_invocation(&mut self, syscall_nr: u32, latency_ns: u64, success: bool) {
        if let Some(handler) = self.handlers.get_mut(&syscall_nr) {
            handler.record(latency_ns, success);
            // Check hot status
            if handler.invocations >= HOT_PATH_THRESHOLD && !handler.is_hot {
                handler.is_hot = true;
                handler.handler_type = HandlerType::FastInline;
            }
        }
    }

    /// Rebuild hot cache
    pub fn rebuild_hot_cache(&mut self) {
        let mut entries: Vec<(u32, u64)> = self.handlers.iter()
            .filter(|(_, h)| h.is_hot)
            .map(|(&nr, h)| (nr, h.invocations))
            .collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1));
        entries.truncate(self.hot_cache_capacity);
        self.hot_cache = entries.into_iter().map(|(nr, _)| nr).collect();
    }

    /// Is hot path
    pub fn is_hot(&self, syscall_nr: u32) -> bool {
        self.hot_cache.contains(&syscall_nr)
    }

    /// Handler count
    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }

    /// Hot count
    pub fn hot_count(&self) -> usize {
        self.hot_cache.len()
    }
}

// ============================================================================
// SPECULATIVE PRE-SELECTION
// ============================================================================

/// Prediction for next syscall
#[derive(Debug, Clone)]
pub struct DispatchPrediction {
    /// Predicted syscall number
    pub predicted_nr: u32,
    /// Confidence
    pub confidence: f64,
    /// Pre-selected handler type
    pub handler_type: HandlerType,
}

/// Markov predictor for syscall sequences
#[derive(Debug)]
pub struct SyscallPredictor {
    /// Transition counts: (from, to) -> count
    transitions: BTreeMap<u64, u64>,
    /// Last syscall per process
    last_syscall: BTreeMap<u64, u32>,
}

impl SyscallPredictor {
    pub fn new() -> Self {
        Self {
            transitions: BTreeMap::new(),
            last_syscall: BTreeMap::new(),
        }
    }

    /// Transition key (FNV-1a)
    fn transition_key(from: u32, to: u32) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        hash ^= from as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= to as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash
    }

    /// Record transition
    pub fn record(&mut self, pid: u64, syscall_nr: u32) {
        if let Some(&last) = self.last_syscall.get(&pid) {
            let key = Self::transition_key(last, syscall_nr);
            *self.transitions.entry(key).or_insert(0) += 1;
        }
        self.last_syscall.insert(pid, syscall_nr);
    }

    /// Predict next syscall for process
    pub fn predict(&self, pid: u64, table: &DispatchTable) -> Option<DispatchPrediction> {
        let &last = self.last_syscall.get(&pid)?;
        let mut best_nr = 0u32;
        let mut best_count = 0u64;
        let mut total = 0u64;

        // Check all registered handlers as candidates
        for (&nr, _) in &table.handlers {
            let key = Self::transition_key(last, nr);
            let count = self.transitions.get(&key).copied().unwrap_or(0);
            total += count;
            if count > best_count {
                best_count = count;
                best_nr = nr;
            }
        }

        if best_count == 0 || total == 0 {
            return None;
        }

        let confidence = best_count as f64 / total as f64;
        let handler_type = table.lookup(best_nr)
            .map(|h| h.handler_type)
            .unwrap_or(HandlerType::Standard);

        Some(DispatchPrediction {
            predicted_nr: best_nr,
            confidence,
            handler_type,
        })
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Dispatch stats
#[derive(Debug, Clone, Default)]
pub struct BridgeDispatchStats {
    /// Total dispatches
    pub total_dispatches: u64,
    /// Hot path dispatches
    pub hot_dispatches: u64,
    /// Denied dispatches
    pub denied: u64,
    /// Deferred dispatches
    pub deferred: u64,
    /// Prediction accuracy
    pub prediction_hits: u64,
    /// Prediction total
    pub prediction_total: u64,
}

/// Bridge dispatch optimizer
pub struct BridgeDispatchOptimizer {
    /// Dispatch table
    pub table: DispatchTable,
    /// Predictor
    predictor: SyscallPredictor,
    /// Stats
    stats: BridgeDispatchStats,
}

impl BridgeDispatchOptimizer {
    pub fn new() -> Self {
        Self {
            table: DispatchTable::new(64),
            predictor: SyscallPredictor::new(),
            stats: BridgeDispatchStats::default(),
        }
    }

    /// Register handler
    pub fn register_handler(&mut self, syscall_nr: u32, handler_type: HandlerType) {
        self.table.register(syscall_nr, handler_type);
    }

    /// Dispatch syscall
    pub fn dispatch(&mut self, pid: u64, syscall_nr: u32) -> DispatchDecision {
        self.stats.total_dispatches += 1;

        // Check prediction accuracy
        self.stats.prediction_total += 1;
        // Would check prediction here

        let decision = self.table.decide(syscall_nr);
        match decision {
            DispatchDecision::Deny => self.stats.denied += 1,
            DispatchDecision::Defer => self.stats.deferred += 1,
            DispatchDecision::Immediate => {
                if self.table.is_hot(syscall_nr) {
                    self.stats.hot_dispatches += 1;
                }
            }
            _ => {}
        }

        // Record for prediction
        self.predictor.record(pid, syscall_nr);

        decision
    }

    /// Record completion
    pub fn record_completion(&mut self, syscall_nr: u32, latency_ns: u64, success: bool) {
        self.table.record_invocation(syscall_nr, latency_ns, success);
    }

    /// Predict next syscall
    pub fn predict(&self, pid: u64) -> Option<DispatchPrediction> {
        self.predictor.predict(pid, &self.table)
    }

    /// Optimize (rebuild caches)
    pub fn optimize(&mut self) {
        self.table.rebuild_hot_cache();
    }

    /// Stats
    pub fn stats(&self) -> &BridgeDispatchStats {
        &self.stats
    }
}
