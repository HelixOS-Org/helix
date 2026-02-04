//! # Helix Limine - Industrial-Grade Limine Protocol Implementation
//!
//! This crate provides a **complete, type-safe, zero-copy** implementation of the
//! [Limine Boot Protocol](https://github.com/limine-bootloader/limine/blob/trunk/PROTOCOL.md).
//!
//! ## Features
//!
//! - **Complete Protocol Coverage**: All Limine request types implemented
//! - **Type Safety**: Strong typing with compile-time guarantees
//! - **Zero-Copy**: No allocations during boot, direct memory access
//! - **SMP Support**: Full multi-processor initialization
//! - **Memory Safety**: Minimal unsafe, maximum verification
//! - **Extensibility**: Designed for future protocol versions
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! #![no_std]
//! #![no_main]
//!
//! use helix_limine::prelude::*;
//!
//! // Declare requests (placed in .limine_requests section)
//! static BOOTLOADER_INFO: BootloaderInfoRequest =
//!     BootloaderInfoRequest::new();
//! static MEMORY_MAP: MemoryMapRequest = MemoryMapRequest::new();
//! static HHDM: HhdmRequest = HhdmRequest::new();
//! static FRAMEBUFFER: FramebufferRequest = FramebufferRequest::new();
//! static SMP: SmpRequest = SmpRequest::new();
//!
//! #[no_mangle]
//! extern "C" fn _start() -> ! {
//!     // Access responses (filled by bootloader)
//!     let boot_info = BOOTLOADER_INFO.response().expect("No bootloader info");
//!     let memory_map = MEMORY_MAP.response().expect("No memory map");
//!     let hhdm = HHDM.response().expect("No HHDM");
//!
//!     // Use typed abstractions
//!     for region in memory_map.regions() {
//!         if region.is_usable() {
//!             // Initialize memory allocator
//!         }
//!     }
//!
//!     loop {}
//! }
//! ```
//!
//! ## Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────┐
//! │                     HELIX LIMINE STACK                             │
//! ├────────────────────────────────────────────────────────────────────┤
//! │  Layer 3: Safe Abstractions (memory, cpu, framebuffer, firmware)  │
//! │  Layer 2: Response Parsing (typed, lifetime-bound)                │
//! │  Layer 1: Request Declarations (static, linker section)           │
//! │  Layer 0: Raw Protocol (FFI structures, constants)                │
//! └────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Protocol Version
//!
//! This implementation targets **Limine Protocol Revision 2** and is
//! forward-compatible with future revisions through the revision negotiation
//! mechanism.

#![no_std]
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::unnested_or_patterns)]
#![allow(unused_doc_comments)]
#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]

// =============================================================================
// Module Declarations
// =============================================================================

/// Raw protocol definitions (Layer 0)
pub mod protocol;

/// Request structures (Layer 1)
pub mod requests;

/// Response parsing (Layer 2)
pub mod responses;

/// Memory model abstractions
pub mod memory;

/// Kernel relocation (PIE/KASLR)
pub mod relocation;

/// CPU topology and SMP
#[cfg(feature = "smp")]
pub mod cpu;

/// Framebuffer engine
#[cfg(feature = "framebuffer")]
pub mod framebuffer;

/// Firmware interfaces (ACPI, SMBIOS, EFI, DTB)
pub mod firmware;

/// File abstractions (kernel, modules)
pub mod file;

/// Boot information unified abstraction
pub mod boot_info;

/// Entry point handling
pub mod entry;

/// Validation and invariants
pub mod validate;

/// Error types
pub mod error;

// =============================================================================
// Prelude - Commonly used items
// =============================================================================

/// Prelude module for convenient imports
pub mod prelude {
    // Re-export common request types
    // Re-export boot info
    pub use crate::boot_info::BootInfo;
    // Re-export CPU utilities
    #[cfg(feature = "smp")]
    pub use crate::cpu::{cpu_count, current_cpu_id, is_bsp};
    // Re-export entry utilities
    pub use crate::entry::halt_loop;
    // Re-export error types
    pub use crate::error::Error;
    // Re-export file types
    pub use crate::file::{File, FileType, ModuleCollection};
    // Re-export firmware utilities
    #[cfg(feature = "acpi")]
    pub use crate::firmware::AcpiFinder;
    // Re-export framebuffer utilities
    #[cfg(feature = "framebuffer")]
    pub use crate::framebuffer::{Console, Graphics, Point, Rect};
    // Re-export memory types
    pub use crate::memory::{MemoryRegion, MemoryRegionKind, PhysAddr, VirtAddr, HHDM};
    // Re-export Color from requests
    #[cfg(feature = "framebuffer")]
    pub use crate::requests::Color;
    #[cfg(feature = "framebuffer")]
    pub use crate::requests::FramebufferRequest;
    #[cfg(feature = "modules")]
    pub use crate::requests::ModuleRequest;
    #[cfg(feature = "acpi")]
    pub use crate::requests::RsdpRequest;
    #[cfg(feature = "smp")]
    pub use crate::requests::SmpRequest;
    pub use crate::requests::{
        BootloaderInfoRequest, HhdmRequest, KernelAddressRequest, KernelFileRequest,
        MemoryMapRequest,
    };
    #[cfg(feature = "framebuffer")]
    pub use crate::responses::FramebufferInfo;
    // Re-export response abstractions
    pub use crate::responses::{BootTime, BootloaderInfo, HhdmInfo, KernelAddress, MemoryMap};
    #[cfg(feature = "smp")]
    pub use crate::responses::{CpuInfo, SmpInfo};
    // Re-export validation
    pub use crate::validate::BootValidator;
}

// =============================================================================
// Re-exports
// =============================================================================

pub use boot_info::BootInfo;
pub use error::Error;
pub use memory::{PhysAddr, VirtAddr, HHDM};
pub use protocol::{LIMINE_MAGIC, LIMINE_REVISION};

// =============================================================================
// Constants
// =============================================================================

/// Limine protocol base revision
///
/// This crate targets Limine protocol revision 2.
pub const PROTOCOL_REVISION: u64 = 2;

/// Limine requests section name
///
/// All Limine requests must be placed in this linker section.
pub const REQUESTS_SECTION: &str = ".limine_requests";

/// Limine requests start marker section
pub const REQUESTS_START_SECTION: &str = ".limine_requests_start";

/// Limine requests end marker section
pub const REQUESTS_END_SECTION: &str = ".limine_requests_end";

// =============================================================================
// Base Revision Request
// =============================================================================

/// Base revision marker
///
/// This structure is used to declare the protocol revision supported by the kernel.
/// It must be placed in the `.limine_requests` section.
///
/// # Example
///
/// ```rust,no_run
/// use helix_limine::BaseRevision;
///
/// #[used]
/// #[link_section = ".limine_requests"]
/// static BASE_REVISION: BaseRevision = BaseRevision::new();
/// ```
#[repr(C)]
pub struct BaseRevision {
    /// Revision marker (set to 0xf9562b2d5c95a6c8)
    id: [u64; 2],
    /// Supported revision (2 for current protocol)
    revision: u64,
}

impl BaseRevision {
    /// Magic identifier for base revision
    const MAGIC: [u64; 2] = [0xf956_2b2d_5c95_a6c8, 0x6a7b_3849_4453_6bdc];

    /// Create a new base revision marker
    pub const fn new() -> Self {
        Self {
            id: Self::MAGIC,
            revision: PROTOCOL_REVISION,
        }
    }

    /// Create with specific revision
    pub const fn with_revision(revision: u64) -> Self {
        Self {
            id: Self::MAGIC,
            revision,
        }
    }

    /// Check if bootloader accepted this revision
    ///
    /// After boot, if the revision field is 0, the bootloader supports
    /// at least this revision. Otherwise, it contains the maximum
    /// revision supported.
    pub fn is_supported(&self) -> bool {
        self.revision == 0
    }

    /// Get the bootloader's maximum supported revision
    ///
    /// Returns `None` if our revision is supported, or `Some(max)`
    /// if the bootloader only supports up to `max`.
    pub fn bootloader_max_revision(&self) -> Option<u64> {
        if self.revision == 0 {
            None
        } else {
            Some(self.revision)
        }
    }
}

impl Default for BaseRevision {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Request Markers
// =============================================================================

/// Marker for the start of the requests section
#[repr(C)]
pub struct RequestsStartMarker {
    marker: [u64; 4],
}

impl RequestsStartMarker {
    /// Create the start marker
    pub const fn new() -> Self {
        Self {
            marker: [
                0xf956_2b2d_5c95_a6c8,
                0x6a7b_3849_4453_6bdc,
                0xf956_2b2d_5c95_a6c8,
                0x6a7b_3849_4453_6bdc,
            ],
        }
    }
}

impl Default for RequestsStartMarker {
    fn default() -> Self {
        Self::new()
    }
}

/// Marker for the end of the requests section
#[repr(C)]
pub struct RequestsEndMarker {
    marker: [u64; 2],
}

impl RequestsEndMarker {
    /// Create the end marker
    pub const fn new() -> Self {
        Self {
            marker: [0xadc0_e053_1bb1_0d03, 0x9572_709f_3176_4c62],
        }
    }
}

impl Default for RequestsEndMarker {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Utility Macros
// =============================================================================

/// Declare Limine requests with proper section placement
///
/// This macro simplifies the declaration of Limine requests by automatically
/// placing them in the correct linker section.
///
/// # Example
///
/// ```rust,no_run
/// use helix_limine::limine_requests;
///
/// limine_requests! {
///     static BOOTLOADER_INFO: BootloaderInfoRequest = BootloaderInfoRequest::new();
///     static MEMORY_MAP: MemoryMapRequest = MemoryMapRequest::new();
///     static HHDM: HhdmRequest = HhdmRequest::new();
/// }
/// ```
#[macro_export]
macro_rules! limine_requests {
    (
        $(
            $(#[$attr:meta])*
            static $name:ident : $ty:ty = $init:expr ;
        )*
    ) => {
        // Start marker
        #[used]
        #[link_section = ".limine_requests_start"]
        static __LIMINE_REQUESTS_START: $crate::RequestsStartMarker =
            $crate::RequestsStartMarker::new();

        // Base revision (always first)
        #[used]
        #[link_section = ".limine_requests"]
        static __LIMINE_BASE_REVISION: $crate::BaseRevision =
            $crate::BaseRevision::new();

        // User requests
        $(
            $(#[$attr])*
            #[used]
            #[link_section = ".limine_requests"]
            static $name: $ty = $init;
        )*

        // End marker
        #[used]
        #[link_section = ".limine_requests_end"]
        static __LIMINE_REQUESTS_END: $crate::RequestsEndMarker =
            $crate::RequestsEndMarker::new();
    };
}

/// Declare a single Limine request with proper section placement
#[macro_export]
macro_rules! limine_request {
    (
        $(#[$attr:meta])*
        static $name:ident : $ty:ty = $init:expr
    ) => {
        $(#[$attr])*
        #[used]
        #[link_section = ".limine_requests"]
        static $name: $ty = $init;
    };
}

// =============================================================================
// Compile-Time Assertions
// =============================================================================

const _: () = {
    // Verify BaseRevision size and alignment
    assert!(core::mem::size_of::<BaseRevision>() == 24);
    assert!(core::mem::align_of::<BaseRevision>() == 8);

    // Verify marker sizes
    assert!(core::mem::size_of::<RequestsStartMarker>() == 32);
    assert!(core::mem::size_of::<RequestsEndMarker>() == 16);
};
