//! Command Queue Management
//!
//! GPU command queue abstraction for submission and synchronization.

use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use bitflags::bitflags;
use lumina_core::Handle;

use crate::device::{QueueCapabilities, QueueType};

// ============================================================================
// Queue Priority
// ============================================================================

/// Queue priority level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum QueuePriority {
    /// Low priority (background tasks).
    Low      = 0,
    /// Normal priority (default).
    Normal   = 1,
    /// High priority (important tasks).
    High     = 2,
    /// Realtime priority (time-critical).
    Realtime = 3,
}

impl QueuePriority {
    /// Convert to float (0.0 - 1.0).
    pub fn as_float(&self) -> f32 {
        match self {
            QueuePriority::Low => 0.0,
            QueuePriority::Normal => 0.5,
            QueuePriority::High => 0.75,
            QueuePriority::Realtime => 1.0,
        }
    }
}

impl Default for QueuePriority {
    fn default() -> Self {
        QueuePriority::Normal
    }
}

// ============================================================================
// Queue Family
// ============================================================================

/// Queue family description.
#[derive(Debug, Clone)]
pub struct QueueFamily {
    /// Family index.
    pub index: u32,
    /// Queue count in this family.
    pub queue_count: u32,
    /// Capabilities.
    pub capabilities: QueueCapabilities,
    /// Timestamp valid bits.
    pub timestamp_valid_bits: u32,
    /// Can present to surface.
    pub can_present: bool,
    /// Minimum image transfer granularity.
    pub min_image_transfer_granularity: [u32; 3],
}

impl QueueFamily {
    /// Check if family supports graphics.
    pub fn supports_graphics(&self) -> bool {
        self.capabilities.contains(QueueCapabilities::GRAPHICS)
    }

    /// Check if family supports compute.
    pub fn supports_compute(&self) -> bool {
        self.capabilities.contains(QueueCapabilities::COMPUTE)
    }

    /// Check if family supports transfer.
    pub fn supports_transfer(&self) -> bool {
        self.capabilities.contains(QueueCapabilities::TRANSFER)
    }

    /// Check if family supports all operations.
    pub fn is_universal(&self) -> bool {
        self.supports_graphics() && self.supports_compute() && self.supports_transfer()
    }

    /// Get queue type.
    pub fn queue_type(&self) -> QueueType {
        if self.supports_graphics() {
            QueueType::Graphics
        } else if self.supports_compute() {
            QueueType::Compute
        } else {
            QueueType::Transfer
        }
    }
}

// ============================================================================
// Queue Handle
// ============================================================================

/// Handle to a queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QueueHandle(Handle<Queue>);

impl QueueHandle {
    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self(Handle::from_raw_parts(index, generation))
    }

    /// Get the index.
    pub fn index(&self) -> u32 {
        self.0.index()
    }
}

// ============================================================================
// Queue
// ============================================================================

/// A GPU command queue.
pub struct Queue {
    /// Handle.
    pub handle: QueueHandle,
    /// Family index.
    pub family_index: u32,
    /// Queue index within family.
    pub queue_index: u32,
    /// Queue type.
    pub queue_type: QueueType,
    /// Priority.
    pub priority: QueuePriority,
    /// Capabilities.
    pub capabilities: QueueCapabilities,
    /// Last submitted fence value.
    last_submitted: AtomicU64,
    /// Last completed fence value.
    last_completed: AtomicU64,
    /// Submission count.
    submission_count: AtomicU64,
}

impl Queue {
    /// Create a new queue.
    pub fn new(
        handle: QueueHandle,
        family_index: u32,
        queue_index: u32,
        queue_type: QueueType,
        priority: QueuePriority,
        capabilities: QueueCapabilities,
    ) -> Self {
        Self {
            handle,
            family_index,
            queue_index,
            queue_type,
            priority,
            capabilities,
            last_submitted: AtomicU64::new(0),
            last_completed: AtomicU64::new(0),
            submission_count: AtomicU64::new(0),
        }
    }

    /// Get last submitted fence value.
    pub fn last_submitted(&self) -> u64 {
        self.last_submitted.load(Ordering::Acquire)
    }

    /// Get last completed fence value.
    pub fn last_completed(&self) -> u64 {
        self.last_completed.load(Ordering::Acquire)
    }

    /// Check if queue supports graphics.
    pub fn supports_graphics(&self) -> bool {
        self.capabilities.contains(QueueCapabilities::GRAPHICS)
    }

    /// Check if queue supports compute.
    pub fn supports_compute(&self) -> bool {
        self.capabilities.contains(QueueCapabilities::COMPUTE)
    }

    /// Check if queue supports transfer.
    pub fn supports_transfer(&self) -> bool {
        self.capabilities.contains(QueueCapabilities::TRANSFER)
    }

    /// Check if queue supports presentation.
    pub fn supports_present(&self) -> bool {
        self.capabilities.contains(QueueCapabilities::PRESENT)
    }

    /// Get submission count.
    pub fn submission_count(&self) -> u64 {
        self.submission_count.load(Ordering::Relaxed)
    }

    /// Submit work (updates internal counters).
    pub fn submit(&self) -> u64 {
        self.submission_count.fetch_add(1, Ordering::Relaxed);
        self.last_submitted.fetch_add(1, Ordering::Release)
    }

    /// Mark fence as completed.
    pub fn complete(&self, fence_value: u64) {
        self.last_completed.store(fence_value, Ordering::Release);
    }

    /// Wait for queue to be idle.
    pub fn wait_idle(&self) {
        // Backend-specific implementation
    }
}

// ============================================================================
// Queue Manager
// ============================================================================

/// Manages all device queues.
pub struct QueueManager {
    /// All queues.
    queues: Vec<Queue>,
    /// Graphics queue index.
    graphics_queue: Option<usize>,
    /// Compute queue index.
    compute_queue: Option<usize>,
    /// Transfer queue index.
    transfer_queue: Option<usize>,
    /// Present queue index.
    present_queue: Option<usize>,
}

impl QueueManager {
    /// Create a new queue manager.
    pub fn new() -> Self {
        Self {
            queues: Vec::new(),
            graphics_queue: None,
            compute_queue: None,
            transfer_queue: None,
            present_queue: None,
        }
    }

    /// Add a queue.
    pub fn add_queue(&mut self, queue: Queue) {
        let index = self.queues.len();

        // Track specialized queues
        if queue.supports_graphics() && self.graphics_queue.is_none() {
            self.graphics_queue = Some(index);
        }
        if queue.supports_compute() && self.compute_queue.is_none() {
            self.compute_queue = Some(index);
        }
        if queue.supports_transfer() && self.transfer_queue.is_none() {
            self.transfer_queue = Some(index);
        }
        if queue.supports_present() && self.present_queue.is_none() {
            self.present_queue = Some(index);
        }

        self.queues.push(queue);
    }

    /// Get graphics queue.
    pub fn graphics_queue(&self) -> Option<&Queue> {
        self.graphics_queue.map(|i| &self.queues[i])
    }

    /// Get compute queue.
    pub fn compute_queue(&self) -> Option<&Queue> {
        self.compute_queue.map(|i| &self.queues[i])
    }

    /// Get transfer queue.
    pub fn transfer_queue(&self) -> Option<&Queue> {
        self.transfer_queue.map(|i| &self.queues[i])
    }

    /// Get present queue.
    pub fn present_queue(&self) -> Option<&Queue> {
        self.present_queue.map(|i| &self.queues[i])
    }

    /// Get queue by handle.
    pub fn get(&self, handle: QueueHandle) -> Option<&Queue> {
        self.queues.iter().find(|q| q.handle == handle)
    }

    /// Get queue by type.
    pub fn get_by_type(&self, queue_type: QueueType) -> Option<&Queue> {
        match queue_type {
            QueueType::Graphics => self.graphics_queue(),
            QueueType::Compute => self.compute_queue(),
            QueueType::Transfer => self.transfer_queue(),
            _ => None,
        }
    }

    /// Get all queues.
    pub fn all_queues(&self) -> &[Queue] {
        &self.queues
    }

    /// Get queue count.
    pub fn queue_count(&self) -> usize {
        self.queues.len()
    }

    /// Wait for all queues to be idle.
    pub fn wait_idle(&self) {
        for queue in &self.queues {
            queue.wait_idle();
        }
    }
}

impl Default for QueueManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Submission Tracker
// ============================================================================

/// Tracks pending submissions.
pub struct SubmissionTracker {
    /// Pending submissions per queue.
    pending: Vec<VecDeque<PendingSubmission>>,
    /// Next submission ID.
    next_id: AtomicU64,
}

/// A pending submission.
#[derive(Debug)]
pub struct PendingSubmission {
    /// Submission ID.
    pub id: u64,
    /// Queue index.
    pub queue_index: usize,
    /// Fence value.
    pub fence_value: u64,
    /// Frame index.
    pub frame: u64,
}

impl SubmissionTracker {
    /// Create a new tracker.
    pub fn new(queue_count: usize) -> Self {
        let mut pending = Vec::with_capacity(queue_count);
        for _ in 0..queue_count {
            pending.push(VecDeque::new());
        }

        Self {
            pending,
            next_id: AtomicU64::new(0),
        }
    }

    /// Track a new submission.
    pub fn track(&mut self, queue_index: usize, fence_value: u64, frame: u64) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        if queue_index < self.pending.len() {
            self.pending[queue_index].push_back(PendingSubmission {
                id,
                queue_index,
                fence_value,
                frame,
            });
        }

        id
    }

    /// Remove completed submissions.
    pub fn retire_completed(
        &mut self,
        queue_index: usize,
        completed_value: u64,
    ) -> Vec<PendingSubmission> {
        let mut retired = Vec::new();

        if queue_index < self.pending.len() {
            while let Some(front) = self.pending[queue_index].front() {
                if front.fence_value <= completed_value {
                    if let Some(submission) = self.pending[queue_index].pop_front() {
                        retired.push(submission);
                    }
                } else {
                    break;
                }
            }
        }

        retired
    }

    /// Get pending count for queue.
    pub fn pending_count(&self, queue_index: usize) -> usize {
        self.pending.get(queue_index).map(|q| q.len()).unwrap_or(0)
    }

    /// Get total pending count.
    pub fn total_pending(&self) -> usize {
        self.pending.iter().map(|q| q.len()).sum()
    }
}
