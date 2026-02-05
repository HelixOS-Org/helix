//! Time and Timer Services
//!
//! UEFI time management, RTC access, and high-precision timing.

use core::fmt;

// =============================================================================
// TIME STRUCTURE
// =============================================================================

/// UEFI Time structure
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Time {
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
    /// Padding
    _pad1: u8,
    /// Nanosecond (0-999999999)
    pub nanosecond: u32,
    /// Timezone (-1440 to 1440 or 2047 for unspecified)
    pub timezone: i16,
    /// Daylight saving time
    pub daylight: u8,
    /// Padding
    _pad2: u8,
}

impl Time {
    /// Unspecified timezone
    pub const TIMEZONE_UNSPECIFIED: i16 = 2047;

    /// Daylight saving time adjust
    pub const DAYLIGHT_ADJUST: u8 = 0x01;

    /// Daylight saving time in effect
    pub const DAYLIGHT_IN_EFFECT: u8 = 0x02;

    /// Create new time
    pub const fn new(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Self {
        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            _pad1: 0,
            nanosecond: 0,
            timezone: Self::TIMEZONE_UNSPECIFIED,
            daylight: 0,
            _pad2: 0,
        }
    }

    /// Create empty time
    pub const fn empty() -> Self {
        Self {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
            _pad1: 0,
            nanosecond: 0,
            timezone: Self::TIMEZONE_UNSPECIFIED,
            daylight: 0,
            _pad2: 0,
        }
    }

    /// Validate time
    pub fn is_valid(&self) -> bool {
        if self.year < 1900 || self.year > 9999 {
            return false;
        }
        if self.month < 1 || self.month > 12 {
            return false;
        }
        if self.day < 1 || self.day > days_in_month(self.year, self.month) {
            return false;
        }
        if self.hour > 23 {
            return false;
        }
        if self.minute > 59 {
            return false;
        }
        if self.second > 59 {
            return false;
        }
        if self.nanosecond > 999_999_999 {
            return false;
        }
        if self.timezone != Self::TIMEZONE_UNSPECIFIED
            && (self.timezone < -1440 || self.timezone > 1440)
        {
            return false;
        }
        true
    }

    /// Is leap year
    pub fn is_leap_year(&self) -> bool {
        is_leap_year(self.year)
    }

    /// Day of year (1-366)
    pub fn day_of_year(&self) -> u16 {
        let mut days = 0u16;
        for m in 1..self.month {
            days += days_in_month(self.year, m) as u16;
        }
        days + self.day as u16
    }

    /// Day of week (0=Sunday, 6=Saturday)
    pub fn day_of_week(&self) -> u8 {
        // Zeller's congruence
        let mut year_adj = self.year as i32;
        let mut month_adj = self.month as i32;

        if month_adj < 3 {
            month_adj += 12;
            year_adj -= 1;
        }

        let day_of_month = self.day as i32;
        let year_of_century = year_adj % 100;
        let century = year_adj / 100;

        let zeller_result = (day_of_month
            + (13 * (month_adj + 1)) / 5
            + year_of_century
            + year_of_century / 4
            + century / 4
            - 2 * century)
            % 7;

        // Convert from Zeller (Saturday=0) to (Sunday=0)
        ((zeller_result + 6) % 7) as u8
    }

    /// Week of year (ISO 8601)
    pub fn week_of_year(&self) -> u8 {
        let day_of_year = self.day_of_year() as i32;
        let day_of_week = self.day_of_week() as i32;

        // Thursday-based week
        let week = (day_of_year - day_of_week + 10) / 7;

        week.max(1).min(53) as u8
    }

    /// Convert to Unix timestamp (seconds since 1970-01-01 00:00:00 UTC)
    pub fn to_unix_timestamp(&self) -> i64 {
        let mut days = 0i64;

        // Years
        for y in 1970..self.year {
            days += if is_leap_year(y) { 366 } else { 365 };
        }

        // Months
        for m in 1..self.month {
            days += days_in_month(self.year, m) as i64;
        }

        // Days
        days += (self.day - 1) as i64;

        // Convert to seconds
        let mut secs = days * 86400;
        secs += self.hour as i64 * 3600;
        secs += self.minute as i64 * 60;
        secs += self.second as i64;

        // Adjust for timezone
        if self.timezone != Self::TIMEZONE_UNSPECIFIED {
            secs -= self.timezone as i64 * 60;
        }

        secs
    }

    /// Create from Unix timestamp
    pub fn from_unix_timestamp(timestamp: i64) -> Self {
        let mut remaining = timestamp;

        // Calculate year
        let mut year = 1970u16;
        loop {
            let year_secs = if is_leap_year(year) {
                366 * 86400
            } else {
                365 * 86400
            };

            if remaining < year_secs {
                break;
            }

            remaining -= year_secs;
            year += 1;

            if year > 9999 {
                break;
            }
        }

        // Calculate month
        let mut month = 1u8;
        while month <= 12 {
            let month_secs = days_in_month(year, month) as i64 * 86400;
            if remaining < month_secs {
                break;
            }
            remaining -= month_secs;
            month += 1;
        }

        // Calculate day
        let day = (remaining / 86400) as u8 + 1;
        remaining %= 86400;

        // Calculate hour
        let hour = (remaining / 3600) as u8;
        remaining %= 3600;

        // Calculate minute
        let minute = (remaining / 60) as u8;

        // Calculate second
        let second = (remaining % 60) as u8;

        Self::new(year, month, day, hour, minute, second)
    }

    /// Add seconds
    pub fn add_seconds(&mut self, seconds: i64) {
        let current = self.to_unix_timestamp();
        *self = Self::from_unix_timestamp(current + seconds);
    }

    /// Compare times
    pub fn compare(&self, other: &Time) -> core::cmp::Ordering {
        // Compare by timestamp for accuracy
        self.to_unix_timestamp().cmp(&other.to_unix_timestamp())
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

// =============================================================================
// TIME CAPABILITIES
// =============================================================================

/// RTC capabilities
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct TimeCapabilities {
    /// Resolution in counts per second (1 Hz = 1)
    pub resolution: u32,
    /// Accuracy in PPM
    pub accuracy: u32,
    /// Sets to zero on reset
    pub sets_to_zero: bool,
}

impl TimeCapabilities {
    /// Create new capabilities
    pub const fn new(resolution: u32, accuracy: u32, sets_to_zero: bool) -> Self {
        Self {
            resolution,
            accuracy,
            sets_to_zero,
        }
    }
}

// =============================================================================
// DURATION
// =============================================================================

/// Duration with nanosecond precision
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration {
    /// Seconds
    secs: u64,
    /// Nanoseconds (0-999_999_999)
    nanos: u32,
}

impl Duration {
    /// Zero duration
    pub const ZERO: Duration = Duration { secs: 0, nanos: 0 };

    /// Maximum duration
    pub const MAX: Duration = Duration {
        secs: u64::MAX,
        nanos: 999_999_999,
    };

    /// Nanoseconds per second
    const NANOS_PER_SEC: u32 = 1_000_000_000;

    /// Create from seconds
    pub const fn from_secs(secs: u64) -> Self {
        Self { secs, nanos: 0 }
    }

    /// Create from milliseconds
    pub const fn from_millis(millis: u64) -> Self {
        Self {
            secs: millis / 1000,
            nanos: ((millis % 1000) * 1_000_000) as u32,
        }
    }

    /// Create from microseconds
    pub const fn from_micros(micros: u64) -> Self {
        Self {
            secs: micros / 1_000_000,
            nanos: ((micros % 1_000_000) * 1000) as u32,
        }
    }

    /// Create from nanoseconds
    pub const fn from_nanos(nanos: u64) -> Self {
        Self {
            secs: nanos / Self::NANOS_PER_SEC as u64,
            nanos: (nanos % Self::NANOS_PER_SEC as u64) as u32,
        }
    }

    /// Create from seconds and nanoseconds
    pub const fn new(secs: u64, nanos: u32) -> Self {
        Self {
            secs: secs + (nanos / Self::NANOS_PER_SEC) as u64,
            nanos: nanos % Self::NANOS_PER_SEC,
        }
    }

    /// Get seconds
    pub const fn as_secs(&self) -> u64 {
        self.secs
    }

    /// Get total milliseconds
    pub const fn as_millis(&self) -> u128 {
        self.secs as u128 * 1000 + self.nanos as u128 / 1_000_000
    }

    /// Get total microseconds
    pub const fn as_micros(&self) -> u128 {
        self.secs as u128 * 1_000_000 + self.nanos as u128 / 1000
    }

    /// Get total nanoseconds
    pub const fn as_nanos(&self) -> u128 {
        self.secs as u128 * Self::NANOS_PER_SEC as u128 + self.nanos as u128
    }

    /// Get subsecond nanoseconds
    pub const fn subsec_nanos(&self) -> u32 {
        self.nanos
    }

    /// Is zero
    pub const fn is_zero(&self) -> bool {
        self.secs == 0 && self.nanos == 0
    }

    /// Checked add
    pub fn checked_add(self, rhs: Duration) -> Option<Duration> {
        let mut secs = self.secs.checked_add(rhs.secs)?;
        let mut nanos = self.nanos + rhs.nanos;

        if nanos >= Self::NANOS_PER_SEC {
            nanos -= Self::NANOS_PER_SEC;
            secs = secs.checked_add(1)?;
        }

        Some(Duration { secs, nanos })
    }

    /// Checked sub
    pub fn checked_sub(self, rhs: Duration) -> Option<Duration> {
        let mut secs = self.secs.checked_sub(rhs.secs)?;
        let nanos;

        if self.nanos >= rhs.nanos {
            nanos = self.nanos - rhs.nanos;
        } else {
            secs = secs.checked_sub(1)?;
            nanos = Self::NANOS_PER_SEC + self.nanos - rhs.nanos;
        }

        Some(Duration { secs, nanos })
    }

    /// Checked multiply
    pub fn checked_mul(self, rhs: u32) -> Option<Duration> {
        let total_nanos = self.nanos as u64 * rhs as u64;
        let secs = self
            .secs
            .checked_mul(rhs as u64)?
            .checked_add(total_nanos / Self::NANOS_PER_SEC as u64)?;
        let nanos = (total_nanos % Self::NANOS_PER_SEC as u64) as u32;

        Some(Duration { secs, nanos })
    }

    /// Saturating add
    pub fn saturating_add(self, rhs: Duration) -> Duration {
        self.checked_add(rhs).unwrap_or(Self::MAX)
    }

    /// Saturating sub
    pub fn saturating_sub(self, rhs: Duration) -> Duration {
        self.checked_sub(rhs).unwrap_or(Self::ZERO)
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.secs == 0 && self.nanos < 1000 {
            write!(f, "{}ns", self.nanos)
        } else if self.secs == 0 && self.nanos < 1_000_000 {
            write!(f, "{}µs", self.nanos / 1000)
        } else if self.secs == 0 {
            write!(f, "{}ms", self.nanos / 1_000_000)
        } else {
            write!(f, "{}.{:03}s", self.secs, self.nanos / 1_000_000)
        }
    }
}

// =============================================================================
// INSTANT
// =============================================================================

/// Monotonic instant for measuring elapsed time
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant {
    /// Tick count
    ticks: u64,
}

impl Instant {
    /// Create instant from ticks
    pub const fn from_ticks(ticks: u64) -> Self {
        Self { ticks }
    }

    /// Get ticks
    pub const fn ticks(&self) -> u64 {
        self.ticks
    }

    /// Duration since another instant
    pub fn duration_since(&self, earlier: Instant, frequency: u64) -> Duration {
        if self.ticks <= earlier.ticks || frequency == 0 {
            return Duration::ZERO;
        }

        let diff = self.ticks - earlier.ticks;

        let secs = diff / frequency;
        let remaining = diff % frequency;
        let nanos = ((remaining as u128 * 1_000_000_000) / frequency as u128) as u32;

        Duration::new(secs, nanos)
    }

    /// Elapsed since this instant
    pub fn elapsed(&self, now: Instant, frequency: u64) -> Duration {
        now.duration_since(*self, frequency)
    }

    /// Check if duration has passed
    pub fn has_elapsed(&self, now: Instant, duration: Duration, frequency: u64) -> bool {
        self.elapsed(now, frequency) >= duration
    }
}

// =============================================================================
// TIMER
// =============================================================================

/// Timer mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerMode {
    /// One-shot timer
    OneShot,
    /// Periodic timer
    Periodic,
}

/// Timer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerState {
    /// Idle
    Idle,
    /// Running
    Running,
    /// Expired
    Expired,
    /// Cancelled
    Cancelled,
}

/// Software timer
pub struct Timer {
    /// Mode
    mode: TimerMode,
    /// State
    state: TimerState,
    /// Period in ticks
    period: u64,
    /// Start tick
    start_tick: u64,
    /// Target tick
    target_tick: u64,
    /// Expiration count
    expirations: u64,
}

impl Timer {
    /// Create one-shot timer
    pub const fn oneshot() -> Self {
        Self {
            mode: TimerMode::OneShot,
            state: TimerState::Idle,
            period: 0,
            start_tick: 0,
            target_tick: 0,
            expirations: 0,
        }
    }

    /// Create periodic timer
    pub const fn periodic() -> Self {
        Self {
            mode: TimerMode::Periodic,
            state: TimerState::Idle,
            period: 0,
            start_tick: 0,
            target_tick: 0,
            expirations: 0,
        }
    }

    /// Start timer
    pub fn start(&mut self, current_tick: u64, period_ticks: u64) {
        self.period = period_ticks;
        self.start_tick = current_tick;
        self.target_tick = current_tick.saturating_add(period_ticks);
        self.state = TimerState::Running;
    }

    /// Start with duration
    pub fn start_duration(&mut self, current_tick: u64, duration: Duration, frequency: u64) {
        let period_ticks = (duration.as_nanos() * frequency as u128 / 1_000_000_000) as u64;
        self.start(current_tick, period_ticks);
    }

    /// Cancel timer
    pub fn cancel(&mut self) {
        self.state = TimerState::Cancelled;
    }

    /// Check timer
    pub fn check(&mut self, current_tick: u64) -> bool {
        if self.state != TimerState::Running {
            return false;
        }

        if current_tick >= self.target_tick {
            self.expirations += 1;

            match self.mode {
                TimerMode::OneShot => {
                    self.state = TimerState::Expired;
                },
                TimerMode::Periodic => {
                    self.target_tick = self.target_tick.saturating_add(self.period);
                },
            }

            return true;
        }

        false
    }

    /// Is running
    pub fn is_running(&self) -> bool {
        self.state == TimerState::Running
    }

    /// Is expired
    pub fn is_expired(&self) -> bool {
        self.state == TimerState::Expired
    }

    /// Get expiration count
    pub fn expirations(&self) -> u64 {
        self.expirations
    }

    /// Time until expiration
    pub fn remaining(&self, current_tick: u64) -> u64 {
        if self.state != TimerState::Running || current_tick >= self.target_tick {
            return 0;
        }

        self.target_tick - current_tick
    }
}

// =============================================================================
// STOPWATCH
// =============================================================================

/// Stopwatch for measuring intervals
pub struct Stopwatch {
    /// Start tick
    start: u64,
    /// Accumulated ticks (when paused)
    accumulated: u64,
    /// Is running
    running: bool,
    /// Lap times
    laps: [u64; 16],
    /// Lap count
    lap_count: usize,
}

impl Stopwatch {
    /// Create new stopwatch
    pub const fn new() -> Self {
        Self {
            start: 0,
            accumulated: 0,
            running: false,
            laps: [0; 16],
            lap_count: 0,
        }
    }

    /// Start or resume
    pub fn start(&mut self, current_tick: u64) {
        if !self.running {
            self.start = current_tick;
            self.running = true;
        }
    }

    /// Stop (pause)
    pub fn stop(&mut self, current_tick: u64) {
        if self.running {
            self.accumulated += current_tick.saturating_sub(self.start);
            self.running = false;
        }
    }

    /// Reset
    pub fn reset(&mut self) {
        self.start = 0;
        self.accumulated = 0;
        self.running = false;
        self.lap_count = 0;
    }

    /// Lap (record time and continue)
    pub fn lap(&mut self, current_tick: u64) {
        if self.lap_count < 16 {
            self.laps[self.lap_count] = self.elapsed_ticks(current_tick);
            self.lap_count += 1;
        }
    }

    /// Get elapsed ticks
    pub fn elapsed_ticks(&self, current_tick: u64) -> u64 {
        if self.running {
            self.accumulated + current_tick.saturating_sub(self.start)
        } else {
            self.accumulated
        }
    }

    /// Get elapsed duration
    pub fn elapsed(&self, current_tick: u64, frequency: u64) -> Duration {
        let ticks = self.elapsed_ticks(current_tick);

        if frequency == 0 {
            return Duration::ZERO;
        }

        let secs = ticks / frequency;
        let remaining = ticks % frequency;
        let nanos = ((remaining as u128 * 1_000_000_000) / frequency as u128) as u32;

        Duration::new(secs, nanos)
    }

    /// Is running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Get lap times
    pub fn laps(&self) -> &[u64] {
        &self.laps[..self.lap_count]
    }
}

// =============================================================================
// COUNTDOWN
// =============================================================================

/// Countdown timer
pub struct Countdown {
    /// Target tick
    target: u64,
    /// Is active
    active: bool,
}

impl Countdown {
    /// Create new countdown
    pub const fn new() -> Self {
        Self {
            target: 0,
            active: false,
        }
    }

    /// Start countdown
    pub fn start(&mut self, current_tick: u64, duration_ticks: u64) {
        self.target = current_tick.saturating_add(duration_ticks);
        self.active = true;
    }

    /// Start with duration
    pub fn start_duration(&mut self, current_tick: u64, duration: Duration, frequency: u64) {
        let ticks = (duration.as_nanos() * frequency as u128 / 1_000_000_000) as u64;
        self.start(current_tick, ticks);
    }

    /// Check if finished
    pub fn is_finished(&self, current_tick: u64) -> bool {
        self.active && current_tick >= self.target
    }

    /// Remaining ticks
    pub fn remaining(&self, current_tick: u64) -> u64 {
        if !self.active || current_tick >= self.target {
            return 0;
        }
        self.target - current_tick
    }

    /// Remaining duration
    pub fn remaining_duration(&self, current_tick: u64, frequency: u64) -> Duration {
        let ticks = self.remaining(current_tick);

        if frequency == 0 || ticks == 0 {
            return Duration::ZERO;
        }

        let secs = ticks / frequency;
        let remaining = ticks % frequency;
        let nanos = ((remaining as u128 * 1_000_000_000) / frequency as u128) as u32;

        Duration::new(secs, nanos)
    }

    /// Cancel
    pub fn cancel(&mut self) {
        self.active = false;
    }

    /// Is active
    pub fn is_active(&self) -> bool {
        self.active
    }
}

// =============================================================================
// TSC (TIME STAMP COUNTER)
// =============================================================================

/// Read TSC (x86_64)
#[cfg(target_arch = "x86_64")]
pub fn read_tsc() -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        core::arch::asm!(
            "rdtsc",
            out("eax") low,
            out("edx") high,
            options(nostack, nomem)
        );
    }
    ((high as u64) << 32) | (low as u64)
}

/// Read TSC (aarch64 - use cycle counter)
#[cfg(target_arch = "aarch64")]
pub fn read_tsc() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, cntvct_el0",
            out(reg) value,
            options(nostack, nomem)
        );
    }
    value
}

/// Read TSC fallback
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
pub fn read_tsc() -> u64 {
    0
}

/// Read TSC with ordering fence (x86_64)
#[cfg(target_arch = "x86_64")]
pub fn read_tsc_ordered() -> u64 {
    let low: u32;
    let high: u32;
    let _aux: u32;
    unsafe {
        core::arch::asm!(
            "rdtscp",
            out("eax") low,
            out("edx") high,
            out("ecx") _aux,
            options(nostack, nomem)
        );
    }
    ((high as u64) << 32) | (low as u64)
}

#[cfg(not(target_arch = "x86_64"))]
pub fn read_tsc_ordered() -> u64 {
    read_tsc()
}

/// Estimate TSC frequency by delay
pub fn estimate_tsc_frequency() -> u64 {
    // Try to use a known delay source
    // For UEFI, we'd typically use stall() - here we estimate roughly

    #[cfg(target_arch = "x86_64")]
    {
        let start = read_tsc();

        // Simple delay loop (~1ms on typical CPU)
        for _ in 0..100_000 {
            unsafe {
                core::arch::asm!("pause", options(nomem, nostack));
            }
        }

        let end = read_tsc();
        let cycles = end.saturating_sub(start);

        // Assume ~1ms delay, scale to 1 second
        cycles.saturating_mul(1000)
    }

    #[cfg(target_arch = "aarch64")]
    {
        // Read counter frequency
        let freq: u64;
        unsafe {
            core::arch::asm!(
                "mrs {}, cntfrq_el0",
                out(reg) freq,
                options(nostack, nomem)
            );
        }
        freq
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        0
    }
}

// =============================================================================
// DELAY FUNCTIONS
// =============================================================================

/// Delay for specified TSC cycles
#[inline]
pub fn delay_cycles(cycles: u64) {
    let start = read_tsc();
    let target = start.saturating_add(cycles);

    while read_tsc() < target {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::asm!("pause", options(nomem, nostack));
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            core::arch::asm!("yield", options(nomem, nostack));
        }
    }
}

/// Delay for microseconds (needs calibrated frequency)
pub fn delay_us(us: u64, frequency: u64) {
    if frequency == 0 {
        return;
    }

    let cycles = us * frequency / 1_000_000;
    delay_cycles(cycles);
}

/// Delay for milliseconds
pub fn delay_ms(ms: u64, frequency: u64) {
    delay_us(ms * 1000, frequency);
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Is leap year
pub const fn is_leap_year(year: u16) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Days in month
pub const fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 => 31,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        },
        3 => 31,
        4 => 30,
        5 => 31,
        6 => 30,
        7 => 31,
        8 => 31,
        9 => 30,
        10 => 31,
        11 => 30,
        12 => 31,
        _ => 0,
    }
}

/// Days in year
pub const fn days_in_year(year: u16) -> u16 {
    if is_leap_year(year) {
        366
    } else {
        365
    }
}

/// Month name
pub const fn month_name(month: u8) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    }
}

/// Month short name
pub const fn month_short_name(month: u8) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "???",
    }
}

/// Day of week name
pub const fn day_name(day: u8) -> &'static str {
    match day {
        0 => "Sunday",
        1 => "Monday",
        2 => "Tuesday",
        3 => "Wednesday",
        4 => "Thursday",
        5 => "Friday",
        6 => "Saturday",
        _ => "Unknown",
    }
}

/// Day of week short name
pub const fn day_short_name(day: u8) -> &'static str {
    match day {
        0 => "Sun",
        1 => "Mon",
        2 => "Tue",
        3 => "Wed",
        4 => "Thu",
        5 => "Fri",
        6 => "Sat",
        _ => "???",
    }
}

// =============================================================================
// TIMEZONE
// =============================================================================

/// Common timezone offsets (in minutes from UTC)
pub mod timezone {
    pub const UTC: i16 = 0;
    pub const GMT: i16 = 0;
    pub const CET: i16 = 60; // Central European Time
    pub const EET: i16 = 120; // Eastern European Time
    pub const MSK: i16 = 180; // Moscow Time
    pub const IST: i16 = 330; // India Standard Time
    pub const CST: i16 = 480; // China Standard Time
    pub const JST: i16 = 540; // Japan Standard Time
    pub const AEST: i16 = 600; // Australian Eastern Standard Time
    pub const NZST: i16 = 720; // New Zealand Standard Time
    pub const EST: i16 = -300; // Eastern Standard Time (US)
    pub const CST_US: i16 = -360; // Central Standard Time (US)
    pub const MST: i16 = -420; // Mountain Standard Time (US)
    pub const PST: i16 = -480; // Pacific Standard Time (US)
    pub const AKST: i16 = -540; // Alaska Standard Time
    pub const HST: i16 = -600; // Hawaii Standard Time
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_time_creation() {
        let time = Time::new(2024, 6, 15, 14, 30, 45);
        assert_eq!(time.year, 2024);
        assert_eq!(time.month, 6);
        assert_eq!(time.day, 15);
        assert_eq!(time.hour, 14);
        assert_eq!(time.minute, 30);
        assert_eq!(time.second, 45);
    }

    #[test]
    fn test_time_validation() {
        assert!(Time::new(2024, 6, 15, 14, 30, 45).is_valid());
        assert!(!Time::new(2024, 13, 15, 14, 30, 45).is_valid()); // Invalid month
        assert!(!Time::new(2024, 6, 31, 14, 30, 45).is_valid()); // June has 30 days
        assert!(!Time::new(2024, 6, 15, 24, 30, 45).is_valid()); // Invalid hour
    }

    #[test]
    fn test_leap_year() {
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2023));
    }

    #[test]
    fn test_days_in_month() {
        assert_eq!(days_in_month(2024, 2), 29); // Leap year
        assert_eq!(days_in_month(2023, 2), 28); // Non-leap year
        assert_eq!(days_in_month(2024, 1), 31);
        assert_eq!(days_in_month(2024, 4), 30);
    }

    #[test]
    fn test_unix_timestamp() {
        let time = Time::new(1970, 1, 1, 0, 0, 0);
        assert_eq!(time.to_unix_timestamp(), 0);

        let time2 = Time::new(2000, 1, 1, 0, 0, 0);
        assert!(time2.to_unix_timestamp() > 0);
    }

    #[test]
    fn test_duration() {
        let d1 = Duration::from_secs(5);
        let d2 = Duration::from_millis(2500);

        assert_eq!(d1.as_secs(), 5);
        assert_eq!(d2.as_secs(), 2);
        assert_eq!(d2.as_millis(), 2500);

        let sum = d1.checked_add(d2).unwrap();
        assert_eq!(sum.as_millis(), 7500);
    }
}
