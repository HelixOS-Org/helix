//! Time Services
//!
//! High-level interface for UEFI time services.

use core::fmt;

use super::runtime::runtime_services;
use crate::raw::types::*;

// =============================================================================
// DATE TIME
// =============================================================================

/// Date and time representation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateTime {
    /// Year (1900-9999)
    pub year: u16,
    /// Month (1-12)
    pub month: u8,
    /// Day (1-31)
    pub day: u8,
    /// Hour (0-23)
    pub hour: u8,
    /// Minute (0-59)
    pub minute: u8,
    /// Second (0-59)
    pub second: u8,
    /// Nanosecond (0-999,999,999)
    pub nanosecond: u32,
    /// Timezone offset in minutes from UTC (-1440 to 1440)
    pub timezone: i16,
    /// Daylight saving time
    pub daylight: DaylightSaving,
}

impl DateTime {
    /// Create a new date time
    pub const fn new(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Self {
        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            nanosecond: 0,
            timezone: TIMEZONE_UNSPECIFIED,
            daylight: DaylightSaving::None,
        }
    }

    /// Get current time from UEFI
    pub fn now() -> Result<Self, Status> {
        let rs = unsafe { runtime_services() };
        let (time, _caps) = rs.get_time()?;
        Ok(Self::from_uefi(&time))
    }

    /// Get current time with capabilities
    pub fn now_with_capabilities() -> Result<(Self, ClockCapabilities), Status> {
        let rs = unsafe { runtime_services() };
        let (time, caps) = rs.get_time()?;
        Ok((
            Self::from_uefi(&time),
            caps.map(ClockCapabilities::from_uefi).unwrap_or_default(),
        ))
    }

    /// Set system time
    pub fn set_system_time(&self) -> Result<(), Status> {
        let rs = unsafe { runtime_services() };
        rs.set_time(&self.to_uefi())
    }

    /// Convert from UEFI time
    pub fn from_uefi(time: &Time) -> Self {
        Self {
            year: time.year,
            month: time.month,
            day: time.day,
            hour: time.hour,
            minute: time.minute,
            second: time.second,
            nanosecond: time.nanosecond,
            timezone: time.timezone,
            daylight: DaylightSaving::from_uefi(time.daylight),
        }
    }

    /// Convert to UEFI time
    pub fn to_uefi(&self) -> Time {
        Time {
            year: self.year,
            month: self.month,
            day: self.day,
            hour: self.hour,
            minute: self.minute,
            second: self.second,
            pad1: 0,
            nanosecond: self.nanosecond,
            timezone: self.timezone,
            daylight: self.daylight.to_uefi(),
            pad2: 0,
        }
    }

    /// Check if valid
    pub fn is_valid(&self) -> bool {
        self.year >= 1900
            && self.year <= 9999
            && self.month >= 1
            && self.month <= 12
            && self.day >= 1
            && self.day <= 31
            && self.hour <= 23
            && self.minute <= 59
            && self.second <= 59
            && self.nanosecond < 1_000_000_000
    }

    /// Get day of week (0 = Sunday, 6 = Saturday)
    /// Uses Zeller's congruence
    pub fn day_of_week(&self) -> u8 {
        let mut y = self.year as i32;
        let mut m = self.month as i32;

        if m < 3 {
            m += 12;
            y -= 1;
        }

        let q = self.day as i32;
        let k = y % 100;
        let j = y / 100;

        let h = (q + (13 * (m + 1)) / 5 + k + k / 4 + j / 4 - 2 * j) % 7;
        ((h + 6) % 7) as u8
    }

    /// Get day of year (1-366)
    pub fn day_of_year(&self) -> u16 {
        let days_in_month = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let mut day: u16 = self.day as u16;

        for m in 1..self.month {
            day += days_in_month[m as usize] as u16;
        }

        // Add leap day if applicable
        if self.month > 2 && self.is_leap_year() {
            day += 1;
        }

        day
    }

    /// Check if leap year
    pub fn is_leap_year(&self) -> bool {
        let y = self.year;
        (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
    }

    /// Format as ISO 8601
    pub fn format_iso8601(&self) -> alloc::string::String {
        alloc::format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            self.year,
            self.month,
            self.day,
            self.hour,
            self.minute,
            self.second
        )
    }

    /// Format as RFC 2822
    pub fn format_rfc2822(&self) -> alloc::string::String {
        let day_names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        let month_names = [
            "", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];

        let dow = self.day_of_week() as usize;

        alloc::format!(
            "{}, {:02} {} {:04} {:02}:{:02}:{:02}",
            day_names[dow],
            self.day,
            month_names[self.month as usize],
            self.year,
            self.hour,
            self.minute,
            self.second
        )
    }

    /// Convert to Unix timestamp (seconds since 1970-01-01 00:00:00 UTC)
    pub fn to_unix_timestamp(&self) -> i64 {
        // Days since epoch
        let mut days: i64 = 0;

        // Years
        for y in 1970..self.year {
            days += if (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0) {
                366
            } else {
                365
            };
        }

        // Months
        let days_in_month = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        for m in 1..self.month {
            days += days_in_month[m as usize] as i64;
        }
        if self.month > 2 && self.is_leap_year() {
            days += 1;
        }

        // Days
        days += (self.day - 1) as i64;

        // Convert to seconds
        let mut seconds = days * 86400;
        seconds += self.hour as i64 * 3600;
        seconds += self.minute as i64 * 60;
        seconds += self.second as i64;

        // Adjust for timezone
        if self.timezone != TIMEZONE_UNSPECIFIED {
            seconds -= self.timezone as i64 * 60;
        }

        seconds
    }

    /// Create from Unix timestamp
    pub fn from_unix_timestamp(timestamp: i64) -> Self {
        let mut remaining = timestamp;

        // Calculate year
        let mut year = 1970u16;
        loop {
            let days_in_year = if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                366
            } else {
                365
            };
            let seconds_in_year = days_in_year * 86400;

            if remaining < seconds_in_year {
                break;
            }
            remaining -= seconds_in_year;
            year += 1;
        }

        // Calculate month and day
        let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let days_in_month: [i64; 13] = if leap {
            [0, 31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        } else {
            [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        };

        let mut day_of_year = remaining / 86400;
        remaining %= 86400;

        let mut month = 1u8;
        while month <= 12 && day_of_year >= days_in_month[month as usize] {
            day_of_year -= days_in_month[month as usize];
            month += 1;
        }

        let day = (day_of_year + 1) as u8;
        let hour = (remaining / 3600) as u8;
        remaining %= 3600;
        let minute = (remaining / 60) as u8;
        let second = (remaining % 60) as u8;

        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            nanosecond: 0,
            timezone: 0, // UTC
            daylight: DaylightSaving::None,
        }
    }
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_iso8601())
    }
}

impl Default for DateTime {
    fn default() -> Self {
        Self::new(1970, 1, 1, 0, 0, 0)
    }
}

/// Timezone unspecified value
pub const TIMEZONE_UNSPECIFIED: i16 = 0x07FF;

// =============================================================================
// DAYLIGHT SAVING
// =============================================================================

/// Daylight saving time flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaylightSaving {
    /// No daylight saving
    None,
    /// Time is affected by daylight saving but not currently in effect
    AdjustDaylight,
    /// Daylight saving is in effect
    InDaylight,
    /// Both flags set
    AdjustDaylightAndInDaylight,
}

impl DaylightSaving {
    /// Convert from UEFI daylight value
    pub fn from_uefi(value: u8) -> Self {
        match value & 0x03 {
            0 => Self::None,
            1 => Self::AdjustDaylight,
            2 => Self::InDaylight,
            3 => Self::AdjustDaylightAndInDaylight,
            _ => Self::None,
        }
    }

    /// Convert to UEFI daylight value
    pub fn to_uefi(self) -> u8 {
        match self {
            Self::None => 0,
            Self::AdjustDaylight => 1,
            Self::InDaylight => 2,
            Self::AdjustDaylightAndInDaylight => 3,
        }
    }
}

impl Default for DaylightSaving {
    fn default() -> Self {
        Self::None
    }
}

// =============================================================================
// CLOCK CAPABILITIES
// =============================================================================

/// Clock capabilities
#[derive(Debug, Clone, Copy, Default)]
pub struct ClockCapabilities {
    /// Clock resolution in counts per second
    pub resolution: u32,
    /// Clock accuracy in parts per million
    pub accuracy: u32,
    /// Whether clock stores time in local time or UTC
    pub sets_to_zero: bool,
}

impl ClockCapabilities {
    /// Convert from UEFI capabilities
    pub fn from_uefi(caps: TimeCapabilities) -> Self {
        Self {
            resolution: caps.resolution,
            accuracy: caps.accuracy,
            sets_to_zero: caps.sets_to_zero != 0,
        }
    }

    /// Get resolution in nanoseconds
    pub fn resolution_ns(&self) -> u64 {
        if self.resolution == 0 {
            0
        } else {
            1_000_000_000 / self.resolution as u64
        }
    }

    /// Get accuracy percentage
    pub fn accuracy_percent(&self) -> f32 {
        self.accuracy as f32 / 10_000.0
    }
}

// =============================================================================
// WAKEUP
// =============================================================================

/// Wakeup time wrapper
pub struct WakeupTime {
    /// Whether wakeup is enabled
    pub enabled: bool,
    /// Whether wakeup is pending
    pub pending: bool,
    /// Wakeup time
    pub time: DateTime,
}

impl WakeupTime {
    /// Get current wakeup time
    pub fn get() -> Result<Self, Status> {
        let rs = unsafe { runtime_services() };
        let (enabled, pending, time) = rs.get_wakeup_time()?;

        Ok(Self {
            enabled,
            pending,
            time: DateTime::from_uefi(&time),
        })
    }

    /// Set wakeup time
    pub fn set(time: &DateTime) -> Result<(), Status> {
        let rs = unsafe { runtime_services() };
        rs.set_wakeup_time(true, Some(&time.to_uefi()))
    }

    /// Clear wakeup time
    pub fn clear() -> Result<(), Status> {
        let rs = unsafe { runtime_services() };
        rs.set_wakeup_time(false, None)
    }
}

// =============================================================================
// DURATION
// =============================================================================

/// Time duration
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration {
    /// Total nanoseconds
    nanos: u64,
}

impl Duration {
    /// Zero duration
    pub const ZERO: Self = Self { nanos: 0 };

    /// Maximum duration
    pub const MAX: Self = Self { nanos: u64::MAX };

    /// Create from nanoseconds
    pub const fn from_nanos(nanos: u64) -> Self {
        Self { nanos }
    }

    /// Create from microseconds
    pub const fn from_micros(micros: u64) -> Self {
        Self {
            nanos: micros.saturating_mul(1000),
        }
    }

    /// Create from milliseconds
    pub const fn from_millis(millis: u64) -> Self {
        Self {
            nanos: millis.saturating_mul(1_000_000),
        }
    }

    /// Create from seconds
    pub const fn from_secs(secs: u64) -> Self {
        Self {
            nanos: secs.saturating_mul(1_000_000_000),
        }
    }

    /// Create from minutes
    pub const fn from_mins(mins: u64) -> Self {
        Self {
            nanos: mins.saturating_mul(60_000_000_000),
        }
    }

    /// Create from hours
    pub const fn from_hours(hours: u64) -> Self {
        Self {
            nanos: hours.saturating_mul(3_600_000_000_000),
        }
    }

    /// Get total nanoseconds
    pub const fn as_nanos(&self) -> u64 {
        self.nanos
    }

    /// Get total microseconds
    pub const fn as_micros(&self) -> u64 {
        self.nanos / 1000
    }

    /// Get total milliseconds
    pub const fn as_millis(&self) -> u64 {
        self.nanos / 1_000_000
    }

    /// Get total seconds
    pub const fn as_secs(&self) -> u64 {
        self.nanos / 1_000_000_000
    }

    /// Get subsecond nanoseconds
    pub const fn subsec_nanos(&self) -> u32 {
        (self.nanos % 1_000_000_000) as u32
    }

    /// Check if zero
    pub const fn is_zero(&self) -> bool {
        self.nanos == 0
    }

    /// Saturating add
    pub const fn saturating_add(self, other: Self) -> Self {
        Self {
            nanos: self.nanos.saturating_add(other.nanos),
        }
    }

    /// Saturating sub
    pub const fn saturating_sub(self, other: Self) -> Self {
        Self {
            nanos: self.nanos.saturating_sub(other.nanos),
        }
    }

    /// Convert to UEFI 100ns units
    pub const fn to_uefi_100ns(&self) -> u64 {
        self.nanos / 100
    }
}

impl core::ops::Add for Duration {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            nanos: self.nanos + rhs.nanos,
        }
    }
}

impl core::ops::Sub for Duration {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            nanos: self.nanos - rhs.nanos,
        }
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let secs = self.as_secs();
        let nanos = self.subsec_nanos();

        if secs > 0 {
            write!(f, "{}.{:09}s", secs, nanos)
        } else if nanos >= 1_000_000 {
            write!(f, "{}ms", nanos / 1_000_000)
        } else if nanos >= 1_000 {
            write!(f, "{}µs", nanos / 1_000)
        } else {
            write!(f, "{}ns", nanos)
        }
    }
}

// =============================================================================
// STOPWATCH
// =============================================================================

/// Simple stopwatch for timing
pub struct Stopwatch {
    start: DateTime,
}

impl Stopwatch {
    /// Start a new stopwatch
    pub fn start() -> Result<Self, Status> {
        Ok(Self {
            start: DateTime::now()?,
        })
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> Result<Duration, Status> {
        let now = DateTime::now()?;
        let start_ts = self.start.to_unix_timestamp();
        let now_ts = now.to_unix_timestamp();

        let diff_secs = (now_ts - start_ts).max(0) as u64;
        Ok(Duration::from_secs(diff_secs))
    }

    /// Reset the stopwatch
    pub fn reset(&mut self) -> Result<(), Status> {
        self.start = DateTime::now()?;
        Ok(())
    }

    /// Lap: get elapsed time and reset
    pub fn lap(&mut self) -> Result<Duration, Status> {
        let elapsed = self.elapsed()?;
        self.reset()?;
        Ok(elapsed)
    }
}

// =============================================================================
// EXTERN ALLOC
// =============================================================================

extern crate alloc;

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_datetime_validation() {
        let valid = DateTime::new(2024, 6, 15, 12, 30, 45);
        assert!(valid.is_valid());

        let invalid = DateTime::new(2024, 13, 1, 0, 0, 0);
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_day_of_week() {
        // 2024-01-01 is Monday
        let dt = DateTime::new(2024, 1, 1, 0, 0, 0);
        assert_eq!(dt.day_of_week(), 1);
    }

    #[test]
    fn test_leap_year() {
        assert!(DateTime::new(2024, 1, 1, 0, 0, 0).is_leap_year());
        assert!(!DateTime::new(2023, 1, 1, 0, 0, 0).is_leap_year());
        assert!(!DateTime::new(2100, 1, 1, 0, 0, 0).is_leap_year());
        assert!(DateTime::new(2000, 1, 1, 0, 0, 0).is_leap_year());
    }

    #[test]
    fn test_unix_timestamp() {
        let epoch = DateTime::new(1970, 1, 1, 0, 0, 0);
        assert_eq!(epoch.to_unix_timestamp(), 0);

        let ts = 1718451600i64; // 2024-06-15 12:00:00 UTC
        let dt = DateTime::from_unix_timestamp(ts);
        assert_eq!(dt.year, 2024);
    }

    #[test]
    fn test_duration() {
        let d = Duration::from_millis(1500);
        assert_eq!(d.as_secs(), 1);
        assert_eq!(d.subsec_nanos(), 500_000_000);
    }
}
