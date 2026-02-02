//! Memory Region Management
//!
//! Track and manage memory regions during boot.

use crate::error::Result;
use crate::raw::types::*;

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

// =============================================================================
// REGION MANAGER
// =============================================================================

/// Memory region manager
pub struct RegionManager {
    /// All regions
    regions: Vec<MemoryRegion>,
    /// Reserved regions
    reserved: Vec<ReservedRegion>,
}

impl RegionManager {
    /// Create new region manager
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
            reserved: Vec::new(),
        }
    }

    /// Add memory region
    pub fn add_region(
        &mut self,
        base: PhysicalAddress,
        size: u64,
        region_type: RegionType,
    ) -> Result<()> {
        self.regions.push(MemoryRegion {
            base,
            size,
            region_type,
            attributes: RegionAttributes::default(),
        });
        Ok(())
    }

    /// Add region with attributes
    pub fn add_region_with_attributes(
        &mut self,
        base: PhysicalAddress,
        size: u64,
        region_type: RegionType,
        attributes: RegionAttributes,
    ) -> Result<()> {
        self.regions.push(MemoryRegion {
            base,
            size,
            region_type,
            attributes,
        });
        Ok(())
    }

    /// Mark region as reserved
    pub fn mark_reserved(&mut self, base: PhysicalAddress, size: u64) -> Result<()> {
        self.mark_reserved_for(base, size, "reserved")
    }

    /// Mark region as reserved with name
    pub fn mark_reserved_for(
        &mut self,
        base: PhysicalAddress,
        size: u64,
        name: &str,
    ) -> Result<()> {
        self.reserved.push(ReservedRegion {
            base,
            size,
            name: String::from(name),
        });
        Ok(())
    }

    /// Get all regions
    pub fn regions(&self) -> &[MemoryRegion] {
        &self.regions
    }

    /// Get regions by type
    pub fn regions_by_type(&self, region_type: RegionType) -> Vec<&MemoryRegion> {
        self.regions
            .iter()
            .filter(|r| r.region_type == region_type)
            .collect()
    }

    /// Get reserved regions
    pub fn reserved_regions(&self) -> &[ReservedRegion] {
        &self.reserved
    }

    /// Find largest region by type
    pub fn find_largest_by_type(&self, region_type: RegionType) -> Option<(PhysicalAddress, u64)> {
        self.regions
            .iter()
            .filter(|r| r.region_type == region_type)
            .max_by_key(|r| r.size)
            .map(|r| (r.base, r.size))
    }

    /// Find region containing address
    pub fn find_containing(&self, address: PhysicalAddress) -> Option<&MemoryRegion> {
        self.regions.iter().find(|r| r.contains(address))
    }

    /// Check if address is reserved
    pub fn is_reserved(&self, address: PhysicalAddress) -> bool {
        self.reserved
            .iter()
            .any(|r| address >= r.base && address < r.base + r.size)
    }

    /// Check if range overlaps reserved
    pub fn overlaps_reserved(&self, base: PhysicalAddress, size: u64) -> bool {
        let end = base + size;
        self.reserved.iter().any(|r| {
            let r_end = r.base + r.size;
            base < r_end && r.base < end
        })
    }

    /// Find free region of given size
    pub fn find_free_region(&self, size: u64, alignment: u64) -> Option<PhysicalAddress> {
        for region in &self.regions {
            if region.region_type != RegionType::Usable {
                continue;
            }

            let aligned_base = (region.base.0 + alignment - 1) & !(alignment - 1);
            let offset = aligned_base - region.base.0;

            if region.size >= offset + size {
                // Check not reserved
                let aligned_addr = PhysicalAddress(aligned_base);
                if !self.overlaps_reserved(aligned_addr, size) {
                    return Some(aligned_addr);
                }
            }
        }
        None
    }

    /// Find free region above address
    pub fn find_free_region_above(
        &self,
        min_address: PhysicalAddress,
        size: u64,
        alignment: u64,
    ) -> Option<PhysicalAddress> {
        for region in &self.regions {
            if region.region_type != RegionType::Usable {
                continue;
            }

            // Check if region is above minimum
            let region_end = region.base.0 + region.size;
            if region_end <= min_address.0 {
                continue;
            }

            let start = region.base.0.max(min_address.0);
            let aligned_start = (start + alignment - 1) & !(alignment - 1);

            if aligned_start < region_end && region_end - aligned_start >= size {
                if !self.overlaps_reserved(PhysicalAddress(aligned_start), size) {
                    return Some(PhysicalAddress(aligned_start));
                }
            }
        }
        None
    }

    /// Sort regions by base address
    pub fn sort_by_address(&mut self) {
        self.regions.sort_by_key(|r| r.base);
    }

    /// Merge contiguous regions of same type
    pub fn merge_contiguous(&mut self) {
        if self.regions.len() < 2 {
            return;
        }

        self.sort_by_address();

        let mut merged = Vec::with_capacity(self.regions.len());
        let mut current = self.regions[0].clone();

        for region in &self.regions[1..] {
            let current_end = current.base + current.size;

            if region.base == current_end
                && region.region_type == current.region_type
                && region.attributes == current.attributes
            {
                current.size += region.size;
            } else {
                merged.push(current);
                current = region.clone();
            }
        }
        merged.push(current);

        self.regions = merged;
    }

    /// Get total memory of type
    pub fn total_memory_of_type(&self, region_type: RegionType) -> u64 {
        self.regions
            .iter()
            .filter(|r| r.region_type == region_type)
            .map(|r| r.size)
            .sum()
    }

    /// Get total usable memory
    pub fn total_usable(&self) -> u64 {
        self.total_memory_of_type(RegionType::Usable)
    }

    /// Get region count
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    /// Get reserved count
    pub fn reserved_count(&self) -> usize {
        self.reserved.len()
    }

    /// Clear all regions
    pub fn clear(&mut self) {
        self.regions.clear();
        self.reserved.clear();
    }
}

impl Default for RegionManager {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// MEMORY REGION
// =============================================================================

/// Memory region
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    /// Base address
    pub base: PhysicalAddress,
    /// Size in bytes
    pub size: u64,
    /// Region type
    pub region_type: RegionType,
    /// Attributes
    pub attributes: RegionAttributes,
}

impl MemoryRegion {
    /// Create new region
    pub fn new(base: PhysicalAddress, size: u64, region_type: RegionType) -> Self {
        Self {
            base,
            size,
            region_type,
            attributes: RegionAttributes::default(),
        }
    }

    /// Get end address
    pub fn end(&self) -> PhysicalAddress {
        self.base + self.size
    }

    /// Check if contains address
    pub fn contains(&self, address: PhysicalAddress) -> bool {
        address >= self.base && address < self.end()
    }

    /// Check if overlaps with another region
    pub fn overlaps(&self, other: &MemoryRegion) -> bool {
        self.base < other.end() && other.base < self.end()
    }

    /// Get intersection with another region
    pub fn intersection(&self, other: &MemoryRegion) -> Option<MemoryRegion> {
        if !self.overlaps(other) {
            return None;
        }

        let start = self.base.max(other.base);
        let end = self.end().min(other.end());

        Some(MemoryRegion {
            base: start,
            size: end - start,
            region_type: self.region_type,
            attributes: self.attributes,
        })
    }

    /// Split region at address
    pub fn split_at(&self, address: PhysicalAddress) -> Option<(MemoryRegion, MemoryRegion)> {
        if !self.contains(address) || address == self.base {
            return None;
        }

        let first = MemoryRegion {
            base: self.base,
            size: address - self.base,
            region_type: self.region_type,
            attributes: self.attributes,
        };

        let second = MemoryRegion {
            base: address,
            size: self.end() - address,
            region_type: self.region_type,
            attributes: self.attributes,
        };

        Some((first, second))
    }

    /// Get size in pages
    pub fn pages(&self) -> u64 {
        (self.size + 0xFFF) / 4096
    }
}

// =============================================================================
// REGION TYPE
// =============================================================================

/// Memory region type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionType {
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
    /// Memory mapped I/O port space
    MmioPortSpace,
    /// PAL code (IA64)
    PalCode,
    /// Persistent memory
    Persistent,
    /// Kernel
    Kernel,
    /// Kernel stack
    KernelStack,
    /// Page tables
    PageTables,
    /// Framebuffer
    Framebuffer,
    /// Unknown
    Unknown,
}

impl RegionType {
    /// Check if region is usable for general allocation
    pub fn is_usable(&self) -> bool {
        matches!(self, Self::Usable | Self::Loader | Self::BootServices)
    }

    /// Check if region must be preserved
    pub fn is_preserved(&self) -> bool {
        matches!(
            self,
            Self::RuntimeServices
                | Self::AcpiNvs
                | Self::Reserved
                | Self::Kernel
                | Self::PageTables
        )
    }

    /// Check if region is reclaimable after boot
    pub fn is_reclaimable(&self) -> bool {
        matches!(
            self,
            Self::Loader | Self::BootServices | Self::AcpiReclaimable
        )
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Usable => "Usable",
            Self::Reserved => "Reserved",
            Self::AcpiReclaimable => "ACPI Reclaimable",
            Self::AcpiNvs => "ACPI NVS",
            Self::BadMemory => "Bad Memory",
            Self::Loader => "Loader",
            Self::BootServices => "Boot Services",
            Self::RuntimeServices => "Runtime Services",
            Self::Mmio => "MMIO",
            Self::MmioPortSpace => "MMIO Port Space",
            Self::PalCode => "PAL Code",
            Self::Persistent => "Persistent",
            Self::Kernel => "Kernel",
            Self::KernelStack => "Kernel Stack",
            Self::PageTables => "Page Tables",
            Self::Framebuffer => "Framebuffer",
            Self::Unknown => "Unknown",
        }
    }
}

// =============================================================================
// REGION ATTRIBUTES
// =============================================================================

/// Region attributes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RegionAttributes {
    /// Uncacheable
    pub uncacheable: bool,
    /// Write combining
    pub write_combining: bool,
    /// Write through
    pub write_through: bool,
    /// Write back
    pub write_back: bool,
    /// Uncacheable exported
    pub uncacheable_exported: bool,
    /// Write protected
    pub write_protected: bool,
    /// Read protected
    pub read_protected: bool,
    /// Execute protected
    pub execute_protected: bool,
    /// Non-volatile
    pub non_volatile: bool,
    /// More reliable
    pub more_reliable: bool,
    /// Read only
    pub read_only: bool,
    /// Specific-purpose memory
    pub specific_purpose: bool,
    /// Crypto capable
    pub crypto: bool,
    /// Runtime accessible
    pub runtime: bool,
}

impl RegionAttributes {
    /// Create from UEFI memory attribute
    pub fn from_uefi(attr: u64) -> Self {
        Self {
            uncacheable: (attr & 0x1) != 0,
            write_combining: (attr & 0x2) != 0,
            write_through: (attr & 0x4) != 0,
            write_back: (attr & 0x8) != 0,
            uncacheable_exported: (attr & 0x10) != 0,
            write_protected: (attr & 0x1000) != 0,
            read_protected: (attr & 0x2000) != 0,
            execute_protected: (attr & 0x4000) != 0,
            non_volatile: (attr & 0x8000) != 0,
            more_reliable: (attr & 0x10000) != 0,
            read_only: (attr & 0x20000) != 0,
            specific_purpose: (attr & 0x40000) != 0,
            crypto: (attr & 0x80000) != 0,
            runtime: (attr & 0x8000000000000000) != 0,
        }
    }

    /// Convert to UEFI memory attribute
    pub fn to_uefi(&self) -> u64 {
        let mut attr = 0u64;
        if self.uncacheable {
            attr |= 0x1;
        }
        if self.write_combining {
            attr |= 0x2;
        }
        if self.write_through {
            attr |= 0x4;
        }
        if self.write_back {
            attr |= 0x8;
        }
        if self.uncacheable_exported {
            attr |= 0x10;
        }
        if self.write_protected {
            attr |= 0x1000;
        }
        if self.read_protected {
            attr |= 0x2000;
        }
        if self.execute_protected {
            attr |= 0x4000;
        }
        if self.non_volatile {
            attr |= 0x8000;
        }
        if self.more_reliable {
            attr |= 0x10000;
        }
        if self.read_only {
            attr |= 0x20000;
        }
        if self.specific_purpose {
            attr |= 0x40000;
        }
        if self.crypto {
            attr |= 0x80000;
        }
        if self.runtime {
            attr |= 0x8000000000000000;
        }
        attr
    }
}

// =============================================================================
// RESERVED REGION
// =============================================================================

/// Reserved memory region with name
#[derive(Debug, Clone)]
pub struct ReservedRegion {
    /// Base address
    pub base: PhysicalAddress,
    /// Size
    pub size: u64,
    /// Name/purpose
    pub name: String,
}

impl ReservedRegion {
    /// Get end address
    pub fn end(&self) -> PhysicalAddress {
        self.base + self.size
    }

    /// Check if contains address
    pub fn contains(&self, address: PhysicalAddress) -> bool {
        address >= self.base && address < self.end()
    }
}

// =============================================================================
// KERNEL REGIONS
// =============================================================================

/// Standard kernel memory regions
pub struct KernelRegions {
    /// Kernel code
    pub code: Option<MemoryRegion>,
    /// Kernel data
    pub data: Option<MemoryRegion>,
    /// Kernel rodata
    pub rodata: Option<MemoryRegion>,
    /// Kernel BSS
    pub bss: Option<MemoryRegion>,
    /// Kernel stack
    pub stack: Option<MemoryRegion>,
    /// Kernel heap
    pub heap: Option<MemoryRegion>,
    /// Page tables
    pub page_tables: Option<MemoryRegion>,
    /// Boot info
    pub boot_info: Option<MemoryRegion>,
    /// Initrd/ramdisk
    pub initrd: Option<MemoryRegion>,
    /// Framebuffer
    pub framebuffer: Option<MemoryRegion>,
}

impl KernelRegions {
    /// Create empty kernel regions
    pub fn new() -> Self {
        Self {
            code: None,
            data: None,
            rodata: None,
            bss: None,
            stack: None,
            heap: None,
            page_tables: None,
            boot_info: None,
            initrd: None,
            framebuffer: None,
        }
    }

    /// Get all defined regions
    pub fn all_regions(&self) -> Vec<&MemoryRegion> {
        let mut regions = Vec::new();
        if let Some(ref r) = self.code {
            regions.push(r);
        }
        if let Some(ref r) = self.data {
            regions.push(r);
        }
        if let Some(ref r) = self.rodata {
            regions.push(r);
        }
        if let Some(ref r) = self.bss {
            regions.push(r);
        }
        if let Some(ref r) = self.stack {
            regions.push(r);
        }
        if let Some(ref r) = self.heap {
            regions.push(r);
        }
        if let Some(ref r) = self.page_tables {
            regions.push(r);
        }
        if let Some(ref r) = self.boot_info {
            regions.push(r);
        }
        if let Some(ref r) = self.initrd {
            regions.push(r);
        }
        if let Some(ref r) = self.framebuffer {
            regions.push(r);
        }
        regions
    }

    /// Get total kernel memory usage
    pub fn total_size(&self) -> u64 {
        self.all_regions().iter().map(|r| r.size).sum()
    }
}

impl Default for KernelRegions {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// REGION BUILDER
// =============================================================================

/// Builder for constructing memory region layouts
pub struct RegionBuilder {
    /// Current base address
    current_base: PhysicalAddress,
    /// Alignment
    alignment: u64,
    /// Regions
    regions: Vec<MemoryRegion>,
}

impl RegionBuilder {
    /// Create new builder
    pub fn new(base: PhysicalAddress) -> Self {
        Self {
            current_base: base,
            alignment: 4096,
            regions: Vec::new(),
        }
    }

    /// Set alignment
    pub fn align(mut self, alignment: u64) -> Self {
        self.alignment = alignment;
        self
    }

    /// Add region
    pub fn add(mut self, size: u64, region_type: RegionType) -> Self {
        // Align base
        let aligned = (self.current_base.0 + self.alignment - 1) & !(self.alignment - 1);

        self.regions.push(MemoryRegion {
            base: PhysicalAddress(aligned),
            size,
            region_type,
            attributes: RegionAttributes::default(),
        });

        self.current_base = PhysicalAddress(aligned + size);
        self
    }

    /// Add region with specific attributes
    pub fn add_with_attributes(
        mut self,
        size: u64,
        region_type: RegionType,
        attributes: RegionAttributes,
    ) -> Self {
        let aligned = (self.current_base.0 + self.alignment - 1) & !(self.alignment - 1);

        self.regions.push(MemoryRegion {
            base: PhysicalAddress(aligned),
            size,
            region_type,
            attributes,
        });

        self.current_base = PhysicalAddress(aligned + size);
        self
    }

    /// Skip bytes
    pub fn skip(mut self, size: u64) -> Self {
        self.current_base = PhysicalAddress(self.current_base.0 + size);
        self
    }

    /// Align to boundary
    pub fn align_to(mut self, boundary: u64) -> Self {
        self.current_base = PhysicalAddress((self.current_base.0 + boundary - 1) & !(boundary - 1));
        self
    }

    /// Get current address
    pub fn current(&self) -> PhysicalAddress {
        self.current_base
    }

    /// Build regions
    pub fn build(self) -> Vec<MemoryRegion> {
        self.regions
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_contains() {
        let region = MemoryRegion::new(0x1000, 0x1000, RegionType::Usable);
        assert!(region.contains(0x1000));
        assert!(region.contains(0x1500));
        assert!(!region.contains(0x2000));
        assert!(!region.contains(0x500));
    }

    #[test]
    fn test_region_overlaps() {
        let r1 = MemoryRegion::new(0x1000, 0x1000, RegionType::Usable);
        let r2 = MemoryRegion::new(0x1800, 0x1000, RegionType::Usable);
        let r3 = MemoryRegion::new(0x2000, 0x1000, RegionType::Usable);

        assert!(r1.overlaps(&r2));
        assert!(!r1.overlaps(&r3));
    }

    #[test]
    fn test_region_manager() {
        let mut manager = RegionManager::new();
        manager
            .add_region(0x1000, 0x10000, RegionType::Usable)
            .unwrap();
        manager
            .add_region(0x20000, 0x5000, RegionType::Reserved)
            .unwrap();

        assert_eq!(manager.region_count(), 2);
        assert_eq!(manager.total_usable(), 0x10000);
    }

    #[test]
    fn test_region_builder() {
        let regions = RegionBuilder::new(0x100000)
            .align(0x1000)
            .add(0x10000, RegionType::Kernel)
            .add(0x1000, RegionType::KernelStack)
            .build();

        assert_eq!(regions.len(), 2);
        assert_eq!(regions[0].base, 0x100000);
        assert_eq!(regions[1].base, 0x110000);
    }
}
