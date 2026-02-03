//! GPU Async Types for Lumina
//!
//! This module provides asynchronous GPU operation primitives
//! for efficient non-blocking GPU work submission and tracking.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Async Handles
// ============================================================================

/// Async operation handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuAsyncHandle(pub u64);

impl GpuAsyncHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for GpuAsyncHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// GPU future handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuFutureHandle(pub u64);

impl GpuFutureHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for GpuFutureHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// GPU task handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuTaskHandle(pub u64);

impl GpuTaskHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for GpuTaskHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// GPU continuation handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuContinuationHandle(pub u64);

impl GpuContinuationHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for GpuContinuationHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Async Operation Types
// ============================================================================

/// Async operation state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AsyncOperationState {
    /// Not started
    #[default]
    Pending = 0,
    /// Submitted to GPU
    Submitted = 1,
    /// Running on GPU
    Running = 2,
    /// Completed successfully
    Completed = 3,
    /// Failed with error
    Failed = 4,
    /// Cancelled
    Cancelled = 5,
    /// Timed out
    TimedOut = 6,
}

impl AsyncOperationState {
    /// Is terminal state
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled | Self::TimedOut)
    }

    /// Is success
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Completed)
    }

    /// Is error
    pub const fn is_error(&self) -> bool {
        matches!(self, Self::Failed | Self::TimedOut)
    }
}

/// Async operation type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AsyncOperationType {
    /// General compute work
    #[default]
    Compute = 0,
    /// Graphics rendering
    Graphics = 1,
    /// Transfer operation
    Transfer = 2,
    /// Video decode
    VideoDecode = 3,
    /// Video encode
    VideoEncode = 4,
    /// Ray tracing
    RayTracing = 5,
    /// Sparse binding
    SparseBinding = 6,
    /// Mixed operations
    Mixed = 7,
}

/// Async operation priority
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AsyncPriority {
    /// Low priority
    Low = 0,
    /// Normal priority
    #[default]
    Normal = 1,
    /// High priority
    High = 2,
    /// Real-time priority
    RealTime = 3,
}

impl AsyncPriority {
    /// Priority value (0-255)
    pub const fn value(&self) -> u8 {
        match self {
            Self::Low => 64,
            Self::Normal => 128,
            Self::High => 192,
            Self::RealTime => 255,
        }
    }
}

// ============================================================================
// Async Operation Info
// ============================================================================

/// Async operation create info
#[derive(Clone, Debug)]
pub struct AsyncOperationCreateInfo {
    /// Name for debugging
    pub name: String,
    /// Operation type
    pub operation_type: AsyncOperationType,
    /// Priority
    pub priority: AsyncPriority,
    /// Timeout in microseconds (0 = no timeout)
    pub timeout_us: u64,
    /// Enable profiling
    pub enable_profiling: bool,
    /// Auto-submit on creation
    pub auto_submit: bool,
}

impl AsyncOperationCreateInfo {
    /// Creates new info
    pub fn new(operation_type: AsyncOperationType) -> Self {
        Self {
            name: String::new(),
            operation_type,
            priority: AsyncPriority::Normal,
            timeout_us: 0,
            enable_profiling: false,
            auto_submit: false,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With priority
    pub fn with_priority(mut self, priority: AsyncPriority) -> Self {
        self.priority = priority;
        self
    }

    /// With timeout
    pub fn with_timeout_us(mut self, timeout: u64) -> Self {
        self.timeout_us = timeout;
        self
    }

    /// With timeout in milliseconds
    pub fn with_timeout_ms(self, timeout_ms: u64) -> Self {
        self.with_timeout_us(timeout_ms * 1000)
    }

    /// Enable profiling
    pub fn with_profiling(mut self) -> Self {
        self.enable_profiling = true;
        self
    }

    /// Auto submit
    pub fn with_auto_submit(mut self) -> Self {
        self.auto_submit = true;
        self
    }

    /// Compute operation
    pub fn compute() -> Self {
        Self::new(AsyncOperationType::Compute)
    }

    /// Graphics operation
    pub fn graphics() -> Self {
        Self::new(AsyncOperationType::Graphics)
    }

    /// Transfer operation
    pub fn transfer() -> Self {
        Self::new(AsyncOperationType::Transfer)
    }

    /// High priority compute
    pub fn compute_high_priority() -> Self {
        Self::compute().with_priority(AsyncPriority::High)
    }

    /// Real-time graphics
    pub fn graphics_realtime() -> Self {
        Self::graphics().with_priority(AsyncPriority::RealTime)
    }
}

impl Default for AsyncOperationCreateInfo {
    fn default() -> Self {
        Self::compute()
    }
}

// ============================================================================
// Async Result Types
// ============================================================================

/// Async operation result
#[derive(Clone, Debug)]
pub struct AsyncResult<T> {
    /// Result value (if success)
    pub value: Option<T>,
    /// Error (if failure)
    pub error: Option<AsyncError>,
    /// Execution time in nanoseconds
    pub execution_time_ns: u64,
    /// GPU timestamp start
    pub gpu_timestamp_start: u64,
    /// GPU timestamp end
    pub gpu_timestamp_end: u64,
}

impl<T> AsyncResult<T> {
    /// Success result
    pub fn success(value: T) -> Self {
        Self {
            value: Some(value),
            error: None,
            execution_time_ns: 0,
            gpu_timestamp_start: 0,
            gpu_timestamp_end: 0,
        }
    }

    /// Error result
    pub fn error(err: AsyncError) -> Self {
        Self {
            value: None,
            error: Some(err),
            execution_time_ns: 0,
            gpu_timestamp_start: 0,
            gpu_timestamp_end: 0,
        }
    }

    /// Is success
    pub fn is_success(&self) -> bool {
        self.value.is_some()
    }

    /// Is error
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// With timing
    pub fn with_timing(mut self, execution_ns: u64, start: u64, end: u64) -> Self {
        self.execution_time_ns = execution_ns;
        self.gpu_timestamp_start = start;
        self.gpu_timestamp_end = end;
        self
    }
}

impl<T: Default> Default for AsyncResult<T> {
    fn default() -> Self {
        Self::success(T::default())
    }
}

/// Async error
#[derive(Clone, Debug)]
pub struct AsyncError {
    /// Error code
    pub code: AsyncErrorCode,
    /// Error message
    pub message: String,
}

impl AsyncError {
    /// Creates new error
    pub fn new(code: AsyncErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Timeout error
    pub fn timeout() -> Self {
        Self::new(AsyncErrorCode::Timeout, "Operation timed out")
    }

    /// Device lost error
    pub fn device_lost() -> Self {
        Self::new(AsyncErrorCode::DeviceLost, "GPU device lost")
    }

    /// Cancelled error
    pub fn cancelled() -> Self {
        Self::new(AsyncErrorCode::Cancelled, "Operation cancelled")
    }
}

/// Async error code
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AsyncErrorCode {
    /// Unknown error
    #[default]
    Unknown = 0,
    /// Timeout
    Timeout = 1,
    /// Device lost
    DeviceLost = 2,
    /// Out of memory
    OutOfMemory = 3,
    /// Cancelled
    Cancelled = 4,
    /// Invalid operation
    InvalidOperation = 5,
    /// Resource not ready
    NotReady = 6,
    /// Validation error
    ValidationError = 7,
}

// ============================================================================
// GPU Future
// ============================================================================

/// GPU future create info
#[derive(Clone, Debug)]
pub struct GpuFutureCreateInfo {
    /// Name
    pub name: String,
    /// Initial value
    pub initial_value: u64,
    /// Signaled on creation
    pub signaled: bool,
}

impl GpuFutureCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            initial_value: 0,
            signaled: false,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With initial value
    pub fn with_initial_value(mut self, value: u64) -> Self {
        self.initial_value = value;
        self
    }

    /// Signaled on creation
    pub fn signaled(mut self) -> Self {
        self.signaled = true;
        self
    }
}

impl Default for GpuFutureCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// GPU future state
#[derive(Clone, Debug)]
pub struct GpuFutureState {
    /// Handle
    pub handle: GpuFutureHandle,
    /// Current value
    pub value: u64,
    /// Target value
    pub target_value: u64,
    /// Is complete
    pub complete: bool,
}

impl Default for GpuFutureState {
    fn default() -> Self {
        Self {
            handle: GpuFutureHandle::NULL,
            value: 0,
            target_value: 0,
            complete: false,
        }
    }
}

// ============================================================================
// GPU Task System
// ============================================================================

/// GPU task create info
#[derive(Clone, Debug)]
pub struct GpuTaskCreateInfo {
    /// Name
    pub name: String,
    /// Task type
    pub task_type: GpuTaskType,
    /// Priority
    pub priority: AsyncPriority,
    /// Dependencies
    pub dependencies: Vec<GpuTaskHandle>,
    /// User data
    pub user_data: u64,
}

impl GpuTaskCreateInfo {
    /// Creates new info
    pub fn new(task_type: GpuTaskType) -> Self {
        Self {
            name: String::new(),
            task_type,
            priority: AsyncPriority::Normal,
            dependencies: Vec::new(),
            user_data: 0,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With priority
    pub fn with_priority(mut self, priority: AsyncPriority) -> Self {
        self.priority = priority;
        self
    }

    /// With dependency
    pub fn with_dependency(mut self, dep: GpuTaskHandle) -> Self {
        self.dependencies.push(dep);
        self
    }

    /// With dependencies
    pub fn with_dependencies(mut self, deps: impl IntoIterator<Item = GpuTaskHandle>) -> Self {
        self.dependencies.extend(deps);
        self
    }

    /// With user data
    pub fn with_user_data(mut self, data: u64) -> Self {
        self.user_data = data;
        self
    }

    /// Render task
    pub fn render() -> Self {
        Self::new(GpuTaskType::Render)
    }

    /// Compute task
    pub fn compute() -> Self {
        Self::new(GpuTaskType::Compute)
    }

    /// Upload task
    pub fn upload() -> Self {
        Self::new(GpuTaskType::Upload)
    }

    /// Download task
    pub fn download() -> Self {
        Self::new(GpuTaskType::Download)
    }
}

impl Default for GpuTaskCreateInfo {
    fn default() -> Self {
        Self::compute()
    }
}

/// GPU task type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum GpuTaskType {
    /// Render task
    Render = 0,
    /// Compute task
    #[default]
    Compute = 1,
    /// Upload to GPU
    Upload = 2,
    /// Download from GPU
    Download = 3,
    /// Barrier/sync
    Barrier = 4,
    /// Present
    Present = 5,
}

/// GPU task state
#[derive(Clone, Debug)]
pub struct GpuTaskState {
    /// Handle
    pub handle: GpuTaskHandle,
    /// Current state
    pub state: AsyncOperationState,
    /// Progress (0-100)
    pub progress: u32,
    /// Start time (nanoseconds)
    pub start_time_ns: u64,
    /// End time (nanoseconds)
    pub end_time_ns: u64,
}

impl Default for GpuTaskState {
    fn default() -> Self {
        Self {
            handle: GpuTaskHandle::NULL,
            state: AsyncOperationState::Pending,
            progress: 0,
            start_time_ns: 0,
            end_time_ns: 0,
        }
    }
}

// ============================================================================
// Continuation System
// ============================================================================

/// GPU continuation create info
#[derive(Clone, Debug)]
pub struct GpuContinuationCreateInfo {
    /// Name
    pub name: String,
    /// Source task
    pub source: GpuTaskHandle,
    /// Continuation type
    pub continuation_type: ContinuationType,
    /// Run on same queue
    pub same_queue: bool,
}

impl GpuContinuationCreateInfo {
    /// Creates new info
    pub fn new(source: GpuTaskHandle) -> Self {
        Self {
            name: String::new(),
            source,
            continuation_type: ContinuationType::OnComplete,
            same_queue: true,
        }
    }

    /// On complete
    pub fn on_complete(source: GpuTaskHandle) -> Self {
        Self::new(source).with_type(ContinuationType::OnComplete)
    }

    /// On success
    pub fn on_success(source: GpuTaskHandle) -> Self {
        Self::new(source).with_type(ContinuationType::OnSuccess)
    }

    /// On failure
    pub fn on_failure(source: GpuTaskHandle) -> Self {
        Self::new(source).with_type(ContinuationType::OnFailure)
    }

    /// With type
    pub fn with_type(mut self, continuation_type: ContinuationType) -> Self {
        self.continuation_type = continuation_type;
        self
    }

    /// On different queue
    pub fn on_different_queue(mut self) -> Self {
        self.same_queue = false;
        self
    }
}

impl Default for GpuContinuationCreateInfo {
    fn default() -> Self {
        Self::new(GpuTaskHandle::NULL)
    }
}

/// Continuation type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ContinuationType {
    /// On complete (success or failure)
    #[default]
    OnComplete = 0,
    /// On success only
    OnSuccess = 1,
    /// On failure only
    OnFailure = 2,
    /// On cancellation
    OnCancel = 3,
    /// On timeout
    OnTimeout = 4,
}

// ============================================================================
// Batch Operations
// ============================================================================

/// Async batch
#[derive(Clone, Debug)]
pub struct AsyncBatch {
    /// Name
    pub name: String,
    /// Operations
    pub operations: Vec<GpuAsyncHandle>,
    /// Execute in order
    pub sequential: bool,
    /// Cancel on first error
    pub cancel_on_error: bool,
}

impl AsyncBatch {
    /// Creates new batch
    pub fn new() -> Self {
        Self {
            name: String::new(),
            operations: Vec::new(),
            sequential: false,
            cancel_on_error: true,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Add operation
    pub fn add(mut self, op: GpuAsyncHandle) -> Self {
        self.operations.push(op);
        self
    }

    /// Add multiple operations
    pub fn add_all(mut self, ops: impl IntoIterator<Item = GpuAsyncHandle>) -> Self {
        self.operations.extend(ops);
        self
    }

    /// Sequential execution
    pub fn sequential(mut self) -> Self {
        self.sequential = true;
        self
    }

    /// Parallel execution
    pub fn parallel(mut self) -> Self {
        self.sequential = false;
        self
    }

    /// Don't cancel on error
    pub fn continue_on_error(mut self) -> Self {
        self.cancel_on_error = false;
        self
    }

    /// Count
    pub fn count(&self) -> usize {
        self.operations.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }
}

impl Default for AsyncBatch {
    fn default() -> Self {
        Self::new()
    }
}

/// Batch result
#[derive(Clone, Debug, Default)]
pub struct AsyncBatchResult {
    /// Completed count
    pub completed: u32,
    /// Failed count
    pub failed: u32,
    /// Cancelled count
    pub cancelled: u32,
    /// Total time in nanoseconds
    pub total_time_ns: u64,
    /// First error (if any)
    pub first_error: Option<AsyncError>,
}

impl AsyncBatchResult {
    /// All succeeded
    pub fn all_succeeded(&self) -> bool {
        self.failed == 0 && self.cancelled == 0
    }

    /// Total operations
    pub fn total(&self) -> u32 {
        self.completed + self.failed + self.cancelled
    }
}

// ============================================================================
// Wait Operations
// ============================================================================

/// Wait info
#[derive(Clone, Debug)]
pub struct AsyncWaitInfo {
    /// Operations to wait for
    pub operations: Vec<GpuAsyncHandle>,
    /// Wait mode
    pub mode: WaitMode,
    /// Timeout in nanoseconds (0 = infinite)
    pub timeout_ns: u64,
}

impl AsyncWaitInfo {
    /// Wait for single operation
    pub fn single(op: GpuAsyncHandle) -> Self {
        Self {
            operations: alloc::vec![op],
            mode: WaitMode::All,
            timeout_ns: 0,
        }
    }

    /// Wait for all operations
    pub fn all(ops: impl IntoIterator<Item = GpuAsyncHandle>) -> Self {
        Self {
            operations: ops.into_iter().collect(),
            mode: WaitMode::All,
            timeout_ns: 0,
        }
    }

    /// Wait for any operation
    pub fn any(ops: impl IntoIterator<Item = GpuAsyncHandle>) -> Self {
        Self {
            operations: ops.into_iter().collect(),
            mode: WaitMode::Any,
            timeout_ns: 0,
        }
    }

    /// With timeout
    pub fn with_timeout_ns(mut self, timeout: u64) -> Self {
        self.timeout_ns = timeout;
        self
    }

    /// With timeout in microseconds
    pub fn with_timeout_us(self, timeout: u64) -> Self {
        self.with_timeout_ns(timeout * 1000)
    }

    /// With timeout in milliseconds
    pub fn with_timeout_ms(self, timeout: u64) -> Self {
        self.with_timeout_ns(timeout * 1_000_000)
    }
}

impl Default for AsyncWaitInfo {
    fn default() -> Self {
        Self {
            operations: Vec::new(),
            mode: WaitMode::All,
            timeout_ns: 0,
        }
    }
}

/// Wait mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum WaitMode {
    /// Wait for all operations
    #[default]
    All = 0,
    /// Wait for any operation
    Any = 1,
}

/// Wait result
#[derive(Clone, Debug, Default)]
pub struct AsyncWaitResult {
    /// Did timeout
    pub timed_out: bool,
    /// Completed operations
    pub completed: Vec<GpuAsyncHandle>,
    /// Still pending operations
    pub pending: Vec<GpuAsyncHandle>,
    /// Wait time in nanoseconds
    pub wait_time_ns: u64,
}

impl AsyncWaitResult {
    /// All completed
    pub fn all_completed(&self) -> bool {
        self.pending.is_empty() && !self.timed_out
    }
}

// ============================================================================
// Polling System
// ============================================================================

/// Poll result
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PollResult {
    /// Not ready yet
    #[default]
    Pending = 0,
    /// Ready / completed
    Ready = 1,
    /// Error occurred
    Error = 2,
    /// Already polled/consumed
    Consumed = 3,
}

impl PollResult {
    /// Is ready
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }

    /// Is pending
    pub const fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }
}

/// Poll info
#[derive(Clone, Debug, Default)]
pub struct AsyncPollInfo {
    /// Operation handle
    pub operation: GpuAsyncHandle,
    /// Current state
    pub state: AsyncOperationState,
    /// Progress (0-100)
    pub progress: u32,
    /// Estimated remaining time in nanoseconds
    pub estimated_remaining_ns: u64,
}

// ============================================================================
// Statistics
// ============================================================================

/// Async system statistics
#[derive(Clone, Debug, Default)]
pub struct AsyncStats {
    /// Total operations submitted
    pub total_submitted: u64,
    /// Total operations completed
    pub total_completed: u64,
    /// Total operations failed
    pub total_failed: u64,
    /// Total operations cancelled
    pub total_cancelled: u64,
    /// Total operations timed out
    pub total_timed_out: u64,
    /// Current pending operations
    pub pending_count: u32,
    /// Average wait time in nanoseconds
    pub avg_wait_time_ns: u64,
    /// Average execution time in nanoseconds
    pub avg_execution_time_ns: u64,
    /// Peak concurrent operations
    pub peak_concurrent: u32,
}

impl AsyncStats {
    /// Total processed
    pub fn total_processed(&self) -> u64 {
        self.total_completed + self.total_failed + self.total_cancelled + self.total_timed_out
    }

    /// Success rate (0.0 - 1.0)
    pub fn success_rate(&self) -> f32 {
        let total = self.total_processed();
        if total == 0 {
            return 1.0;
        }
        self.total_completed as f32 / total as f32
    }
}
