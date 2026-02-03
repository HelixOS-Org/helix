//! Sub-Allocators
//!
//! Various sub-allocation strategies for GPU memory.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::cmp::Ordering as CmpOrdering;

// ============================================================================
// Sub-Allocator Trait
// ============================================================================

/// A sub-allocation within a memory region.
#[derive(Debug, Clone, Copy)]
pub struct SubAlloc {
    /// Offset.
    pub offset: u64,
    /// Size.
    pub size: u64,
}

impl SubAlloc {
    /// Create a new sub-allocation.
    pub fn new(offset: u64, size: u64) -> Self {
        Self { offset, size }
    }

    /// Get end offset.
    pub fn end(&self) -> u64 {
        self.offset + self.size
    }

    /// Check if adjacent.
    pub fn is_adjacent(&self, other: &SubAlloc) -> bool {
        self.end() == other.offset || other.end() == self.offset
    }
}

// ============================================================================
// Linear Allocator
// ============================================================================

/// A linear (bump) allocator.
///
/// Fast allocation, but can only be reset as a whole.
pub struct LinearAllocator {
    /// Total size.
    size: u64,
    /// Current offset.
    offset: u64,
    /// Peak usage.
    peak: u64,
    /// Allocation count.
    allocation_count: u32,
}

impl LinearAllocator {
    /// Create a new linear allocator.
    pub fn new(size: u64) -> Self {
        Self {
            size,
            offset: 0,
            peak: 0,
            allocation_count: 0,
        }
    }

    /// Allocate memory.
    pub fn allocate(&mut self, size: u64, alignment: u64) -> Option<SubAlloc> {
        let aligned_offset = self.align(self.offset, alignment);

        if aligned_offset + size > self.size {
            return None;
        }

        let result = SubAlloc::new(aligned_offset, size);
        self.offset = aligned_offset + size;
        self.peak = self.peak.max(self.offset);
        self.allocation_count += 1;

        Some(result)
    }

    /// Reset the allocator.
    pub fn reset(&mut self) {
        self.offset = 0;
        self.allocation_count = 0;
    }

    /// Get used memory.
    pub fn used(&self) -> u64 {
        self.offset
    }

    /// Get available memory.
    pub fn available(&self) -> u64 {
        self.size.saturating_sub(self.offset)
    }

    /// Get peak usage.
    pub fn peak(&self) -> u64 {
        self.peak
    }

    /// Get allocation count.
    pub fn allocation_count(&self) -> u32 {
        self.allocation_count
    }

    /// Align offset.
    fn align(&self, offset: u64, alignment: u64) -> u64 {
        let alignment = alignment.max(1);
        (offset + alignment - 1) & !(alignment - 1)
    }
}

// ============================================================================
// Buddy Allocator
// ============================================================================

/// Order index for buddy allocator.
type Order = u32;

/// A buddy allocator.
///
/// Splits memory into power-of-two sized blocks.
pub struct BuddyAllocator {
    /// Total size (must be power of 2).
    size: u64,
    /// Minimum block size.
    min_block_size: u64,
    /// Maximum order.
    max_order: Order,
    /// Free lists per order.
    free_lists: Vec<Vec<u64>>,
    /// Allocation map (offset -> order).
    allocations: BTreeMap<u64, Order>,
    /// Used memory.
    used: u64,
}

impl BuddyAllocator {
    /// Create a new buddy allocator.
    pub fn new(size: u64, min_block_size: u64) -> Self {
        let size = size.next_power_of_two();
        let min_block_size = min_block_size.next_power_of_two();
        let max_order = (size / min_block_size).trailing_zeros();

        let mut free_lists = vec![Vec::new(); max_order as usize + 1];
        free_lists[max_order as usize].push(0);

        Self {
            size,
            min_block_size,
            max_order,
            free_lists,
            allocations: BTreeMap::new(),
            used: 0,
        }
    }

    /// Allocate memory.
    pub fn allocate(&mut self, size: u64) -> Option<SubAlloc> {
        let size = size.max(self.min_block_size).next_power_of_two();
        let order = self.size_to_order(size);

        if order > self.max_order {
            return None;
        }

        // Find a free block
        let block_order = self.find_free_block(order)?;
        let offset = self.free_lists[block_order as usize].pop()?;

        // Split blocks if necessary
        self.split_block(offset, block_order, order);

        // Record allocation
        self.allocations.insert(offset, order);
        self.used += self.order_to_size(order);

        Some(SubAlloc::new(offset, self.order_to_size(order)))
    }

    /// Free memory.
    pub fn free(&mut self, offset: u64) {
        let order = match self.allocations.remove(&offset) {
            Some(order) => order,
            None => return,
        };

        self.used -= self.order_to_size(order);

        // Try to merge with buddy
        self.merge_with_buddy(offset, order);
    }

    /// Find a free block of at least the given order.
    fn find_free_block(&self, order: Order) -> Option<Order> {
        for o in order..=self.max_order {
            if !self.free_lists[o as usize].is_empty() {
                return Some(o);
            }
        }
        None
    }

    /// Split a block down to the target order.
    fn split_block(&mut self, offset: u64, from_order: Order, to_order: Order) {
        let mut current_order = from_order;
        let mut current_offset = offset;

        while current_order > to_order {
            current_order -= 1;
            let buddy_offset = current_offset + self.order_to_size(current_order);
            self.free_lists[current_order as usize].push(buddy_offset);
        }
    }

    /// Try to merge a block with its buddy.
    fn merge_with_buddy(&mut self, offset: u64, order: Order) {
        let mut current_offset = offset;
        let mut current_order = order;

        while current_order < self.max_order {
            let buddy_offset = self.buddy_offset(current_offset, current_order);

            // Check if buddy is free
            let buddy_pos = self.free_lists[current_order as usize]
                .iter()
                .position(|&o| o == buddy_offset);

            match buddy_pos {
                Some(pos) => {
                    // Remove buddy from free list
                    self.free_lists[current_order as usize].swap_remove(pos);

                    // Merge: take the lower offset
                    current_offset = current_offset.min(buddy_offset);
                    current_order += 1;
                },
                None => {
                    // Buddy is not free, stop merging
                    break;
                },
            }
        }

        // Add merged block to free list
        self.free_lists[current_order as usize].push(current_offset);
    }

    /// Get buddy offset.
    fn buddy_offset(&self, offset: u64, order: Order) -> u64 {
        offset ^ self.order_to_size(order)
    }

    /// Convert size to order.
    fn size_to_order(&self, size: u64) -> Order {
        let blocks = (size + self.min_block_size - 1) / self.min_block_size;
        blocks.next_power_of_two().trailing_zeros()
    }

    /// Convert order to size.
    fn order_to_size(&self, order: Order) -> u64 {
        self.min_block_size << order
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
        self.allocations.len()
    }

    /// Get fragmentation estimate.
    pub fn fragmentation(&self) -> f32 {
        if self.used == 0 {
            return 0.0;
        }

        // Count free blocks at each level
        let total_free_blocks: usize = self.free_lists.iter().map(|l| l.len()).sum();
        let ideal_blocks = 1; // Ideally one big free block

        if total_free_blocks <= ideal_blocks {
            0.0
        } else {
            1.0 - (ideal_blocks as f32 / total_free_blocks as f32)
        }
    }
}

// ============================================================================
// TLSF Allocator (Two-Level Segregated Fit)
// ============================================================================

/// First-level index size.
const FL_INDEX_COUNT: usize = 32;
/// Second-level index size (log2).
const SL_INDEX_COUNT_LOG2: usize = 5;
/// Second-level index size.
const SL_INDEX_COUNT: usize = 1 << SL_INDEX_COUNT_LOG2;

/// A free block in TLSF.
#[derive(Debug, Clone)]
struct TlsfBlock {
    offset: u64,
    size: u64,
    is_free: bool,
    prev_physical: Option<u64>,
    next_physical: Option<u64>,
}

/// TLSF allocator.
///
/// O(1) allocation and deallocation with low fragmentation.
pub struct TlsfAllocator {
    /// Total size.
    size: u64,
    /// All blocks.
    blocks: BTreeMap<u64, TlsfBlock>,
    /// First-level bitmap.
    fl_bitmap: u32,
    /// Second-level bitmaps.
    sl_bitmaps: [u32; FL_INDEX_COUNT],
    /// Free lists [fl][sl].
    free_lists: [[Vec<u64>; SL_INDEX_COUNT]; FL_INDEX_COUNT],
    /// Used memory.
    used: u64,
    /// Minimum allocation size.
    min_allocation_size: u64,
}

impl TlsfAllocator {
    /// Create a new TLSF allocator.
    pub fn new(size: u64) -> Self {
        let mut allocator = Self {
            size,
            blocks: BTreeMap::new(),
            fl_bitmap: 0,
            sl_bitmaps: [0; FL_INDEX_COUNT],
            free_lists: Default::default(),
            used: 0,
            min_allocation_size: 16,
        };

        // Create initial free block
        allocator.insert_free_block(0, size);

        allocator
    }

    /// Allocate memory.
    pub fn allocate(&mut self, size: u64, alignment: u64) -> Option<SubAlloc> {
        let size = size.max(self.min_allocation_size);
        let aligned_size = self.adjust_size(size, alignment);

        // Find suitable free block
        let (fl, sl) = self.find_suitable_block(aligned_size)?;
        let offset = self.free_lists[fl][sl].pop()?;

        // Update bitmaps
        if self.free_lists[fl][sl].is_empty() {
            self.sl_bitmaps[fl] &= !(1 << sl);
            if self.sl_bitmaps[fl] == 0 {
                self.fl_bitmap &= !(1 << fl);
            }
        }

        // Get block
        let block = self.blocks.get_mut(&offset)?;
        let block_size = block.size;
        block.is_free = false;

        // Split if necessary
        let remaining = block_size - aligned_size;
        if remaining >= self.min_allocation_size {
            // Update block size
            let block = self.blocks.get_mut(&offset).unwrap();
            block.size = aligned_size;

            // Create new free block
            let new_offset = offset + aligned_size;
            self.insert_free_block(new_offset, remaining);

            // Update physical links
            let old_next = self.blocks.get(&offset).unwrap().next_physical;
            self.blocks.get_mut(&offset).unwrap().next_physical = Some(new_offset);
            self.blocks.get_mut(&new_offset).unwrap().prev_physical = Some(offset);
            self.blocks.get_mut(&new_offset).unwrap().next_physical = old_next;
        }

        self.used += aligned_size;
        Some(SubAlloc::new(offset, aligned_size))
    }

    /// Free memory.
    pub fn free(&mut self, offset: u64) {
        let block = match self.blocks.get_mut(&offset) {
            Some(b) if !b.is_free => b,
            _ => return,
        };

        let size = block.size;
        block.is_free = true;
        self.used -= size;

        // Try to merge with neighbors
        self.merge_with_neighbors(offset);
    }

    /// Insert a free block.
    fn insert_free_block(&mut self, offset: u64, size: u64) {
        let block = TlsfBlock {
            offset,
            size,
            is_free: true,
            prev_physical: None,
            next_physical: None,
        };

        self.blocks.insert(offset, block);

        // Add to free list
        let (fl, sl) = self.mapping(size);
        self.free_lists[fl][sl].push(offset);

        // Update bitmaps
        self.sl_bitmaps[fl] |= 1 << sl;
        self.fl_bitmap |= 1 << fl;
    }

    /// Remove a free block from free lists.
    fn remove_free_block(&mut self, offset: u64, size: u64) {
        let (fl, sl) = self.mapping(size);

        if let Some(pos) = self.free_lists[fl][sl].iter().position(|&o| o == offset) {
            self.free_lists[fl][sl].swap_remove(pos);

            if self.free_lists[fl][sl].is_empty() {
                self.sl_bitmaps[fl] &= !(1 << sl);
                if self.sl_bitmaps[fl] == 0 {
                    self.fl_bitmap &= !(1 << fl);
                }
            }
        }
    }

    /// Merge with neighboring free blocks.
    fn merge_with_neighbors(&mut self, offset: u64) {
        let block = match self.blocks.get(&offset) {
            Some(b) => b.clone(),
            None => return,
        };

        let mut merged_offset = offset;
        let mut merged_size = block.size;

        // Merge with previous
        if let Some(prev_offset) = block.prev_physical {
            if let Some(prev_block) = self.blocks.get(&prev_offset) {
                if prev_block.is_free {
                    let prev_size = prev_block.size;
                    self.remove_free_block(prev_offset, prev_size);
                    self.blocks.remove(&prev_offset);
                    merged_offset = prev_offset;
                    merged_size += prev_size;
                }
            }
        }

        // Merge with next
        if let Some(next_offset) = block.next_physical {
            if let Some(next_block) = self.blocks.get(&next_offset) {
                if next_block.is_free {
                    let next_size = next_block.size;
                    self.remove_free_block(next_offset, next_size);
                    self.blocks.remove(&next_offset);
                    merged_size += next_size;
                }
            }
        }

        // Remove current and insert merged
        self.remove_free_block(offset, block.size);
        self.blocks.remove(&offset);
        self.insert_free_block(merged_offset, merged_size);
    }

    /// Find a suitable free block.
    fn find_suitable_block(&self, size: u64) -> Option<(usize, usize)> {
        let (fl, sl) = self.mapping(size);

        // Search in current second-level
        let sl_map = self.sl_bitmaps[fl] & (!0u32 << sl);
        if sl_map != 0 {
            let sl_idx = sl_map.trailing_zeros() as usize;
            return Some((fl, sl_idx));
        }

        // Search in higher first-levels
        let fl_map = self.fl_bitmap & (!0u32 << (fl + 1));
        if fl_map == 0 {
            return None;
        }

        let fl_idx = fl_map.trailing_zeros() as usize;
        let sl_idx = self.sl_bitmaps[fl_idx].trailing_zeros() as usize;

        Some((fl_idx, sl_idx))
    }

    /// Map size to (fl, sl) indices.
    fn mapping(&self, size: u64) -> (usize, usize) {
        if size < (1 << SL_INDEX_COUNT_LOG2) {
            return (0, size as usize);
        }

        let fl = (63 - size.leading_zeros()) as usize;
        let sl = ((size >> (fl - SL_INDEX_COUNT_LOG2)) ^ (1 << SL_INDEX_COUNT_LOG2)) as usize;

        (fl.min(FL_INDEX_COUNT - 1), sl.min(SL_INDEX_COUNT - 1))
    }

    /// Adjust size for alignment.
    fn adjust_size(&self, size: u64, alignment: u64) -> u64 {
        let alignment = alignment.max(1);
        (size + alignment - 1) & !(alignment - 1)
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
        self.blocks.values().filter(|b| !b.is_free).count()
    }
}

impl Default for TlsfAllocator {
    fn default() -> Self {
        Self::new(256 * 1024 * 1024) // 256MB
    }
}

// ============================================================================
// Free List Allocator
// ============================================================================

/// A simple free list allocator.
pub struct FreeListAllocator {
    /// Total size.
    size: u64,
    /// Free regions (offset, size).
    free_regions: Vec<(u64, u64)>,
    /// Used memory.
    used: u64,
    /// Allocation count.
    allocation_count: u32,
}

impl FreeListAllocator {
    /// Create a new free list allocator.
    pub fn new(size: u64) -> Self {
        Self {
            size,
            free_regions: vec![(0, size)],
            used: 0,
            allocation_count: 0,
        }
    }

    /// Allocate using first-fit strategy.
    pub fn allocate(&mut self, size: u64, alignment: u64) -> Option<SubAlloc> {
        for i in 0..self.free_regions.len() {
            let (offset, region_size) = self.free_regions[i];
            let aligned_offset = self.align(offset, alignment);
            let padding = aligned_offset - offset;

            if region_size >= padding + size {
                // Found a fit
                let alloc = SubAlloc::new(aligned_offset, size);

                // Update free region
                if padding > 0 {
                    // Keep padding as free
                    self.free_regions[i] = (offset, padding);

                    // Add remaining if any
                    let remaining = region_size - padding - size;
                    if remaining > 0 {
                        self.free_regions.push((aligned_offset + size, remaining));
                    }
                } else {
                    // No padding
                    let remaining = region_size - size;
                    if remaining > 0 {
                        self.free_regions[i] = (aligned_offset + size, remaining);
                    } else {
                        self.free_regions.swap_remove(i);
                    }
                }

                self.used += size;
                self.allocation_count += 1;
                return Some(alloc);
            }
        }

        None
    }

    /// Free memory.
    pub fn free(&mut self, offset: u64, size: u64) {
        self.used -= size;
        self.allocation_count -= 1;

        // Add to free list
        self.free_regions.push((offset, size));

        // Sort and merge
        self.free_regions.sort_by_key(|&(o, _)| o);
        self.merge_free_regions();
    }

    /// Merge adjacent free regions.
    fn merge_free_regions(&mut self) {
        let mut i = 0;
        while i < self.free_regions.len().saturating_sub(1) {
            let (offset1, size1) = self.free_regions[i];
            let (offset2, size2) = self.free_regions[i + 1];

            if offset1 + size1 == offset2 {
                self.free_regions[i] = (offset1, size1 + size2);
                self.free_regions.remove(i + 1);
            } else {
                i += 1;
            }
        }
    }

    /// Align offset.
    fn align(&self, offset: u64, alignment: u64) -> u64 {
        let alignment = alignment.max(1);
        (offset + alignment - 1) & !(alignment - 1)
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
    pub fn allocation_count(&self) -> u32 {
        self.allocation_count
    }

    /// Get fragmentation (number of free regions - 1).
    pub fn fragmentation(&self) -> u32 {
        self.free_regions.len().saturating_sub(1) as u32
    }
}

// ============================================================================
// Stack Allocator
// ============================================================================

/// A stack allocator (LIFO).
pub struct StackAllocator {
    /// Total size.
    size: u64,
    /// Current offset.
    offset: u64,
    /// Markers for rollback.
    markers: Vec<u64>,
}

impl StackAllocator {
    /// Create a new stack allocator.
    pub fn new(size: u64) -> Self {
        Self {
            size,
            offset: 0,
            markers: Vec::new(),
        }
    }

    /// Allocate memory.
    pub fn allocate(&mut self, size: u64, alignment: u64) -> Option<SubAlloc> {
        let aligned_offset = self.align(self.offset, alignment);

        if aligned_offset + size > self.size {
            return None;
        }

        let result = SubAlloc::new(aligned_offset, size);
        self.offset = aligned_offset + size;

        Some(result)
    }

    /// Push a marker for later rollback.
    pub fn push_marker(&mut self) -> usize {
        let marker = self.markers.len();
        self.markers.push(self.offset);
        marker
    }

    /// Pop to a marker.
    pub fn pop_marker(&mut self, marker: usize) {
        if marker < self.markers.len() {
            self.offset = self.markers[marker];
            self.markers.truncate(marker);
        }
    }

    /// Reset to beginning.
    pub fn reset(&mut self) {
        self.offset = 0;
        self.markers.clear();
    }

    /// Align offset.
    fn align(&self, offset: u64, alignment: u64) -> u64 {
        let alignment = alignment.max(1);
        (offset + alignment - 1) & !(alignment - 1)
    }

    /// Get used memory.
    pub fn used(&self) -> u64 {
        self.offset
    }

    /// Get available memory.
    pub fn available(&self) -> u64 {
        self.size - self.offset
    }
}
