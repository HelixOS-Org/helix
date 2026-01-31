//! # Initialization Executor
//!
//! This module provides the main execution engine for subsystem initialization.
//! It supports multiple execution modes: sequential, parallel, lazy, and
//! conditional initialization.
//!
//! ## Executor Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                        INIT EXECUTOR                                         │
//! │                                                                              │
//! │  ┌────────────────────────────────────────────────────────────────────────┐ │
//! │  │                     EXECUTION MODES                                     │ │
//! │  │                                                                         │ │
//! │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │ │
//! │  │  │ Sequential  │  │  Parallel   │  │    Lazy     │  │ Conditional │   │ │
//! │  │  │             │  │             │  │             │  │             │   │ │
//! │  │  │  A ──▶ B    │  │  A ━━▶ C    │  │  Init on    │  │  If config  │   │ │
//! │  │  │    ──▶ C    │  │  B ━━▶ D    │  │  first use  │  │  enabled    │   │ │
//! │  │  │    ──▶ D    │  │             │  │             │  │             │   │ │
//! │  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘   │ │
//! │  │                                                                         │ │
//! │  └────────────────────────────────────────────────────────────────────────┘ │
//! │                                                                              │
//! │  ┌────────────────────────────────────────────────────────────────────────┐ │
//! │  │                     EXECUTION FLOW                                      │ │
//! │  │                                                                         │ │
//! │  │   ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐        │ │
//! │  │   │ Validate │───▶│  Order   │───▶│ Execute  │───▶│ Verify   │        │ │
//! │  │   │  Deps    │    │  (Topo)  │    │  (Mode)  │    │ Complete │        │ │
//! │  │   └──────────┘    └──────────┘    └──────────┘    └──────────┘        │ │
//! │  │        │               │               │               │              │ │
//! │  │        ▼               ▼               ▼               ▼              │ │
//! │  │   ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐        │ │
//! │  │   │  Cycle   │    │ Priority │    │ Timeout  │    │ Barrier  │        │ │
//! │  │   │  Check   │    │  Sort    │    │ Monitor  │    │  Wait    │        │ │
//! │  │   └──────────┘    └──────────┘    └──────────┘    └──────────┘        │ │
//! │  │                                                                         │ │
//! │  └────────────────────────────────────────────────────────────────────────┘ │
//! │                                                                              │
//! │  ┌────────────────────────────────────────────────────────────────────────┐ │
//! │  │                     ERROR HANDLING                                      │ │
//! │  │                                                                         │ │
//! │  │   On Error:                                                             │ │
//! │  │     1. Log error details                                                │ │
//! │  │     2. Check if essential                                               │ │
//! │  │     3. Execute rollback chain                                           │ │
//! │  │     4. Mark dependents as failed                                        │ │
//! │  │     5. Continue or abort based on policy                                │ │
//! │  │                                                                         │ │
//! │  └────────────────────────────────────────────────────────────────────────┘ │
//! │                                                                              │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use core::time::Duration;

use crate::phase::{InitPhase, PhaseBarrier, get_barrier, mark_phase_complete, mark_phase_failed};
use crate::error::{InitResult, InitError, ErrorKind, ErrorHandler, ErrorPolicy, RollbackChain};
use crate::subsystem::{SubsystemId, SubsystemState, SubsystemWrapper};
use crate::registry::{SubsystemRegistry, with_registry_mut};
use crate::context::InitContext;
use crate::dependency::DependencyResolver;

extern crate alloc;
use alloc::collections::{BTreeSet, BTreeMap};
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;

// =============================================================================
// EXECUTION MODE
// =============================================================================

/// How subsystems should be initialized
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    /// Initialize one at a time, in dependency order
    Sequential,

    /// Initialize independent subsystems in parallel
    Parallel,

    /// Don't initialize until first use
    Lazy,

    /// Only initialize if condition is met
    Conditional,
}

impl Default for ExecutionMode {
    fn default() -> Self {
        ExecutionMode::Sequential
    }
}

// =============================================================================
// EXECUTOR CONFIG
// =============================================================================

/// Configuration for the executor
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Execution mode
    pub mode: ExecutionMode,

    /// Maximum parallel tasks
    pub max_parallel: usize,

    /// Default timeout per subsystem (microseconds)
    pub default_timeout_us: u64,

    /// Whether to continue on non-essential failures
    pub continue_on_failure: bool,

    /// Whether to enable rollback on failure
    pub enable_rollback: bool,

    /// Maximum retry count
    pub max_retries: u32,

    /// Retry delay (microseconds)
    pub retry_delay_us: u64,

    /// Whether to validate dependencies before execution
    pub validate_deps: bool,

    /// Whether to collect timing metrics
    pub collect_metrics: bool,

    /// Phases to execute (empty = all)
    pub phases: Vec<InitPhase>,

    /// Subsystems to skip
    pub skip: Vec<SubsystemId>,

    /// Subsystems to force enable
    pub force_enable: Vec<SubsystemId>,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            mode: ExecutionMode::Sequential,
            max_parallel: 4,
            default_timeout_us: 10_000_000, // 10 seconds
            continue_on_failure: false,
            enable_rollback: true,
            max_retries: 3,
            retry_delay_us: 10_000, // 10ms
            validate_deps: true,
            collect_metrics: true,
            phases: Vec::new(),
            skip: Vec::new(),
            force_enable: Vec::new(),
        }
    }
}

impl ExecutorConfig {
    /// Create new config with mode
    pub fn new(mode: ExecutionMode) -> Self {
        Self {
            mode,
            ..Default::default()
        }
    }

    /// Builder: set max parallel
    pub fn max_parallel(mut self, n: usize) -> Self {
        self.max_parallel = n;
        self
    }

    /// Builder: set timeout
    pub fn timeout(mut self, us: u64) -> Self {
        self.default_timeout_us = us;
        self
    }

    /// Builder: continue on failure
    pub fn continue_on_failure(mut self, yes: bool) -> Self {
        self.continue_on_failure = yes;
        self
    }

    /// Builder: enable rollback
    pub fn enable_rollback(mut self, yes: bool) -> Self {
        self.enable_rollback = yes;
        self
    }

    /// Builder: set phases
    pub fn phases(mut self, phases: Vec<InitPhase>) -> Self {
        self.phases = phases;
        self
    }

    /// Builder: skip subsystems
    pub fn skip(mut self, ids: Vec<SubsystemId>) -> Self {
        self.skip = ids;
        self
    }
}

// =============================================================================
// EXECUTION RESULT
// =============================================================================

/// Result of executing initialization
#[derive(Debug)]
pub struct ExecutionResult {
    /// Overall success
    pub success: bool,

    /// Number of subsystems initialized
    pub initialized: usize,

    /// Number of failures
    pub failures: usize,

    /// Number of skipped
    pub skipped: usize,

    /// Total duration (microseconds)
    pub total_duration_us: u64,

    /// Per-phase results
    pub phase_results: [PhaseResult; 5],

    /// Failed subsystem IDs
    pub failed: Vec<SubsystemId>,

    /// Errors encountered
    pub errors: Vec<(SubsystemId, InitError)>,
}

/// Result for a single phase
#[derive(Debug, Clone, Default)]
pub struct PhaseResult {
    /// Phase completed
    pub complete: bool,

    /// Number initialized
    pub initialized: usize,

    /// Number failed
    pub failed: usize,

    /// Duration (microseconds)
    pub duration_us: u64,
}

impl ExecutionResult {
    /// Create new result
    fn new() -> Self {
        Self {
            success: true,
            initialized: 0,
            failures: 0,
            skipped: 0,
            total_duration_us: 0,
            phase_results: Default::default(),
            failed: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Record a failure
    fn record_failure(&mut self, id: SubsystemId, error: InitError) {
        self.success = false;
        self.failures += 1;
        self.failed.push(id);
        self.errors.push((id, error));
    }

    /// Record success
    fn record_success(&mut self) {
        self.initialized += 1;
    }

    /// Record skip
    fn record_skip(&mut self) {
        self.skipped += 1;
    }
}

// =============================================================================
// INIT EXECUTOR
// =============================================================================

/// The main initialization executor
pub struct InitExecutor {
    /// Configuration
    config: ExecutorConfig,

    /// Context
    context: InitContext,

    /// Error handler
    error_handler: ErrorHandler,

    /// Rollback chain
    rollback: RollbackChain,

    /// Satisfied dependencies
    satisfied: BTreeSet<SubsystemId>,

    /// Failed subsystems
    failed: BTreeSet<SubsystemId>,

    /// Skipped subsystems
    skipped: BTreeSet<SubsystemId>,

    /// Execution metrics
    metrics: ExecutionMetrics,

    /// Abort flag
    abort: AtomicBool,

    /// Current phase
    current_phase: InitPhase,

    /// Execution started
    started: bool,
}

/// Execution metrics
#[derive(Debug, Default)]
pub struct ExecutionMetrics {
    /// Start time
    pub start_time: u64,

    /// End time
    pub end_time: u64,

    /// Per-subsystem timings
    pub subsystem_timings: BTreeMap<SubsystemId, SubsystemTiming>,

    /// Total init calls
    pub total_inits: AtomicU32,

    /// Total retries
    pub total_retries: AtomicU32,

    /// Peak memory usage (if tracked)
    pub peak_memory: AtomicU64,
}

/// Timing for a single subsystem
#[derive(Debug, Clone, Default)]
pub struct SubsystemTiming {
    /// Validation time
    pub validate_us: u64,
    /// Init time
    pub init_us: u64,
    /// Post-phase time
    pub post_phase_us: u64,
    /// Retries
    pub retries: u32,
}

impl InitExecutor {
    /// Create new executor with config
    pub fn new(config: ExecutorConfig) -> Self {
        Self {
            context: InitContext::new(InitPhase::Boot),
            config,
            error_handler: ErrorHandler::default(),
            rollback: RollbackChain::new(),
            satisfied: BTreeSet::new(),
            failed: BTreeSet::new(),
            skipped: BTreeSet::new(),
            metrics: ExecutionMetrics::default(),
            abort: AtomicBool::new(false),
            current_phase: InitPhase::Boot,
            started: false,
        }
    }

    /// Create with default config
    pub fn default_executor() -> Self {
        Self::new(ExecutorConfig::default())
    }

    /// Set error handler
    pub fn with_error_handler(mut self, handler: ErrorHandler) -> Self {
        self.error_handler = handler;
        self
    }

    /// Set context
    pub fn with_context(mut self, context: InitContext) -> Self {
        self.context = context;
        self
    }

    /// Execute initialization
    pub fn execute(&mut self, registry: &mut SubsystemRegistry) -> InitResult<ExecutionResult> {
        self.metrics.start_time = crate::get_timestamp();
        self.started = true;

        let mut result = ExecutionResult::new();

        // Validate dependencies
        if self.config.validate_deps {
            registry.validate()?;
        }

        // Determine phases to execute
        let phases = if self.config.phases.is_empty() {
            InitPhase::all().to_vec()
        } else {
            self.config.phases.clone()
        };

        // Execute each phase
        for phase in phases {
            if self.abort.load(Ordering::Acquire) {
                break;
            }

            let phase_result = self.execute_phase(registry, phase)?;
            result.phase_results[phase as usize] = phase_result.clone();

            result.initialized += phase_result.initialized;
            result.failures += phase_result.failed;

            if phase_result.failed > 0 && !self.config.continue_on_failure {
                result.success = false;
                break;
            }
        }

        self.metrics.end_time = crate::get_timestamp();
        result.total_duration_us = self.metrics.end_time - self.metrics.start_time;
        result.failed = self.failed.iter().copied().collect();

        Ok(result)
    }

    /// Execute a single phase
    fn execute_phase(
        &mut self,
        registry: &mut SubsystemRegistry,
        phase: InitPhase,
    ) -> InitResult<PhaseResult> {
        let phase_start = crate::get_timestamp();
        self.current_phase = phase;

        // Advance context phase
        while self.context.phase() < phase {
            self.context.advance_phase()?;
        }

        let mut result = PhaseResult::default();

        // Get subsystem order for this phase
        let order = match self.config.mode {
            ExecutionMode::Parallel => {
                // Get parallel batches
                let batches = registry.get_parallel_batches(phase);
                self.execute_parallel_batches(registry, batches, &mut result)?;
                return Ok(result);
            }
            _ => registry.get_phase_order(phase),
        };

        // Set barrier expectation
        let barrier = get_barrier(phase);
        barrier.set_expected(order.len() as u32);
        barrier.record_start(phase_start);

        // Execute each subsystem
        for id in order {
            if self.abort.load(Ordering::Acquire) {
                break;
            }

            // Check if should skip
            if self.config.skip.contains(&id) {
                self.skipped.insert(id);
                result.initialized += 1; // Count as processed
                mark_phase_complete(phase);
                continue;
            }

            // Check if dependency failed
            if self.should_skip_due_to_failure(registry, id) {
                self.failed.insert(id);
                result.failed += 1;
                mark_phase_failed(phase);
                continue;
            }

            // Execute subsystem
            match self.execute_subsystem(registry, id) {
                Ok(()) => {
                    self.satisfied.insert(id);
                    result.initialized += 1;
                    mark_phase_complete(phase);
                }
                Err(e) => {
                    self.failed.insert(id);
                    result.failed += 1;
                    mark_phase_failed(phase);

                    self.context.error(alloc::format!(
                        "Subsystem {:?} failed: {}",
                        id, e
                    ));

                    // Check if essential
                    if let Some(entry) = registry.get(id) {
                        if entry.info().essential {
                            return Err(e.with_subsystem(id));
                        }
                    }

                    if !self.config.continue_on_failure {
                        return Err(e.with_subsystem(id));
                    }
                }
            }
        }

        // Wait for barrier
        barrier.record_end(crate::get_timestamp());
        result.duration_us = barrier.duration_us();
        result.complete = result.failed == 0;

        Ok(result)
    }

    /// Execute subsystem with retries
    fn execute_subsystem(
        &mut self,
        registry: &mut SubsystemRegistry,
        id: SubsystemId,
    ) -> InitResult<()> {
        let start = crate::get_timestamp();

        self.context.set_current_subsystem(id);

        // Get the entry
        let entry = registry.get_mut(id).ok_or_else(|| {
            InitError::new(ErrorKind::NotFound, "Subsystem not found")
                .with_subsystem(id)
        })?;

        let info = entry.info();
        let timeout = if info.timeout_us > 0 {
            info.timeout_us
        } else {
            self.config.default_timeout_us
        };

        self.context.info(alloc::format!("Initializing: {}", info.name));

        // Validate first
        let validate_start = crate::get_timestamp();
        entry.wrapper.inner().validate(&self.context)?;
        let validate_time = crate::get_timestamp() - validate_start;

        // Transition to ready state
        let state = entry.wrapper.state();
        if state == SubsystemState::Registered {
            // Need to transition through states
            if let Some(entry) = registry.get_mut(id) {
                entry.wrapper.inner_mut().validate(&self.context)?;
            }
        }

        // Execute with retries
        let mut retries = 0u32;
        let mut last_error: Option<InitError> = None;

        while retries <= self.config.max_retries {
            // Re-get entry (may have changed)
            let entry = match registry.get_mut(id) {
                Some(e) => e,
                None => return Err(InitError::new(ErrorKind::NotFound, "Subsystem not found")),
            };

            // Attempt initialization
            match entry.wrapper.try_init(&mut self.context) {
                Ok(()) => {
                    let total_time = crate::get_timestamp() - start;

                    // Record metrics
                    if self.config.collect_metrics {
                        self.metrics.subsystem_timings.insert(id, SubsystemTiming {
                            validate_us: validate_time,
                            init_us: total_time - validate_time,
                            post_phase_us: 0,
                            retries,
                        });
                    }

                    self.metrics.total_inits.fetch_add(1, Ordering::Relaxed);
                    self.context.info(alloc::format!(
                        "Initialized {} in {} us",
                        entry.info().name,
                        total_time
                    ));

                    self.context.clear_current_subsystem();
                    return Ok(());
                }
                Err(e) => {
                    last_error = Some(e);

                    if retries < self.config.max_retries {
                        // Check if error is recoverable
                        if let Some(ref err) = last_error {
                            if err.kind().retry_count() > 0 {
                                retries += 1;
                                self.metrics.total_retries.fetch_add(1, Ordering::Relaxed);

                                // Delay before retry
                                spin_delay(self.config.retry_delay_us);
                                continue;
                            }
                        }
                    }

                    break;
                }
            }
        }

        // Failed after retries
        let error = last_error.unwrap_or_else(|| {
            InitError::new(ErrorKind::SubsystemFailed, "Initialization failed")
        });

        // Execute rollback if enabled
        if self.config.enable_rollback {
            let _ = self.context.rollback_current();
        }

        self.context.clear_current_subsystem();
        registry.record_init_result(false);

        Err(error.with_subsystem(id))
    }

    /// Execute parallel batches
    fn execute_parallel_batches(
        &mut self,
        registry: &mut SubsystemRegistry,
        batches: Vec<Vec<SubsystemId>>,
        result: &mut PhaseResult,
    ) -> InitResult<()> {
        for batch in batches {
            // In real parallel execution, we'd spawn tasks here
            // For now, execute sequentially within batch
            for id in batch {
                if self.config.skip.contains(&id) {
                    self.skipped.insert(id);
                    continue;
                }

                match self.execute_subsystem(registry, id) {
                    Ok(()) => {
                        self.satisfied.insert(id);
                        result.initialized += 1;
                    }
                    Err(e) => {
                        self.failed.insert(id);
                        result.failed += 1;

                        if let Some(entry) = registry.get(id) {
                            if entry.info().essential && !self.config.continue_on_failure {
                                return Err(e);
                            }
                        }
                    }
                }
            }
        }

        result.complete = result.failed == 0;
        Ok(())
    }

    /// Check if subsystem should be skipped due to dependency failure
    fn should_skip_due_to_failure(
        &self,
        registry: &SubsystemRegistry,
        id: SubsystemId,
    ) -> bool {
        if let Some(entry) = registry.get(id) {
            for dep in entry.info().dependencies {
                if dep.kind == crate::subsystem::DependencyKind::Required {
                    if self.failed.contains(&dep.id) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Request abort
    pub fn abort(&self) {
        self.abort.store(true, Ordering::Release);
    }

    /// Get context
    pub fn context(&self) -> &InitContext {
        &self.context
    }

    /// Get mutable context
    pub fn context_mut(&mut self) -> &mut InitContext {
        &mut self.context
    }

    /// Get metrics
    pub fn metrics(&self) -> &ExecutionMetrics {
        &self.metrics
    }

    /// Get satisfied subsystems
    pub fn satisfied(&self) -> &BTreeSet<SubsystemId> {
        &self.satisfied
    }

    /// Get failed subsystems
    pub fn failed(&self) -> &BTreeSet<SubsystemId> {
        &self.failed
    }
}

/// Simple spin delay
fn spin_delay(us: u64) {
    let start = crate::get_timestamp();
    while crate::get_timestamp() - start < us {
        core::hint::spin_loop();
    }
}

// =============================================================================
// SHUTDOWN EXECUTOR
// =============================================================================

/// Executor for shutdown (reverse order)
pub struct ShutdownExecutor {
    /// Timeout per subsystem
    timeout_us: u64,

    /// Whether to force shutdown on timeout
    force_on_timeout: bool,
}

impl ShutdownExecutor {
    /// Create new shutdown executor
    pub fn new() -> Self {
        Self {
            timeout_us: 5_000_000, // 5 seconds
            force_on_timeout: true,
        }
    }

    /// Set timeout
    pub fn timeout(mut self, us: u64) -> Self {
        self.timeout_us = us;
        self
    }

    /// Execute shutdown in reverse order
    pub fn execute(
        &self,
        registry: &mut SubsystemRegistry,
        ctx: &mut InitContext,
    ) -> InitResult<()> {
        // Get reverse order
        let order: Vec<SubsystemId> = registry.get_init_order()?
            .into_iter()
            .rev()
            .collect();

        let mut errors = Vec::new();

        for id in order {
            if let Some(entry) = registry.get_mut(id) {
                if entry.state().can_shutdown() {
                    ctx.set_current_subsystem(id);

                    match entry.wrapper.try_shutdown(ctx) {
                        Ok(()) => {
                            ctx.info(alloc::format!("Shutdown: {}", entry.info().name));
                        }
                        Err(e) => {
                            ctx.error(alloc::format!(
                                "Shutdown failed: {}: {}",
                                entry.info().name, e
                            ));
                            errors.push((id, e));
                        }
                    }

                    ctx.clear_current_subsystem();
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(InitError::new(
                ErrorKind::SubsystemShutdownFailed,
                "Some subsystems failed to shut down",
            ))
        }
    }
}

impl Default for ShutdownExecutor {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// LAZY EXECUTOR
// =============================================================================

/// Executor for lazy initialization
pub struct LazyExecutor {
    /// Subsystems marked for lazy init
    lazy_subsystems: BTreeSet<SubsystemId>,

    /// Initialization lock per subsystem
    init_locks: BTreeMap<SubsystemId, AtomicBool>,
}

impl LazyExecutor {
    /// Create new lazy executor
    pub fn new() -> Self {
        Self {
            lazy_subsystems: BTreeSet::new(),
            init_locks: BTreeMap::new(),
        }
    }

    /// Mark subsystem for lazy init
    pub fn mark_lazy(&mut self, id: SubsystemId) {
        self.lazy_subsystems.insert(id);
        self.init_locks.insert(id, AtomicBool::new(false));
    }

    /// Check if subsystem is lazy
    pub fn is_lazy(&self, id: SubsystemId) -> bool {
        self.lazy_subsystems.contains(&id)
    }

    /// Initialize on demand
    pub fn init_on_demand(
        &self,
        registry: &mut SubsystemRegistry,
        ctx: &mut InitContext,
        id: SubsystemId,
    ) -> InitResult<()> {
        // Check if already initialized
        if let Some(entry) = registry.get(id) {
            if entry.state() == SubsystemState::Active {
                return Ok(());
            }
        }

        // Try to acquire init lock
        if let Some(lock) = self.init_locks.get(&id) {
            if lock.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
                // Another thread is initializing
                return Ok(());
            }
        }

        // Initialize dependencies first
        if let Some(entry) = registry.get(id) {
            for dep in entry.info().dependencies {
                if self.is_lazy(dep.id) {
                    self.init_on_demand(registry, ctx, dep.id)?;
                }
            }
        }

        // Initialize the subsystem
        if let Some(entry) = registry.get_mut(id) {
            entry.wrapper.try_init(ctx)?;
        }

        Ok(())
    }
}

impl Default for LazyExecutor {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subsystem::{SubsystemInfo, Subsystem};
    use crate::phase::PhaseCapabilities;

    struct TestSubsystem {
        info: SubsystemInfo,
        init_called: bool,
    }

    impl TestSubsystem {
        fn new(name: &'static str, phase: InitPhase) -> Self {
            Self {
                info: SubsystemInfo::new(name, phase),
                init_called: false,
            }
        }
    }

    impl Subsystem for TestSubsystem {
        fn info(&self) -> &SubsystemInfo {
            &self.info
        }

        fn init(&mut self, _ctx: &mut InitContext) -> InitResult<()> {
            self.init_called = true;
            Ok(())
        }
    }

    #[test]
    fn test_executor_config() {
        let config = ExecutorConfig::new(ExecutionMode::Parallel)
            .max_parallel(8)
            .timeout(5_000_000)
            .continue_on_failure(true);

        assert_eq!(config.mode, ExecutionMode::Parallel);
        assert_eq!(config.max_parallel, 8);
        assert_eq!(config.default_timeout_us, 5_000_000);
        assert!(config.continue_on_failure);
    }

    #[test]
    fn test_execution_result() {
        let mut result = ExecutionResult::new();

        result.record_success();
        result.record_success();
        result.record_failure(
            SubsystemId::from_name("test"),
            InitError::new(ErrorKind::Timeout, "Test")
        );

        assert!(!result.success);
        assert_eq!(result.initialized, 2);
        assert_eq!(result.failures, 1);
        assert_eq!(result.failed.len(), 1);
    }

    #[test]
    fn test_executor_creation() {
        let executor = InitExecutor::default_executor();

        assert_eq!(executor.config.mode, ExecutionMode::Sequential);
        assert!(!executor.started);
    }

    #[test]
    fn test_shutdown_executor() {
        let executor = ShutdownExecutor::new()
            .timeout(1_000_000);

        assert_eq!(executor.timeout_us, 1_000_000);
    }

    #[test]
    fn test_lazy_executor() {
        let mut executor = LazyExecutor::new();
        let id = SubsystemId::from_name("lazy_test");

        executor.mark_lazy(id);
        assert!(executor.is_lazy(id));
        assert!(!executor.is_lazy(SubsystemId::from_name("other")));
    }
}
