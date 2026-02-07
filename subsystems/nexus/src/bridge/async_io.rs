//! # Async Intelligent I/O
//!
//! Non-blocking syscall dispatch with smart scheduling and priority management.
//! Allows the kernel to reorder, merge, and prioritize async operations
//! based on app behavior and system load.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::syscall::{SyscallId, SyscallType};

// ============================================================================
// ASYNC TYPES
// ============================================================================

/// Priority for async I/O operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AsyncPriority {
    /// Background — can be delayed indefinitely
    Background = 0,
    /// Low — best effort, no deadline
    Low = 1,
    /// Normal — standard priority
    Normal = 2,
    /// High — latency-sensitive
    High = 3,
    /// Realtime — must complete within deadline
    Realtime = 4,
}

/// Status of an async I/O request
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncStatus {
    /// Waiting in queue
    Queued,
    /// Currently being executed
    InProgress,
    /// Completed successfully
    Completed,
    /// Failed with an error
    Failed,
    /// Cancelled by the caller
    Cancelled,
}

/// An async I/O request
#[derive(Debug, Clone)]
pub struct AsyncIoRequest {
    /// Original syscall ID
    pub id: SyscallId,
    /// Syscall type
    pub syscall_type: SyscallType,
    /// Data size
    pub data_size: usize,
    /// Priority
    pub priority: AsyncPriority,
    /// Submission time
    pub submitted_at: u64,
    /// Deadline (0 = no deadline)
    pub deadline: u64,
    /// Process ID
    pub pid: u64,
    /// Whether this can be merged with other requests
    pub mergeable: bool,
    /// Callback token for completion notification
    pub callback_token: u64,
}

impl AsyncIoRequest {
    pub fn new(
        id: SyscallId,
        syscall_type: SyscallType,
        data_size: usize,
        priority: AsyncPriority,
    ) -> Self {
        Self {
            id,
            syscall_type,
            data_size,
            priority,
            submitted_at: 0,
            deadline: 0,
            pid: 0,
            mergeable: syscall_type.is_batchable(),
            callback_token: 0,
        }
    }

    pub fn with_deadline(mut self, deadline: u64) -> Self {
        self.deadline = deadline;
        self
    }

    pub fn with_pid(mut self, pid: u64) -> Self {
        self.pid = pid;
        self
    }

    pub fn with_callback(mut self, token: u64) -> Self {
        self.callback_token = token;
        self
    }

    pub fn has_deadline(&self) -> bool {
        self.deadline > 0
    }
}

/// Completion result for an async operation
#[derive(Debug, Clone)]
pub struct AsyncCompletion {
    /// The request that completed
    pub id: SyscallId,
    /// Status
    pub status: AsyncStatus,
    /// Return value (or error code)
    pub result: i64,
    /// Actual bytes transferred
    pub bytes_transferred: usize,
    /// Time spent in queue (ns)
    pub queue_time_ns: u64,
    /// Time spent executing (ns)
    pub execution_time_ns: u64,
    /// Callback token
    pub callback_token: u64,
}

impl AsyncCompletion {
    pub fn success(id: SyscallId, result: i64, bytes: usize, token: u64) -> Self {
        Self {
            id,
            status: AsyncStatus::Completed,
            result,
            bytes_transferred: bytes,
            queue_time_ns: 0,
            execution_time_ns: 0,
            callback_token: token,
        }
    }

    pub fn failure(id: SyscallId, errno: i64, token: u64) -> Self {
        Self {
            id,
            status: AsyncStatus::Failed,
            result: -errno,
            bytes_transferred: 0,
            queue_time_ns: 0,
            execution_time_ns: 0,
            callback_token: token,
        }
    }

    pub fn total_latency_ns(&self) -> u64 {
        self.queue_time_ns + self.execution_time_ns
    }
}

// ============================================================================
// ASYNC I/O ENGINE
// ============================================================================

/// Ticket for tracking an async request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AsyncTicket(u64);

/// The async I/O engine — manages submission, scheduling, and completion
/// of non-blocking I/O operations.
pub struct AsyncIoEngine {
    /// Pending requests, ordered by priority
    queues: [Vec<AsyncIoRequest>; 5], // One per priority level
    /// In-progress requests
    in_progress: BTreeMap<u64, AsyncIoRequest>,
    /// Completed requests awaiting collection
    completions: Vec<AsyncCompletion>,
    /// Maximum concurrent in-progress operations
    max_concurrency: usize,
    /// Next ticket ID
    next_ticket: u64,
    /// Ticket to request mapping
    tickets: BTreeMap<u64, (SyscallId, AsyncStatus)>,
    /// Statistics
    total_submitted: u64,
    total_completed: u64,
    total_cancelled: u64,
}

impl AsyncIoEngine {
    /// Create a new engine with max concurrency
    pub fn new(max_concurrency: usize) -> Self {
        Self {
            queues: [
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ],
            in_progress: BTreeMap::new(),
            completions: Vec::new(),
            max_concurrency,
            next_ticket: 1,
            tickets: BTreeMap::new(),
            total_submitted: 0,
            total_completed: 0,
            total_cancelled: 0,
        }
    }

    /// Submit an async request, returns a ticket for tracking
    pub fn submit(&mut self, request: AsyncIoRequest) -> AsyncTicket {
        let ticket = AsyncTicket(self.next_ticket);
        self.next_ticket += 1;

        let priority_idx = request.priority as usize;
        self.tickets
            .insert(ticket.0, (request.id, AsyncStatus::Queued));
        self.queues[priority_idx].push(request);
        self.total_submitted += 1;

        ticket
    }

    /// Get the status of a request by ticket
    pub fn status(&self, ticket: AsyncTicket) -> AsyncStatus {
        self.tickets
            .get(&ticket.0)
            .map(|(_, s)| *s)
            .unwrap_or(AsyncStatus::Failed)
    }

    /// Cancel a pending request
    pub fn cancel(&mut self, ticket: AsyncTicket) -> bool {
        if let Some((id, status)) = self.tickets.get_mut(&ticket.0) {
            if *status == AsyncStatus::Queued {
                *status = AsyncStatus::Cancelled;
                self.total_cancelled += 1;

                // Remove from queues
                for queue in &mut self.queues {
                    queue.retain(|r| r.id != *id);
                }
                return true;
            }
        }
        false
    }

    /// Schedule the next batch of requests for execution.
    /// Returns requests that should be dispatched now.
    pub fn schedule(&mut self) -> Vec<AsyncIoRequest> {
        let available_slots = self.max_concurrency.saturating_sub(self.in_progress.len());
        if available_slots == 0 {
            return Vec::new();
        }

        let mut dispatched = Vec::new();

        // Dispatch from highest priority first
        for priority in (0..5).rev() {
            while dispatched.len() < available_slots && !self.queues[priority].is_empty() {
                let request = self.queues[priority].remove(0);
                let id_key = request.id.0;

                // Update ticket status
                if let Some((_, status)) = self.tickets.get_mut(&self.find_ticket(request.id)) {
                    *status = AsyncStatus::InProgress;
                }

                self.in_progress.insert(id_key, request.clone());
                dispatched.push(request);
            }
        }

        dispatched
    }

    /// Report completion of an async operation
    pub fn complete(&mut self, id: SyscallId, result: i64, bytes: usize) {
        if let Some(request) = self.in_progress.remove(&id.0) {
            let completion = AsyncCompletion::success(id, result, bytes, request.callback_token);

            // Update ticket status
            if let Some((_, status)) = self.tickets.get_mut(&self.find_ticket(id)) {
                *status = AsyncStatus::Completed;
            }

            self.completions.push(completion);
            self.total_completed += 1;
        }
    }

    /// Report failure of an async operation
    pub fn fail(&mut self, id: SyscallId, errno: i64) {
        if let Some(request) = self.in_progress.remove(&id.0) {
            let completion = AsyncCompletion::failure(id, errno, request.callback_token);

            if let Some((_, status)) = self.tickets.get_mut(&self.find_ticket(id)) {
                *status = AsyncStatus::Failed;
            }

            self.completions.push(completion);
            self.total_completed += 1;
        }
    }

    /// Collect completed operations (drains the completion queue)
    pub fn collect_completions(&mut self) -> Vec<AsyncCompletion> {
        core::mem::take(&mut self.completions)
    }

    /// Number of pending requests across all queues
    pub fn pending_count(&self) -> usize {
        self.queues.iter().map(|q| q.len()).sum()
    }

    /// Number of in-progress requests
    pub fn in_progress_count(&self) -> usize {
        self.in_progress.len()
    }

    /// Check for deadline violations and escalate priority
    pub fn check_deadlines(&mut self, current_time: u64) -> Vec<SyscallId> {
        let mut escalated = Vec::new();

        // Check queued requests for deadline pressure
        for priority in 0..4 {
            let mut to_escalate = Vec::new();

            for (idx, req) in self.queues[priority].iter().enumerate() {
                if req.has_deadline() && current_time >= req.deadline.saturating_sub(1000) {
                    to_escalate.push(idx);
                }
            }

            // Escalate by moving to higher priority queue (reverse order to maintain indices)
            for idx in to_escalate.into_iter().rev() {
                let req = self.queues[priority].remove(idx);
                escalated.push(req.id);
                self.queues[priority + 1].push(req);
            }
        }

        escalated
    }

    /// Find ticket ID for a syscall ID
    fn find_ticket(&self, id: SyscallId) -> u64 {
        self.tickets
            .iter()
            .find(|(_, (sid, _))| *sid == id)
            .map(|(ticket, _)| *ticket)
            .unwrap_or(0)
    }
}
