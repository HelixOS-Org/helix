//! I/O request representation.

use core::sync::atomic::{AtomicU64, Ordering};

use super::types::{IoOpType, IoPriority};
use crate::core::NexusTimestamp;

// ============================================================================
// I/O REQUEST
// ============================================================================

/// I/O request
#[derive(Debug, Clone)]
pub struct IoRequest {
    /// Request ID
    pub id: u64,
    /// Operation type
    pub op_type: IoOpType,
    /// Device ID
    pub device_id: u32,
    /// Starting sector/offset
    pub offset: u64,
    /// Size in bytes
    pub size: u32,
    /// Priority
    pub priority: IoPriority,
    /// Submission timestamp
    pub submitted_at: NexusTimestamp,
    /// Completion timestamp
    pub completed_at: Option<NexusTimestamp>,
    /// Process ID that issued the request
    pub process_id: u64,
}

impl IoRequest {
    /// Create new request
    pub fn new(op_type: IoOpType, device_id: u32, offset: u64, size: u32) -> Self {
        static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

        Self {
            id: REQUEST_ID.fetch_add(1, Ordering::Relaxed),
            op_type,
            device_id,
            offset,
            size,
            priority: IoPriority::Normal,
            submitted_at: NexusTimestamp::now(),
            completed_at: None,
            process_id: 0,
        }
    }

    /// Set priority
    #[inline(always)]
    pub fn with_priority(mut self, priority: IoPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set process ID
    #[inline(always)]
    pub fn with_process(mut self, pid: u64) -> Self {
        self.process_id = pid;
        self
    }

    /// Mark as completed
    #[inline(always)]
    pub fn complete(&mut self) {
        self.completed_at = Some(NexusTimestamp::now());
    }

    /// Get latency in ticks
    #[inline(always)]
    pub fn latency(&self) -> Option<u64> {
        self.completed_at
            .map(|c| c.duration_since(self.submitted_at))
    }

    /// Is read operation?
    #[inline(always)]
    pub fn is_read(&self) -> bool {
        self.op_type == IoOpType::Read
    }

    /// Is write operation?
    #[inline(always)]
    pub fn is_write(&self) -> bool {
        self.op_type == IoOpType::Write
    }

    /// End offset
    #[inline(always)]
    pub fn end_offset(&self) -> u64 {
        self.offset + self.size as u64
    }
}
