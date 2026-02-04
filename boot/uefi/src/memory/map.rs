//! Memory Map Management
//!
//! Complete memory map handling and analysis.

use crate::raw::memory::{MemoryAttribute, MemoryDescriptor, MemoryType};
use crate::raw::types::*;

extern crate alloc;
use alloc::vec::Vec;

// =============================================================================
// MEMORY MAP INFO
// =============================================================================

/// Memory map information
pub struct MemoryMapInfo {
    /// Map key
    map_key: usize,
    /// Descriptors
    descriptors: Vec<MemoryDescriptor>,
    /// Descriptor size
    descriptor_size: usize,
    /// Descriptor version
    descriptor_version: u32,
}

impl MemoryMapInfo {
    /// Create new memory map info
    pub fn new(
        map_key: usize,
        descriptors: Vec<MemoryDescriptor>,
        descriptor_size: usize,
        descriptor_version: u32,
    ) -> Self {
        Self {
            map_key,
            descriptors,
            descriptor_size,
            descriptor_version,
        }
    }

    /// Get map key
    pub fn map_key(&self) -> usize {
        self.map_key
    }

    /// Get descriptors
    pub fn descriptors(&self) -> &[MemoryDescriptor] {
        &self.descriptors
    }

    /// Get descriptor count
    pub fn descriptor_count(&self) -> usize {
        self.descriptors.len()
    }

    /// Get descriptor size
    pub fn descriptor_size(&self) -> usize {
        self.descriptor_size
    }

    /// Get descriptor version
    pub fn descriptor_version(&self) -> u32 {
        self.descriptor_version
    }

    /// Iterate over descriptors
    pub fn iter(&self) -> MemoryMapIterator<'_> {
        MemoryMapIterator::new(&self.descriptors)
    }

    /// Get usable memory entries
    pub fn usable_entries(&self) -> impl Iterator<Item = &MemoryDescriptor> {
        self.descriptors
            .iter()
            .filter(|d| d.memory_type == MemoryType::ConventionalMemory as u32 as u32)
    }

    /// Get total usable memory
    pub fn total_usable(&self) -> u64 {
        self.usable_entries()
            .map(|d| d.number_of_pages * 4096)
            .sum()
    }

    /// Get total memory
    pub fn total_memory(&self) -> u64 {
        self.descriptors
            .iter()
            .map(|d| d.number_of_pages * 4096)
            .sum()
    }

    /// Find descriptor containing address
    pub fn find_by_address(&self, address: PhysicalAddress) -> Option<&MemoryDescriptor> {
        self.descriptors.iter().find(|d| {
            let end = d.physical_start + d.number_of_pages * 4096;
            address >= d.physical_start && address < end
        })
    }

    /// Find largest usable region
    pub fn find_largest_usable(&self) -> Option<&MemoryDescriptor> {
        self.usable_entries().max_by_key(|d| d.number_of_pages)
    }

    /// Get runtime services entries
    pub fn runtime_entries(&self) -> impl Iterator<Item = &MemoryDescriptor> {
        self.descriptors
            .iter()
            .filter(|d| d.attribute.contains(MemoryAttribute::RUNTIME))
    }

    /// Sort descriptors by physical address
    pub fn sort_by_address(&mut self) {
        self.descriptors.sort_by_key(|d| d.physical_start);
    }

    /// Merge contiguous regions of same type
    pub fn merge_contiguous(&mut self) {
        if self.descriptors.len() < 2 {
            return;
        }

        self.sort_by_address();

        let mut merged = Vec::with_capacity(self.descriptors.len());
        let mut current = self.descriptors[0].clone();

        for desc in &self.descriptors[1..] {
            let current_end = current.physical_start + current.number_of_pages * 4096;

            // Check if contiguous and same type
            if desc.physical_start == current_end
                && desc.memory_type == current.memory_type
                && desc.attribute == current.attribute
            {
                // Merge
                current.number_of_pages += desc.number_of_pages;
            } else {
                merged.push(current);
                current = desc.clone();
            }
        }
        merged.push(current);

        self.descriptors = merged;
    }

    /// Get memory below 4GB
    pub fn memory_below_4gb(&self) -> u64 {
        self.descriptors
            .iter()
            .filter(|d| d.physical_start.0 < 0x1_0000_0000)
            .map(|d| {
                let end = d.physical_start.0 + d.number_of_pages * 4096;
                if end <= 0x1_0000_0000 {
                    d.number_of_pages * 4096
                } else {
                    0x1_0000_0000 - d.physical_start.0
                }
            })
            .sum()
    }

    /// Get memory above 4GB
    pub fn memory_above_4gb(&self) -> u64 {
        self.descriptors
            .iter()
            .filter(|d| d.physical_start.0 + d.number_of_pages * 4096 > 0x1_0000_0000)
            .map(|d| {
                if d.physical_start.0 >= 0x1_0000_0000 {
                    d.number_of_pages * 4096
                } else {
                    let end = d.physical_start.0 + d.number_of_pages * 4096;
                    end - 0x1_0000_0000
                }
            })
            .sum()
    }
}

// =============================================================================
// MEMORY MAP ITERATOR
// =============================================================================

/// Iterator over memory map entries
pub struct MemoryMapIterator<'a> {
    descriptors: &'a [MemoryDescriptor],
    index: usize,
}

impl<'a> MemoryMapIterator<'a> {
    /// Create new iterator
    pub fn new(descriptors: &'a [MemoryDescriptor]) -> Self {
        Self {
            descriptors,
            index: 0,
        }
    }
}

impl<'a> Iterator for MemoryMapIterator<'a> {
    type Item = &'a MemoryDescriptor;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.descriptors.len() {
            let desc = &self.descriptors[self.index];
            self.index += 1;
            Some(desc)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.descriptors.len() - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for MemoryMapIterator<'a> {}

// =============================================================================
// MEMORY RANGE
// =============================================================================

/// Memory range description
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryRange {
    /// Start address
    pub start: PhysicalAddress,
    /// End address (exclusive)
    pub end: PhysicalAddress,
    /// Memory type
    pub memory_type: RangeType,
}

impl MemoryRange {
    /// Create new range
    pub fn new(start: PhysicalAddress, end: PhysicalAddress, memory_type: RangeType) -> Self {
        Self {
            start,
            end,
            memory_type,
        }
    }

    /// Get size
    pub fn size(&self) -> u64 {
        self.end - self.start
    }

    /// Check if contains address
    pub fn contains(&self, address: PhysicalAddress) -> bool {
        address >= self.start && address < self.end
    }

    /// Check if overlaps with another range
    pub fn overlaps(&self, other: &MemoryRange) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// Get intersection with another range
    pub fn intersection(&self, other: &MemoryRange) -> Option<MemoryRange> {
        if !self.overlaps(other) {
            return None;
        }

        Some(MemoryRange {
            start: self.start.max(other.start),
            end: self.end.min(other.end),
            memory_type: self.memory_type,
        })
    }
}

impl PartialOrd for MemoryRange {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MemoryRange {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.start.cmp(&other.start)
    }
}

/// Range type (simplified from UEFI memory types)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeType {
    /// Usable RAM
    Usable,
    /// Reserved (unusable)
    Reserved,
    /// ACPI reclaimable
    AcpiReclaimable,
    /// ACPI NVS
    AcpiNvs,
    /// Bad memory
    BadMemory,
    /// Loader code/data
    Loader,
    /// Boot services
    BootServices,
    /// Runtime services
    RuntimeServices,
    /// Memory mapped I/O
    Mmio,
    /// Unknown
    Unknown,
}

impl From<MemoryType> for RangeType {
    fn from(mt: MemoryType) -> Self {
        match mt {
            MemoryType::ConventionalMemory => Self::Usable,
            MemoryType::ReservedMemoryType => Self::Reserved,
            MemoryType::UnusableMemory => Self::BadMemory,
            MemoryType::AcpiReclaimMemory => Self::AcpiReclaimable,
            MemoryType::AcpiMemoryNvs => Self::AcpiNvs,
            MemoryType::MemoryMappedIo | MemoryType::MemoryMappedIoPortSpace => Self::Mmio,
            MemoryType::LoaderCode | MemoryType::LoaderData => Self::Loader,
            MemoryType::BootServicesCode | MemoryType::BootServicesData => Self::BootServices,
            MemoryType::RuntimeServicesCode | MemoryType::RuntimeServicesData => {
                Self::RuntimeServices
            },
            _ => Self::Unknown,
        }
    }
}

// =============================================================================
// MEMORY LAYOUT
// =============================================================================

/// Standard memory layout constants
pub mod layout {
    use super::*;

    /// Real mode IVT
    pub const IVT_START: PhysicalAddress = PhysicalAddress(0x0000);
    pub const IVT_END: PhysicalAddress = PhysicalAddress(0x0400);

    /// BIOS Data Area
    pub const BDA_START: PhysicalAddress = PhysicalAddress(0x0400);
    pub const BDA_END: PhysicalAddress = PhysicalAddress(0x0500);

    /// Conventional memory start
    pub const CONVENTIONAL_START: PhysicalAddress = PhysicalAddress(0x0500);

    /// Extended BIOS Data Area (approximate)
    pub const EBDA_START: PhysicalAddress = PhysicalAddress(0x9FC00);
    pub const EBDA_END: PhysicalAddress = PhysicalAddress(0xA0000);

    /// VGA memory
    pub const VGA_START: PhysicalAddress = PhysicalAddress(0xA0000);
    pub const VGA_END: PhysicalAddress = PhysicalAddress(0xC0000);

    /// Video BIOS
    pub const VIDEO_BIOS_START: PhysicalAddress = PhysicalAddress(0xC0000);
    pub const VIDEO_BIOS_END: PhysicalAddress = PhysicalAddress(0xC8000);

    /// Option ROMs
    pub const OPTION_ROM_START: PhysicalAddress = PhysicalAddress(0xC8000);
    pub const OPTION_ROM_END: PhysicalAddress = PhysicalAddress(0xE0000);

    /// System BIOS
    pub const SYSTEM_BIOS_START: PhysicalAddress = PhysicalAddress(0xF0000);
    pub const SYSTEM_BIOS_END: PhysicalAddress = PhysicalAddress(0x100000);

    /// First megabyte
    pub const FIRST_MB: PhysicalAddress = PhysicalAddress(0x100000);

    /// ISA memory hole start
    pub const ISA_HOLE_START: PhysicalAddress = PhysicalAddress(0x00F00000);

    /// 16MB boundary
    pub const MB_16: PhysicalAddress = PhysicalAddress(0x01000000);

    /// 4GB boundary
    pub const GB_4: PhysicalAddress = PhysicalAddress(0x100000000);

    /// Check if address is in low memory (<1MB)
    pub const fn is_low_memory(addr: PhysicalAddress) -> bool {
        addr.0 < FIRST_MB.0
    }

    /// Check if address is in VGA range
    pub const fn is_vga_memory(addr: PhysicalAddress) -> bool {
        addr.0 >= VGA_START.0 && addr.0 < VGA_END.0
    }

    /// Check if address is in BIOS range
    pub const fn is_bios_memory(addr: PhysicalAddress) -> bool {
        addr.0 >= SYSTEM_BIOS_START.0 && addr.0 < SYSTEM_BIOS_END.0
    }
}

// =============================================================================
// E820 COMPATIBILITY
// =============================================================================

/// E820 memory map entry (Linux kernel compatible)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct E820Entry {
    /// Base address
    pub base: u64,
    /// Length
    pub length: u64,
    /// Type
    pub entry_type: u32,
    /// Extended attributes (optional)
    pub extended: u32,
}

impl E820Entry {
    /// Create new E820 entry
    pub fn new(base: u64, length: u64, entry_type: E820Type) -> Self {
        Self {
            base,
            length,
            entry_type: entry_type as u32,
            extended: 0,
        }
    }

    /// Get end address
    pub fn end(&self) -> u64 {
        self.base + self.length
    }

    /// Get type
    pub fn get_type(&self) -> E820Type {
        E820Type::from(self.entry_type)
    }
}

/// E820 memory types
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum E820Type {
    /// Usable RAM
    Usable           = 1,
    /// Reserved
    Reserved         = 2,
    /// ACPI reclaimable
    AcpiReclaimable  = 3,
    /// ACPI NVS
    AcpiNvs          = 4,
    /// Bad memory
    BadMemory        = 5,
    /// Persistent memory (PRAM/PMEM)
    PersistentMemory = 7,
    /// Unknown type
    Unknown          = 0,
}

impl From<u32> for E820Type {
    fn from(value: u32) -> Self {
        match value {
            1 => Self::Usable,
            2 => Self::Reserved,
            3 => Self::AcpiReclaimable,
            4 => Self::AcpiNvs,
            5 => Self::BadMemory,
            7 => Self::PersistentMemory,
            _ => Self::Unknown,
        }
    }
}

impl From<MemoryType> for E820Type {
    fn from(mt: MemoryType) -> Self {
        match mt {
            MemoryType::ConventionalMemory => Self::Usable,
            MemoryType::LoaderCode | MemoryType::LoaderData => Self::Usable,
            MemoryType::BootServicesCode | MemoryType::BootServicesData => Self::Usable,
            MemoryType::RuntimeServicesCode | MemoryType::RuntimeServicesData => Self::Reserved,
            MemoryType::AcpiReclaimMemory => Self::AcpiReclaimable,
            MemoryType::AcpiMemoryNvs => Self::AcpiNvs,
            MemoryType::UnusableMemory => Self::BadMemory,
            MemoryType::PersistentMemory => Self::PersistentMemory,
            _ => Self::Reserved,
        }
    }
}

/// E820 memory map builder
pub struct E820MapBuilder {
    entries: Vec<E820Entry>,
}

impl E820MapBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Add entry
    pub fn add(&mut self, base: u64, length: u64, entry_type: E820Type) {
        self.entries.push(E820Entry::new(base, length, entry_type));
    }

    /// Build from UEFI memory map
    pub fn from_uefi_map(map: &MemoryMapInfo) -> Self {
        let mut builder = Self::new();

        for desc in map.descriptors() {
            let length = desc.number_of_pages * 4096;
            let e820_type = E820Type::from(desc.memory_type);
            builder.add(desc.physical_start.0, length, e820_type);
        }

        builder.sort();
        builder.merge_adjacent();

        builder
    }

    /// Sort entries by base address
    pub fn sort(&mut self) {
        self.entries.sort_by_key(|e| e.base);
    }

    /// Merge adjacent entries of same type
    pub fn merge_adjacent(&mut self) {
        if self.entries.len() < 2 {
            return;
        }

        let mut merged = Vec::with_capacity(self.entries.len());
        let mut current = self.entries[0];

        for entry in &self.entries[1..] {
            if entry.base == current.end() && entry.entry_type == current.entry_type {
                current.length += entry.length;
            } else {
                merged.push(current);
                current = *entry;
            }
        }
        merged.push(current);

        self.entries = merged;
    }

    /// Get entries
    pub fn entries(&self) -> &[E820Entry] {
        &self.entries
    }

    /// Get entry count
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Build final map
    pub fn build(self) -> Vec<E820Entry> {
        self.entries
    }
}

impl Default for E820MapBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// MEMORY STATISTICS
// =============================================================================

/// Analyze memory map and return statistics
pub fn analyze_memory_map(map: &MemoryMapInfo) -> MemoryAnalysis {
    let mut analysis = MemoryAnalysis::default();

    for desc in map.descriptors() {
        let size = desc.number_of_pages * 4096;
        analysis.total_entries += 1;

        // Convert u32 memory_type for matching
        let mt = desc.memory_type;
        if mt == MemoryType::ConventionalMemory as u32 {
            analysis.usable_memory += size;
            analysis.usable_entries += 1;
        } else if mt == MemoryType::LoaderCode as u32 || mt == MemoryType::LoaderData as u32 {
            analysis.loader_memory += size;
        } else if mt == MemoryType::BootServicesCode as u32
            || mt == MemoryType::BootServicesData as u32
        {
            analysis.boot_services_memory += size;
        } else if mt == MemoryType::RuntimeServicesCode as u32
            || mt == MemoryType::RuntimeServicesData as u32
        {
            analysis.runtime_memory += size;
            analysis.runtime_entries += 1;
        } else if mt == MemoryType::AcpiReclaimMemory as u32 {
            analysis.acpi_reclaim_memory += size;
        } else if mt == MemoryType::AcpiMemoryNvs as u32 {
            analysis.acpi_nvs_memory += size;
        } else if mt == MemoryType::MemoryMappedIo as u32
            || mt == MemoryType::MemoryMappedIoPortSpace as u32
        {
            analysis.mmio_memory += size;
        } else if mt == MemoryType::ReservedMemoryType as u32 {
            analysis.reserved_memory += size;
        } else if mt == MemoryType::UnusableMemory as u32 {
            analysis.unusable_memory += size;
        } else if mt == MemoryType::PersistentMemory as u32 {
            analysis.persistent_memory += size;
        }

        // Track highest address
        let end = desc.physical_start.0 + size;
        if end > analysis.highest_address {
            analysis.highest_address = end;
        }

        // Track lowest usable address
        if desc.memory_type == MemoryType::ConventionalMemory as u32 {
            if analysis.lowest_usable_address == 0
                || desc.physical_start.0 < analysis.lowest_usable_address
            {
                analysis.lowest_usable_address = desc.physical_start.0;
            }
        }
    }

    analysis.total_memory = analysis.usable_memory
        + analysis.loader_memory
        + analysis.boot_services_memory
        + analysis.runtime_memory
        + analysis.acpi_reclaim_memory
        + analysis.acpi_nvs_memory
        + analysis.reserved_memory;

    analysis
}

/// Memory analysis results
#[derive(Debug, Clone, Default)]
pub struct MemoryAnalysis {
    /// Total memory descriptors
    pub total_entries: usize,
    /// Usable memory entries
    pub usable_entries: usize,
    /// Runtime services entries
    pub runtime_entries: usize,
    /// Total reported memory
    pub total_memory: u64,
    /// Usable conventional memory
    pub usable_memory: u64,
    /// Loader code/data
    pub loader_memory: u64,
    /// Boot services memory
    pub boot_services_memory: u64,
    /// Runtime services memory
    pub runtime_memory: u64,
    /// ACPI reclaimable memory
    pub acpi_reclaim_memory: u64,
    /// ACPI NVS memory
    pub acpi_nvs_memory: u64,
    /// Memory mapped I/O
    pub mmio_memory: u64,
    /// Reserved memory
    pub reserved_memory: u64,
    /// Unusable/bad memory
    pub unusable_memory: u64,
    /// Persistent memory (NVDIMMs)
    pub persistent_memory: u64,
    /// Highest physical address
    pub highest_address: u64,
    /// Lowest usable address
    pub lowest_usable_address: u64,
}

impl MemoryAnalysis {
    /// Get total usable memory in MB
    pub fn usable_mb(&self) -> u64 {
        self.usable_memory / (1024 * 1024)
    }

    /// Get total memory in MB
    pub fn total_mb(&self) -> u64 {
        self.total_memory / (1024 * 1024)
    }

    /// Get memory that will be available after boot services exit
    pub fn available_after_exit_boot_services(&self) -> u64 {
        self.usable_memory + self.loader_memory + self.boot_services_memory
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_memory_range_contains() {
        let range = MemoryRange::new(0x1000, 0x2000, RangeType::Usable);
        assert!(range.contains(0x1000));
        assert!(range.contains(0x1500));
        assert!(!range.contains(0x2000)); // Exclusive end
        assert!(!range.contains(0x0500));
    }

    #[test]
    fn test_memory_range_overlaps() {
        let r1 = MemoryRange::new(0x1000, 0x2000, RangeType::Usable);
        let r2 = MemoryRange::new(0x1500, 0x2500, RangeType::Usable);
        let r3 = MemoryRange::new(0x2000, 0x3000, RangeType::Usable);

        assert!(r1.overlaps(&r2));
        assert!(!r1.overlaps(&r3));
    }

    #[test]
    fn test_e820_type_conversion() {
        assert_eq!(E820Type::from(1), E820Type::Usable);
        assert_eq!(E820Type::from(2), E820Type::Reserved);
        assert_eq!(E820Type::from(3), E820Type::AcpiReclaimable);
    }

    #[test]
    fn test_layout_checks() {
        assert!(layout::is_low_memory(0x50000));
        assert!(!layout::is_low_memory(0x200000));
        assert!(layout::is_vga_memory(0xA5000));
        assert!(layout::is_bios_memory(0xF5000));
    }
}
