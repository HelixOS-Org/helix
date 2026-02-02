//! # Boot Control Requests
//!
//! This module provides boot-related Limine requests:
//! - Entry point override
//! - Stack size configuration
//! - Boot time

use super::{LimineRequest, ResponsePtr, SafeResponse};
use crate::protocol::raw::EntryPointFn;
use crate::protocol::request_ids::{BOOT_TIME_ID, ENTRY_POINT_ID, STACK_SIZE_ID};

// =============================================================================
// Entry Point Request
// =============================================================================

/// Entry point request
///
/// Allows specifying a custom kernel entry point.
///
/// # Example
///
/// ```rust,no_run
/// use helix_limine::requests::EntryPointRequest;
///
/// extern "C" fn kernel_main() -> ! {
///     loop {}
/// }
///
/// #[used]
/// #[link_section = ".limine_requests"]
/// static ENTRY_POINT: EntryPointRequest = EntryPointRequest::new(kernel_main);
/// ```
#[repr(C)]
pub struct EntryPointRequest {
    /// Request identifier
    id: [u64; 4],
    /// Protocol revision
    revision: u64,
    /// Response pointer
    response: ResponsePtr<EntryPointResponse>,
    /// Entry point function
    entry: EntryPointFn,
}

impl EntryPointRequest {
    /// Create a new entry point request
    pub const fn new(entry: EntryPointFn) -> Self {
        Self {
            id: ENTRY_POINT_ID,
            revision: 0,
            response: ResponsePtr::null(),
            entry,
        }
    }
}

impl LimineRequest for EntryPointRequest {
    type Response = EntryPointResponse;

    fn id(&self) -> [u64; 4] {
        self.id
    }
    fn revision(&self) -> u64 {
        self.revision
    }
    fn has_response(&self) -> bool {
        self.response.is_available()
    }
    fn response(&self) -> Option<&Self::Response> {
        unsafe { self.response.get() }
    }
}

unsafe impl Sync for EntryPointRequest {}

/// Entry point response
#[repr(C)]
pub struct EntryPointResponse {
    /// Response revision
    revision: u64,
}

impl EntryPointResponse {
    /// Get the response revision
    pub fn revision(&self) -> u64 {
        self.revision
    }
}

unsafe impl SafeResponse for EntryPointResponse {
    fn validate(&self) -> bool {
        true
    }
}

impl core::fmt::Debug for EntryPointResponse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EntryPointResponse")
            .field("revision", &self.revision)
            .finish()
    }
}

// =============================================================================
// Stack Size Request
// =============================================================================

/// Stack size request
///
/// Allows specifying a custom stack size for the kernel.
///
/// # Example
///
/// ```rust,no_run
/// use helix_limine::requests::StackSizeRequest;
///
/// // Request a 256 KB stack
/// #[used]
/// #[link_section = ".limine_requests"]
/// static STACK_SIZE: StackSizeRequest = StackSizeRequest::new(256 * 1024);
/// ```
#[repr(C)]
pub struct StackSizeRequest {
    /// Request identifier
    id: [u64; 4],
    /// Protocol revision
    revision: u64,
    /// Response pointer
    response: ResponsePtr<StackSizeResponse>,
    /// Requested stack size
    stack_size: u64,
}

impl StackSizeRequest {
    /// Create a new stack size request
    pub const fn new(stack_size: u64) -> Self {
        Self {
            id: STACK_SIZE_ID,
            revision: 0,
            response: ResponsePtr::null(),
            stack_size,
        }
    }

    /// Create with default stack size (64 KB)
    pub const fn default_size() -> Self {
        Self::new(64 * 1024)
    }

    /// Create with 128 KB stack
    pub const fn size_128k() -> Self {
        Self::new(128 * 1024)
    }

    /// Create with 256 KB stack
    pub const fn size_256k() -> Self {
        Self::new(256 * 1024)
    }

    /// Create with 512 KB stack
    pub const fn size_512k() -> Self {
        Self::new(512 * 1024)
    }

    /// Create with 1 MB stack
    pub const fn size_1m() -> Self {
        Self::new(1024 * 1024)
    }
}

impl Default for StackSizeRequest {
    fn default() -> Self {
        Self::default_size()
    }
}

impl LimineRequest for StackSizeRequest {
    type Response = StackSizeResponse;

    fn id(&self) -> [u64; 4] {
        self.id
    }
    fn revision(&self) -> u64 {
        self.revision
    }
    fn has_response(&self) -> bool {
        self.response.is_available()
    }
    fn response(&self) -> Option<&Self::Response> {
        unsafe { self.response.get() }
    }
}

unsafe impl Sync for StackSizeRequest {}

/// Stack size response
#[repr(C)]
pub struct StackSizeResponse {
    /// Response revision
    revision: u64,
}

impl StackSizeResponse {
    /// Get the response revision
    pub fn revision(&self) -> u64 {
        self.revision
    }
}

unsafe impl SafeResponse for StackSizeResponse {
    fn validate(&self) -> bool {
        true
    }
}

impl core::fmt::Debug for StackSizeResponse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StackSizeResponse")
            .field("revision", &self.revision)
            .finish()
    }
}

// =============================================================================
// Boot Time Request
// =============================================================================

/// Boot time request
///
/// Provides the boot time as a UNIX timestamp.
///
/// # Example
///
/// ```rust,no_run
/// use helix_limine::requests::BootTimeRequest;
///
/// #[used]
/// #[link_section = ".limine_requests"]
/// static BOOT_TIME: BootTimeRequest = BootTimeRequest::new();
///
/// fn print_boot_time() {
///     if let Some(time) = BOOT_TIME.response() {
///         // time.timestamp() is seconds since 1970-01-01
///     }
/// }
/// ```
#[repr(C)]
pub struct BootTimeRequest {
    /// Request identifier
    id: [u64; 4],
    /// Protocol revision
    revision: u64,
    /// Response pointer
    response: ResponsePtr<BootTimeResponse>,
}

impl BootTimeRequest {
    /// Create a new boot time request
    pub const fn new() -> Self {
        Self {
            id: BOOT_TIME_ID,
            revision: 0,
            response: ResponsePtr::null(),
        }
    }
}

impl Default for BootTimeRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl LimineRequest for BootTimeRequest {
    type Response = BootTimeResponse;

    fn id(&self) -> [u64; 4] {
        self.id
    }
    fn revision(&self) -> u64 {
        self.revision
    }
    fn has_response(&self) -> bool {
        self.response.is_available()
    }
    fn response(&self) -> Option<&Self::Response> {
        unsafe { self.response.get() }
    }
}

unsafe impl Sync for BootTimeRequest {}

/// Boot time response
#[repr(C)]
pub struct BootTimeResponse {
    /// Response revision
    revision: u64,
    /// Boot time (UNIX timestamp)
    boot_time: i64,
}

impl BootTimeResponse {
    /// Get the boot time as a UNIX timestamp
    pub fn timestamp(&self) -> i64 {
        self.boot_time
    }

    /// Get the response revision
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Get seconds since boot (requires current time)
    pub fn uptime_seconds(&self, current_time: i64) -> i64 {
        current_time - self.boot_time
    }

    /// Format as a simple date/time structure
    pub fn as_datetime(&self) -> DateTime {
        DateTime::from_timestamp(self.boot_time)
    }
}

unsafe impl SafeResponse for BootTimeResponse {
    fn validate(&self) -> bool {
        // Boot time should be after year 2000
        self.boot_time > 946684800
    }
}

impl core::fmt::Debug for BootTimeResponse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BootTimeResponse")
            .field("timestamp", &self.boot_time)
            .field("datetime", &self.as_datetime())
            .finish()
    }
}

/// Simple date/time structure
#[derive(Debug, Clone, Copy)]
pub struct DateTime {
    /// Year (e.g., 2024)
    pub year: u16,
    /// Month (1-12)
    pub month: u8,
    /// Day of month (1-31)
    pub day: u8,
    /// Hour (0-23)
    pub hour: u8,
    /// Minute (0-59)
    pub minute: u8,
    /// Second (0-59)
    pub second: u8,
}

impl DateTime {
    /// Convert from UNIX timestamp
    pub fn from_timestamp(timestamp: i64) -> Self {
        // Simple implementation - doesn't handle leap seconds
        let mut days = (timestamp / 86400) as i32;
        let time_of_day = (timestamp % 86400) as u32;

        let mut year = 1970i32;

        loop {
            let days_in_year = if Self::is_leap_year(year) { 366 } else { 365 };
            if days < days_in_year {
                break;
            }
            days -= days_in_year;
            year += 1;
        }

        let leap = Self::is_leap_year(year);
        let days_in_month = [
            31,
            if leap { 29 } else { 28 },
            31,
            30,
            31,
            30,
            31,
            31,
            30,
            31,
            30,
            31,
        ];

        let mut month = 0u8;
        for (i, &dim) in days_in_month.iter().enumerate() {
            if days < dim {
                month = i as u8 + 1;
                break;
            }
            days -= dim;
        }

        Self {
            year: year as u16,
            month,
            day: days as u8 + 1,
            hour: (time_of_day / 3600) as u8,
            minute: ((time_of_day % 3600) / 60) as u8,
            second: (time_of_day % 60) as u8,
        }
    }

    /// Check if a year is a leap year
    fn is_leap_year(year: i32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }
}

impl core::fmt::Display for DateTime {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datetime_conversion() {
        // 2024-01-01 00:00:00 = 1704067200
        let dt = DateTime::from_timestamp(1704067200);
        assert_eq!(dt.year, 2024);
        assert_eq!(dt.month, 1);
        assert_eq!(dt.day, 1);
        assert_eq!(dt.hour, 0);
        assert_eq!(dt.minute, 0);
        assert_eq!(dt.second, 0);
    }

    #[test]
    fn test_leap_year() {
        assert!(DateTime::is_leap_year(2000));
        assert!(DateTime::is_leap_year(2024));
        assert!(!DateTime::is_leap_year(2023));
        assert!(!DateTime::is_leap_year(1900));
    }
}
