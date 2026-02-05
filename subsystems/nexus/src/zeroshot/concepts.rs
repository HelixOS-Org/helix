//! Kernel concept definitions for zero-shot learning.

/// Kernel concept class
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KernelConcept {
    /// Process states
    ProcessState(ProcessStateClass),
    /// Memory patterns
    MemoryPattern(MemoryPatternClass),
    /// I/O patterns
    IoPattern(IoPatternClass),
    /// Error types
    ErrorType(ErrorTypeClass),
    /// Security events
    SecurityEvent(SecurityEventClass),
    /// Performance anomalies
    Anomaly(AnomalyClass),
    /// Unknown (to be classified)
    Unknown(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcessStateClass {
    Running,
    Blocked,
    Waiting,
    Zombie,
    Sleeping,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryPatternClass {
    Sequential,
    Random,
    Strided,
    Hot,
    Cold,
    Thrashing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IoPatternClass {
    Bursty,
    Steady,
    Idle,
    Saturated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorTypeClass {
    PageFault,
    SegmentationFault,
    Overflow,
    Underflow,
    Timeout,
    ResourceExhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SecurityEventClass {
    NormalAccess,
    SuspiciousAccess,
    PrivilegeEscalation,
    BufferOverflow,
    IntrusionAttempt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnomalyClass {
    Normal,
    MildAnomaly,
    SevereAnomaly,
    Critical,
}
