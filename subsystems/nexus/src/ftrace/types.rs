//! Core ftrace types.

// ============================================================================
// CORE TYPES
// ============================================================================

/// Trace ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TraceId(pub u64);

impl TraceId {
    /// Create new trace ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Function address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FuncAddr(pub u64);

impl FuncAddr {
    /// Create new address
    #[inline(always)]
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }
}

/// CPU ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CpuId(pub u32);

impl CpuId {
    /// Create new CPU ID
    #[inline(always)]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
}

/// PID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pid(pub i32);

impl Pid {
    /// Create new PID
    #[inline(always)]
    pub const fn new(pid: i32) -> Self {
        Self(pid)
    }

    /// Kernel thread
    #[inline(always)]
    pub const fn kernel() -> Self {
        Self(0)
    }
}
