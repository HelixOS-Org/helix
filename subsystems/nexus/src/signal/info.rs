//! Signal Information Types
//!
//! Signal info and pending signal structures.

use super::{DeliveryState, ProcessId, SignalNumber, ThreadId};

/// Signal information
#[derive(Debug, Clone)]
pub struct SignalInfo {
    /// Signal number
    pub signo: SignalNumber,
    /// Error number
    pub errno: i32,
    /// Signal code
    pub code: i32,
    /// Sending process ID
    pub sender_pid: ProcessId,
    /// Sending user ID
    pub sender_uid: u32,
    /// Signal value (for realtime signals)
    pub value: i64,
    /// Timestamp (nanoseconds)
    pub timestamp: u64,
    /// Fault address (for SIGSEGV, SIGBUS)
    pub fault_addr: Option<u64>,
}

impl SignalInfo {
    /// Create new signal info
    pub fn new(signo: SignalNumber, sender_pid: ProcessId, timestamp: u64) -> Self {
        Self {
            signo,
            errno: 0,
            code: 0,
            sender_pid,
            sender_uid: 0,
            value: 0,
            timestamp,
            fault_addr: None,
        }
    }

    /// Set error number
    #[inline(always)]
    pub fn with_errno(mut self, errno: i32) -> Self {
        self.errno = errno;
        self
    }

    /// Set signal code
    #[inline(always)]
    pub fn with_code(mut self, code: i32) -> Self {
        self.code = code;
        self
    }

    /// Set signal value
    #[inline(always)]
    pub fn with_value(mut self, value: i64) -> Self {
        self.value = value;
        self
    }

    /// Set fault address
    #[inline(always)]
    pub fn with_fault_addr(mut self, addr: u64) -> Self {
        self.fault_addr = Some(addr);
        self
    }
}

/// Pending signal entry
#[derive(Debug, Clone)]
pub struct PendingSignal {
    /// Signal info
    pub info: SignalInfo,
    /// Target process
    pub target_pid: ProcessId,
    /// Target thread (None for process-directed)
    pub target_tid: Option<ThreadId>,
    /// Current state
    pub state: DeliveryState,
    /// Delivery attempts
    pub delivery_attempts: u32,
    /// Maximum delivery attempts
    pub max_attempts: u32,
}

impl PendingSignal {
    /// Create new pending signal
    pub fn new(info: SignalInfo, target_pid: ProcessId) -> Self {
        Self {
            info,
            target_pid,
            target_tid: None,
            state: DeliveryState::Pending,
            delivery_attempts: 0,
            max_attempts: 3,
        }
    }

    /// Set target thread
    #[inline(always)]
    pub fn with_target_thread(mut self, tid: ThreadId) -> Self {
        self.target_tid = Some(tid);
        self
    }

    /// Check if signal can be retried
    #[inline(always)]
    pub fn can_retry(&self) -> bool {
        self.delivery_attempts < self.max_attempts
    }

    /// Increment delivery attempt
    #[inline(always)]
    pub fn increment_attempt(&mut self) {
        self.delivery_attempts += 1;
    }
}
