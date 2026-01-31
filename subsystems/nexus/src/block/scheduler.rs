//! I/O Scheduler Types
//!
//! I/O scheduling algorithms and request types.

/// I/O scheduler type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoScheduler {
    /// None (passthrough)
    None,
    /// Deadline
    MqDeadline,
    /// BFQ (Budget Fair Queueing)
    Bfq,
    /// Kyber
    Kyber,
    /// CFQ (legacy)
    Cfq,
    /// Noop (legacy)
    Noop,
    /// Unknown
    Unknown,
}

impl IoScheduler {
    /// Get scheduler name
    pub fn name(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::MqDeadline => "mq-deadline",
            Self::Bfq => "bfq",
            Self::Kyber => "kyber",
            Self::Cfq => "cfq",
            Self::Noop => "noop",
            Self::Unknown => "unknown",
        }
    }

    /// Best for rotational
    pub fn best_for_rotational() -> Self {
        Self::MqDeadline
    }

    /// Best for SSD
    pub fn best_for_ssd() -> Self {
        Self::None
    }

    /// Best for NVMe
    pub fn best_for_nvme() -> Self {
        Self::None
    }

    /// Is fair queueing
    pub fn is_fair(&self) -> bool {
        matches!(self, Self::Bfq | Self::Cfq)
    }
}

/// I/O request type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoRequestType {
    /// Read
    Read,
    /// Write
    Write,
    /// Flush
    Flush,
    /// Discard (TRIM)
    Discard,
    /// Write zeroes
    WriteZeroes,
    /// Zone reset
    ZoneReset,
    /// Other
    Other,
}

impl IoRequestType {
    /// Get type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Flush => "flush",
            Self::Discard => "discard",
            Self::WriteZeroes => "write_zeroes",
            Self::ZoneReset => "zone_reset",
            Self::Other => "other",
        }
    }
}
