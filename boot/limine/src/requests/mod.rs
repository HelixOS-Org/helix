//! # Limine Request Definitions
//!
//! This module provides high-level, type-safe request structures for all
//! Limine boot protocol requests. Each request type is encapsulated in
//! a safe wrapper that handles proper initialization and access patterns.
//!
//! ## Request Categories
//!
//! ### System Information
//! - [`BootloaderInfoRequest`]: Get bootloader name and version
//! - [`BootTimeRequest`]: Get boot time (UNIX timestamp)
//!
//! ### Memory Management
//! - [`MemoryMapRequest`]: Get physical memory map
//! - [`HhdmRequest`]: Get Higher Half Direct Map offset
//! - [`PagingModeRequest`]: Configure paging mode
//!
//! ### Kernel Information
//! - [`KernelFileRequest`]: Get kernel file information
//! - [`KernelAddressRequest`]: Get kernel physical/virtual addresses
//! - [`ModuleRequest`]: Get loaded modules
//!
//! ### CPU/SMP
//! - [`SmpRequest`]: Symmetric Multi-Processing support
//!
//! ### Framebuffer
//! - [`FramebufferRequest`]: Get framebuffer(s) for graphics
//!
//! ### Firmware
//! - [`RsdpRequest`]: ACPI RSDP pointer
//! - [`SmbiosRequest`]: SMBIOS tables
//! - [`EfiSystemTableRequest`]: EFI System Table
//! - [`EfiMemmapRequest`]: EFI memory map
//! - [`DtbRequest`]: Device Tree Blob (ARM/RISC-V)
//!
//! ### Boot Control
//! - [`EntryPointRequest`]: Custom kernel entry point
//! - [`StackSizeRequest`]: Custom stack size
//!
//! ## Usage
//!
//! ```rust,no_run
//! use helix_limine::requests::*;
//!
//! // Declare requests in a special section
//! #[used]
//! #[link_section = ".limine_requests"]
//! static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();
//!
//! #[used]
//! #[link_section = ".limine_requests"]
//! static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();
//! ```

pub mod boot;
pub mod bootloader;
pub mod firmware;
pub mod framebuffer;
pub mod kernel;
pub mod memory;
pub mod smp;

// Re-export all request types
use core::sync::atomic::{AtomicPtr, Ordering};

pub use boot::*;
pub use bootloader::*;
pub use firmware::*;
pub use framebuffer::*;
pub use kernel::*;
pub use memory::*;
pub use smp::*;

/// Trait for all Limine requests
///
/// This trait provides a common interface for accessing request responses.
pub trait LimineRequest {
    /// The response type for this request
    type Response;

    /// Get the request's unique identifier
    fn id(&self) -> [u64; 4];

    /// Get the request revision
    fn revision(&self) -> u64;

    /// Check if the bootloader has provided a response
    fn has_response(&self) -> bool;

    /// Get the response, if available
    fn response(&self) -> Option<&Self::Response>;
}

/// Helper struct for atomic response pointer management
#[repr(C)]
pub struct ResponsePtr<T> {
    ptr: AtomicPtr<T>,
}

impl<T> ResponsePtr<T> {
    /// Create a null response pointer
    pub const fn null() -> Self {
        Self {
            ptr: AtomicPtr::new(core::ptr::null_mut()),
        }
    }

    /// Check if the response is available
    pub fn is_available(&self) -> bool {
        !self.ptr.load(Ordering::Acquire).is_null()
    }

    /// Get a reference to the response, if available
    ///
    /// # Safety
    ///
    /// The caller must ensure the response pointer is valid and
    /// points to properly initialized memory.
    pub unsafe fn get(&self) -> Option<&T> {
        let ptr = self.ptr.load(Ordering::Acquire);
        if ptr.is_null() {
            None
        } else {
            // SAFETY: Caller guarantees the pointer is valid
            unsafe { Some(&*ptr) }
        }
    }

    /// Get the raw pointer
    pub fn as_ptr(&self) -> *const T {
        self.ptr.load(Ordering::Acquire)
    }
}

// Safety: Response pointers are only written by bootloader during boot
unsafe impl<T> Sync for ResponsePtr<T> {}

/// Marker trait for responses that can be safely accessed
///
/// # Safety
///
/// Implementors must ensure the response structure is valid and
/// can be safely accessed from any context.
pub unsafe trait SafeResponse: Sized {
    /// Validate the response structure
    fn validate(&self) -> bool;
}
