//! Memory Allocator
//!
//! Boot-time memory allocators for UEFI environment.

use super::{size_to_pages, PAGE_SIZE};
use crate::raw::types::*;

extern crate alloc;
use alloc::alloc::{GlobalAlloc, Layout};
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicUsize, Ordering};

// =============================================================================
// BOOT ALLOCATOR
// =============================================================================

/// Boot-time page allocator
pub struct BootAllocator {
    /// Allocation regions
    regions: Vec<AllocationRegion>,
    /// Total allocated pages
    allocated_pages: usize,
    /// Total freed pages
    freed_pages: usize,
    /// Allocation tracking
    allocations: Vec<Allocation>,
}

impl BootAllocator {
    /// Create new boot allocator
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
            allocated_pages: 0,
            freed_pages: 0,
            allocations: Vec::new(),
        }
    }

    /// Add memory region
    pub fn add_region(&mut self, base: PhysicalAddress, size: u64) {
        let region = AllocationRegion::new(base, size);
        self.regions.push(region);
    }

    /// Allocate pages
    pub fn allocate_pages(&mut self, count: u64) -> Option<PhysicalAddress> {
        self.allocate_pages_aligned(count, PAGE_SIZE)
    }

    /// Allocate aligned pages
    pub fn allocate_pages_aligned(
        &mut self,
        count: u64,
        alignment: u64,
    ) -> Option<PhysicalAddress> {
        for region in &mut self.regions {
            if let Some(addr) = region.allocate_aligned(count, alignment) {
                self.allocated_pages += count as usize;
                self.allocations.push(Allocation {
                    address: addr,
                    pages: count,
                    purpose: AllocationPurpose::General,
                });
                return Some(addr);
            }
        }
        None
    }

    /// Allocate pages for specific purpose
    pub fn allocate_pages_for(
        &mut self,
        count: u64,
        purpose: AllocationPurpose,
    ) -> Option<PhysicalAddress> {
        if let Some(addr) = self.allocate_pages(count) {
            // Update allocation record
            if let Some(alloc) = self.allocations.last_mut() {
                alloc.purpose = purpose;
            }
            Some(addr)
        } else {
            None
        }
    }

    /// Free pages
    pub fn free_pages(&mut self, addr: PhysicalAddress, count: u64) {
        for region in &mut self.regions {
            if region.contains(addr) {
                region.free(addr, count);
                self.freed_pages += count as usize;

                // Remove from tracking
                self.allocations.retain(|a| a.address != addr);
                return;
            }
        }
    }

    /// Get allocation statistics
    pub fn stats(&self) -> AllocatorStats {
        let total_pages: u64 = self.regions.iter().map(|r| r.total_pages).sum();
        let free_pages: u64 = self.regions.iter().map(|r| r.free_pages).sum();

        AllocatorStats {
            total_pages,
            free_pages,
            allocated_pages: self.allocated_pages as u64,
            freed_pages: self.freed_pages as u64,
            region_count: self.regions.len(),
            allocation_count: self.allocations.len(),
        }
    }

    /// Get allocations by purpose
    pub fn allocations_by_purpose(&self, purpose: AllocationPurpose) -> Vec<&Allocation> {
        self.allocations
            .iter()
            .filter(|a| a.purpose == purpose)
            .collect()
    }

    /// Get all allocations
    pub fn allocations(&self) -> &[Allocation] {
        &self.allocations
    }
}

impl Default for BootAllocator {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// ALLOCATION REGION
// =============================================================================

/// Memory region for allocation
#[derive(Debug)]
struct AllocationRegion {
    /// Base address
    base: PhysicalAddress,
    /// Total pages
    total_pages: u64,
    /// Free pages
    free_pages: u64,
    /// Next allocation offset
    next_offset: u64,
    /// Free list (simple bump allocator with free list)
    free_list: Vec<FreeBlock>,
}

impl AllocationRegion {
    /// Create new region
    fn new(base: PhysicalAddress, size: u64) -> Self {
        let pages = size / PAGE_SIZE;
        Self {
            base,
            total_pages: pages,
            free_pages: pages,
            next_offset: 0,
            free_list: Vec::new(),
        }
    }

    /// Allocate from region
    fn allocate(&mut self, pages: u64) -> Option<PhysicalAddress> {
        self.allocate_aligned(pages, PAGE_SIZE)
    }

    /// Allocate with alignment
    fn allocate_aligned(&mut self, pages: u64, alignment: u64) -> Option<PhysicalAddress> {
        // First try free list
        if let Some(idx) = self.find_free_block(pages, alignment) {
            let block = self.free_list.remove(idx);
            self.free_pages -= pages;

            // If block is larger, add remainder back
            if block.pages > pages {
                let remainder_addr = block.address + pages * PAGE_SIZE;
                let remainder_pages = block.pages - pages;
                self.free_list.push(FreeBlock {
                    address: remainder_addr,
                    pages: remainder_pages,
                });
            }

            return Some(block.address);
        }

        // Bump allocate
        let size_bytes = pages * PAGE_SIZE;
        let current_addr = self.base + self.next_offset;
        let aligned_addr = (current_addr.0 + alignment - 1) & !(alignment - 1);
        let waste = aligned_addr - current_addr.0;
        let total_needed = waste + size_bytes;

        let remaining = (self.total_pages * PAGE_SIZE) - self.next_offset;
        if total_needed > remaining {
            return None;
        }

        self.next_offset += total_needed;
        self.free_pages -= pages;

        Some(PhysicalAddress(aligned_addr))
    }

    /// Find suitable free block
    fn find_free_block(&self, pages: u64, alignment: u64) -> Option<usize> {
        for (idx, block) in self.free_list.iter().enumerate() {
            let aligned = (block.address.0 + alignment - 1) & !(alignment - 1);
            let waste = aligned - block.address.0;
            let waste_pages = (waste + PAGE_SIZE - 1) / PAGE_SIZE;

            if block.pages >= pages + waste_pages {
                return Some(idx);
            }
        }
        None
    }

    /// Free pages back to region
    fn free(&mut self, addr: PhysicalAddress, pages: u64) {
        self.free_list.push(FreeBlock {
            address: addr,
            pages,
        });
        self.free_pages += pages;

        // Coalesce adjacent blocks
        self.coalesce_free_list();
    }

    /// Coalesce adjacent free blocks
    fn coalesce_free_list(&mut self) {
        if self.free_list.len() < 2 {
            return;
        }

        // Sort by address
        self.free_list.sort_by_key(|b| b.address);

        let mut i = 0;
        while i + 1 < self.free_list.len() {
            let current_end = self.free_list[i].address + self.free_list[i].pages * PAGE_SIZE;
            if current_end == self.free_list[i + 1].address {
                // Merge
                self.free_list[i].pages += self.free_list[i + 1].pages;
                self.free_list.remove(i + 1);
            } else {
                i += 1;
            }
        }
    }

    /// Check if address is in region
    fn contains(&self, addr: PhysicalAddress) -> bool {
        addr >= self.base && addr < self.base + self.total_pages * PAGE_SIZE
    }
}

/// Free block
#[derive(Debug, Clone)]
struct FreeBlock {
    address: PhysicalAddress,
    pages: u64,
}

// =============================================================================
// POOL ALLOCATOR
// =============================================================================

/// Pool allocator for small allocations
pub struct PoolAllocator {
    /// Backing page allocator
    page_allocator: *mut BootAllocator,
    /// Size classes
    size_classes: [SizeClass; 8],
    /// Statistics
    alloc_count: usize,
    free_count: usize,
}

impl PoolAllocator {
    /// Size class thresholds
    const SIZE_CLASSES: [usize; 8] = [16, 32, 64, 128, 256, 512, 1024, 2048];

    /// Create new pool allocator
    pub fn new(page_allocator: *mut BootAllocator) -> Self {
        Self {
            page_allocator,
            size_classes: [
                SizeClass::new(16),
                SizeClass::new(32),
                SizeClass::new(64),
                SizeClass::new(128),
                SizeClass::new(256),
                SizeClass::new(512),
                SizeClass::new(1024),
                SizeClass::new(2048),
            ],
            alloc_count: 0,
            free_count: 0,
        }
    }

    /// Allocate memory
    pub unsafe fn allocate(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        let actual_size = size.max(align);

        // Find size class
        for (idx, &class_size) in Self::SIZE_CLASSES.iter().enumerate() {
            if actual_size <= class_size {
                if let Some(ptr) = self.size_classes[idx].allocate(self.page_allocator) {
                    self.alloc_count += 1;
                    return Some(ptr);
                }
            }
        }

        // Large allocation - use page allocator directly
        let pages = size_to_pages(actual_size as u64);
        if let Some(addr) = (*self.page_allocator).allocate_pages(pages) {
            self.alloc_count += 1;
            return Some(addr.0 as *mut u8);
        }

        None
    }

    /// Free memory
    pub unsafe fn free(&mut self, ptr: *mut u8, size: usize) {
        let actual_size = size;

        // Find size class
        for (idx, &class_size) in Self::SIZE_CLASSES.iter().enumerate() {
            if actual_size <= class_size {
                self.size_classes[idx].free(ptr);
                self.free_count += 1;
                return;
            }
        }

        // Large allocation
        let pages = size_to_pages(actual_size as u64);
        (*self.page_allocator).free_pages(PhysicalAddress(ptr as u64), pages);
        self.free_count += 1;
    }

    /// Get statistics
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            alloc_count: self.alloc_count,
            free_count: self.free_count,
            active_allocations: self.alloc_count - self.free_count,
        }
    }
}

/// Size class for pool allocator
struct SizeClass {
    /// Object size
    size: usize,
    /// Free list head
    free_list: *mut FreeNode,
    /// Allocated slab count
    slab_count: usize,
}

impl SizeClass {
    /// Create new size class
    fn new(size: usize) -> Self {
        Self {
            size,
            free_list: ptr::null_mut(),
            slab_count: 0,
        }
    }

    /// Allocate from size class
    unsafe fn allocate(&mut self, page_alloc: *mut BootAllocator) -> Option<*mut u8> {
        // Check free list
        if !self.free_list.is_null() {
            let node = self.free_list;
            self.free_list = (*node).next;
            return Some(node as *mut u8);
        }

        // Allocate new slab
        if let Some(slab_addr) = (*page_alloc).allocate_pages(1) {
            self.slab_count += 1;

            // Initialize free list from slab
            let objects_per_slab = PAGE_SIZE as usize / self.size;
            for i in 0..objects_per_slab {
                let obj_addr = (slab_addr.0 as usize + i * self.size) as *mut FreeNode;
                (*obj_addr).next = self.free_list;
                self.free_list = obj_addr;
            }

            // Return first object
            let node = self.free_list;
            self.free_list = (*node).next;
            return Some(node as *mut u8);
        }

        None
    }

    /// Free to size class
    unsafe fn free(&mut self, ptr: *mut u8) {
        let node = ptr as *mut FreeNode;
        (*node).next = self.free_list;
        self.free_list = node;
    }
}

/// Free list node
#[repr(C)]
struct FreeNode {
    next: *mut FreeNode,
}

// =============================================================================
// BUMP ALLOCATOR
// =============================================================================

/// Simple bump allocator for boot-time use
pub struct BumpAllocator {
    /// Current position
    current: AtomicUsize,
    /// End of region
    end: usize,
    /// Start of region
    start: usize,
}

impl BumpAllocator {
    /// Create new bump allocator
    pub const fn new() -> Self {
        Self {
            current: AtomicUsize::new(0),
            end: 0,
            start: 0,
        }
    }

    /// Initialize with memory region
    pub fn init(&mut self, start: usize, size: usize) {
        self.start = start;
        self.end = start + size;
        self.current.store(start, Ordering::SeqCst);
    }

    /// Allocate memory
    pub fn allocate(&self, size: usize, align: usize) -> Option<NonNull<u8>> {
        loop {
            let current = self.current.load(Ordering::Relaxed);
            let aligned = (current + align - 1) & !(align - 1);
            let new_current = aligned + size;

            if new_current > self.end {
                return None;
            }

            if self
                .current
                .compare_exchange_weak(current, new_current, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                return NonNull::new(aligned as *mut u8);
            }
        }
    }

    /// Reset allocator
    pub fn reset(&self) {
        self.current.store(self.start, Ordering::SeqCst);
    }

    /// Get used bytes
    pub fn used(&self) -> usize {
        self.current.load(Ordering::Relaxed) - self.start
    }

    /// Get free bytes
    pub fn free(&self) -> usize {
        self.end - self.current.load(Ordering::Relaxed)
    }
}

// =============================================================================
// GLOBAL ALLOCATOR
// =============================================================================

/// UEFI global allocator wrapper
pub struct UefiAllocator {
    /// Inner allocator
    inner: UnsafeCell<Option<*mut PoolAllocator>>,
}

impl UefiAllocator {
    /// Create new uninitialized allocator
    pub const fn new() -> Self {
        Self {
            inner: UnsafeCell::new(None),
        }
    }

    /// Initialize with pool allocator
    pub unsafe fn init(&self, allocator: *mut PoolAllocator) {
        *self.inner.get() = Some(allocator);
    }
}

unsafe impl GlobalAlloc for UefiAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if let Some(allocator) = *self.inner.get() {
            (*allocator)
                .allocate(layout.size(), layout.align())
                .unwrap_or(ptr::null_mut())
        } else {
            ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if let Some(allocator) = *self.inner.get() {
            (*allocator).free(ptr, layout.size());
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = self.alloc(Layout::from_size_align_unchecked(new_size, layout.align()));
        if !new_ptr.is_null() {
            ptr::copy_nonoverlapping(ptr, new_ptr, layout.size().min(new_size));
            self.dealloc(ptr, layout);
        }
        new_ptr
    }
}

unsafe impl Sync for UefiAllocator {}

// =============================================================================
// ALLOCATION TYPES
// =============================================================================

/// Tracked allocation
#[derive(Debug, Clone)]
pub struct Allocation {
    /// Address
    pub address: PhysicalAddress,
    /// Page count
    pub pages: u64,
    /// Purpose
    pub purpose: AllocationPurpose,
}

impl Allocation {
    /// Get size in bytes
    pub fn size(&self) -> u64 {
        self.pages * PAGE_SIZE
    }

    /// Get end address
    pub fn end(&self) -> PhysicalAddress {
        self.address + self.size()
    }
}

/// Allocation purpose
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationPurpose {
    /// General allocation
    General,
    /// Page tables
    PageTables,
    /// Kernel image
    KernelImage,
    /// Kernel stack
    KernelStack,
    /// Initrd/ramdisk
    Initrd,
    /// Framebuffer
    Framebuffer,
    /// ACPI tables
    AcpiTables,
    /// Boot info structure
    BootInfo,
    /// Memory map
    MemoryMap,
    /// Module data
    Module,
}

/// Allocator statistics
#[derive(Debug, Clone, Default)]
pub struct AllocatorStats {
    /// Total pages in allocator
    pub total_pages: u64,
    /// Free pages
    pub free_pages: u64,
    /// Allocated pages (cumulative)
    pub allocated_pages: u64,
    /// Freed pages (cumulative)
    pub freed_pages: u64,
    /// Region count
    pub region_count: usize,
    /// Active allocation count
    pub allocation_count: usize,
}

impl AllocatorStats {
    /// Get currently used pages
    pub fn used_pages(&self) -> u64 {
        self.total_pages - self.free_pages
    }

    /// Get usage percentage
    pub fn usage_percent(&self) -> f32 {
        if self.total_pages == 0 {
            0.0
        } else {
            (self.used_pages() as f32 / self.total_pages as f32) * 100.0
        }
    }
}

/// Pool allocator statistics
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    /// Total allocation calls
    pub alloc_count: usize,
    /// Total free calls
    pub free_count: usize,
    /// Active allocations
    pub active_allocations: usize,
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocation() {
        let mut allocator = BootAllocator::new();
        allocator.add_region(0x100000, 16 * PAGE_SIZE);

        let addr1 = allocator.allocate_pages(1);
        assert!(addr1.is_some());

        let addr2 = allocator.allocate_pages(2);
        assert!(addr2.is_some());

        let stats = allocator.stats();
        assert_eq!(stats.allocated_pages, 3);
    }

    #[test]
    fn test_bump_allocator() {
        let mut bump = BumpAllocator::new();
        bump.init(0x1000, 0x1000);

        let ptr1 = bump.allocate(64, 8);
        assert!(ptr1.is_some());

        let ptr2 = bump.allocate(128, 16);
        assert!(ptr2.is_some());

        assert!(bump.used() > 0);
    }
}
