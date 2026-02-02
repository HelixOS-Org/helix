//! # VRAM Heap Management
//!
//! GPU memory heaps with different characteristics.

use magma_core::{ByteSize, Error, GpuAddr, Result};

use crate::buddy::BuddyAllocator;

// =============================================================================
// HEAP TYPES
// =============================================================================

/// GPU memory heap type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HeapType {
    /// Device local (VRAM) - fastest
    DeviceLocal,
    /// Host visible (BAR aperture)
    HostVisible,
    /// Host cached (system memory)
    HostCached,
}

impl HeapType {
    /// Get priority (higher = prefer for GPU access)
    pub const fn priority(&self) -> u8 {
        match self {
            HeapType::DeviceLocal => 2,
            HeapType::HostVisible => 1,
            HeapType::HostCached => 0,
        }
    }

    /// Check if CPU can access this heap
    pub const fn is_cpu_accessible(&self) -> bool {
        matches!(self, HeapType::HostVisible | HeapType::HostCached)
    }
}

// =============================================================================
// HEAP FLAGS
// =============================================================================

bitflags::bitflags! {
    /// Heap property flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct HeapFlags: u32 {
        /// Memory is device-local (VRAM)
        const DEVICE_LOCAL = 1 << 0;
        /// Memory is host-visible
        const HOST_VISIBLE = 1 << 1;
        /// Memory is host-coherent
        const HOST_COHERENT = 1 << 2;
        /// Memory is host-cached
        const HOST_CACHED = 1 << 3;
        /// Memory is protected
        const PROTECTED = 1 << 4;
        /// Memory supports ECC
        const ECC = 1 << 5;
    }
}

// =============================================================================
// VRAM HEAP
// =============================================================================

/// A GPU memory heap
#[derive(Debug)]
pub struct VramHeap {
    /// Heap type
    heap_type: HeapType,
    /// Base address
    base: GpuAddr,
    /// Total size
    size: ByteSize,
    /// Buddy allocator
    allocator: BuddyAllocator,
    /// Heap flags
    flags: HeapFlags,
}

impl VramHeap {
    /// Create a new VRAM heap
    pub fn new(
        heap_type: HeapType,
        base: GpuAddr,
        size: ByteSize,
        flags: HeapFlags,
    ) -> Result<Self> {
        let allocator = BuddyAllocator::new(base, size)?;

        Ok(Self {
            heap_type,
            base,
            size,
            allocator,
            flags,
        })
    }

    /// Get heap type
    pub fn heap_type(&self) -> HeapType {
        self.heap_type
    }

    /// Get base address
    pub fn base(&self) -> GpuAddr {
        self.base
    }

    /// Get total size
    pub fn size(&self) -> ByteSize {
        self.size
    }

    /// Get available space
    pub fn available(&self) -> ByteSize {
        self.allocator.free_space()
    }

    /// Get heap flags
    pub fn flags(&self) -> HeapFlags {
        self.flags
    }

    /// Allocate from this heap
    pub fn allocate(&mut self, size: ByteSize) -> Result<HeapAllocation> {
        let block = self.allocator.allocate(size)?;

        Ok(HeapAllocation {
            addr: block.addr,
            size: block.size,
            heap_type: self.heap_type,
        })
    }

    /// Free an allocation
    pub fn free(&mut self, alloc: HeapAllocation) -> Result<()> {
        let block = crate::buddy::BuddyBlock {
            addr: alloc.addr,
            size: alloc.size,
            order: Self::size_to_order(alloc.size.as_bytes())?,
            free: false,
        };
        self.allocator.free(block)
    }

    /// Size to order helper
    fn size_to_order(size: u64) -> Result<u8> {
        if size < crate::buddy::MIN_BLOCK_SIZE {
            return Ok(0);
        }

        let rounded = size.next_power_of_two();
        let order =
            (rounded.trailing_zeros() - crate::buddy::MIN_BLOCK_SIZE.trailing_zeros()) as u8;

        if (order as usize) < crate::buddy::MAX_ORDER {
            Ok(order)
        } else {
            Err(Error::InvalidParameter)
        }
    }

    /// Get fragmentation ratio
    pub fn fragmentation(&self) -> f32 {
        self.allocator.fragmentation()
    }

    /// Get allocator stats
    pub fn stats(&self) -> &crate::buddy::AllocatorStats {
        self.allocator.stats()
    }
}

/// An allocation from a heap
#[derive(Debug, Clone)]
pub struct HeapAllocation {
    /// GPU address
    pub addr: GpuAddr,
    /// Size
    pub size: ByteSize,
    /// Source heap type
    pub heap_type: HeapType,
}

// =============================================================================
// HEAP MANAGER
// =============================================================================

/// Manages multiple GPU heaps
#[derive(Debug)]
pub struct HeapManager {
    /// Available heaps
    heaps: alloc::vec::Vec<VramHeap>,
}

impl HeapManager {
    /// Create new heap manager
    pub fn new() -> Self {
        Self {
            heaps: alloc::vec::Vec::new(),
        }
    }

    /// Add a heap
    pub fn add_heap(&mut self, heap: VramHeap) {
        self.heaps.push(heap);
    }

    /// Get heap by type
    pub fn get_heap(&self, heap_type: HeapType) -> Option<&VramHeap> {
        self.heaps.iter().find(|h| h.heap_type == heap_type)
    }

    /// Get mutable heap by type
    pub fn get_heap_mut(&mut self, heap_type: HeapType) -> Option<&mut VramHeap> {
        self.heaps.iter_mut().find(|h| h.heap_type == heap_type)
    }

    /// Allocate from preferred heap, falling back to others
    pub fn allocate(&mut self, size: ByteSize, preferred: HeapType) -> Result<HeapAllocation> {
        // Try preferred heap first
        if let Some(heap) = self.get_heap_mut(preferred) {
            if let Ok(alloc) = heap.allocate(size) {
                return Ok(alloc);
            }
        }

        // Fall back to other heaps by priority
        let mut heaps_by_priority: alloc::vec::Vec<_> = self
            .heaps
            .iter_mut()
            .filter(|h| h.heap_type != preferred)
            .collect();

        heaps_by_priority.sort_by(|a, b| b.heap_type.priority().cmp(&a.heap_type.priority()));

        for heap in heaps_by_priority {
            if let Ok(alloc) = heap.allocate(size) {
                return Ok(alloc);
            }
        }

        Err(Error::OutOfMemory)
    }

    /// Free allocation
    pub fn free(&mut self, alloc: HeapAllocation) -> Result<()> {
        if let Some(heap) = self.get_heap_mut(alloc.heap_type) {
            heap.free(alloc)
        } else {
            Err(Error::InvalidParameter)
        }
    }

    /// Get total available space across all heaps
    pub fn total_available(&self) -> ByteSize {
        ByteSize::from_bytes(self.heaps.iter().map(|h| h.available().as_bytes()).sum())
    }

    /// Get total size across all heaps
    pub fn total_size(&self) -> ByteSize {
        ByteSize::from_bytes(self.heaps.iter().map(|h| h.size().as_bytes()).sum())
    }
}

impl Default for HeapManager {
    fn default() -> Self {
        Self::new()
    }
}
