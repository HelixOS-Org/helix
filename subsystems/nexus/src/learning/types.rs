//! Core learning types
//!
//! This module provides fundamental types for the learning system.

/// Learning session ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionId(pub u64);

impl SessionId {
    /// Create new session ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Experience ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExperienceId(pub u64);

impl ExperienceId {
    /// Create new experience ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Rule ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleId(pub u64);

impl RuleId {
    /// Create new rule ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Hypothesis ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HypothesisId(pub u64);

impl HypothesisId {
    /// Create new hypothesis ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Timestamp (nanoseconds)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(pub u64);

impl Timestamp {
    /// Create new timestamp
    #[inline(always)]
    pub const fn new(ns: u64) -> Self {
        Self(ns)
    }
}
