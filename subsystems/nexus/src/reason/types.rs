//! Core types for Causal Reasoning
//!
//! This module provides fundamental identifiers for causal reasoning.

/// Causal event ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CausalEventId(pub u64);

impl CausalEventId {
    /// Create new event ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get raw value
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Causal node ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CausalNodeId(pub u64);

impl CausalNodeId {
    /// Create new node ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get raw value
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Causal edge ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CausalEdgeId(pub u64);

impl CausalEdgeId {
    /// Create new edge ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get raw value
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Chain ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChainId(pub u64);

impl ChainId {
    /// Create new chain ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get raw value
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Query ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QueryId(pub u64);

impl QueryId {
    /// Create new query ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get raw value
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}
