//! Temporal Types for NEXUS
//!
//! Time-related types: timestamps, durations, and time ranges.

#![allow(dead_code)]

// ============================================================================
// TIMESTAMP
// ============================================================================

/// Timestamp in nanoseconds since boot
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct Timestamp(pub u64);

impl Timestamp {
    /// Create timestamp
    #[inline]
    pub const fn new(ns: u64) -> Self {
        Self(ns)
    }

    /// Zero timestamp (boot time)
    pub const ZERO: Self = Self(0);

    /// Maximum timestamp
    pub const MAX: Self = Self(u64::MAX);

    /// Current time (placeholder - would use real clock)
    #[inline(always)]
    pub fn now() -> Self {
        // In real implementation: read from kernel clock
        Self(0)
    }

    /// As nanoseconds
    #[inline]
    pub const fn as_nanos(&self) -> u64 {
        self.0
    }

    /// As microseconds
    #[inline]
    pub const fn as_micros(&self) -> u64 {
        self.0 / 1_000
    }

    /// As milliseconds
    #[inline]
    pub const fn as_millis(&self) -> u64 {
        self.0 / 1_000_000
    }

    /// As seconds
    #[inline]
    pub const fn as_secs(&self) -> u64 {
        self.0 / 1_000_000_000
    }

    /// Difference from another timestamp
    #[inline]
    pub const fn elapsed_since(&self, earlier: Self) -> Duration {
        Duration::from_nanos(self.0.saturating_sub(earlier.0))
    }

    /// Add duration
    #[inline]
    pub const fn add(&self, duration: Duration) -> Self {
        Self(self.0.saturating_add(duration.0))
    }

    /// Subtract duration
    #[inline]
    pub const fn sub(&self, duration: Duration) -> Self {
        Self(self.0.saturating_sub(duration.0))
    }
}

impl core::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let secs = self.as_secs();
        let millis = (self.0 / 1_000_000) % 1000;
        write!(f, "{}.{:03}s", secs, millis)
    }
}

// ============================================================================
// DURATION
// ============================================================================

/// Duration in nanoseconds
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct Duration(pub u64);

impl Duration {
    /// Create from nanoseconds
    #[inline]
    pub const fn from_nanos(ns: u64) -> Self {
        Self(ns)
    }

    /// Create from microseconds
    #[inline]
    pub const fn from_micros(us: u64) -> Self {
        Self(us.saturating_mul(1_000))
    }

    /// Create from milliseconds
    #[inline]
    pub const fn from_millis(ms: u64) -> Self {
        Self(ms.saturating_mul(1_000_000))
    }

    /// Create from seconds
    #[inline]
    pub const fn from_secs(secs: u64) -> Self {
        Self(secs.saturating_mul(1_000_000_000))
    }

    /// Zero duration
    pub const ZERO: Self = Self(0);

    /// Maximum duration
    pub const MAX: Self = Self(u64::MAX);

    /// One nanosecond
    pub const NANOSECOND: Self = Self(1);

    /// One microsecond
    pub const MICROSECOND: Self = Self(1_000);

    /// One millisecond
    pub const MILLISECOND: Self = Self(1_000_000);

    /// One second
    pub const SECOND: Self = Self(1_000_000_000);

    /// One minute
    pub const MINUTE: Self = Self(60_000_000_000);

    /// One hour
    pub const HOUR: Self = Self(3_600_000_000_000);

    /// As nanoseconds
    #[inline]
    pub const fn as_nanos(&self) -> u64 {
        self.0
    }

    /// As microseconds
    #[inline]
    pub const fn as_micros(&self) -> u64 {
        self.0 / 1_000
    }

    /// As milliseconds
    #[inline]
    pub const fn as_millis(&self) -> u64 {
        self.0 / 1_000_000
    }

    /// As seconds (f32)
    #[inline]
    pub fn as_secs_f32(&self) -> f32 {
        self.0 as f32 / 1_000_000_000.0
    }

    /// Is zero
    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.0 == 0
    }

    /// Saturating add
    #[inline]
    pub const fn saturating_add(&self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    /// Saturating sub
    #[inline]
    pub const fn saturating_sub(&self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }
}

impl core::fmt::Display for Duration {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.0 >= 1_000_000_000 {
            write!(f, "{:.3}s", self.as_secs_f32())
        } else if self.0 >= 1_000_000 {
            write!(f, "{}ms", self.as_millis())
        } else if self.0 >= 1_000 {
            write!(f, "{}µs", self.as_micros())
        } else {
            write!(f, "{}ns", self.0)
        }
    }
}

// ============================================================================
// TIME RANGE
// ============================================================================

/// Time range (inclusive)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeRange {
    /// Start timestamp
    pub start: Timestamp,
    /// End timestamp
    pub end: Timestamp,
}

impl TimeRange {
    /// Create new range
    #[inline]
    pub const fn new(start: Timestamp, end: Timestamp) -> Self {
        Self { start, end }
    }

    /// Create range starting from timestamp with duration
    #[inline]
    pub const fn from_start(start: Timestamp, duration: Duration) -> Self {
        Self {
            start,
            end: start.add(duration),
        }
    }

    /// Duration of the range
    #[inline]
    pub const fn duration(&self) -> Duration {
        self.end.elapsed_since(self.start)
    }

    /// Check if timestamp is within range
    #[inline]
    pub const fn contains(&self, ts: Timestamp) -> bool {
        ts.0 >= self.start.0 && ts.0 <= self.end.0
    }

    /// Check if ranges overlap
    #[inline]
    pub const fn overlaps(&self, other: &Self) -> bool {
        self.start.0 <= other.end.0 && self.end.0 >= other.start.0
    }

    /// Merge with another range (union)
    #[inline]
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            start: Timestamp::new(self.start.0.min(other.start.0)),
            end: Timestamp::new(self.end.0.max(other.end.0)),
        }
    }

    /// Intersection with another range
    #[inline]
    pub fn intersect(&self, other: &Self) -> Option<Self> {
        if !self.overlaps(other) {
            return None;
        }
        Some(Self {
            start: Timestamp::new(self.start.0.max(other.start.0)),
            end: Timestamp::new(self.end.0.min(other.end.0)),
        })
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp() {
        let ts = Timestamp::new(1_500_000_000);
        assert_eq!(ts.as_millis(), 1500);
        assert_eq!(ts.as_secs(), 1);
    }

    #[test]
    fn test_duration() {
        let d = Duration::from_millis(1500);
        assert_eq!(d.as_millis(), 1500);
        assert_eq!(d.as_micros(), 1_500_000);
    }

    #[test]
    fn test_time_range() {
        let r1 = TimeRange::new(Timestamp::new(0), Timestamp::new(100));
        let r2 = TimeRange::new(Timestamp::new(50), Timestamp::new(150));
        assert!(r1.overlaps(&r2));
        assert!(r1.contains(Timestamp::new(50)));
        assert!(!r1.contains(Timestamp::new(150)));
    }

    #[test]
    fn test_time_range_merge() {
        let r1 = TimeRange::new(Timestamp::new(0), Timestamp::new(100));
        let r2 = TimeRange::new(Timestamp::new(50), Timestamp::new(150));
        let merged = r1.merge(&r2);
        assert_eq!(merged.start.0, 0);
        assert_eq!(merged.end.0, 150);
    }
}
