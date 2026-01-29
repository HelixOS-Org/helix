//! # Unified Boot Information Abstraction
//!
//! This module provides a protocol-agnostic boot information interface,
//! allowing kernel code to work with boot data regardless of the
//! underlying boot protocol (Multiboot2, Limine, UEFI, etc.).
//!
//! ## Design Philosophy
//!
//! The `BootInfo` struct and `BootProtocol` trait abstract away the
//! differences between boot protocols, providing a unified interface:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                        KERNEL (protocol-agnostic)                       │
//! │                                                                         │
//! │    fn init(boot_info: &BootInfo) {                                      │
//! │        let memory_map = boot_info.memory_map();                         │
//! │        let cmdline = boot_info.cmdline();                               │
//! │        // Works with any boot protocol!                                 │
//! │    }                                                                    │
//! └───────────────────────────────────┬─────────────────────────────────────┘
//!                                     │
//!                                     ▼
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                          BootInfo Trait                                 │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  • memory_map()     → Iterator<MemoryRegion>                            │
//! │  • cmdline()        → Option<&str>                                      │
//! │  • framebuffer()    → Option<FramebufferInfo>                           │
//! │  • rsdp()           → Option<*const u8>                                 │
//! │  • modules()        → Iterator<BootModule>                              │
//! └───────────────────────────────────┬─────────────────────────────────────┘
//!                                     │
//!          ┌──────────────────────────┼──────────────────────────┐
//!          │                          │                          │
//!          ▼                          ▼                          ▼
//! ┌─────────────────┐      ┌─────────────────┐      ┌─────────────────┐
//! │   Multiboot2    │      │     Limine      │      │      UEFI       │
//! │  Implementation │      │ (future)        │      │  (future)       │
//! └─────────────────┘      └─────────────────┘      └─────────────────┘
//! ```

use core::fmt;

use crate::memory::{MemoryRegion, MemoryStats};
use crate::info::Multiboot2Info;

// =============================================================================
// Boot Protocol Trait
// =============================================================================

/// Trait for boot protocol implementations
///
/// This trait defines the common interface that all boot protocols must
/// implement. It allows kernel code to be protocol-agnostic.
///
/// # Example
///
/// ```rust,no_run
/// use helix_multiboot2::{BootProtocol, BootInfo};
///
/// fn kernel_init<P: BootProtocol>(boot_info: &P) {
///     // Access memory map
///     for region in boot_info.memory_regions() {
///         if region.is_usable() {
///             // Initialize memory allocator...
///         }
///     }
///
///     // Get command line
///     if let Some(cmdline) = boot_info.cmdline() {
///         // Parse boot options...
///     }
/// }
/// ```
pub trait BootProtocol {
    /// Get the boot protocol name
    fn protocol_name(&self) -> &'static str;

    /// Get the command line, if available
    fn cmdline(&self) -> Option<&str>;

    /// Get the bootloader name, if available
    fn bootloader_name(&self) -> Option<&str>;

    /// Get an iterator over memory regions
    fn memory_regions(&self) -> MemoryRegionIter<'_>;

    /// Get the total available memory in bytes
    fn total_memory(&self) -> u64;

    /// Get the ACPI RSDP pointer, if available
    fn rsdp(&self) -> Option<*const u8>;

    /// Get framebuffer information, if available
    fn framebuffer_addr(&self) -> Option<u64>;

    /// Get the kernel load address, if available
    fn kernel_load_addr(&self) -> Option<u64>;
}

// =============================================================================
// Memory Region Iterator (Protocol-Agnostic)
// =============================================================================

/// Protocol-agnostic memory region iterator
///
/// This wrapper allows iterating over memory regions regardless of
/// the underlying boot protocol.
pub struct MemoryRegionIter<'a> {
    inner: MemoryRegionIterInner<'a>,
}

enum MemoryRegionIterInner<'a> {
    Multiboot2(crate::memory::MemoryRegionIterator<'a>),
    Static(core::slice::Iter<'a, MemoryRegion>),
    Empty,
}

impl<'a> MemoryRegionIter<'a> {
    /// Create from a Multiboot2 memory map
    pub fn from_multiboot2(iter: crate::memory::MemoryRegionIterator<'a>) -> Self {
        Self {
            inner: MemoryRegionIterInner::Multiboot2(iter),
        }
    }

    /// Create from a static slice of regions
    pub fn from_slice(regions: &'a [MemoryRegion]) -> Self {
        Self {
            inner: MemoryRegionIterInner::Static(regions.iter()),
        }
    }

    /// Create an empty iterator
    pub fn empty() -> Self {
        Self {
            inner: MemoryRegionIterInner::Empty,
        }
    }
}

impl<'a> Iterator for MemoryRegionIter<'a> {
    type Item = MemoryRegion;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            MemoryRegionIterInner::Multiboot2(iter) => iter.next(),
            MemoryRegionIterInner::Static(iter) => iter.next().copied(),
            MemoryRegionIterInner::Empty => None,
        }
    }
}

// =============================================================================
// Unified Boot Info
// =============================================================================

/// Unified boot information structure
///
/// This struct provides a protocol-agnostic view of boot information.
/// It can be created from any supported boot protocol (Multiboot2, Limine, etc.)
/// and provides a common interface for kernel initialization.
///
/// # Supported Protocols
///
/// - **Multiboot2**: Via `from_multiboot2()`
/// - **Limine**: Via `from_limine()` (future)
/// - **UEFI**: Via `from_uefi()` (future)
///
/// # Example
///
/// ```rust,no_run
/// use helix_multiboot2::{BootInfo, Multiboot2Info, BOOTLOADER_MAGIC};
///
/// fn kernel_main(magic: u32, info_ptr: *const u8) {
///     let boot_info = if magic == BOOTLOADER_MAGIC {
///         let mb2 = unsafe { Multiboot2Info::from_ptr(info_ptr).unwrap() };
///         BootInfo::from_multiboot2(mb2)
///     } else {
///         panic!("Unknown boot protocol");
///     };
///
///     // Use unified interface
///     init_memory(&boot_info);
///     init_framebuffer(&boot_info);
/// }
/// ```
pub struct BootInfo<'boot> {
    /// Boot protocol variant
    inner: BootInfoInner<'boot>,
    /// Cached memory statistics
    memory_stats: Option<MemoryStats>,
}

/// Internal representation of boot info
enum BootInfoInner<'boot> {
    /// Multiboot2 boot information
    Multiboot2(Multiboot2Info<'boot>),

    /// Static/manual boot information (for testing or custom protocols)
    Static(StaticBootInfo<'boot>),
}

/// Static boot information for testing or custom protocols
struct StaticBootInfo<'boot> {
    cmdline: Option<&'boot str>,
    bootloader: Option<&'boot str>,
    memory_regions: &'boot [MemoryRegion],
    rsdp: Option<*const u8>,
    framebuffer_addr: Option<u64>,
    kernel_load_addr: Option<u64>,
}

impl<'boot> BootInfo<'boot> {
    /// Create from Multiboot2 boot information
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use helix_multiboot2::{BootInfo, Multiboot2Info};
    ///
    /// let mb2 = unsafe { Multiboot2Info::from_ptr(ptr).unwrap() };
    /// let boot_info = BootInfo::from_multiboot2(mb2);
    /// ```
    #[must_use]
    pub fn from_multiboot2(info: Multiboot2Info<'boot>) -> Self {
        Self {
            inner: BootInfoInner::Multiboot2(info),
            memory_stats: None,
        }
    }

    /// Create a static boot info for testing
    ///
    /// This is useful for unit tests or for platforms that don't use
    /// a standard boot protocol.
    #[must_use]
    pub fn from_static(
        cmdline: Option<&'boot str>,
        memory_regions: &'boot [MemoryRegion],
    ) -> Self {
        Self {
            inner: BootInfoInner::Static(StaticBootInfo {
                cmdline,
                bootloader: None,
                memory_regions,
                rsdp: None,
                framebuffer_addr: None,
                kernel_load_addr: None,
            }),
            memory_stats: None,
        }
    }

    /// Get the boot protocol name
    #[must_use]
    pub fn protocol_name(&self) -> &'static str {
        match &self.inner {
            BootInfoInner::Multiboot2(_) => "Multiboot2",
            BootInfoInner::Static(_) => "Static",
        }
    }

    /// Get the command line
    #[must_use]
    pub fn cmdline(&self) -> Option<&str> {
        match &self.inner {
            BootInfoInner::Multiboot2(info) => info.cmdline(),
            BootInfoInner::Static(info) => info.cmdline,
        }
    }

    /// Get the bootloader name
    #[must_use]
    pub fn bootloader_name(&self) -> Option<&str> {
        match &self.inner {
            BootInfoInner::Multiboot2(info) => info.bootloader_name(),
            BootInfoInner::Static(info) => info.bootloader,
        }
    }

    /// Get an iterator over memory regions
    pub fn memory_regions(&self) -> MemoryRegionIter<'boot> {
        match &self.inner {
            BootInfoInner::Multiboot2(info) => {
                if let Some(mmap) = info.memory_map() {
                    MemoryRegionIter::from_multiboot2(mmap.regions())
                } else {
                    MemoryRegionIter::empty()
                }
            }
            BootInfoInner::Static(info) => {
                MemoryRegionIter::from_slice(info.memory_regions)
            }
        }
    }

    /// Get an iterator over usable memory regions only
    pub fn usable_memory_regions(&self) -> impl Iterator<Item = MemoryRegion> + 'boot {
        self.memory_regions().filter(|r| r.is_usable())
    }

    /// Get total available memory in bytes
    pub fn total_memory(&self) -> u64 {
        self.memory_regions().map(|r| r.length()).sum()
    }

    /// Get total usable memory in bytes
    pub fn total_usable_memory(&self) -> u64 {
        self.usable_memory_regions().map(|r| r.length()).sum()
    }

    /// Find the largest usable memory region
    pub fn largest_usable_region(&self) -> Option<MemoryRegion> {
        self.usable_memory_regions().max_by_key(|r| r.length())
    }

    /// Get ACPI RSDP pointer
    #[must_use]
    pub fn rsdp(&self) -> Option<*const u8> {
        match &self.inner {
            #[cfg(feature = "acpi")]
            BootInfoInner::Multiboot2(info) => {
                info.acpi_new_rsdp()
                    .or_else(|| info.acpi_old_rsdp())
                    .map(|d| d.as_ptr())
            }
            #[cfg(not(feature = "acpi"))]
            BootInfoInner::Multiboot2(_) => None,
            BootInfoInner::Static(info) => info.rsdp,
        }
    }

    /// Get framebuffer address
    #[must_use]
    pub fn framebuffer_addr(&self) -> Option<u64> {
        match &self.inner {
            #[cfg(feature = "framebuffer")]
            BootInfoInner::Multiboot2(info) => info.framebuffer().map(|fb| fb.addr),
            #[cfg(not(feature = "framebuffer"))]
            BootInfoInner::Multiboot2(_) => None,
            BootInfoInner::Static(info) => info.framebuffer_addr,
        }
    }

    /// Get kernel load address
    #[must_use]
    pub fn kernel_load_addr(&self) -> Option<u64> {
        match &self.inner {
            BootInfoInner::Multiboot2(info) => info.load_base_addr(),
            BootInfoInner::Static(info) => info.kernel_load_addr,
        }
    }

    /// Compute and cache memory statistics
    pub fn compute_memory_stats(&mut self) -> MemoryStats {
        if self.memory_stats.is_none() {
            match &self.inner {
                BootInfoInner::Multiboot2(info) => {
                    if let Some(mmap) = info.memory_map() {
                        self.memory_stats = Some(MemoryStats::from_map(&mmap));
                    }
                }
                BootInfoInner::Static(info) => {
                    // Compute from slice
                    let mut stats = MemoryStats {
                        total: 0,
                        available: 0,
                        reserved: 0,
                        acpi_reclaimable: 0,
                        acpi_nvs: 0,
                        bad: 0,
                        region_count: info.memory_regions.len(),
                        usable_region_count: 0,
                    };

                    for region in info.memory_regions {
                        stats.total += region.length();
                        if region.is_usable() {
                            stats.available += region.length();
                            stats.usable_region_count += 1;
                        }
                    }

                    self.memory_stats = Some(stats);
                }
            }
        }

        self.memory_stats.unwrap_or(MemoryStats {
            total: 0,
            available: 0,
            reserved: 0,
            acpi_reclaimable: 0,
            acpi_nvs: 0,
            bad: 0,
            region_count: 0,
            usable_region_count: 0,
        })
    }
}

impl fmt::Debug for BootInfo<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BootInfo")
            .field("protocol", &self.protocol_name())
            .field("cmdline", &self.cmdline())
            .field("bootloader", &self.bootloader_name())
            .field("total_memory", &format_args!("{} MB", self.total_memory() / (1024 * 1024)))
            .finish()
    }
}

impl BootProtocol for BootInfo<'_> {
    fn protocol_name(&self) -> &'static str {
        BootInfo::protocol_name(self)
    }

    fn cmdline(&self) -> Option<&str> {
        BootInfo::cmdline(self)
    }

    fn bootloader_name(&self) -> Option<&str> {
        BootInfo::bootloader_name(self)
    }

    fn memory_regions(&self) -> MemoryRegionIter<'_> {
        match &self.inner {
            BootInfoInner::Multiboot2(info) => {
                if let Some(mmap) = info.memory_map() {
                    MemoryRegionIter::from_multiboot2(mmap.regions())
                } else {
                    MemoryRegionIter::empty()
                }
            }
            BootInfoInner::Static(info) => {
                MemoryRegionIter::from_slice(info.memory_regions)
            }
        }
    }

    fn total_memory(&self) -> u64 {
        BootInfo::total_memory(self)
    }

    fn rsdp(&self) -> Option<*const u8> {
        BootInfo::rsdp(self)
    }

    fn framebuffer_addr(&self) -> Option<u64> {
        BootInfo::framebuffer_addr(self)
    }

    fn kernel_load_addr(&self) -> Option<u64> {
        BootInfo::kernel_load_addr(self)
    }
}

// =============================================================================
// Boot Info Builder (for testing)
// =============================================================================

/// Builder for creating test boot info
///
/// This is useful for unit testing kernel code without a real bootloader.
pub struct BootInfoBuilder<'a> {
    cmdline: Option<&'a str>,
    bootloader: Option<&'a str>,
    memory_regions: &'a [MemoryRegion],
    rsdp: Option<*const u8>,
    framebuffer_addr: Option<u64>,
    kernel_load_addr: Option<u64>,
}

impl<'a> BootInfoBuilder<'a> {
    /// Create a new builder
    #[must_use]
    pub const fn new() -> Self {
        Self {
            cmdline: None,
            bootloader: None,
            memory_regions: &[],
            rsdp: None,
            framebuffer_addr: None,
            kernel_load_addr: None,
        }
    }

    /// Set the command line
    #[must_use]
    pub const fn cmdline(mut self, cmdline: &'a str) -> Self {
        self.cmdline = Some(cmdline);
        self
    }

    /// Set the bootloader name
    #[must_use]
    pub const fn bootloader(mut self, name: &'a str) -> Self {
        self.bootloader = Some(name);
        self
    }

    /// Set memory regions
    #[must_use]
    pub const fn memory_regions(mut self, regions: &'a [MemoryRegion]) -> Self {
        self.memory_regions = regions;
        self
    }

    /// Set RSDP pointer
    #[must_use]
    pub const fn rsdp(mut self, ptr: *const u8) -> Self {
        self.rsdp = Some(ptr);
        self
    }

    /// Set framebuffer address
    #[must_use]
    pub const fn framebuffer_addr(mut self, addr: u64) -> Self {
        self.framebuffer_addr = Some(addr);
        self
    }

    /// Set kernel load address
    #[must_use]
    pub const fn kernel_load_addr(mut self, addr: u64) -> Self {
        self.kernel_load_addr = Some(addr);
        self
    }

    /// Build the boot info
    #[must_use]
    pub fn build(self) -> BootInfo<'a> {
        BootInfo {
            inner: BootInfoInner::Static(StaticBootInfo {
                cmdline: self.cmdline,
                bootloader: self.bootloader,
                memory_regions: self.memory_regions,
                rsdp: self.rsdp,
                framebuffer_addr: self.framebuffer_addr,
                kernel_load_addr: self.kernel_load_addr,
            }),
            memory_stats: None,
        }
    }
}

impl<'a> Default for BootInfoBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}
