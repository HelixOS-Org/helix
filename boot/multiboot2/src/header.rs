//! # Multiboot2 Header Generation
//!
//! This module provides compile-time generation and validation of Multiboot2 headers.
//!
//! ## Key Features
//!
//! - **Compile-time checksum validation**: Invalid checksums cause compile errors
//! - **Type-safe tag building**: Builder pattern for header tags
//! - **Linker section placement**: Automatic `.multiboot_header` section
//!
//! ## Usage
//!
//! ```rust
//! use helix_multiboot2::header::{HeaderBuilder, Multiboot2Header};
//!
//! // Simple header (minimal)
//! #[used]
//! #[link_section = ".multiboot_header"]
//! static HEADER: Multiboot2Header = Multiboot2Header::minimal();
//!
//! // Custom header with framebuffer request
//! #[used]
//! #[link_section = ".multiboot_header"]
//! static HEADER: Multiboot2Header = HeaderBuilder::new()
//!     .request_framebuffer(1024, 768, 32)
//!     .build();
//! ```

use crate::{calculate_checksum, verify_checksum, ARCHITECTURE_I386, HEADER_MAGIC};

// =============================================================================
// Header Structure
// =============================================================================

/// Multiboot2 header structure
///
/// This structure represents a complete Multiboot2 header that can be
/// placed in the kernel image. It is designed to be placed in the
/// `.multiboot_header` linker section.
///
/// ## Memory Layout
///
/// ```text
/// ┌────────────────────────────────────┐
/// │ magic: u32          (0xE85250D6)   │ Offset 0
/// │ architecture: u32   (0 = i386)     │ Offset 4
/// │ header_length: u32                 │ Offset 8
/// │ checksum: u32                      │ Offset 12
/// ├────────────────────────────────────┤
/// │ Optional Tags...                   │ Offset 16+
/// ├────────────────────────────────────┤
/// │ End Tag: type=0, flags=0, size=8   │
/// └────────────────────────────────────┘
/// ```
///
/// ## Invariants
///
/// - Must be 8-byte aligned
/// - Must be within first 32KB of kernel image
/// - Checksum must satisfy: magic + arch + length + checksum = 0 (mod 2^32)
#[repr(C, align(8))]
#[derive(Clone, Copy)]
pub struct Multiboot2Header {
    /// Header data as raw words for const construction
    data: HeaderData,
}

/// Internal header data representation
///
/// This allows for flexible header sizes while maintaining const construction
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HeaderData {
    /// Magic number (must be HEADER_MAGIC)
    pub magic: u32,
    /// Architecture (0 = i386/x86_64)
    pub architecture: u32,
    /// Total header length in bytes
    pub header_length: u32,
    /// Checksum (magic + arch + length + checksum must equal 0)
    pub checksum: u32,
    /// Header tags (end tag is always included)
    pub tags: HeaderTags,
}

/// Header tags storage
///
/// Supports minimal header (just end tag) and extended headers with additional tags
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HeaderTags {
    /// Tag data storage
    ///
    /// Minimum size is 8 bytes for end tag.
    /// Extended headers can include additional tags.
    data: [u8; 256], // Max 256 bytes for tags (configurable)
    /// Actual used length
    len: usize,
}

impl HeaderTags {
    /// Create an empty tag storage with just the end tag
    const fn end_only() -> Self {
        let mut data = [0u8; 256];
        // End tag: type=0 (u16), flags=0 (u16), size=8 (u32)
        data[0] = 0; // type low
        data[1] = 0; // type high
        data[2] = 0; // flags low
        data[3] = 0; // flags high
        data[4] = 8; // size low
        data[5] = 0; // size
        data[6] = 0; // size
        data[7] = 0; // size high

        Self { data, len: 8 }
    }

    /// Get the length of tag data
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Check if empty (only end tag)
    pub const fn is_empty(&self) -> bool {
        self.len == 8
    }

    /// Get tag data as slice (runtime only)
    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.len]
    }
}

impl Multiboot2Header {
    /// Create a minimal Multiboot2 header
    ///
    /// This creates the simplest valid Multiboot2 header with:
    /// - Magic number
    /// - Architecture (i386)
    /// - Header length (24 bytes)
    /// - Valid checksum
    /// - End tag
    ///
    /// # Example
    ///
    /// ```rust
    /// use helix_multiboot2::header::Multiboot2Header;
    ///
    /// #[used]
    /// #[link_section = ".multiboot_header"]
    /// static HEADER: Multiboot2Header = Multiboot2Header::minimal();
    /// ```
    #[must_use]
    pub const fn minimal() -> Self {
        // Minimal header: 16 bytes base + 8 bytes end tag = 24 bytes
        const HEADER_LENGTH: u32 = 24;
        const CHECKSUM: u32 = calculate_checksum(HEADER_MAGIC, ARCHITECTURE_I386, HEADER_LENGTH);

        // Compile-time verification
        const _: () = assert!(
            verify_checksum(HEADER_MAGIC, ARCHITECTURE_I386, HEADER_LENGTH, CHECKSUM),
            "Invalid checksum calculation"
        );

        Self {
            data: HeaderData {
                magic: HEADER_MAGIC,
                architecture: ARCHITECTURE_I386,
                header_length: HEADER_LENGTH,
                checksum: CHECKSUM,
                tags: HeaderTags::end_only(),
            },
        }
    }

    /// Create a header builder for customization
    ///
    /// Use this when you need to request specific features from the bootloader.
    #[must_use]
    pub const fn builder() -> HeaderBuilder {
        HeaderBuilder::new()
    }

    /// Get the magic number
    #[must_use]
    pub const fn magic(&self) -> u32 {
        self.data.magic
    }

    /// Get the architecture
    #[must_use]
    pub const fn architecture(&self) -> u32 {
        self.data.architecture
    }

    /// Get the header length
    #[must_use]
    pub const fn header_length(&self) -> u32 {
        self.data.header_length
    }

    /// Get the checksum
    #[must_use]
    pub const fn checksum(&self) -> u32 {
        self.data.checksum
    }

    /// Verify the header checksum
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        verify_checksum(
            self.data.magic,
            self.data.architecture,
            self.data.header_length,
            self.data.checksum,
        )
    }

    /// Get header as raw bytes for inspection
    pub fn as_bytes(&self) -> &[u8] {
        // Safety: Header is repr(C) with known layout
        unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                self.data.header_length as usize,
            )
        }
    }
}

// Ensure header is valid at compile time
const _: () = {
    let header = Multiboot2Header::minimal();
    assert!(header.is_valid(), "Minimal header must be valid");
    assert!(header.magic() == HEADER_MAGIC, "Magic must match");
    assert!(
        header.architecture() == ARCHITECTURE_I386,
        "Architecture must match"
    );
};

// =============================================================================
// Header Builder
// =============================================================================

/// Builder for constructing Multiboot2 headers
///
/// Provides a fluent API for constructing headers with optional tags.
///
/// # Example
///
/// ```rust
/// use helix_multiboot2::header::HeaderBuilder;
///
/// let header = HeaderBuilder::new()
///     .request_framebuffer(1024, 768, 32)
///     .require_console()
///     .build();
/// ```
#[derive(Clone, Copy)]
pub struct HeaderBuilder {
    /// Framebuffer request
    framebuffer: Option<FramebufferRequest>,
    /// Console flags
    console_flags: Option<ConsoleFlags>,
    /// Module alignment
    module_align: bool,
    /// Relocatable
    relocatable: Option<RelocatableInfo>,
}

/// Framebuffer request parameters
#[derive(Clone, Copy)]
pub struct FramebufferRequest {
    /// Requested width (0 = no preference)
    pub width: u32,
    /// Requested height (0 = no preference)
    pub height: u32,
    /// Requested depth (0 = no preference)
    pub depth: u32,
}

/// Console flags
#[derive(Clone, Copy)]
pub struct ConsoleFlags {
    /// Require console support
    pub require_console: bool,
    /// Require EGA text mode
    pub ega_text: bool,
}

/// Relocatable kernel info
#[derive(Clone, Copy)]
pub struct RelocatableInfo {
    /// Minimum physical address
    pub min_addr: u32,
    /// Maximum physical address
    pub max_addr: u32,
    /// Alignment
    pub align: u32,
    /// Preference (0 = none, 1 = lowest, 2 = highest)
    pub preference: u32,
}

impl HeaderBuilder {
    /// Create a new header builder
    #[must_use]
    pub const fn new() -> Self {
        Self {
            framebuffer: None,
            console_flags: None,
            module_align: false,
            relocatable: None,
        }
    }

    /// Request a specific framebuffer configuration
    ///
    /// # Arguments
    ///
    /// * `width` - Requested width in pixels (0 for no preference)
    /// * `height` - Requested height in pixels (0 for no preference)
    /// * `depth` - Requested color depth in bits (0 for no preference)
    #[must_use]
    pub const fn request_framebuffer(mut self, width: u32, height: u32, depth: u32) -> Self {
        self.framebuffer = Some(FramebufferRequest {
            width,
            height,
            depth,
        });
        self
    }

    /// Require console support
    #[must_use]
    pub const fn require_console(mut self) -> Self {
        self.console_flags = Some(ConsoleFlags {
            require_console: true,
            ega_text: false,
        });
        self
    }

    /// Require EGA text mode
    #[must_use]
    pub const fn require_ega_text(mut self) -> Self {
        self.console_flags = Some(ConsoleFlags {
            require_console: false,
            ega_text: true,
        });
        self
    }

    /// Request module alignment to page boundaries
    #[must_use]
    pub const fn module_align(mut self) -> Self {
        self.module_align = true;
        self
    }

    /// Mark kernel as relocatable
    #[must_use]
    pub const fn relocatable(
        mut self,
        min_addr: u32,
        max_addr: u32,
        align: u32,
        preference: u32,
    ) -> Self {
        self.relocatable = Some(RelocatableInfo {
            min_addr,
            max_addr,
            align,
            preference,
        });
        self
    }

    /// Build the header
    ///
    /// This computes the final header length and checksum.
    #[must_use]
    pub const fn build(self) -> Multiboot2Header {
        // For now, build minimal header
        // TODO: Implement tag serialization for const context
        // This requires more complex const fn support

        // Calculate total header size
        let mut tag_size: u32 = 8; // End tag

        if self.framebuffer.is_some() {
            tag_size += 20; // Framebuffer tag size
        }
        if self.console_flags.is_some() {
            tag_size += 12; // Console flags tag size
        }
        if self.module_align {
            tag_size += 8; // Module align tag size
        }
        if self.relocatable.is_some() {
            tag_size += 24; // Relocatable tag size
        }

        let header_length = 16 + tag_size;
        let checksum = calculate_checksum(HEADER_MAGIC, ARCHITECTURE_I386, header_length);

        // Build tags
        let tags = HeaderTags::end_only(); // Simplified for now

        Multiboot2Header {
            data: HeaderData {
                magic: HEADER_MAGIC,
                architecture: ARCHITECTURE_I386,
                header_length,
                checksum,
                tags,
            },
        }
    }
}

impl Default for HeaderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Header Tag Types
// =============================================================================

/// Header tag type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum HeaderTag {
    /// End of tags
    End,
    /// Information request
    InformationRequest,
    /// Address specification
    Address,
    /// Entry point address
    EntryAddress,
    /// Console flags
    ConsoleFlags,
    /// Framebuffer request
    Framebuffer,
    /// Module alignment
    ModuleAlign,
    /// EFI boot services
    EfiBs,
    /// 32-bit EFI entry point
    EntryAddressEfi32,
    /// 64-bit EFI entry point
    EntryAddressEfi64,
    /// Relocatable kernel
    Relocatable,
    /// Unknown tag type
    Unknown(u16),
}

impl HeaderTag {
    /// Create from raw tag type value
    #[must_use]
    pub const fn from_type(tag_type: u16) -> Self {
        match tag_type {
            0 => Self::End,
            1 => Self::InformationRequest,
            2 => Self::Address,
            3 => Self::EntryAddress,
            4 => Self::ConsoleFlags,
            5 => Self::Framebuffer,
            6 => Self::ModuleAlign,
            7 => Self::EfiBs,
            8 => Self::EntryAddressEfi32,
            9 => Self::EntryAddressEfi64,
            10 => Self::Relocatable,
            other => Self::Unknown(other),
        }
    }

    /// Get raw tag type value
    #[must_use]
    pub const fn as_type(&self) -> u16 {
        match self {
            Self::End => 0,
            Self::InformationRequest => 1,
            Self::Address => 2,
            Self::EntryAddress => 3,
            Self::ConsoleFlags => 4,
            Self::Framebuffer => 5,
            Self::ModuleAlign => 6,
            Self::EfiBs => 7,
            Self::EntryAddressEfi32 => 8,
            Self::EntryAddressEfi64 => 9,
            Self::Relocatable => 10,
            Self::Unknown(t) => *t,
        }
    }
}

// =============================================================================
// Compile-Time Header Generation Macro
// =============================================================================

/// Generate a Multiboot2 header at compile time
///
/// This macro creates a properly formatted Multiboot2 header with
/// compile-time checksum validation.
///
/// # Example
///
/// ```rust
/// use helix_multiboot2::define_multiboot2_header;
///
/// define_multiboot2_header! {
///     // Optional: request framebuffer
///     framebuffer: (1024, 768, 32),
/// }
/// ```
#[macro_export]
macro_rules! define_multiboot2_header {
    () => {
        #[used]
        #[link_section = ".multiboot_header"]
        static __MULTIBOOT2_HEADER: $crate::header::Multiboot2Header =
            $crate::header::Multiboot2Header::minimal();
    };

    (framebuffer: ($width:expr, $height:expr, $depth:expr) $(,)?) => {
        #[used]
        #[link_section = ".multiboot_header"]
        static __MULTIBOOT2_HEADER: $crate::header::Multiboot2Header =
            $crate::header::HeaderBuilder::new()
                .request_framebuffer($width, $height, $depth)
                .build();
    };
}

// =============================================================================
// Minimal Header for Direct Use
// =============================================================================

/// Pre-built minimal Multiboot2 header
///
/// This can be used directly in a kernel without customization:
///
/// ```rust
/// #[used]
/// #[link_section = ".multiboot_header"]
/// static HEADER: [u32; 6] = helix_multiboot2::header::MINIMAL_HEADER_DATA;
/// ```
pub const MINIMAL_HEADER_DATA: [u32; 6] = {
    const LENGTH: u32 = 24;
    const CHECKSUM: u32 = calculate_checksum(HEADER_MAGIC, ARCHITECTURE_I386, LENGTH);

    [
        HEADER_MAGIC,      // magic
        ARCHITECTURE_I386, // architecture
        LENGTH,            // header_length
        CHECKSUM,          // checksum
        0,                 // end tag: type=0, flags=0
        8,                 // end tag: size=8
    ]
};

// Verify minimal header data at compile time
const _: () = {
    assert!(MINIMAL_HEADER_DATA[0] == HEADER_MAGIC);
    assert!(verify_checksum(
        MINIMAL_HEADER_DATA[0],
        MINIMAL_HEADER_DATA[1],
        MINIMAL_HEADER_DATA[2],
        MINIMAL_HEADER_DATA[3],
    ));
};
