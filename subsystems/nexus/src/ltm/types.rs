//! Core types for Long-Term Memory
//!
//! This module provides fundamental identifiers and time structures for LTM.

/// Memory ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MemoryId(pub u64);

impl MemoryId {
    /// Create new memory ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get raw value
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Episode ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EpisodeId(pub u64);

impl EpisodeId {
    /// Create new episode ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get raw value
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Pattern ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PatternId(pub u64);

impl PatternId {
    /// Create new pattern ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get raw value
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Procedure ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcedureId(pub u64);

impl ProcedureId {
    /// Create new procedure ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get raw value
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Boot ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BootId(pub u64);

impl BootId {
    /// Create new boot ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get raw value
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Timestamp (nanoseconds since epoch)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(pub u64);

impl Timestamp {
    /// Create new timestamp
    pub const fn new(ns: u64) -> Self {
        Self(ns)
    }

    /// Now (placeholder - would use real time source)
    pub fn now() -> Self {
        Self(0) // In real implementation, get current time
    }

    /// Difference in nanoseconds
    pub fn diff(&self, other: &Self) -> u64 {
        self.0.saturating_sub(other.0)
    }

    /// Add duration
    pub fn add_ns(&self, ns: u64) -> Self {
        Self(self.0.saturating_add(ns))
    }

    /// As seconds
    pub fn as_secs(&self) -> u64 {
        self.0 / 1_000_000_000
    }

    /// As milliseconds
    pub fn as_millis(&self) -> u64 {
        self.0 / 1_000_000
    }

    /// Get raw nanoseconds
    pub const fn as_nanos(&self) -> u64 {
        self.0
    }
}

/// Time range
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeRange {
    /// Start timestamp
    pub start: Timestamp,
    /// End timestamp
    pub end: Timestamp,
}

impl TimeRange {
    /// Create new range
    pub const fn new(start: Timestamp, end: Timestamp) -> Self {
        Self { start, end }
    }

    /// Duration in nanoseconds
    pub fn duration_ns(&self) -> u64 {
        self.end.0.saturating_sub(self.start.0)
    }

    /// Contains timestamp
    pub fn contains(&self, ts: Timestamp) -> bool {
        ts >= self.start && ts <= self.end
    }

    /// Overlaps with another range
    pub fn overlaps(&self, other: &Self) -> bool {
        self.start <= other.end && self.end >= other.start
    }
}
