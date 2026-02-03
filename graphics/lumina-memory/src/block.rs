//! Memory Block Management
//!
//! Memory blocks represent contiguous GPU memory regions.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use bitflags::bitflags;
use lumina_core::Handle;

use crate::{AllocationType, MemoryLocation};

// ============================================================================
// Block Flags
// ============================================================================

bitflags! {
    /// Memory block flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct BlockFlags: u32 {
        /// Block is mapped.
        const MAPPED = 1 << 0;
        /// Block is dedicated.
        const DEDICATED = 1 << 1;
        /// Block is linear.
        const LINEAR = 1 << 2;
        /// Block is exportable.
        const EXPORTABLE = 1 << 3;
        /// Block is imported.
        const IMPORTED = 1 << 4;
    }
}

impl Default for BlockFlags {
    fn default() -> Self {
        BlockFlags::empty()
    }
}

// ============================================================================
// Block Handle
// ============================================================================

/// Handle to a memory block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockHandle(Handle<MemoryBlock>);

impl BlockHandle {
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
// Memory Block
// ============================================================================

/// A contiguous block of GPU memory.
pub struct MemoryBlock {
    /// Handle.
    pub handle: BlockHandle,
    /// Block size.
    pub size: u64,
    /// Memory type index.
    pub memory_type_index: u32,
    /// Memory location.
    pub location: MemoryLocation,
    /// Block flags.
    pub flags: BlockFlags,
    /// Mapped pointer (if mapped).
    pub mapped_ptr: Option<*mut u8>,
    /// Total allocations in block.
    pub allocation_count: u32,
    /// Used memory in block.
    pub used_memory: AtomicU64,
    /// Debug name.
    pub name: Option<String>,
    /// Frame created.
    pub created_frame: u64,
}

impl MemoryBlock {
    /// Create a new memory block.
    pub fn new(
        handle: BlockHandle,
        size: u64,
        memory_type_index: u32,
        location: MemoryLocation,
        flags: BlockFlags,
        created_frame: u64,
    ) -> Self {
        Self {
            handle,
            size,
            memory_type_index,
            location,
            flags,
            mapped_ptr: None,
            allocation_count: 0,
            used_memory: AtomicU64::new(0),
            name: None,
            created_frame,
        }
    }

    /// Get available memory.
    pub fn available(&self) -> u64 {
        self.size.saturating_sub(self.used())
    }

    /// Get used memory.
    pub fn used(&self) -> u64 {
        self.used_memory.load(Ordering::Relaxed)
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.allocation_count == 0
    }

    /// Check if full.
    pub fn is_full(&self) -> bool {
        self.available() == 0
    }

    /// Check if mapped.
    pub fn is_mapped(&self) -> bool {
        self.mapped_ptr.is_some()
    }

    /// Get utilization ratio.
    pub fn utilization(&self) -> f32 {
        if self.size == 0 {
            0.0
        } else {
            self.used() as f32 / self.size as f32
        }
    }

    /// Allocate from block.
    pub fn allocate(&mut self, size: u64) -> Option<u64> {
        let used = self.used();
        if used + size > self.size {
            return None;
        }

        self.used_memory.fetch_add(size, Ordering::Relaxed);
        self.allocation_count += 1;
        Some(used)
    }

    /// Free from block.
    pub fn free(&mut self, size: u64) {
        self.used_memory.fetch_sub(size, Ordering::Relaxed);
        self.allocation_count = self.allocation_count.saturating_sub(1);
    }

    /// Get mapped slice.
    pub fn mapped_slice(&self, offset: u64, size: u64) -> Option<&[u8]> {
        self.mapped_ptr.map(|ptr| unsafe {
            core::slice::from_raw_parts(ptr.add(offset as usize), size as usize)
        })
    }

    /// Get mapped slice (mutable).
    pub fn mapped_slice_mut(&mut self, offset: u64, size: u64) -> Option<&mut [u8]> {
        self.mapped_ptr.map(|ptr| unsafe {
            core::slice::from_raw_parts_mut(ptr.add(offset as usize), size as usize)
        })
    }
}

// ============================================================================
// Block Info
// ============================================================================

/// Memory block information.
#[derive(Debug, Clone)]
pub struct BlockInfo {
    /// Block size.
    pub size: u64,
    /// Used memory.
    pub used: u64,
    /// Allocation count.
    pub allocation_count: u32,
    /// Memory type index.
    pub memory_type_index: u32,
    /// Memory location.
    pub location: MemoryLocation,
    /// Is mapped.
    pub is_mapped: bool,
    /// Is dedicated.
    pub is_dedicated: bool,
}

impl BlockInfo {
    /// Create from block.
    pub fn from_block(block: &MemoryBlock) -> Self {
        Self {
            size: block.size,
            used: block.used(),
            allocation_count: block.allocation_count,
            memory_type_index: block.memory_type_index,
            location: block.location,
            is_mapped: block.is_mapped(),
            is_dedicated: block.flags.contains(BlockFlags::DEDICATED),
        }
    }

    /// Get available memory.
    pub fn available(&self) -> u64 {
        self.size.saturating_sub(self.used)
    }

    /// Get utilization ratio.
    pub fn utilization(&self) -> f32 {
        if self.size == 0 {
            0.0
        } else {
            self.used as f32 / self.size as f32
        }
    }
}

// ============================================================================
// Block Description
// ============================================================================

/// Description for creating a memory block.
#[derive(Debug, Clone)]
pub struct BlockDesc {
    /// Block size.
    pub size: u64,
    /// Memory type index.
    pub memory_type_index: u32,
    /// Memory location.
    pub location: MemoryLocation,
    /// Block flags.
    pub flags: BlockFlags,
    /// Debug name.
    pub name: Option<String>,
}

impl Default for BlockDesc {
    fn default() -> Self {
        Self {
            size: 256 * 1024 * 1024, // 256MB
            memory_type_index: 0,
            location: MemoryLocation::GpuOnly,
            flags: BlockFlags::empty(),
            name: None,
        }
    }
}

impl BlockDesc {
    /// Create a new block description.
    pub fn new(size: u64) -> Self {
        Self {
            size,
            ..Default::default()
        }
    }

    /// Set memory type index.
    pub fn with_memory_type(mut self, index: u32) -> Self {
        self.memory_type_index = index;
        self
    }

    /// Set memory location.
    pub fn with_location(mut self, location: MemoryLocation) -> Self {
        self.location = location;
        self
    }

    /// Set flags.
    pub fn with_flags(mut self, flags: BlockFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Set as mapped.
    pub fn mapped(mut self) -> Self {
        self.flags |= BlockFlags::MAPPED;
        self
    }

    /// Set debug name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

// ============================================================================
// Sub-Allocation
// ============================================================================

/// A sub-allocation within a block.
#[derive(Debug, Clone)]
pub struct SubAllocation {
    /// Block handle.
    pub block: BlockHandle,
    /// Offset within block.
    pub offset: u64,
    /// Size.
    pub size: u64,
    /// Allocation type.
    pub allocation_type: AllocationType,
    /// Is free.
    pub is_free: bool,
}

impl SubAllocation {
    /// Create a new sub-allocation.
    pub fn new(
        block: BlockHandle,
        offset: u64,
        size: u64,
        allocation_type: AllocationType,
    ) -> Self {
        Self {
            block,
            offset,
            size,
            allocation_type,
            is_free: false,
        }
    }

    /// Create a free sub-allocation.
    pub fn free_region(block: BlockHandle, offset: u64, size: u64) -> Self {
        Self {
            block,
            offset,
            size,
            allocation_type: AllocationType::Unknown,
            is_free: true,
        }
    }

    /// Get end offset.
    pub fn end(&self) -> u64 {
        self.offset + self.size
    }

    /// Check if adjacent to another sub-allocation.
    pub fn is_adjacent(&self, other: &SubAllocation) -> bool {
        self.block.index() == other.block.index()
            && (self.end() == other.offset || other.end() == self.offset)
    }
}

// ============================================================================
// Block Manager
// ============================================================================

/// Memory block manager.
pub struct BlockManager {
    /// Blocks.
    blocks: Vec<Option<MemoryBlock>>,
    /// Free indices.
    free_indices: Vec<u32>,
    /// Generations.
    generations: Vec<u32>,
    /// Default block size.
    pub default_block_size: u64,
    /// Current frame.
    current_frame: u64,
}

impl BlockManager {
    /// Create a new block manager.
    pub fn new(default_block_size: u64) -> Self {
        Self {
            blocks: Vec::new(),
            free_indices: Vec::new(),
            generations: Vec::new(),
            default_block_size,
            current_frame: 0,
        }
    }

    /// Create a memory block.
    pub fn create_block(&mut self, desc: &BlockDesc) -> BlockHandle {
        let index = if let Some(index) = self.free_indices.pop() {
            index
        } else {
            let index = self.blocks.len() as u32;
            self.blocks.push(None);
            self.generations.push(0);
            index
        };

        let generation = self.generations[index as usize];
        let handle = BlockHandle::new(index, generation);

        let mut block = MemoryBlock::new(
            handle,
            desc.size,
            desc.memory_type_index,
            desc.location,
            desc.flags,
            self.current_frame,
        );
        block.name = desc.name.clone();

        self.blocks[index as usize] = Some(block);

        handle
    }

    /// Destroy a memory block.
    pub fn destroy_block(&mut self, handle: BlockHandle) -> bool {
        let index = handle.index() as usize;
        if index >= self.blocks.len() {
            return false;
        }
        if self.generations[index] != handle.generation() {
            return false;
        }

        self.blocks[index] = None;
        self.generations[index] = self.generations[index].wrapping_add(1);
        self.free_indices.push(index as u32);

        true
    }

    /// Get a block.
    pub fn get(&self, handle: BlockHandle) -> Option<&MemoryBlock> {
        let index = handle.index() as usize;
        if index >= self.blocks.len() {
            return None;
        }
        if self.generations[index] != handle.generation() {
            return None;
        }
        self.blocks[index].as_ref()
    }

    /// Get a block (mutable).
    pub fn get_mut(&mut self, handle: BlockHandle) -> Option<&mut MemoryBlock> {
        let index = handle.index() as usize;
        if index >= self.blocks.len() {
            return None;
        }
        if self.generations[index] != handle.generation() {
            return None;
        }
        self.blocks[index].as_mut()
    }

    /// Get block info.
    pub fn get_info(&self, handle: BlockHandle) -> Option<BlockInfo> {
        self.get(handle).map(BlockInfo::from_block)
    }

    /// Find block with available space.
    pub fn find_available(&self, size: u64, location: MemoryLocation) -> Option<BlockHandle> {
        self.blocks
            .iter()
            .filter_map(|b| b.as_ref())
            .find(|b| b.location == location && b.available() >= size)
            .map(|b| b.handle)
    }

    /// Get all blocks.
    pub fn all_blocks(&self) -> impl Iterator<Item = &MemoryBlock> {
        self.blocks.iter().filter_map(|b| b.as_ref())
    }

    /// Get empty blocks.
    pub fn empty_blocks(&self) -> impl Iterator<Item = &MemoryBlock> {
        self.all_blocks().filter(|b| b.is_empty())
    }

    /// Get block count.
    pub fn block_count(&self) -> usize {
        self.blocks.iter().filter(|b| b.is_some()).count()
    }

    /// Get total memory.
    pub fn total_memory(&self) -> u64 {
        self.all_blocks().map(|b| b.size).sum()
    }

    /// Get used memory.
    pub fn used_memory(&self) -> u64 {
        self.all_blocks().map(|b| b.used()).sum()
    }

    /// Advance frame.
    pub fn advance_frame(&mut self) {
        self.current_frame += 1;
    }
}

impl Default for BlockManager {
    fn default() -> Self {
        Self::new(256 * 1024 * 1024) // 256MB default
    }
}

// ============================================================================
// Block Statistics
// ============================================================================

/// Block manager statistics.
#[derive(Debug, Clone, Default)]
pub struct BlockStatistics {
    /// Total blocks.
    pub total_blocks: u32,
    /// Empty blocks.
    pub empty_blocks: u32,
    /// Full blocks.
    pub full_blocks: u32,
    /// Total memory.
    pub total_memory: u64,
    /// Used memory.
    pub used_memory: u64,
    /// Average utilization.
    pub average_utilization: f32,
}

impl BlockStatistics {
    /// Calculate from block manager.
    pub fn from_manager(manager: &BlockManager) -> Self {
        let blocks: Vec<_> = manager.all_blocks().collect();
        let total_blocks = blocks.len() as u32;
        let empty_blocks = blocks.iter().filter(|b| b.is_empty()).count() as u32;
        let full_blocks = blocks.iter().filter(|b| b.is_full()).count() as u32;
        let total_memory: u64 = blocks.iter().map(|b| b.size).sum();
        let used_memory: u64 = blocks.iter().map(|b| b.used()).sum();
        let average_utilization = if total_memory == 0 {
            0.0
        } else {
            used_memory as f32 / total_memory as f32
        };

        Self {
            total_blocks,
            empty_blocks,
            full_blocks,
            total_memory,
            used_memory,
            average_utilization,
        }
    }
}
