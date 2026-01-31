//! Signal Core Types
//!
//! Fundamental types for signal handling.

/// Process identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcessId(pub u64);

impl ProcessId {
    /// Create a new process ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Thread identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ThreadId(pub u64);

impl ThreadId {
    /// Create a new thread ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Signal number
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignalNumber(pub u32);

impl SignalNumber {
    /// SIGHUP
    pub const SIGHUP: Self = Self(1);
    /// SIGINT
    pub const SIGINT: Self = Self(2);
    /// SIGQUIT
    pub const SIGQUIT: Self = Self(3);
    /// SIGILL
    pub const SIGILL: Self = Self(4);
    /// SIGTRAP
    pub const SIGTRAP: Self = Self(5);
    /// SIGABRT
    pub const SIGABRT: Self = Self(6);
    /// SIGBUS
    pub const SIGBUS: Self = Self(7);
    /// SIGFPE
    pub const SIGFPE: Self = Self(8);
    /// SIGKILL
    pub const SIGKILL: Self = Self(9);
    /// SIGUSR1
    pub const SIGUSR1: Self = Self(10);
    /// SIGSEGV
    pub const SIGSEGV: Self = Self(11);
    /// SIGUSR2
    pub const SIGUSR2: Self = Self(12);
    /// SIGPIPE
    pub const SIGPIPE: Self = Self(13);
    /// SIGALRM
    pub const SIGALRM: Self = Self(14);
    /// SIGTERM
    pub const SIGTERM: Self = Self(15);
    /// SIGCHLD
    pub const SIGCHLD: Self = Self(17);
    /// SIGCONT
    pub const SIGCONT: Self = Self(18);
    /// SIGSTOP
    pub const SIGSTOP: Self = Self(19);
    /// SIGTSTP
    pub const SIGTSTP: Self = Self(20);

    /// Create a new signal number
    pub const fn new(num: u32) -> Self {
        Self(num)
    }

    /// Get the raw number value
    pub const fn raw(&self) -> u32 {
        self.0
    }

    /// Get signal name
    pub fn name(&self) -> &'static str {
        match self.0 {
            1 => "SIGHUP",
            2 => "SIGINT",
            3 => "SIGQUIT",
            4 => "SIGILL",
            5 => "SIGTRAP",
            6 => "SIGABRT",
            7 => "SIGBUS",
            8 => "SIGFPE",
            9 => "SIGKILL",
            10 => "SIGUSR1",
            11 => "SIGSEGV",
            12 => "SIGUSR2",
            13 => "SIGPIPE",
            14 => "SIGALRM",
            15 => "SIGTERM",
            17 => "SIGCHLD",
            18 => "SIGCONT",
            19 => "SIGSTOP",
            20 => "SIGTSTP",
            _ => "UNKNOWN",
        }
    }

    /// Check if signal is fatal by default
    pub fn is_fatal(&self) -> bool {
        matches!(self.0, 1 | 2 | 3 | 4 | 6 | 7 | 8 | 9 | 11 | 13 | 14 | 15)
    }

    /// Check if signal can be caught
    pub fn can_catch(&self) -> bool {
        !matches!(self.0, 9 | 19) // SIGKILL and SIGSTOP cannot be caught
    }

    /// Check if signal is real-time signal
    pub fn is_realtime(&self) -> bool {
        self.0 >= 32
    }
}

/// Signal category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalCategory {
    /// Process termination signals
    Termination,
    /// Process control signals
    Control,
    /// Error/exception signals
    Error,
    /// Timer signals
    Timer,
    /// I/O signals
    Io,
    /// User-defined signals
    User,
    /// Real-time signals
    RealTime,
    /// Other/unknown
    Other,
}

impl SignalCategory {
    /// Categorize a signal number
    pub fn from_signal(sig: SignalNumber) -> Self {
        match sig.0 {
            1 | 2 | 3 | 15 => Self::Termination,
            17 | 18 | 19 | 20 => Self::Control,
            4 | 6 | 7 | 8 | 11 => Self::Error,
            14 => Self::Timer,
            5 | 13 | 29 => Self::Io,
            10 | 12 => Self::User,
            32..=64 => Self::RealTime,
            _ => Self::Other,
        }
    }
}

/// Signal action type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalAction {
    /// Default action
    Default,
    /// Ignore signal
    Ignore,
    /// Call handler function
    Handler,
    /// Use sigaction with flags
    SigAction,
    /// Signal blocked
    Blocked,
}

/// Signal delivery state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryState {
    /// Pending delivery
    Pending,
    /// Currently being delivered
    Delivering,
    /// Successfully delivered
    Delivered,
    /// Blocked (cannot be delivered)
    Blocked,
    /// Ignored
    Ignored,
    /// Coalesced with another signal
    Coalesced,
    /// Dropped (queue full)
    Dropped,
}
