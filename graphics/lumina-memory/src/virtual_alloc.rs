//! Virtual Memory Allocation
//!
//! Virtual address space management for sparse resources.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use bitflags::bitflags;
use lumina_core::Handle;

// ============================================================================
// Virtual Allocation Flags
// ============================================================================

bitflags! {
    /// Virtual allocation flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct VirtualAllocationFlags: u32 {
        /// Upper address range.
        const UPPER_ADDRESS = 1 << 0;
        /// Minimum offset.
        const MIN_OFFSET = 1 << 1;
        /// Strategy: min memory.
        const STRATEGY_MIN_MEMORY = 1 << 2;
        /// Strategy: min time.
        const STRATEGY_MIN_TIME = 1 << 3;
        /// Strategy: min offset.
        const STRATEGY_MIN_OFFSET = 1 << 4;
    }
}

impl Default for VirtualAllocationFlags {
    fn default() -> Self {
        VirtualAllocationFlags::empty()
    }
}

// ============================================================================
// Virtual Allocation Handle
// ============================================================================

/// Handle to a virtual allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VirtualAllocationHandle(Handle<VirtualAllocation>);

impl VirtualAllocationHandle {
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
// Virtual Allocation
// ============================================================================

/// A virtual memory allocation.
#[derive(Debug, Clone)]
pub struct VirtualAllocation {
    /// Handle.
    pub handle: VirtualAllocationHandle,
    /// Virtual address offset.
    pub offset: u64,
    /// Size.
    pub size: u64,
    /// User data.
    pub user_data: u64,
}

impl VirtualAllocation {
    /// Create a new virtual allocation.
    pub fn new(handle: VirtualAllocationHandle, offset: u64, size: u64) -> Self {
        Self {
            handle,
            offset,
            size,
            user_data: 0,
        }
    }

    /// Get end offset.
    pub fn end(&self) -> u64 {
        self.offset + self.size
    }
}

// ============================================================================
// Virtual Allocation Info
// ============================================================================

/// Information about a virtual allocation.
#[derive(Debug, Clone)]
pub struct VirtualAllocationInfo {
    /// Offset.
    pub offset: u64,
    /// Size.
    pub size: u64,
    /// User data.
    pub user_data: u64,
}

impl VirtualAllocationInfo {
    /// Create from allocation.
    pub fn from_allocation(alloc: &VirtualAllocation) -> Self {
        Self {
            offset: alloc.offset,
            size: alloc.size,
            user_data: alloc.user_data,
        }
    }
}

// ============================================================================
// Virtual Allocation Description
// ============================================================================

/// Description for creating a virtual allocation.
#[derive(Debug, Clone)]
pub struct VirtualAllocationDesc {
    /// Size.
    pub size: u64,
    /// Alignment.
    pub alignment: u64,
    /// Flags.
    pub flags: VirtualAllocationFlags,
    /// User data.
    pub user_data: u64,
}

impl Default for VirtualAllocationDesc {
    fn default() -> Self {
        Self {
            size: 0,
            alignment: 1,
            flags: VirtualAllocationFlags::empty(),
            user_data: 0,
        }
    }
}

impl VirtualAllocationDesc {
    /// Create a new description.
    pub fn new(size: u64) -> Self {
        Self {
            size,
            ..Default::default()
        }
    }

    /// Set alignment.
    pub fn with_alignment(mut self, alignment: u64) -> Self {
        self.alignment = alignment;
        self
    }

    /// Set flags.
    pub fn with_flags(mut self, flags: VirtualAllocationFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Set user data.
    pub fn with_user_data(mut self, user_data: u64) -> Self {
        self.user_data = user_data;
        self
    }
}

// ============================================================================
// Virtual Block
// ============================================================================

/// A virtual memory block (region in virtual address space).
#[derive(Debug, Clone)]
struct VirtualBlock {
    /// Offset.
    offset: u64,
    /// Size.
    size: u64,
    /// Is free.
    is_free: bool,
}

impl VirtualBlock {
    /// Create a new virtual block.
    fn new(offset: u64, size: u64, is_free: bool) -> Self {
        Self {
            offset,
            size,
            is_free,
        }
    }

    /// Get end offset.
    fn end(&self) -> u64 {
        self.offset + self.size
    }
}

// ============================================================================
// Virtual Allocator
// ============================================================================

/// A virtual memory allocator.
pub struct VirtualAllocator {
    /// Total size.
    size: u64,
    /// Blocks (sorted by offset).
    blocks: BTreeMap<u64, VirtualBlock>,
    /// Allocations.
    allocations: Vec<Option<VirtualAllocation>>,
    /// Free indices.
    free_indices: Vec<u32>,
    /// Generations.
    generations: Vec<u32>,
    /// Used memory.
    used: u64,
    /// Debug name.
    pub name: Option<String>,
}

impl VirtualAllocator {
    /// Create a new virtual allocator.
    pub fn new(size: u64) -> Self {
        let mut blocks = BTreeMap::new();
        blocks.insert(0, VirtualBlock::new(0, size, true));

        Self {
            size,
            blocks,
            allocations: Vec::new(),
            free_indices: Vec::new(),
            generations: Vec::new(),
            used: 0,
            name: None,
        }
    }

    /// Allocate virtual memory.
    pub fn allocate(&mut self, desc: &VirtualAllocationDesc) -> Option<VirtualAllocationHandle> {
        let alignment = desc.alignment.max(1);

        // Find a suitable free block
        let (offset, block_size) = self.find_free_block(desc.size, alignment)?;

        // Create allocation handle
        let index = if let Some(index) = self.free_indices.pop() {
            index
        } else {
            let index = self.allocations.len() as u32;
            self.allocations.push(None);
            self.generations.push(0);
            index
        };

        let generation = self.generations[index as usize];
        let handle = VirtualAllocationHandle::new(index, generation);

        let mut allocation = VirtualAllocation::new(handle, offset, desc.size);
        allocation.user_data = desc.user_data;

        self.allocations[index as usize] = Some(allocation);

        // Split the block
        self.split_block(offset, desc.size, block_size);

        self.used += desc.size;
        Some(handle)
    }

    /// Free virtual memory.
    pub fn free(&mut self, handle: VirtualAllocationHandle) {
        let index = handle.index() as usize;
        if index >= self.allocations.len() {
            return;
        }
        if self.generations[index] != handle.generation() {
            return;
        }

        let allocation = match self.allocations[index].take() {
            Some(a) => a,
            None => return,
        };

        self.generations[index] = self.generations[index].wrapping_add(1);
        self.free_indices.push(index as u32);

        self.used -= allocation.size;

        // Mark block as free and merge
        self.free_block(allocation.offset, allocation.size);
    }

    /// Find a free block.
    fn find_free_block(&self, size: u64, alignment: u64) -> Option<(u64, u64)> {
        for (&offset, block) in &self.blocks {
            if !block.is_free {
                continue;
            }

            let aligned_offset = self.align(offset, alignment);
            let padding = aligned_offset - offset;

            if block.size >= padding + size {
                return Some((aligned_offset, block.size));
            }
        }
        None
    }

    /// Split a block after allocation.
    fn split_block(&mut self, offset: u64, alloc_size: u64, block_size: u64) {
        // Get the original block offset
        let block_offset = self
            .blocks
            .range(..=offset)
            .next_back()
            .map(|(&o, _)| o)
            .unwrap_or(0);

        let block = self.blocks.remove(&block_offset).unwrap();

        // Padding before allocation
        let padding = offset - block_offset;
        if padding > 0 {
            self.blocks
                .insert(block_offset, VirtualBlock::new(block_offset, padding, true));
        }

        // The allocation itself
        self.blocks
            .insert(offset, VirtualBlock::new(offset, alloc_size, false));

        // Remaining free space
        let remaining = block.size - padding - alloc_size;
        if remaining > 0 {
            let remaining_offset = offset + alloc_size;
            self.blocks.insert(
                remaining_offset,
                VirtualBlock::new(remaining_offset, remaining, true),
            );
        }
    }

    /// Free a block and merge with neighbors.
    fn free_block(&mut self, offset: u64, size: u64) {
        // Mark as free
        if let Some(block) = self.blocks.get_mut(&offset) {
            block.is_free = true;
        } else {
            self.blocks
                .insert(offset, VirtualBlock::new(offset, size, true));
        }

        // Merge with neighbors
        self.merge_free_blocks();
    }

    /// Merge adjacent free blocks.
    fn merge_free_blocks(&mut self) {
        let offsets: Vec<u64> = self.blocks.keys().copied().collect();

        let mut i = 0;
        while i < offsets.len().saturating_sub(1) {
            let offset1 = offsets[i];
            let offset2 = offsets[i + 1];

            let block1 = self.blocks.get(&offset1).cloned();
            let block2 = self.blocks.get(&offset2).cloned();

            if let (Some(b1), Some(b2)) = (block1, block2) {
                if b1.is_free && b2.is_free && b1.end() == offset2 {
                    // Merge
                    self.blocks.remove(&offset2);
                    self.blocks.get_mut(&offset1).unwrap().size += b2.size;
                    // Don't increment i, check again with the same block
                    continue;
                }
            }

            i += 1;
        }
    }

    /// Align offset.
    fn align(&self, offset: u64, alignment: u64) -> u64 {
        let alignment = alignment.max(1);
        (offset + alignment - 1) & !(alignment - 1)
    }

    /// Get allocation.
    pub fn get(&self, handle: VirtualAllocationHandle) -> Option<&VirtualAllocation> {
        let index = handle.index() as usize;
        if index >= self.allocations.len() {
            return None;
        }
        if self.generations[index] != handle.generation() {
            return None;
        }
        self.allocations[index].as_ref()
    }

    /// Get allocation info.
    pub fn get_info(&self, handle: VirtualAllocationHandle) -> Option<VirtualAllocationInfo> {
        self.get(handle).map(VirtualAllocationInfo::from_allocation)
    }

    /// Get total size.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Get used memory.
    pub fn used(&self) -> u64 {
        self.used
    }

    /// Get available memory.
    pub fn available(&self) -> u64 {
        self.size - self.used
    }

    /// Get allocation count.
    pub fn allocation_count(&self) -> usize {
        self.allocations.iter().filter(|a| a.is_some()).count()
    }

    /// Get statistics.
    pub fn statistics(&self) -> VirtualAllocatorStatistics {
        let free_blocks = self.blocks.values().filter(|b| b.is_free).count() as u32;
        let largest_free = self
            .blocks
            .values()
            .filter(|b| b.is_free)
            .map(|b| b.size)
            .max()
            .unwrap_or(0);

        VirtualAllocatorStatistics {
            size: self.size,
            used: self.used,
            allocation_count: self.allocation_count() as u32,
            free_block_count: free_blocks,
            largest_free_block: largest_free,
        }
    }

    /// Clear all allocations.
    pub fn clear(&mut self) {
        self.blocks.clear();
        self.blocks.insert(0, VirtualBlock::new(0, self.size, true));
        self.allocations.clear();
        self.free_indices.clear();
        self.generations.clear();
        self.used = 0;
    }
}

impl Default for VirtualAllocator {
    fn default() -> Self {
        Self::new(1024 * 1024 * 1024) // 1GB
    }
}

// ============================================================================
// Virtual Allocator Statistics
// ============================================================================

/// Virtual allocator statistics.
#[derive(Debug, Clone, Default)]
pub struct VirtualAllocatorStatistics {
    /// Total size.
    pub size: u64,
    /// Used memory.
    pub used: u64,
    /// Allocation count.
    pub allocation_count: u32,
    /// Free block count.
    pub free_block_count: u32,
    /// Largest free block.
    pub largest_free_block: u64,
}

impl VirtualAllocatorStatistics {
    /// Get utilization ratio.
    pub fn utilization(&self) -> f32 {
        if self.size == 0 {
            0.0
        } else {
            self.used as f32 / self.size as f32
        }
    }

    /// Get fragmentation estimate.
    pub fn fragmentation(&self) -> f32 {
        if self.free_block_count <= 1 {
            0.0
        } else {
            1.0 - (1.0 / self.free_block_count as f32)
        }
    }
}

// ============================================================================
// Sparse Resource
// ============================================================================

/// A sparse resource page.
#[derive(Debug, Clone, Copy)]
pub struct SparsePage {
    /// Page index.
    pub index: u32,
    /// Is resident (backed by memory).
    pub is_resident: bool,
    /// Memory offset (if resident).
    pub memory_offset: u64,
}

impl SparsePage {
    /// Create a non-resident page.
    pub fn non_resident(index: u32) -> Self {
        Self {
            index,
            is_resident: false,
            memory_offset: 0,
        }
    }

    /// Create a resident page.
    pub fn resident(index: u32, memory_offset: u64) -> Self {
        Self {
            index,
            is_resident: true,
            memory_offset,
        }
    }
}

/// Sparse resource management.
pub struct SparseResource {
    /// Page size.
    pub page_size: u64,
    /// Total page count.
    pub page_count: u32,
    /// Pages.
    pages: Vec<SparsePage>,
    /// Resident page count.
    resident_count: u32,
}

impl SparseResource {
    /// Create a new sparse resource.
    pub fn new(total_size: u64, page_size: u64) -> Self {
        let page_count = ((total_size + page_size - 1) / page_size) as u32;
        let pages = (0..page_count)
            .map(|i| SparsePage::non_resident(i))
            .collect();

        Self {
            page_size,
            page_count,
            pages,
            resident_count: 0,
        }
    }

    /// Make a page resident.
    pub fn make_resident(&mut self, page_index: u32, memory_offset: u64) -> bool {
        if page_index >= self.page_count {
            return false;
        }

        let page = &mut self.pages[page_index as usize];
        if !page.is_resident {
            page.is_resident = true;
            page.memory_offset = memory_offset;
            self.resident_count += 1;
        }
        true
    }

    /// Make a page non-resident.
    pub fn make_non_resident(&mut self, page_index: u32) -> bool {
        if page_index >= self.page_count {
            return false;
        }

        let page = &mut self.pages[page_index as usize];
        if page.is_resident {
            page.is_resident = false;
            page.memory_offset = 0;
            self.resident_count -= 1;
        }
        true
    }

    /// Get page.
    pub fn get_page(&self, page_index: u32) -> Option<&SparsePage> {
        self.pages.get(page_index as usize)
    }

    /// Check if page is resident.
    pub fn is_resident(&self, page_index: u32) -> bool {
        self.pages
            .get(page_index as usize)
            .map(|p| p.is_resident)
            .unwrap_or(false)
    }

    /// Get resident page count.
    pub fn resident_count(&self) -> u32 {
        self.resident_count
    }

    /// Get resident memory size.
    pub fn resident_memory(&self) -> u64 {
        self.resident_count as u64 * self.page_size
    }

    /// Get total virtual size.
    pub fn total_size(&self) -> u64 {
        self.page_count as u64 * self.page_size
    }

    /// Get residency ratio.
    pub fn residency_ratio(&self) -> f32 {
        if self.page_count == 0 {
            0.0
        } else {
            self.resident_count as f32 / self.page_count as f32
        }
    }

    /// Get non-resident pages.
    pub fn non_resident_pages(&self) -> impl Iterator<Item = u32> + '_ {
        self.pages
            .iter()
            .filter(|p| !p.is_resident)
            .map(|p| p.index)
    }

    /// Get resident pages.
    pub fn resident_pages(&self) -> impl Iterator<Item = &SparsePage> {
        self.pages.iter().filter(|p| p.is_resident)
    }
}

// ============================================================================
// Page Table
// ============================================================================

/// A page table for virtual memory mapping.
pub struct PageTable {
    /// Page size.
    pub page_size: u64,
    /// Page entries (virtual page -> physical page).
    entries: BTreeMap<u64, u64>,
}

impl PageTable {
    /// Create a new page table.
    pub fn new(page_size: u64) -> Self {
        Self {
            page_size,
            entries: BTreeMap::new(),
        }
    }

    /// Map a virtual page to a physical page.
    pub fn map(&mut self, virtual_page: u64, physical_page: u64) {
        self.entries.insert(virtual_page, physical_page);
    }

    /// Unmap a virtual page.
    pub fn unmap(&mut self, virtual_page: u64) {
        self.entries.remove(&virtual_page);
    }

    /// Translate virtual page to physical.
    pub fn translate(&self, virtual_page: u64) -> Option<u64> {
        self.entries.get(&virtual_page).copied()
    }

    /// Translate virtual address to physical.
    pub fn translate_address(&self, virtual_address: u64) -> Option<u64> {
        let virtual_page = virtual_address / self.page_size;
        let offset = virtual_address % self.page_size;

        self.translate(virtual_page)
            .map(|physical_page| physical_page * self.page_size + offset)
    }

    /// Check if a page is mapped.
    pub fn is_mapped(&self, virtual_page: u64) -> bool {
        self.entries.contains_key(&virtual_page)
    }

    /// Get mapped page count.
    pub fn mapped_count(&self) -> usize {
        self.entries.len()
    }

    /// Clear all mappings.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
