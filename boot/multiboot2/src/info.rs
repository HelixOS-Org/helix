//! # Boot Information Parsing
//!
//! This module provides the core parsing functionality for Multiboot2
//! boot information structures.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                    MULTIBOOT2 INFORMATION STRUCTURE                     │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌─────────────┐                                                        │
//! │  │ total_size  │ ← u32: Total size of boot info including this field   │
//! │  │ reserved    │ ← u32: Must be 0                                       │
//! │  ├─────────────┤                                                        │
//! │  │   Tag 0     │ ← 8-byte aligned                                       │
//! │  │   (type,    │                                                        │
//! │  │    size,    │                                                        │
//! │  │    data)    │                                                        │
//! │  ├─────────────┤ ← Padding to 8-byte alignment                          │
//! │  │   Tag 1     │                                                        │
//! │  │   ...       │                                                        │
//! │  ├─────────────┤                                                        │
//! │  │   ...       │                                                        │
//! │  ├─────────────┤                                                        │
//! │  │  End Tag    │ ← type=0, size=8                                       │
//! │  └─────────────┘                                                        │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

use core::marker::PhantomData;
use core::{fmt, str};

use crate::memory::MemoryMap;
use crate::validate::{validate_header, validate_pointer, validate_tag, ValidationResult};
use crate::{align_up, tag_types, TAG_ALIGNMENT};

// =============================================================================
// Multiboot2 Information Structure
// =============================================================================

/// Parsed Multiboot2 boot information
///
/// This structure provides safe, typed access to the boot information
/// passed by a Multiboot2-compliant bootloader.
///
/// # Lifetime
///
/// The `'boot` lifetime ensures that all parsed data remains valid.
/// References obtained from this structure cannot outlive the boot info.
///
/// # Safety
///
/// Creating this structure requires a single `unsafe` call to `from_ptr`.
/// After creation, all operations are safe.
///
/// # Example
///
/// ```rust,no_run
/// use helix_multiboot2::info::Multiboot2Info;
///
/// fn kernel_main(magic: u32, info_ptr: *const u8) {
///     // Single unsafe boundary
///     let boot_info = unsafe {
///         Multiboot2Info::from_ptr(info_ptr).expect("Invalid boot info")
///     };
///
///     // Everything else is safe!
///     println!("Total size: {} bytes", boot_info.total_size());
///
///     for tag in boot_info.tags() {
///         // Process typed tags...
///     }
/// }
/// ```
pub struct Multiboot2Info<'boot> {
    /// Base pointer to the boot information
    ptr: *const u8,
    /// Total size of the boot information
    total_size: u32,
    /// Phantom data for lifetime
    _marker: PhantomData<&'boot [u8]>,
}

impl<'boot> Multiboot2Info<'boot> {
    /// Create a new Multiboot2Info from a raw pointer
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - `ptr` points to valid Multiboot2 boot information
    /// - The memory remains valid for lifetime `'boot`
    /// - The memory is not modified during the lifetime
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The pointer is null
    /// - The pointer is not 8-byte aligned
    /// - The total size is invalid
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use helix_multiboot2::info::Multiboot2Info;
    ///
    /// // Pointer received from bootloader (in EDI/RDI register)
    /// let ptr: *const u8 = 0x12345678 as *const u8;
    ///
    /// let boot_info = unsafe { Multiboot2Info::from_ptr(ptr) };
    /// ```
    pub unsafe fn from_ptr(ptr: *const u8) -> ValidationResult<Self> {
        // Validate pointer
        validate_pointer(ptr)?;

        // Read total size (first u32)
        // Safety: We validated the pointer is non-null and aligned
        let total_size = unsafe { *(ptr as *const u32) };

        // Validate header
        validate_header(total_size)?;

        Ok(Self {
            ptr,
            total_size,
            _marker: PhantomData,
        })
    }

    /// Get the total size of the boot information in bytes
    #[must_use]
    pub const fn total_size(&self) -> u32 {
        self.total_size
    }

    /// Get the raw pointer to the boot information
    ///
    /// This is mainly useful for debugging purposes.
    #[must_use]
    pub const fn as_ptr(&self) -> *const u8 {
        self.ptr
    }

    /// Get an iterator over all tags
    pub fn tags(&self) -> TagIterator<'boot> {
        TagIterator::new(self.ptr, self.total_size)
    }

    /// Find a specific tag by type
    pub fn find_tag(&self, tag_type: u32) -> Option<Tag<'boot>> {
        self.tags().find(|t| t.tag_type() == tag_type)
    }

    /// Get the command line, if present
    pub fn cmdline(&self) -> Option<&'boot str> {
        self.find_tag(tag_types::CMDLINE).and_then(|t| {
            if let Tag::Cmdline(c) = t {
                Some(c.as_str())
            } else {
                None
            }
        })
    }

    /// Get the bootloader name, if present
    pub fn bootloader_name(&self) -> Option<&'boot str> {
        self.find_tag(tag_types::BOOTLOADER_NAME).and_then(|t| {
            if let Tag::BootloaderName(n) = t {
                Some(n.as_str())
            } else {
                None
            }
        })
    }

    /// Get the memory map, if present
    pub fn memory_map(&self) -> Option<MemoryMap<'boot>> {
        self.find_tag(tag_types::MEMORY_MAP).and_then(|t| {
            if let Tag::MemoryMap(m) = t {
                Some(m)
            } else {
                None
            }
        })
    }

    /// Get basic memory information (legacy), if present
    pub fn basic_memory_info(&self) -> Option<BasicMemoryInfo> {
        self.find_tag(tag_types::BASIC_MEMINFO).and_then(|t| {
            if let Tag::BasicMemInfo(m) = t {
                Some(m)
            } else {
                None
            }
        })
    }

    /// Get framebuffer information, if present
    #[cfg(feature = "framebuffer")]
    pub fn framebuffer(&self) -> Option<FramebufferInfo<'boot>> {
        self.find_tag(tag_types::FRAMEBUFFER).and_then(|t| {
            if let Tag::Framebuffer(fb) = t {
                Some(fb)
            } else {
                None
            }
        })
    }

    /// Get all boot modules
    pub fn modules(&self) -> impl Iterator<Item = BootModule<'boot>> {
        self.tags().filter_map(|t| {
            if let Tag::Module(m) = t {
                Some(m)
            } else {
                None
            }
        })
    }

    /// Get ACPI RSDP (old), if present
    #[cfg(feature = "acpi")]
    pub fn acpi_old_rsdp(&self) -> Option<&'boot [u8]> {
        self.find_tag(tag_types::ACPI_OLD).and_then(|t| {
            if let Tag::AcpiOld(data) = t {
                Some(data)
            } else {
                None
            }
        })
    }

    /// Get ACPI RSDP (new/v2), if present
    #[cfg(feature = "acpi")]
    pub fn acpi_new_rsdp(&self) -> Option<&'boot [u8]> {
        self.find_tag(tag_types::ACPI_NEW).and_then(|t| {
            if let Tag::AcpiNew(data) = t {
                Some(data)
            } else {
                None
            }
        })
    }

    /// Get ELF sections, if present
    #[cfg(feature = "elf_sections")]
    pub fn elf_sections(&self) -> Option<ElfSections<'boot>> {
        self.find_tag(tag_types::ELF_SECTIONS).and_then(|t| {
            if let Tag::ElfSections(s) = t {
                Some(s)
            } else {
                None
            }
        })
    }

    /// Get image load base address, if present
    pub fn load_base_addr(&self) -> Option<u64> {
        self.find_tag(tag_types::LOAD_BASE_ADDR).and_then(|t| {
            if let Tag::LoadBaseAddr(addr) = t {
                Some(addr)
            } else {
                None
            }
        })
    }
}

impl fmt::Debug for Multiboot2Info<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Multiboot2Info")
            .field("ptr", &format_args!("{:p}", self.ptr))
            .field("total_size", &self.total_size)
            .field("cmdline", &self.cmdline())
            .field("bootloader", &self.bootloader_name())
            .finish()
    }
}

// Safety: The boot info is read-only
unsafe impl Send for Multiboot2Info<'_> {}
unsafe impl Sync for Multiboot2Info<'_> {}

// =============================================================================
// Tag Iterator
// =============================================================================

/// Iterator over Multiboot2 tags
///
/// This iterator provides zero-copy access to tags within the boot information.
/// Tags are parsed on-demand and returned as typed `Tag` variants.
pub struct TagIterator<'boot> {
    /// Base pointer
    base: *const u8,
    /// Current offset (starts at 8 to skip header)
    offset: usize,
    /// Total size
    total_size: u32,
    /// Marker
    _marker: PhantomData<&'boot [u8]>,
}

impl<'boot> TagIterator<'boot> {
    /// Create a new tag iterator
    fn new(base: *const u8, total_size: u32) -> Self {
        Self {
            base,
            offset: 8, // Skip total_size and reserved fields
            total_size,
            _marker: PhantomData,
        }
    }
}

impl<'boot> Iterator for TagIterator<'boot> {
    type Item = Tag<'boot>;

    fn next(&mut self) -> Option<Self::Item> {
        // Check bounds
        if self.offset + 8 > self.total_size as usize {
            return None;
        }

        // Read tag header
        // Safety: We checked bounds and the original creation validated alignment
        let tag_ptr = unsafe { self.base.add(self.offset) };
        let tag_type = unsafe { *(tag_ptr as *const u32) };
        let tag_size = unsafe { *(tag_ptr.add(4) as *const u32) };

        // Validate tag
        if validate_tag(tag_type, tag_size, self.offset, self.total_size).is_err() {
            return None;
        }

        // End tag terminates iteration
        if tag_type == tag_types::END {
            return None;
        }

        // Get tag data slice
        let data_ptr = unsafe { tag_ptr.add(8) };
        let data_len = tag_size.saturating_sub(8) as usize;
        let data = unsafe { core::slice::from_raw_parts(data_ptr, data_len) };

        // Advance to next tag (aligned to 8 bytes)
        self.offset = align_up(self.offset + tag_size as usize, TAG_ALIGNMENT);

        // Parse and return typed tag
        Some(Tag::parse(tag_type, tag_size, data))
    }
}

// =============================================================================
// Tag Enum
// =============================================================================

/// Typed representation of a Multiboot2 tag
///
/// This enum provides type-safe access to tag data. Each variant contains
/// the parsed representation of that tag type.
#[derive(Clone)]
#[non_exhaustive]
pub enum Tag<'boot> {
    /// Boot command line
    Cmdline(CmdlineTag<'boot>),

    /// Bootloader name
    BootloaderName(BootloaderNameTag<'boot>),

    /// Boot module
    Module(BootModule<'boot>),

    /// Basic memory information
    BasicMemInfo(BasicMemoryInfo),

    /// BIOS boot device
    BootDevice(BootDevice),

    /// Memory map
    MemoryMap(MemoryMap<'boot>),

    /// Framebuffer information
    #[cfg(feature = "framebuffer")]
    Framebuffer(FramebufferInfo<'boot>),

    /// ELF sections
    #[cfg(feature = "elf_sections")]
    ElfSections(ElfSections<'boot>),

    /// ACPI old RSDP
    #[cfg(feature = "acpi")]
    AcpiOld(&'boot [u8]),

    /// ACPI new RSDP
    #[cfg(feature = "acpi")]
    AcpiNew(&'boot [u8]),

    /// Image load base address
    LoadBaseAddr(u64),

    /// EFI 64-bit system table
    Efi64SystemTable(u64),

    /// Unknown or unhandled tag type
    Unknown {
        /// Tag type
        tag_type: u32,
        /// Tag size
        size: u32,
        /// Raw tag data
        data: &'boot [u8],
    },
}

impl<'boot> Tag<'boot> {
    /// Parse a tag from raw data
    fn parse(tag_type: u32, size: u32, data: &'boot [u8]) -> Self {
        match tag_type {
            tag_types::CMDLINE => Self::Cmdline(CmdlineTag::from_data(data)),

            tag_types::BOOTLOADER_NAME => Self::BootloaderName(BootloaderNameTag::from_data(data)),

            tag_types::MODULE => Self::Module(BootModule::from_data(data)),

            tag_types::BASIC_MEMINFO => Self::BasicMemInfo(BasicMemoryInfo::from_data(data)),

            tag_types::BOOT_DEVICE => Self::BootDevice(BootDevice::from_data(data)),

            tag_types::MEMORY_MAP => {
                // Memory map: entry_size (u32), entry_version (u32), entries...
                if data.len() >= 8 {
                    let entry_size = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                    let entry_version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
                    let entries = &data[8..];
                    Self::MemoryMap(MemoryMap::new(entry_size, entry_version, entries))
                } else {
                    Self::Unknown {
                        tag_type,
                        size,
                        data,
                    }
                }
            },

            #[cfg(feature = "framebuffer")]
            tag_types::FRAMEBUFFER => Self::Framebuffer(FramebufferInfo::from_data(data)),

            #[cfg(feature = "elf_sections")]
            tag_types::ELF_SECTIONS => Self::ElfSections(ElfSections::from_data(data)),

            #[cfg(feature = "acpi")]
            tag_types::ACPI_OLD => Self::AcpiOld(data),

            #[cfg(feature = "acpi")]
            tag_types::ACPI_NEW => Self::AcpiNew(data),

            tag_types::LOAD_BASE_ADDR => {
                if data.len() >= 8 {
                    let addr = u64::from_le_bytes([
                        data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
                    ]);
                    Self::LoadBaseAddr(addr)
                } else {
                    Self::Unknown {
                        tag_type,
                        size,
                        data,
                    }
                }
            },

            tag_types::EFI64_ST => {
                if data.len() >= 8 {
                    let addr = u64::from_le_bytes([
                        data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
                    ]);
                    Self::Efi64SystemTable(addr)
                } else {
                    Self::Unknown {
                        tag_type,
                        size,
                        data,
                    }
                }
            },

            _ => Self::Unknown {
                tag_type,
                size,
                data,
            },
        }
    }

    /// Get the tag type
    #[must_use]
    pub fn tag_type(&self) -> u32 {
        match self {
            Self::Cmdline(_) => tag_types::CMDLINE,
            Self::BootloaderName(_) => tag_types::BOOTLOADER_NAME,
            Self::Module(_) => tag_types::MODULE,
            Self::BasicMemInfo(_) => tag_types::BASIC_MEMINFO,
            Self::BootDevice(_) => tag_types::BOOT_DEVICE,
            Self::MemoryMap(_) => tag_types::MEMORY_MAP,
            #[cfg(feature = "framebuffer")]
            Self::Framebuffer(_) => tag_types::FRAMEBUFFER,
            #[cfg(feature = "elf_sections")]
            Self::ElfSections(_) => tag_types::ELF_SECTIONS,
            #[cfg(feature = "acpi")]
            Self::AcpiOld(_) => tag_types::ACPI_OLD,
            #[cfg(feature = "acpi")]
            Self::AcpiNew(_) => tag_types::ACPI_NEW,
            Self::LoadBaseAddr(_) => tag_types::LOAD_BASE_ADDR,
            Self::Efi64SystemTable(_) => tag_types::EFI64_ST,
            Self::Unknown { tag_type, .. } => *tag_type,
        }
    }
}

impl fmt::Debug for Tag<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cmdline(c) => f.debug_tuple("Cmdline").field(&c.as_str()).finish(),
            Self::BootloaderName(n) => f.debug_tuple("BootloaderName").field(&n.as_str()).finish(),
            Self::Module(m) => f.debug_tuple("Module").field(m).finish(),
            Self::BasicMemInfo(m) => f.debug_tuple("BasicMemInfo").field(m).finish(),
            Self::BootDevice(d) => f.debug_tuple("BootDevice").field(d).finish(),
            Self::MemoryMap(m) => f.debug_tuple("MemoryMap").field(m).finish(),
            #[cfg(feature = "framebuffer")]
            Self::Framebuffer(fb) => f.debug_tuple("Framebuffer").field(fb).finish(),
            #[cfg(feature = "elf_sections")]
            Self::ElfSections(s) => f.debug_tuple("ElfSections").field(s).finish(),
            #[cfg(feature = "acpi")]
            Self::AcpiOld(d) => f.debug_tuple("AcpiOld").field(&d.len()).finish(),
            #[cfg(feature = "acpi")]
            Self::AcpiNew(d) => f.debug_tuple("AcpiNew").field(&d.len()).finish(),
            Self::LoadBaseAddr(a) => f
                .debug_tuple("LoadBaseAddr")
                .field(&format_args!("{:#x}", a))
                .finish(),
            Self::Efi64SystemTable(a) => f
                .debug_tuple("Efi64SystemTable")
                .field(&format_args!("{:#x}", a))
                .finish(),
            Self::Unknown { tag_type, size, .. } => f
                .debug_struct("Unknown")
                .field("type", tag_type)
                .field("size", size)
                .finish(),
        }
    }
}

// =============================================================================
// Specific Tag Types
// =============================================================================

/// Command line tag
#[derive(Clone)]
pub struct CmdlineTag<'boot> {
    /// Raw string data (null-terminated)
    data: &'boot [u8],
}

impl<'boot> CmdlineTag<'boot> {
    fn from_data(data: &'boot [u8]) -> Self {
        Self { data }
    }

    /// Get the command line as a string
    ///
    /// Returns an empty string if the data is not valid UTF-8.
    #[must_use]
    pub fn as_str(&self) -> &'boot str {
        // Find null terminator
        let len = self
            .data
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.data.len());
        str::from_utf8(&self.data[..len]).unwrap_or("")
    }

    /// Get the raw bytes (including null terminator)
    #[must_use]
    pub fn as_bytes(&self) -> &'boot [u8] {
        self.data
    }
}

/// Bootloader name tag
#[derive(Clone)]
pub struct BootloaderNameTag<'boot> {
    data: &'boot [u8],
}

impl<'boot> BootloaderNameTag<'boot> {
    fn from_data(data: &'boot [u8]) -> Self {
        Self { data }
    }

    /// Get the bootloader name as a string
    #[must_use]
    pub fn as_str(&self) -> &'boot str {
        let len = self
            .data
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.data.len());
        str::from_utf8(&self.data[..len]).unwrap_or("")
    }
}

/// Boot module
#[derive(Clone)]
pub struct BootModule<'boot> {
    /// Module start address
    pub mod_start: u32,
    /// Module end address
    pub mod_end: u32,
    /// Module command line
    cmdline: &'boot [u8],
}

impl<'boot> BootModule<'boot> {
    fn from_data(data: &'boot [u8]) -> Self {
        let mod_start = if data.len() >= 4 {
            u32::from_le_bytes([data[0], data[1], data[2], data[3]])
        } else {
            0
        };

        let mod_end = if data.len() >= 8 {
            u32::from_le_bytes([data[4], data[5], data[6], data[7]])
        } else {
            0
        };

        let cmdline = if data.len() > 8 { &data[8..] } else { &[] };

        Self {
            mod_start,
            mod_end,
            cmdline,
        }
    }

    /// Get the module start address
    #[must_use]
    pub const fn start(&self) -> u32 {
        self.mod_start
    }

    /// Get the module end address
    #[must_use]
    pub const fn end(&self) -> u32 {
        self.mod_end
    }

    /// Get the module size in bytes
    #[must_use]
    pub const fn size(&self) -> u32 {
        self.mod_end.saturating_sub(self.mod_start)
    }

    /// Get the module command line
    #[must_use]
    pub fn cmdline(&self) -> &'boot str {
        let len = self
            .cmdline
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.cmdline.len());
        str::from_utf8(&self.cmdline[..len]).unwrap_or("")
    }
}

impl fmt::Debug for BootModule<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BootModule")
            .field("start", &format_args!("{:#x}", self.mod_start))
            .field("end", &format_args!("{:#x}", self.mod_end))
            .field("size", &self.size())
            .field("cmdline", &self.cmdline())
            .finish()
    }
}

/// Basic memory information (legacy)
#[derive(Debug, Clone, Copy)]
pub struct BasicMemoryInfo {
    /// Lower memory in KB (from 0)
    pub mem_lower: u32,
    /// Upper memory in KB (from 1MB)
    pub mem_upper: u32,
}

impl BasicMemoryInfo {
    fn from_data(data: &[u8]) -> Self {
        let mem_lower = if data.len() >= 4 {
            u32::from_le_bytes([data[0], data[1], data[2], data[3]])
        } else {
            0
        };

        let mem_upper = if data.len() >= 8 {
            u32::from_le_bytes([data[4], data[5], data[6], data[7]])
        } else {
            0
        };

        Self {
            mem_lower,
            mem_upper,
        }
    }

    /// Get total memory in KB
    #[must_use]
    pub const fn total_kb(&self) -> u32 {
        self.mem_lower + self.mem_upper + 1024 // Add the 1MB hole
    }
}

/// BIOS boot device
#[derive(Debug, Clone, Copy)]
pub struct BootDevice {
    /// BIOS drive number
    pub biosdev: u32,
    /// Partition number
    pub partition: u32,
    /// Sub-partition number
    pub sub_partition: u32,
}

impl BootDevice {
    fn from_data(data: &[u8]) -> Self {
        let biosdev = if data.len() >= 4 {
            u32::from_le_bytes([data[0], data[1], data[2], data[3]])
        } else {
            0
        };

        let partition = if data.len() >= 8 {
            u32::from_le_bytes([data[4], data[5], data[6], data[7]])
        } else {
            0
        };

        let sub_partition = if data.len() >= 12 {
            u32::from_le_bytes([data[8], data[9], data[10], data[11]])
        } else {
            0
        };

        Self {
            biosdev,
            partition,
            sub_partition,
        }
    }
}

// =============================================================================
// Optional Tag Types (feature-gated)
// =============================================================================

/// Framebuffer information
#[cfg(feature = "framebuffer")]
#[derive(Clone)]
pub struct FramebufferInfo<'boot> {
    /// Framebuffer physical address
    pub addr: u64,
    /// Pitch (bytes per line)
    pub pitch: u32,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Bits per pixel
    pub bpp: u8,
    /// Framebuffer type
    pub fb_type: u8,
    /// Color info
    color_info: &'boot [u8],
}

#[cfg(feature = "framebuffer")]
impl<'boot> FramebufferInfo<'boot> {
    fn from_data(data: &'boot [u8]) -> Self {
        let addr = if data.len() >= 8 {
            u64::from_le_bytes([
                data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
            ])
        } else {
            0
        };

        let pitch = if data.len() >= 12 {
            u32::from_le_bytes([data[8], data[9], data[10], data[11]])
        } else {
            0
        };

        let width = if data.len() >= 16 {
            u32::from_le_bytes([data[12], data[13], data[14], data[15]])
        } else {
            0
        };

        let height = if data.len() >= 20 {
            u32::from_le_bytes([data[16], data[17], data[18], data[19]])
        } else {
            0
        };

        let bpp = if data.len() >= 21 { data[20] } else { 0 };
        let fb_type = if data.len() >= 22 { data[21] } else { 0 };

        let color_info = if data.len() > 24 { &data[24..] } else { &[] };

        Self {
            addr,
            pitch,
            width,
            height,
            bpp,
            fb_type,
            color_info,
        }
    }

    /// Get framebuffer size in bytes
    #[must_use]
    pub const fn size(&self) -> u64 {
        self.pitch as u64 * self.height as u64
    }
}

#[cfg(feature = "framebuffer")]
impl fmt::Debug for FramebufferInfo<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FramebufferInfo")
            .field("addr", &format_args!("{:#x}", self.addr))
            .field(
                "resolution",
                &format_args!("{}x{}", self.width, self.height),
            )
            .field("bpp", &self.bpp)
            .field("pitch", &self.pitch)
            .finish()
    }
}

/// ELF sections
#[cfg(feature = "elf_sections")]
#[derive(Clone)]
pub struct ElfSections<'boot> {
    /// Number of sections
    pub num: u32,
    /// Section entry size
    pub entsize: u32,
    /// String table index
    pub shndx: u32,
    /// Raw section data
    sections: &'boot [u8],
}

#[cfg(feature = "elf_sections")]
impl<'boot> ElfSections<'boot> {
    fn from_data(data: &'boot [u8]) -> Self {
        let num = if data.len() >= 4 {
            u32::from_le_bytes([data[0], data[1], data[2], data[3]])
        } else {
            0
        };

        let entsize = if data.len() >= 8 {
            u32::from_le_bytes([data[4], data[5], data[6], data[7]])
        } else {
            0
        };

        let shndx = if data.len() >= 12 {
            u32::from_le_bytes([data[8], data[9], data[10], data[11]])
        } else {
            0
        };

        let sections = if data.len() > 12 { &data[12..] } else { &[] };

        Self {
            num,
            entsize,
            shndx,
            sections,
        }
    }
}

#[cfg(feature = "elf_sections")]
impl fmt::Debug for ElfSections<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ElfSections")
            .field("num", &self.num)
            .field("entsize", &self.entsize)
            .field("shndx", &self.shndx)
            .finish()
    }
}
