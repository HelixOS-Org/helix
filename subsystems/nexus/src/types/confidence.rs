//! Confidence and Probability Types
//!
//! Types for expressing certainty and probability in cognitive decisions.

#![allow(dead_code)]

use core::cmp::Ordering;

// ============================================================================
// CONFIDENCE
// ============================================================================

/// Confidence level (0.0 to 1.0)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Confidence(f32);

impl Confidence {
    /// Create new confidence (clamped to 0.0-1.0)
    #[inline]
    pub fn new(value: f32) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    /// Zero confidence
    pub const ZERO: Self = Self(0.0);

    /// Full confidence
    pub const FULL: Self = Self(1.0);

    /// Low confidence threshold
    pub const LOW: Self = Self(0.3);

    /// Medium confidence threshold
    pub const MEDIUM: Self = Self(0.6);

    /// High confidence threshold
    pub const HIGH: Self = Self(0.85);

    /// Very high confidence threshold
    pub const VERY_HIGH: Self = Self(0.95);

    /// Get raw value
    #[inline]
    pub const fn value(&self) -> f32 {
        self.0
    }

    /// As percentage (0-100)
    #[inline]
    pub fn as_percentage(&self) -> f32 {
        self.0 * 100.0
    }

    /// Check if meets threshold
    #[inline]
    pub fn meets(&self, threshold: Self) -> bool {
        self.0 >= threshold.0
    }

    /// Combine with another confidence (multiply)
    #[inline]
    pub fn combine(&self, other: Self) -> Self {
        Self(self.0 * other.0)
    }

    /// Average with another confidence
    #[inline]
    pub fn average(&self, other: Self) -> Self {
        Self((self.0 + other.0) / 2.0)
    }

    /// Weighted average
    pub fn weighted_average(values: &[(Self, f32)]) -> Self {
        if values.is_empty() {
            return Self::ZERO;
        }
        let total_weight: f32 = values.iter().map(|(_, w)| w).sum();
        if total_weight == 0.0 {
            return Self::ZERO;
        }
        let weighted_sum: f32 = values.iter().map(|(c, w)| c.0 * w).sum();
        Self::new(weighted_sum / total_weight)
    }

    /// Confidence level classification
    pub fn level(&self) -> ConfidenceLevel {
        match self.0 {
            x if x >= 0.95 => ConfidenceLevel::VeryHigh,
            x if x >= 0.85 => ConfidenceLevel::High,
            x if x >= 0.60 => ConfidenceLevel::Medium,
            x if x >= 0.30 => ConfidenceLevel::Low,
            _ => ConfidenceLevel::VeryLow,
        }
    }

    /// Is this confidence actionable (medium or higher)?
    #[inline]
    pub fn is_actionable(&self) -> bool {
        self.0 >= 0.60
    }

    /// Decay confidence over time
    #[inline]
    pub fn decay(&self, factor: f32) -> Self {
        Self::new(self.0 * factor)
    }

    /// Boost confidence (with ceiling)
    #[inline]
    pub fn boost(&self, factor: f32) -> Self {
        Self::new(self.0 * factor)
    }
}

impl PartialOrd for Confidence {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl core::fmt::Display for Confidence {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:.1}%", self.as_percentage())
    }
}

// ============================================================================
// CONFIDENCE LEVEL
// ============================================================================

/// Confidence level classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConfidenceLevel {
    /// < 30%
    VeryLow,
    /// 30-60%
    Low,
    /// 60-85%
    Medium,
    /// 85-95%
    High,
    /// > 95%
    VeryHigh,
}

impl ConfidenceLevel {
    /// Get level name
    pub const fn name(&self) -> &'static str {
        match self {
            Self::VeryLow => "very_low",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::VeryHigh => "very_high",
        }
    }

    /// Is actionable (medium or higher)
    pub const fn is_actionable(&self) -> bool {
        matches!(self, Self::Medium | Self::High | Self::VeryHigh)
    }

    /// Convert to threshold value
    pub const fn threshold(&self) -> f32 {
        match self {
            Self::VeryLow => 0.0,
            Self::Low => 0.30,
            Self::Medium => 0.60,
            Self::High => 0.85,
            Self::VeryHigh => 0.95,
        }
    }
}

// ============================================================================
// PROBABILITY
// ============================================================================

/// Probability value (0.0 to 1.0)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Probability(f32);

impl Probability {
    /// Create new probability (clamped to 0.0-1.0)
    #[inline]
    pub fn new(value: f32) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    /// Zero probability
    pub const ZERO: Self = Self(0.0);

    /// Certain probability
    pub const CERTAIN: Self = Self(1.0);

    /// Get raw value
    #[inline]
    pub const fn value(&self) -> f32 {
        self.0
    }

    /// Complement (1 - p)
    #[inline]
    pub fn complement(&self) -> Self {
        Self(1.0 - self.0)
    }

    /// Joint probability (p1 * p2) for independent events
    #[inline]
    pub fn joint(&self, other: Self) -> Self {
        Self(self.0 * other.0)
    }

    /// Union probability (p1 + p2 - p1*p2)
    #[inline]
    pub fn union(&self, other: Self) -> Self {
        Self::new(self.0 + other.0 - self.0 * other.0)
    }

    /// Convert to percentage
    #[inline]
    pub fn as_percentage(&self) -> f32 {
        self.0 * 100.0
    }
}

impl core::fmt::Display for Probability {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:.1}%", self.as_percentage())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence() {
        let c = Confidence::new(0.75);
        assert_eq!(c.level(), ConfidenceLevel::Medium);
        assert!(c.meets(Confidence::MEDIUM));
        assert!(!c.meets(Confidence::HIGH));
    }

    #[test]
    fn test_confidence_clamp() {
        let c1 = Confidence::new(1.5);
        assert_eq!(c1.value(), 1.0);
        let c2 = Confidence::new(-0.5);
        assert_eq!(c2.value(), 0.0);
    }

    #[test]
    fn test_confidence_combine() {
        let c1 = Confidence::new(0.8);
        let c2 = Confidence::new(0.9);
        let combined = c1.combine(c2);
        assert!((combined.value() - 0.72).abs() < 0.01);
    }

    #[test]
    fn test_probability() {
        let p = Probability::new(0.3);
        let comp = p.complement();
        assert!((comp.value() - 0.7).abs() < 0.01);
    }
}
