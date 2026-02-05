//! Advanced Memory Management
//!
//! Comprehensive memory management for UEFI boot environment.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    Memory Management Layer                       │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │  ┌──────────────────┐  ┌──────────────────┐  ┌───────────────┐ │
//! │  │   Memory Map     │  │   Page Tables    │  │   Allocator   │ │
//! │  │  ┌────────────┐  │  │  ┌────────────┐  │  │ ┌───────────┐ │ │
//! │  │  │ Descriptor │  │  │  │   PML4     │  │  │ │   Pool    │ │ │
//! │  │  │  Iterator  │  │  │  │   PDPT     │  │  │ │   Page    │ │ │
//! │  │  │  Regions   │  │  │  │   PD/PT    │  │  │ │   Slab    │ │ │
//! │  │  └────────────┘  │  │  └────────────┘  │  │ └───────────┘ │ │
//! │  └──────────────────┘  └──────────────────┘  └───────────────┘ │
//! │                                                                 │
//! │  ┌──────────────────┐  ┌──────────────────┐  ┌───────────────┐ │
//! │  │  Virtual Memory  │  │   Address Map    │  │   Regions     │ │
//! │  │  ┌────────────┐  │  │  ┌────────────┐  │  │ ┌───────────┐ │ │
//! │  │  │  Mapping   │  │  │  │  Physical  │  │  │ │ Kernel    │ │ │
//! │  │  │  Unmapping │  │  │  │  Virtual   │  │  │ │ Stack     │ │ │
//! │  │  │  Protect   │  │  │  │  Reserved  │  │  │ │ Heap      │ │ │
//! │  │  └────────────┘  │  │  └────────────┘  │  │ └───────────┘ │ │
//! │  └──────────────────┘  └──────────────────┘  └───────────────┘ │
//! │                                                                 │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

pub mod allocator;
pub mod map;
pub mod paging;
pub mod regions;
pub mod virtual_memory;

use crate::error::Result;
use crate::raw::types::*;

extern crate alloc;

// =============================================================================
// MEMORY MANAGER
// =============================================================================

/// Memory manager for boot environment
pub struct MemoryManager {
    /// Memory map
    map: Option<map::MemoryMapInfo>,
    /// Page table manager
    paging: Option<paging::PageTableManager>,
    /// Memory regions
    regions: regions::RegionManager,
    /// Statistics
    stats: MemoryStats,
}

impl MemoryManager {
    /// Create new memory manager
    pub fn new() -> Self {
        Self {
            map: None,
            paging: None,
            regions: regions::RegionManager::new(),
            stats: MemoryStats::default(),
        }
    }

    /// Initialize from UEFI memory map
    pub unsafe fn init_from_memory_map(
        &mut self,
        map_key: usize,
        descriptors: &[crate::raw::memory::MemoryDescriptor],
        descriptor_size: usize,
        descriptor_version: u32,
    ) -> Result<()> {
        // Create memory map info
        self.map = Some(map::MemoryMapInfo::new(
            map_key,
            descriptors.to_vec(),
            descriptor_size,
            descriptor_version,
        ));

        // Calculate statistics
        self.calculate_stats();

        // Initialize regions
        self.init_regions()?;

        Ok(())
    }

    /// Calculate memory statistics
    fn calculate_stats(&mut self) {
        if let Some(ref map) = self.map {
            let mut total = 0u64;
            let mut usable = 0u64;
            let mut reserved = 0u64;
            let mut runtime = 0u64;
            let mut acpi = 0u64;
            let mut mmio = 0u64;
            let mut loader = 0u64;
            let mut boot = 0u64;

            for desc in map.descriptors() {
                let size = desc.number_of_pages * 4096;
                total += size;

                use crate::raw::memory::MemoryType;
                let mt = desc.memory_type;
                if mt == MemoryType::ConventionalMemory as u32 {
                    usable += size;
                } else if mt == MemoryType::LoaderCode as u32 || mt == MemoryType::LoaderData as u32
                {
                    loader += size;
                } else if mt == MemoryType::BootServicesCode as u32
                    || mt == MemoryType::BootServicesData as u32
                {
                    boot += size;
                } else if mt == MemoryType::RuntimeServicesCode as u32
                    || mt == MemoryType::RuntimeServicesData as u32
                {
                    runtime += size;
                } else if mt == MemoryType::AcpiReclaimMemory as u32
                    || mt == MemoryType::AcpiNvsMemory as u32
                {
                    acpi += size;
                } else if mt == MemoryType::MemoryMappedIo as u32
                    || mt == MemoryType::MemoryMappedIoPortSpace as u32
                {
                    mmio += size;
                } else if mt == MemoryType::ReservedMemoryType as u32
                    || mt == MemoryType::UnusableMemory as u32
                {
                    reserved += size;
                }
            }

            self.stats = MemoryStats {
                total_memory: total,
                usable_memory: usable,
                reserved_memory: reserved,
                runtime_memory: runtime,
                acpi_memory: acpi,
                mmio_memory: mmio,
                loader_memory: loader,
                boot_services_memory: boot,
                descriptor_count: map.descriptor_count(),
            };
        }
    }

    /// Initialize memory regions
    fn init_regions(&mut self) -> Result<()> {
        if let Some(ref map) = self.map {
            for desc in map.descriptors() {
                use crate::raw::memory::MemoryType;
                let mt = desc.memory_type;
                let region_type = if mt == MemoryType::ConventionalMemory as u32 {
                    regions::RegionType::Usable
                } else if mt == MemoryType::ReservedMemory as u32 {
                    regions::RegionType::Reserved
                } else if mt == MemoryType::AcpiReclaimMemory as u32 {
                    regions::RegionType::AcpiReclaimable
                } else if mt == MemoryType::AcpiNvsMemory as u32 {
                    regions::RegionType::AcpiNvs
                } else if mt == MemoryType::MemoryMappedIo as u32 {
                    regions::RegionType::Mmio
                } else if mt == MemoryType::RuntimeServicesCode as u32
                    || mt == MemoryType::RuntimeServicesData as u32
                {
                    regions::RegionType::RuntimeServices
                } else if mt == MemoryType::BootServicesCode as u32
                    || mt == MemoryType::BootServicesData as u32
                {
                    regions::RegionType::BootServices
                } else if mt == MemoryType::LoaderCode as u32 || mt == MemoryType::LoaderData as u32
                {
                    regions::RegionType::Loader
                } else {
                    regions::RegionType::Unknown
                };

                self.regions.add_region(
                    desc.physical_start,
                    desc.number_of_pages * 4096,
                    region_type,
                )?;
            }
        }
        Ok(())
    }

    /// Get memory map
    pub fn memory_map(&self) -> Option<&map::MemoryMapInfo> {
        self.map.as_ref()
    }

    /// Get memory statistics
    pub fn stats(&self) -> &MemoryStats {
        &self.stats
    }

    /// Get region manager
    pub fn regions(&self) -> &regions::RegionManager {
        &self.regions
    }

    /// Get mutable region manager
    pub fn regions_mut(&mut self) -> &mut regions::RegionManager {
        &mut self.regions
    }

    /// Initialize paging
    pub unsafe fn init_paging(&mut self) -> Result<()> {
        self.paging = Some(paging::PageTableManager::new());
        if let Some(ref mut paging) = self.paging {
            paging.init()?;
        }
        Ok(())
    }

    /// Get page table manager
    pub fn paging(&self) -> Option<&paging::PageTableManager> {
        self.paging.as_ref()
    }

    /// Get mutable page table manager
    pub fn paging_mut(&mut self) -> Option<&mut paging::PageTableManager> {
        self.paging.as_mut()
    }

    /// Find largest usable region
    pub fn find_largest_usable_region(&self) -> Option<(PhysicalAddress, u64)> {
        self.regions
            .find_largest_by_type(regions::RegionType::Usable)
    }

    /// Find suitable region for kernel
    pub fn find_kernel_region(&self, size: u64, alignment: u64) -> Option<PhysicalAddress> {
        if let Some(ref map) = self.map {
            for desc in map.descriptors() {
                let mt = desc.memory_type;
                if mt != crate::raw::memory::MemoryType::ConventionalMemory as u32 {
                    continue;
                }

                let region_size = desc.number_of_pages * 4096;
                if region_size < size {
                    continue;
                }

                // Align start address
                let aligned_start = (desc.physical_start.0 + alignment - 1) & !(alignment - 1);
                let offset = aligned_start - desc.physical_start.0;

                if region_size - offset >= size {
                    // Prefer higher memory
                    if aligned_start >= 0x100000 {
                        // Above 1MB
                        return Some(PhysicalAddress(aligned_start));
                    }
                }
            }
        }
        None
    }

    /// Reserve memory region
    pub fn reserve_region(&mut self, base: PhysicalAddress, size: u64) -> Result<()> {
        self.regions.mark_reserved(base, size)
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// MEMORY STATISTICS
// =============================================================================

/// Memory statistics
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    /// Total physical memory
    pub total_memory: u64,
    /// Usable memory
    pub usable_memory: u64,
    /// Reserved memory
    pub reserved_memory: u64,
    /// Runtime services memory
    pub runtime_memory: u64,
    /// ACPI memory
    pub acpi_memory: u64,
    /// MMIO memory
    pub mmio_memory: u64,
    /// Loader memory
    pub loader_memory: u64,
    /// Boot services memory
    pub boot_services_memory: u64,
    /// Number of memory descriptors
    pub descriptor_count: usize,
}

impl MemoryStats {
    /// Get total memory in MB
    pub fn total_mb(&self) -> u64 {
        self.total_memory / (1024 * 1024)
    }

    /// Get usable memory in MB
    pub fn usable_mb(&self) -> u64 {
        self.usable_memory / (1024 * 1024)
    }

    /// Get percentage of usable memory
    pub fn usable_percentage(&self) -> f32 {
        if self.total_memory == 0 {
            0.0
        } else {
            (self.usable_memory as f32 / self.total_memory as f32) * 100.0
        }
    }
}

// =============================================================================
// CONSTANTS
// =============================================================================

/// Page size (4 KiB)
pub const PAGE_SIZE: u64 = 4096;

/// Large page size (2 MiB)
pub const LARGE_PAGE_SIZE: u64 = 2 * 1024 * 1024;

/// Huge page size (1 GiB)
pub const HUGE_PAGE_SIZE: u64 = 1024 * 1024 * 1024;

/// Page shift
pub const PAGE_SHIFT: u32 = 12;

/// Large page shift
pub const LARGE_PAGE_SHIFT: u32 = 21;

/// Huge page shift
pub const HUGE_PAGE_SHIFT: u32 = 30;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Align address up to page boundary
pub const fn page_align_up(addr: u64) -> u64 {
    (addr + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

/// Align address down to page boundary
pub const fn page_align_down(addr: u64) -> u64 {
    addr & !(PAGE_SIZE - 1)
}

/// Calculate number of pages for size
pub const fn size_to_pages(size: u64) -> u64 {
    (size + PAGE_SIZE - 1) / PAGE_SIZE
}

/// Calculate size from pages
pub const fn pages_to_size(pages: u64) -> u64 {
    pages * PAGE_SIZE
}

/// Check if address is page aligned
pub const fn is_page_aligned(addr: u64) -> bool {
    (addr & (PAGE_SIZE - 1)) == 0
}

/// Align to arbitrary boundary
pub const fn align_up(addr: u64, alignment: u64) -> u64 {
    (addr + alignment - 1) & !(alignment - 1)
}

/// Align down to arbitrary boundary
pub const fn align_down(addr: u64, alignment: u64) -> u64 {
    addr & !(alignment - 1)
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_page_align_up() {
        assert_eq!(page_align_up(0), 0);
        assert_eq!(page_align_up(1), 4096);
        assert_eq!(page_align_up(4095), 4096);
        assert_eq!(page_align_up(4096), 4096);
        assert_eq!(page_align_up(4097), 8192);
    }

    #[test]
    fn test_page_align_down() {
        assert_eq!(page_align_down(0), 0);
        assert_eq!(page_align_down(4095), 0);
        assert_eq!(page_align_down(4096), 4096);
        assert_eq!(page_align_down(8191), 4096);
    }

    #[test]
    fn test_size_to_pages() {
        assert_eq!(size_to_pages(0), 0);
        assert_eq!(size_to_pages(1), 1);
        assert_eq!(size_to_pages(4096), 1);
        assert_eq!(size_to_pages(4097), 2);
    }

    #[test]
    fn test_is_page_aligned() {
        assert!(is_page_aligned(0));
        assert!(is_page_aligned(4096));
        assert!(!is_page_aligned(1));
        assert!(!is_page_aligned(4095));
    }
}
