//! # Memory Region Management

use crate::MemResult;
use helix_hal::PhysAddr;
use alloc::vec::Vec;
use spin::RwLock;

/// Memory region descriptor
#[derive(Debug, Clone)]
pub struct MemoryRegionDescriptor {
    /// Start address
    pub start: PhysAddr,
    /// Size in bytes
    pub size: u64,
    /// Region type
    pub region_type: RegionType,
    /// Region name
    pub name: &'static str,
    /// Is reserved
    pub reserved: bool,
}

/// Region types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionType {
    /// RAM
    Ram,
    /// ROM
    Rom,
    /// Device memory (MMIO)
    Device,
    /// Reserved
    Reserved,
    /// ACPI tables
    Acpi,
    /// Kernel
    Kernel,
    /// Initrd
    Initrd,
    /// Framebuffer
    Framebuffer,
}

/// Memory region manager
pub struct RegionManager {
    /// All regions
    regions: RwLock<Vec<MemoryRegionDescriptor>>,
}

impl RegionManager {
    /// Create a new manager
    pub const fn new() -> Self {
        Self {
            regions: RwLock::new(Vec::new()),
        }
    }

    /// Add a region
    pub fn add(&self, region: MemoryRegionDescriptor) {
        self.regions.write().push(region);
    }

    /// Get all regions
    pub fn all(&self) -> Vec<MemoryRegionDescriptor> {
        self.regions.read().clone()
    }

    /// Get regions by type
    pub fn by_type(&self, t: RegionType) -> Vec<MemoryRegionDescriptor> {
        self.regions.read()
            .iter()
            .filter(|r| r.region_type == t)
            .cloned()
            .collect()
    }

    /// Get usable RAM regions
    pub fn usable_ram(&self) -> Vec<MemoryRegionDescriptor> {
        self.regions.read()
            .iter()
            .filter(|r| r.region_type == RegionType::Ram && !r.reserved)
            .cloned()
            .collect()
    }

    /// Total RAM
    pub fn total_ram(&self) -> u64 {
        self.regions.read()
            .iter()
            .filter(|r| r.region_type == RegionType::Ram)
            .map(|r| r.size)
            .sum()
    }

    /// Reserve a region
    pub fn reserve(&self, start: PhysAddr, size: u64, name: &'static str) -> MemResult<()> {
        self.regions.write().push(MemoryRegionDescriptor {
            start,
            size,
            region_type: RegionType::Reserved,
            name,
            reserved: true,
        });
        Ok(())
    }
}

/// Global region manager
static REGIONS: RegionManager = RegionManager::new();

/// Get the region manager
pub fn region_manager() -> &'static RegionManager {
    &REGIONS
}
