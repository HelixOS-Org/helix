//! UEFI Runtime Services Wrapper
//!
//! Wrapper for UEFI runtime services that persist after ExitBootServices.

use core::fmt;

// =============================================================================
// RUNTIME SERVICES TABLE
// =============================================================================

/// Runtime services revision
pub const RUNTIME_SERVICES_REVISION: u32 = (2 << 16) | 100; // 2.100

/// Runtime services table
#[repr(C)]
pub struct RuntimeServicesTable {
    /// Header
    pub header: TableHeader,
    // Time services
    pub get_time: usize,
    pub set_time: usize,
    pub get_wakeup_time: usize,
    pub set_wakeup_time: usize,
    // Virtual memory services
    pub set_virtual_address_map: usize,
    pub convert_pointer: usize,
    // Variable services
    pub get_variable: usize,
    pub get_next_variable_name: usize,
    pub set_variable: usize,
    // Miscellaneous services
    pub get_next_high_mono_count: usize,
    pub reset_system: usize,
    // Capsule services (UEFI 2.0+)
    pub update_capsule: usize,
    pub query_capsule_capabilities: usize,
    // Variable information (UEFI 2.0+)
    pub query_variable_info: usize,
}

/// Table header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TableHeader {
    pub signature: u64,
    pub revision: u32,
    pub header_size: u32,
    pub crc32: u32,
    pub reserved: u32,
}

impl TableHeader {
    /// Runtime services signature
    pub const RUNTIME_SERVICES_SIGNATURE: u64 = 0x56524553544E5552; // "RUNTSERV"
}

// =============================================================================
// TIME SERVICES
// =============================================================================

/// EFI time
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct EfiTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub pad1: u8,
    pub nanosecond: u32,
    pub timezone: i16,
    pub daylight: u8,
    pub pad2: u8,
}

impl EfiTime {
    /// Unspecified timezone
    pub const TIMEZONE_UNSPECIFIED: i16 = 0x07FF;

    /// Daylight savings time adjustment
    pub const DAYLIGHT_ADJUST: u8 = 0x01;
    /// Time is in daylight savings time
    pub const DAYLIGHT_TIME: u8 = 0x02;

    /// Create new time
    pub fn new(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Self {
        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            pad1: 0,
            nanosecond: 0,
            timezone: Self::TIMEZONE_UNSPECIFIED,
            daylight: 0,
            pad2: 0,
        }
    }

    /// Is valid
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
            && self.nanosecond <= 999_999_999
    }

    /// Days in month
    pub fn days_in_month(&self) -> u8 {
        match self.month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if self.is_leap_year() {
                    29
                } else {
                    28
                }
            },
            _ => 0,
        }
    }

    /// Is leap year
    pub fn is_leap_year(&self) -> bool {
        (self.year % 4 == 0 && self.year % 100 != 0) || (self.year % 400 == 0)
    }

    /// Day of year (1-366)
    pub fn day_of_year(&self) -> u16 {
        let mut day = self.day as u16;

        for m in 1..self.month {
            day += match m {
                1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                4 | 6 | 9 | 11 => 30,
                2 => {
                    if self.is_leap_year() {
                        29
                    } else {
                        28
                    }
                },
                _ => 0,
            };
        }

        day
    }

    /// Day of week (0 = Sunday, 6 = Saturday)
    pub fn day_of_week(&self) -> u8 {
        // Zeller's algorithm
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
        let h = if h < 0 { h + 7 } else { h };

        ((h + 6) % 7) as u8 // Adjust to Sunday = 0
    }

    /// Convert to Unix timestamp (seconds since 1970-01-01 00:00:00 UTC)
    pub fn to_unix_timestamp(&self) -> i64 {
        let mut days: i64 = 0;

        // Years since 1970
        for y in 1970..self.year {
            days += if (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0) {
                366
            } else {
                365
            };
        }

        // Days in this year
        days += self.day_of_year() as i64 - 1;

        // Convert to seconds
        let mut secs = days * 86400;
        secs += self.hour as i64 * 3600;
        secs += self.minute as i64 * 60;
        secs += self.second as i64;

        // Apply timezone
        if self.timezone != Self::TIMEZONE_UNSPECIFIED {
            secs -= self.timezone as i64 * 60;
        }

        secs
    }

    /// Create from Unix timestamp
    pub fn from_unix_timestamp(timestamp: i64) -> Self {
        let mut secs = timestamp;

        // Extract time
        let second = (secs % 60) as u8;
        secs /= 60;
        let minute = (secs % 60) as u8;
        secs /= 60;
        let hour = (secs % 24) as u8;
        let mut days = secs / 24;

        // Calculate year
        let mut year = 1970u16;
        loop {
            let days_in_year = if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                366
            } else {
                365
            };

            if days < days_in_year {
                break;
            }

            days -= days_in_year;
            year += 1;
        }

        // Calculate month and day
        let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let days_in_months = [
            31,
            if is_leap { 29 } else { 28 },
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

        let mut month = 1u8;
        for &dim in &days_in_months {
            if days < dim {
                break;
            }
            days -= dim;
            month += 1;
        }

        let day = (days + 1) as u8;

        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            pad1: 0,
            nanosecond: 0,
            timezone: 0,
            daylight: 0,
            pad2: 0,
        }
    }
}

impl fmt::Display for EfiTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

/// Time capabilities
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct TimeCapabilities {
    /// Resolution in counts per second
    pub resolution: u32,
    /// Accuracy in parts per million
    pub accuracy: u32,
    /// Sets to zero on set_time
    pub sets_to_zero: bool,
}

// =============================================================================
// VARIABLE SERVICES
// =============================================================================

/// Variable attributes
pub mod var_attrs {
    /// Non-volatile
    pub const NON_VOLATILE: u32 = 0x00000001;
    /// Bootservice access
    pub const BOOTSERVICE_ACCESS: u32 = 0x00000002;
    /// Runtime access
    pub const RUNTIME_ACCESS: u32 = 0x00000004;
    /// Hardware error record
    pub const HARDWARE_ERROR_RECORD: u32 = 0x00000008;
    /// Authenticated write access (deprecated)
    pub const AUTHENTICATED_WRITE_ACCESS: u32 = 0x00000010;
    /// Time based authenticated write access
    pub const TIME_BASED_AUTHENTICATED_WRITE_ACCESS: u32 = 0x00000020;
    /// Append write
    pub const APPEND_WRITE: u32 = 0x00000040;
    /// Enhanced authenticated access
    pub const ENHANCED_AUTHENTICATED_ACCESS: u32 = 0x00000080;
}

/// Common variable names
pub mod var_names {
    /// Language codes
    pub const LANG_CODES: &[u16] = &[
        'L' as u16, 'a' as u16, 'n' as u16, 'g' as u16, 'C' as u16, 'o' as u16, 'd' as u16,
        'e' as u16, 's' as u16, 0,
    ];

    /// Platform language codes
    pub const PLATFORM_LANG_CODES: &[u16] = &[
        'P' as u16, 'l' as u16, 'a' as u16, 't' as u16, 'f' as u16, 'o' as u16, 'r' as u16,
        'm' as u16, 'L' as u16, 'a' as u16, 'n' as u16, 'g' as u16, 'C' as u16, 'o' as u16,
        'd' as u16, 'e' as u16, 's' as u16, 0,
    ];

    /// Timeout
    pub const TIMEOUT: &[u16] = &[
        'T' as u16, 'i' as u16, 'm' as u16, 'e' as u16, 'o' as u16, 'u' as u16, 't' as u16, 0,
    ];

    /// Boot order
    pub const BOOT_ORDER: &[u16] = &[
        'B' as u16, 'o' as u16, 'o' as u16, 't' as u16, 'O' as u16, 'r' as u16, 'd' as u16,
        'e' as u16, 'r' as u16, 0,
    ];

    /// Boot current
    pub const BOOT_CURRENT: &[u16] = &[
        'B' as u16, 'o' as u16, 'o' as u16, 't' as u16, 'C' as u16, 'u' as u16, 'r' as u16,
        'r' as u16, 'e' as u16, 'n' as u16, 't' as u16, 0,
    ];

    /// Boot next
    pub const BOOT_NEXT: &[u16] = &[
        'B' as u16, 'o' as u16, 'o' as u16, 't' as u16, 'N' as u16, 'e' as u16, 'x' as u16,
        't' as u16, 0,
    ];

    /// Console out device
    pub const CON_OUT_DEV: &[u16] = &[
        'C' as u16, 'o' as u16, 'n' as u16, 'O' as u16, 'u' as u16, 't' as u16, 'D' as u16,
        'e' as u16, 'v' as u16, 0,
    ];

    /// Secure boot
    pub const SECURE_BOOT: &[u16] = &[
        'S' as u16, 'e' as u16, 'c' as u16, 'u' as u16, 'r' as u16, 'e' as u16, 'B' as u16,
        'o' as u16, 'o' as u16, 't' as u16, 0,
    ];

    /// Setup mode
    pub const SETUP_MODE: &[u16] = &[
        'S' as u16, 'e' as u16, 't' as u16, 'u' as u16, 'p' as u16, 'M' as u16, 'o' as u16,
        'd' as u16, 'e' as u16, 0,
    ];
}

/// Global variable GUID
pub const EFI_GLOBAL_VARIABLE_GUID: [u8; 16] = [
    0x61, 0xDF, 0xE4, 0x8B, 0xCA, 0x93, 0xD2, 0x11, 0xAA, 0x0D, 0x00, 0xE0, 0x98, 0x03, 0x2B, 0x8C,
];

/// Variable info
#[derive(Debug, Clone)]
pub struct VariableInfo {
    /// Variable name
    pub name: [u16; 128],
    pub name_len: usize,
    /// Vendor GUID
    pub vendor_guid: [u8; 16],
    /// Attributes
    pub attributes: u32,
    /// Data size
    pub data_size: usize,
}

impl VariableInfo {
    /// Create new variable info
    pub fn new() -> Self {
        Self {
            name: [0; 128],
            name_len: 0,
            vendor_guid: [0; 16],
            attributes: 0,
            data_size: 0,
        }
    }

    /// Is non-volatile
    pub fn is_non_volatile(&self) -> bool {
        self.attributes & var_attrs::NON_VOLATILE != 0
    }

    /// Is runtime accessible
    pub fn is_runtime(&self) -> bool {
        self.attributes & var_attrs::RUNTIME_ACCESS != 0
    }

    /// Is boot service accessible
    pub fn is_boot_service(&self) -> bool {
        self.attributes & var_attrs::BOOTSERVICE_ACCESS != 0
    }
}

impl Default for VariableInfo {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// RESET SERVICES
// =============================================================================

/// Reset type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ResetType {
    /// Cold reset
    Cold             = 0,
    /// Warm reset
    Warm             = 1,
    /// Shutdown
    Shutdown         = 2,
    /// Platform specific reset
    PlatformSpecific = 3,
}

// =============================================================================
// CAPSULE SERVICES
// =============================================================================

/// Capsule header
#[repr(C)]
pub struct CapsuleHeader {
    /// Capsule GUID
    pub capsule_guid: [u8; 16],
    /// Header size
    pub header_size: u32,
    /// Flags
    pub flags: u32,
    /// Capsule image size
    pub capsule_image_size: u32,
}

/// Capsule flags
pub mod capsule_flags {
    /// Persist across reset
    pub const PERSIST_ACROSS_RESET: u32 = 0x00010000;
    /// Populate system table
    pub const POPULATE_SYSTEM_TABLE: u32 = 0x00020000;
    /// Initiate reset
    pub const INITIATE_RESET: u32 = 0x00040000;
}

// =============================================================================
// RUNTIME SERVICES WRAPPER
// =============================================================================

/// Runtime services wrapper
pub struct RuntimeServices {
    /// Table pointer
    table: *const RuntimeServicesTable,
    /// Virtual address mode
    virtual_mode: bool,
}

impl RuntimeServices {
    /// Create from table pointer
    pub unsafe fn from_table(table: *const RuntimeServicesTable) -> Self {
        Self {
            table,
            virtual_mode: false,
        }
    }

    /// Get current time
    pub fn get_time(&self) -> Result<(EfiTime, Option<TimeCapabilities>), RuntimeError> {
        // Would call runtime_services.get_time
        Ok((EfiTime::default(), None))
    }

    /// Set current time
    pub fn set_time(&self, time: &EfiTime) -> Result<(), RuntimeError> {
        if !time.is_valid() {
            return Err(RuntimeError::InvalidParameter);
        }

        // Would call runtime_services.set_time
        Ok(())
    }

    /// Get wakeup time
    pub fn get_wakeup_time(&self) -> Result<(bool, bool, EfiTime), RuntimeError> {
        // Would call runtime_services.get_wakeup_time
        // Returns (enabled, pending, time)
        Ok((false, false, EfiTime::default()))
    }

    /// Set wakeup time
    pub fn set_wakeup_time(
        &self,
        _enable: bool,
        _time: Option<&EfiTime>,
    ) -> Result<(), RuntimeError> {
        // Would call runtime_services.set_wakeup_time
        Ok(())
    }

    /// Get variable
    pub fn get_variable(
        &self,
        _name: &[u16],
        _vendor_guid: &[u8; 16],
        _buffer: &mut [u8],
    ) -> Result<(usize, u32), RuntimeError> {
        // Would call runtime_services.get_variable
        // Returns (data_size, attributes)
        Ok((0, 0))
    }

    /// Set variable
    pub fn set_variable(
        &self,
        _name: &[u16],
        _vendor_guid: &[u8; 16],
        _attributes: u32,
        _data: &[u8],
    ) -> Result<(), RuntimeError> {
        // Would call runtime_services.set_variable
        Ok(())
    }

    /// Delete variable
    pub fn delete_variable(
        &self,
        name: &[u16],
        vendor_guid: &[u8; 16],
    ) -> Result<(), RuntimeError> {
        self.set_variable(name, vendor_guid, 0, &[])
    }

    /// Get next variable name
    pub fn get_next_variable_name(
        &self,
        _name: &mut [u16],
        _name_size: &mut usize,
        _vendor_guid: &mut [u8; 16],
    ) -> Result<bool, RuntimeError> {
        // Would call runtime_services.get_next_variable_name
        // Returns true if more variables, false if done
        Ok(false)
    }

    /// Enumerate all variables
    pub fn enumerate_variables<F>(&self, mut callback: F) -> Result<(), RuntimeError>
    where
        F: FnMut(&VariableInfo) -> bool,
    {
        let mut name = [0u16; 128];
        let mut name_size = 128usize;
        let mut vendor_guid = [0u8; 16];

        // Start with empty name
        name[0] = 0;

        loop {
            match self.get_next_variable_name(&mut name, &mut name_size, &mut vendor_guid) {
                Ok(true) => {
                    let mut info = VariableInfo::new();
                    info.name = name;
                    info.name_len = name_size;
                    info.vendor_guid = vendor_guid;

                    // Get variable attributes
                    let mut buffer = [0u8; 1];
                    if let Ok((size, attrs)) =
                        self.get_variable(&name[..name_size], &vendor_guid, &mut buffer)
                    {
                        info.data_size = size;
                        info.attributes = attrs;
                    }

                    if !callback(&info) {
                        break;
                    }
                },
                Ok(false) => break,
                Err(e) => return Err(e),
            }
        }

        Ok(())
    }

    /// Query variable info
    pub fn query_variable_info(
        &self,
        _attributes: u32,
    ) -> Result<VariableStorageInfo, RuntimeError> {
        // Would call runtime_services.query_variable_info
        Ok(VariableStorageInfo {
            maximum_variable_storage_size: 0,
            remaining_variable_storage_size: 0,
            maximum_variable_size: 0,
        })
    }

    /// Reset system
    pub fn reset_system(&self, _reset_type: ResetType, _status: usize, _data: Option<&[u8]>) -> ! {
        // Would call runtime_services.reset_system
        // This function never returns
        loop {
            core::hint::spin_loop();
        }
    }

    /// Get next high monotonic count
    pub fn get_next_high_monotonic_count(&self) -> Result<u32, RuntimeError> {
        // Would call runtime_services.get_next_high_mono_count
        Ok(0)
    }

    /// Update capsule
    pub fn update_capsule(
        &self,
        _capsules: &[&CapsuleHeader],
        scatter_gather_list: usize,
    ) -> Result<(), RuntimeError> {
        // Would call runtime_services.update_capsule
        Ok(())
    }

    /// Query capsule capabilities
    pub fn query_capsule_capabilities(
        &self,
        capsules: &[&CapsuleHeader],
    ) -> Result<CapsuleCapabilities, RuntimeError> {
        // Would call runtime_services.query_capsule_capabilities
        Ok(CapsuleCapabilities {
            maximum_capsule_size: 0,
            reset_type: ResetType::Cold,
        })
    }

    /// Set virtual address map
    pub fn set_virtual_address_map(
        &mut self,
        memory_map: &[MemoryDescriptor],
        descriptor_size: usize,
        descriptor_version: u32,
    ) -> Result<(), RuntimeError> {
        // Would call runtime_services.set_virtual_address_map
        self.virtual_mode = true;
        Ok(())
    }

    /// Convert pointer
    pub fn convert_pointer(
        &self,
        debug_disposition: u32,
        address: &mut usize,
    ) -> Result<(), RuntimeError> {
        // Would call runtime_services.convert_pointer
        Ok(())
    }

    /// Is in virtual mode
    pub fn is_virtual_mode(&self) -> bool {
        self.virtual_mode
    }
}

/// Variable storage info
#[derive(Debug, Clone, Copy)]
pub struct VariableStorageInfo {
    /// Maximum storage size
    pub maximum_variable_storage_size: u64,
    /// Remaining storage size
    pub remaining_variable_storage_size: u64,
    /// Maximum variable size
    pub maximum_variable_size: u64,
}

/// Capsule capabilities
#[derive(Debug, Clone, Copy)]
pub struct CapsuleCapabilities {
    /// Maximum capsule size
    pub maximum_capsule_size: u64,
    /// Required reset type
    pub reset_type: ResetType,
}

/// Memory descriptor for virtual address mapping
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MemoryDescriptor {
    /// Memory type
    pub memory_type: u32,
    /// Physical start
    pub physical_start: u64,
    /// Virtual start
    pub virtual_start: u64,
    /// Number of pages
    pub number_of_pages: u64,
    /// Attributes
    pub attribute: u64,
}

// =============================================================================
// CONVENIENCE FUNCTIONS
// =============================================================================

/// Get boot order variable
pub fn get_boot_order(rt: &RuntimeServices) -> Result<[u16; 64], RuntimeError> {
    let mut buffer = [0u8; 128];
    let (size, _) = rt.get_variable(
        var_names::BOOT_ORDER,
        &EFI_GLOBAL_VARIABLE_GUID,
        &mut buffer,
    )?;

    let mut order = [0u16; 64];
    let count = (size / 2).min(64);

    for i in 0..count {
        order[i] = u16::from_le_bytes([buffer[i * 2], buffer[i * 2 + 1]]);
    }

    Ok(order)
}

/// Get boot timeout
pub fn get_boot_timeout(rt: &RuntimeServices) -> Result<u16, RuntimeError> {
    let mut buffer = [0u8; 2];
    rt.get_variable(var_names::TIMEOUT, &EFI_GLOBAL_VARIABLE_GUID, &mut buffer)?;
    Ok(u16::from_le_bytes(buffer))
}

/// Set boot timeout
pub fn set_boot_timeout(rt: &RuntimeServices, timeout: u16) -> Result<(), RuntimeError> {
    rt.set_variable(
        var_names::TIMEOUT,
        &EFI_GLOBAL_VARIABLE_GUID,
        var_attrs::NON_VOLATILE | var_attrs::BOOTSERVICE_ACCESS | var_attrs::RUNTIME_ACCESS,
        &timeout.to_le_bytes(),
    )
}

/// Check if secure boot is enabled
pub fn is_secure_boot_enabled(rt: &RuntimeServices) -> Result<bool, RuntimeError> {
    let mut buffer = [0u8; 1];
    rt.get_variable(
        var_names::SECURE_BOOT,
        &EFI_GLOBAL_VARIABLE_GUID,
        &mut buffer,
    )?;
    Ok(buffer[0] != 0)
}

/// Check if in setup mode
pub fn is_setup_mode(rt: &RuntimeServices) -> Result<bool, RuntimeError> {
    let mut buffer = [0u8; 1];
    rt.get_variable(
        var_names::SETUP_MODE,
        &EFI_GLOBAL_VARIABLE_GUID,
        &mut buffer,
    )?;
    Ok(buffer[0] != 0)
}

// =============================================================================
// RUNTIME ERROR
// =============================================================================

/// Runtime error
#[derive(Debug, Clone)]
pub enum RuntimeError {
    /// Invalid parameter
    InvalidParameter,
    /// Not found
    NotFound,
    /// Buffer too small
    BufferTooSmall,
    /// Device error
    DeviceError,
    /// Write protected
    WriteProtected,
    /// Out of resources
    OutOfResources,
    /// Unsupported
    Unsupported,
    /// Security violation
    SecurityViolation,
    /// Not ready
    NotReady,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParameter => write!(f, "invalid parameter"),
            Self::NotFound => write!(f, "not found"),
            Self::BufferTooSmall => write!(f, "buffer too small"),
            Self::DeviceError => write!(f, "device error"),
            Self::WriteProtected => write!(f, "write protected"),
            Self::OutOfResources => write!(f, "out of resources"),
            Self::Unsupported => write!(f, "unsupported"),
            Self::SecurityViolation => write!(f, "security violation"),
            Self::NotReady => write!(f, "not ready"),
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_efi_time() {
        let time = EfiTime::new(2024, 6, 15, 10, 30, 45);
        assert!(time.is_valid());
        assert_eq!(time.day_of_week(), 6); // Saturday
    }

    #[test]
    fn test_unix_timestamp() {
        let time = EfiTime::new(2024, 1, 1, 0, 0, 0);
        let ts = time.to_unix_timestamp();

        let restored = EfiTime::from_unix_timestamp(ts);
        assert_eq!(restored.year, 2024);
        assert_eq!(restored.month, 1);
        assert_eq!(restored.day, 1);
    }

    #[test]
    fn test_leap_year() {
        let time = EfiTime::new(2024, 2, 29, 0, 0, 0);
        assert!(time.is_leap_year());
        assert_eq!(time.days_in_month(), 29);

        let time2 = EfiTime::new(2023, 2, 28, 0, 0, 0);
        assert!(!time2.is_leap_year());
        assert_eq!(time2.days_in_month(), 28);
    }
}
