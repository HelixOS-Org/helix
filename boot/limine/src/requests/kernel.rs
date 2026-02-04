//! # Kernel Requests
//!
//! This module provides kernel-related Limine requests:
//! - Kernel file information
//! - Kernel physical/virtual addresses
//! - Boot modules

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::missing_fields_in_debug)]
#![allow(clippy::needless_lifetimes)]

use core::ffi::CStr;
use core::{ptr, slice};

use super::{LimineRequest, ResponsePtr, SafeResponse};
use crate::protocol::raw::{RawFile, RawUuid};
use crate::protocol::request_ids::{KERNEL_ADDRESS_ID, KERNEL_FILE_ID, MODULE_ID};

// =============================================================================
// Kernel File Request
// =============================================================================

/// Kernel file request
///
/// Provides access to the kernel file that was loaded by the bootloader.
///
/// # Example
///
/// ```rust,no_run
/// use helix_limine::requests::KernelFileRequest;
///
/// #[used]
/// #[link_section = ".limine_requests"]
/// static KERNEL_FILE: KernelFileRequest = KernelFileRequest::new();
///
/// fn get_kernel_size() -> usize {
///     KERNEL_FILE
///         .response()
///         .and_then(|r| r.file())
///         .map(|f| f.size())
///         .unwrap_or(0)
/// }
/// ```
#[repr(C)]
pub struct KernelFileRequest {
    /// Request identifier
    id: [u64; 4],
    /// Protocol revision
    revision: u64,
    /// Response pointer
    response: ResponsePtr<KernelFileResponse>,
}

impl KernelFileRequest {
    /// Create a new kernel file request
    pub const fn new() -> Self {
        Self {
            id: KERNEL_FILE_ID,
            revision: 0,
            response: ResponsePtr::null(),
        }
    }
}

impl Default for KernelFileRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl LimineRequest for KernelFileRequest {
    type Response = KernelFileResponse;

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

unsafe impl Sync for KernelFileRequest {}

/// Kernel file response
#[repr(C)]
pub struct KernelFileResponse {
    /// Response revision
    revision: u64,
    /// Kernel file pointer
    kernel_file: *const RawFile,
}

impl KernelFileResponse {
    /// Get the kernel file
    pub fn file(&self) -> Option<File<'_>> {
        if self.kernel_file.is_null() {
            None
        } else {
            // Safety: Bootloader guarantees valid file pointer
            unsafe { Some(File::from_raw(&*self.kernel_file)) }
        }
    }

    /// Get the response revision
    pub fn revision(&self) -> u64 {
        self.revision
    }
}

unsafe impl SafeResponse for KernelFileResponse {
    fn validate(&self) -> bool {
        !self.kernel_file.is_null()
    }
}

impl core::fmt::Debug for KernelFileResponse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("KernelFileResponse")
            .field("file", &self.file())
            .field("revision", &self.revision)
            .finish()
    }
}

// =============================================================================
// Kernel Address Request
// =============================================================================

/// Kernel address request
///
/// Provides the physical and virtual base addresses of the kernel.
#[repr(C)]
pub struct KernelAddressRequest {
    /// Request identifier
    id: [u64; 4],
    /// Protocol revision
    revision: u64,
    /// Response pointer
    response: ResponsePtr<KernelAddressResponse>,
}

impl KernelAddressRequest {
    /// Create a new kernel address request
    pub const fn new() -> Self {
        Self {
            id: KERNEL_ADDRESS_ID,
            revision: 0,
            response: ResponsePtr::null(),
        }
    }
}

impl Default for KernelAddressRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl LimineRequest for KernelAddressRequest {
    type Response = KernelAddressResponse;

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

unsafe impl Sync for KernelAddressRequest {}

/// Kernel address response
#[repr(C)]
pub struct KernelAddressResponse {
    /// Response revision
    revision: u64,
    /// Physical base address
    physical_base: u64,
    /// Virtual base address
    virtual_base: u64,
}

impl KernelAddressResponse {
    /// Get the kernel's physical base address
    pub fn physical_base(&self) -> u64 {
        self.physical_base
    }

    /// Get the kernel's virtual base address
    pub fn virtual_base(&self) -> u64 {
        self.virtual_base
    }

    /// Get the response revision
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Calculate the offset between physical and virtual addresses
    pub fn offset(&self) -> i64 {
        self.virtual_base.wrapping_sub(self.physical_base) as i64
    }

    /// Convert a kernel physical address to virtual
    pub fn phys_to_virt(&self, phys: u64) -> u64 {
        phys.wrapping_add(self.offset() as u64)
    }

    /// Convert a kernel virtual address to physical
    pub fn virt_to_phys(&self, virt: u64) -> u64 {
        virt.wrapping_sub(self.offset() as u64)
    }
}

unsafe impl SafeResponse for KernelAddressResponse {
    fn validate(&self) -> bool {
        self.physical_base != 0 && self.virtual_base != 0
    }
}

impl core::fmt::Debug for KernelAddressResponse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("KernelAddressResponse")
            .field(
                "physical_base",
                &format_args!("{:#018x}", self.physical_base),
            )
            .field("virtual_base", &format_args!("{:#018x}", self.virtual_base))
            .field("offset", &format_args!("{:#x}", self.offset()))
            .finish()
    }
}

// =============================================================================
// Module Request
// =============================================================================

/// Module request
///
/// Provides access to boot modules loaded by the bootloader.
///
/// # Module Filtering
///
/// Modules can be filtered by path using the internal modules feature.
///
/// # Example
///
/// ```rust,no_run
/// use helix_limine::requests::ModuleRequest;
///
/// #[used]
/// #[link_section = ".limine_requests"]
/// static MODULES: ModuleRequest = ModuleRequest::new();
///
/// fn list_modules() {
///     if let Some(modules) = MODULES.response() {
///         for module in modules.modules() {
///             println!("Module: {} ({} bytes)", module.path(), module.size());
///         }
///     }
/// }
/// ```
#[repr(C)]
pub struct ModuleRequest {
    /// Request identifier
    id: [u64; 4],
    /// Protocol revision
    revision: u64,
    /// Response pointer
    response: ResponsePtr<ModuleResponse>,
    /// Internal module count (revision 1+)
    internal_module_count: u64,
    /// Internal modules pointer (revision 1+)
    internal_modules: *const *const InternalModule,
}

impl ModuleRequest {
    /// Create a new module request
    pub const fn new() -> Self {
        Self {
            id: MODULE_ID,
            revision: 0,
            response: ResponsePtr::null(),
            internal_module_count: 0,
            internal_modules: ptr::null(),
        }
    }

    /// Create a request with revision 1 (internal modules support)
    pub const fn with_revision(revision: u64) -> Self {
        Self {
            id: MODULE_ID,
            revision,
            response: ResponsePtr::null(),
            internal_module_count: 0,
            internal_modules: ptr::null(),
        }
    }
}

impl Default for ModuleRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl LimineRequest for ModuleRequest {
    type Response = ModuleResponse;

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

unsafe impl Sync for ModuleRequest {}

/// Internal module definition (for revision 1+)
#[repr(C)]
pub struct InternalModule {
    /// Module path
    pub path: *const i8,
    /// Command line
    pub cmdline: *const i8,
    /// Flags
    pub flags: u64,
}

/// Module flags
pub mod module_flags {
    /// Module is required (boot fails if not found)
    pub const REQUIRED: u64 = 1 << 0;
    /// Module is compressed (internal use)
    pub const COMPRESSED: u64 = 1 << 1;
}

/// Module response
#[repr(C)]
pub struct ModuleResponse {
    /// Response revision
    revision: u64,
    /// Module count
    module_count: u64,
    /// Modules pointer
    modules: *const *const RawFile,
}

impl ModuleResponse {
    /// Get the number of modules
    pub fn module_count(&self) -> usize {
        self.module_count as usize
    }

    /// Get the response revision
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Iterate over all modules
    pub fn modules(&self) -> ModuleIterator<'_> {
        ModuleIterator {
            response: self,
            index: 0,
        }
    }

    /// Get a specific module by index
    pub fn get(&self, index: usize) -> Option<File<'_>> {
        if index >= self.module_count() || self.modules.is_null() {
            return None;
        }

        unsafe {
            let file_ptr = *self.modules.add(index);
            if file_ptr.is_null() {
                None
            } else {
                Some(File::from_raw(&*file_ptr))
            }
        }
    }

    /// Find a module by path
    pub fn find_by_path(&self, path: &str) -> Option<File<'_>> {
        self.modules().find(|m| m.path() == path)
    }

    /// Find a module by path suffix (e.g., filename)
    pub fn find_by_name(&self, name: &str) -> Option<File<'_>> {
        self.modules().find(|m| m.path().ends_with(name))
    }
}

unsafe impl SafeResponse for ModuleResponse {
    fn validate(&self) -> bool {
        self.module_count == 0 || !self.modules.is_null()
    }
}

impl core::fmt::Debug for ModuleResponse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ModuleResponse")
            .field("module_count", &self.module_count())
            .field("revision", &self.revision)
            .finish()
    }
}

/// Iterator over modules
pub struct ModuleIterator<'a> {
    response: &'a ModuleResponse,
    index: usize,
}

impl<'a> Iterator for ModuleIterator<'a> {
    type Item = File<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let file = self.response.get(self.index)?;
        self.index += 1;
        Some(file)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.response.module_count() - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for ModuleIterator<'a> {}

// =============================================================================
// File Abstraction
// =============================================================================

/// A loaded file (kernel or module)
///
/// This is a safe wrapper around the raw file structure.
#[derive(Clone, Copy)]
pub struct File<'a> {
    raw: &'a RawFile,
}

impl<'a> File<'a> {
    /// Create from raw file pointer
    fn from_raw(raw: &'a RawFile) -> Self {
        Self { raw }
    }

    /// Get the file's address in memory
    pub fn address(&self) -> *const u8 {
        self.raw.address
    }

    /// Get the file size in bytes
    pub fn size(&self) -> usize {
        self.raw.size as usize
    }

    /// Get the file path
    pub fn path(&self) -> &str {
        if self.raw.path.is_null() {
            return "";
        }
        unsafe { CStr::from_ptr(self.raw.path).to_str().unwrap_or("") }
    }

    /// Get the command line / string
    pub fn cmdline(&self) -> &str {
        if self.raw.cmdline.is_null() {
            return "";
        }
        unsafe { CStr::from_ptr(self.raw.cmdline).to_str().unwrap_or("") }
    }

    /// Get the command line as a 'static reference
    ///
    /// # Safety
    /// The pointer comes from the bootloader and remains valid for the kernel lifetime.
    pub fn cmdline_static(&self) -> &'static str {
        if self.raw.cmdline.is_null() {
            return "";
        }
        unsafe { CStr::from_ptr(self.raw.cmdline).to_str().unwrap_or("") }
    }

    /// Get the file contents as a byte slice
    ///
    /// # Safety
    ///
    /// This is safe because the bootloader guarantees the memory
    /// is valid and accessible.
    pub fn data(&self) -> &[u8] {
        if self.raw.address.is_null() || self.raw.size == 0 {
            return &[];
        }
        unsafe { slice::from_raw_parts(self.raw.address, self.raw.size as usize) }
    }

    /// Get the media type
    pub fn media_type(&self) -> MediaType {
        MediaType::from_raw(self.raw.media_type)
    }

    /// Get the partition index
    pub fn partition_index(&self) -> u32 {
        self.raw.partition_index
    }

    /// Get the MBR disk ID
    pub fn mbr_disk_id(&self) -> u32 {
        self.raw.mbr_disk_id
    }

    /// Get the GPT disk UUID
    pub fn gpt_disk_uuid(&self) -> Uuid {
        Uuid(self.raw.gpt_disk_uuid)
    }

    /// Get the GPT partition UUID
    pub fn gpt_partition_uuid(&self) -> Uuid {
        Uuid(self.raw.gpt_part_uuid)
    }

    /// Get the partition UUID
    pub fn partition_uuid(&self) -> Uuid {
        Uuid(self.raw.part_uuid)
    }

    /// Get TFTP information (if media type is TFTP)
    pub fn tftp_info(&self) -> Option<TftpInfo> {
        if self.media_type() == MediaType::Tftp {
            Some(TftpInfo {
                ip: self.raw.tftp_ip,
                port: self.raw.tftp_port,
            })
        } else {
            None
        }
    }

    /// Get the file revision
    pub fn revision(&self) -> u64 {
        self.raw.revision
    }
}

impl<'a> core::fmt::Debug for File<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("File")
            .field("path", &self.path())
            .field("size", &self.size())
            .field("cmdline", &self.cmdline())
            .field("media_type", &self.media_type())
            .finish()
    }
}

/// Media type for loaded files
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    /// Generic/unknown media
    Generic,
    /// Optical drive (CD/DVD)
    Optical,
    /// TFTP boot
    Tftp,
    /// Unknown media type
    Unknown(u32),
}

impl MediaType {
    /// Create from raw value
    pub fn from_raw(raw: u32) -> Self {
        match raw {
            0 => Self::Generic,
            1 => Self::Optical,
            2 => Self::Tftp,
            other => Self::Unknown(other),
        }
    }

    /// Convert to raw value
    pub fn to_raw(self) -> u32 {
        match self {
            Self::Generic => 0,
            Self::Optical => 1,
            Self::Tftp => 2,
            Self::Unknown(v) => v,
        }
    }
}

/// UUID wrapper
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Uuid(RawUuid);

impl Uuid {
    /// Create a null UUID
    pub const fn null() -> Self {
        Self(RawUuid::null())
    }

    /// Check if UUID is null
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }

    /// Get the raw bytes
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0.bytes
    }
}

impl core::fmt::Debug for Uuid {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.is_null() {
            write!(f, "Uuid(null)")
        } else {
            write!(f, "Uuid({:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x})",
                self.0.bytes[0], self.0.bytes[1], self.0.bytes[2], self.0.bytes[3],
                self.0.bytes[4], self.0.bytes[5],
                self.0.bytes[6], self.0.bytes[7],
                self.0.bytes[8], self.0.bytes[9],
                self.0.bytes[10], self.0.bytes[11], self.0.bytes[12],
                self.0.bytes[13], self.0.bytes[14], self.0.bytes[15]
            )
        }
    }
}

impl core::fmt::Display for Uuid {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self, f)
    }
}

/// TFTP boot information
#[derive(Debug, Clone, Copy)]
pub struct TftpInfo {
    /// TFTP server IP (big-endian)
    pub ip: u32,
    /// TFTP server port
    pub port: u32,
}

impl TftpInfo {
    /// Get the IP as a dotted string
    pub fn ip_string(&self) -> [u8; 15] {
        let bytes = self.ip.to_be_bytes();
        let mut result = [0u8; 15];
        // Format: xxx.xxx.xxx.xxx
        let mut pos = 0;
        for (i, &b) in bytes.iter().enumerate() {
            if i > 0 {
                result[pos] = b'.';
                pos += 1;
            }
            if b >= 100 {
                result[pos] = b'0' + b / 100;
                pos += 1;
            }
            if b >= 10 {
                result[pos] = b'0' + (b / 10) % 10;
                pos += 1;
            }
            result[pos] = b'0' + b % 10;
            pos += 1;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_type_conversion() {
        assert_eq!(MediaType::from_raw(0), MediaType::Generic);
        assert_eq!(MediaType::from_raw(1), MediaType::Optical);
        assert_eq!(MediaType::from_raw(2), MediaType::Tftp);
        assert_eq!(MediaType::from_raw(99), MediaType::Unknown(99));
    }

    #[test]
    fn test_uuid_null() {
        let uuid = Uuid::null();
        assert!(uuid.is_null());
    }
}
