//! # Helix Memory Subsystem
//!
//! The memory subsystem provides:
//! - Physical memory management framework
//! - Virtual memory management framework
//! - Allocator framework
//! - Memory region tracking
//! - Memory protection
//!
//! ## Key Principle
//!
//! Like all Helix subsystems, this provides FRAMEWORKS, not implementations.
//! Actual allocators are provided as modules that can be swapped.

#![no_std]
#![feature(allocator_api)]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

extern crate alloc;

pub mod allocator;
pub mod physical;
pub mod protection;
pub mod region;
pub mod virtual_memory;

use helix_hal::{PageSize, PhysAddr, VirtAddr};

/// Memory subsystem result type
pub type MemResult<T> = Result<T, MemError>;

/// Memory subsystem errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemError {
    /// Out of memory
    OutOfMemory,
    /// Invalid address
    InvalidAddress,
    /// Invalid size
    InvalidSize,
    /// Address not aligned
    NotAligned,
    /// Region already mapped
    AlreadyMapped,
    /// Region not mapped
    NotMapped,
    /// Permission denied
    PermissionDenied,
    /// Invalid region
    InvalidRegion,
    /// Allocator not initialized
    NotInitialized,
    /// Internal error
    Internal,
}

/// Memory statistics
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    /// Total physical memory
    pub total_physical: u64,
    /// Free physical memory
    pub free_physical: u64,
    /// Used physical memory
    pub used_physical: u64,
    /// Total virtual memory
    pub total_virtual: u64,
    /// Used virtual memory
    pub used_virtual: u64,
    /// Number of allocations
    pub allocations: u64,
    /// Number of deallocations
    pub deallocations: u64,
}

/// Frame - represents a physical page
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame {
    /// Physical address of the frame
    address: PhysAddr,
    /// Size of the frame
    size: PageSize,
}

impl Frame {
    /// Create a new frame
    pub const fn new(address: PhysAddr, size: PageSize) -> Self {
        Self { address, size }
    }

    /// Get the physical address
    pub fn address(&self) -> PhysAddr {
        self.address
    }

    /// Get the size
    pub fn size(&self) -> PageSize {
        self.size
    }

    /// Get the end address
    pub fn end(&self) -> PhysAddr {
        PhysAddr::new(self.address.as_u64() + self.size.size() as u64)
    }

    /// Check if address is within this frame
    pub fn contains(&self, addr: PhysAddr) -> bool {
        addr >= self.address && addr < self.end()
    }
}

/// Page - represents a virtual page
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Page {
    /// Virtual address of the page
    address: VirtAddr,
    /// Size of the page
    size: PageSize,
}

impl Page {
    /// Create a new page
    pub const fn new(address: VirtAddr, size: PageSize) -> Self {
        Self { address, size }
    }

    /// Get the virtual address
    pub fn address(&self) -> VirtAddr {
        self.address
    }

    /// Get the size
    pub fn size(&self) -> PageSize {
        self.size
    }

    /// Get the end address
    pub fn end(&self) -> VirtAddr {
        VirtAddr::new(self.address.as_u64() + self.size.size() as u64)
    }

    /// Check if address is within this page
    pub fn contains(&self, addr: VirtAddr) -> bool {
        addr >= self.address && addr < self.end()
    }
}

/// Memory zone types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryZone {
    /// DMA-capable memory (first 16MB on x86)
    Dma,
    /// DMA32 memory (first 4GB on x86_64)
    Dma32,
    /// Normal memory
    Normal,
    /// High memory (above normal zone)
    High,
    /// Device memory (MMIO)
    Device,
}
