//! Raw UEFI type definitions
//!
//! This module contains all the fundamental types used throughout the UEFI
//! specification. These types are designed to be binary-compatible with the
//! UEFI ABI.

use core::fmt;
use core::ptr::NonNull;

// =============================================================================
// BASIC TYPES
// =============================================================================

/// UEFI Boolean type (1 byte)
pub type Boolean = u8;

/// UEFI INTN type (native signed integer)
pub type Intn = isize;

/// UEFI UINTN type (native unsigned integer)
pub type Uintn = usize;

/// UEFI INT8 type
pub type Int8 = i8;

/// UEFI UINT8 type
pub type Uint8 = u8;

/// UEFI INT16 type
pub type Int16 = i16;

/// UEFI UINT16 type
pub type Uint16 = u16;

/// UEFI INT32 type
pub type Int32 = i32;

/// UEFI UINT32 type
pub type Uint32 = u32;

/// UEFI INT64 type
pub type Int64 = i64;

/// UEFI UINT64 type
pub type Uint64 = u64;

/// UEFI CHAR8 type (ASCII character)
pub type Char8 = u8;

/// UEFI CHAR16 type (UCS-2 character)
pub type Char16 = u16;

/// Physical address type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct PhysicalAddress(pub u64);

impl PhysicalAddress {
    /// Null physical address
    pub const NULL: Self = Self(0);

    /// Create a new physical address
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Get the raw address value
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Check if this is a null address
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    /// Add an offset to the address
    pub const fn add(self, offset: u64) -> Self {
        Self(self.0 + offset)
    }

    /// Subtract an offset from the address
    pub const fn sub(self, offset: u64) -> Self {
        Self(self.0 - offset)
    }

    /// Align up to the given alignment
    pub const fn align_up(self, align: u64) -> Self {
        let mask = align - 1;
        Self((self.0 + mask) & !mask)
    }

    /// Align down to the given alignment
    pub const fn align_down(self, align: u64) -> Self {
        let mask = align - 1;
        Self(self.0 & !mask)
    }

    /// Check if aligned to the given boundary
    pub const fn is_aligned(self, align: u64) -> bool {
        (self.0 & (align - 1)) == 0
    }
}

impl fmt::Display for PhysicalAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:016X}", self.0)
    }
}

impl core::ops::Add<u64> for PhysicalAddress {
    type Output = Self;
    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl core::ops::Sub<u64> for PhysicalAddress {
    type Output = Self;
    fn sub(self, rhs: u64) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl core::ops::Sub<PhysicalAddress> for PhysicalAddress {
    type Output = u64;
    fn sub(self, rhs: PhysicalAddress) -> Self::Output {
        self.0 - rhs.0
    }
}

impl core::ops::AddAssign<u64> for PhysicalAddress {
    fn add_assign(&mut self, rhs: u64) {
        self.0 += rhs;
    }
}

impl core::ops::SubAssign<u64> for PhysicalAddress {
    fn sub_assign(&mut self, rhs: u64) {
        self.0 -= rhs;
    }
}

impl From<u64> for PhysicalAddress {
    fn from(addr: u64) -> Self {
        Self(addr)
    }
}

impl From<PhysicalAddress> for u64 {
    fn from(addr: PhysicalAddress) -> Self {
        addr.0
    }
}

impl core::ops::BitAnd<u64> for PhysicalAddress {
    type Output = u64;
    fn bitand(self, rhs: u64) -> Self::Output {
        self.0 & rhs
    }
}

impl core::ops::BitOr<u64> for PhysicalAddress {
    type Output = u64;
    fn bitor(self, rhs: u64) -> Self::Output {
        self.0 | rhs
    }
}

impl core::ops::Shr<u32> for PhysicalAddress {
    type Output = u64;
    fn shr(self, rhs: u32) -> Self::Output {
        self.0 >> rhs
    }
}

impl core::ops::Shl<u32> for PhysicalAddress {
    type Output = u64;
    fn shl(self, rhs: u32) -> Self::Output {
        self.0 << rhs
    }
}

impl core::ops::Not for PhysicalAddress {
    type Output = u64;
    fn not(self) -> Self::Output {
        !self.0
    }
}

/// Virtual address type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VirtualAddress(pub u64);

impl VirtualAddress {
    /// Null virtual address
    pub const NULL: Self = Self(0);

    /// Create a new virtual address
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Get the raw address value
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Check if this is a null address
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    /// Add an offset to the address
    pub const fn add(self, offset: u64) -> Self {
        Self(self.0 + offset)
    }

    /// Subtract an offset from the address
    pub const fn sub(self, offset: u64) -> Self {
        Self(self.0 - offset)
    }

    /// Align up to the given alignment
    pub const fn align_up(self, align: u64) -> Self {
        let mask = align - 1;
        Self((self.0 + mask) & !mask)
    }

    /// Align down to the given alignment
    pub const fn align_down(self, align: u64) -> Self {
        let mask = align - 1;
        Self(self.0 & !mask)
    }

    /// Convert to pointer
    pub const fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }

    /// Convert to mutable pointer
    pub const fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }
}

impl fmt::Display for VirtualAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:016X}", self.0)
    }
}

impl core::ops::Add<u64> for VirtualAddress {
    type Output = Self;
    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl core::ops::Sub<u64> for VirtualAddress {
    type Output = Self;
    fn sub(self, rhs: u64) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl core::ops::Sub<VirtualAddress> for VirtualAddress {
    type Output = u64;
    fn sub(self, rhs: VirtualAddress) -> Self::Output {
        self.0 - rhs.0
    }
}

impl core::ops::AddAssign<u64> for VirtualAddress {
    fn add_assign(&mut self, rhs: u64) {
        self.0 += rhs;
    }
}

impl core::ops::SubAssign<u64> for VirtualAddress {
    fn sub_assign(&mut self, rhs: u64) {
        self.0 -= rhs;
    }
}

impl From<u64> for VirtualAddress {
    fn from(addr: u64) -> Self {
        Self(addr)
    }
}

impl From<VirtualAddress> for u64 {
    fn from(addr: VirtualAddress) -> Self {
        addr.0
    }
}

impl<T> From<*const T> for VirtualAddress {
    fn from(ptr: *const T) -> Self {
        Self(ptr as u64)
    }
}

impl<T> From<*mut T> for VirtualAddress {
    fn from(ptr: *mut T) -> Self {
        Self(ptr as u64)
    }
}

impl core::ops::BitAnd<u64> for VirtualAddress {
    type Output = u64;
    fn bitand(self, rhs: u64) -> Self::Output {
        self.0 & rhs
    }
}

impl core::ops::BitOr<u64> for VirtualAddress {
    type Output = u64;
    fn bitor(self, rhs: u64) -> Self::Output {
        self.0 | rhs
    }
}

impl core::ops::Shr<u32> for VirtualAddress {
    type Output = u64;
    fn shr(self, rhs: u32) -> Self::Output {
        self.0 >> rhs
    }
}

impl core::ops::Shr<i32> for VirtualAddress {
    type Output = u64;
    fn shr(self, rhs: i32) -> Self::Output {
        self.0 >> rhs
    }
}

impl core::ops::Shr<usize> for VirtualAddress {
    type Output = u64;
    fn shr(self, rhs: usize) -> Self::Output {
        self.0 >> rhs
    }
}

impl core::ops::Shl<u32> for VirtualAddress {
    type Output = u64;
    fn shl(self, rhs: u32) -> Self::Output {
        self.0 << rhs
    }
}

impl core::ops::Not for VirtualAddress {
    type Output = u64;
    fn not(self) -> Self::Output {
        !self.0
    }
}

// =============================================================================
// HANDLE
// =============================================================================

/// UEFI Handle - opaque pointer to protocol instances
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Handle(pub *mut core::ffi::c_void);

impl Handle {
    /// Null handle
    pub const NULL: Self = Self(core::ptr::null_mut());

    /// Create a null handle (function form)
    pub const fn null() -> Self {
        Self::NULL
    }

    /// Create a new handle from a raw pointer
    pub const fn new(ptr: *mut core::ffi::c_void) -> Self {
        Self(ptr)
    }

    /// Check if this is a null handle
    pub const fn is_null(self) -> bool {
        self.0.is_null()
    }

    /// Get the raw pointer
    pub const fn as_ptr(self) -> *mut core::ffi::c_void {
        self.0
    }

    /// Convert to NonNull if not null
    pub fn as_non_null(self) -> Option<NonNull<core::ffi::c_void>> {
        NonNull::new(self.0)
    }
}

impl fmt::Debug for Handle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Handle({:p})", self.0)
    }
}

impl Default for Handle {
    fn default() -> Self {
        Self::NULL
    }
}

// Handle is Send + Sync since UEFI is single-threaded during boot
unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

// =============================================================================
// STATUS
// =============================================================================

/// UEFI Status code
///
/// Status codes are used throughout UEFI to indicate success or failure.
/// The high bit indicates an error (1) vs warning/success (0).
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Status(pub u64);

impl Status {
    /// Success
    pub const SUCCESS: Self = Self(0);

    /// Warning: Unknown Glyph
    pub const WARN_UNKNOWN_GLYPH: Self = Self(1);
    pub const WARN_DELETE_FAILURE: Self = Self(2);
    pub const WARN_WRITE_FAILURE: Self = Self(3);
    pub const WARN_BUFFER_TOO_SMALL: Self = Self(4);
    pub const WARN_STALE_DATA: Self = Self(5);
    pub const WARN_FILE_SYSTEM: Self = Self(6);
    pub const WARN_RESET_REQUIRED: Self = Self(7);

    /// Error bit mask
    const ERROR_BIT: u64 = 1u64 << 63;

    /// Error: Load Error
    pub const LOAD_ERROR: Self = Self(Self::ERROR_BIT | 1);
    pub const INVALID_PARAMETER: Self = Self(Self::ERROR_BIT | 2);
    pub const UNSUPPORTED: Self = Self(Self::ERROR_BIT | 3);
    pub const BAD_BUFFER_SIZE: Self = Self(Self::ERROR_BIT | 4);
    pub const BUFFER_TOO_SMALL: Self = Self(Self::ERROR_BIT | 5);
    pub const NOT_READY: Self = Self(Self::ERROR_BIT | 6);
    pub const DEVICE_ERROR: Self = Self(Self::ERROR_BIT | 7);
    pub const WRITE_PROTECTED: Self = Self(Self::ERROR_BIT | 8);
    pub const OUT_OF_RESOURCES: Self = Self(Self::ERROR_BIT | 9);
    pub const VOLUME_CORRUPTED: Self = Self(Self::ERROR_BIT | 10);
    pub const VOLUME_FULL: Self = Self(Self::ERROR_BIT | 11);
    pub const NO_MEDIA: Self = Self(Self::ERROR_BIT | 12);
    pub const MEDIA_CHANGED: Self = Self(Self::ERROR_BIT | 13);
    pub const NOT_FOUND: Self = Self(Self::ERROR_BIT | 14);
    pub const ACCESS_DENIED: Self = Self(Self::ERROR_BIT | 15);
    pub const NO_RESPONSE: Self = Self(Self::ERROR_BIT | 16);
    pub const NO_MAPPING: Self = Self(Self::ERROR_BIT | 17);
    pub const TIMEOUT: Self = Self(Self::ERROR_BIT | 18);
    pub const NOT_STARTED: Self = Self(Self::ERROR_BIT | 19);
    pub const ALREADY_STARTED: Self = Self(Self::ERROR_BIT | 20);
    pub const ABORTED: Self = Self(Self::ERROR_BIT | 21);
    pub const ICMP_ERROR: Self = Self(Self::ERROR_BIT | 22);
    pub const TFTP_ERROR: Self = Self(Self::ERROR_BIT | 23);
    pub const PROTOCOL_ERROR: Self = Self(Self::ERROR_BIT | 24);
    pub const INCOMPATIBLE_VERSION: Self = Self(Self::ERROR_BIT | 25);
    pub const SECURITY_VIOLATION: Self = Self(Self::ERROR_BIT | 26);
    pub const CRC_ERROR: Self = Self(Self::ERROR_BIT | 27);
    pub const END_OF_MEDIA: Self = Self(Self::ERROR_BIT | 28);
    pub const END_OF_FILE: Self = Self(Self::ERROR_BIT | 31);
    pub const INVALID_LANGUAGE: Self = Self(Self::ERROR_BIT | 32);
    pub const COMPROMISED_DATA: Self = Self(Self::ERROR_BIT | 33);
    pub const IP_ADDRESS_CONFLICT: Self = Self(Self::ERROR_BIT | 34);
    pub const HTTP_ERROR: Self = Self(Self::ERROR_BIT | 35);

    /// Create a new status code
    pub const fn new(code: u64) -> Self {
        Self(code)
    }

    /// Check if this is a success status
    pub const fn is_success(self) -> bool {
        self.0 == 0
    }

    /// Check if this is an error status
    pub const fn is_error(self) -> bool {
        (self.0 & Self::ERROR_BIT) != 0
    }

    /// Check if this is a warning status
    pub const fn is_warning(self) -> bool {
        !self.is_success() && !self.is_error()
    }

    /// Get the status code value
    pub const fn code(self) -> u64 {
        self.0 & !Self::ERROR_BIT
    }

    /// Convert to Result with Status as error type
    pub fn to_status_result(self) -> Result<(), Self> {
        if self.is_success() {
            Ok(())
        } else {
            Err(self)
        }
    }

    /// Convert to Result with a value on success, Status as error type
    pub fn to_status_result_with<T>(self, value: T) -> Result<T, Self> {
        if self.is_success() {
            Ok(value)
        } else {
            Err(self)
        }
    }

    /// Convert to Result
    pub fn to_result(self) -> Result<(), crate::error::Error> {
        if self.is_success() {
            Ok(())
        } else {
            Err(crate::error::Error::from_status(self))
        }
    }

    /// Convert to Result with a value on success
    pub fn to_result_with<T>(self, value: T) -> Result<T, crate::error::Error> {
        if self.is_success() {
            Ok(value)
        } else {
            Err(crate::error::Error::from_status(self))
        }
    }
}

impl fmt::Debug for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match *self {
            Self::SUCCESS => "SUCCESS",
            Self::WARN_UNKNOWN_GLYPH => "WARN_UNKNOWN_GLYPH",
            Self::LOAD_ERROR => "LOAD_ERROR",
            Self::INVALID_PARAMETER => "INVALID_PARAMETER",
            Self::UNSUPPORTED => "UNSUPPORTED",
            Self::BAD_BUFFER_SIZE => "BAD_BUFFER_SIZE",
            Self::BUFFER_TOO_SMALL => "BUFFER_TOO_SMALL",
            Self::NOT_READY => "NOT_READY",
            Self::DEVICE_ERROR => "DEVICE_ERROR",
            Self::WRITE_PROTECTED => "WRITE_PROTECTED",
            Self::OUT_OF_RESOURCES => "OUT_OF_RESOURCES",
            Self::VOLUME_CORRUPTED => "VOLUME_CORRUPTED",
            Self::VOLUME_FULL => "VOLUME_FULL",
            Self::NO_MEDIA => "NO_MEDIA",
            Self::MEDIA_CHANGED => "MEDIA_CHANGED",
            Self::NOT_FOUND => "NOT_FOUND",
            Self::ACCESS_DENIED => "ACCESS_DENIED",
            Self::NO_RESPONSE => "NO_RESPONSE",
            Self::NO_MAPPING => "NO_MAPPING",
            Self::TIMEOUT => "TIMEOUT",
            Self::NOT_STARTED => "NOT_STARTED",
            Self::ALREADY_STARTED => "ALREADY_STARTED",
            Self::ABORTED => "ABORTED",
            Self::ICMP_ERROR => "ICMP_ERROR",
            Self::TFTP_ERROR => "TFTP_ERROR",
            Self::PROTOCOL_ERROR => "PROTOCOL_ERROR",
            Self::INCOMPATIBLE_VERSION => "INCOMPATIBLE_VERSION",
            Self::SECURITY_VIOLATION => "SECURITY_VIOLATION",
            Self::CRC_ERROR => "CRC_ERROR",
            Self::END_OF_MEDIA => "END_OF_MEDIA",
            Self::END_OF_FILE => "END_OF_FILE",
            Self::INVALID_LANGUAGE => "INVALID_LANGUAGE",
            Self::COMPROMISED_DATA => "COMPROMISED_DATA",
            Self::IP_ADDRESS_CONFLICT => "IP_ADDRESS_CONFLICT",
            Self::HTTP_ERROR => "HTTP_ERROR",
            _ => return write!(f, "Status(0x{:X})", self.0),
        };
        write!(f, "Status::{}", name)
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Debug>::fmt(self, f)
    }
}

impl Default for Status {
    fn default() -> Self {
        Self::SUCCESS
    }
}

// =============================================================================
// GUID
// =============================================================================

/// UEFI Globally Unique Identifier (GUID)
///
/// GUIDs are used throughout UEFI to identify protocols, tables, and
/// other objects.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Guid {
    /// First component (big-endian in textual representation)
    pub data1: u32,
    /// Second component
    pub data2: u16,
    /// Third component
    pub data3: u16,
    /// Fourth component (array of 8 bytes)
    pub data4: [u8; 8],
}

impl Guid {
    /// Null GUID
    pub const NULL: Self = Self {
        data1: 0,
        data2: 0,
        data3: 0,
        data4: [0; 8],
    };

    /// Create a GUID from its components
    pub const fn new(data1: u32, data2: u16, data3: u16, data4: [u8; 8]) -> Self {
        Self {
            data1,
            data2,
            data3,
            data4,
        }
    }

    /// Create a GUID from a 128-bit value
    pub const fn from_u128(value: u128) -> Self {
        let bytes = value.to_be_bytes();
        Self {
            data1: u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            data2: u16::from_be_bytes([bytes[4], bytes[5]]),
            data3: u16::from_be_bytes([bytes[6], bytes[7]]),
            data4: [
                bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                bytes[15],
            ],
        }
    }

    /// Convert to bytes in mixed-endian format
    pub const fn to_bytes(self) -> [u8; 16] {
        let d1 = self.data1.to_le_bytes();
        let d2 = self.data2.to_le_bytes();
        let d3 = self.data3.to_le_bytes();
        [
            d1[0],
            d1[1],
            d1[2],
            d1[3],
            d2[0],
            d2[1],
            d3[0],
            d3[1],
            self.data4[0],
            self.data4[1],
            self.data4[2],
            self.data4[3],
            self.data4[4],
            self.data4[5],
            self.data4[6],
            self.data4[7],
        ]
    }

    /// Create from bytes in mixed-endian format
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self {
            data1: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            data2: u16::from_le_bytes([bytes[4], bytes[5]]),
            data3: u16::from_le_bytes([bytes[6], bytes[7]]),
            data4: [
                bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                bytes[15],
            ],
        }
    }

    /// Check if this is the null GUID
    pub const fn is_null(self) -> bool {
        self.data1 == 0
            && self.data2 == 0
            && self.data3 == 0
            && self.data4[0] == 0
            && self.data4[1] == 0
            && self.data4[2] == 0
            && self.data4[3] == 0
            && self.data4[4] == 0
            && self.data4[5] == 0
            && self.data4[6] == 0
            && self.data4[7] == 0
    }
}

impl fmt::Debug for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Guid({})", self)
    }
}

impl fmt::Display for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
            self.data1,
            self.data2,
            self.data3,
            self.data4[0],
            self.data4[1],
            self.data4[2],
            self.data4[3],
            self.data4[4],
            self.data4[5],
            self.data4[6],
            self.data4[7]
        )
    }
}

// =============================================================================
// WELL-KNOWN GUIDS
// =============================================================================

/// Well-known UEFI GUIDs
pub mod guids {
    use super::Guid;

    /// EFI_LOADED_IMAGE_PROTOCOL_GUID
    pub const LOADED_IMAGE_PROTOCOL: Guid = Guid::new(0x5B1B31A1, 0x9562, 0x11D2, [
        0x8E, 0x3F, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B,
    ]);

    /// EFI_LOADED_IMAGE_DEVICE_PATH_PROTOCOL_GUID
    pub const LOADED_IMAGE_DEVICE_PATH_PROTOCOL: Guid = Guid::new(0xBC62157E, 0x3E33, 0x4FEC, [
        0x99, 0x20, 0x2D, 0x3B, 0x36, 0xD7, 0x50, 0xDF,
    ]);

    /// EFI_DEVICE_PATH_PROTOCOL_GUID
    pub const DEVICE_PATH_PROTOCOL: Guid = Guid::new(0x09576E91, 0x6D3F, 0x11D2, [
        0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B,
    ]);

    /// EFI_DEVICE_PATH_UTILITIES_PROTOCOL_GUID
    pub const DEVICE_PATH_UTILITIES_PROTOCOL: Guid = Guid::new(0x0379BE4E, 0xD706, 0x437D, [
        0xB0, 0x37, 0xED, 0xB8, 0x2F, 0xB7, 0x72, 0xA4,
    ]);

    /// EFI_DEVICE_PATH_TO_TEXT_PROTOCOL_GUID
    pub const DEVICE_PATH_TO_TEXT_PROTOCOL: Guid = Guid::new(0x8B843E20, 0x8132, 0x4852, [
        0x90, 0xCC, 0x55, 0x1A, 0x4E, 0x4A, 0x7F, 0x1C,
    ]);

    /// EFI_DEVICE_PATH_FROM_TEXT_PROTOCOL_GUID
    pub const DEVICE_PATH_FROM_TEXT_PROTOCOL: Guid = Guid::new(0x05C99A21, 0xC70F, 0x4AD2, [
        0x8A, 0x5F, 0x35, 0xDF, 0x33, 0x43, 0xF5, 0x1E,
    ]);

    /// EFI_HASH_PROTOCOL_GUID
    pub const HASH_PROTOCOL: Guid = Guid::new(0xC5184932, 0xDBA5, 0x46DB, [
        0xA5, 0xBA, 0xCC, 0x0B, 0xDA, 0x9C, 0x14, 0x35,
    ]);

    /// EFI_SIMPLE_TEXT_INPUT_PROTOCOL_GUID
    pub const SIMPLE_TEXT_INPUT_PROTOCOL: Guid = Guid::new(0x387477C1, 0x69C7, 0x11D2, [
        0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B,
    ]);

    /// EFI_SIMPLE_TEXT_INPUT_EX_PROTOCOL_GUID
    pub const SIMPLE_TEXT_INPUT_EX_PROTOCOL: Guid = Guid::new(0xDD9E7534, 0x7762, 0x4698, [
        0x8C, 0x14, 0xF5, 0x85, 0x17, 0xA6, 0x25, 0xAA,
    ]);

    /// EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL_GUID
    pub const SIMPLE_TEXT_OUTPUT_PROTOCOL: Guid = Guid::new(0x387477C2, 0x69C7, 0x11D2, [
        0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B,
    ]);

    /// EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID
    pub const GRAPHICS_OUTPUT_PROTOCOL: Guid = Guid::new(0x9042A9DE, 0x23DC, 0x4A38, [
        0x96, 0xFB, 0x7A, 0xDE, 0xD0, 0x80, 0x51, 0x6A,
    ]);

    /// EFI_EDID_ACTIVE_PROTOCOL_GUID
    pub const EDID_ACTIVE_PROTOCOL: Guid = Guid::new(0xBD8C1056, 0x9F36, 0x44EC, [
        0x92, 0xA8, 0xA6, 0x33, 0x7F, 0x81, 0x79, 0x86,
    ]);

    /// EFI_EDID_DISCOVERED_PROTOCOL_GUID
    pub const EDID_DISCOVERED_PROTOCOL: Guid = Guid::new(0x1C0C34F6, 0xD380, 0x41FA, [
        0xA0, 0x49, 0x8A, 0xD0, 0x6C, 0x1A, 0x66, 0xAA,
    ]);

    /// EFI_SIMPLE_FILE_SYSTEM_PROTOCOL_GUID
    pub const SIMPLE_FILE_SYSTEM_PROTOCOL: Guid = Guid::new(0x0964E5B22, 0x6459, 0x11D2, [
        0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B,
    ]);

    /// EFI_FILE_INFO_ID
    pub const FILE_INFO: Guid = Guid::new(0x09576E92, 0x6D3F, 0x11D2, [
        0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B,
    ]);

    /// EFI_FILE_SYSTEM_INFO_ID
    pub const FILE_SYSTEM_INFO: Guid = Guid::new(0x09576E93, 0x6D3F, 0x11D2, [
        0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B,
    ]);

    /// EFI_FILE_SYSTEM_VOLUME_LABEL_ID
    pub const FILE_SYSTEM_VOLUME_LABEL: Guid = Guid::new(0xDB47D7D3, 0xFE81, 0x11D3, [
        0x9A, 0x35, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
    ]);

    /// EFI_BLOCK_IO_PROTOCOL_GUID
    pub const BLOCK_IO_PROTOCOL: Guid = Guid::new(0x964E5B21, 0x6459, 0x11D2, [
        0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B,
    ]);

    /// EFI_BLOCK_IO2_PROTOCOL_GUID
    pub const BLOCK_IO2_PROTOCOL: Guid = Guid::new(0xA77B2472, 0xE282, 0x4E9F, [
        0xA2, 0x45, 0xC2, 0xC0, 0xE2, 0x7B, 0xBC, 0xC1,
    ]);

    /// EFI_DISK_IO_PROTOCOL_GUID
    pub const DISK_IO_PROTOCOL: Guid = Guid::new(0xCE345171, 0xBA0B, 0x11D2, [
        0x8E, 0x4F, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B,
    ]);

    /// EFI_DISK_IO2_PROTOCOL_GUID
    pub const DISK_IO2_PROTOCOL: Guid = Guid::new(0x151C8EAE, 0x7F2C, 0x472C, [
        0x9E, 0x54, 0x98, 0x28, 0x19, 0x4F, 0x6A, 0x88,
    ]);

    /// EFI_PARTITION_INFO_PROTOCOL_GUID
    pub const PARTITION_INFO_PROTOCOL: Guid = Guid::new(0x8CF2F62C, 0xBC9B, 0x4821, [
        0x80, 0x8D, 0xEC, 0x9E, 0xC4, 0x21, 0xA1, 0xA0,
    ]);

    /// EFI_PCI_IO_PROTOCOL_GUID
    pub const PCI_IO_PROTOCOL: Guid = Guid::new(0x4CF5B200, 0x68B8, 0x4CA5, [
        0x9E, 0xEC, 0xB2, 0x3E, 0x3F, 0x50, 0x02, 0x9A,
    ]);

    /// EFI_PCI_ROOT_BRIDGE_IO_PROTOCOL_GUID
    pub const PCI_ROOT_BRIDGE_IO_PROTOCOL: Guid = Guid::new(0x2F707EBB, 0x4A1A, 0x11D4, [
        0x9A, 0x38, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
    ]);

    /// EFI_SERIAL_IO_PROTOCOL_GUID
    pub const SERIAL_IO_PROTOCOL: Guid = Guid::new(0xBB25CF6F, 0xF1D4, 0x11D2, [
        0x9A, 0x0C, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0xFD,
    ]);

    /// EFI_USB_IO_PROTOCOL_GUID
    pub const USB_IO_PROTOCOL: Guid = Guid::new(0x2B2F68D6, 0x0CD2, 0x44CF, [
        0x8E, 0x8B, 0xBB, 0xA2, 0x0B, 0x1B, 0x5B, 0x75,
    ]);

    /// EFI_SIMPLE_NETWORK_PROTOCOL_GUID
    pub const SIMPLE_NETWORK_PROTOCOL: Guid = Guid::new(0xA19832B9, 0xAC25, 0x11D3, [
        0x9A, 0x2D, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
    ]);

    /// EFI_MANAGED_NETWORK_SERVICE_BINDING_PROTOCOL_GUID
    pub const MANAGED_NETWORK_SERVICE_BINDING_PROTOCOL: Guid =
        Guid::new(0xF36FF770, 0xA7E1, 0x42CF, [
            0x9E, 0xD2, 0x56, 0xF0, 0xF2, 0x71, 0xF4, 0x4C,
        ]);

    /// EFI_MANAGED_NETWORK_PROTOCOL_GUID
    pub const MANAGED_NETWORK_PROTOCOL: Guid = Guid::new(0x7AB33A91, 0xACE5, 0x4326, [
        0xB5, 0x72, 0xE7, 0xEE, 0x33, 0xD3, 0x9F, 0x16,
    ]);

    /// EFI_ARP_SERVICE_BINDING_PROTOCOL_GUID
    pub const ARP_SERVICE_BINDING_PROTOCOL: Guid = Guid::new(0xF44C00EE, 0x1F2C, 0x4A00, [
        0xAA, 0x09, 0x1C, 0x9C, 0x4C, 0x85, 0x85, 0x45,
    ]);

    /// EFI_ARP_PROTOCOL_GUID
    pub const ARP_PROTOCOL: Guid = Guid::new(0xF4B427BB, 0xBA21, 0x4F16, [
        0xBC, 0x4E, 0x43, 0xE4, 0x16, 0xAB, 0x61, 0x9C,
    ]);

    /// EFI_IP4_SERVICE_BINDING_PROTOCOL_GUID
    pub const IP4_SERVICE_BINDING_PROTOCOL: Guid = Guid::new(0xC51711E7, 0xB4BF, 0x404A, [
        0xBF, 0xB8, 0x0A, 0x04, 0x8E, 0xF1, 0xFF, 0xE4,
    ]);

    /// EFI_IP4_PROTOCOL_GUID
    pub const IP4_PROTOCOL: Guid = Guid::new(0x41D94CD2, 0x35B6, 0x455A, [
        0x82, 0x58, 0xD4, 0xE5, 0x13, 0x34, 0xAA, 0xDD,
    ]);

    /// EFI_IP4_CONFIG_PROTOCOL_GUID
    pub const IP4_CONFIG_PROTOCOL: Guid = Guid::new(0x3B95AA31, 0x3793, 0x434B, [
        0x86, 0x67, 0xC8, 0x07, 0x08, 0x92, 0xE0, 0x5E,
    ]);

    /// EFI_IP4_CONFIG2_PROTOCOL_GUID
    pub const IP4_CONFIG2_PROTOCOL: Guid = Guid::new(0x5B446ED1, 0xE30B, 0x4FAA, [
        0x87, 0x1A, 0x36, 0x54, 0xEC, 0xA3, 0x60, 0x80,
    ]);

    /// EFI_UDP4_SERVICE_BINDING_PROTOCOL_GUID
    pub const UDP4_SERVICE_BINDING_PROTOCOL: Guid = Guid::new(0x83F01464, 0x99BD, 0x45E5, [
        0xB3, 0x83, 0xAF, 0x63, 0x05, 0xD8, 0xE9, 0xE6,
    ]);

    /// EFI_UDP4_PROTOCOL_GUID
    pub const UDP4_PROTOCOL: Guid = Guid::new(0x3AD9DF29, 0x4501, 0x478D, [
        0xB1, 0xF8, 0x7F, 0x7F, 0xE7, 0x0E, 0x50, 0xF3,
    ]);

    /// EFI_TCP4_SERVICE_BINDING_PROTOCOL_GUID
    pub const TCP4_SERVICE_BINDING_PROTOCOL: Guid = Guid::new(0x00720665, 0x67EB, 0x4A99, [
        0xBA, 0xF7, 0xD3, 0xC3, 0x3A, 0x1C, 0x7C, 0xC9,
    ]);

    /// EFI_TCP4_PROTOCOL_GUID
    pub const TCP4_PROTOCOL: Guid = Guid::new(0x65530BC7, 0xA359, 0x410F, [
        0xB0, 0x10, 0x5A, 0xAD, 0xC7, 0xEC, 0x2B, 0x62,
    ]);

    /// EFI_ACPI_TABLE_GUID (ACPI 1.0)
    pub const ACPI_TABLE: Guid = Guid::new(0xEB9D2D30, 0x2D88, 0x11D3, [
        0x9A, 0x16, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
    ]);

    /// EFI_ACPI_20_TABLE_GUID (ACPI 2.0+)
    pub const ACPI_20_TABLE: Guid = Guid::new(0x8868E871, 0xE4F1, 0x11D3, [
        0xBC, 0x22, 0x00, 0x80, 0xC7, 0x3C, 0x88, 0x81,
    ]);

    /// SMBIOS_TABLE_GUID
    pub const SMBIOS_TABLE: Guid = Guid::new(0xEB9D2D31, 0x2D88, 0x11D3, [
        0x9A, 0x16, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
    ]);

    /// SMBIOS3_TABLE_GUID
    pub const SMBIOS3_TABLE: Guid = Guid::new(0xF2FD1544, 0x9794, 0x4A2C, [
        0x99, 0x2E, 0xE5, 0xBB, 0xCF, 0x20, 0xE3, 0x94,
    ]);

    /// EFI_MPS_TABLE_GUID
    pub const MPS_TABLE: Guid = Guid::new(0xEB9D2D2F, 0x2D88, 0x11D3, [
        0x9A, 0x16, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
    ]);

    /// EFI_DEVICE_TREE_GUID
    pub const DEVICE_TREE: Guid = Guid::new(0xB1B621D5, 0xF19C, 0x41A5, [
        0x83, 0x0B, 0xD9, 0x15, 0x2C, 0x69, 0xAA, 0xE0,
    ]);

    /// EFI_GLOBAL_VARIABLE
    pub const GLOBAL_VARIABLE: Guid = Guid::new(0x8BE4DF61, 0x93CA, 0x11D2, [
        0xAA, 0x0D, 0x00, 0xE0, 0x98, 0x03, 0x2B, 0x8C,
    ]);

    /// EFI_IMAGE_SECURITY_DATABASE_GUID
    pub const IMAGE_SECURITY_DATABASE: Guid = Guid::new(0xD719B2CB, 0x3D3A, 0x4596, [
        0xA3, 0xBC, 0xDA, 0xD0, 0x0E, 0x67, 0x65, 0x6F,
    ]);

    /// EFI_CERT_SHA256_GUID
    pub const CERT_SHA256: Guid = Guid::new(0xC1C41626, 0x504C, 0x4092, [
        0xAC, 0xA9, 0x41, 0xF9, 0x36, 0x93, 0x43, 0x28,
    ]);

    /// EFI_CERT_RSA2048_GUID
    pub const CERT_RSA2048: Guid = Guid::new(0x3C5766E8, 0x269C, 0x4E34, [
        0xAA, 0x14, 0xED, 0x77, 0x6E, 0x85, 0xB3, 0xB6,
    ]);

    /// EFI_CERT_RSA2048_SHA256_GUID
    pub const CERT_RSA2048_SHA256: Guid = Guid::new(0xE2B36190, 0x879B, 0x4A3D, [
        0xAD, 0x8D, 0xF2, 0xE7, 0xBB, 0xA3, 0x27, 0x84,
    ]);

    /// EFI_CERT_X509_GUID
    pub const CERT_X509: Guid = Guid::new(0xA5C059A1, 0x94E4, 0x4AA7, [
        0x87, 0xB5, 0xAB, 0x15, 0x5C, 0x2B, 0xF0, 0x72,
    ]);

    /// EFI_CERT_X509_SHA256_GUID
    pub const CERT_X509_SHA256: Guid = Guid::new(0x3BD2A492, 0x96C0, 0x4079, [
        0xB4, 0x20, 0xFC, 0xF9, 0x8E, 0xF1, 0x03, 0xED,
    ]);

    /// EFI_CERT_TYPE_PKCS7_GUID
    pub const CERT_TYPE_PKCS7: Guid = Guid::new(0x4AAFD29D, 0x68DF, 0x49EE, [
        0x8A, 0xA9, 0x34, 0x7D, 0x37, 0x56, 0x65, 0xA7,
    ]);

    /// EFI_RNG_PROTOCOL_GUID
    pub const RNG_PROTOCOL: Guid = Guid::new(0x3152BCA5, 0xEADE, 0x433D, [
        0x86, 0x2E, 0xC0, 0x1C, 0xDC, 0x29, 0x1F, 0x44,
    ]);

    /// EFI_RNG_ALGORITHM_SP800_90_CTR_256_GUID
    pub const RNG_ALGORITHM_SP800_90_CTR_256: Guid = Guid::new(0x44F0DE6E, 0x4D8C, 0x4045, [
        0xA8, 0xC7, 0x4D, 0xD1, 0x68, 0x85, 0x6B, 0x9E,
    ]);

    /// EFI_RNG_ALGORITHM_RAW
    pub const RNG_ALGORITHM_RAW: Guid = Guid::new(0xE43176D7, 0xB6E8, 0x4827, [
        0xB7, 0x84, 0x7F, 0xFD, 0xC4, 0xB6, 0x85, 0x61,
    ]);

    /// EFI_TCG2_PROTOCOL_GUID
    pub const TCG2_PROTOCOL: Guid = Guid::new(0x607F766C, 0x7455, 0x42BE, [
        0x93, 0x0B, 0xE4, 0xD7, 0x6D, 0xB2, 0x72, 0x0F,
    ]);

    /// EFI_MEMORY_ATTRIBUTE_TABLE_GUID
    pub const MEMORY_ATTRIBUTE_TABLE: Guid = Guid::new(0xDCFA911D, 0x26EB, 0x469F, [
        0xA2, 0x20, 0x38, 0xB7, 0xDC, 0x46, 0x12, 0x20,
    ]);

    /// EFI_CONSOLE_OUT_DEVICE_GUID
    pub const CONSOLE_OUT_DEVICE: Guid = Guid::new(0xD3B36F2C, 0xD551, 0x11D4, [
        0x9A, 0x46, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
    ]);

    /// EFI_CONSOLE_IN_DEVICE_GUID
    pub const CONSOLE_IN_DEVICE: Guid = Guid::new(0xD3B36F2B, 0xD551, 0x11D4, [
        0x9A, 0x46, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
    ]);

    /// EFI_STANDARD_ERROR_DEVICE_GUID
    pub const STANDARD_ERROR_DEVICE: Guid = Guid::new(0xD3B36F2D, 0xD551, 0x11D4, [
        0x9A, 0x46, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
    ]);

    /// EFI_HOB_LIST_GUID (Hand-Off Block List)
    pub const HOB_LIST: Guid = Guid::new(0x7739F24C, 0x93D7, 0x11D4, [
        0x9A, 0x3A, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
    ]);

    /// EFI_MEMORY_TYPE_INFORMATION_GUID
    pub const MEMORY_TYPE_INFORMATION: Guid = Guid::new(0x4C19049F, 0x4137, 0x4DD3, [
        0x9C, 0x10, 0x8B, 0x97, 0xA8, 0x3F, 0xFD, 0xFA,
    ]);

    /// EFI_DEBUG_IMAGE_INFO_TABLE_GUID
    pub const DEBUG_IMAGE_INFO_TABLE: Guid = Guid::new(0x49152E77, 0x1ADA, 0x4764, [
        0xB7, 0xA2, 0x7A, 0xFE, 0xFE, 0xD9, 0x5E, 0x8B,
    ]);
}

// =============================================================================
// EVENT
// =============================================================================

/// UEFI Event type
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Event(pub *mut core::ffi::c_void);

impl Event {
    /// Null event
    pub const NULL: Self = Self(core::ptr::null_mut());

    /// Create a null event (function form)
    pub const fn null() -> Self {
        Self::NULL
    }

    /// Create a new event from a raw pointer
    pub const fn new(ptr: *mut core::ffi::c_void) -> Self {
        Self(ptr)
    }

    /// Check if this is a null event
    pub const fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl fmt::Debug for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Event({:p})", self.0)
    }
}

impl Default for Event {
    fn default() -> Self {
        Self::NULL
    }
}

unsafe impl Send for Event {}
unsafe impl Sync for Event {}

// =============================================================================
// EVENT TYPES
// =============================================================================

/// Event type flags
pub mod event_type {
    /// Timer event
    pub const TIMER: u32 = 0x80000000;
    /// Runtime event
    pub const RUNTIME: u32 = 0x40000000;
    /// Notify wait
    pub const NOTIFY_WAIT: u32 = 0x00000100;
    /// Notify signal
    pub const NOTIFY_SIGNAL: u32 = 0x00000200;
    /// Signal exit boot services
    pub const SIGNAL_EXIT_BOOT_SERVICES: u32 = 0x00000201;
    /// Signal virtual address change
    pub const SIGNAL_VIRTUAL_ADDRESS_CHANGE: u32 = 0x60000202;
}

/// Timer delay type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TimerDelay {
    /// Cancel timer
    Cancel   = 0,
    /// Periodic timer
    Periodic = 1,
    /// Relative timer
    Relative = 2,
}

// =============================================================================
// TIME
// =============================================================================

/// UEFI Time structure
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct Time {
    /// Year (1900 - 9999)
    pub year: u16,
    /// Month (1 - 12)
    pub month: u8,
    /// Day (1 - 31)
    pub day: u8,
    /// Hour (0 - 23)
    pub hour: u8,
    /// Minute (0 - 59)
    pub minute: u8,
    /// Second (0 - 59)
    pub second: u8,
    /// Padding
    pub pad1: u8,
    /// Nanosecond (0 - 999,999,999)
    pub nanosecond: u32,
    /// Timezone (-1440 to 1440 or 2047 for unspecified)
    pub timezone: i16,
    /// Daylight saving time flags
    pub daylight: u8,
    /// Padding
    pub pad2: u8,
}

impl Time {
    /// Unspecified timezone value
    pub const UNSPECIFIED_TIMEZONE: i16 = 0x07FF;

    /// Daylight: Adjust for daylight savings time
    pub const ADJUST_DAYLIGHT: u8 = 0x01;
    /// Daylight: In daylight savings time
    pub const IN_DAYLIGHT: u8 = 0x02;

    /// Create a new time structure
    pub const fn new(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Self {
        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            pad1: 0,
            nanosecond: 0,
            timezone: Self::UNSPECIFIED_TIMEZONE,
            daylight: 0,
            pad2: 0,
        }
    }

    /// Check if this is a valid time
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

/// UEFI Time Capabilities
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct TimeCapabilities {
    /// Resolution in counts per second
    pub resolution: u32,
    /// Accuracy in parts per million
    pub accuracy: u32,
    /// True if a time set operation clears the device's time below resolution
    pub sets_to_zero: Boolean,
}

// =============================================================================
// TABLE HEADER
// =============================================================================

/// UEFI Table Header
///
/// Common header for all UEFI tables
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TableHeader {
    /// Signature identifying the table
    pub signature: u64,
    /// Revision of the table
    pub revision: u32,
    /// Size of the entire table including header
    pub header_size: u32,
    /// CRC32 of the entire table
    pub crc32: u32,
    /// Reserved (must be zero)
    pub reserved: u32,
}

impl TableHeader {
    /// EFI_SYSTEM_TABLE_SIGNATURE
    pub const SYSTEM_TABLE_SIGNATURE: u64 = 0x5453595320494249; // "IBI SYST"

    /// EFI_BOOT_SERVICES_SIGNATURE
    pub const BOOT_SERVICES_SIGNATURE: u64 = 0x56524553544F4F42; // "BOOTSERV"

    /// EFI_RUNTIME_SERVICES_SIGNATURE
    pub const RUNTIME_SERVICES_SIGNATURE: u64 = 0x56524553544E5552; // "RUNTSERV"

    /// Create a new table header
    pub const fn new(signature: u64, revision: u32, header_size: u32) -> Self {
        Self {
            signature,
            revision,
            header_size,
            crc32: 0,
            reserved: 0,
        }
    }

    /// Validate the table header
    pub fn validate(&self, expected_signature: u64) -> bool {
        self.signature == expected_signature && self.reserved == 0
    }

    /// Get the UEFI specification version
    pub fn version(&self) -> (u16, u16) {
        let major = (self.revision >> 16) as u16;
        let minor = (self.revision & 0xFFFF) as u16;
        (major, minor)
    }
}

// =============================================================================
// CONFIGURATION TABLE
// =============================================================================

/// UEFI Configuration Table Entry
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ConfigurationTable {
    /// GUID identifying the table type
    pub vendor_guid: Guid,
    /// Pointer to the table data
    pub vendor_table: *mut core::ffi::c_void,
}

impl ConfigurationTable {
    /// Check if this is an ACPI table (1.0)
    pub fn is_acpi(&self) -> bool {
        self.vendor_guid == guids::ACPI_TABLE
    }

    /// Check if this is an ACPI 2.0+ table
    pub fn is_acpi2(&self) -> bool {
        self.vendor_guid == guids::ACPI_20_TABLE
    }

    /// Check if this is an SMBIOS table
    pub fn is_smbios(&self) -> bool {
        self.vendor_guid == guids::SMBIOS_TABLE
    }

    /// Check if this is an SMBIOS 3.0 table
    pub fn is_smbios3(&self) -> bool {
        self.vendor_guid == guids::SMBIOS3_TABLE
    }

    /// Check if this is a device tree
    pub fn is_device_tree(&self) -> bool {
        self.vendor_guid == guids::DEVICE_TREE
    }
}

// =============================================================================
// RESET TYPE
// =============================================================================

/// System reset type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ResetType {
    /// Cold reset
    Cold             = 0,
    /// Warm reset
    Warm             = 1,
    /// Shutdown
    Shutdown         = 2,
    /// Platform-specific reset
    PlatformSpecific = 3,
}

// =============================================================================
// ALLOCATE TYPE
// =============================================================================

/// Memory allocation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AllocateType {
    /// Allocate any available range
    AllocateAnyPages   = 0,
    /// Allocate at or below a maximum address
    AllocateMaxAddress = 1,
    /// Allocate at a specific address
    AllocateAddress    = 2,
}

// =============================================================================
// INTERFACE TYPE
// =============================================================================

/// Protocol interface type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum InterfaceType {
    /// Native interface
    Native = 0,
}

// =============================================================================
// LOCATE SEARCH TYPE
// =============================================================================

/// Protocol location search type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum LocateSearchType {
    /// All handles
    AllHandles       = 0,
    /// By registration key
    ByRegisterNotify = 1,
    /// By protocol
    ByProtocol       = 2,
}

// =============================================================================
// OPEN PROTOCOL ATTRIBUTES
// =============================================================================

/// Attributes for opening a protocol
pub mod open_protocol {
    /// Open by handle protocol
    pub const BY_HANDLE_PROTOCOL: u32 = 0x00000001;
    /// Get protocol
    pub const GET_PROTOCOL: u32 = 0x00000002;
    /// Test protocol
    pub const TEST_PROTOCOL: u32 = 0x00000004;
    /// By child controller
    pub const BY_CHILD_CONTROLLER: u32 = 0x00000008;
    /// By driver
    pub const BY_DRIVER: u32 = 0x00000010;
    /// Exclusive access
    pub const EXCLUSIVE: u32 = 0x00000020;
}

// =============================================================================
// TPL (Task Priority Level)
// =============================================================================

/// Task Priority Level
pub type Tpl = usize;

/// TPL Application
pub const TPL_APPLICATION: Tpl = 4;
/// TPL Callback
pub const TPL_CALLBACK: Tpl = 8;
/// TPL Notify
pub const TPL_NOTIFY: Tpl = 16;
/// TPL High Level
pub const TPL_HIGH_LEVEL: Tpl = 31;

// =============================================================================
// INPUT KEY
// =============================================================================

/// Input key structure
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct InputKey {
    /// Scan code
    pub scan_code: u16,
    /// Unicode character
    pub unicode_char: Char16,
}

impl InputKey {
    /// Null scan code (no special key pressed)
    pub const SCAN_NULL: u16 = 0x00;
    /// Up arrow
    pub const SCAN_UP: u16 = 0x01;
    /// Down arrow
    pub const SCAN_DOWN: u16 = 0x02;
    /// Right arrow
    pub const SCAN_RIGHT: u16 = 0x03;
    /// Left arrow
    pub const SCAN_LEFT: u16 = 0x04;
    /// Home
    pub const SCAN_HOME: u16 = 0x05;
    /// End
    pub const SCAN_END: u16 = 0x06;
    /// Insert
    pub const SCAN_INSERT: u16 = 0x07;
    /// Delete
    pub const SCAN_DELETE: u16 = 0x08;
    /// Page Up
    pub const SCAN_PAGE_UP: u16 = 0x09;
    /// Page Down
    pub const SCAN_PAGE_DOWN: u16 = 0x0A;
    /// F1
    pub const SCAN_F1: u16 = 0x0B;
    /// F2
    pub const SCAN_F2: u16 = 0x0C;
    /// F3
    pub const SCAN_F3: u16 = 0x0D;
    /// F4
    pub const SCAN_F4: u16 = 0x0E;
    /// F5
    pub const SCAN_F5: u16 = 0x0F;
    /// F6
    pub const SCAN_F6: u16 = 0x10;
    /// F7
    pub const SCAN_F7: u16 = 0x11;
    /// F8
    pub const SCAN_F8: u16 = 0x12;
    /// F9
    pub const SCAN_F9: u16 = 0x13;
    /// F10
    pub const SCAN_F10: u16 = 0x14;
    /// Escape
    pub const SCAN_ESC: u16 = 0x17;
}

// =============================================================================
// CAPSULE
// =============================================================================

/// Capsule header for firmware updates
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CapsuleHeader {
    /// Capsule GUID
    pub capsule_guid: Guid,
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
// PROTOCOL INFORMATION
// =============================================================================

/// Protocol information entry
#[derive(Debug)]
#[repr(C)]
pub struct OpenProtocolInformationEntry {
    /// Agent handle
    pub agent_handle: Handle,
    /// Controller handle
    pub controller_handle: Handle,
    /// Attributes
    pub attributes: u32,
    /// Open count
    pub open_count: u32,
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guid_display() {
        let guid = guids::ACPI_20_TABLE;
        let s = format!("{}", guid);
        assert!(s.contains('-'));
    }

    #[test]
    fn test_status_is_error() {
        assert!(!Status::SUCCESS.is_error());
        assert!(Status::NOT_FOUND.is_error());
        assert!(Status::OUT_OF_RESOURCES.is_error());
    }

    #[test]
    fn test_physical_address_alignment() {
        let addr = PhysicalAddress::new(0x1234);
        let aligned = addr.align_up(0x1000);
        assert_eq!(aligned.as_u64(), 0x2000);
    }

    #[test]
    fn test_time_validity() {
        let time = Time::new(2024, 1, 15, 12, 30, 45);
        assert!(time.is_valid());

        let invalid = Time::new(2024, 13, 1, 0, 0, 0);
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_table_header_version() {
        let header = TableHeader::new(
            TableHeader::SYSTEM_TABLE_SIGNATURE,
            0x00020046, // UEFI 2.70
            96,
        );
        assert_eq!(header.version(), (2, 70));
    }
}
