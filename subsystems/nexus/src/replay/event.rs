//! Replay event types and data structures.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::core::NexusTimestamp;

/// A recorded event for replay
#[derive(Debug, Clone)]
pub struct ReplayEvent {
    /// Event ID
    pub id: u64,
    /// Sequence number
    pub sequence: u64,
    /// Event type
    pub event_type: ReplayEventType,
    /// Timestamp
    pub timestamp: NexusTimestamp,
    /// CPU ID
    pub cpu: u32,
    /// Thread/task ID
    pub task: u64,
    /// Event data
    pub data: EventData,
}

impl ReplayEvent {
    /// Create a new replay event
    pub fn new(event_type: ReplayEventType, sequence: u64) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            sequence,
            event_type,
            timestamp: NexusTimestamp::now(),
            cpu: 0,
            task: 0,
            data: EventData::None,
        }
    }

    /// Set CPU
    #[inline(always)]
    pub fn with_cpu(mut self, cpu: u32) -> Self {
        self.cpu = cpu;
        self
    }

    /// Set task
    #[inline(always)]
    pub fn with_task(mut self, task: u64) -> Self {
        self.task = task;
        self
    }

    /// Set data
    #[inline(always)]
    pub fn with_data(mut self, data: EventData) -> Self {
        self.data = data;
        self
    }
}

/// Type of replay event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayEventType {
    /// Interrupt delivery
    Interrupt,
    /// System call (combined entry/exit)
    Syscall,
    /// System call entry
    SyscallEntry,
    /// System call exit
    SyscallExit,
    /// Timer tick
    Timer,
    /// I/O completion
    IoCompletion,
    /// Memory allocation
    Allocation,
    /// Memory free
    Free,
    /// Lock acquisition
    LockAcquire,
    /// Lock release
    LockRelease,
    /// Context switch
    ContextSwitch,
    /// Signal delivery
    Signal,
    /// Random number request
    Random,
    /// External input
    ExternalInput,
    /// Checkpoint marker
    Checkpoint,
}

/// Event data
#[derive(Debug, Clone)]
pub enum EventData {
    /// No data
    None,
    /// Integer value
    Int(i64),
    /// Unsigned value
    Uint(u64),
    /// Bytes
    Bytes(Vec<u8>),
    /// String
    String(String),
    /// Interrupt data
    Interrupt { vector: u8, error_code: Option<u32> },
    /// Syscall data
    Syscall {
        number: u64,
        args: [u64; 6],
        result: i64,
    },
    /// I/O data
    Io {
        operation: IoOperation,
        offset: u64,
        size: usize,
    },
    /// Random bytes
    Random(Vec<u8>),
}

/// I/O operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoOperation {
    /// Read operation
    Read,
    /// Write operation
    Write,
    /// Flush operation
    Flush,
    /// Sync operation
    Sync,
}
