//! # File Abstraction Module
//!
//! This module provides file abstractions for kernel modules and other
//! files loaded by the bootloader.
//!
//! ## Features
//!
//! - Kernel file access
//! - Module loading and enumeration
//! - File path parsing
//! - Media type detection
//! - Embedded file support

use core::{slice, str};

// =============================================================================
// File Representation
// =============================================================================

/// Represents a file loaded by the bootloader
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct File {
    /// Virtual address of file contents
    address: u64,
    /// File size in bytes
    size: u64,
    /// File path (null-terminated)
    path: *const u8,
    /// Command line (null-terminated, may be null)
    cmdline: *const u8,
    /// Media type
    media_type: MediaType,
    /// Partition index
    partition: u32,
    /// TFTP IP (for network boot)
    tftp_ip: u32,
    /// TFTP port
    tftp_port: u32,
    /// MBR disk ID
    mbr_disk_id: u32,
    /// GPT disk GUID
    gpt_disk_uuid: [u8; 16],
    /// GPT partition GUID
    gpt_part_uuid: [u8; 16],
    /// Partition GUID (filesystem)
    part_uuid: [u8; 16],
}

impl File {
    /// Create a file reference from raw data
    ///
    /// # Safety
    ///
    /// All pointers must be valid and point to null-terminated strings.
    pub const unsafe fn from_raw(
        address: u64,
        size: u64,
        path: *const u8,
        cmdline: *const u8,
        media_type: MediaType,
        partition: u32,
    ) -> Self {
        Self {
            address,
            size,
            path,
            cmdline,
            media_type,
            partition,
            tftp_ip: 0,
            tftp_port: 0,
            mbr_disk_id: 0,
            gpt_disk_uuid: [0; 16],
            gpt_part_uuid: [0; 16],
            part_uuid: [0; 16],
        }
    }

    /// Get file contents as byte slice
    #[allow(clippy::cast_possible_truncation)] // size fits in usize on 64-bit systems
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.address as *const u8, self.size as usize) }
    }

    /// Get file address
    pub fn address(&self) -> u64 {
        self.address
    }

    /// Get file size
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Get file path
    pub fn path(&self) -> &str {
        if self.path.is_null() {
            return "";
        }
        unsafe {
            let mut len = 0;
            while *self.path.add(len) != 0 {
                len += 1;
            }
            let bytes = slice::from_raw_parts(self.path, len);
            str::from_utf8(bytes).unwrap_or("")
        }
    }

    /// Get file name (last component of path)
    pub fn name(&self) -> &str {
        let path = self.path();
        path.rsplit('/').next().unwrap_or(path)
    }

    /// Get file extension
    pub fn extension(&self) -> Option<&str> {
        let name = self.name();
        if let Some(pos) = name.rfind('.') {
            Some(&name[pos + 1..])
        } else {
            None
        }
    }

    /// Get command line
    pub fn cmdline(&self) -> Option<&str> {
        if self.cmdline.is_null() {
            return None;
        }
        unsafe {
            let mut len = 0;
            while *self.cmdline.add(len) != 0 {
                len += 1;
            }
            let bytes = slice::from_raw_parts(self.cmdline, len);
            str::from_utf8(bytes).ok()
        }
    }

    /// Get media type
    pub fn media_type(&self) -> MediaType {
        self.media_type
    }

    /// Get partition index
    pub fn partition(&self) -> u32 {
        self.partition
    }

    /// Check if file is from network boot
    pub fn is_network_boot(&self) -> bool {
        matches!(self.media_type, MediaType::Tftp)
    }

    /// Get TFTP server IP
    #[allow(clippy::cast_possible_truncation)] // extracting individual bytes from u32 IP address
    pub fn tftp_ip(&self) -> Option<[u8; 4]> {
        if !self.is_network_boot() {
            return None;
        }
        Some([
            (self.tftp_ip >> 24) as u8,
            (self.tftp_ip >> 16) as u8,
            (self.tftp_ip >> 8) as u8,
            self.tftp_ip as u8,
        ])
    }

    /// Get GPT disk UUID
    pub fn gpt_disk_uuid(&self) -> Option<&[u8; 16]> {
        if self.gpt_disk_uuid == [0; 16] {
            None
        } else {
            Some(&self.gpt_disk_uuid)
        }
    }

    /// Get GPT partition UUID
    pub fn gpt_partition_uuid(&self) -> Option<&[u8; 16]> {
        if self.gpt_part_uuid == [0; 16] {
            None
        } else {
            Some(&self.gpt_part_uuid)
        }
    }

    /// Check if file is an ELF binary
    pub fn is_elf(&self) -> bool {
        if self.size < 4 {
            return false;
        }
        let bytes = self.as_bytes();
        bytes[0] == 0x7f && bytes[1] == b'E' && bytes[2] == b'L' && bytes[3] == b'F'
    }

    /// Get ELF class (32 or 64 bit)
    pub fn elf_class(&self) -> Option<ElfClass> {
        if !self.is_elf() || self.size < 5 {
            return None;
        }
        match self.as_bytes()[4] {
            1 => Some(ElfClass::Elf32),
            2 => Some(ElfClass::Elf64),
            _ => None,
        }
    }
}

impl core::fmt::Debug for File {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("File")
            .field("path", &self.path())
            .field("size", &self.size)
            .field("address", &format_args!("{:#x}", self.address))
            .field("media_type", &self.media_type)
            .finish()
    }
}

/// Media type for file source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MediaType {
    /// Generic/unknown
    Generic = 0,
    /// Optical disc (CD/DVD)
    Optical = 1,
    /// TFTP network boot
    Tftp    = 2,
}

impl MediaType {
    /// Create from raw value
    pub const fn from_raw(value: u32) -> Self {
        match value {
            1 => Self::Optical,
            2 => Self::Tftp,
            _ => Self::Generic,
        }
    }
}

/// ELF class
///
/// Indicates whether an ELF binary is 32-bit or 64-bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfClass {
    /// 32-bit ELF binary
    Elf32,
    /// 64-bit ELF binary
    Elf64,
}

// =============================================================================
// Module Collection
// =============================================================================

/// Collection of modules loaded by bootloader
pub struct ModuleCollection {
    modules: *const *const File,
    count: usize,
}

impl ModuleCollection {
    /// Create from raw pointers
    ///
    /// # Safety
    ///
    /// Pointers must be valid.
    pub const unsafe fn from_raw(modules: *const *const File, count: usize) -> Self {
        Self { modules, count }
    }

    /// Get module count
    pub fn count(&self) -> usize {
        self.count
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get module by index
    pub fn get(&self, index: usize) -> Option<&File> {
        if index >= self.count {
            return None;
        }
        unsafe {
            let ptr = *self.modules.add(index);
            if ptr.is_null() {
                None
            } else {
                Some(&*ptr)
            }
        }
    }

    /// Find module by path
    pub fn find_by_path(&self, path: &str) -> Option<&File> {
        self.iter().find(|f| f.path() == path)
    }

    /// Find module by name
    pub fn find_by_name(&self, name: &str) -> Option<&File> {
        self.iter().find(|f| f.name() == name)
    }

    /// Find module by command line
    pub fn find_by_cmdline(&self, cmdline: &str) -> Option<&File> {
        self.iter().find(|f| f.cmdline() == Some(cmdline))
    }

    /// Find module containing string in command line
    pub fn find_by_cmdline_contains(&self, pattern: &str) -> Option<&File> {
        self.iter()
            .find(|f| f.cmdline().is_some_and(|c| c.contains(pattern)))
    }

    /// Iterate over modules
    pub fn iter(&self) -> ModuleIterator<'_> {
        ModuleIterator {
            collection: self,
            index: 0,
        }
    }

    /// Get total size of all modules
    pub fn total_size(&self) -> u64 {
        self.iter().map(File::size).sum()
    }
}

/// Iterator over modules
pub struct ModuleIterator<'a> {
    collection: &'a ModuleCollection,
    index: usize,
}

impl<'a> Iterator for ModuleIterator<'a> {
    type Item = &'a File;

    fn next(&mut self) -> Option<Self::Item> {
        let file = self.collection.get(self.index)?;
        self.index += 1;
        Some(file)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.collection.count - self.index;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for ModuleIterator<'_> {}

// =============================================================================
// Path Utilities
// =============================================================================

/// File path parsing utilities
pub struct FilePath<'a> {
    path: &'a str,
}

impl<'a> FilePath<'a> {
    /// Create from string
    pub const fn new(path: &'a str) -> Self {
        Self { path }
    }

    /// Get full path
    pub fn as_str(&self) -> &str {
        self.path
    }

    /// Get file name
    pub fn file_name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(self.path)
    }

    /// Get file stem (name without extension)
    pub fn file_stem(&self) -> &str {
        let name = self.file_name();
        if let Some(pos) = name.rfind('.') {
            &name[..pos]
        } else {
            name
        }
    }

    /// Get extension
    pub fn extension(&self) -> Option<&str> {
        let name = self.file_name();
        if let Some(pos) = name.rfind('.') {
            Some(&name[pos + 1..])
        } else {
            None
        }
    }

    /// Get parent directory
    pub fn parent(&self) -> Option<&str> {
        if let Some(pos) = self.path.rfind('/') {
            if pos == 0 {
                Some("/")
            } else {
                Some(&self.path[..pos])
            }
        } else {
            None
        }
    }

    /// Check if path is absolute
    pub fn is_absolute(&self) -> bool {
        self.path.starts_with('/')
    }

    /// Get path components
    pub fn components(&self) -> impl Iterator<Item = &str> {
        self.path.split('/').filter(|s| !s.is_empty())
    }

    /// Join with another path
    pub fn join(&self, other: &str) -> PathBuf {
        let mut result = PathBuf::new();
        result.push(self.path);
        result.push(other);
        result
    }
}

/// Owned path buffer
pub struct PathBuf {
    buffer: [u8; 256],
    len: usize,
}

impl PathBuf {
    /// Create empty path
    pub const fn new() -> Self {
        Self {
            buffer: [0; 256],
            len: 0,
        }
    }

    /// Create from string
    pub fn from_str(s: &str) -> Self {
        let mut buf = Self::new();
        buf.push(s);
        buf
    }

    /// Get as string slice
    pub fn as_str(&self) -> &str {
        unsafe { str::from_utf8_unchecked(&self.buffer[..self.len]) }
    }

    /// Get remaining capacity
    pub fn remaining(&self) -> usize {
        self.buffer.len() - self.len
    }

    /// Push path component
    pub fn push(&mut self, component: &str) {
        if component.is_empty() {
            return;
        }

        // Add separator if needed
        if self.len > 0
            && self.buffer[self.len - 1] != b'/'
            && !component.starts_with('/')
            && self.remaining() > 0
        {
            self.buffer[self.len] = b'/';
            self.len += 1;
        }

        // Copy component
        let bytes = component.as_bytes();
        let to_copy = bytes.len().min(self.remaining());
        self.buffer[self.len..self.len + to_copy].copy_from_slice(&bytes[..to_copy]);
        self.len += to_copy;
    }

    /// Pop last component
    pub fn pop(&mut self) -> bool {
        if let Some(pos) = self.as_str().rfind('/') {
            self.len = pos;
            true
        } else if self.len > 0 {
            self.len = 0;
            true
        } else {
            false
        }
    }

    /// Clear path
    pub fn clear(&mut self) {
        self.len = 0;
    }
}

impl Default for PathBuf {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Debug for PathBuf {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PathBuf({:?})", self.as_str())
    }
}

// =============================================================================
// File Type Detection
// =============================================================================

/// Detected file type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// Unknown/binary
    Unknown,
    /// ELF executable
    Elf,
    /// PE executable
    Pe,
    /// Mach-O executable
    MachO,
    /// TAR archive
    Tar,
    /// GZIP compressed
    Gzip,
    /// CPIO archive
    Cpio,
    /// Device tree blob
    DeviceTree,
    /// Plain text
    Text,
    /// Initramfs/initrd
    Initrd,
}

impl FileType {
    /// Detect file type from magic bytes
    pub fn detect(data: &[u8]) -> Self {
        if data.len() < 4 {
            return Self::Unknown;
        }

        // ELF
        if data[0..4] == [0x7f, b'E', b'L', b'F'] {
            return Self::Elf;
        }

        // PE
        if data[0..2] == [b'M', b'Z'] {
            return Self::Pe;
        }

        // Mach-O (32 and 64 bit, both endians)
        if data.len() >= 4 {
            let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
            if matches!(magic, 0xfeed_face | 0xfeed_facf | 0xcefa_edfe | 0xcffa_edfe) {
                return Self::MachO;
            }
        }

        // GZIP
        if data[0..2] == [0x1f, 0x8b] {
            return Self::Gzip;
        }

        // TAR (ustar magic at offset 257)
        if data.len() > 262 && &data[257..262] == b"ustar" {
            return Self::Tar;
        }

        // CPIO (various formats)
        if data.len() >= 6 {
            if &data[0..6] == b"070701" || &data[0..6] == b"070702" {
                return Self::Cpio;
            }
            let cpio_magic = u16::from_le_bytes([data[0], data[1]]);
            if cpio_magic == 0o70707 {
                return Self::Cpio;
            }
        }

        // Device Tree Blob
        if data.len() >= 4 {
            let dtb_magic = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
            if dtb_magic == 0xd00d_feed {
                return Self::DeviceTree;
            }
        }

        // Simple text detection (ASCII printable + whitespace)
        if Self::looks_like_text(data) {
            return Self::Text;
        }

        Self::Unknown
    }

    /// Check if data looks like text
    fn looks_like_text(data: &[u8]) -> bool {
        let sample = if data.len() > 512 { &data[..512] } else { data };
        sample
            .iter()
            .all(|&b| b == b'\n' || b == b'\r' || b == b'\t' || (b >= 0x20 && b < 0x7f))
    }

    /// Detect from file
    pub fn detect_file(file: &File) -> Self {
        Self::detect(file.as_bytes())
    }
}

// =============================================================================
// Initramfs Support
// =============================================================================

/// CPIO archive entry
pub struct CpioEntry<'a> {
    /// File name
    pub name: &'a str,
    /// File mode
    pub mode: u32,
    /// File size
    pub size: u32,
    /// File data
    pub data: &'a [u8],
}

/// CPIO archive iterator (newc format)
pub struct CpioIterator<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> CpioIterator<'a> {
    /// Create from data
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    /// Parse hex string
    fn parse_hex(s: &[u8]) -> Option<u32> {
        let mut result = 0u32;
        for &b in s {
            let digit = match b {
                b'0'..=b'9' => b - b'0',
                b'a'..=b'f' => b - b'a' + 10,
                b'A'..=b'F' => b - b'A' + 10,
                _ => return None,
            };
            result = result * 16 + u32::from(digit);
        }
        Some(result)
    }

    /// Align to 4-byte boundary
    fn align4(n: usize) -> usize {
        (n + 3) & !3
    }
}

impl<'a> Iterator for CpioIterator<'a> {
    type Item = CpioEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset + 110 > self.data.len() {
            return None;
        }

        let header = &self.data[self.offset..];

        // Check magic
        if &header[0..6] != b"070701" && &header[0..6] != b"070702" {
            return None;
        }

        // Parse header fields
        let mode = Self::parse_hex(&header[14..22])?;
        let filesize = Self::parse_hex(&header[54..62])? as usize;
        let namesize = Self::parse_hex(&header[94..102])? as usize;

        // Get name
        let name_start = self.offset + 110;
        let name_end = name_start + namesize - 1; // Exclude null terminator
        if name_end > self.data.len() {
            return None;
        }
        let name = str::from_utf8(&self.data[name_start..name_end]).ok()?;

        // Check for trailer
        if name == "TRAILER!!!" {
            return None;
        }

        // Get data
        let data_start = Self::align4(name_start + namesize);
        let data_end = data_start + filesize;
        if data_end > self.data.len() {
            return None;
        }
        let data = &self.data[data_start..data_end];

        // Move to next entry
        self.offset = Self::align4(data_end);

        // SAFETY: CPIO format uses 32-bit file sizes, so this is expected to fit
        #[allow(clippy::cast_possible_truncation)]
        let size_truncated = filesize as u32;
        Some(CpioEntry {
            name,
            mode,
            size: size_truncated,
            data,
        })
    }
}

/// Parse CPIO initramfs
pub fn parse_initramfs(file: &File) -> Option<CpioIterator<'_>> {
    let data = file.as_bytes();

    // Check for GZIP compression
    if data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b {
        // Would need decompression - not supported in no_std without additional crate
        return None;
    }

    // Check for CPIO magic
    if data.len() >= 6 && (&data[0..6] == b"070701" || &data[0..6] == b"070702") {
        return Some(CpioIterator::new(data));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_path() {
        let path = FilePath::new("/boot/kernel/vmlinux.elf");
        assert_eq!(path.file_name(), "vmlinux.elf");
        assert_eq!(path.file_stem(), "vmlinux");
        assert_eq!(path.extension(), Some("elf"));
        assert_eq!(path.parent(), Some("/boot/kernel"));
        assert!(path.is_absolute());
    }

    #[test]
    fn test_path_buf() {
        let mut buf = PathBuf::new();
        buf.push("/boot");
        buf.push("kernel");
        buf.push("vmlinux");
        assert_eq!(buf.as_str(), "/boot/kernel/vmlinux");

        assert!(buf.pop());
        assert_eq!(buf.as_str(), "/boot/kernel");
    }

    #[test]
    fn test_file_type_detection() {
        assert_eq!(FileType::detect(&[0x7f, b'E', b'L', b'F']), FileType::Elf);
        assert_eq!(FileType::detect(&[b'M', b'Z']), FileType::Pe);
        assert_eq!(FileType::detect(&[0x1f, 0x8b]), FileType::Gzip);
    }

    #[test]
    fn test_media_type() {
        assert_eq!(MediaType::from_raw(0), MediaType::Generic);
        assert_eq!(MediaType::from_raw(1), MediaType::Optical);
        assert_eq!(MediaType::from_raw(2), MediaType::Tftp);
        assert_eq!(MediaType::from_raw(99), MediaType::Generic);
    }
}
