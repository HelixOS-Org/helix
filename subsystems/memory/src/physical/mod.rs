//! # Physical Memory Management
//!
//! Framework for physical memory allocation.

pub mod bitmap;
pub mod buddy;
pub mod frame_allocator;

use alloc::sync::Arc;
use alloc::vec::Vec;

use helix_hal::{PageSize, PhysAddr};
use spin::RwLock;

use crate::{Frame, MemError, MemResult, MemoryZone};

/// Physical memory allocator trait
///
/// Implementations provide actual allocation algorithms.
pub trait PhysicalAllocator: Send + Sync {
    /// Get allocator name
    fn name(&self) -> &'static str;

    /// Initialize with memory regions
    fn init(&mut self, regions: &[PhysicalRegion]) -> MemResult<()>;

    /// Allocate a single frame
    fn allocate(&self, size: PageSize) -> MemResult<Frame>;

    /// Allocate multiple contiguous frames
    fn allocate_contiguous(&self, count: usize, size: PageSize) -> MemResult<Frame>;

    /// Allocate from a specific zone
    fn allocate_zone(&self, size: PageSize, zone: MemoryZone) -> MemResult<Frame>;

    /// Deallocate a frame
    fn deallocate(&self, frame: Frame) -> MemResult<()>;

    /// Get free frame count
    fn free_frames(&self) -> usize;

    /// Get total frame count
    fn total_frames(&self) -> usize;

    /// Get statistics
    fn stats(&self) -> AllocatorStats;
}

/// Physical memory region
#[derive(Debug, Clone)]
pub struct PhysicalRegion {
    /// Start address
    pub start: PhysAddr,
    /// Size in bytes
    pub size: u64,
    /// Region type
    pub region_type: PhysicalRegionType,
    /// Memory zone
    pub zone: MemoryZone,
}

impl PhysicalRegion {
    /// Create a new region
    pub fn new(start: PhysAddr, size: u64, region_type: PhysicalRegionType) -> Self {
        let zone = Self::determine_zone(start);
        Self {
            start,
            size,
            region_type,
            zone,
        }
    }

    /// Determine zone based on address
    fn determine_zone(addr: PhysAddr) -> MemoryZone {
        let addr = addr.as_u64();
        if addr < 0x100_0000 {
            MemoryZone::Dma
        } else if addr < 0x1_0000_0000 {
            MemoryZone::Dma32
        } else {
            MemoryZone::Normal
        }
    }

    /// Get end address
    pub fn end(&self) -> PhysAddr {
        PhysAddr::new(self.start.as_u64() + self.size)
    }

    /// Check if usable
    pub fn is_usable(&self) -> bool {
        self.region_type == PhysicalRegionType::Usable
    }
}

/// Physical region types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalRegionType {
    /// Usable RAM
    Usable,
    /// Reserved by firmware
    Reserved,
    /// ACPI tables (reclaimable)
    AcpiReclaimable,
    /// ACPI NVS
    AcpiNvs,
    /// Bad memory
    Bad,
    /// Kernel code/data
    Kernel,
    /// Bootloader
    Bootloader,
    /// Device MMIO
    Device,
}

/// Allocator statistics
#[derive(Debug, Clone, Default)]
pub struct AllocatorStats {
    /// Total allocations
    pub allocations: u64,
    /// Total deallocations
    pub deallocations: u64,
    /// Current allocation count
    pub current_allocations: u64,
    /// Peak allocation count
    pub peak_allocations: u64,
    /// Total bytes allocated
    pub bytes_allocated: u64,
    /// Total bytes freed
    pub bytes_freed: u64,
    /// Fragmentation estimate (0-100)
    pub fragmentation: u8,
}

/// Physical memory manager
pub struct PhysicalMemoryManager {
    /// Current allocator
    allocator: RwLock<Option<Arc<dyn PhysicalAllocator>>>,
    /// Memory regions
    regions: RwLock<Vec<PhysicalRegion>>,
}

impl PhysicalMemoryManager {
    /// Create a new manager
    pub const fn new() -> Self {
        Self {
            allocator: RwLock::new(None),
            regions: RwLock::new(Vec::new()),
        }
    }

    /// Set the allocator
    pub fn set_allocator(&self, allocator: Arc<dyn PhysicalAllocator>) {
        *self.allocator.write() = Some(allocator);
    }

    /// Add a memory region
    pub fn add_region(&self, region: PhysicalRegion) {
        self.regions.write().push(region);
    }

    /// Initialize with all regions
    pub fn init(&self) -> MemResult<()> {
        let _regions = self.regions.read().clone();
        let mut allocator = self.allocator.write();

        if let Some(ref mut _alloc) = *allocator {
            // Can't mutate through Arc, would need different design
            // For now, just verify it's set
            Ok(())
        } else {
            Err(MemError::NotInitialized)
        }
    }

    /// Allocate a frame
    pub fn allocate(&self, size: PageSize) -> MemResult<Frame> {
        self.allocator
            .read()
            .as_ref()
            .ok_or(MemError::NotInitialized)?
            .allocate(size)
    }

    /// Deallocate a frame
    pub fn deallocate(&self, frame: Frame) -> MemResult<()> {
        self.allocator
            .read()
            .as_ref()
            .ok_or(MemError::NotInitialized)?
            .deallocate(frame)
    }
}

/// Global physical memory manager
static PMM: PhysicalMemoryManager = PhysicalMemoryManager::new();

/// Get the physical memory manager
pub fn pmm() -> &'static PhysicalMemoryManager {
    &PMM
}
