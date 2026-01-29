//! # Bootloader Information Request
//!
//! This module provides the bootloader info request type, which allows
//! the kernel to retrieve information about the bootloader that loaded it.

use core::ffi::CStr;


use crate::protocol::request_ids::BOOTLOADER_INFO_ID;
use crate::protocol::raw::RawBootloaderInfoResponse;
use super::{LimineRequest, ResponsePtr, SafeResponse};

/// Bootloader information request
///
/// This request allows retrieving the name and version of the bootloader.
///
/// # Example
///
/// ```rust,no_run
/// use helix_limine::requests::BootloaderInfoRequest;
///
/// #[used]
/// #[link_section = ".limine_requests"]
/// static BOOTLOADER_INFO: BootloaderInfoRequest = BootloaderInfoRequest::new();
///
/// fn print_bootloader_info() {
///     if let Some(info) = BOOTLOADER_INFO.response() {
///         println!("Bootloader: {} v{}", info.name(), info.version());
///     }
/// }
/// ```
#[repr(C)]
pub struct BootloaderInfoRequest {
    /// Request identifier
    id: [u64; 4],
    /// Protocol revision
    revision: u64,
    /// Response pointer (filled by bootloader)
    response: ResponsePtr<BootloaderInfoResponse>,
}

impl BootloaderInfoRequest {
    /// Create a new bootloader info request
    pub const fn new() -> Self {
        Self {
            id: BOOTLOADER_INFO_ID,
            revision: 0,
            response: ResponsePtr::null(),
        }
    }

    /// Create a request with a specific revision
    pub const fn with_revision(revision: u64) -> Self {
        Self {
            id: BOOTLOADER_INFO_ID,
            revision,
            response: ResponsePtr::null(),
        }
    }
}

impl Default for BootloaderInfoRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl LimineRequest for BootloaderInfoRequest {
    type Response = BootloaderInfoResponse;

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
        // Safety: Response is initialized by bootloader before kernel starts
        unsafe { self.response.get() }
    }
}

// Safety: Request is read-only after initialization
unsafe impl Sync for BootloaderInfoRequest {}

/// Bootloader information response
///
/// Contains the name and version strings of the bootloader.
#[repr(C)]
pub struct BootloaderInfoResponse {
    /// Raw response from bootloader
    raw: RawBootloaderInfoResponse,
}

impl BootloaderInfoResponse {
    /// Get the bootloader name as a string slice
    ///
    /// Returns an empty string if the name is not available.
    pub fn name(&self) -> &str {
        if self.raw.name.is_null() {
            return "";
        }

        // Safety: Bootloader guarantees null-terminated ASCII string
        unsafe {
            CStr::from_ptr(self.raw.name)
                .to_str()
                .unwrap_or("")
        }
    }

    /// Get the bootloader version as a string slice
    ///
    /// Returns an empty string if the version is not available.
    pub fn version(&self) -> &str {
        if self.raw.version.is_null() {
            return "";
        }

        // Safety: Bootloader guarantees null-terminated ASCII string
        unsafe {
            CStr::from_ptr(self.raw.version)
                .to_str()
                .unwrap_or("")
        }
    }

    /// Get the response revision
    pub fn revision(&self) -> u64 {
        self.raw.revision
    }

    /// Get the raw name pointer (for FFI)
    pub fn name_ptr(&self) -> *const i8 {
        self.raw.name
    }

    /// Get the raw version pointer (for FFI)
    pub fn version_ptr(&self) -> *const i8 {
        self.raw.version
    }
}

unsafe impl SafeResponse for BootloaderInfoResponse {
    fn validate(&self) -> bool {
        // Name and version can be null, so validation always passes
        true
    }
}

impl core::fmt::Debug for BootloaderInfoResponse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BootloaderInfoResponse")
            .field("name", &self.name())
            .field("version", &self.version())
            .field("revision", &self.revision())
            .finish()
    }
}

impl core::fmt::Display for BootloaderInfoResponse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} v{}", self.name(), self.version())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_creation() {
        let request = BootloaderInfoRequest::new();
        assert_eq!(request.id(), BOOTLOADER_INFO_ID);
        assert_eq!(request.revision(), 0);
        assert!(!request.has_response());
    }

    #[test]
    fn test_request_with_revision() {
        let request = BootloaderInfoRequest::with_revision(1);
        assert_eq!(request.revision(), 1);
    }
}
