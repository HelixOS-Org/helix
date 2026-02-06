//! # Thread States
//!
//! Thread state machine definition.

/// Thread state (unit-only for atomic storage)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
#[derive(Default)]
pub enum ThreadState {
    /// Thread is being created
    #[default]
    Creating = 0,
    /// Thread is ready to run
    Ready    = 1,
    /// Thread is currently running
    Running  = 2,
    /// Thread is blocked waiting for something
    Blocked  = 3,
    /// Thread is sleeping
    Sleeping = 4,
    /// Thread is stopped (by signal or debugger)
    Stopped  = 5,
    /// Thread is dead (waiting to be cleaned up)
    Dead     = 6,
    /// Thread is a zombie (terminated but not reaped)
    Zombie   = 7,
}

impl ThreadState {
    /// Convert from u32
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Creating),
            1 => Some(Self::Ready),
            2 => Some(Self::Running),
            3 => Some(Self::Blocked),
            4 => Some(Self::Sleeping),
            5 => Some(Self::Stopped),
            6 => Some(Self::Dead),
            7 => Some(Self::Zombie),
            _ => None,
        }
    }

    /// Convert to u32
    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

/// Reason for blocking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockReason {
    /// Waiting for I/O
    Io,
    /// Waiting for a lock
    Lock,
    /// Waiting for a signal
    Signal,
    /// Waiting for a futex
    Futex,
    /// Waiting for IPC
    Ipc,
    /// Waiting for memory
    Memory,
    /// Waiting for child process
    Child,
    /// Waiting for timer
    Timer,
    /// Other/unspecified
    Other,
}

impl ThreadState {
    /// Check if thread can be scheduled
    pub fn is_runnable(&self) -> bool {
        matches!(self, ThreadState::Ready | ThreadState::Running)
    }

    /// Check if thread is blocked
    pub fn is_blocked(&self) -> bool {
        matches!(self, ThreadState::Blocked | ThreadState::Sleeping)
    }

    /// Check if thread is terminated
    pub fn is_terminated(&self) -> bool {
        matches!(self, ThreadState::Dead | ThreadState::Zombie)
    }

    /// Valid transitions from this state
    pub fn valid_transitions(&self) -> &[ThreadState] {
        match self {
            ThreadState::Creating => &[ThreadState::Ready, ThreadState::Dead],
            ThreadState::Ready => &[ThreadState::Running, ThreadState::Dead],
            ThreadState::Running => &[
                ThreadState::Ready,
                ThreadState::Blocked,
                ThreadState::Sleeping,
                ThreadState::Stopped,
                ThreadState::Dead,
                ThreadState::Zombie,
            ],
            ThreadState::Blocked => &[ThreadState::Ready, ThreadState::Dead],
            ThreadState::Sleeping => &[ThreadState::Ready, ThreadState::Dead],
            ThreadState::Stopped => &[ThreadState::Ready, ThreadState::Dead],
            ThreadState::Dead => &[],
            ThreadState::Zombie => &[ThreadState::Dead],
        }
    }
}
