//! Core trace types

#![allow(dead_code)]

use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// TRACE ID
// ============================================================================

/// Unique trace ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TraceId(pub u64);

impl TraceId {
    /// Generate a new trace ID
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }

    /// Get raw value
    #[inline(always)]
    pub fn raw(&self) -> u64 {
        self.0
    }
}

impl Default for TraceId {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// SPAN ID
// ============================================================================

/// Unique span ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpanId(pub u64);

impl SpanId {
    /// Generate a new span ID
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }

    /// Get raw value
    #[inline(always)]
    pub fn raw(&self) -> u64 {
        self.0
    }

    /// Root span ID
    pub const ROOT: Self = Self(0);
}

impl Default for SpanId {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TRACE LEVEL
// ============================================================================

/// Trace level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TraceLevel {
    /// Error level
    Error = 0,
    /// Warning level
    Warn  = 1,
    /// Info level
    Info  = 2,
    /// Debug level
    Debug = 3,
    /// Trace level (most verbose)
    Trace = 4,
}

impl TraceLevel {
    /// Get from numeric value
    #[inline]
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Error,
            1 => Self::Warn,
            2 => Self::Info,
            3 => Self::Debug,
            _ => Self::Trace,
        }
    }

    /// Get name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Error => "ERROR",
            Self::Warn => "WARN",
            Self::Info => "INFO",
            Self::Debug => "DEBUG",
            Self::Trace => "TRACE",
        }
    }
}
