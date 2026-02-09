//! Work Item and Queue Information
//!
//! This module provides structures for tracking work items and queue metadata.

use alloc::string::String;

use super::{WorkId, WorkPriority, WorkQueueId, WorkQueueType, WorkState};

/// Work item information
#[derive(Debug, Clone)]
pub struct WorkInfo {
    /// Work item ID
    pub id: WorkId,
    /// Parent queue ID
    pub queue_id: WorkQueueId,
    /// Work priority
    pub priority: WorkPriority,
    /// Current state
    pub state: WorkState,
    /// Creation timestamp (ticks)
    pub created_at: u64,
    /// Enqueue timestamp (ticks)
    pub enqueued_at: u64,
    /// Start timestamp if running (ticks)
    pub started_at: Option<u64>,
    /// Completion timestamp if done (ticks)
    pub completed_at: Option<u64>,
    /// Expected execution time (nanoseconds)
    pub expected_duration_ns: u64,
    /// Actual execution time if completed (nanoseconds)
    pub actual_duration_ns: Option<u64>,
    /// CPU affinity mask
    pub cpu_affinity: u64,
    /// Number of retries
    pub retry_count: u32,
    /// Maximum retries allowed
    pub max_retries: u32,
    /// Work function name/identifier
    pub function_name: String,
    /// Is work cancellable
    pub cancellable: bool,
}

impl WorkInfo {
    /// Create new work info
    pub fn new(
        id: WorkId,
        queue_id: WorkQueueId,
        priority: WorkPriority,
        function_name: String,
    ) -> Self {
        Self {
            id,
            queue_id,
            priority,
            state: WorkState::Pending,
            created_at: 0,
            enqueued_at: 0,
            started_at: None,
            completed_at: None,
            expected_duration_ns: 0,
            actual_duration_ns: None,
            cpu_affinity: u64::MAX, // All CPUs
            retry_count: 0,
            max_retries: 3,
            function_name,
            cancellable: true,
        }
    }

    /// Get queue wait time in nanoseconds
    #[inline]
    pub fn wait_time_ns(&self) -> u64 {
        if let Some(started) = self.started_at {
            started.saturating_sub(self.enqueued_at)
        } else {
            0
        }
    }

    /// Check if work is overdue
    #[inline]
    pub fn is_overdue(&self, current_time: u64) -> bool {
        if self.state == WorkState::Pending {
            let wait_time = current_time.saturating_sub(self.enqueued_at);
            wait_time > self.expected_duration_ns * 10 // 10x expected duration
        } else {
            false
        }
    }
}

/// Work queue information
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct WorkQueueInfo {
    /// Queue ID
    pub id: WorkQueueId,
    /// Queue name
    pub name: String,
    /// Queue type
    pub queue_type: WorkQueueType,
    /// Maximum concurrent workers
    pub max_workers: u32,
    /// Current active workers
    pub active_workers: u32,
    /// Current pending work items
    pub pending_count: u64,
    /// Total work items processed
    pub processed_count: u64,
    /// Total work items failed
    pub failed_count: u64,
    /// Average processing time (nanoseconds)
    pub avg_processing_time_ns: u64,
    /// Peak processing time (nanoseconds)
    pub peak_processing_time_ns: u64,
    /// Average queue depth
    pub avg_queue_depth: f32,
    /// Peak queue depth
    pub peak_queue_depth: u64,
    /// Is queue frozen
    pub frozen: bool,
    /// CPU mask for bound queues
    pub cpu_mask: u64,
    /// Creation timestamp
    pub created_at: u64,
}

impl WorkQueueInfo {
    /// Create new work queue info
    pub fn new(id: WorkQueueId, name: String, queue_type: WorkQueueType) -> Self {
        Self {
            id,
            name,
            queue_type,
            max_workers: 1,
            active_workers: 0,
            pending_count: 0,
            processed_count: 0,
            failed_count: 0,
            avg_processing_time_ns: 0,
            peak_processing_time_ns: 0,
            avg_queue_depth: 0.0,
            peak_queue_depth: 0,
            frozen: false,
            cpu_mask: u64::MAX,
            created_at: 0,
        }
    }

    /// Calculate queue utilization
    #[inline]
    pub fn utilization(&self) -> f32 {
        if self.max_workers == 0 {
            return 0.0;
        }
        (self.active_workers as f32 / self.max_workers as f32).min(1.0)
    }

    /// Calculate failure rate
    #[inline]
    pub fn failure_rate(&self) -> f32 {
        let total = self.processed_count + self.failed_count;
        if total == 0 {
            return 0.0;
        }
        self.failed_count as f32 / total as f32
    }
}
