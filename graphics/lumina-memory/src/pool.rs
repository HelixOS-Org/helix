//! Memory Pool
//!
//! Pooled memory allocators for efficient allocation of same-sized objects.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use bitflags::bitflags;
use lumina_core::Handle;

use crate::{AllocationType, MemoryLocation};

// ============================================================================
// Pool Flags
// ============================================================================

bitflags! {
    /// Memory pool flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PoolFlags: u32 {
        /// Linear allocation (no fragmentation).
        const LINEAR = 1 << 0;
        /// Allow sub-allocations.
        const ALLOW_SUBALLOCATION = 1 << 1;
        /// Persistent mapping.
        const PERSISTENT_MAP = 1 << 2;
        /// Write combined.
        const WRITE_COMBINED = 1 << 3;
        /// Ring buffer mode.
        const RING_BUFFER = 1 << 4;
    }
}

impl Default for PoolFlags {
    fn default() -> Self {
        PoolFlags::empty()
    }
}

// ============================================================================
// Pool Handle
// ============================================================================

/// Handle to a memory pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PoolHandle(Handle<MemoryPool>);

impl PoolHandle {
    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self(Handle::from_raw_parts(index, generation))
    }

    /// Get the index.
    pub fn index(&self) -> u32 {
        self.0.index()
    }

    /// Get the generation.
    pub fn generation(&self) -> u32 {
        self.0.generation()
    }

    /// Invalid handle.
    pub const INVALID: Self = Self(Handle::INVALID);
}

// ============================================================================
// Pool Allocation Handle
// ============================================================================

/// Handle to a pool allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PoolAllocationHandle(Handle<PoolAllocation>);

impl PoolAllocationHandle {
    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self(Handle::from_raw_parts(index, generation))
    }

    /// Get the index.
    pub fn index(&self) -> u32 {
        self.0.index()
    }

    /// Get the generation.
    pub fn generation(&self) -> u32 {
        self.0.generation()
    }

    /// Invalid handle.
    pub const INVALID: Self = Self(Handle::INVALID);
}

// ============================================================================
// Pool Allocation
// ============================================================================

/// An allocation from a pool.
#[derive(Debug, Clone)]
pub struct PoolAllocation {
    /// Handle.
    pub handle: PoolAllocationHandle,
    /// Pool handle.
    pub pool: PoolHandle,
    /// Offset within pool.
    pub offset: u64,
    /// Size.
    pub size: u64,
    /// Mapped pointer.
    pub mapped_ptr: Option<*mut u8>,
}

impl PoolAllocation {
    /// Create a new pool allocation.
    pub fn new(handle: PoolAllocationHandle, pool: PoolHandle, offset: u64, size: u64) -> Self {
        Self {
            handle,
            pool,
            offset,
            size,
            mapped_ptr: None,
        }
    }

    /// Get end offset.
    pub fn end(&self) -> u64 {
        self.offset + self.size
    }

    /// Check if mapped.
    pub fn is_mapped(&self) -> bool {
        self.mapped_ptr.is_some()
    }

    /// Get mapped slice.
    pub fn mapped_slice(&self) -> Option<&[u8]> {
        self.mapped_ptr
            .map(|ptr| unsafe { core::slice::from_raw_parts(ptr, self.size as usize) })
    }

    /// Get mapped slice (mutable).
    pub fn mapped_slice_mut(&mut self) -> Option<&mut [u8]> {
        self.mapped_ptr
            .map(|ptr| unsafe { core::slice::from_raw_parts_mut(ptr, self.size as usize) })
    }
}

// ============================================================================
// Pool Description
// ============================================================================

/// Description for creating a memory pool.
#[derive(Debug, Clone)]
pub struct PoolDesc {
    /// Total pool size.
    pub size: u64,
    /// Block size (for fixed-size pools).
    pub block_size: u64,
    /// Memory type index.
    pub memory_type_index: u32,
    /// Memory location.
    pub location: MemoryLocation,
    /// Allocation type.
    pub allocation_type: AllocationType,
    /// Pool flags.
    pub flags: PoolFlags,
    /// Alignment.
    pub alignment: u64,
    /// Debug name.
    pub name: Option<String>,
}

impl Default for PoolDesc {
    fn default() -> Self {
        Self {
            size: 64 * 1024 * 1024, // 64MB
            block_size: 0,
            memory_type_index: 0,
            location: MemoryLocation::GpuOnly,
            allocation_type: AllocationType::Unknown,
            flags: PoolFlags::empty(),
            alignment: 256,
            name: None,
        }
    }
}

impl PoolDesc {
    /// Create a new pool description.
    pub fn new(size: u64) -> Self {
        Self {
            size,
            ..Default::default()
        }
    }

    /// Set block size.
    pub fn with_block_size(mut self, size: u64) -> Self {
        self.block_size = size;
        self
    }

    /// Set memory type.
    pub fn with_memory_type(mut self, index: u32) -> Self {
        self.memory_type_index = index;
        self
    }

    /// Set memory location.
    pub fn with_location(mut self, location: MemoryLocation) -> Self {
        self.location = location;
        self
    }

    /// Set allocation type.
    pub fn with_allocation_type(mut self, allocation_type: AllocationType) -> Self {
        self.allocation_type = allocation_type;
        self
    }

    /// Set flags.
    pub fn with_flags(mut self, flags: PoolFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Set alignment.
    pub fn with_alignment(mut self, alignment: u64) -> Self {
        self.alignment = alignment;
        self
    }

    /// Set debug name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Create for buffers.
    pub fn buffer_pool(size: u64, location: MemoryLocation) -> Self {
        Self {
            size,
            location,
            allocation_type: AllocationType::Buffer,
            ..Default::default()
        }
    }

    /// Create for images.
    pub fn image_pool(size: u64, location: MemoryLocation) -> Self {
        Self {
            size,
            location,
            allocation_type: AllocationType::Image,
            ..Default::default()
        }
    }

    /// Create linear pool.
    pub fn linear_pool(size: u64, location: MemoryLocation) -> Self {
        Self {
            size,
            location,
            flags: PoolFlags::LINEAR,
            ..Default::default()
        }
    }

    /// Create ring buffer pool.
    pub fn ring_buffer_pool(size: u64, location: MemoryLocation) -> Self {
        Self {
            size,
            location,
            flags: PoolFlags::RING_BUFFER,
            ..Default::default()
        }
    }
}

// ============================================================================
// Memory Pool
// ============================================================================

/// A memory pool.
pub struct MemoryPool {
    /// Handle.
    pub handle: PoolHandle,
    /// Pool size.
    pub size: u64,
    /// Block size (for fixed-size pools).
    pub block_size: u64,
    /// Used memory.
    used: AtomicU64,
    /// Memory type index.
    pub memory_type_index: u32,
    /// Memory location.
    pub location: MemoryLocation,
    /// Allocation type.
    pub allocation_type: AllocationType,
    /// Pool flags.
    pub flags: PoolFlags,
    /// Alignment.
    pub alignment: u64,
    /// Mapped pointer (if mapped).
    pub mapped_ptr: Option<*mut u8>,
    /// Allocations.
    allocations: Vec<Option<PoolAllocation>>,
    /// Free indices.
    free_indices: Vec<u32>,
    /// Generations.
    generations: Vec<u32>,
    /// Current offset (for linear/ring allocation).
    current_offset: AtomicU64,
    /// Debug name.
    pub name: Option<String>,
}

impl MemoryPool {
    /// Create a new memory pool.
    pub fn new(handle: PoolHandle, desc: &PoolDesc) -> Self {
        Self {
            handle,
            size: desc.size,
            block_size: desc.block_size,
            used: AtomicU64::new(0),
            memory_type_index: desc.memory_type_index,
            location: desc.location,
            allocation_type: desc.allocation_type,
            flags: desc.flags,
            alignment: desc.alignment,
            mapped_ptr: None,
            allocations: Vec::new(),
            free_indices: Vec::new(),
            generations: Vec::new(),
            current_offset: AtomicU64::new(0),
            name: desc.name.clone(),
        }
    }

    /// Get used memory.
    pub fn used(&self) -> u64 {
        self.used.load(Ordering::Relaxed)
    }

    /// Get available memory.
    pub fn available(&self) -> u64 {
        self.size.saturating_sub(self.used())
    }

    /// Get utilization ratio.
    pub fn utilization(&self) -> f32 {
        if self.size == 0 {
            0.0
        } else {
            self.used() as f32 / self.size as f32
        }
    }

    /// Check if linear pool.
    pub fn is_linear(&self) -> bool {
        self.flags.contains(PoolFlags::LINEAR)
    }

    /// Check if ring buffer.
    pub fn is_ring_buffer(&self) -> bool {
        self.flags.contains(PoolFlags::RING_BUFFER)
    }

    /// Align offset.
    fn align_offset(&self, offset: u64) -> u64 {
        let alignment = self.alignment.max(1);
        (offset + alignment - 1) & !(alignment - 1)
    }

    /// Allocate from pool.
    pub fn allocate(&mut self, size: u64) -> Option<PoolAllocationHandle> {
        let aligned_size = self.align_offset(size);

        // Get allocation offset
        let offset = if self.is_linear() || self.is_ring_buffer() {
            let current = self.current_offset.load(Ordering::Relaxed);
            let aligned_offset = self.align_offset(current);

            if self.is_ring_buffer() {
                // Wrap around for ring buffer
                if aligned_offset + aligned_size > self.size {
                    0 // Wrap to beginning
                } else {
                    aligned_offset
                }
            } else {
                if aligned_offset + aligned_size > self.size {
                    return None; // Out of space
                }
                aligned_offset
            }
        } else {
            // Simple bump allocation
            let current = self.current_offset.load(Ordering::Relaxed);
            let aligned_offset = self.align_offset(current);
            if aligned_offset + aligned_size > self.size {
                return None;
            }
            aligned_offset
        };

        // Update offset
        self.current_offset
            .store(offset + aligned_size, Ordering::Relaxed);

        // Create allocation
        let index = if let Some(index) = self.free_indices.pop() {
            index
        } else {
            let index = self.allocations.len() as u32;
            self.allocations.push(None);
            self.generations.push(0);
            index
        };

        let generation = self.generations[index as usize];
        let handle = PoolAllocationHandle::new(index, generation);

        let mut allocation = PoolAllocation::new(handle, self.handle, offset, aligned_size);
        if let Some(pool_ptr) = self.mapped_ptr {
            allocation.mapped_ptr = Some(unsafe { pool_ptr.add(offset as usize) });
        }

        self.allocations[index as usize] = Some(allocation);
        self.used.fetch_add(aligned_size, Ordering::Relaxed);

        Some(handle)
    }

    /// Free allocation.
    pub fn free(&mut self, handle: PoolAllocationHandle) {
        let index = handle.index() as usize;
        if index >= self.allocations.len() {
            return;
        }
        if self.generations[index] != handle.generation() {
            return;
        }

        if let Some(allocation) = self.allocations[index].take() {
            self.used.fetch_sub(allocation.size, Ordering::Relaxed);
        }

        self.generations[index] = self.generations[index].wrapping_add(1);
        self.free_indices.push(index as u32);
    }

    /// Get allocation.
    pub fn get(&self, handle: PoolAllocationHandle) -> Option<&PoolAllocation> {
        let index = handle.index() as usize;
        if index >= self.allocations.len() {
            return None;
        }
        if self.generations[index] != handle.generation() {
            return None;
        }
        self.allocations[index].as_ref()
    }

    /// Reset pool (for linear pools).
    pub fn reset(&mut self) {
        if self.is_linear() {
            self.current_offset.store(0, Ordering::Relaxed);
            self.used.store(0, Ordering::Relaxed);
            self.allocations.clear();
            self.free_indices.clear();
            self.generations.clear();
        }
    }

    /// Get allocation count.
    pub fn allocation_count(&self) -> usize {
        self.allocations.iter().filter(|a| a.is_some()).count()
    }
}

// ============================================================================
// Pool Statistics
// ============================================================================

/// Pool statistics.
#[derive(Debug, Clone, Default)]
pub struct PoolStatistics {
    /// Pool size.
    pub size: u64,
    /// Used memory.
    pub used: u64,
    /// Allocation count.
    pub allocation_count: u32,
    /// Peak used.
    pub peak_used: u64,
    /// Total allocations.
    pub total_allocations: u64,
    /// Total frees.
    pub total_frees: u64,
}

impl PoolStatistics {
    /// Get utilization ratio.
    pub fn utilization(&self) -> f32 {
        if self.size == 0 {
            0.0
        } else {
            self.used as f32 / self.size as f32
        }
    }

    /// Get available memory.
    pub fn available(&self) -> u64 {
        self.size.saturating_sub(self.used)
    }
}

// ============================================================================
// Pool Manager
// ============================================================================

/// Memory pool manager.
pub struct PoolManager {
    /// Pools.
    pools: Vec<Option<MemoryPool>>,
    /// Free indices.
    free_indices: Vec<u32>,
    /// Generations.
    generations: Vec<u32>,
}

impl PoolManager {
    /// Create a new pool manager.
    pub fn new() -> Self {
        Self {
            pools: Vec::new(),
            free_indices: Vec::new(),
            generations: Vec::new(),
        }
    }

    /// Create a pool.
    pub fn create_pool(&mut self, desc: &PoolDesc) -> PoolHandle {
        let index = if let Some(index) = self.free_indices.pop() {
            index
        } else {
            let index = self.pools.len() as u32;
            self.pools.push(None);
            self.generations.push(0);
            index
        };

        let generation = self.generations[index as usize];
        let handle = PoolHandle::new(index, generation);

        let pool = MemoryPool::new(handle, desc);
        self.pools[index as usize] = Some(pool);

        handle
    }

    /// Destroy a pool.
    pub fn destroy_pool(&mut self, handle: PoolHandle) -> bool {
        let index = handle.index() as usize;
        if index >= self.pools.len() {
            return false;
        }
        if self.generations[index] != handle.generation() {
            return false;
        }

        self.pools[index] = None;
        self.generations[index] = self.generations[index].wrapping_add(1);
        self.free_indices.push(index as u32);

        true
    }

    /// Get pool.
    pub fn get(&self, handle: PoolHandle) -> Option<&MemoryPool> {
        let index = handle.index() as usize;
        if index >= self.pools.len() {
            return None;
        }
        if self.generations[index] != handle.generation() {
            return None;
        }
        self.pools[index].as_ref()
    }

    /// Get pool (mutable).
    pub fn get_mut(&mut self, handle: PoolHandle) -> Option<&mut MemoryPool> {
        let index = handle.index() as usize;
        if index >= self.pools.len() {
            return None;
        }
        if self.generations[index] != handle.generation() {
            return None;
        }
        self.pools[index].as_mut()
    }

    /// Allocate from pool.
    pub fn allocate(&mut self, pool: PoolHandle, size: u64) -> Option<PoolAllocationHandle> {
        self.get_mut(pool)?.allocate(size)
    }

    /// Free pool allocation.
    pub fn free(&mut self, pool: PoolHandle, allocation: PoolAllocationHandle) {
        if let Some(pool) = self.get_mut(pool) {
            pool.free(allocation);
        }
    }

    /// Get pool count.
    pub fn pool_count(&self) -> usize {
        self.pools.iter().filter(|p| p.is_some()).count()
    }

    /// Get total memory.
    pub fn total_memory(&self) -> u64 {
        self.pools
            .iter()
            .filter_map(|p| p.as_ref())
            .map(|p| p.size)
            .sum()
    }

    /// Get used memory.
    pub fn used_memory(&self) -> u64 {
        self.pools
            .iter()
            .filter_map(|p| p.as_ref())
            .map(|p| p.used())
            .sum()
    }
}

impl Default for PoolManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Fixed Size Pool
// ============================================================================

/// A fixed-size object pool.
pub struct FixedSizePool {
    /// Block size.
    pub block_size: u64,
    /// Total blocks.
    pub total_blocks: u32,
    /// Free list.
    free_list: Vec<u32>,
    /// Allocation bitmap.
    allocated: Vec<bool>,
    /// Mapped pointer.
    pub mapped_ptr: Option<*mut u8>,
}

impl FixedSizePool {
    /// Create a new fixed-size pool.
    pub fn new(block_size: u64, total_blocks: u32) -> Self {
        Self {
            block_size,
            total_blocks,
            free_list: (0..total_blocks).rev().collect(),
            allocated: vec![false; total_blocks as usize],
            mapped_ptr: None,
        }
    }

    /// Allocate a block.
    pub fn allocate(&mut self) -> Option<u32> {
        let index = self.free_list.pop()?;
        self.allocated[index as usize] = true;
        Some(index)
    }

    /// Free a block.
    pub fn free(&mut self, index: u32) {
        if index < self.total_blocks && self.allocated[index as usize] {
            self.allocated[index as usize] = false;
            self.free_list.push(index);
        }
    }

    /// Get block offset.
    pub fn offset(&self, index: u32) -> u64 {
        index as u64 * self.block_size
    }

    /// Get block pointer.
    pub fn ptr(&self, index: u32) -> Option<*mut u8> {
        self.mapped_ptr
            .map(|ptr| unsafe { ptr.add(self.offset(index) as usize) })
    }

    /// Get allocated count.
    pub fn allocated_count(&self) -> u32 {
        self.total_blocks - self.free_list.len() as u32
    }

    /// Get free count.
    pub fn free_count(&self) -> u32 {
        self.free_list.len() as u32
    }

    /// Check if full.
    pub fn is_full(&self) -> bool {
        self.free_list.is_empty()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.free_list.len() == self.total_blocks as usize
    }
}
