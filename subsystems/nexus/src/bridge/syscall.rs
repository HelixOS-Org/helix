//! # Intelligent Syscall Interceptor & Router
//!
//! Core module that intercepts syscalls, enriches them with context,
//! and routes them through the optimization pipeline.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// SYSCALL TYPES
// ============================================================================

/// Unique identifier for a syscall invocation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SyscallId(pub u64);

/// Classification of syscall types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyscallType {
    // I/O syscalls
    Read,
    Write,
    Open,
    Close,
    Seek,
    Stat,
    Readdir,
    Fsync,
    Ioctl,

    // Memory syscalls
    Mmap,
    Munmap,
    Mprotect,
    Brk,

    // Process syscalls
    Fork,
    Exec,
    Exit,
    Wait,
    Kill,

    // Network syscalls
    Socket,
    Bind,
    Listen,
    Accept,
    Connect,
    Send,
    Recv,

    // Synchronization syscalls
    Futex,
    SemWait,
    SemPost,

    // Time syscalls
    ClockGettime,
    Nanosleep,

    // Other
    Unknown(u64),
}

impl SyscallType {
    /// Whether this is an I/O syscall
    pub fn is_io(&self) -> bool {
        matches!(
            self,
            Self::Read
                | Self::Write
                | Self::Open
                | Self::Close
                | Self::Seek
                | Self::Stat
                | Self::Readdir
                | Self::Fsync
                | Self::Ioctl
        )
    }

    /// Whether this is a memory syscall
    pub fn is_memory(&self) -> bool {
        matches!(self, Self::Mmap | Self::Munmap | Self::Mprotect | Self::Brk)
    }

    /// Whether this is a process syscall
    pub fn is_process(&self) -> bool {
        matches!(
            self,
            Self::Fork | Self::Exec | Self::Exit | Self::Wait | Self::Kill
        )
    }

    /// Whether this is a network syscall
    pub fn is_network(&self) -> bool {
        matches!(
            self,
            Self::Socket
                | Self::Bind
                | Self::Listen
                | Self::Accept
                | Self::Connect
                | Self::Send
                | Self::Recv
        )
    }

    /// Whether this syscall can be batched with others of the same type
    pub fn is_batchable(&self) -> bool {
        matches!(
            self,
            Self::Read | Self::Write | Self::Send | Self::Recv | Self::Stat | Self::Readdir
        )
    }

    /// Whether this syscall can be predicted from patterns
    pub fn is_predictable(&self) -> bool {
        matches!(
            self,
            Self::Read
                | Self::Write
                | Self::Stat
                | Self::Send
                | Self::Recv
                | Self::ClockGettime
                | Self::Futex
        )
    }

    /// Typical latency category (0=fast, 1=medium, 2=slow)
    pub fn latency_class(&self) -> u8 {
        match self {
            Self::ClockGettime | Self::Brk => 0,
            Self::Read | Self::Write | Self::Mmap | Self::Munmap | Self::Futex => 1,
            Self::Open | Self::Fsync | Self::Fork | Self::Exec | Self::Connect | Self::Accept => 2,
            _ => 1,
        }
    }

    /// Convert from raw syscall number
    pub fn from_number(nr: u64) -> Self {
        match nr {
            0 => Self::Read,
            1 => Self::Write,
            2 => Self::Open,
            3 => Self::Close,
            8 => Self::Seek,
            4 => Self::Stat,
            9 => Self::Mmap,
            11 => Self::Munmap,
            10 => Self::Mprotect,
            12 => Self::Brk,
            56 => Self::Fork,
            59 => Self::Exec,
            60 => Self::Exit,
            61 => Self::Wait,
            62 => Self::Kill,
            41 => Self::Socket,
            49 => Self::Bind,
            50 => Self::Listen,
            43 => Self::Accept,
            42 => Self::Connect,
            44 => Self::Send,
            45 => Self::Recv,
            202 => Self::Futex,
            228 => Self::ClockGettime,
            35 => Self::Nanosleep,
            16 => Self::Ioctl,
            74 => Self::Fsync,
            78 => Self::Readdir,
            other => Self::Unknown(other),
        }
    }
}

// ============================================================================
// SYSCALL CONTEXT
// ============================================================================

/// Rich context attached to each syscall invocation
#[derive(Debug, Clone)]
pub struct SyscallContext {
    /// Unique ID for this invocation
    pub id: SyscallId,
    /// The syscall type
    pub syscall_type: SyscallType,
    /// Process ID of the caller
    pub pid: u64,
    /// Thread ID of the caller
    pub tid: u64,
    /// Timestamp (ticks since boot)
    pub timestamp: u64,
    /// Arguments (up to 6 for Linux-style)
    pub args: [u64; 6],
    /// Estimated data size (bytes) for I/O ops
    pub data_size: usize,
    /// CPU core the caller is running on
    pub cpu_id: u32,
    /// Whether this was predicted by the prediction engine
    pub predicted: bool,
    /// Optimization hints attached by the profiler
    pub hints: Vec<OptimizationHint>,
}

impl SyscallContext {
    /// Create a new syscall context
    pub fn new(id: SyscallId, syscall_type: SyscallType, pid: u64, tid: u64) -> Self {
        Self {
            id,
            syscall_type,
            pid,
            tid,
            timestamp: 0,
            args: [0; 6],
            data_size: 0,
            cpu_id: 0,
            predicted: false,
            hints: Vec::new(),
        }
    }

    /// Attach a timestamp
    pub fn with_timestamp(mut self, ts: u64) -> Self {
        self.timestamp = ts;
        self
    }

    /// Attach arguments
    pub fn with_args(mut self, args: [u64; 6]) -> Self {
        self.args = args;
        self
    }

    /// Attach data size
    pub fn with_data_size(mut self, size: usize) -> Self {
        self.data_size = size;
        self
    }

    /// Add an optimization hint
    pub fn add_hint(&mut self, hint: OptimizationHint) {
        self.hints.push(hint);
    }
}

// ============================================================================
// OPTIMIZATION HINTS
// ============================================================================

/// Hints that guide syscall optimization
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizationHint {
    /// Prefetch data — the app will likely read more
    Prefetch { ahead_bytes: usize },
    /// Use large pages for this mapping
    UseLargePages,
    /// This access pattern is sequential
    Sequential,
    /// This access pattern is random
    Random,
    /// Batch this with similar pending requests
    Batchable,
    /// This is latency-sensitive — prioritize
    LatencySensitive,
    /// This is throughput-oriented — batch aggressively
    ThroughputOriented,
    /// The app will close this FD soon
    ShortLived,
    /// The app will keep this FD open for a long time
    LongLived,
    /// Zero-copy path is available
    ZeroCopy,
    /// Async completion is preferred
    PreferAsync,
}

// ============================================================================
// SYSCALL RESULT
// ============================================================================

/// Result of a syscall execution
#[derive(Debug, Clone)]
pub struct SyscallResult {
    /// The syscall context
    pub id: SyscallId,
    /// Return value
    pub return_value: i64,
    /// Actual execution time (nanoseconds)
    pub latency_ns: u64,
    /// Whether optimization was applied
    pub optimized: bool,
    /// Optimization type applied, if any
    pub optimization_applied: Option<String>,
    /// Whether this was served from prediction cache
    pub served_from_prediction: bool,
}

impl SyscallResult {
    /// Create a success result
    pub fn success(id: SyscallId, return_value: i64, latency_ns: u64) -> Self {
        Self {
            id,
            return_value,
            latency_ns,
            optimized: false,
            optimization_applied: None,
            served_from_prediction: false,
        }
    }

    /// Create an error result
    pub fn error(id: SyscallId, errno: i64, latency_ns: u64) -> Self {
        Self {
            id,
            return_value: -errno,
            latency_ns,
            optimized: false,
            optimization_applied: None,
            served_from_prediction: false,
        }
    }

    /// Mark as optimized
    pub fn with_optimization(mut self, desc: &str) -> Self {
        self.optimized = true;
        self.optimization_applied = Some(String::from(desc));
        self
    }
}

// ============================================================================
// SYSCALL METRICS
// ============================================================================

/// Aggregated metrics for syscall performance
#[derive(Debug, Clone)]
pub struct SyscallMetrics {
    /// Total syscalls intercepted
    pub total_intercepted: u64,
    /// Total syscalls optimized
    pub total_optimized: u64,
    /// Total syscalls served from prediction
    pub total_predicted: u64,
    /// Total syscalls batched
    pub total_batched: u64,
    /// Average latency (ns)
    pub avg_latency_ns: u64,
    /// p99 latency (ns)
    pub p99_latency_ns: u64,
    /// Latency reduction percentage vs baseline
    pub latency_reduction_pct: f64,
    /// Per-type metrics
    pub per_type: BTreeMap<u64, TypeMetrics>,
}

/// Metrics for a specific syscall type
#[derive(Debug, Clone, Default)]
pub struct TypeMetrics {
    pub count: u64,
    pub total_latency_ns: u64,
    pub optimized_count: u64,
    pub predicted_count: u64,
    pub batched_count: u64,
}

impl SyscallMetrics {
    pub fn new() -> Self {
        Self {
            total_intercepted: 0,
            total_optimized: 0,
            total_predicted: 0,
            total_batched: 0,
            avg_latency_ns: 0,
            p99_latency_ns: 0,
            latency_reduction_pct: 0.0,
            per_type: BTreeMap::new(),
        }
    }

    /// Record a completed syscall
    pub fn record(&mut self, result: &SyscallResult, syscall_type: SyscallType) {
        self.total_intercepted += 1;
        if result.optimized {
            self.total_optimized += 1;
        }
        if result.served_from_prediction {
            self.total_predicted += 1;
        }

        // Update running average
        let prev_total = self.avg_latency_ns * (self.total_intercepted - 1);
        self.avg_latency_ns = (prev_total + result.latency_ns) / self.total_intercepted;

        // Per-type tracking
        let type_key = match syscall_type {
            SyscallType::Unknown(n) => n,
            _ => syscall_type.from_number_reverse(),
        };
        let entry = self.per_type.entry(type_key).or_default();
        entry.count += 1;
        entry.total_latency_ns += result.latency_ns;
        if result.optimized {
            entry.optimized_count += 1;
        }
        if result.served_from_prediction {
            entry.predicted_count += 1;
        }
    }

    /// Optimization hit rate
    pub fn optimization_rate(&self) -> f64 {
        if self.total_intercepted == 0 {
            return 0.0;
        }
        self.total_optimized as f64 / self.total_intercepted as f64
    }

    /// Prediction hit rate
    pub fn prediction_rate(&self) -> f64 {
        if self.total_intercepted == 0 {
            return 0.0;
        }
        self.total_predicted as f64 / self.total_intercepted as f64
    }
}

impl SyscallType {
    fn from_number_reverse(&self) -> u64 {
        match self {
            Self::Read => 0,
            Self::Write => 1,
            Self::Open => 2,
            Self::Close => 3,
            Self::Seek => 8,
            Self::Stat => 4,
            Self::Mmap => 9,
            Self::Munmap => 11,
            Self::Mprotect => 10,
            Self::Brk => 12,
            Self::Fork => 56,
            Self::Exec => 59,
            Self::Exit => 60,
            Self::Wait => 61,
            Self::Kill => 62,
            Self::Socket => 41,
            Self::Bind => 49,
            Self::Listen => 50,
            Self::Accept => 43,
            Self::Connect => 42,
            Self::Send => 44,
            Self::Recv => 45,
            Self::Futex => 202,
            Self::ClockGettime => 228,
            Self::Nanosleep => 35,
            Self::Ioctl => 16,
            Self::Fsync => 74,
            Self::Readdir => 78,
            Self::SemWait => 230,
            Self::SemPost => 231,
            Self::Unknown(n) => *n,
        }
    }
}

// ============================================================================
// SYSCALL INTERCEPTOR
// ============================================================================

/// The intelligent syscall interceptor — sits between userland and the kernel
/// execution path, enriching each syscall with context, predictions, and
/// optimization decisions.
pub struct SyscallInterceptor {
    /// Rolling ID counter
    next_id: u64,
    /// Whether interception is active
    active: bool,
    /// Metrics collector
    metrics: SyscallMetrics,
    /// Per-process recent syscall history (pid -> recent types)
    history: BTreeMap<u64, Vec<SyscallType>>,
    /// History window size
    history_window: usize,
}

impl SyscallInterceptor {
    /// Create a new interceptor with the given history window size
    pub fn new(history_window: usize) -> Self {
        Self {
            next_id: 1,
            active: true,
            metrics: SyscallMetrics::new(),
            history: BTreeMap::new(),
            history_window,
        }
    }

    /// Enable the interceptor
    pub fn enable(&mut self) {
        self.active = true;
    }

    /// Disable the interceptor (passthrough mode)
    pub fn disable(&mut self) {
        self.active = false;
    }

    /// Intercept a raw syscall and produce a rich context
    pub fn intercept(
        &mut self,
        nr: u64,
        args: [u64; 6],
        pid: u64,
        tid: u64,
        cpu_id: u32,
        timestamp: u64,
    ) -> SyscallContext {
        let id = SyscallId(self.next_id);
        self.next_id += 1;

        let syscall_type = SyscallType::from_number(nr);

        // Track history
        let hist = self
            .history
            .entry(pid)
            .or_insert_with(|| Vec::with_capacity(self.history_window));
        if hist.len() >= self.history_window {
            hist.remove(0);
        }
        hist.push(syscall_type);

        // Build context with data size estimation
        let data_size = Self::estimate_data_size(syscall_type, &args);

        let mut ctx = SyscallContext::new(id, syscall_type, pid, tid)
            .with_timestamp(timestamp)
            .with_args(args)
            .with_data_size(data_size);
        ctx.cpu_id = cpu_id;

        // Auto-detect optimization hints from patterns
        if self.active {
            self.attach_auto_hints(&mut ctx);
        }

        ctx
    }

    /// Record a completed syscall result
    pub fn complete(&mut self, result: &SyscallResult, syscall_type: SyscallType) {
        self.metrics.record(result, syscall_type);
    }

    /// Get current metrics
    pub fn metrics(&self) -> &SyscallMetrics {
        &self.metrics
    }

    /// Get recent history for a process
    pub fn process_history(&self, pid: u64) -> Option<&[SyscallType]> {
        self.history.get(&pid).map(|v| v.as_slice())
    }

    /// Estimate data size from syscall arguments
    fn estimate_data_size(syscall_type: SyscallType, args: &[u64; 6]) -> usize {
        match syscall_type {
            SyscallType::Read | SyscallType::Write => args[2] as usize,
            SyscallType::Send | SyscallType::Recv => args[2] as usize,
            SyscallType::Mmap => args[1] as usize,
            _ => 0,
        }
    }

    /// Automatically attach optimization hints based on observed patterns
    fn attach_auto_hints(&self, ctx: &mut SyscallContext) {
        if let Some(history) = self.history.get(&ctx.pid) {
            // Detect sequential I/O pattern
            if history.len() >= 3 {
                let last_three: Vec<_> = history.iter().rev().take(3).collect();
                let all_reads = last_three.iter().all(|t| **t == SyscallType::Read);
                let all_writes = last_three.iter().all(|t| **t == SyscallType::Write);

                if all_reads && ctx.syscall_type == SyscallType::Read {
                    ctx.add_hint(OptimizationHint::Sequential);
                    ctx.add_hint(OptimizationHint::Prefetch {
                        ahead_bytes: ctx.data_size * 4,
                    });
                }

                if all_writes && ctx.syscall_type == SyscallType::Write {
                    ctx.add_hint(OptimizationHint::Sequential);
                    ctx.add_hint(OptimizationHint::ThroughputOriented);
                }
            }

            // Detect batchable pattern
            if ctx.syscall_type.is_batchable() {
                let recent_same = history
                    .iter()
                    .rev()
                    .take(5)
                    .filter(|t| **t == ctx.syscall_type)
                    .count();
                if recent_same >= 3 {
                    ctx.add_hint(OptimizationHint::Batchable);
                }
            }
        }
    }
}

// ============================================================================
// SYSCALL ROUTER
// ============================================================================

/// Routes syscalls through the optimization pipeline based on context and hints
pub struct SyscallRouter {
    /// Whether routing is enabled
    enabled: bool,
    /// Batch threshold — minimum pending to trigger batching
    batch_threshold: usize,
    /// Prediction enabled
    prediction_enabled: bool,
    /// Async I/O enabled
    async_enabled: bool,
}

impl SyscallRouter {
    pub fn new() -> Self {
        Self {
            enabled: true,
            batch_threshold: 3,
            prediction_enabled: true,
            async_enabled: true,
        }
    }

    /// Determine the routing decision for a syscall
    pub fn route(&self, ctx: &SyscallContext) -> RoutingDecision {
        if !self.enabled {
            return RoutingDecision::Direct;
        }

        // Check if this was already predicted and cached
        if ctx.predicted {
            return RoutingDecision::ServeFromCache;
        }

        // Check for async preference
        if ctx.hints.contains(&OptimizationHint::PreferAsync) && self.async_enabled {
            return RoutingDecision::AsyncDispatch;
        }

        // Check for batching opportunity
        if ctx.hints.contains(&OptimizationHint::Batchable) {
            return RoutingDecision::BatchQueue;
        }

        // Check for prefetch opportunity
        for hint in &ctx.hints {
            if let OptimizationHint::Prefetch { ahead_bytes } = hint {
                return RoutingDecision::OptimizedWithPrefetch {
                    prefetch_bytes: *ahead_bytes,
                };
            }
        }

        // Latency-sensitive → fast path
        if ctx.hints.contains(&OptimizationHint::LatencySensitive) {
            return RoutingDecision::FastPath;
        }

        // Default: optimized direct
        RoutingDecision::Direct
    }
}

/// How to route a syscall through the pipeline
#[derive(Debug, Clone, PartialEq)]
pub enum RoutingDecision {
    /// Execute directly, no optimization
    Direct,
    /// Fast path — skip non-essential processing
    FastPath,
    /// Queue for batching with similar requests
    BatchQueue,
    /// Dispatch asynchronously
    AsyncDispatch,
    /// Execute with prefetch
    OptimizedWithPrefetch { prefetch_bytes: usize },
    /// Serve from prediction cache
    ServeFromCache,
}
