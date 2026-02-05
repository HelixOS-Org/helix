//! Raw Memory Types and Descriptors
//!
//! This module defines the low-level memory structures used by UEFI,
//! including memory types, descriptors, and attributes.

extern crate alloc;
use alloc::vec::Vec;
use core::fmt;

use super::types::{PhysicalAddress, VirtualAddress};

// =============================================================================
// MEMORY TYPE
// =============================================================================

/// UEFI Memory Type
///
/// Describes the type/purpose of a memory region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MemoryType {
    /// Reserved memory (unusable)
    ReservedMemory      = 0,
    /// Loader code
    LoaderCode          = 1,
    /// Loader data
    LoaderData          = 2,
    /// Boot services code (reclaimable after `ExitBootServices`)
    BootServicesCode    = 3,
    /// Boot services data (reclaimable after `ExitBootServices`)
    BootServicesData    = 4,
    /// Runtime services code (must be preserved)
    RuntimeServicesCode = 5,
    /// Runtime services data (must be preserved)
    RuntimeServicesData = 6,
    /// Conventional (free) memory
    ConventionalMemory  = 7,
    /// Unusable memory (errors detected)
    UnusableMemory      = 8,
    /// ACPI reclaim memory (can be freed after parsing ACPI tables)
    AcpiReclaimMemory   = 9,
    /// ACPI NVS memory (must be preserved)
    AcpiNvsMemory       = 10,
    /// Memory-mapped I/O
    MemoryMappedIo      = 11,
    /// Memory-mapped I/O port space
    MemoryMappedIoPortSpace = 12,
    /// Processor reserved (PAL code on Itanium)
    PalCode             = 13,
    /// Persistent memory (NVDIMM)
    PersistentMemory    = 14,
    /// Unaccepted memory (TDX, SEV-SNP)
    UnacceptedMemory    = 15,
}

impl MemoryType {
    /// Alias for `ReservedMemory` (backwards compatibility)
    #[doc(hidden)]
    #[deprecated(note = "Use MemoryType::ReservedMemory instead")]
    pub const RESERVED_MEMORY_TYPE: Self = Self::ReservedMemory;
    /// Alias for `AcpiNvsMemory` (backwards compatibility)
    #[doc(hidden)]
    #[deprecated(note = "Use MemoryType::AcpiNvsMemory instead")]
    pub const ACPI_MEMORY_NVS: Self = Self::AcpiNvsMemory;

    /// Check if this memory type is usable after `ExitBootServices`
    pub const fn is_usable_after_exit(self) -> bool {
        matches!(
            self,
            Self::LoaderCode
                | Self::LoaderData
                | Self::BootServicesCode
                | Self::BootServicesData
                | Self::ConventionalMemory
                | Self::AcpiReclaimMemory
                | Self::PersistentMemory
        )
    }

    /// Check if this memory type must be preserved
    pub const fn must_preserve(self) -> bool {
        matches!(
            self,
            Self::RuntimeServicesCode
                | Self::RuntimeServicesData
                | Self::AcpiNvsMemory
                | Self::ReservedMemory
                | Self::UnusableMemory
                | Self::MemoryMappedIo
                | Self::MemoryMappedIoPortSpace
                | Self::PalCode
        )
    }

    /// Check if this is conventional (free) memory
    pub const fn is_conventional(self) -> bool {
        matches!(self, Self::ConventionalMemory)
    }

    /// Check if this is boot services memory
    pub const fn is_boot_services(self) -> bool {
        matches!(self, Self::BootServicesCode | Self::BootServicesData)
    }

    /// Check if this is runtime services memory
    pub const fn is_runtime_services(self) -> bool {
        matches!(self, Self::RuntimeServicesCode | Self::RuntimeServicesData)
    }

    /// Check if this is loader memory
    pub const fn is_loader(self) -> bool {
        matches!(self, Self::LoaderCode | Self::LoaderData)
    }

    /// Check if this is ACPI memory
    pub const fn is_acpi(self) -> bool {
        matches!(self, Self::AcpiReclaimMemory | Self::AcpiNvsMemory)
    }

    /// Get the name of this memory type
    pub const fn name(self) -> &'static str {
        match self {
            Self::ReservedMemory => "Reserved",
            Self::LoaderCode => "LoaderCode",
            Self::LoaderData => "LoaderData",
            Self::BootServicesCode => "BootServicesCode",
            Self::BootServicesData => "BootServicesData",
            Self::RuntimeServicesCode => "RuntimeServicesCode",
            Self::RuntimeServicesData => "RuntimeServicesData",
            Self::ConventionalMemory => "Conventional",
            Self::UnusableMemory => "Unusable",
            Self::AcpiReclaimMemory => "ACPIReclaim",
            Self::AcpiNvsMemory => "ACPINVS",
            Self::MemoryMappedIo => "MMIO",
            Self::MemoryMappedIoPortSpace => "MMIOPortSpace",
            Self::PalCode => "PALCode",
            Self::PersistentMemory => "Persistent",
            Self::UnacceptedMemory => "Unaccepted",
        }
    }

    /// Convert from raw u32 value
    pub const fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::ReservedMemory),
            1 => Some(Self::LoaderCode),
            2 => Some(Self::LoaderData),
            3 => Some(Self::BootServicesCode),
            4 => Some(Self::BootServicesData),
            5 => Some(Self::RuntimeServicesCode),
            6 => Some(Self::RuntimeServicesData),
            7 => Some(Self::ConventionalMemory),
            8 => Some(Self::UnusableMemory),
            9 => Some(Self::AcpiReclaimMemory),
            10 => Some(Self::AcpiNvsMemory),
            11 => Some(Self::MemoryMappedIo),
            12 => Some(Self::MemoryMappedIoPortSpace),
            13 => Some(Self::PalCode),
            14 => Some(Self::PersistentMemory),
            15 => Some(Self::UnacceptedMemory),
            _ => None,
        }
    }
}

impl fmt::Display for MemoryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// =============================================================================
// MEMORY ATTRIBUTES
// =============================================================================

/// Memory attributes bitflags
#[derive(Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct MemoryAttribute(pub u64);

impl MemoryAttribute {
    /// Memory is uncacheable
    pub const UC: Self = Self(0x0000_0000_0000_0001);
    /// Memory supports write combining
    pub const WC: Self = Self(0x0000_0000_0000_0002);
    /// Memory supports write-through caching
    pub const WT: Self = Self(0x0000_0000_0000_0004);
    /// Memory supports write-back caching
    pub const WB: Self = Self(0x0000_0000_0000_0008);
    /// Memory is uncacheable and exported (for firmware communication)
    pub const UCE: Self = Self(0x0000_0000_0000_0010);
    /// Memory is write-protected
    pub const WP: Self = Self(0x0000_0000_0000_1000);
    /// Memory is read-protected
    pub const RP: Self = Self(0x0000_0000_0000_2000);
    /// Memory is not executable
    pub const XP: Self = Self(0x0000_0000_0000_4000);
    /// Memory supports non-volatile storage
    pub const NV: Self = Self(0x0000_0000_0000_8000);
    /// Memory is more reliable than other memory
    pub const MORE_RELIABLE: Self = Self(0x0000_0000_0001_0000);
    /// Memory is read-only
    pub const RO: Self = Self(0x0000_0000_0002_0000);
    /// Memory is a specific purpose memory (SPM)
    pub const SP: Self = Self(0x0000_0000_0004_0000);
    /// Memory requires CPU-specific instructions to access
    pub const CPU_CRYPTO: Self = Self(0x0000_0000_0008_0000);
    /// Memory needs to be runtime mapped
    pub const RUNTIME: Self = Self(0x8000_0000_0000_0000);

    /// Empty attribute set
    pub const NONE: Self = Self(0);

    /// Default cacheable memory (write-back)
    pub const CACHEABLE: Self = Self(Self::WB.0);

    /// Create from raw value
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Get the raw bits
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Check if the attribute contains all of the given flags
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Check if the attribute contains any of the given flags
    pub const fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    /// Combine with another attribute
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Intersect with another attribute
    #[must_use]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Remove flags
    #[must_use]
    pub const fn difference(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    /// Check if memory is cacheable
    pub const fn is_cacheable(self) -> bool {
        self.intersects(Self::WC) || self.intersects(Self::WT) || self.intersects(Self::WB)
    }

    /// Check if memory is executable
    pub const fn is_executable(self) -> bool {
        !self.contains(Self::XP)
    }

    /// Check if memory is writable
    pub const fn is_writable(self) -> bool {
        !self.contains(Self::RO) && !self.contains(Self::WP)
    }

    /// Check if memory is runtime (needs virtual address mapping)
    pub const fn is_runtime(self) -> bool {
        self.contains(Self::RUNTIME)
    }

    /// Get cache type name
    pub const fn cache_type(&self) -> &'static str {
        if self.contains(Self::UC) || self.contains(Self::UCE) {
            "UC"
        } else if self.contains(Self::WC) {
            "WC"
        } else if self.contains(Self::WT) {
            "WT"
        } else if self.contains(Self::WB) {
            "WB"
        } else {
            "Unknown"
        }
    }
}

impl fmt::Debug for MemoryAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut flags = Vec::new();

        if self.contains(Self::UC) {
            flags.push("UC");
        }
        if self.contains(Self::WC) {
            flags.push("WC");
        }
        if self.contains(Self::WT) {
            flags.push("WT");
        }
        if self.contains(Self::WB) {
            flags.push("WB");
        }
        if self.contains(Self::UCE) {
            flags.push("UCE");
        }
        if self.contains(Self::WP) {
            flags.push("WP");
        }
        if self.contains(Self::RP) {
            flags.push("RP");
        }
        if self.contains(Self::XP) {
            flags.push("XP");
        }
        if self.contains(Self::NV) {
            flags.push("NV");
        }
        if self.contains(Self::MORE_RELIABLE) {
            flags.push("MORE_RELIABLE");
        }
        if self.contains(Self::RO) {
            flags.push("RO");
        }
        if self.contains(Self::SP) {
            flags.push("SP");
        }
        if self.contains(Self::CPU_CRYPTO) {
            flags.push("CPU_CRYPTO");
        }
        if self.contains(Self::RUNTIME) {
            flags.push("RUNTIME");
        }

        if flags.is_empty() {
            write!(f, "MemoryAttribute(NONE)")
        } else {
            write!(f, "MemoryAttribute({})", flags.join(" | "))
        }
    }
}

impl core::ops::BitOr for MemoryAttribute {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

impl core::ops::BitAnd for MemoryAttribute {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        self.intersection(rhs)
    }
}

impl core::ops::BitOrAssign for MemoryAttribute {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = self.union(rhs);
    }
}

impl core::ops::Not for MemoryAttribute {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

// =============================================================================
// MEMORY DESCRIPTOR
// =============================================================================

/// UEFI Memory Descriptor
///
/// Describes a region of memory in the system memory map.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct MemoryDescriptor {
    /// Type of memory region
    pub memory_type: u32,
    /// Physical address of the first byte
    pub physical_start: PhysicalAddress,
    /// Virtual address of the first byte (after `SetVirtualAddressMap`)
    pub virtual_start: VirtualAddress,
    /// Number of 4KB pages in the region
    pub number_of_pages: u64,
    /// Memory attributes
    pub attribute: MemoryAttribute,
}

impl MemoryDescriptor {
    /// Page size (4KB)
    pub const PAGE_SIZE: u64 = 4096;

    /// Create a new memory descriptor
    pub const fn new(
        memory_type: MemoryType,
        physical_start: PhysicalAddress,
        number_of_pages: u64,
        attribute: MemoryAttribute,
    ) -> Self {
        Self {
            memory_type: memory_type as u32,
            physical_start,
            virtual_start: VirtualAddress::NULL,
            number_of_pages,
            attribute,
        }
    }

    /// Get the memory type
    pub const fn get_type(&self) -> Option<MemoryType> {
        MemoryType::from_u32(self.memory_type)
    }

    /// Get the size in bytes
    pub const fn size(&self) -> u64 {
        self.number_of_pages * Self::PAGE_SIZE
    }

    /// Get the end physical address (exclusive)
    pub const fn physical_end(&self) -> PhysicalAddress {
        PhysicalAddress::new(self.physical_start.as_u64() + self.size())
    }

    /// Check if this region contains an address
    pub const fn contains(&self, addr: PhysicalAddress) -> bool {
        addr.as_u64() >= self.physical_start.as_u64()
            && addr.as_u64() < self.physical_end().as_u64()
    }

    /// Check if this region overlaps with another
    pub const fn overlaps(&self, other: &Self) -> bool {
        self.physical_start.as_u64() < other.physical_end().as_u64()
            && other.physical_start.as_u64() < self.physical_end().as_u64()
    }

    /// Check if this memory is usable after `ExitBootServices`
    pub fn is_usable(&self) -> bool {
        self.get_type()
            .is_some_and(MemoryType::is_usable_after_exit)
    }

    /// Check if this is conventional memory
    pub fn is_conventional(&self) -> bool {
        self.get_type().is_some_and(MemoryType::is_conventional)
    }

    /// Check if this memory requires runtime mapping
    pub const fn requires_runtime_mapping(&self) -> bool {
        self.attribute.is_runtime()
    }
}

impl fmt::Debug for MemoryDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let type_name = self.get_type().map_or("Unknown", MemoryType::name);

        f.debug_struct("MemoryDescriptor")
            .field("type", &type_name)
            .field("physical_start", &format_args!("{}", self.physical_start))
            .field("virtual_start", &format_args!("{}", self.virtual_start))
            .field("pages", &self.number_of_pages)
            .field("size", &format_args!("0x{:X}", self.size()))
            .field("attribute", &self.attribute)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for MemoryDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let type_name = self.get_type().map_or("Unknown", MemoryType::name);
        let physical_start = self.physical_start;
        let physical_end = self.physical_end();
        let size = self.size();
        let cache_type = self.attribute.cache_type();

        write!(
            f,
            "{physical_start} - {physical_end} ({size:>12} bytes) [{type_name}] {cache_type:?}"
        )
    }
}

// =============================================================================
// MEMORY MAP
// =============================================================================

/// Memory map key for `ExitBootServices`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct MemoryMapKey(pub usize);

impl MemoryMapKey {
    /// Create a new memory map key
    pub const fn new(key: usize) -> Self {
        Self(key)
    }

    /// Create default (zero) key
    pub const fn default() -> Self {
        Self(0)
    }
}

/// Memory map iterator
pub struct MemoryMapIter<'a> {
    ptr: *const u8,
    end: *const u8,
    descriptor_size: usize,
    _marker: core::marker::PhantomData<&'a MemoryDescriptor>,
}

impl MemoryMapIter<'_> {
    /// Create a new memory map iterator
    ///
    /// # Safety
    /// The caller must ensure the pointer and size are valid.
    pub const unsafe fn new(ptr: *const u8, size: usize, descriptor_size: usize) -> Self {
        Self {
            ptr,
            end: ptr.add(size),
            descriptor_size,
            _marker: core::marker::PhantomData,
        }
    }

    /// Get the number of entries
    pub fn count(&self) -> usize {
        let total_size = self.end as usize - self.ptr as usize;
        total_size / self.descriptor_size
    }
}

impl Iterator for MemoryMapIter<'_> {
    type Item = MemoryDescriptor;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ptr >= self.end {
            return None;
        }

        let descriptor = unsafe { core::ptr::read_unaligned(self.ptr.cast::<MemoryDescriptor>()) };
        self.ptr = unsafe { self.ptr.add(self.descriptor_size) };

        Some(descriptor)
    }
}

/// Owned memory map
pub struct MemoryMap {
    /// Buffer containing the memory descriptors
    buffer: *mut u8,
    /// Size of the buffer
    _buffer_size: usize,
    /// Actual size of the memory map
    map_size: usize,
    /// Size of each descriptor
    descriptor_size: usize,
    /// Descriptor version
    descriptor_version: u32,
    /// Memory map key
    key: MemoryMapKey,
}

impl MemoryMap {
    /// Create a new memory map
    ///
    /// # Safety
    /// The buffer must be valid and contain a valid memory map.
    pub const unsafe fn new(
        buffer: *mut u8,
        buffer_size: usize,
        map_size: usize,
        descriptor_size: usize,
        descriptor_version: u32,
        key: MemoryMapKey,
    ) -> Self {
        Self {
            buffer,
            _buffer_size: buffer_size,
            map_size,
            descriptor_size,
            descriptor_version,
            key,
        }
    }

    /// Get the memory map key
    pub const fn key(&self) -> MemoryMapKey {
        self.key
    }

    /// Get the descriptor size
    pub const fn descriptor_size(&self) -> usize {
        self.descriptor_size
    }

    /// Get the descriptor version
    pub const fn descriptor_version(&self) -> u32 {
        self.descriptor_version
    }

    /// Get the number of entries
    pub const fn entry_count(&self) -> usize {
        self.map_size / self.descriptor_size
    }

    /// Get an iterator over the memory descriptors
    pub const fn iter(&self) -> MemoryMapIter<'_> {
        unsafe { MemoryMapIter::new(self.buffer, self.map_size, self.descriptor_size) }
    }

    /// Find memory descriptors by type
    pub fn find_by_type(
        &self,
        memory_type: MemoryType,
    ) -> impl Iterator<Item = MemoryDescriptor> + '_ {
        self.iter()
            .filter(move |d| d.get_type() == Some(memory_type))
    }

    /// Get total usable memory
    pub fn total_usable_memory(&self) -> u64 {
        self.iter()
            .filter(MemoryDescriptor::is_usable)
            .map(|d| d.size())
            .sum()
    }

    /// Get total conventional memory
    pub fn total_conventional_memory(&self) -> u64 {
        self.iter()
            .filter(MemoryDescriptor::is_conventional)
            .map(|d| d.size())
            .sum()
    }

    /// Find the largest conventional memory region
    pub fn largest_conventional_region(&self) -> Option<MemoryDescriptor> {
        self.iter()
            .filter(MemoryDescriptor::is_conventional)
            .max_by_key(MemoryDescriptor::size)
    }

    /// Get memory at or above a specific address
    pub fn memory_above(&self, addr: PhysicalAddress) -> impl Iterator<Item = MemoryDescriptor> + '_ {
        self.iter()
            .filter(move |d| d.physical_start.as_u64() >= addr.as_u64())
    }

    /// Get memory below a specific address
    pub fn memory_below(&self, addr: PhysicalAddress) -> impl Iterator<Item = MemoryDescriptor> + '_ {
        self.iter()
            .filter(move |d| d.physical_end().as_u64() <= addr.as_u64())
    }

    /// Find a suitable region for allocation
    pub fn find_suitable_region(&self, size: u64, alignment: u64) -> Option<PhysicalAddress> {
        let pages_needed = size.div_ceil(MemoryDescriptor::PAGE_SIZE);

        for desc in self {
            if !desc.is_conventional() {
                continue;
            }

            if desc.number_of_pages < pages_needed {
                continue;
            }

            let aligned_start = desc.physical_start.align_up(alignment);
            let aligned_end = PhysicalAddress::new(aligned_start.as_u64() + size);

            if aligned_end.as_u64() <= desc.physical_end().as_u64() {
                return Some(aligned_start);
            }
        }

        None
    }

    /// Print the memory map for debugging
    pub fn print(&self) {
        for (i, desc) in self.into_iter().enumerate() {
            // In real code, this would use a proper logging mechanism
            let _ = (i, desc);
        }
    }
}
impl<'a> IntoIterator for &'a MemoryMap {
    type Item = MemoryDescriptor;
    type IntoIter = MemoryMapIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// =============================================================================
// MEMORY STATISTICS
// =============================================================================

/// Memory statistics from the memory map
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryStats {
    /// Total physical memory
    pub total_physical: u64,
    /// Total usable memory (after `ExitBootServices`)
    pub total_usable: u64,
    /// Total conventional memory
    pub total_conventional: u64,
    /// Total boot services memory
    pub total_boot_services: u64,
    /// Total runtime services memory
    pub total_runtime_services: u64,
    /// Total ACPI memory
    pub total_acpi: u64,
    /// Total reserved/unusable memory
    pub total_reserved: u64,
    /// Total MMIO memory
    pub total_mmio: u64,
    /// Number of memory regions
    pub region_count: usize,
    /// Largest contiguous conventional region
    pub largest_conventional: u64,
    /// Highest physical address
    pub highest_address: PhysicalAddress,
}

impl MemoryStats {
    /// Calculate statistics from a memory map
    pub fn from_memory_map(map: &MemoryMap) -> Self {
        let mut stats = Self::default();
        let mut largest_conventional = 0u64;
        let mut highest = 0u64;

        for desc in map {
            stats.region_count += 1;

            let size = desc.size();
            let end = desc.physical_end().as_u64();

            if end > highest {
                highest = end;
            }

            if let Some(mem_type) = desc.get_type() {
                stats.total_physical += size;

                if mem_type.is_usable_after_exit() {
                    stats.total_usable += size;
                }

                if mem_type.is_conventional() {
                    stats.total_conventional += size;
                    if size > largest_conventional {
                        largest_conventional = size;
                    }
                }

                if mem_type.is_boot_services() {
                    stats.total_boot_services += size;
                }

                if mem_type.is_runtime_services() {
                    stats.total_runtime_services += size;
                }

                if mem_type.is_acpi() {
                    stats.total_acpi += size;
                }

                match mem_type {
                    MemoryType::ReservedMemory | MemoryType::UnusableMemory => {
                        stats.total_reserved += size;
                    },
                    MemoryType::MemoryMappedIo | MemoryType::MemoryMappedIoPortSpace => {
                        stats.total_mmio += size;
                    },
                    _ => {},
                }
            }
        }

        stats.largest_conventional = largest_conventional;
        stats.highest_address = PhysicalAddress::new(highest);

        stats
    }

    /// Format memory size in human-readable form
    pub fn format_size(size: u64) -> ([u8; 16], usize) {
        let mut buf = [0u8; 16];
        let units = ["B", "KiB", "MiB", "GiB", "TiB"];

        let mut value = size;
        let mut unit_idx = 0;

        while value >= 1024 && unit_idx < units.len() - 1 {
            value /= 1024;
            unit_idx += 1;
        }

        // Simple formatting (no_std compatible)
        let len = format_u64(value, &mut buf);

        buf[len] = b' ';
        let unit = units[unit_idx].as_bytes();
        buf[len + 1..len + 1 + unit.len()].copy_from_slice(unit);

        (buf, len + 1 + unit.len())
    }
}

/// Format a u64 into a buffer
fn format_u64(mut value: u64, buf: &mut [u8]) -> usize {
    if value == 0 {
        buf[0] = b'0';
        return 1;
    }

    let mut digits = [0u8; 20];
    let mut len = 0;

    while value > 0 {
        digits[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }

    for i in 0..len {
        buf[i] = digits[len - 1 - i];
    }

    len
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_memory_type_properties() {
        assert!(MemoryType::ConventionalMemory.is_conventional());
        assert!(MemoryType::ConventionalMemory.is_usable_after_exit());
        assert!(!MemoryType::RuntimeServicesCode.is_usable_after_exit());
        assert!(MemoryType::RuntimeServicesCode.must_preserve());
    }

    #[test]
    fn test_memory_attribute_operations() {
        let attr = MemoryAttribute::WB | MemoryAttribute::RUNTIME;
        assert!(attr.contains(MemoryAttribute::WB));
        assert!(attr.contains(MemoryAttribute::RUNTIME));
        assert!(!attr.contains(MemoryAttribute::XP));
        assert!(attr.is_cacheable());
        assert!(attr.is_runtime());
    }

    #[test]
    fn test_memory_descriptor_size() {
        let desc = MemoryDescriptor::new(
            MemoryType::ConventionalMemory,
            PhysicalAddress::new(0x100000),
            256, // 1 MiB
            MemoryAttribute::WB,
        );
        assert_eq!(desc.size(), 256 * 4096);
        assert_eq!(desc.physical_end().as_u64(), 0x100000 + 256 * 4096);
    }

    #[test]
    fn test_memory_descriptor_contains() {
        let desc = MemoryDescriptor::new(
            MemoryType::ConventionalMemory,
            PhysicalAddress::new(0x100000),
            256,
            MemoryAttribute::WB,
        );

        assert!(desc.contains(PhysicalAddress::new(0x100000)));
        assert!(desc.contains(PhysicalAddress::new(0x150000)));
        assert!(!desc.contains(PhysicalAddress::new(0x200000)));
    }
}
