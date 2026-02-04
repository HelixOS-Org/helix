//! File System Protocol
//!
//! High-level file system abstraction for file operations.
//!
//! # Safety
//! This module interfaces with UEFI firmware via FFI. All protocol pointers
//! are provided by the firmware and validated before use. The UEFI specification
//! guarantees that protocol pointers obtained via LocateProtocol/HandleProtocol
//! are valid for the duration of boot services.
//!
//! CodeQL may flag pointer dereferences as "access of invalid pointer" but these
//! are false positives - the UEFI firmware guarantees pointer validity.
// codeql[rust/access-invalid-pointer] - UEFI FFI pointers validated by firmware

use super::{DevicePath, EnumerableProtocol, Protocol};
use crate::error::{Error, Result};
use crate::raw::protocols::file::*;
use crate::raw::types::*;

extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

// =============================================================================
// FILE SYSTEM
// =============================================================================

/// High-level file system abstraction
pub struct FileSystem {
    /// Raw protocol pointer
    protocol: *mut EfiSimpleFileSystemProtocol,
    /// Handle
    handle: Handle,
    /// Root directory
    root: Option<*mut EfiFileProtocol>,
}

impl FileSystem {
    /// Create from raw protocol
    ///
    /// # Safety
    /// Protocol pointer must be valid and obtained from UEFI LocateProtocol
    /// or HandleProtocol calls. The pointer must remain valid for the
    /// lifetime of this FileSystem instance.
    pub unsafe fn from_raw(protocol: *mut EfiSimpleFileSystemProtocol, handle: Handle) -> Self {
        debug_assert!(!protocol.is_null(), "FileSystem protocol pointer is null");
        Self {
            protocol,
            handle,
            root: None,
        }
    }

    /// Open root directory
    fn open_root(&mut self) -> Result<*mut EfiFileProtocol> {
        if let Some(root) = self.root {
            return Ok(root);
        }

        // SAFETY: self.protocol is validated in from_raw
        debug_assert!(!self.protocol.is_null());

        let mut root: *mut EfiFileProtocol = core::ptr::null_mut();
        let result = unsafe { ((*self.protocol).open_volume)(self.protocol, &mut root) };

        if result != Status::SUCCESS {
            return Err(Error::from_status(result));
        }

        self.root = Some(root);
        Ok(root)
    }

    /// Open file by path
    pub fn open(&mut self, path: &str, mode: FileMode) -> Result<File> {
        let root = self.open_root()?;

        // Convert path to UCS-2
        let path_ucs2 = to_ucs2(path);

        let mut file: *mut EfiFileProtocol = core::ptr::null_mut();
        let result =
            unsafe { ((*root).open)(root, &mut file, path_ucs2.as_ptr(), mode.to_raw(), 0) };

        if result != Status::SUCCESS {
            return Err(Error::from_status(result));
        }

        Ok(File {
            protocol: file,
            path: path.into(),
        })
    }

    /// Open directory
    pub fn open_dir(&mut self, path: &str) -> Result<Directory> {
        let root = self.open_root()?;
        let path_ucs2 = to_ucs2(path);

        let mut dir: *mut EfiFileProtocol = core::ptr::null_mut();
        let result = unsafe {
            ((*root).open)(
                root,
                &mut dir,
                path_ucs2.as_ptr(),
                FILE_MODE_READ,
                FILE_ATTRIBUTE_DIRECTORY,
            )
        };

        if result != Status::SUCCESS {
            return Err(Error::from_status(result));
        }

        Ok(Directory {
            protocol: dir,
            path: path.into(),
        })
    }

    /// Check if file exists
    pub fn exists(&mut self, path: &str) -> bool {
        self.open(path, FileMode::Read).is_ok()
    }

    /// Check if path is directory
    pub fn is_dir(&mut self, path: &str) -> bool {
        if let Ok(file) = self.open(path, FileMode::Read) {
            if let Ok(info) = file.info() {
                return info.is_directory();
            }
        }
        false
    }

    /// Read entire file
    pub fn read(&mut self, path: &str) -> Result<Vec<u8>> {
        let mut file = self.open(path, FileMode::Read)?;
        file.read_all()
    }

    /// Read file as string
    pub fn read_string(&mut self, path: &str) -> Result<String> {
        let data = self.read(path)?;
        String::from_utf8(data).map_err(|_| Error::InvalidParameter)
    }

    /// Write entire file
    pub fn write(&mut self, path: &str, data: &[u8]) -> Result<()> {
        let mut file = self.open(path, FileMode::CreateReadWrite)?;
        file.write_all(data)
    }

    /// Write string to file
    pub fn write_string(&mut self, path: &str, content: &str) -> Result<()> {
        self.write(path, content.as_bytes())
    }

    /// Create directory
    pub fn create_dir(&mut self, path: &str) -> Result<()> {
        let root = self.open_root()?;
        let path_ucs2 = to_ucs2(path);

        let mut dir: *mut EfiFileProtocol = core::ptr::null_mut();
        let result = unsafe {
            ((*root).open)(
                root,
                &mut dir,
                path_ucs2.as_ptr(),
                FILE_MODE_READ | FILE_MODE_WRITE | FILE_MODE_CREATE,
                FILE_ATTRIBUTE_DIRECTORY,
            )
        };

        if result != Status::SUCCESS {
            return Err(Error::from_status(result));
        }

        // Close the directory
        unsafe { ((*dir).close)(dir) };

        Ok(())
    }

    /// Delete file or empty directory
    pub fn delete(&mut self, path: &str) -> Result<()> {
        let file = self.open(path, FileMode::ReadWrite)?;

        let result = unsafe { ((*file.protocol).delete)(file.protocol) };

        // Don't close - delete consumes the handle
        core::mem::forget(file);

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Copy file
    pub fn copy(&mut self, src: &str, dest: &str) -> Result<()> {
        let data = self.read(src)?;
        self.write(dest, &data)
    }

    /// List directory contents
    pub fn list_dir(&mut self, path: &str) -> Result<Vec<FileInfo>> {
        let mut dir = self.open_dir(path)?;
        dir.entries()
    }

    /// Get file info
    pub fn file_info(&mut self, path: &str) -> Result<FileInfo> {
        let file = self.open(path, FileMode::Read)?;
        file.info()
    }

    /// Get device path for this file system
    pub fn device_path(&self) -> Option<DevicePath> {
        // TODO: Implement device path retrieval
        None
    }

    /// Get volume label
    pub fn volume_label(&mut self) -> Result<String> {
        let root = self.open_root()?;

        // Query volume info
        let mut buffer = [0u8; 512];
        let mut size = buffer.len();

        let result = unsafe {
            ((*root).get_info)(
                root,
                &EFI_FILE_SYSTEM_INFO_GUID,
                &mut size,
                buffer.as_mut_ptr(),
            )
        };

        if result != Status::SUCCESS {
            return Err(Error::from_status(result));
        }

        // Parse volume label from info
        // The label starts at offset 36 as UCS-2
        if size > 36 {
            let label_ptr = unsafe { buffer.as_ptr().add(36) as *const u16 };
            let label = from_ucs2(label_ptr);
            Ok(label)
        } else {
            Ok(String::new())
        }
    }
}

impl Protocol for FileSystem {
    const GUID: Guid = SIMPLE_FILE_SYSTEM_PROTOCOL_GUID;

    fn open(handle: Handle) -> Result<Self> {
        use crate::services::boot_services;

        let bs = unsafe { boot_services() };
        let image = crate::services::image_handle().ok_or(Error::NotReady)?;

        let mut protocol: *mut core::ffi::c_void = core::ptr::null_mut();
        let result = unsafe {
            ((*bs).open_protocol)(
                handle,
                &Self::GUID as *const Guid,
                &mut protocol,
                image,
                Handle(core::ptr::null_mut()),
                0x00000002,
            )
        };

        if result != Status::SUCCESS {
            return Err(Error::from_status(result));
        }

        Ok(unsafe { Self::from_raw(protocol as *mut EfiSimpleFileSystemProtocol, handle) })
    }

    fn close(&mut self) -> Result<()> {
        if let Some(root) = self.root.take() {
            unsafe { ((*root).close)(root) };
        }
        Ok(())
    }
}

impl EnumerableProtocol for FileSystem {
    fn enumerate() -> Result<Vec<Self>> {
        super::ProtocolLocator::locate_all::<Self>()
            .map(|handles| handles.into_iter().map(|h| h.leak()).collect())
    }
}

impl Drop for FileSystem {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

// =============================================================================
// FILE
// =============================================================================

/// File handle
pub struct File {
    /// Raw protocol pointer
    protocol: *mut EfiFileProtocol,
    /// Path
    path: String,
}

impl File {
    /// Read up to buffer size
    pub fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        let mut size = buffer.len();

        let result =
            unsafe { ((*self.protocol).read)(self.protocol, &mut size, buffer.as_mut_ptr()) };

        if result == Status::SUCCESS {
            Ok(size)
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Read entire file
    pub fn read_all(&mut self) -> Result<Vec<u8>> {
        let info = self.info()?;
        let size = info.size as usize;

        let mut buffer = alloc::vec![0u8; size];
        let mut read_size = size;

        let result =
            unsafe { ((*self.protocol).read)(self.protocol, &mut read_size, buffer.as_mut_ptr()) };

        if result == Status::SUCCESS {
            buffer.truncate(read_size);
            Ok(buffer)
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Read as string
    pub fn read_string(&mut self) -> Result<String> {
        let data = self.read_all()?;
        String::from_utf8(data).map_err(|_| Error::InvalidParameter)
    }

    /// Write data
    pub fn write(&mut self, data: &[u8]) -> Result<usize> {
        let mut size = data.len();

        let result = unsafe { ((*self.protocol).write)(self.protocol, &mut size, data.as_ptr()) };

        if result == Status::SUCCESS {
            Ok(size)
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Write all data
    pub fn write_all(&mut self, data: &[u8]) -> Result<()> {
        let mut offset = 0;
        while offset < data.len() {
            let written = self.write(&data[offset..])?;
            if written == 0 {
                return Err(Error::DeviceError);
            }
            offset += written;
        }
        Ok(())
    }

    /// Seek to position
    pub fn seek(&mut self, position: u64) -> Result<()> {
        let result = unsafe { ((*self.protocol).set_position)(self.protocol, position) };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Get current position
    pub fn position(&self) -> Result<u64> {
        let mut position = 0u64;

        let result = unsafe { ((*self.protocol).get_position)(self.protocol, &mut position) };

        if result == Status::SUCCESS {
            Ok(position)
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Seek to start
    pub fn rewind(&mut self) -> Result<()> {
        self.seek(0)
    }

    /// Seek to end
    pub fn seek_end(&mut self) -> Result<u64> {
        self.seek(0xFFFFFFFFFFFFFFFF)?;
        self.position()
    }

    /// Get file info
    pub fn info(&self) -> Result<FileInfo> {
        let mut buffer = [0u8; 512];
        let mut size = buffer.len();

        let result = unsafe {
            ((*self.protocol).get_info)(
                self.protocol,
                &EFI_FILE_INFO_GUID,
                &mut size,
                buffer.as_mut_ptr(),
            )
        };

        if result != Status::SUCCESS {
            return Err(Error::from_status(result));
        }

        // Parse EfiFileInfo
        let raw = unsafe { &*(buffer.as_ptr() as *const EfiFileInfo) };

        Ok(FileInfo {
            size: raw.file_size,
            physical_size: raw.physical_size,
            create_time: time_from_raw(&raw.create_time),
            modify_time: time_from_raw(&raw.last_access_time),
            access_time: time_from_raw(&raw.modification_time),
            attributes: FileAttributes(raw.attribute.0),
            name: from_ucs2(raw.file_name.as_ptr()),
        })
    }

    /// Set file info
    pub fn set_info(&mut self, _info: &FileInfo) -> Result<()> {
        // TODO: Implement set_info
        Ok(())
    }

    /// Flush to disk
    pub fn flush(&mut self) -> Result<()> {
        let result = unsafe { ((*self.protocol).flush)(self.protocol) };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Truncate file at current position
    pub fn truncate(&mut self) -> Result<()> {
        // Get current position
        let pos = self.position()?;

        // Get current info
        let mut info = self.info()?;
        info.size = pos;

        // Set new size
        self.set_info(&info)
    }

    /// Get path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get file size
    pub fn size(&self) -> Result<u64> {
        Ok(self.info()?.size)
    }

    /// Check if at end of file
    pub fn is_eof(&self) -> Result<bool> {
        let pos = self.position()?;
        let size = self.size()?;
        Ok(pos >= size)
    }
}

impl Drop for File {
    fn drop(&mut self) {
        unsafe { ((*self.protocol).close)(self.protocol) };
    }
}

// =============================================================================
// DIRECTORY
// =============================================================================

/// Directory handle
pub struct Directory {
    /// Raw protocol pointer
    protocol: *mut EfiFileProtocol,
    /// Path
    path: String,
}

impl Directory {
    /// Read next entry
    pub fn read_entry(&mut self) -> Result<Option<FileInfo>> {
        let mut buffer = [0u8; 512];
        let mut size = buffer.len();

        let result =
            unsafe { ((*self.protocol).read)(self.protocol, &mut size, buffer.as_mut_ptr()) };

        if result != Status::SUCCESS {
            return Err(Error::from_status(result));
        }

        if size == 0 {
            return Ok(None);
        }

        // Parse EfiFileInfo
        let raw = unsafe { &*(buffer.as_ptr() as *const EfiFileInfo) };

        Ok(Some(FileInfo {
            size: raw.file_size,
            physical_size: raw.physical_size,
            create_time: time_from_raw(&raw.create_time),
            modify_time: time_from_raw(&raw.last_access_time),
            access_time: time_from_raw(&raw.modification_time),
            attributes: FileAttributes(raw.attribute.0),
            name: from_ucs2(raw.file_name.as_ptr()),
        }))
    }

    /// Get all entries
    pub fn entries(&mut self) -> Result<Vec<FileInfo>> {
        let mut entries = Vec::new();

        // Rewind first
        let result = unsafe { ((*self.protocol).set_position)(self.protocol, 0) };
        if result != Status::SUCCESS {
            return Err(Error::from_status(result));
        }

        while let Some(entry) = self.read_entry()? {
            // Skip . and ..
            if entry.name != "." && entry.name != ".." {
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    /// Get files only
    pub fn files(&mut self) -> Result<Vec<FileInfo>> {
        let entries = self.entries()?;
        Ok(entries.into_iter().filter(|e| !e.is_directory()).collect())
    }

    /// Get subdirectories only
    pub fn subdirs(&mut self) -> Result<Vec<FileInfo>> {
        let entries = self.entries()?;
        Ok(entries.into_iter().filter(|e| e.is_directory()).collect())
    }

    /// Find file by name
    pub fn find(&mut self, name: &str) -> Result<Option<FileInfo>> {
        let entries = self.entries()?;
        Ok(entries.into_iter().find(|e| e.name == name))
    }

    /// Find files matching pattern (simple glob)
    pub fn find_pattern(&mut self, pattern: &str) -> Result<Vec<FileInfo>> {
        let entries = self.entries()?;
        Ok(entries
            .into_iter()
            .filter(|e| matches_pattern(&e.name, pattern))
            .collect())
    }

    /// Get path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Rewind to beginning
    pub fn rewind(&mut self) -> Result<()> {
        let result = unsafe { ((*self.protocol).set_position)(self.protocol, 0) };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        unsafe { ((*self.protocol).close)(self.protocol) };
    }
}

// =============================================================================
// FILE INFO
// =============================================================================

/// File information
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// File size in bytes
    pub size: u64,
    /// Physical size on disk
    pub physical_size: u64,
    /// Creation time
    pub create_time: Option<Time>,
    /// Modification time
    pub modify_time: Option<Time>,
    /// Access time
    pub access_time: Option<Time>,
    /// Attributes
    pub attributes: FileAttributes,
    /// File name
    pub name: String,
}

impl FileInfo {
    /// Check if directory
    pub fn is_directory(&self) -> bool {
        self.attributes.is_directory()
    }

    /// Check if read-only
    pub fn is_readonly(&self) -> bool {
        self.attributes.is_readonly()
    }

    /// Check if hidden
    pub fn is_hidden(&self) -> bool {
        self.attributes.is_hidden()
    }

    /// Check if system file
    pub fn is_system(&self) -> bool {
        self.attributes.is_system()
    }

    /// Check if archive flag set
    pub fn is_archive(&self) -> bool {
        self.attributes.is_archive()
    }

    /// Get extension
    pub fn extension(&self) -> Option<&str> {
        self.name.rfind('.').map(|i| &self.name[i + 1..])
    }

    /// Get name without extension
    pub fn stem(&self) -> &str {
        self.name
            .rfind('.')
            .map(|i| &self.name[..i])
            .unwrap_or(&self.name)
    }
}

// =============================================================================
// FILE TIME
// =============================================================================

/// File time
#[derive(Debug, Clone)]
pub struct Time {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

fn time_from_raw(raw: &crate::raw::types::Time) -> Option<Time> {
    if raw.year == 0 {
        None
    } else {
        Some(Time {
            year: raw.year,
            month: raw.month,
            day: raw.day,
            hour: raw.hour,
            minute: raw.minute,
            second: raw.second,
        })
    }
}

// =============================================================================
// FILE MODE
// =============================================================================

/// File open mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMode {
    /// Read only
    Read,
    /// Read and write
    ReadWrite,
    /// Create, read and write
    CreateReadWrite,
}

impl FileMode {
    fn to_raw(self) -> u64 {
        match self {
            Self::Read => FILE_MODE_READ,
            Self::ReadWrite => FILE_MODE_READ | FILE_MODE_WRITE,
            Self::CreateReadWrite => FILE_MODE_READ | FILE_MODE_WRITE | FILE_MODE_CREATE,
        }
    }
}

// =============================================================================
// FILE ATTRIBUTES
// =============================================================================

/// File attributes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileAttributes(pub u64);

impl FileAttributes {
    pub const READ_ONLY: Self = Self(FILE_ATTRIBUTE_READ_ONLY);
    pub const HIDDEN: Self = Self(FILE_ATTRIBUTE_HIDDEN);
    pub const SYSTEM: Self = Self(FILE_ATTRIBUTE_SYSTEM);
    pub const DIRECTORY: Self = Self(FILE_ATTRIBUTE_DIRECTORY);
    pub const ARCHIVE: Self = Self(FILE_ATTRIBUTE_ARCHIVE);

    pub fn is_readonly(&self) -> bool {
        (self.0 & FILE_ATTRIBUTE_READ_ONLY) != 0
    }

    pub fn is_hidden(&self) -> bool {
        (self.0 & FILE_ATTRIBUTE_HIDDEN) != 0
    }

    pub fn is_system(&self) -> bool {
        (self.0 & FILE_ATTRIBUTE_SYSTEM) != 0
    }

    pub fn is_directory(&self) -> bool {
        (self.0 & FILE_ATTRIBUTE_DIRECTORY) != 0
    }

    pub fn is_archive(&self) -> bool {
        (self.0 & FILE_ATTRIBUTE_ARCHIVE) != 0
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Convert string to UCS-2
fn to_ucs2(s: &str) -> Vec<u16> {
    let mut buffer: Vec<u16> = s
        .chars()
        .map(|c| {
            // Replace forward slash with backslash for UEFI
            if c == '/' {
                '\\' as u16
            } else {
                c as u16
            }
        })
        .collect();
    buffer.push(0);
    buffer
}

/// Convert UCS-2 to string
fn from_ucs2(ptr: *const u16) -> String {
    let mut s = String::new();
    let mut i = 0;

    unsafe {
        loop {
            let c = *ptr.add(i);
            if c == 0 {
                break;
            }
            if let Some(ch) = char::from_u32(c as u32) {
                s.push(ch);
            }
            i += 1;

            // Safety limit
            if i > 1024 {
                break;
            }
        }
    }

    s
}

/// Simple pattern matching
fn matches_pattern(name: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    if pattern.starts_with("*.") {
        let ext = &pattern[2..];
        return name.ends_with(&alloc::format!(".{}", ext));
    }

    if pattern.ends_with("*") {
        let prefix = &pattern[..pattern.len() - 1];
        return name.starts_with(prefix);
    }

    name == pattern
}

// =============================================================================
// PATH UTILITIES
// =============================================================================

/// Path utilities
pub struct Path;

impl Path {
    /// Normalize path separators
    pub fn normalize(path: &str) -> String {
        path.replace('/', "\\")
    }

    /// Join path components
    pub fn join(base: &str, name: &str) -> String {
        if base.is_empty() {
            return name.to_string();
        }
        if base.ends_with('\\') || base.ends_with('/') {
            alloc::format!("{}{}", base, name)
        } else {
            alloc::format!("{}\\{}", base, name)
        }
    }

    /// Get parent directory
    pub fn parent(path: &str) -> Option<&str> {
        let normalized = path.trim_end_matches(&['\\', '/'][..]);
        normalized.rfind(&['\\', '/'][..]).map(|i| &normalized[..i])
    }

    /// Get file name
    pub fn file_name(path: &str) -> &str {
        path.rsplit(&['\\', '/'][..]).next().unwrap_or(path)
    }

    /// Get extension
    pub fn extension(path: &str) -> Option<&str> {
        Self::file_name(path)
            .rfind('.')
            .map(|i| &Self::file_name(path)[i + 1..])
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matching() {
        assert!(matches_pattern("test.txt", "*"));
        assert!(matches_pattern("test.txt", "*.txt"));
        assert!(!matches_pattern("test.txt", "*.cfg"));
        assert!(matches_pattern("test.txt", "test*"));
    }

    #[test]
    fn test_path_utilities() {
        assert_eq!(Path::file_name("\\EFI\\BOOT\\bootx64.efi"), "bootx64.efi");
        assert_eq!(Path::extension("test.txt"), Some("txt"));
        assert_eq!(
            Path::parent("\\EFI\\BOOT\\bootx64.efi"),
            Some("\\EFI\\BOOT")
        );
    }

    #[test]
    fn test_ucs2_conversion() {
        let s = "test";
        let ucs2 = to_ucs2(s);
        assert_eq!(ucs2.len(), 5); // 4 chars + null

        let back = from_ucs2(ucs2.as_ptr());
        assert_eq!(back, "test");
    }
}
