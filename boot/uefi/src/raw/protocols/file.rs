//! Simple File System Protocol
//!
//! Provides access to FAT-based file systems.

use core::fmt;

use crate::raw::types::{guids, Boolean, Char16, Event, Guid, Status, Time};

// =============================================================================
// COMPATIBILITY GUIDS
// =============================================================================

/// `EFI_FILE_INFO_GUID` (compatibility alias)
pub const EFI_FILE_INFO_GUID: Guid = guids::FILE_INFO;

/// `EFI_FILE_SYSTEM_INFO_GUID` (compatibility alias)
pub const EFI_FILE_SYSTEM_INFO_GUID: Guid = guids::FILE_SYSTEM_INFO;

/// `SIMPLE_FILE_SYSTEM_PROTOCOL_GUID` (compatibility alias)
pub const SIMPLE_FILE_SYSTEM_PROTOCOL_GUID: Guid = guids::SIMPLE_FILE_SYSTEM_PROTOCOL;

// =============================================================================
// SIMPLE FILE SYSTEM PROTOCOL
// =============================================================================

/// Simple File System Protocol
#[repr(C)]
#[derive(Debug)]
pub struct EfiSimpleFileSystemProtocol {
    /// Revision of the protocol
    pub revision: u64,

    /// Open the root directory
    pub open_volume:
        unsafe extern "efiapi" fn(this: *mut Self, root: *mut *mut EfiFileProtocol) -> Status,
}

impl EfiSimpleFileSystemProtocol {
    /// Protocol GUID
    pub const GUID: Guid = guids::SIMPLE_FILE_SYSTEM_PROTOCOL;

    /// Protocol revision 1.0
    pub const REVISION_1: u64 = 0x0001_0000;

    /// Open the root directory
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn open_volume(&mut self) -> Result<*mut EfiFileProtocol, Status> {
        let mut root = core::ptr::null_mut();
        let status = (self.open_volume)(self, &mut root);
        status.to_status_result_with(root)
    }
}

// =============================================================================
// FILE PROTOCOL
// =============================================================================

/// File Protocol
#[repr(C)]
#[derive(Debug)]
pub struct EfiFileProtocol {
    /// Revision of the protocol
    pub revision: u64,

    /// Open a file
    pub open: unsafe extern "efiapi" fn(
        this: *mut Self,
        new_handle: *mut *mut Self,
        file_name: *const Char16,
        open_mode: u64,
        attributes: u64,
    ) -> Status,

    /// Close a file
    pub close: unsafe extern "efiapi" fn(this: *mut Self) -> Status,

    /// Delete a file
    pub delete: unsafe extern "efiapi" fn(this: *mut Self) -> Status,

    /// Read from a file
    pub read: unsafe extern "efiapi" fn(
        this: *mut Self,
        buffer_size: *mut usize,
        buffer: *mut u8,
    ) -> Status,

    /// Write to a file
    pub write: unsafe extern "efiapi" fn(
        this: *mut Self,
        buffer_size: *mut usize,
        buffer: *const u8,
    ) -> Status,

    /// Get file position
    pub get_position: unsafe extern "efiapi" fn(this: *mut Self, position: *mut u64) -> Status,

    /// Set file position
    pub set_position: unsafe extern "efiapi" fn(this: *mut Self, position: u64) -> Status,

    /// Get file info
    pub get_info: unsafe extern "efiapi" fn(
        this: *mut Self,
        information_type: *const Guid,
        buffer_size: *mut usize,
        buffer: *mut u8,
    ) -> Status,

    /// Set file info
    pub set_info: unsafe extern "efiapi" fn(
        this: *mut Self,
        information_type: *const Guid,
        buffer_size: usize,
        buffer: *const u8,
    ) -> Status,

    /// Flush file
    pub flush: unsafe extern "efiapi" fn(this: *mut Self) -> Status,

    // EFI_FILE_PROTOCOL Revision 2 (UEFI 2.0+)
    /// Open a file (extended)
    pub open_ex: Option<
        unsafe extern "efiapi" fn(
            this: *mut Self,
            new_handle: *mut *mut Self,
            file_name: *const Char16,
            open_mode: u64,
            attributes: u64,
            token: *mut EfiFileIoToken,
        ) -> Status,
    >,

    /// Read from a file (extended)
    pub read_ex:
        Option<unsafe extern "efiapi" fn(this: *mut Self, token: *mut EfiFileIoToken) -> Status>,

    /// Write to a file (extended)
    pub write_ex:
        Option<unsafe extern "efiapi" fn(this: *mut Self, token: *mut EfiFileIoToken) -> Status>,

    /// Flush file (extended)
    pub flush_ex:
        Option<unsafe extern "efiapi" fn(this: *mut Self, token: *mut EfiFileIoToken) -> Status>,
}

impl EfiFileProtocol {
    /// Protocol revision 1.0
    pub const REVISION_1: u64 = 0x00010000;
    /// Protocol revision 2.0
    pub const REVISION_2: u64 = 0x00020000;

    /// Open a file
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer and file name are valid.
    pub unsafe fn open(
        &mut self,
        file_name: *const Char16,
        open_mode: FileMode,
        attributes: FileAttribute,
    ) -> Result<*mut Self, Status> {
        let mut handle = core::ptr::null_mut();
        let status = (self.open)(self, &mut handle, file_name, open_mode.0, attributes.0);
        status.to_status_result_with(handle)
    }

    /// Close the file
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn close(&mut self) -> Result<(), Status> {
        let status = (self.close)(self);
        status.to_status_result()
    }

    /// Delete the file
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn delete(&mut self) -> Result<(), Status> {
        let status = (self.delete)(self);
        status.to_status_result()
    }

    /// Read from the file
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer and buffer are valid.
    pub unsafe fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Status> {
        let mut size = buffer.len();
        let status = (self.read)(self, &mut size, buffer.as_mut_ptr());
        status.to_status_result_with(size)
    }

    /// Write to the file
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn write(&mut self, buffer: &[u8]) -> Result<usize, Status> {
        let mut size = buffer.len();
        let status = (self.write)(self, &mut size, buffer.as_ptr());
        status.to_status_result_with(size)
    }

    /// Get the current file position
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn get_position(&mut self) -> Result<u64, Status> {
        let mut position = 0;
        let status = (self.get_position)(self, &mut position);
        status.to_status_result_with(position)
    }

    /// Set the file position
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn set_position(&mut self, position: u64) -> Result<(), Status> {
        let status = (self.set_position)(self, position);
        status.to_status_result()
    }

    /// Seek to the end of the file
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn seek_to_end(&mut self) -> Result<(), Status> {
        self.set_position(0xFFFFFFFFFFFFFFFF)
    }

    /// Rewind to the beginning of the file
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn rewind(&mut self) -> Result<(), Status> {
        self.set_position(0)
    }

    /// Get file info
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer and buffer are valid.
    pub unsafe fn get_info(
        &mut self,
        info_type: &Guid,
        buffer: &mut [u8],
    ) -> Result<usize, Status> {
        let mut size = buffer.len();
        let status = (self.get_info)(self, info_type, &mut size, buffer.as_mut_ptr());

        if status == Status::BUFFER_TOO_SMALL {
            // Return required size
            Err(status)
        } else {
            status.to_status_result_with(size)
        }
    }

    /// Get required buffer size for file info
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn get_info_size(&mut self, info_type: &Guid) -> Result<usize, Status> {
        let mut size = 0;
        let status = (self.get_info)(self, info_type, &mut size, core::ptr::null_mut());

        if status == Status::BUFFER_TOO_SMALL || status.is_success() {
            Ok(size)
        } else {
            Err(status)
        }
    }

    /// Flush the file
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn flush(&mut self) -> Result<(), Status> {
        let status = (self.flush)(self);
        status.to_status_result()
    }
}

// =============================================================================
// FILE MODE
// =============================================================================

/// File open mode flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct FileMode(pub u64);

/// Compatibility constant: Open for reading
pub const FILE_MODE_READ: u64 = 0x0000000000000001;
/// Compatibility constant: Open for writing
pub const FILE_MODE_WRITE: u64 = 0x0000000000000002;
/// Compatibility constant: Create file
pub const FILE_MODE_CREATE: u64 = 0x8000000000000000;

impl FileMode {
    /// Open for reading
    pub const READ: Self = Self(0x0000000000000001);
    /// Open for writing
    pub const WRITE: Self = Self(0x0000000000000002);
    /// Create file if it doesn't exist
    pub const CREATE: Self = Self(0x8000000000000000);

    /// Open for reading only
    pub const READ_ONLY: Self = Self::READ;
    /// Open for reading and writing
    pub const READ_WRITE: Self = Self(Self::READ.0 | Self::WRITE.0);
    /// Create and open for reading and writing
    pub const CREATE_READ_WRITE: Self = Self(Self::READ.0 | Self::WRITE.0 | Self::CREATE.0);

    /// Combine modes
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl core::ops::BitOr for FileMode {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

// =============================================================================
// FILE ATTRIBUTE
// =============================================================================

/// File attribute flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct FileAttribute(pub u64);

/// Compatibility constant: Read-only
pub const FILE_ATTRIBUTE_READ_ONLY: u64 = 0x0000000000000001;
/// Compatibility constant: Hidden
pub const FILE_ATTRIBUTE_HIDDEN: u64 = 0x0000000000000002;
/// Compatibility constant: System
pub const FILE_ATTRIBUTE_SYSTEM: u64 = 0x0000000000000004;
/// Compatibility constant: Directory
pub const FILE_ATTRIBUTE_DIRECTORY: u64 = 0x0000000000000010;
/// Compatibility constant: Archive
pub const FILE_ATTRIBUTE_ARCHIVE: u64 = 0x0000000000000020;

impl FileAttribute {
    /// No attributes
    pub const NONE: Self = Self(0);
    /// Read-only
    pub const READ_ONLY: Self = Self(0x0000000000000001);
    /// Hidden
    pub const HIDDEN: Self = Self(0x0000000000000002);
    /// System file
    pub const SYSTEM: Self = Self(0x0000000000000004);
    /// Reserved
    pub const RESERVED: Self = Self(0x0000000000000008);
    /// Directory
    pub const DIRECTORY: Self = Self(0x0000000000000010);
    /// Archive
    pub const ARCHIVE: Self = Self(0x0000000000000020);
    /// Valid attribute mask
    pub const VALID_ATTR: Self = Self(0x0000000000000037);

    /// Check if this is a directory
    pub const fn is_directory(self) -> bool {
        (self.0 & Self::DIRECTORY.0) != 0
    }

    /// Check if read-only
    pub const fn is_read_only(self) -> bool {
        (self.0 & Self::READ_ONLY.0) != 0
    }

    /// Check if hidden
    pub const fn is_hidden(self) -> bool {
        (self.0 & Self::HIDDEN.0) != 0
    }

    /// Check if system file
    pub const fn is_system(self) -> bool {
        (self.0 & Self::SYSTEM.0) != 0
    }

    /// Combine attributes
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl core::ops::BitOr for FileAttribute {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

// =============================================================================
// FILE INFO
// =============================================================================

/// File information structure
#[derive(Clone, Copy)]
#[repr(C)]
pub struct EfiFileInfo {
    /// Size of this structure including file name
    pub size: u64,
    /// File size in bytes
    pub file_size: u64,
    /// Physical size in bytes
    pub physical_size: u64,
    /// Creation time
    pub create_time: Time,
    /// Last access time
    pub last_access_time: Time,
    /// Modification time
    pub modification_time: Time,
    /// File attributes
    pub attribute: FileAttribute,
    /// File name (variable length, null-terminated UCS-2)
    /// This is actually a flexible array member
    pub file_name: [Char16; 1],
}

impl EfiFileInfo {
    /// File info GUID
    pub const GUID: Guid = guids::FILE_INFO;

    /// Get file name as a slice
    ///
    /// # Safety
    /// The caller must ensure the structure is valid and properly sized.
    pub unsafe fn file_name(&self) -> &[Char16] {
        let name_offset = core::mem::offset_of!(Self, file_name);
        let size = usize::try_from(self.size).unwrap_or(0);
        let name_len = size.saturating_sub(name_offset) / 2;

        let ptr = core::ptr::addr_of!(self.file_name[0]);
        let slice = core::slice::from_raw_parts(ptr, name_len);

        // Find null terminator
        let actual_len = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());

        &slice[..actual_len]
    }

    /// Check if this is a directory
    pub const fn is_directory(&self) -> bool {
        self.attribute.is_directory()
    }

    /// Check if this is a regular file
    pub const fn is_file(&self) -> bool {
        !self.attribute.is_directory()
    }
}

impl fmt::Debug for EfiFileInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EfiFileInfo")
            .field("size", &self.size)
            .field("file_size", &self.file_size)
            .field("physical_size", &self.physical_size)
            .field("attribute", &self.attribute)
            .finish_non_exhaustive()
    }
}

/// File system information structure
#[derive(Clone, Copy)]
#[repr(C)]
pub struct EfiFileSystemInfo {
    /// Size of this structure including volume label
    pub size: u64,
    /// Read-only flag
    pub read_only: Boolean,
    /// Volume size in bytes
    pub volume_size: u64,
    /// Free space in bytes
    pub free_space: u64,
    /// Block size
    pub block_size: u32,
    /// Volume label (variable length, null-terminated UCS-2)
    pub volume_label: [Char16; 1],
}

impl EfiFileSystemInfo {
    /// File system info GUID
    pub const GUID: Guid = guids::FILE_SYSTEM_INFO;

    /// Get volume label as a slice
    ///
    /// # Safety
    /// The caller must ensure the structure is valid and properly sized.
    pub unsafe fn volume_label(&self) -> &[Char16] {
        let label_offset = core::mem::offset_of!(Self, volume_label);
        let size = usize::try_from(self.size).unwrap_or(0);
        let label_len = size.saturating_sub(label_offset) / 2;

        let ptr = core::ptr::addr_of!(self.volume_label[0]);
        let slice = core::slice::from_raw_parts(ptr, label_len);

        // Find null terminator
        let actual_len = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());

        &slice[..actual_len]
    }

    /// Check if the volume is read-only
    pub const fn is_read_only(&self) -> bool {
        self.read_only != 0
    }
}

impl fmt::Debug for EfiFileSystemInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EfiFileSystemInfo")
            .field("size", &self.size)
            .field("read_only", &(self.read_only != 0))
            .field("volume_size", &self.volume_size)
            .field("free_space", &self.free_space)
            .field("block_size", &self.block_size)
            .finish_non_exhaustive()
    }
}

/// File system volume label
#[derive(Clone, Copy)]
#[repr(C)]
pub struct EfiFileSystemVolumeLabel {
    /// Volume label (variable length, null-terminated UCS-2)
    pub volume_label: [Char16; 1],
}

impl EfiFileSystemVolumeLabel {
    /// File system volume label GUID
    pub const GUID: Guid = guids::FILE_SYSTEM_VOLUME_LABEL;
}

// =============================================================================
// FILE I/O TOKEN
// =============================================================================

/// File I/O token for async operations
#[derive(Debug)]
#[repr(C)]
pub struct EfiFileIoToken {
    /// Event to signal on completion
    pub event: Event,
    /// Status of the operation
    pub status: Status,
    /// Number of bytes transferred
    pub buffer_size: usize,
    /// Buffer for the data
    pub buffer: *mut u8,
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_file_mode() {
        let mode = FileMode::READ | FileMode::WRITE;
        assert_eq!(mode.0, 0x03);
    }

    #[test]
    fn test_file_attribute() {
        let attr = FileAttribute::DIRECTORY;
        assert!(attr.is_directory());
        assert!(!attr.is_read_only());
    }
}
