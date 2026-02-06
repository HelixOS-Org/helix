//! # Memory Requests
//!
//! This module provides memory-related Limine requests including:
//! - Memory map (physical memory layout)
//! - HHDM (Higher Half Direct Map) offset
//! - Paging mode configuration

use super::{LimineRequest, ResponsePtr, SafeResponse};
use crate::protocol::magic::{
    LIMINE_MEMMAP_ACPI_NVS, LIMINE_MEMMAP_ACPI_RECLAIMABLE, LIMINE_MEMMAP_BAD_MEMORY,
    LIMINE_MEMMAP_BOOTLOADER_RECLAIMABLE, LIMINE_MEMMAP_FRAMEBUFFER,
    LIMINE_MEMMAP_KERNEL_AND_MODULES, LIMINE_MEMMAP_RESERVED, LIMINE_MEMMAP_USABLE,
    LIMINE_PAGING_MODE_X86_64_4LVL, LIMINE_PAGING_MODE_X86_64_5LVL,
};
use crate::protocol::raw::RawMemmapEntry;
use crate::protocol::request_ids::{HHDM_ID, MEMMAP_ID, PAGING_MODE_ID};

// =============================================================================
// Memory Map Request
// =============================================================================

/// Memory map request
///
/// Requests the physical memory map from the bootloader.
///
/// # Example
///
/// ```rust,no_run
/// use helix_limine::requests::MemoryMapRequest;
///
/// #[used]
/// #[link_section = ".limine_requests"]
/// static MEMMAP: MemoryMapRequest = MemoryMapRequest::new();
///
/// fn count_usable_memory() -> u64 {
///     if let Some(memmap) = MEMMAP.response() {
///         memmap
///             .entries()
///             .filter(|e| e.kind() == MemoryKind::Usable)
///             .map(|e| e.length())
///             .sum()
///     } else {
///         0
///     }
/// }
/// ```
#[repr(C)]
pub struct MemoryMapRequest {
    /// Request identifier
    id: [u64; 4],
    /// Protocol revision
    revision: u64,
    /// Response pointer
    response: ResponsePtr<MemoryMapResponse>,
}

impl MemoryMapRequest {
    /// Create a new memory map request
    pub const fn new() -> Self {
        Self {
            id: MEMMAP_ID,
            revision: 0,
            response: ResponsePtr::null(),
        }
    }

    /// Create with specific revision
    pub const fn with_revision(revision: u64) -> Self {
        Self {
            id: MEMMAP_ID,
            revision,
            response: ResponsePtr::null(),
        }
    }
}

impl Default for MemoryMapRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl LimineRequest for MemoryMapRequest {
    type Response = MemoryMapResponse;

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

unsafe impl Sync for MemoryMapRequest {}

/// Memory map response
#[repr(C)]
pub struct MemoryMapResponse {
    /// Response revision
    revision: u64,
    /// Entry count
    entry_count: u64,
    /// Entries pointer
    entries: *const *const RawMemmapEntry,
}

impl MemoryMapResponse {
    /// Get the response revision
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Get the number of memory map entries
    #[allow(clippy::cast_possible_truncation)] // entry_count is always small enough to fit in usize
    pub fn entry_count(&self) -> usize {
        self.entry_count as usize
    }

    /// Iterate over memory map entries
    pub fn entries(&self) -> MemoryMapIterator<'_> {
        MemoryMapIterator {
            response: self,
            index: 0,
        }
    }

    /// Get a specific entry by index
    pub fn get(&self, index: usize) -> Option<MemoryEntry> {
        if index >= self.entry_count() || self.entries.is_null() {
            return None;
        }

        // Safety: Bootloader guarantees valid entry pointers
        unsafe {
            let entry_ptr = *self.entries.add(index);
            if entry_ptr.is_null() {
                None
            } else {
                Some(MemoryEntry::from_raw(&*entry_ptr))
            }
        }
    }

    /// Calculate total usable memory
    pub fn total_usable_memory(&self) -> u64 {
        self.entries()
            .filter(|e| e.kind() == MemoryKind::Usable)
            .map(|e| e.length())
            .sum()
    }

    /// Calculate total memory (all types)
    pub fn total_memory(&self) -> u64 {
        self.entries().map(|e| e.length()).sum()
    }

    /// Find largest usable region
    pub fn largest_usable_region(&self) -> Option<MemoryEntry> {
        self.entries()
            .filter(|e| e.kind() == MemoryKind::Usable)
            .max_by_key(MemoryEntry::length)
    }

    /// Find a usable region of at least the given size
    pub fn find_usable_region(&self, min_size: u64) -> Option<MemoryEntry> {
        self.entries()
            .filter(|e| e.kind() == MemoryKind::Usable && e.length() >= min_size)
            .next()
    }

    /// Find a usable region at or above the given address
    pub fn find_usable_above(&self, min_addr: u64, min_size: u64) -> Option<MemoryEntry> {
        self.entries()
            .filter(|e| {
                e.kind() == MemoryKind::Usable && e.base() >= min_addr && e.length() >= min_size
            })
            .next()
    }
}

unsafe impl SafeResponse for MemoryMapResponse {
    fn validate(&self) -> bool {
        self.entry_count > 0 && !self.entries.is_null()
    }
}

impl core::fmt::Debug for MemoryMapResponse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MemoryMapResponse")
            .field("revision", &self.revision())
            .field("entry_count", &self.entry_count())
            .field("total_usable", &self.total_usable_memory())
            .finish()
    }
}

/// Iterator over memory map entries
pub struct MemoryMapIterator<'a> {
    response: &'a MemoryMapResponse,
    index: usize,
}

impl Iterator for MemoryMapIterator<'_> {
    type Item = MemoryEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.response.entry_count() {
            return None;
        }

        let entry = self.response.get(self.index)?;
        self.index += 1;
        Some(entry)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.response.entry_count() - self.index;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for MemoryMapIterator<'_> {}

/// A memory region entry
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryEntry {
    /// Base address
    base: u64,
    /// Length in bytes
    length: u64,
    /// Memory type
    kind: MemoryKind,
}

impl MemoryEntry {
    /// Create from raw entry
    fn from_raw(raw: &RawMemmapEntry) -> Self {
        Self {
            base: raw.base,
            length: raw.length,
            kind: MemoryKind::from_raw(raw.entry_type),
        }
    }

    /// Get the base address
    pub fn base(&self) -> u64 {
        self.base
    }

    /// Get the length in bytes
    pub fn length(&self) -> u64 {
        self.length
    }

    /// Get the end address (exclusive)
    pub fn end(&self) -> u64 {
        self.base.saturating_add(self.length)
    }

    /// Get the memory type
    pub fn kind(&self) -> MemoryKind {
        self.kind
    }

    /// Check if this region contains an address
    pub fn contains(&self, addr: u64) -> bool {
        addr >= self.base && addr < self.end()
    }

    /// Check if this region overlaps with another
    pub fn overlaps(&self, other: &MemoryEntry) -> bool {
        self.base < other.end() && other.base < self.end()
    }

    /// Check if this is usable memory
    pub fn is_usable(&self) -> bool {
        self.kind == MemoryKind::Usable
    }

    /// Check if this memory can be reclaimed
    pub fn is_reclaimable(&self) -> bool {
        matches!(
            self.kind,
            MemoryKind::BootloaderReclaimable | MemoryKind::AcpiReclaimable
        )
    }
}

impl core::fmt::Display for MemoryEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:#018x} - {:#018x} ({:?}, {} KB)",
            self.base,
            self.end(),
            self.kind,
            self.length / 1024
        )
    }
}

/// Memory region types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u64)]
pub enum MemoryKind {
    /// Usable RAM
    Usable           = LIMINE_MEMMAP_USABLE,
    /// Reserved by firmware
    Reserved         = LIMINE_MEMMAP_RESERVED,
    /// ACPI reclaimable memory
    AcpiReclaimable  = LIMINE_MEMMAP_ACPI_RECLAIMABLE,
    /// ACPI NVS memory
    AcpiNvs          = LIMINE_MEMMAP_ACPI_NVS,
    /// Bad memory
    BadMemory        = LIMINE_MEMMAP_BAD_MEMORY,
    /// Bootloader reclaimable memory
    BootloaderReclaimable = LIMINE_MEMMAP_BOOTLOADER_RECLAIMABLE,
    /// Kernel and modules
    KernelAndModules = LIMINE_MEMMAP_KERNEL_AND_MODULES,
    /// Framebuffer memory
    Framebuffer      = LIMINE_MEMMAP_FRAMEBUFFER,
    /// Unknown type
    Unknown          = 0xFFFF,
}

impl MemoryKind {
    /// Convert from raw type value
    pub fn from_raw(raw: u64) -> Self {
        match raw {
            LIMINE_MEMMAP_USABLE => Self::Usable,
            LIMINE_MEMMAP_RESERVED => Self::Reserved,
            LIMINE_MEMMAP_ACPI_RECLAIMABLE => Self::AcpiReclaimable,
            LIMINE_MEMMAP_ACPI_NVS => Self::AcpiNvs,
            LIMINE_MEMMAP_BAD_MEMORY => Self::BadMemory,
            LIMINE_MEMMAP_BOOTLOADER_RECLAIMABLE => Self::BootloaderReclaimable,
            LIMINE_MEMMAP_KERNEL_AND_MODULES => Self::KernelAndModules,
            LIMINE_MEMMAP_FRAMEBUFFER => Self::Framebuffer,
            _ => Self::Unknown,
        }
    }

    /// Convert to raw type value
    pub fn to_raw(self) -> u64 {
        self as u64
    }

    /// Get a human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Usable => "Usable",
            Self::Reserved => "Reserved",
            Self::AcpiReclaimable => "ACPI Reclaimable",
            Self::AcpiNvs => "ACPI NVS",
            Self::BadMemory => "Bad Memory",
            Self::BootloaderReclaimable => "Bootloader Reclaimable",
            Self::KernelAndModules => "Kernel and Modules",
            Self::Framebuffer => "Framebuffer",
            Self::Unknown => "Unknown",
        }
    }
}

// =============================================================================
// HHDM Request (Higher Half Direct Map)
// =============================================================================

/// HHDM (Higher Half Direct Map) request
///
/// The HHDM is a direct mapping of all physical memory at a high virtual
/// address. This allows easy conversion between physical and virtual addresses.
///
/// # Example
///
/// ```rust,no_run
/// use helix_limine::requests::HhdmRequest;
///
/// #[used]
/// #[link_section = ".limine_requests"]
/// static HHDM: HhdmRequest = HhdmRequest::new();
///
/// fn phys_to_virt(phys: u64) -> u64 {
///     if let Some(hhdm) = HHDM.response() {
///         phys + hhdm.offset()
///     } else {
///         panic!("HHDM not available")
///     }
/// }
/// ```
#[repr(C)]
pub struct HhdmRequest {
    /// Request identifier
    id: [u64; 4],
    /// Protocol revision
    revision: u64,
    /// Response pointer
    response: ResponsePtr<HhdmResponse>,
}

impl HhdmRequest {
    /// Create a new HHDM request
    pub const fn new() -> Self {
        Self {
            id: HHDM_ID,
            revision: 0,
            response: ResponsePtr::null(),
        }
    }
}

impl Default for HhdmRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl LimineRequest for HhdmRequest {
    type Response = HhdmResponse;

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

unsafe impl Sync for HhdmRequest {}

/// HHDM response
#[repr(C)]
pub struct HhdmResponse {
    /// Response revision
    revision: u64,
    /// HHDM offset (virtual address where physical 0 is mapped)
    offset: u64,
}

impl HhdmResponse {
    /// Get the HHDM offset
    ///
    /// Physical address 0 is mapped at this virtual address.
    /// To convert physical to virtual: `virt = phys + offset`
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// Get the response revision
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Convert a physical address to a virtual address
    pub fn phys_to_virt(&self, phys: u64) -> u64 {
        phys.wrapping_add(self.offset)
    }

    /// Convert a virtual address to a physical address
    ///
    /// This only works for addresses within the HHDM range.
    pub fn virt_to_phys(&self, virt: u64) -> Option<u64> {
        if virt >= self.offset {
            Some(virt - self.offset)
        } else {
            None
        }
    }

    /// Get a pointer to physical memory at the given address
    ///
    /// # Safety
    ///
    /// The caller must ensure the physical address is valid and
    /// the returned pointer is used correctly.
    pub unsafe fn phys_ptr<T>(&self, phys: u64) -> *const T {
        self.phys_to_virt(phys) as *const T
    }

    /// Get a mutable pointer to physical memory
    ///
    /// # Safety
    ///
    /// The caller must ensure the physical address is valid and
    /// the returned pointer is used correctly.
    pub unsafe fn phys_ptr_mut<T>(&self, phys: u64) -> *mut T {
        self.phys_to_virt(phys) as *mut T
    }
}

unsafe impl SafeResponse for HhdmResponse {
    fn validate(&self) -> bool {
        // HHDM offset should be in higher half (top 48 bits set for 4-level paging)
        self.offset >= 0xFFFF_8000_0000_0000
    }
}

impl core::fmt::Debug for HhdmResponse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("HhdmResponse")
            .field("offset", &format_args!("{:#018x}", self.offset))
            .field("revision", &self.revision)
            .finish()
    }
}

// =============================================================================
// Paging Mode Request
// =============================================================================

/// Paging mode request
///
/// Allows the kernel to specify a preferred paging mode.
///
/// # Paging Modes (`x86_64`)
///
/// - 4-level paging: 48-bit virtual addresses (256 `TiB`)
/// - 5-level paging: 57-bit virtual addresses (128 `PiB`)
///
/// # Example
///
/// ```rust,no_run
/// use helix_limine::requests::{PagingMode, PagingModeRequest};
///
/// #[used]
/// #[link_section = ".limine_requests"]
/// static PAGING: PagingModeRequest = PagingModeRequest::new()
///     .with_mode(PagingMode::FourLevel)
///     .with_max_mode(PagingMode::FiveLevel);
/// ```
#[repr(C)]
pub struct PagingModeRequest {
    /// Request identifier
    id: [u64; 4],
    /// Protocol revision
    revision: u64,
    /// Response pointer
    response: ResponsePtr<PagingModeResponse>,
    /// Preferred mode
    mode: u64,
    /// Minimum mode (0 = any)
    min_mode: u64,
    /// Maximum mode (0 = any)
    max_mode: u64,
}

impl PagingModeRequest {
    /// Create a new paging mode request with defaults
    pub const fn new() -> Self {
        Self {
            id: PAGING_MODE_ID,
            revision: 0,
            response: ResponsePtr::null(),
            mode: LIMINE_PAGING_MODE_X86_64_4LVL,
            min_mode: 0,
            max_mode: 0,
        }
    }

    /// Set the preferred paging mode
    #[must_use]
    pub const fn with_mode(mut self, mode: PagingMode) -> Self {
        self.mode = mode.to_raw();
        self
    }

    /// Set the minimum acceptable paging mode
    #[must_use]
    pub const fn with_min_mode(mut self, mode: PagingMode) -> Self {
        self.min_mode = mode.to_raw();
        self
    }

    /// Set the maximum acceptable paging mode
    #[must_use]
    pub const fn with_max_mode(mut self, mode: PagingMode) -> Self {
        self.max_mode = mode.to_raw();
        self
    }
}

impl Default for PagingModeRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl LimineRequest for PagingModeRequest {
    type Response = PagingModeResponse;

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

unsafe impl Sync for PagingModeRequest {}

/// Paging mode response
#[repr(C)]
pub struct PagingModeResponse {
    /// Response revision
    revision: u64,
    /// Actual mode used
    mode: u64,
}

impl PagingModeResponse {
    /// Get the actual paging mode used
    pub fn mode(&self) -> PagingMode {
        PagingMode::from_raw(self.mode)
    }

    /// Get the response revision
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Check if 5-level paging is enabled
    pub fn is_five_level(&self) -> bool {
        self.mode == LIMINE_PAGING_MODE_X86_64_5LVL
    }

    /// Check if 4-level paging is enabled
    pub fn is_four_level(&self) -> bool {
        self.mode == LIMINE_PAGING_MODE_X86_64_4LVL
    }

    /// Get the maximum virtual address width in bits
    pub fn virtual_address_width(&self) -> u8 {
        match self.mode() {
            PagingMode::FourLevel => 48,
            PagingMode::FiveLevel => 57,
            _ => 48,
        }
    }

    /// Get the maximum virtual address
    pub fn max_virtual_address(&self) -> u64 {
        match self.mode() {
            PagingMode::FourLevel => 0x0000_7FFF_FFFF_FFFF,
            PagingMode::FiveLevel => 0x00FF_FFFF_FFFF_FFFF,
            _ => 0x0000_7FFF_FFFF_FFFF,
        }
    }
}

unsafe impl SafeResponse for PagingModeResponse {
    fn validate(&self) -> bool {
        matches!(self.mode(), PagingMode::FourLevel | PagingMode::FiveLevel)
    }
}

impl core::fmt::Debug for PagingModeResponse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PagingModeResponse")
            .field("mode", &self.mode())
            .field("virtual_address_width", &self.virtual_address_width())
            .finish()
    }
}

/// Paging mode enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PagingMode {
    /// 4-level paging (48-bit virtual addresses)
    FourLevel,
    /// 5-level paging (57-bit virtual addresses)
    FiveLevel,
    /// Unknown paging mode
    Unknown(u64),
}

impl PagingMode {
    /// Create from raw value
    pub fn from_raw(raw: u64) -> Self {
        match raw {
            LIMINE_PAGING_MODE_X86_64_4LVL => Self::FourLevel,
            LIMINE_PAGING_MODE_X86_64_5LVL => Self::FiveLevel,
            other => Self::Unknown(other),
        }
    }

    /// Convert to raw value
    pub const fn to_raw(self) -> u64 {
        match self {
            Self::FourLevel => LIMINE_PAGING_MODE_X86_64_4LVL,
            Self::FiveLevel => LIMINE_PAGING_MODE_X86_64_5LVL,
            Self::Unknown(v) => v,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_kind_conversion() {
        assert_eq!(MemoryKind::from_raw(0), MemoryKind::Usable);
        assert_eq!(MemoryKind::from_raw(1), MemoryKind::Reserved);
        assert_eq!(MemoryKind::from_raw(999), MemoryKind::Unknown);
    }

    #[test]
    fn test_paging_mode_conversion() {
        assert_eq!(PagingMode::from_raw(0), PagingMode::FourLevel);
        assert_eq!(PagingMode::from_raw(1), PagingMode::FiveLevel);
    }

    #[test]
    fn test_memory_entry() {
        let entry = MemoryEntry {
            base: 0x1000,
            length: 0x2000,
            kind: MemoryKind::Usable,
        };

        assert!(entry.contains(0x1500));
        assert!(!entry.contains(0x500));
        assert!(entry.is_usable());
    }
}
