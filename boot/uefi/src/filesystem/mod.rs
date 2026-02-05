//! UEFI File System Support
//!
//! FAT file system access for boot file loading.

use core::fmt;

// =============================================================================
// FILE SYSTEM PROTOCOL GUIDs
// =============================================================================

/// EFI Simple File System Protocol GUID
pub const EFI_SIMPLE_FILE_SYSTEM_PROTOCOL_GUID: [u8; 16] = [
    0x22, 0x5B, 0x4E, 0x96, 0x59, 0x64, 0xD2, 0x11, 0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B,
];

/// EFI File Info GUID
pub const EFI_FILE_INFO_GUID: [u8; 16] = [
    0xDD, 0x8B, 0xA6, 0x09, 0x60, 0x9A, 0xD3, 0x11, 0x8A, 0x39, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
];

/// EFI File System Info GUID
pub const EFI_FILE_SYSTEM_INFO_GUID: [u8; 16] = [
    0xDD, 0x8B, 0xA6, 0x09, 0x60, 0x9A, 0xD3, 0x11, 0x8A, 0x39, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4E,
];

/// EFI File System Volume Label GUID
pub const EFI_FILE_SYSTEM_VOLUME_LABEL_GUID: [u8; 16] = [
    0x7A, 0x3E, 0xA5, 0xDB, 0x00, 0x20, 0xD3, 0x11, 0x9A, 0x2D, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
];

// =============================================================================
// FILE ATTRIBUTES
// =============================================================================

/// File attributes
pub mod attrs {
    /// Read-only
    pub const READ_ONLY: u64 = 0x0000000000000001;
    /// Hidden
    pub const HIDDEN: u64 = 0x0000000000000002;
    /// System file
    pub const SYSTEM: u64 = 0x0000000000000004;
    /// Reserved
    pub const RESERVED: u64 = 0x0000000000000008;
    /// Directory
    pub const DIRECTORY: u64 = 0x0000000000000010;
    /// Archive
    pub const ARCHIVE: u64 = 0x0000000000000020;
    /// Valid attributes mask
    pub const VALID_ATTR: u64 = 0x0000000000000037;
}

/// File open modes
pub mod modes {
    /// Read mode
    pub const READ: u64 = 0x0000000000000001;
    /// Write mode
    pub const WRITE: u64 = 0x0000000000000002;
    /// Create mode
    pub const CREATE: u64 = 0x8000000000000000;
}

// =============================================================================
// FILE INFO STRUCTURES
// =============================================================================

/// File info header (fixed part)
#[repr(C)]
pub struct FileInfoHeader {
    /// Size of structure including filename
    pub size: u64,
    /// File size in bytes
    pub file_size: u64,
    /// Physical size on disk
    pub physical_size: u64,
    /// Creation time
    pub create_time: EfiTime,
    /// Last access time
    pub last_access_time: EfiTime,
    /// Modification time
    pub modification_time: EfiTime,
    /// File attributes
    pub attribute: u64,
    // Followed by null-terminated filename in UCS-2
}

/// File info with inline name
pub struct FileInfo {
    /// Size
    pub size: u64,
    /// File size
    pub file_size: u64,
    /// Physical size
    pub physical_size: u64,
    /// Create time
    pub create_time: EfiTime,
    /// Last access time
    pub last_access_time: EfiTime,
    /// Modification time
    pub modification_time: EfiTime,
    /// Attributes
    pub attribute: u64,
    /// Filename (UCS-2)
    pub filename: [u16; 256],
    /// Filename length
    pub filename_len: usize,
}

impl FileInfo {
    /// Create empty file info
    pub fn new() -> Self {
        Self {
            size: 0,
            file_size: 0,
            physical_size: 0,
            create_time: EfiTime::default(),
            last_access_time: EfiTime::default(),
            modification_time: EfiTime::default(),
            attribute: 0,
            filename: [0; 256],
            filename_len: 0,
        }
    }

    /// Parse from buffer
    pub fn from_buffer(buffer: &[u8]) -> Option<Self> {
        if buffer.len() < 80 {
            return None;
        }

        let mut info = Self::new();

        info.size = u64::from_le_bytes(buffer[0..8].try_into().ok()?);
        info.file_size = u64::from_le_bytes(buffer[8..16].try_into().ok()?);
        info.physical_size = u64::from_le_bytes(buffer[16..24].try_into().ok()?);
        info.create_time = EfiTime::from_bytes(&buffer[24..40])?;
        info.last_access_time = EfiTime::from_bytes(&buffer[40..56])?;
        info.modification_time = EfiTime::from_bytes(&buffer[56..72])?;
        info.attribute = u64::from_le_bytes(buffer[72..80].try_into().ok()?);

        // Parse filename (UCS-2)
        let mut pos = 80;
        let mut name_len = 0;

        while pos + 1 < buffer.len() && name_len < 255 {
            let c = u16::from_le_bytes([buffer[pos], buffer[pos + 1]]);
            if c == 0 {
                break;
            }
            info.filename[name_len] = c;
            name_len += 1;
            pos += 2;
        }
        info.filename_len = name_len;

        Some(info)
    }

    /// Is directory
    pub fn is_directory(&self) -> bool {
        self.attribute & attrs::DIRECTORY != 0
    }

    /// Is read-only
    pub fn is_read_only(&self) -> bool {
        self.attribute & attrs::READ_ONLY != 0
    }

    /// Is hidden
    pub fn is_hidden(&self) -> bool {
        self.attribute & attrs::HIDDEN != 0
    }

    /// Is system file
    pub fn is_system(&self) -> bool {
        self.attribute & attrs::SYSTEM != 0
    }

    /// Get filename as UTF-8
    pub fn filename_str(&self, buffer: &mut [u8]) -> usize {
        let mut pos = 0;

        for i in 0..self.filename_len {
            if pos >= buffer.len() {
                break;
            }

            let c = self.filename[i];
            if c < 128 {
                buffer[pos] = c as u8;
                pos += 1;
            } else {
                // UTF-8 encode
                if c < 0x800 && pos + 1 < buffer.len() {
                    buffer[pos] = 0xC0 | ((c >> 6) as u8);
                    buffer[pos + 1] = 0x80 | ((c & 0x3F) as u8);
                    pos += 2;
                }
            }
        }

        pos
    }
}

impl Default for FileInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// File system info
pub struct FileSystemInfo {
    /// Total size
    pub size: u64,
    /// Read-only flag
    pub read_only: bool,
    /// Volume size
    pub volume_size: u64,
    /// Free space
    pub free_space: u64,
    /// Block size
    pub block_size: u32,
    /// Volume label
    pub volume_label: [u16; 64],
    /// Label length
    pub label_len: usize,
}

impl FileSystemInfo {
    /// Create empty
    pub fn new() -> Self {
        Self {
            size: 0,
            read_only: false,
            volume_size: 0,
            free_space: 0,
            block_size: 512,
            volume_label: [0; 64],
            label_len: 0,
        }
    }

    /// Get volume label as string
    pub fn volume_label_str(&self, buffer: &mut [u8]) -> usize {
        let mut pos = 0;

        for i in 0..self.label_len {
            if pos >= buffer.len() {
                break;
            }
            let c = self.volume_label[i];
            if c < 128 {
                buffer[pos] = c as u8;
                pos += 1;
            }
        }

        pos
    }
}

impl Default for FileSystemInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// EFI time structure
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
    /// Size in bytes
    pub const SIZE: usize = 16;

    /// Create from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            year: u16::from_le_bytes([bytes[0], bytes[1]]),
            month: bytes[2],
            day: bytes[3],
            hour: bytes[4],
            minute: bytes[5],
            second: bytes[6],
            pad1: bytes[7],
            nanosecond: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            timezone: i16::from_le_bytes([bytes[12], bytes[13]]),
            daylight: bytes[14],
            pad2: bytes[15],
        })
    }

    /// To bytes
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[0..2].copy_from_slice(&self.year.to_le_bytes());
        bytes[2] = self.month;
        bytes[3] = self.day;
        bytes[4] = self.hour;
        bytes[5] = self.minute;
        bytes[6] = self.second;
        bytes[7] = self.pad1;
        bytes[8..12].copy_from_slice(&self.nanosecond.to_le_bytes());
        bytes[12..14].copy_from_slice(&self.timezone.to_le_bytes());
        bytes[14] = self.daylight;
        bytes[15] = self.pad2;
        bytes
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
    }
}

impl fmt::Display for EfiTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

// =============================================================================
// FILE HANDLE
// =============================================================================

/// File handle
pub struct FileHandle {
    /// Handle pointer
    _handle: usize,
    /// Current position
    position: u64,
    /// File size (cached)
    size: u64,
    /// Is directory
    is_directory: bool,
    /// Is open
    is_open: bool,
}

impl FileHandle {
    /// Create new file handle
    pub fn new(handle: usize) -> Self {
        Self {
            _handle: handle,
            position: 0,
            size: 0,
            is_directory: false,
            is_open: true,
        }
    }

    /// Read from file
    pub fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, FileError> {
        if !self.is_open {
            return Err(FileError::NotOpen);
        }

        if self.is_directory {
            return Err(FileError::IsDirectory);
        }

        // Would call EFI_FILE_PROTOCOL.Read()
        // Updates self.position

        Ok(0)
    }

    /// Write to file
    pub fn write(&mut self, _buffer: &[u8]) -> Result<usize, FileError> {
        if !self.is_open {
            return Err(FileError::NotOpen);
        }

        if self.is_directory {
            return Err(FileError::IsDirectory);
        }

        // Would call EFI_FILE_PROTOCOL.Write()

        Ok(0)
    }

    /// Seek to position
    pub fn seek(&mut self, position: SeekFrom) -> Result<u64, FileError> {
        if !self.is_open {
            return Err(FileError::NotOpen);
        }

        let new_pos = match position {
            SeekFrom::Start(pos) => pos,
            SeekFrom::Current(offset) => {
                if offset >= 0 {
                    self.position.saturating_add(offset as u64)
                } else {
                    self.position.saturating_sub((-offset) as u64)
                }
            },
            SeekFrom::End(offset) => {
                if offset >= 0 {
                    self.size.saturating_add(offset as u64)
                } else {
                    self.size.saturating_sub((-offset) as u64)
                }
            },
        };

        self.position = new_pos;

        // Would call EFI_FILE_PROTOCOL.SetPosition()

        Ok(self.position)
    }

    /// Get current position
    pub fn position(&self) -> u64 {
        self.position
    }

    /// Get file size
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Flush file
    pub fn flush(&mut self) -> Result<(), FileError> {
        if !self.is_open {
            return Err(FileError::NotOpen);
        }

        // Would call EFI_FILE_PROTOCOL.Flush()

        Ok(())
    }

    /// Close file
    pub fn close(&mut self) -> Result<(), FileError> {
        if !self.is_open {
            return Ok(());
        }

        // Would call EFI_FILE_PROTOCOL.Close()

        self.is_open = false;
        Ok(())
    }

    /// Delete file
    pub fn delete(mut self) -> Result<(), FileError> {
        if !self.is_open {
            return Err(FileError::NotOpen);
        }

        // Would call EFI_FILE_PROTOCOL.Delete()

        self.is_open = false;
        Ok(())
    }

    /// Get file info
    pub fn get_info(&self) -> Result<FileInfo, FileError> {
        if !self.is_open {
            return Err(FileError::NotOpen);
        }

        // Would call EFI_FILE_PROTOCOL.GetInfo()

        Ok(FileInfo::new())
    }

    /// Set file info
    pub fn set_info(&mut self, _info: &FileInfo) -> Result<(), FileError> {
        if !self.is_open {
            return Err(FileError::NotOpen);
        }

        // Would call EFI_FILE_PROTOCOL.SetInfo()

        Ok(())
    }

    /// Read directory entry
    pub fn read_dir(&mut self) -> Result<Option<FileInfo>, FileError> {
        if !self.is_open {
            return Err(FileError::NotOpen);
        }

        if !self.is_directory {
            return Err(FileError::NotDirectory);
        }

        // Would call EFI_FILE_PROTOCOL.Read() with buffer
        // Returns None when no more entries

        Ok(None)
    }

    /// Rewind directory
    pub fn rewind(&mut self) -> Result<(), FileError> {
        self.seek(SeekFrom::Start(0))?;
        Ok(())
    }
}

impl Drop for FileHandle {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

/// Seek position
#[derive(Debug, Clone, Copy)]
pub enum SeekFrom {
    /// From start
    Start(u64),
    /// From current position
    Current(i64),
    /// From end
    End(i64),
}

// =============================================================================
// FILE SYSTEM
// =============================================================================

/// File system handle
pub struct FileSystem {
    /// Protocol handle
    _handle: usize,
    /// Root directory handle
    root: Option<FileHandle>,
    /// Volume label
    volume_label: [u8; 64],
    /// Label length
    label_len: usize,
}

impl FileSystem {
    /// Create new file system
    pub fn new(handle: usize) -> Self {
        Self {
            _handle: handle,
            root: None,
            volume_label: [0; 64],
            label_len: 0,
        }
    }

    /// Open volume (root directory)
    pub fn open_volume(&mut self) -> Result<&mut FileHandle, FileError> {
        if self.root.is_some() {
            return Ok(self.root.as_mut().unwrap());
        }

        // Would call EFI_SIMPLE_FILE_SYSTEM_PROTOCOL.OpenVolume()

        self.root = Some(FileHandle {
            _handle: 0, // Would be set by protocol call
            position: 0,
            size: 0,
            is_directory: true,
            is_open: true,
        });

        Ok(self.root.as_mut().unwrap())
    }

    /// Open file
    pub fn open(&mut self, path: &str, _mode: u64, _attrs: u64) -> Result<FileHandle, FileError> {
        let _root = self.open_volume()?;

        // Convert path to UCS-2
        let mut path_ucs2 = [0u16; 256];
        let _path_len = path_to_ucs2(path, &mut path_ucs2);

        // Would call root.handle.Open()

        Ok(FileHandle::new(0))
    }

    /// Open file for reading
    pub fn open_read(&mut self, path: &str) -> Result<FileHandle, FileError> {
        self.open(path, modes::READ, 0)
    }

    /// Open file for writing (create if not exists)
    pub fn open_write(&mut self, path: &str) -> Result<FileHandle, FileError> {
        self.open(path, modes::READ | modes::WRITE | modes::CREATE, 0)
    }

    /// Check if file exists
    pub fn exists(&mut self, path: &str) -> bool {
        self.open_read(path).is_ok()
    }

    /// Get file size
    pub fn file_size(&mut self, path: &str) -> Result<u64, FileError> {
        let file = self.open_read(path)?;
        Ok(file.size())
    }

    /// Read entire file
    pub fn read_file(&mut self, path: &str, buffer: &mut [u8]) -> Result<usize, FileError> {
        let mut file = self.open_read(path)?;
        let size = file.size() as usize;

        if size > buffer.len() {
            return Err(FileError::BufferTooSmall);
        }

        file.read(&mut buffer[..size])
    }

    /// Write entire file
    pub fn write_file(&mut self, path: &str, data: &[u8]) -> Result<(), FileError> {
        let mut file = self.open_write(path)?;
        file.write(data)?;
        file.flush()?;
        Ok(())
    }

    /// Create directory
    pub fn create_directory(&mut self, path: &str) -> Result<FileHandle, FileError> {
        self.open(
            path,
            modes::READ | modes::WRITE | modes::CREATE,
            attrs::DIRECTORY,
        )
    }

    /// Delete file or directory
    pub fn delete(&mut self, path: &str) -> Result<(), FileError> {
        let file = self.open(path, modes::READ | modes::WRITE, 0)?;
        file.delete()
    }

    /// List directory
    pub fn read_dir(&mut self, path: &str) -> Result<DirectoryIterator, FileError> {
        let dir = self.open_read(path)?;

        if !dir.is_directory {
            return Err(FileError::NotDirectory);
        }

        Ok(DirectoryIterator { dir })
    }

    /// Get volume info
    pub fn volume_info(&mut self) -> Result<FileSystemInfo, FileError> {
        // Would call GetInfo with EFI_FILE_SYSTEM_INFO_GUID
        Ok(FileSystemInfo::new())
    }

    /// Get volume label
    pub fn volume_label(&self) -> &str {
        core::str::from_utf8(&self.volume_label[..self.label_len]).unwrap_or("")
    }
}

/// Directory iterator
pub struct DirectoryIterator {
    dir: FileHandle,
}

impl Iterator for DirectoryIterator {
    type Item = Result<FileInfo, FileError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.dir.read_dir() {
            Ok(Some(info)) => Some(Ok(info)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

// =============================================================================
// PATH UTILITIES
// =============================================================================

/// Path component iterator
pub struct PathComponents<'a> {
    path: &'a str,
    pos: usize,
}

impl<'a> PathComponents<'a> {
    /// Create new iterator
    pub fn new(path: &'a str) -> Self {
        // Skip leading separator
        let start = if path.starts_with('\\') || path.starts_with('/') {
            1
        } else {
            0
        };
        Self { path, pos: start }
    }
}

impl<'a> Iterator for PathComponents<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.path.len() {
            return None;
        }

        let remaining = &self.path[self.pos..];

        // Find next separator
        let end = remaining
            .find(|c| c == '\\' || c == '/')
            .unwrap_or(remaining.len());

        if end == 0 {
            self.pos += 1;
            return self.next();
        }

        let component = &remaining[..end];
        self.pos += end + 1;

        Some(component)
    }
}

/// Path builder
pub struct PathBuilder {
    buffer: [u8; 512],
    len: usize,
}

impl PathBuilder {
    /// Create new path builder
    pub fn new() -> Self {
        Self {
            buffer: [0; 512],
            len: 0,
        }
    }

    /// Start from root
    pub fn root() -> Self {
        let mut pb = Self::new();
        pb.buffer[0] = b'\\';
        pb.len = 1;
        pb
    }

    /// Push component
    pub fn push(&mut self, component: &str) -> &mut Self {
        if self.len > 0 && self.buffer[self.len - 1] != b'\\' {
            if self.len < self.buffer.len() {
                self.buffer[self.len] = b'\\';
                self.len += 1;
            }
        }

        let bytes = component.as_bytes();
        let copy_len = bytes.len().min(self.buffer.len() - self.len);
        self.buffer[self.len..self.len + copy_len].copy_from_slice(&bytes[..copy_len]);
        self.len += copy_len;

        self
    }

    /// Pop last component
    pub fn pop(&mut self) -> &mut Self {
        // Find last separator
        for i in (0..self.len).rev() {
            if self.buffer[i] == b'\\' {
                self.len = i;
                if self.len == 0 {
                    self.buffer[0] = b'\\';
                    self.len = 1;
                }
                break;
            }
        }
        self
    }

    /// Get path as str
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buffer[..self.len]).unwrap_or("")
    }

    /// Get path length
    pub fn len(&self) -> usize {
        self.len
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Default for PathBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert path to UCS-2
fn path_to_ucs2(path: &str, buffer: &mut [u16]) -> usize {
    let mut pos = 0;

    for c in path.chars() {
        if pos >= buffer.len() - 1 {
            break;
        }

        // Convert backslash/forward slash
        let c = if c == '/' { '\\' } else { c };

        buffer[pos] = c as u16;
        pos += 1;
    }

    buffer[pos] = 0; // Null terminator
    pos + 1
}

/// Convert UCS-2 to path
fn _ucs2_to_path(ucs2: &[u16], buffer: &mut [u8]) -> usize {
    let mut pos = 0;

    for &c in ucs2 {
        if c == 0 || pos >= buffer.len() {
            break;
        }

        if c < 128 {
            buffer[pos] = c as u8;
            pos += 1;
        }
    }

    pos
}

// =============================================================================
// KERNEL LOADER
// =============================================================================

/// Kernel file loader
pub struct KernelLoader<'a> {
    fs: &'a mut FileSystem,
}

impl<'a> KernelLoader<'a> {
    /// Create new kernel loader
    pub fn new(fs: &'a mut FileSystem) -> Self {
        Self { fs }
    }

    /// Load kernel file
    pub fn load_kernel(&mut self, path: &str, buffer: &mut [u8]) -> Result<KernelInfo, FileError> {
        let mut file = self.fs.open_read(path)?;
        let size = file.size() as usize;

        if size > buffer.len() {
            return Err(FileError::BufferTooSmall);
        }

        let bytes_read = file.read(&mut buffer[..size])?;

        // Detect kernel type
        let kernel_type = self.detect_kernel_type(&buffer[..bytes_read])?;

        Ok(KernelInfo {
            size: bytes_read,
            kernel_type,
            entry_point: 0, // Would be parsed from headers
            load_address: 0,
        })
    }

    /// Detect kernel type from magic
    fn detect_kernel_type(&self, data: &[u8]) -> Result<KernelType, FileError> {
        if data.len() < 4 {
            return Err(FileError::InvalidFormat);
        }

        // ELF magic
        if &data[0..4] == b"\x7FELF" {
            return Ok(KernelType::Elf);
        }

        // PE magic (MZ)
        if &data[0..2] == b"MZ" {
            // Check for PE signature
            if data.len() >= 64 {
                let pe_offset =
                    u32::from_le_bytes(data[60..64].try_into().unwrap_or([0; 4])) as usize;

                if pe_offset + 4 <= data.len() && &data[pe_offset..pe_offset + 4] == b"PE\x00\x00" {
                    return Ok(KernelType::Pe);
                }
            }
            return Ok(KernelType::Pe);
        }

        // Linux boot protocol
        if data.len() >= 512 && &data[510..512] == &[0x55, 0xAA] {
            return Ok(KernelType::LinuxBoot);
        }

        // Multiboot
        if data.len() >= 8192 {
            for i in (0..8188).step_by(4) {
                let magic = u32::from_le_bytes(data[i..i + 4].try_into().unwrap_or([0; 4]));
                if magic == 0x1BADB002 {
                    return Ok(KernelType::Multiboot);
                }
                if magic == 0xE85250D6 {
                    return Ok(KernelType::Multiboot2);
                }
            }
        }

        Ok(KernelType::Raw)
    }

    /// Load initrd/initramfs
    pub fn load_initrd(&mut self, path: &str, buffer: &mut [u8]) -> Result<usize, FileError> {
        self.fs.read_file(path, buffer)
    }
}

/// Kernel type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelType {
    /// ELF executable
    Elf,
    /// PE executable
    Pe,
    /// Linux boot protocol
    LinuxBoot,
    /// Multiboot 1
    Multiboot,
    /// Multiboot 2
    Multiboot2,
    /// Raw binary
    Raw,
}

/// Kernel info
#[derive(Debug, Clone)]
pub struct KernelInfo {
    /// File size
    pub size: usize,
    /// Kernel type
    pub kernel_type: KernelType,
    /// Entry point address
    pub entry_point: u64,
    /// Load address
    pub load_address: u64,
}

// =============================================================================
// FILE ERROR
// =============================================================================

/// File error
#[derive(Debug, Clone)]
pub enum FileError {
    /// File not found
    NotFound,
    /// Not open
    NotOpen,
    /// Is directory
    IsDirectory,
    /// Not directory
    NotDirectory,
    /// Permission denied
    PermissionDenied,
    /// File exists
    AlreadyExists,
    /// Invalid path
    InvalidPath,
    /// Invalid format
    InvalidFormat,
    /// Buffer too small
    BufferTooSmall,
    /// End of file
    EndOfFile,
    /// Volume full
    VolumeFull,
    /// Read-only file system
    ReadOnly,
    /// Device error
    DeviceError,
    /// Invalid parameter
    InvalidParameter,
    /// Not supported
    NotSupported,
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "file not found"),
            Self::NotOpen => write!(f, "file not open"),
            Self::IsDirectory => write!(f, "is a directory"),
            Self::NotDirectory => write!(f, "not a directory"),
            Self::PermissionDenied => write!(f, "permission denied"),
            Self::AlreadyExists => write!(f, "file already exists"),
            Self::InvalidPath => write!(f, "invalid path"),
            Self::InvalidFormat => write!(f, "invalid format"),
            Self::BufferTooSmall => write!(f, "buffer too small"),
            Self::EndOfFile => write!(f, "end of file"),
            Self::VolumeFull => write!(f, "volume full"),
            Self::ReadOnly => write!(f, "read-only file system"),
            Self::DeviceError => write!(f, "device error"),
            Self::InvalidParameter => write!(f, "invalid parameter"),
            Self::NotSupported => write!(f, "not supported"),
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
    fn test_path_components() {
        let path = "\\EFI\\BOOT\\BOOTX64.EFI";
        let components: Vec<_> = PathComponents::new(path).collect();

        assert_eq!(components, vec!["EFI", "BOOT", "BOOTX64.EFI"]);
    }

    #[test]
    fn test_path_builder() {
        let mut pb = PathBuilder::root();
        pb.push("EFI").push("BOOT").push("BOOTX64.EFI");

        assert_eq!(pb.as_str(), "\\EFI\\BOOT\\BOOTX64.EFI");

        pb.pop();
        assert_eq!(pb.as_str(), "\\EFI\\BOOT");
    }

    #[test]
    fn test_efi_time() {
        let time = EfiTime {
            year: 2024,
            month: 6,
            day: 15,
            hour: 10,
            minute: 30,
            second: 45,
            ..Default::default()
        };

        assert!(time.is_valid());
    }
}
