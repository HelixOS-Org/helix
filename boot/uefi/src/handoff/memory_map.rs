//! Memory Map
//!
//! Boot-time memory map structures for passing to the kernel.

use crate::error::Result;
use crate::raw::types::{PhysicalAddress, VirtualAddress};

extern crate alloc;
use alloc::vec::Vec;

// =============================================================================
// MEMORY TYPE
// =============================================================================

/// Memory type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum MemoryType {
    /// Reserved, unusable memory
    Reserved         = 0,
    /// Usable RAM
    Usable           = 1,
    /// ACPI reclaimable
    AcpiReclaimable  = 2,
    /// ACPI NVS (non-volatile storage)
    AcpiNvs          = 3,
    /// Bad/defective memory
    BadMemory        = 4,
    /// Bootloader reclaimable
    BootloaderReclaimable = 5,
    /// Kernel and modules
    KernelAndModules = 6,
    /// Framebuffer
    Framebuffer      = 7,
    /// EFI runtime services code
    EfiRuntimeCode   = 8,
    /// EFI runtime services data
    EfiRuntimeData   = 9,
    /// Memory-mapped I/O
    Mmio             = 10,
    /// Memory-mapped I/O port space
    MmioPortSpace    = 11,
    /// PAL code (IA64)
    PalCode          = 12,
    /// Persistent memory
    PersistentMemory = 13,
    /// Unknown type
    Unknown          = 0xFFFF,
}

impl MemoryType {
    /// Check if usable by kernel after boot
    pub fn is_usable(&self) -> bool {
        matches!(self, MemoryType::Usable | MemoryType::BootloaderReclaimable)
    }

    /// Check if reclaimable
    pub fn is_reclaimable(&self) -> bool {
        matches!(
            self,
            MemoryType::BootloaderReclaimable | MemoryType::AcpiReclaimable
        )
    }

    /// Check if reserved
    pub fn is_reserved(&self) -> bool {
        matches!(
            self,
            MemoryType::Reserved
                | MemoryType::AcpiNvs
                | MemoryType::BadMemory
                | MemoryType::Mmio
                | MemoryType::MmioPortSpace
        )
    }

    /// Convert from UEFI memory type
    pub fn from_uefi(uefi_type: u32) -> Self {
        match uefi_type {
            0 => MemoryType::Reserved,              // EfiReservedMemoryType
            1 => MemoryType::BootloaderReclaimable, // EfiLoaderCode
            2 => MemoryType::BootloaderReclaimable, // EfiLoaderData
            3 => MemoryType::BootloaderReclaimable, // EfiBootServicesCode
            4 => MemoryType::BootloaderReclaimable, // EfiBootServicesData
            5 => MemoryType::EfiRuntimeCode,        // EfiRuntimeServicesCode
            6 => MemoryType::EfiRuntimeData,        // EfiRuntimeServicesData
            7 => MemoryType::Usable,                // EfiConventionalMemory
            8 => MemoryType::BadMemory,             // EfiUnusableMemory
            9 => MemoryType::AcpiReclaimable,       // EfiACPIReclaimMemory
            10 => MemoryType::AcpiNvs,              // EfiACPIMemoryNVS
            11 => MemoryType::Mmio,                 // EfiMemoryMappedIO
            12 => MemoryType::MmioPortSpace,        // EfiMemoryMappedIOPortSpace
            13 => MemoryType::PalCode,              // EfiPalCode
            14 => MemoryType::PersistentMemory,     // EfiPersistentMemory
            _ => MemoryType::Unknown,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryType::Reserved => "Reserved",
            MemoryType::Usable => "Usable",
            MemoryType::AcpiReclaimable => "ACPI Reclaimable",
            MemoryType::AcpiNvs => "ACPI NVS",
            MemoryType::BadMemory => "Bad Memory",
            MemoryType::BootloaderReclaimable => "Bootloader Reclaimable",
            MemoryType::KernelAndModules => "Kernel/Modules",
            MemoryType::Framebuffer => "Framebuffer",
            MemoryType::EfiRuntimeCode => "EFI Runtime Code",
            MemoryType::EfiRuntimeData => "EFI Runtime Data",
            MemoryType::Mmio => "MMIO",
            MemoryType::MmioPortSpace => "MMIO Port Space",
            MemoryType::PalCode => "PAL Code",
            MemoryType::PersistentMemory => "Persistent Memory",
            MemoryType::Unknown => "Unknown",
        }
    }
}

impl Default for MemoryType {
    fn default() -> Self {
        MemoryType::Reserved
    }
}

// =============================================================================
// MEMORY ATTRIBUTES
// =============================================================================

/// Memory attributes bitflags
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MemoryAttributes(pub u64);

impl MemoryAttributes {
    /// Uncacheable
    pub const UC: Self = Self(1 << 0);
    /// Write Combining
    pub const WC: Self = Self(1 << 1);
    /// Write Through
    pub const WT: Self = Self(1 << 2);
    /// Write Back
    pub const WB: Self = Self(1 << 3);
    /// Uncacheable, exported
    pub const UCE: Self = Self(1 << 4);
    /// Write Protected
    pub const WP: Self = Self(1 << 12);
    /// Read Protected
    pub const RP: Self = Self(1 << 13);
    /// Execute Protected
    pub const XP: Self = Self(1 << 14);
    /// Non-volatile
    pub const NV: Self = Self(1 << 15);
    /// More reliable
    pub const MORE_RELIABLE: Self = Self(1 << 16);
    /// Read-only
    pub const RO: Self = Self(1 << 17);
    /// Specific purpose
    pub const SP: Self = Self(1 << 18);
    /// CPU crypto
    pub const CPU_CRYPTO: Self = Self(1 << 19);
    /// Runtime memory
    pub const RUNTIME: Self = Self(1 << 63);

    /// Empty attributes
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Check if contains attribute
    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Insert attribute
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// Remove attribute
    pub fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
    }

    /// Combine attributes
    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Intersect attributes
    pub fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Check if cacheable
    pub fn is_cacheable(&self) -> bool {
        self.contains(Self::WB) || self.contains(Self::WT)
    }

    /// Check if runtime
    pub fn is_runtime(&self) -> bool {
        self.contains(Self::RUNTIME)
    }

    /// Check if executable
    pub fn is_executable(&self) -> bool {
        !self.contains(Self::XP)
    }

    /// Check if writable
    pub fn is_writable(&self) -> bool {
        !self.contains(Self::WP) && !self.contains(Self::RO)
    }

    /// Check if readable
    pub fn is_readable(&self) -> bool {
        !self.contains(Self::RP)
    }

    /// Get cache type string
    pub fn cache_type_str(&self) -> &'static str {
        if self.contains(Self::WB) {
            "WB"
        } else if self.contains(Self::WT) {
            "WT"
        } else if self.contains(Self::WC) {
            "WC"
        } else if self.contains(Self::UC) || self.contains(Self::UCE) {
            "UC"
        } else {
            "??"
        }
    }
}

// =============================================================================
// MEMORY MAP ENTRY
// =============================================================================

/// Memory map entry
#[derive(Debug, Clone, Copy)]
pub struct MemoryMapEntry {
    /// Physical start address
    pub physical_start: PhysicalAddress,
    /// Virtual start address (if mapped)
    pub virtual_start: VirtualAddress,
    /// Number of 4KB pages
    pub page_count: u64,
    /// Memory type
    pub memory_type: MemoryType,
    /// Memory attributes
    pub attributes: MemoryAttributes,
}

impl MemoryMapEntry {
    /// Create new entry
    pub fn new(physical_start: PhysicalAddress, page_count: u64, memory_type: MemoryType) -> Self {
        Self {
            physical_start,
            virtual_start: VirtualAddress(0),
            page_count,
            memory_type,
            attributes: MemoryAttributes::empty(),
        }
    }

    /// Get size in bytes
    pub fn size(&self) -> u64 {
        self.page_count * 4096
    }

    /// Get end address (exclusive)
    pub fn end(&self) -> PhysicalAddress {
        self.physical_start + self.size()
    }

    /// Check if address is within this entry
    pub fn contains(&self, addr: PhysicalAddress) -> bool {
        addr >= self.physical_start && addr < self.end()
    }

    /// Check if overlaps with range
    pub fn overlaps(&self, start: PhysicalAddress, size: u64) -> bool {
        let end = start + size;
        self.physical_start < end && self.end() > start
    }

    /// Check if adjacent to another entry
    pub fn is_adjacent(&self, other: &Self) -> bool {
        self.end() == other.physical_start || other.end() == self.physical_start
    }

    /// Check if can merge with another entry
    pub fn can_merge(&self, other: &Self) -> bool {
        self.is_adjacent(other)
            && self.memory_type == other.memory_type
            && self.attributes == other.attributes
    }

    /// Merge with another entry
    pub fn merge(&mut self, other: &Self) -> bool {
        if !self.can_merge(other) {
            return false;
        }

        if other.physical_start < self.physical_start {
            self.physical_start = other.physical_start;
        }
        self.page_count += other.page_count;
        true
    }

    /// Split at address
    pub fn split_at(&self, addr: PhysicalAddress) -> Option<(Self, Self)> {
        if addr <= self.physical_start || addr >= self.end() {
            return None;
        }

        let first_pages = (addr - self.physical_start) / 4096;
        let second_pages = self.page_count - first_pages;

        let first = Self {
            physical_start: self.physical_start,
            virtual_start: self.virtual_start,
            page_count: first_pages,
            memory_type: self.memory_type,
            attributes: self.attributes,
        };

        let second = Self {
            physical_start: addr,
            virtual_start: if self.virtual_start.0 != 0 {
                VirtualAddress(self.virtual_start.0 + first_pages * 4096)
            } else {
                VirtualAddress(0)
            },
            page_count: second_pages,
            memory_type: self.memory_type,
            attributes: self.attributes,
        };

        Some((first, second))
    }
}

impl Default for MemoryMapEntry {
    fn default() -> Self {
        Self {
            physical_start: PhysicalAddress(0),
            virtual_start: VirtualAddress(0),
            page_count: 0,
            memory_type: MemoryType::Reserved,
            attributes: MemoryAttributes::empty(),
        }
    }
}

// =============================================================================
// C-COMPATIBLE ENTRY
// =============================================================================

/// C-compatible memory map entry for direct serialization
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryMapEntryRaw {
    /// Physical start address
    pub physical_start: u64,
    /// Virtual start address
    pub virtual_start: u64,
    /// Number of pages
    pub page_count: u64,
    /// Memory type
    pub memory_type: u32,
    /// Reserved
    pub reserved: u32,
    /// Attributes
    pub attributes: u64,
}

impl MemoryMapEntryRaw {
    /// Size of this structure
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Create from MemoryMapEntry
    pub fn from_entry(entry: &MemoryMapEntry) -> Self {
        Self {
            physical_start: entry.physical_start.0,
            virtual_start: entry.virtual_start.0,
            page_count: entry.page_count,
            memory_type: entry.memory_type as u32,
            reserved: 0,
            attributes: entry.attributes.0,
        }
    }

    /// Convert to MemoryMapEntry
    pub fn to_entry(&self) -> MemoryMapEntry {
        MemoryMapEntry {
            physical_start: PhysicalAddress(self.physical_start),
            virtual_start: VirtualAddress(self.virtual_start),
            page_count: self.page_count,
            memory_type: MemoryType::from_uefi(self.memory_type),
            attributes: MemoryAttributes(self.attributes),
        }
    }
}

// =============================================================================
// MEMORY MAP
// =============================================================================

/// Memory map
#[derive(Debug, Clone, Default)]
pub struct MemoryMap {
    /// Memory map entries
    pub entries: Vec<MemoryMapEntry>,
}

impl MemoryMap {
    /// Create new empty memory map
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Create with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
        }
    }

    /// Add entry
    pub fn add(&mut self, entry: MemoryMapEntry) {
        self.entries.push(entry);
    }

    /// Add entry and try to merge
    pub fn add_merge(&mut self, entry: MemoryMapEntry) {
        // Try to merge with existing entry
        for existing in &mut self.entries {
            if existing.can_merge(&entry) {
                existing.merge(&entry);
                return;
            }
        }

        // No merge possible, add new entry
        self.entries.push(entry);
    }

    /// Sort entries by physical address
    pub fn sort(&mut self) {
        self.entries.sort_by_key(|e| e.physical_start);
    }

    /// Merge adjacent entries with same type
    pub fn coalesce(&mut self) {
        if self.entries.is_empty() {
            return;
        }

        self.sort();

        let mut result = Vec::with_capacity(self.entries.len());
        let mut current = self.entries[0];

        for entry in self.entries.iter().skip(1) {
            if current.can_merge(entry) {
                current.merge(entry);
            } else {
                result.push(current);
                current = *entry;
            }
        }

        result.push(current);
        self.entries = result;
    }

    /// Find entry containing address
    pub fn find(&self, addr: PhysicalAddress) -> Option<&MemoryMapEntry> {
        self.entries.iter().find(|e| e.contains(addr))
    }

    /// Find entry containing address (mutable)
    pub fn find_mut(&mut self, addr: PhysicalAddress) -> Option<&mut MemoryMapEntry> {
        self.entries.iter_mut().find(|e| e.contains(addr))
    }

    /// Get total usable memory
    pub fn total_usable(&self) -> u64 {
        self.entries
            .iter()
            .filter(|e| e.memory_type.is_usable())
            .map(MemoryMapEntry::size)
            .sum()
    }

    /// Get total memory (all types)
    pub fn total_memory(&self) -> u64 {
        self.entries.iter().map(MemoryMapEntry::size).sum()
    }

    /// Get highest physical address
    pub fn highest_address(&self) -> u64 {
        self.entries.iter().map(|e| e.end().0).max().unwrap_or(0)
    }

    /// Get lowest usable address
    pub fn lowest_usable(&self) -> Option<u64> {
        self.entries
            .iter()
            .filter(|e| e.memory_type.is_usable())
            .map(|e| e.physical_start.0)
            .min()
    }

    /// Iterate usable regions
    pub fn usable_regions(&self) -> impl Iterator<Item = &MemoryMapEntry> {
        self.entries.iter().filter(|e| e.memory_type.is_usable())
    }

    /// Iterate reserved regions
    pub fn reserved_regions(&self) -> impl Iterator<Item = &MemoryMapEntry> {
        self.entries.iter().filter(|e| e.memory_type.is_reserved())
    }

    /// Count entries by type
    pub fn count_by_type(&self, memory_type: MemoryType) -> usize {
        self.entries
            .iter()
            .filter(|e| e.memory_type == memory_type)
            .count()
    }

    /// Reserve region
    pub fn reserve(
        &mut self,
        start: PhysicalAddress,
        size: u64,
        memory_type: MemoryType,
    ) -> Result<()> {
        let end = start + size;
        let mut new_entries = Vec::new();
        let mut modified = false;

        for entry in &self.entries {
            if entry.overlaps(start, size) {
                modified = true;

                // Split before reserved region
                if entry.physical_start < start {
                    new_entries.push(MemoryMapEntry {
                        physical_start: entry.physical_start,
                        virtual_start: entry.virtual_start,
                        page_count: (start - entry.physical_start) / 4096,
                        memory_type: entry.memory_type,
                        attributes: entry.attributes,
                    });
                }

                // The reserved region itself
                let reserved_start = start.max(entry.physical_start);
                let reserved_end = end.min(entry.end());
                new_entries.push(MemoryMapEntry {
                    physical_start: reserved_start,
                    virtual_start: VirtualAddress(0),
                    page_count: (reserved_end - reserved_start) / 4096,
                    memory_type,
                    attributes: MemoryAttributes::empty(),
                });

                // Split after reserved region
                if entry.end() > end {
                    new_entries.push(MemoryMapEntry {
                        physical_start: end,
                        virtual_start: VirtualAddress(0),
                        page_count: (entry.end() - end) / 4096,
                        memory_type: entry.memory_type,
                        attributes: entry.attributes,
                    });
                }
            } else {
                new_entries.push(*entry);
            }
        }

        if modified {
            self.entries = new_entries;
            self.coalesce();
        }

        Ok(())
    }

    /// Find free region of at least given size
    pub fn find_free(&self, size: u64, alignment: u64) -> Option<PhysicalAddress> {
        for entry in &self.entries {
            if !entry.memory_type.is_usable() {
                continue;
            }

            let aligned_start = (entry.physical_start.0 + alignment - 1) & !(alignment - 1);
            if aligned_start + size <= entry.end().0 {
                return Some(PhysicalAddress(aligned_start));
            }
        }

        None
    }

    /// Find free region below limit
    pub fn find_free_below(
        &self,
        size: u64,
        alignment: u64,
        limit: u64,
    ) -> Option<PhysicalAddress> {
        for entry in &self.entries {
            if !entry.memory_type.is_usable() {
                continue;
            }

            let aligned_start = (entry.physical_start.0 + alignment - 1) & !(alignment - 1);
            let end = aligned_start + size;

            if end <= entry.end().0 && end <= limit {
                return Some(PhysicalAddress(aligned_start));
            }
        }

        None
    }

    /// Get statistics
    pub fn statistics(&self) -> MemoryMapStats {
        let mut stats = MemoryMapStats::default();

        for entry in &self.entries {
            stats.total_entries += 1;

            match entry.memory_type {
                MemoryType::Usable => {
                    stats.usable_entries += 1;
                    stats.usable_memory += entry.size();
                },
                MemoryType::Reserved => {
                    stats.reserved_entries += 1;
                    stats.reserved_memory += entry.size();
                },
                MemoryType::AcpiReclaimable => {
                    stats.acpi_entries += 1;
                    stats.acpi_memory += entry.size();
                },
                MemoryType::BootloaderReclaimable => {
                    stats.bootloader_entries += 1;
                    stats.bootloader_memory += entry.size();
                },
                _ => {
                    stats.other_entries += 1;
                    stats.other_memory += entry.size();
                },
            }
        }

        stats.highest_address = self.highest_address();
        stats
    }
}

// =============================================================================
// MEMORY MAP STATISTICS
// =============================================================================

/// Memory map statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryMapStats {
    /// Total entries
    pub total_entries: usize,
    /// Usable entries
    pub usable_entries: usize,
    /// Usable memory in bytes
    pub usable_memory: u64,
    /// Reserved entries
    pub reserved_entries: usize,
    /// Reserved memory in bytes
    pub reserved_memory: u64,
    /// ACPI entries
    pub acpi_entries: usize,
    /// ACPI memory in bytes
    pub acpi_memory: u64,
    /// Bootloader entries
    pub bootloader_entries: usize,
    /// Bootloader memory in bytes
    pub bootloader_memory: u64,
    /// Other entries
    pub other_entries: usize,
    /// Other memory in bytes
    pub other_memory: u64,
    /// Highest physical address
    pub highest_address: u64,
}

impl MemoryMapStats {
    /// Get total memory
    pub fn total_memory(&self) -> u64 {
        self.usable_memory
            + self.reserved_memory
            + self.acpi_memory
            + self.bootloader_memory
            + self.other_memory
    }

    /// Get usable after boot (usable + bootloader reclaimable)
    pub fn usable_after_boot(&self) -> u64 {
        self.usable_memory + self.bootloader_memory
    }
}

// =============================================================================
// MEMORY MAP BUILDER
// =============================================================================

/// Memory map builder
pub struct MemoryMapBuilder {
    map: MemoryMap,
}

impl MemoryMapBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            map: MemoryMap::new(),
        }
    }

    /// Add usable region
    pub fn usable(mut self, start: PhysicalAddress, size: u64) -> Self {
        self.map
            .add(MemoryMapEntry::new(start, size / 4096, MemoryType::Usable));
        self
    }

    /// Add reserved region
    pub fn reserved(mut self, start: PhysicalAddress, size: u64) -> Self {
        self.map.add(MemoryMapEntry::new(
            start,
            size / 4096,
            MemoryType::Reserved,
        ));
        self
    }

    /// Add ACPI reclaimable region
    pub fn acpi_reclaimable(mut self, start: PhysicalAddress, size: u64) -> Self {
        self.map.add(MemoryMapEntry::new(
            start,
            size / 4096,
            MemoryType::AcpiReclaimable,
        ));
        self
    }

    /// Add ACPI NVS region
    pub fn acpi_nvs(mut self, start: PhysicalAddress, size: u64) -> Self {
        self.map
            .add(MemoryMapEntry::new(start, size / 4096, MemoryType::AcpiNvs));
        self
    }

    /// Add bootloader reclaimable region
    pub fn bootloader_reclaimable(mut self, start: PhysicalAddress, size: u64) -> Self {
        self.map.add(MemoryMapEntry::new(
            start,
            size / 4096,
            MemoryType::BootloaderReclaimable,
        ));
        self
    }

    /// Add kernel region
    pub fn kernel(mut self, start: PhysicalAddress, size: u64) -> Self {
        self.map.add(MemoryMapEntry::new(
            start,
            size / 4096,
            MemoryType::KernelAndModules,
        ));
        self
    }

    /// Add framebuffer region
    pub fn framebuffer(mut self, start: PhysicalAddress, size: u64) -> Self {
        self.map.add(MemoryMapEntry::new(
            start,
            size / 4096,
            MemoryType::Framebuffer,
        ));
        self
    }

    /// Add MMIO region
    pub fn mmio(mut self, start: PhysicalAddress, size: u64) -> Self {
        self.map
            .add(MemoryMapEntry::new(start, size / 4096, MemoryType::Mmio));
        self
    }

    /// Add entry with full control
    pub fn entry(mut self, entry: MemoryMapEntry) -> Self {
        self.map.add(entry);
        self
    }

    /// Build the memory map
    pub fn build(mut self) -> MemoryMap {
        self.map.sort();
        self.map.coalesce();
        self.map
    }
}

impl Default for MemoryMapBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// MEMORY MAP ITERATOR
// =============================================================================

/// Memory map iterator
pub struct MemoryMapIterator<'a> {
    entries: core::slice::Iter<'a, MemoryMapEntry>,
    filter: Option<MemoryType>,
}

impl<'a> MemoryMapIterator<'a> {
    /// Create new iterator
    pub fn new(map: &'a MemoryMap) -> Self {
        Self {
            entries: map.entries.iter(),
            filter: None,
        }
    }

    /// Create filtered iterator
    pub fn with_filter(map: &'a MemoryMap, filter: MemoryType) -> Self {
        Self {
            entries: map.entries.iter(),
            filter: Some(filter),
        }
    }
}

impl<'a> Iterator for MemoryMapIterator<'a> {
    type Item = &'a MemoryMapEntry;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entry = self.entries.next()?;

            if let Some(filter) = self.filter {
                if entry.memory_type == filter {
                    return Some(entry);
                }
            } else {
                return Some(entry);
            }
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
    fn test_memory_type() {
        assert!(MemoryType::Usable.is_usable());
        assert!(MemoryType::Reserved.is_reserved());
        assert!(MemoryType::BootloaderReclaimable.is_reclaimable());
    }

    #[test]
    fn test_memory_attributes() {
        let mut attrs = MemoryAttributes::empty();
        attrs.insert(MemoryAttributes::WB);
        assert!(attrs.is_cacheable());

        attrs.insert(MemoryAttributes::XP);
        assert!(!attrs.is_executable());
    }

    #[test]
    fn test_memory_map_entry() {
        let entry = MemoryMapEntry::new(0x1000, 10, MemoryType::Usable);
        assert_eq!(entry.size(), 10 * 4096);
        assert_eq!(entry.end(), 0x1000 + 10 * 4096);
        assert!(entry.contains(0x2000));
        assert!(!entry.contains(0x100));
    }

    #[test]
    fn test_memory_map_builder() {
        let map = MemoryMapBuilder::new()
            .usable(0x100000, 0x1000000)
            .reserved(0, 0x100000)
            .kernel(0x200000, 0x100000)
            .build();

        assert_eq!(map.entries.len(), 3);
        assert!(map.total_usable() > 0);
    }

    #[test]
    fn test_find_free() {
        let map = MemoryMapBuilder::new().usable(0x100000, 0x1000000).build();

        let addr = map.find_free(0x1000, 0x1000);
        assert!(addr.is_some());
        assert_eq!(addr.unwrap(), 0x100000);
    }
}
