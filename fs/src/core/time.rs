//! Time handling for HelixFS.
//!
//! This module provides timestamp types and utilities for filesystem
//! time tracking, including creation, modification, and access times.

use core::fmt;
use core::ops::{Add, Sub};

#[cfg(feature = "alloc")]
extern crate alloc as alloc_crate;

/// Nanoseconds per second
pub const NSEC_PER_SEC: u64 = 1_000_000_000;

/// Nanoseconds per millisecond
pub const NSEC_PER_MSEC: u64 = 1_000_000;

/// Nanoseconds per microsecond
pub const NSEC_PER_USEC: u64 = 1_000;

/// High-precision timestamp (nanoseconds since Unix epoch).
///
/// This provides nanosecond precision timestamps suitable for modern
/// filesystems. The 64-bit value can represent times until year 2554.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct Timestamp(pub u64);

impl Timestamp {
    /// Unix epoch (January 1, 1970 00:00:00 UTC)
    pub const EPOCH: Self = Self(0);

    /// Maximum representable timestamp
    pub const MAX: Self = Self(u64::MAX);

    /// Invalid timestamp marker
    pub const INVALID: Self = Self(u64::MAX);

    /// Create timestamp from nanoseconds since epoch
    #[inline]
    pub const fn from_nanos(nanos: u64) -> Self {
        Self(nanos)
    }

    /// Create timestamp from seconds since epoch
    #[inline]
    pub const fn from_secs(secs: u64) -> Self {
        Self(secs * NSEC_PER_SEC)
    }

    /// Create timestamp from seconds and nanoseconds
    #[inline]
    pub const fn from_secs_nanos(secs: u64, nanos: u32) -> Self {
        Self(secs * NSEC_PER_SEC + nanos as u64)
    }

    /// Create timestamp from milliseconds since epoch
    #[inline]
    pub const fn from_millis(millis: u64) -> Self {
        Self(millis * NSEC_PER_MSEC)
    }

    /// Get raw nanosecond value
    #[inline]
    pub const fn as_nanos(self) -> u64 {
        self.0
    }

    /// Get seconds component
    #[inline]
    pub const fn secs(self) -> u64 {
        self.0 / NSEC_PER_SEC
    }

    /// Get nanoseconds component (0-999999999)
    #[inline]
    pub const fn subsec_nanos(self) -> u32 {
        (self.0 % NSEC_PER_SEC) as u32
    }

    /// Get milliseconds component
    #[inline]
    pub const fn as_millis(self) -> u64 {
        self.0 / NSEC_PER_MSEC
    }

    /// Get microseconds component
    #[inline]
    pub const fn as_micros(self) -> u64 {
        self.0 / NSEC_PER_USEC
    }

    /// Check if this is a valid timestamp
    #[inline]
    pub const fn is_valid(self) -> bool {
        self.0 != u64::MAX
    }

    /// Check if this is the epoch (zero)
    #[inline]
    pub const fn is_epoch(self) -> bool {
        self.0 == 0
    }

    /// Duration since another timestamp
    #[inline]
    pub const fn duration_since(self, earlier: Self) -> Duration {
        Duration::from_nanos(self.0.saturating_sub(earlier.0))
    }

    /// Add a duration
    #[inline]
    pub const fn add_duration(self, duration: Duration) -> Self {
        Self(self.0.saturating_add(duration.as_nanos()))
    }

    /// Subtract a duration
    #[inline]
    pub const fn sub_duration(self, duration: Duration) -> Self {
        Self(self.0.saturating_sub(duration.as_nanos()))
    }

    /// Get maximum of two timestamps
    #[inline]
    pub const fn max(self, other: Self) -> Self {
        if self.0 > other.0 {
            self
        } else {
            other
        }
    }

    /// Get minimum of two timestamps
    #[inline]
    pub const fn min(self, other: Self) -> Self {
        if self.0 < other.0 {
            self
        } else {
            other
        }
    }
}

impl fmt::Debug for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Timestamp({}.{:09}s)", self.secs(), self.subsec_nanos())
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Basic ISO-8601 like format (simplified, no leap seconds)
        let total_secs = self.secs();
        let nanos = self.subsec_nanos();

        // Break down into date/time components
        let days_since_epoch = total_secs / 86400;
        let secs_in_day = total_secs % 86400;

        let hours = secs_in_day / 3600;
        let minutes = (secs_in_day % 3600) / 60;
        let seconds = secs_in_day % 60;

        // Simplified year calculation (doesn't account for all leap years)
        let mut year = 1970;
        let mut remaining_days = days_since_epoch as i64;

        while remaining_days >= 365 {
            let days_in_year = if is_leap_year(year) { 366 } else { 365 };
            if remaining_days >= days_in_year {
                remaining_days -= days_in_year;
                year += 1;
            } else {
                break;
            }
        }

        // Month and day
        let mut month = 1;
        let days_in_months = if is_leap_year(year) {
            [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        } else {
            [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        };

        for days_in_month in days_in_months.iter() {
            if remaining_days >= *days_in_month as i64 {
                remaining_days -= *days_in_month as i64;
                month += 1;
            } else {
                break;
            }
        }

        let day = remaining_days + 1;

        write!(
            f,
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:09}Z",
            year, month, day, hours, minutes, seconds, nanos
        )
    }
}

impl Add<Duration> for Timestamp {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Duration) -> Self {
        self.add_duration(rhs)
    }
}

impl Sub<Duration> for Timestamp {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Duration) -> Self {
        self.sub_duration(rhs)
    }
}

impl Sub<Timestamp> for Timestamp {
    type Output = Duration;
    #[inline]
    fn sub(self, rhs: Timestamp) -> Duration {
        self.duration_since(rhs)
    }
}

/// Duration type for time intervals.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct Duration(u64);

impl Duration {
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
    pub const MINUTE: Self = Self(60 * 1_000_000_000);

    /// One hour
    pub const HOUR: Self = Self(3600 * 1_000_000_000);

    /// One day
    pub const DAY: Self = Self(86400 * 1_000_000_000);

    /// Create from nanoseconds
    #[inline]
    pub const fn from_nanos(nanos: u64) -> Self {
        Self(nanos)
    }

    /// Create from microseconds
    #[inline]
    pub const fn from_micros(micros: u64) -> Self {
        Self(micros * NSEC_PER_USEC)
    }

    /// Create from milliseconds
    #[inline]
    pub const fn from_millis(millis: u64) -> Self {
        Self(millis * NSEC_PER_MSEC)
    }

    /// Create from seconds
    #[inline]
    pub const fn from_secs(secs: u64) -> Self {
        Self(secs * NSEC_PER_SEC)
    }

    /// Create from seconds and nanoseconds
    #[inline]
    pub const fn from_secs_nanos(secs: u64, nanos: u32) -> Self {
        Self(secs * NSEC_PER_SEC + nanos as u64)
    }

    /// Get total nanoseconds
    #[inline]
    pub const fn as_nanos(self) -> u64 {
        self.0
    }

    /// Get total microseconds
    #[inline]
    pub const fn as_micros(self) -> u64 {
        self.0 / NSEC_PER_USEC
    }

    /// Get total milliseconds
    #[inline]
    pub const fn as_millis(self) -> u64 {
        self.0 / NSEC_PER_MSEC
    }

    /// Get whole seconds
    #[inline]
    pub const fn as_secs(self) -> u64 {
        self.0 / NSEC_PER_SEC
    }

    /// Get subsecond nanoseconds (0-999999999)
    #[inline]
    pub const fn subsec_nanos(self) -> u32 {
        (self.0 % NSEC_PER_SEC) as u32
    }

    /// Get as floating point seconds
    #[inline]
    pub fn as_secs_f64(self) -> f64 {
        self.0 as f64 / NSEC_PER_SEC as f64
    }

    /// Check if duration is zero
    #[inline]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Saturating addition
    #[inline]
    pub const fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    /// Saturating subtraction
    #[inline]
    pub const fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    /// Saturating multiplication
    #[inline]
    pub const fn saturating_mul(self, rhs: u64) -> Self {
        Self(self.0.saturating_mul(rhs))
    }

    /// Checked division
    #[inline]
    pub const fn checked_div(self, rhs: u64) -> Option<Self> {
        match self.0.checked_div(rhs) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }
}

impl fmt::Debug for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Duration({}.{:09}s)",
            self.as_secs(),
            self.subsec_nanos()
        )
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let secs = self.as_secs();
        let nanos = self.subsec_nanos();

        if secs == 0 {
            if nanos < 1_000 {
                write!(f, "{}ns", nanos)
            } else if nanos < 1_000_000 {
                write!(f, "{:.3}µs", nanos as f64 / 1_000.0)
            } else {
                write!(f, "{:.3}ms", nanos as f64 / 1_000_000.0)
            }
        } else if secs < 60 {
            write!(f, "{:.3}s", self.as_secs_f64())
        } else if secs < 3600 {
            write!(f, "{}m {}s", secs / 60, secs % 60)
        } else if secs < 86400 {
            write!(f, "{}h {}m", secs / 3600, (secs % 3600) / 60)
        } else {
            write!(f, "{}d {}h", secs / 86400, (secs % 86400) / 3600)
        }
    }
}

impl Add for Duration {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Duration {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

/// Check if a year is a leap year
#[inline]
const fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// File timestamps structure.
///
/// Contains all timestamp fields for a file/directory.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct FileTime {
    /// Access time (when file was last read)
    pub atime: Timestamp,
    /// Modification time (when content was last changed)
    pub mtime: Timestamp,
    /// Change time (when metadata was last changed)
    pub ctime: Timestamp,
    /// Creation time (when file was created)
    pub crtime: Timestamp,
}

impl FileTime {
    /// Create with all timestamps set to the given value
    #[inline]
    pub const fn all(ts: Timestamp) -> Self {
        Self {
            atime: ts,
            mtime: ts,
            ctime: ts,
            crtime: ts,
        }
    }

    /// Create with current time (requires external time source)
    #[inline]
    pub const fn from_now(now: Timestamp) -> Self {
        Self::all(now)
    }

    /// Update access time
    #[inline]
    pub fn touch_atime(&mut self, ts: Timestamp) {
        self.atime = ts;
    }

    /// Update modification time (also updates ctime)
    #[inline]
    pub fn touch_mtime(&mut self, ts: Timestamp) {
        self.mtime = ts;
        self.ctime = ts;
    }

    /// Update change time only
    #[inline]
    pub fn touch_ctime(&mut self, ts: Timestamp) {
        self.ctime = ts;
    }

    /// Get the most recent modification
    #[inline]
    pub fn latest(&self) -> Timestamp {
        self.atime.max(self.mtime).max(self.ctime)
    }
}

/// Clock source trait for obtaining current time.
///
/// Implementations must be provided by the kernel/runtime environment.
pub trait ClockSource {
    /// Get current timestamp
    fn now(&self) -> Timestamp;

    /// Get monotonic time (for measuring durations)
    fn monotonic(&self) -> Timestamp;
}

/// Dummy clock source for testing (always returns epoch)
#[derive(Clone, Copy, Debug, Default)]
pub struct DummyClock;

impl ClockSource for DummyClock {
    fn now(&self) -> Timestamp {
        Timestamp::EPOCH
    }

    fn monotonic(&self) -> Timestamp {
        Timestamp::EPOCH
    }
}

/// Simple counter-based clock for testing
#[derive(Debug)]
pub struct CounterClock {
    counter: core::sync::atomic::AtomicU64,
}

impl CounterClock {
    /// Create new counter clock
    pub const fn new() -> Self {
        Self {
            counter: core::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Advance time by given nanoseconds
    pub fn advance(&self, nanos: u64) {
        self.counter
            .fetch_add(nanos, core::sync::atomic::Ordering::SeqCst);
    }

    /// Set time to specific value
    pub fn set(&self, nanos: u64) {
        self.counter
            .store(nanos, core::sync::atomic::Ordering::SeqCst);
    }
}

impl ClockSource for CounterClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_nanos(self.counter.load(core::sync::atomic::Ordering::SeqCst))
    }

    fn monotonic(&self) -> Timestamp {
        self.now()
    }
}

impl Default for CounterClock {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Timer Wheel for Timeouts
// ============================================================================

/// Hierarchical timer wheel for efficient timeout management.
///
/// Uses a multi-level wheel structure for O(1) insertion and
/// amortized O(1) expiration handling.
#[cfg(feature = "alloc")]
pub struct TimerWheel<T> {
    /// Wheel levels (millisecond, second, minute, hour)
    wheels: [TimerWheelLevel<T>; 4],
    /// Current time tick
    current_tick: u64,
    /// Tick duration in nanoseconds
    tick_ns: u64,
}

#[cfg(feature = "alloc")]
struct TimerWheelLevel<T> {
    slots: alloc_crate::vec::Vec<alloc_crate::vec::Vec<TimerEntry<T>>>,
    current_slot: usize,
}

#[cfg(feature = "alloc")]
struct TimerEntry<T> {
    deadline: u64,
    data: T,
}

#[cfg(feature = "alloc")]
impl<T> TimerWheel<T> {
    /// Wheel sizes (256 slots each)
    const WHEEL_SIZE: usize = 256;

    /// Create new timer wheel
    pub fn new(tick_ns: u64) -> Self {
        Self {
            wheels: [
                TimerWheelLevel::new(Self::WHEEL_SIZE),
                TimerWheelLevel::new(Self::WHEEL_SIZE),
                TimerWheelLevel::new(Self::WHEEL_SIZE),
                TimerWheelLevel::new(Self::WHEEL_SIZE),
            ],
            current_tick: 0,
            tick_ns,
        }
    }

    /// Insert a timer
    pub fn insert(&mut self, deadline: Timestamp, data: T) {
        let deadline_tick = deadline.as_nanos() / self.tick_ns;
        let delta = deadline_tick.saturating_sub(self.current_tick);

        // Determine which wheel level
        let (level, slot) = if delta < Self::WHEEL_SIZE as u64 {
            (
                0,
                (self.wheels[0].current_slot + delta as usize) % Self::WHEEL_SIZE,
            )
        } else if delta < (Self::WHEEL_SIZE * Self::WHEEL_SIZE) as u64 {
            (
                1,
                ((delta >> 8) as usize + self.wheels[1].current_slot) % Self::WHEEL_SIZE,
            )
        } else if delta < (Self::WHEEL_SIZE * Self::WHEEL_SIZE * Self::WHEEL_SIZE) as u64 {
            (
                2,
                ((delta >> 16) as usize + self.wheels[2].current_slot) % Self::WHEEL_SIZE,
            )
        } else {
            (
                3,
                ((delta >> 24) as usize + self.wheels[3].current_slot) % Self::WHEEL_SIZE,
            )
        };

        self.wheels[level].slots[slot].push(TimerEntry {
            deadline: deadline_tick,
            data,
        });
    }

    /// Advance time and return expired timers
    pub fn advance(&mut self, to: Timestamp) -> alloc_crate::vec::Vec<T> {
        let target_tick = to.as_nanos() / self.tick_ns;
        let mut expired = alloc_crate::vec::Vec::new();
        let mut reinsert = alloc_crate::vec::Vec::new();

        while self.current_tick < target_tick {
            self.current_tick += 1;

            // Process level 0
            let slot = self.wheels[0].current_slot;
            for entry in self.wheels[0].slots[slot].drain(..) {
                if entry.deadline <= self.current_tick {
                    expired.push(entry.data);
                } else {
                    // Collect for re-insertion later
                    reinsert.push((
                        Timestamp::from_nanos(entry.deadline * self.tick_ns),
                        entry.data,
                    ));
                }
            }
            self.wheels[0].current_slot = (slot + 1) % Self::WHEEL_SIZE;

            // Cascade higher levels when level 0 wraps
            if self.wheels[0].current_slot == 0 {
                self.cascade_level(1);
            }
        }

        // Re-insert collected entries
        for (deadline, data) in reinsert {
            self.insert(deadline, data);
        }

        expired
    }

    fn cascade_level(&mut self, level: usize) {
        if level >= 4 {
            return;
        }

        let slot = self.wheels[level].current_slot;
        let entries: alloc_crate::vec::Vec<_> = self.wheels[level].slots[slot].drain(..).collect();

        for entry in entries {
            // Re-insert into lower level
            let delta = entry.deadline.saturating_sub(self.current_tick);
            let new_level = if delta < Self::WHEEL_SIZE as u64 {
                0
            } else {
                level - 1
            };
            let new_slot = if new_level == 0 {
                (self.wheels[0].current_slot + delta as usize) % Self::WHEEL_SIZE
            } else {
                ((delta >> (8 * new_level)) as usize + self.wheels[new_level].current_slot)
                    % Self::WHEEL_SIZE
            };

            self.wheels[new_level].slots[new_slot].push(entry);
        }

        self.wheels[level].current_slot = (slot + 1) % Self::WHEEL_SIZE;

        if self.wheels[level].current_slot == 0 && level < 3 {
            self.cascade_level(level + 1);
        }
    }
}

#[cfg(feature = "alloc")]
impl<T> TimerWheelLevel<T> {
    fn new(size: usize) -> Self {
        let mut slots = alloc_crate::vec::Vec::with_capacity(size);
        for _ in 0..size {
            slots.push(alloc_crate::vec::Vec::new());
        }
        Self {
            slots,
            current_slot: 0,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp() {
        let ts = Timestamp::from_secs(1704067200); // 2024-01-01 00:00:00 UTC
        assert_eq!(ts.secs(), 1704067200);
        assert_eq!(ts.subsec_nanos(), 0);

        let ts2 = Timestamp::from_secs_nanos(1704067200, 500_000_000);
        assert_eq!(ts2.secs(), 1704067200);
        assert_eq!(ts2.subsec_nanos(), 500_000_000);
    }

    #[test]
    fn test_duration() {
        let d = Duration::from_secs(3661); // 1h 1m 1s
        assert_eq!(d.as_secs(), 3661);

        let d2 = Duration::from_millis(1500);
        assert_eq!(d2.as_secs(), 1);
        assert_eq!(d2.subsec_nanos(), 500_000_000);
    }

    #[test]
    fn test_timestamp_arithmetic() {
        let ts1 = Timestamp::from_secs(100);
        let ts2 = Timestamp::from_secs(150);
        let duration = ts2 - ts1;
        assert_eq!(duration.as_secs(), 50);

        let ts3 = ts1 + Duration::from_secs(25);
        assert_eq!(ts3.secs(), 125);
    }
}
