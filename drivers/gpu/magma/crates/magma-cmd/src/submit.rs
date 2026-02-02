//! # Command Submission
//!
//! GPU work submission and synchronization.

use alloc::vec::Vec;

use magma_core::command::{GpuSemaphore, SubmissionToken};
use magma_core::{Error, GpuAddr, Result};

use crate::buffer::CommandBuffer;
use crate::channel::{ChannelId, GpuChannel};

// =============================================================================
// SUBMIT FLAGS
// =============================================================================

bitflags::bitflags! {
    /// Submission flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SubmitFlags: u32 {
        /// No flags
        const NONE = 0;
        /// High priority submission
        const HIGH_PRIORITY = 1 << 0;
        /// Signal fence after completion
        const SIGNAL_FENCE = 1 << 1;
        /// Generate timestamp
        const TIMESTAMP = 1 << 2;
        /// Preemptible submission
        const PREEMPTIBLE = 1 << 3;
    }
}

// =============================================================================
// SUBMISSION
// =============================================================================

/// A command submission
#[derive(Debug)]
pub struct Submission {
    /// Target channel ID
    pub channel: ChannelId,
    /// Command buffers to submit
    pub command_buffers: Vec<GpuAddr>,
    /// Semaphores to wait on
    pub wait_semaphores: Vec<GpuSemaphore>,
    /// Semaphores to signal
    pub signal_semaphores: Vec<GpuSemaphore>,
    /// Submission flags
    pub flags: SubmitFlags,
}

impl Submission {
    /// Create a new submission
    pub fn new(channel: ChannelId) -> Self {
        Self {
            channel,
            command_buffers: Vec::new(),
            wait_semaphores: Vec::new(),
            signal_semaphores: Vec::new(),
            flags: SubmitFlags::SIGNAL_FENCE,
        }
    }

    /// Add command buffer
    pub fn add_command_buffer(&mut self, addr: GpuAddr) -> &mut Self {
        self.command_buffers.push(addr);
        self
    }

    /// Add wait semaphore
    pub fn wait_on(&mut self, semaphore: GpuSemaphore) -> &mut Self {
        self.wait_semaphores.push(semaphore);
        self
    }

    /// Add signal semaphore
    pub fn signal(&mut self, semaphore: GpuSemaphore) -> &mut Self {
        self.signal_semaphores.push(semaphore);
        self
    }

    /// Set flags
    pub fn with_flags(&mut self, flags: SubmitFlags) -> &mut Self {
        self.flags = flags;
        self
    }
}

// =============================================================================
// SUBMIT BATCH
// =============================================================================

/// A batch of submissions
#[derive(Debug)]
pub struct SubmitBatch {
    /// Submissions in this batch
    submissions: Vec<Submission>,
}

impl SubmitBatch {
    /// Create new batch
    pub fn new() -> Self {
        Self {
            submissions: Vec::new(),
        }
    }

    /// Add submission
    pub fn add(&mut self, submission: Submission) {
        self.submissions.push(submission);
    }

    /// Get submissions
    pub fn submissions(&self) -> &[Submission] {
        &self.submissions
    }

    /// Take submissions
    pub fn take(self) -> Vec<Submission> {
        self.submissions
    }
}

impl Default for SubmitBatch {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// SUBMISSION RESULT
// =============================================================================

/// Result of a submission
#[derive(Debug)]
pub struct SubmissionResult {
    /// Submission token for tracking
    pub token: SubmissionToken,
    /// Fence value to wait for
    pub fence_value: u64,
}

// =============================================================================
// SUBMITTER
// =============================================================================

/// Command submitter interface
pub trait Submitter {
    /// Submit a batch of work
    fn submit(&mut self, batch: SubmitBatch) -> Result<Vec<SubmissionResult>>;

    /// Wait for a submission to complete
    fn wait(&self, token: SubmissionToken, timeout_ns: u64) -> Result<bool>;

    /// Check if submission is complete
    fn is_complete(&self, token: SubmissionToken) -> bool;

    /// Wait for all pending work
    fn flush(&mut self) -> Result<()>;

    /// Wait for GPU to be completely idle
    fn drain(&mut self) -> Result<()>;
}

// =============================================================================
// QUEUE
// =============================================================================

/// Submission queue priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QueuePriority {
    /// Low priority (background work)
    Low      = 0,
    /// Normal priority
    Normal   = 1,
    /// High priority
    High     = 2,
    /// Realtime priority
    Realtime = 3,
}

/// Submission queue
#[derive(Debug)]
pub struct SubmitQueue {
    /// Queue priority
    priority: QueuePriority,
    /// Pending submissions
    pending: Vec<Submission>,
    /// Next sequence number
    next_seq: u64,
}

impl SubmitQueue {
    /// Create new queue
    pub fn new(priority: QueuePriority) -> Self {
        Self {
            priority,
            pending: Vec::new(),
            next_seq: 1,
        }
    }

    /// Get queue priority
    pub fn priority(&self) -> QueuePriority {
        self.priority
    }

    /// Enqueue submission
    pub fn enqueue(&mut self, submission: Submission) -> u64 {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.pending.push(submission);
        seq
    }

    /// Get pending count
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Take all pending submissions
    pub fn take_pending(&mut self) -> Vec<Submission> {
        core::mem::take(&mut self.pending)
    }
}
