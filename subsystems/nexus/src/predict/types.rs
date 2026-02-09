//! Core prediction types
//!
//! This module defines fundamental types for crash prediction including
//! confidence levels, prediction kinds, and trend analysis.

#![allow(dead_code)]

/// Confidence level for predictions
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct PredictionConfidence(f32);

impl PredictionConfidence {
    /// Create a new confidence value (clamped to 0.0-1.0)
    pub fn new(value: f32) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    /// Get the raw value
    #[inline(always)]
    pub fn value(&self) -> f32 {
        self.0
    }

    /// Check if confidence is above threshold
    #[inline(always)]
    pub fn is_above(&self, threshold: f32) -> bool {
        self.0 >= threshold
    }

    /// Check if this is a high confidence prediction
    #[inline(always)]
    pub fn is_high(&self) -> bool {
        self.0 >= 0.8
    }

    /// Check if this is a medium confidence prediction
    #[inline(always)]
    pub fn is_medium(&self) -> bool {
        self.0 >= 0.5 && self.0 < 0.8
    }

    /// Check if this is a low confidence prediction
    #[inline(always)]
    pub fn is_low(&self) -> bool {
        self.0 < 0.5
    }

    /// Zero confidence
    pub const ZERO: Self = Self(0.0);

    /// Full confidence
    pub const FULL: Self = Self(1.0);
}

impl Default for PredictionConfidence {
    fn default() -> Self {
        Self::ZERO
    }
}

/// Kind of predicted failure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PredictionKind {
    /// Crash (kernel panic)
    Crash,
    /// Deadlock
    Deadlock,
    /// Memory exhaustion (OOM)
    OutOfMemory,
    /// Memory leak
    MemoryLeak,
    /// CPU starvation
    CpuStarvation,
    /// I/O stall
    IoStall,
    /// Stack overflow
    StackOverflow,
    /// Livelock
    Livelock,
    /// Resource exhaustion (generic)
    ResourceExhaustion,
    /// Performance degradation
    Degradation,
    /// Data corruption
    Corruption,
    /// Security violation
    SecurityViolation,
}

impl PredictionKind {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Crash => "Crash",
            Self::Deadlock => "Deadlock",
            Self::OutOfMemory => "OOM",
            Self::MemoryLeak => "Memory Leak",
            Self::CpuStarvation => "CPU Starvation",
            Self::IoStall => "I/O Stall",
            Self::StackOverflow => "Stack Overflow",
            Self::Livelock => "Livelock",
            Self::ResourceExhaustion => "Resource Exhaustion",
            Self::Degradation => "Degradation",
            Self::Corruption => "Corruption",
            Self::SecurityViolation => "Security Violation",
        }
    }

    /// Get severity (1-10)
    pub fn severity(&self) -> u8 {
        match self {
            Self::Crash => 10,
            Self::Deadlock => 9,
            Self::OutOfMemory => 9,
            Self::StackOverflow => 9,
            Self::Corruption => 10,
            Self::SecurityViolation => 10,
            Self::MemoryLeak => 6,
            Self::CpuStarvation => 7,
            Self::IoStall => 6,
            Self::Livelock => 8,
            Self::ResourceExhaustion => 7,
            Self::Degradation => 4,
        }
    }

    /// Is this a critical prediction?
    #[inline(always)]
    pub fn is_critical(&self) -> bool {
        self.severity() >= 9
    }
}

/// Trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    /// Rapidly increasing
    RapidIncrease,
    /// Slowly increasing
    SlowIncrease,
    /// Stable
    Stable,
    /// Slowly decreasing
    SlowDecrease,
    /// Rapidly decreasing
    RapidDecrease,
}

impl Trend {
    /// Get from gradient
    pub fn from_gradient(gradient: f64) -> Self {
        if gradient > 0.1 {
            Self::RapidIncrease
        } else if gradient > 0.01 {
            Self::SlowIncrease
        } else if gradient < -0.1 {
            Self::RapidDecrease
        } else if gradient < -0.01 {
            Self::SlowDecrease
        } else {
            Self::Stable
        }
    }

    /// Is this a concerning trend?
    #[inline(always)]
    pub fn is_concerning(&self) -> bool {
        matches!(self, Self::RapidIncrease | Self::RapidDecrease)
    }
}
