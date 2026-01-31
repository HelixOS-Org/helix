//! Severity and Priority Types
//!
//! Types for expressing importance and urgency levels.

#![allow(dead_code)]

// ============================================================================
// SEVERITY
// ============================================================================

/// Severity level (1-10)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Severity(u8);

impl Severity {
    /// Create severity (clamped to 1-10)
    #[inline]
    pub const fn new(value: u8) -> Self {
        Self(if value < 1 {
            1
        } else if value > 10 {
            10
        } else {
            value
        })
    }

    /// Minimal severity
    pub const MIN: Self = Self(1);
    /// Trace/debug level
    pub const TRACE: Self = Self(1);
    /// Debug level
    pub const DEBUG: Self = Self(2);
    /// Info level
    pub const INFO: Self = Self(3);
    /// Low severity
    pub const LOW: Self = Self(3);
    /// Notice level
    pub const NOTICE: Self = Self(4);
    /// Medium severity
    pub const MEDIUM: Self = Self(5);
    /// Warning level
    pub const WARNING: Self = Self(6);
    /// High severity
    pub const HIGH: Self = Self(7);
    /// Error level
    pub const ERROR: Self = Self(8);
    /// Critical severity
    pub const CRITICAL: Self = Self(9);
    /// Maximum severity (emergency)
    pub const MAX: Self = Self(10);

    /// Get raw value
    #[inline]
    pub const fn value(&self) -> u8 {
        self.0
    }

    /// Classification
    pub const fn classification(&self) -> SeverityClass {
        match self.0 {
            1..=2 => SeverityClass::Negligible,
            3..=4 => SeverityClass::Low,
            5..=6 => SeverityClass::Medium,
            7..=8 => SeverityClass::High,
            9..=10 => SeverityClass::Critical,
            _ => SeverityClass::Negligible,
        }
    }

    /// Is this severity actionable (requires attention)?
    #[inline]
    pub const fn is_actionable(&self) -> bool {
        self.0 >= 5
    }

    /// Is critical or higher?
    #[inline]
    pub const fn is_critical(&self) -> bool {
        self.0 >= 9
    }
}

impl Default for Severity {
    fn default() -> Self {
        Self::MEDIUM
    }
}

impl core::fmt::Display for Severity {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.classification())
    }
}

// ============================================================================
// SEVERITY CLASS
// ============================================================================

/// Severity classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SeverityClass {
    /// Barely noticeable
    Negligible,
    /// Low impact
    Low,
    /// Moderate impact
    Medium,
    /// Significant impact
    High,
    /// Severe/critical impact
    Critical,
}

impl SeverityClass {
    /// Get class name
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Negligible => "negligible",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

// ============================================================================
// PRIORITY
// ============================================================================

/// Priority level (1-10)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Priority(u8);

impl Priority {
    /// Create priority (clamped to 1-10)
    #[inline]
    pub const fn new(value: u8) -> Self {
        Self(if value < 1 {
            1
        } else if value > 10 {
            10
        } else {
            value
        })
    }

    /// Lowest priority
    pub const LOWEST: Self = Self(1);
    /// Very low priority
    pub const VERY_LOW: Self = Self(2);
    /// Low priority
    pub const LOW: Self = Self(3);
    /// Below normal priority
    pub const BELOW_NORMAL: Self = Self(4);
    /// Normal priority
    pub const NORMAL: Self = Self(5);
    /// Above normal priority
    pub const ABOVE_NORMAL: Self = Self(6);
    /// High priority
    pub const HIGH: Self = Self(7);
    /// Very high priority
    pub const VERY_HIGH: Self = Self(8);
    /// Critical priority
    pub const CRITICAL: Self = Self(9);
    /// Highest priority (emergency)
    pub const HIGHEST: Self = Self(10);

    /// Get raw value
    #[inline]
    pub const fn value(&self) -> u8 {
        self.0
    }

    /// Is high priority?
    #[inline]
    pub const fn is_high(&self) -> bool {
        self.0 >= 7
    }

    /// Is critical?
    #[inline]
    pub const fn is_critical(&self) -> bool {
        self.0 >= 9
    }
}

impl Default for Priority {
    fn default() -> Self {
        Self::NORMAL
    }
}

impl core::fmt::Display for Priority {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let name = match self.0 {
            1 => "lowest",
            2 => "very_low",
            3 => "low",
            4 => "below_normal",
            5 => "normal",
            6 => "above_normal",
            7 => "high",
            8 => "very_high",
            9 => "critical",
            10 => "highest",
            _ => "unknown",
        };
        write!(f, "{}", name)
    }
}

// ============================================================================
// URGENCY
// ============================================================================

/// Urgency level for time-sensitive decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Urgency {
    /// Can wait indefinitely
    None,
    /// Handle within hours
    Low,
    /// Handle within minutes
    Medium,
    /// Handle within seconds
    High,
    /// Handle immediately
    Critical,
}

impl Urgency {
    /// Get urgency name
    pub const fn name(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    /// Deadline in milliseconds
    pub const fn deadline_ms(&self) -> u64 {
        match self {
            Self::None => u64::MAX,
            Self::Low => 3_600_000,    // 1 hour
            Self::Medium => 60_000,     // 1 minute
            Self::High => 5_000,        // 5 seconds
            Self::Critical => 100,      // 100ms
        }
    }
}

impl Default for Urgency {
    fn default() -> Self {
        Self::Medium
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity() {
        let s = Severity::new(8);
        assert_eq!(s.classification(), SeverityClass::High);
        assert!(s.is_actionable());
    }

    #[test]
    fn test_severity_clamp() {
        let s1 = Severity::new(0);
        assert_eq!(s1.value(), 1);
        let s2 = Severity::new(15);
        assert_eq!(s2.value(), 10);
    }

    #[test]
    fn test_priority() {
        let p = Priority::new(7);
        assert!(p.is_high());
        assert!(!p.is_critical());
    }

    #[test]
    fn test_urgency() {
        assert_eq!(Urgency::Critical.deadline_ms(), 100);
        assert_eq!(Urgency::None.deadline_ms(), u64::MAX);
    }
}
