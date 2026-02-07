//! # Syscall Processing Pipeline
//!
//! Defines the multi-stage pipeline through which every syscall flows.
//! Each stage can inspect, modify, delay, batch, or short-circuit a syscall.
//!
//! ## Pipeline Stages
//!
//! 1. **Intercept** — Capture and classify the syscall
//! 2. **Analyze** — Intent analysis, pattern matching
//! 3. **Transform** — Rewrite, merge, or eliminate
//! 4. **Cache Check** — Look up cached results
//! 5. **Security** — Permission and rate-limit checks
//! 6. **Schedule** — Determine execution priority/timing
//! 7. **Execute** — Actual kernel execution
//! 8. **Post-Process** — Cache result, update metrics, feedback

use alloc::string::String;
use alloc::vec::Vec;

use super::syscall::SyscallType;

// ============================================================================
// PIPELINE STAGES
// ============================================================================

/// Stage in the syscall pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PipelineStage {
    /// Initial interception and classification
    Intercept   = 0,
    /// Intent and pattern analysis
    Analyze     = 1,
    /// Transformation and rewriting
    Transform   = 2,
    /// Cache lookup
    CacheCheck  = 3,
    /// Security validation
    Security    = 4,
    /// Scheduling and prioritization
    Schedule    = 5,
    /// Kernel execution
    Execute     = 6,
    /// Post-processing and feedback
    PostProcess = 7,
}

impl PipelineStage {
    /// Get all stages in order
    pub fn all() -> [Self; 8] {
        [
            Self::Intercept,
            Self::Analyze,
            Self::Transform,
            Self::CacheCheck,
            Self::Security,
            Self::Schedule,
            Self::Execute,
            Self::PostProcess,
        ]
    }

    /// Name of this stage
    pub fn name(&self) -> &'static str {
        match self {
            Self::Intercept => "intercept",
            Self::Analyze => "analyze",
            Self::Transform => "transform",
            Self::CacheCheck => "cache_check",
            Self::Security => "security",
            Self::Schedule => "schedule",
            Self::Execute => "execute",
            Self::PostProcess => "post_process",
        }
    }
}

// ============================================================================
// PIPELINE CONTEXT
// ============================================================================

/// Decision made by a pipeline stage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageDecision {
    /// Continue to next stage
    Continue,
    /// Skip to a specific stage
    SkipTo(PipelineStage),
    /// Short-circuit with a cached/computed result
    ShortCircuit,
    /// Abort the syscall (return error)
    Abort,
    /// Defer execution (will be batched or async'd)
    Defer,
    /// Retry the current stage
    Retry,
}

/// Execution priority assigned by the schedule stage
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExecutionPriority {
    /// Immediate execution (latency-critical)
    Immediate  = 0,
    /// High priority
    High       = 1,
    /// Normal priority
    Normal     = 2,
    /// Low priority (can be batched)
    Low        = 3,
    /// Background (execute when idle)
    Background = 4,
    /// Deferred (will be batched)
    Deferred   = 5,
}

/// Context that flows through the pipeline
#[derive(Debug, Clone)]
pub struct PipelineContext {
    /// Unique request ID
    pub request_id: u64,
    /// Process ID
    pub pid: u64,
    /// Thread ID
    pub tid: u64,
    /// Original syscall type
    pub original_type: SyscallType,
    /// Current syscall type (may be transformed)
    pub current_type: SyscallType,
    /// Syscall arguments
    pub args: [u64; 6],
    /// Current pipeline stage
    pub current_stage: PipelineStage,
    /// Entry timestamp (ns)
    pub entry_time_ns: u64,
    /// Per-stage timestamps (ns)
    pub stage_times: [u64; 8],
    /// Execution priority
    pub priority: ExecutionPriority,
    /// Whether this syscall was transformed
    pub was_transformed: bool,
    /// Whether a cache hit occurred
    pub cache_hit: bool,
    /// Result (set after execution or cache hit)
    pub result: Option<i64>,
    /// Error code (if aborted)
    pub error: Option<i32>,
    /// Annotations from stages
    pub annotations: Vec<PipelineAnnotation>,
    /// Whether execution was deferred
    pub deferred: bool,
    /// Security clearance level
    pub security_level: u8,
}

impl PipelineContext {
    pub fn new(
        request_id: u64,
        pid: u64,
        tid: u64,
        syscall_type: SyscallType,
        args: [u64; 6],
    ) -> Self {
        Self {
            request_id,
            pid,
            tid,
            original_type: syscall_type,
            current_type: syscall_type,
            args,
            current_stage: PipelineStage::Intercept,
            entry_time_ns: 0,
            stage_times: [0; 8],
            priority: ExecutionPriority::Normal,
            was_transformed: false,
            cache_hit: false,
            result: None,
            error: None,
            annotations: Vec::new(),
            deferred: false,
            security_level: 0,
        }
    }

    /// Record entering a stage
    pub fn enter_stage(&mut self, stage: PipelineStage, timestamp_ns: u64) {
        self.current_stage = stage;
        self.stage_times[stage as usize] = timestamp_ns;
    }

    /// Total pipeline latency so far
    pub fn elapsed_ns(&self, current_time_ns: u64) -> u64 {
        current_time_ns.saturating_sub(self.entry_time_ns)
    }

    /// Latency of a specific stage
    pub fn stage_latency_ns(&self, stage: PipelineStage) -> u64 {
        let idx = stage as usize;
        if idx + 1 < 8 && self.stage_times[idx + 1] > 0 {
            self.stage_times[idx + 1].saturating_sub(self.stage_times[idx])
        } else {
            0
        }
    }

    /// Add an annotation
    pub fn annotate(&mut self, stage: PipelineStage, message: String) {
        self.annotations.push(PipelineAnnotation { stage, message });
    }
}

/// An annotation attached by a pipeline stage
#[derive(Debug, Clone)]
pub struct PipelineAnnotation {
    pub stage: PipelineStage,
    pub message: String,
}

// ============================================================================
// PIPELINE STATISTICS
// ============================================================================

/// Per-stage statistics
#[derive(Debug, Clone, Default)]
pub struct StageStats {
    /// Total invocations
    pub invocations: u64,
    /// Total latency (ns)
    pub total_latency_ns: u64,
    /// Max latency (ns)
    pub max_latency_ns: u64,
    /// Decisions: continue
    pub continues: u64,
    /// Decisions: short-circuit
    pub short_circuits: u64,
    /// Decisions: abort
    pub aborts: u64,
    /// Decisions: defer
    pub defers: u64,
}

impl StageStats {
    pub fn record(&mut self, latency_ns: u64, decision: StageDecision) {
        self.invocations += 1;
        self.total_latency_ns += latency_ns;
        if latency_ns > self.max_latency_ns {
            self.max_latency_ns = latency_ns;
        }
        match decision {
            StageDecision::Continue | StageDecision::SkipTo(_) | StageDecision::Retry => {
                self.continues += 1;
            },
            StageDecision::ShortCircuit => self.short_circuits += 1,
            StageDecision::Abort => self.aborts += 1,
            StageDecision::Defer => self.defers += 1,
        }
    }

    pub fn avg_latency_ns(&self) -> u64 {
        if self.invocations == 0 {
            0
        } else {
            self.total_latency_ns / self.invocations
        }
    }
}

/// Global pipeline statistics
#[derive(Debug, Clone)]
pub struct PipelineStats {
    /// Per-stage stats
    pub stages: [StageStats; 8],
    /// Total syscalls processed
    pub total_processed: u64,
    /// Total cache hits
    pub total_cache_hits: u64,
    /// Total transformations
    pub total_transforms: u64,
    /// Total security blocks
    pub total_blocked: u64,
    /// Total deferred
    pub total_deferred: u64,
    /// Average end-to-end latency (ns)
    pub avg_e2e_latency_ns: u64,
    /// Running sum for e2e average
    e2e_sum: u64,
}

impl PipelineStats {
    pub fn new() -> Self {
        Self {
            stages: Default::default(),
            total_processed: 0,
            total_cache_hits: 0,
            total_transforms: 0,
            total_blocked: 0,
            total_deferred: 0,
            avg_e2e_latency_ns: 0,
            e2e_sum: 0,
        }
    }

    /// Record a completed pipeline execution
    pub fn record_completion(&mut self, ctx: &PipelineContext, end_time_ns: u64) {
        self.total_processed += 1;
        let e2e = end_time_ns.saturating_sub(ctx.entry_time_ns);
        self.e2e_sum += e2e;
        self.avg_e2e_latency_ns = self.e2e_sum / self.total_processed;

        if ctx.cache_hit {
            self.total_cache_hits += 1;
        }
        if ctx.was_transformed {
            self.total_transforms += 1;
        }
        if ctx.deferred {
            self.total_deferred += 1;
        }
    }

    /// Cache hit rate
    pub fn cache_hit_rate(&self) -> f64 {
        if self.total_processed == 0 {
            0.0
        } else {
            self.total_cache_hits as f64 / self.total_processed as f64
        }
    }

    /// Transform rate
    pub fn transform_rate(&self) -> f64 {
        if self.total_processed == 0 {
            0.0
        } else {
            self.total_transforms as f64 / self.total_processed as f64
        }
    }
}

// ============================================================================
// PIPELINE CONFIGURATION
// ============================================================================

/// Pipeline configuration
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Maximum pipeline latency before fast-path (ns)
    pub max_latency_ns: u64,
    /// Enable intent analysis stage
    pub enable_analysis: bool,
    /// Enable transformation stage
    pub enable_transform: bool,
    /// Enable caching stage
    pub enable_cache: bool,
    /// Enable security stage
    pub enable_security: bool,
    /// Fast-path syscall types (skip analysis/transform)
    pub fast_path_types: Vec<SyscallType>,
    /// Maximum concurrent pipeline contexts
    pub max_concurrent: usize,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            max_latency_ns: 10_000_000, // 10ms max
            enable_analysis: true,
            enable_transform: true,
            enable_cache: true,
            enable_security: true,
            fast_path_types: vec![SyscallType::Brk, SyscallType::Close],
            max_concurrent: 4096,
        }
    }
}

/// The syscall processing pipeline
pub struct SyscallPipeline {
    /// Configuration
    config: PipelineConfig,
    /// Statistics
    stats: PipelineStats,
    /// Next request ID
    next_request_id: u64,
    /// Active contexts
    active_count: u64,
}

impl SyscallPipeline {
    pub fn new(config: PipelineConfig) -> Self {
        Self {
            config,
            stats: PipelineStats::new(),
            next_request_id: 1,
            active_count: 0,
        }
    }

    /// Create a new pipeline context for a syscall
    pub fn create_context(
        &mut self,
        pid: u64,
        tid: u64,
        syscall_type: SyscallType,
        args: [u64; 6],
        timestamp_ns: u64,
    ) -> PipelineContext {
        let id = self.next_request_id;
        self.next_request_id += 1;
        self.active_count += 1;

        let mut ctx = PipelineContext::new(id, pid, tid, syscall_type, args);
        ctx.entry_time_ns = timestamp_ns;
        ctx.enter_stage(PipelineStage::Intercept, timestamp_ns);

        // Check fast-path
        if self.config.fast_path_types.contains(&syscall_type) {
            ctx.priority = ExecutionPriority::Immediate;
        }

        ctx
    }

    /// Determine which stages to skip based on config
    pub fn should_skip(&self, stage: PipelineStage) -> bool {
        match stage {
            PipelineStage::Analyze => !self.config.enable_analysis,
            PipelineStage::Transform => !self.config.enable_transform,
            PipelineStage::CacheCheck => !self.config.enable_cache,
            PipelineStage::Security => !self.config.enable_security,
            _ => false,
        }
    }

    /// Record pipeline completion
    pub fn complete(&mut self, ctx: &PipelineContext, end_time_ns: u64) {
        self.stats.record_completion(ctx, end_time_ns);
        self.active_count = self.active_count.saturating_sub(1);
    }

    /// Record a stage result
    pub fn record_stage(&mut self, stage: PipelineStage, latency_ns: u64, decision: StageDecision) {
        self.stats.stages[stage as usize].record(latency_ns, decision);
    }

    /// Get pipeline statistics
    pub fn stats(&self) -> &PipelineStats {
        &self.stats
    }

    /// Get active context count
    pub fn active_count(&self) -> u64 {
        self.active_count
    }

    /// Get configuration
    pub fn config(&self) -> &PipelineConfig {
        &self.config
    }
}
