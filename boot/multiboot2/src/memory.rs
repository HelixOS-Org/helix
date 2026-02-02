//! # Memory Map Abstractions
//!
//! This module provides type-safe, zero-copy abstractions for memory regions
//! reported by Multiboot2 bootloaders.
//!
//! ## Key Features
//!
//! - **Zero-Copy**: Memory regions are parsed in-place
//! - **Type-Safe**: Region kinds are strongly typed enums
//! - **Iterator-Based**: Efficient iteration over regions
//! - **Filtering**: Easy filtering for usable/reserved memory

use core::fmt;

// =============================================================================
// Memory Region Kind
// =============================================================================

/// Kind of memory region as reported by the bootloader
///
/// This enum represents the various types of memory regions that can be
/// reported in the Multiboot2 memory map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(u32)]
pub enum MemoryRegionKind {
    /// Available RAM that can be freely used
    Available       = 1,

    /// Reserved memory (do not use)
    Reserved        = 2,

    /// ACPI reclaimable memory
    ///
    /// This memory contains ACPI tables. After the OS has finished using
    /// the ACPI tables, this memory can be reclaimed.
    AcpiReclaimable = 3,

    /// ACPI NVS (Non-Volatile Storage) memory
    ///
    /// This memory is used by ACPI for runtime state and must be preserved.
    AcpiNvs         = 4,

    /// Bad memory (defective)
    ///
    /// This memory has been detected as defective and should not be used.
    BadMemory       = 5,

    /// Unknown memory type
    ///
    /// The bootloader reported a type we don't recognize.
    /// Contains the original type value.
    Unknown(u32),
}

impl MemoryRegionKind {
    /// Create from raw type value
    #[must_use]
    pub const fn from_raw(value: u32) -> Self {
        match value {
            1 => Self::Available,
            2 => Self::Reserved,
            3 => Self::AcpiReclaimable,
            4 => Self::AcpiNvs,
            5 => Self::BadMemory,
            other => Self::Unknown(other),
        }
    }

    /// Get raw type value
    #[must_use]
    pub const fn as_raw(&self) -> u32 {
        match self {
            Self::Available => 1,
            Self::Reserved => 2,
            Self::AcpiReclaimable => 3,
            Self::AcpiNvs => 4,
            Self::BadMemory => 5,
            Self::Unknown(v) => *v,
        }
    }

    /// Check if this region is usable for general allocation
    #[must_use]
    pub const fn is_usable(&self) -> bool {
        matches!(self, Self::Available)
    }

    /// Check if this region can eventually be reclaimed
    #[must_use]
    pub const fn is_reclaimable(&self) -> bool {
        matches!(self, Self::Available | Self::AcpiReclaimable)
    }
}

impl fmt::Display for MemoryRegionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Available => write!(f, "Available"),
            Self::Reserved => write!(f, "Reserved"),
            Self::AcpiReclaimable => write!(f, "ACPI Reclaimable"),
            Self::AcpiNvs => write!(f, "ACPI NVS"),
            Self::BadMemory => write!(f, "Bad Memory"),
            Self::Unknown(v) => write!(f, "Unknown({})", v),
        }
    }
}

// =============================================================================
// Memory Region
// =============================================================================

/// A single memory region from the memory map
///
/// This structure represents a contiguous region of physical memory with
/// a specific type. It is designed to be zero-copy and lifetime-bound
/// to the original boot information.
#[derive(Clone, Copy)]
pub struct MemoryRegion {
    /// Starting physical address
    base: u64,
    /// Length in bytes
    length: u64,
    /// Region kind
    kind: MemoryRegionKind,
}

impl MemoryRegion {
    /// Create a new memory region
    #[must_use]
    pub const fn new(base: u64, length: u64, kind: MemoryRegionKind) -> Self {
        Self { base, length, kind }
    }

    /// Create from raw Multiboot2 memory map entry data
    ///
    /// # Safety
    ///
    /// The caller must ensure the data is a valid memory map entry.
    #[must_use]
    pub const fn from_raw(base: u64, length: u64, entry_type: u32) -> Self {
        Self {
            base,
            length,
            kind: MemoryRegionKind::from_raw(entry_type),
        }
    }

    /// Get the starting physical address
    #[must_use]
    pub const fn start(&self) -> u64 {
        self.base
    }

    /// Get the ending physical address (exclusive)
    #[must_use]
    pub const fn end(&self) -> u64 {
        self.base + self.length
    }

    /// Get the length in bytes
    #[must_use]
    pub const fn length(&self) -> u64 {
        self.length
    }

    /// Get the length in bytes (alias for `length`)
    #[must_use]
    pub const fn size(&self) -> u64 {
        self.length
    }

    /// Get the region kind
    #[must_use]
    pub const fn kind(&self) -> MemoryRegionKind {
        self.kind
    }

    /// Check if this region is usable for allocation
    #[must_use]
    pub const fn is_usable(&self) -> bool {
        self.kind.is_usable()
    }

    /// Check if an address falls within this region
    #[must_use]
    pub const fn contains(&self, addr: u64) -> bool {
        addr >= self.base && addr < self.base + self.length
    }

    /// Check if this region overlaps with a range
    #[must_use]
    pub const fn overlaps(&self, start: u64, end: u64) -> bool {
        self.base < end && self.end() > start
    }

    /// Get the number of 4KB pages in this region
    #[must_use]
    pub const fn page_count(&self) -> u64 {
        self.length / 4096
    }

    /// Get the number of pages of a given size
    #[must_use]
    pub const fn pages_of_size(&self, page_size: u64) -> u64 {
        self.length / page_size
    }
}

impl fmt::Debug for MemoryRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MemoryRegion")
            .field("start", &format_args!("{:#x}", self.base))
            .field("end", &format_args!("{:#x}", self.end()))
            .field(
                "size",
                &format_args!("{:#x} ({} KB)", self.length, self.length / 1024),
            )
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for MemoryRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:#012x} - {:#012x} ({:>8} KB) {:?}",
            self.base,
            self.end(),
            self.length / 1024,
            self.kind
        )
    }
}

// =============================================================================
// Memory Map
// =============================================================================

/// Complete memory map from bootloader
///
/// This structure provides access to all memory regions reported by the
/// bootloader. It supports iteration, filtering, and various queries.
///
/// # Lifetime
///
/// The `'boot` lifetime ties the memory map to the boot information.
/// This ensures that the underlying data remains valid.
#[derive(Clone)]
pub struct MemoryMap<'boot> {
    /// Entry size as reported by the bootloader
    entry_size: u32,
    /// Entry version
    entry_version: u32,
    /// Raw entry data
    entries: &'boot [u8],
}

impl<'boot> MemoryMap<'boot> {
    /// Create a new memory map from raw data
    ///
    /// # Arguments
    ///
    /// * `entry_size` - Size of each entry in bytes
    /// * `entry_version` - Entry version (usually 0)
    /// * `entries` - Raw entry data
    ///
    /// # Safety
    ///
    /// The caller must ensure the entry data is valid and properly formatted.
    #[must_use]
    pub const fn new(entry_size: u32, entry_version: u32, entries: &'boot [u8]) -> Self {
        Self {
            entry_size,
            entry_version,
            entries,
        }
    }

    /// Get the entry size
    #[must_use]
    pub const fn entry_size(&self) -> u32 {
        self.entry_size
    }

    /// Get the entry version
    #[must_use]
    pub const fn entry_version(&self) -> u32 {
        self.entry_version
    }

    /// Get the number of entries
    #[must_use]
    pub fn entry_count(&self) -> usize {
        if self.entry_size == 0 {
            0
        } else {
            self.entries.len() / self.entry_size as usize
        }
    }

    /// Check if the memory map is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entry_count() == 0
    }

    /// Iterate over all memory regions
    pub fn regions(&self) -> MemoryRegionIterator<'boot> {
        MemoryRegionIterator {
            entry_size: self.entry_size as usize,
            data: self.entries,
            offset: 0,
        }
    }

    /// Iterate over usable memory regions only
    pub fn usable_regions(&self) -> impl Iterator<Item = MemoryRegion> + 'boot {
        self.regions().filter(|r| r.is_usable())
    }

    /// Get total available memory in bytes
    pub fn total_available(&self) -> u64 {
        self.usable_regions().map(|r| r.length()).sum()
    }

    /// Get total memory (all types) in bytes
    pub fn total_memory(&self) -> u64 {
        self.regions().map(|r| r.length()).sum()
    }

    /// Find the largest usable region
    pub fn largest_usable_region(&self) -> Option<MemoryRegion> {
        self.usable_regions().max_by_key(|r| r.length())
    }

    /// Find a usable region containing the given address
    pub fn find_region_containing(&self, addr: u64) -> Option<MemoryRegion> {
        self.regions().find(|r| r.contains(addr))
    }

    /// Find usable regions above a certain address
    pub fn usable_above(&self, addr: u64) -> impl Iterator<Item = MemoryRegion> + 'boot {
        self.usable_regions().filter(move |r| r.start() >= addr)
    }
}

impl fmt::Debug for MemoryMap<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MemoryMap")
            .field("entry_size", &self.entry_size)
            .field("entry_version", &self.entry_version)
            .field("entry_count", &self.entry_count())
            .field(
                "total_available",
                &format_args!("{} MB", self.total_available() / (1024 * 1024)),
            )
            .finish()
    }
}

// =============================================================================
// Memory Region Iterator
// =============================================================================

/// Iterator over memory regions
pub struct MemoryRegionIterator<'boot> {
    entry_size: usize,
    data: &'boot [u8],
    offset: usize,
}

impl<'boot> Iterator for MemoryRegionIterator<'boot> {
    type Item = MemoryRegion;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset + self.entry_size > self.data.len() {
            return None;
        }

        // Read entry fields
        // Layout: base_addr (u64), length (u64), type (u32), reserved (u32)
        let entry = &self.data[self.offset..self.offset + self.entry_size];

        if entry.len() < 24 {
            return None;
        }

        let base = u64::from_le_bytes([
            entry[0], entry[1], entry[2], entry[3], entry[4], entry[5], entry[6], entry[7],
        ]);

        let length = u64::from_le_bytes([
            entry[8], entry[9], entry[10], entry[11], entry[12], entry[13], entry[14], entry[15],
        ]);

        let entry_type = u32::from_le_bytes([entry[16], entry[17], entry[18], entry[19]]);

        self.offset += self.entry_size;

        Some(MemoryRegion::from_raw(base, length, entry_type))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.entry_size == 0 {
            (0, Some(0))
        } else {
            let remaining = (self.data.len() - self.offset) / self.entry_size;
            (remaining, Some(remaining))
        }
    }
}

impl ExactSizeIterator for MemoryRegionIterator<'_> {}

// =============================================================================
// Memory Statistics
// =============================================================================

/// Statistics about the memory map
#[derive(Debug, Clone, Copy)]
pub struct MemoryStats {
    /// Total memory (all types)
    pub total: u64,
    /// Available memory
    pub available: u64,
    /// Reserved memory
    pub reserved: u64,
    /// ACPI reclaimable memory
    pub acpi_reclaimable: u64,
    /// ACPI NVS memory
    pub acpi_nvs: u64,
    /// Bad memory
    pub bad: u64,
    /// Number of regions
    pub region_count: usize,
    /// Number of usable regions
    pub usable_region_count: usize,
}

impl MemoryStats {
    /// Calculate statistics from a memory map
    pub fn from_map(map: &MemoryMap<'_>) -> Self {
        let mut stats = Self {
            total: 0,
            available: 0,
            reserved: 0,
            acpi_reclaimable: 0,
            acpi_nvs: 0,
            bad: 0,
            region_count: 0,
            usable_region_count: 0,
        };

        for region in map.regions() {
            stats.total += region.length();
            stats.region_count += 1;

            match region.kind() {
                MemoryRegionKind::Available => {
                    stats.available += region.length();
                    stats.usable_region_count += 1;
                },
                MemoryRegionKind::Reserved => {
                    stats.reserved += region.length();
                },
                MemoryRegionKind::AcpiReclaimable => {
                    stats.acpi_reclaimable += region.length();
                },
                MemoryRegionKind::AcpiNvs => {
                    stats.acpi_nvs += region.length();
                },
                MemoryRegionKind::BadMemory => {
                    stats.bad += region.length();
                },
                MemoryRegionKind::Unknown(_) => {
                    stats.reserved += region.length();
                },
            }
        }

        stats
    }
}

impl fmt::Display for MemoryStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Memory Statistics:")?;
        writeln!(
            f,
            "  Total:           {:>10} KB ({} MB)",
            self.total / 1024,
            self.total / (1024 * 1024)
        )?;
        writeln!(
            f,
            "  Available:       {:>10} KB ({} MB)",
            self.available / 1024,
            self.available / (1024 * 1024)
        )?;
        writeln!(f, "  Reserved:        {:>10} KB", self.reserved / 1024)?;
        writeln!(
            f,
            "  ACPI Reclaim:    {:>10} KB",
            self.acpi_reclaimable / 1024
        )?;
        writeln!(f, "  ACPI NVS:        {:>10} KB", self.acpi_nvs / 1024)?;
        writeln!(f, "  Bad:             {:>10} KB", self.bad / 1024)?;
        writeln!(
            f,
            "  Regions:         {:>10} ({} usable)",
            self.region_count, self.usable_region_count
        )
    }
}
