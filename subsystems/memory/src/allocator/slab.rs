//! # Slab Allocator
//!
//! A slab allocator for efficient fixed-size allocations.

use super::{HeapAllocator, HeapStats};
use core::alloc::Layout;
use alloc::vec::Vec;
use alloc::boxed::Box;
use spin::Mutex;
use core::sync::atomic::{AtomicU64, Ordering};

/// Slab size classes
const SIZE_CLASSES: [usize; 8] = [16, 32, 64, 128, 256, 512, 1024, 2048];

/// A single slab
struct Slab {
    /// Memory block
    memory: *mut u8,
    /// Object size
    obj_size: usize,
    /// Number of objects
    capacity: usize,
    /// Free list head (index)
    free_head: Option<usize>,
    /// Number of allocated objects
    allocated: usize,
}

// SAFETY: Slab memory is exclusively owned and managed through the Mutex in SlabAllocator.
// Access to the raw pointer is always synchronized.
unsafe impl Send for Slab {}

impl Slab {
    /// Create a new slab
    unsafe fn new(memory: *mut u8, size: usize, obj_size: usize) -> Self {
        let capacity = size / obj_size;
        
        // Initialize free list
        // SAFETY: Caller guarantees memory is valid for size bytes
        unsafe {
            for i in 0..capacity - 1 {
                let ptr = memory.add(i * obj_size) as *mut usize;
                *ptr = i + 1;
            }
            // Last entry points to nothing
            let last_ptr = memory.add((capacity - 1) * obj_size) as *mut usize;
            *last_ptr = usize::MAX;
        }
        
        Self {
            memory,
            obj_size,
            capacity,
            free_head: Some(0),
            allocated: 0,
        }
    }

    /// Allocate an object
    fn allocate(&mut self) -> Option<*mut u8> {
        let index = self.free_head?;
        
        let ptr = unsafe { self.memory.add(index * self.obj_size) };
        
        // Update free list
        let next = unsafe { *(ptr as *const usize) };
        self.free_head = if next == usize::MAX { None } else { Some(next) };
        self.allocated += 1;
        
        Some(ptr)
    }

    /// Deallocate an object
    fn deallocate(&mut self, ptr: *mut u8) {
        let offset = (ptr as usize) - (self.memory as usize);
        let index = offset / self.obj_size;
        
        // Add to free list
        unsafe {
            *(ptr as *mut usize) = self.free_head.unwrap_or(usize::MAX);
        }
        self.free_head = Some(index);
        self.allocated -= 1;
    }

    /// Check if this slab contains the pointer
    fn contains(&self, ptr: *mut u8) -> bool {
        let start = self.memory as usize;
        let end = start + self.capacity * self.obj_size;
        let p = ptr as usize;
        p >= start && p < end
    }

    /// Is slab full?
    fn is_full(&self) -> bool {
        self.free_head.is_none()
    }

    /// Is slab empty?
    fn is_empty(&self) -> bool {
        self.allocated == 0
    }
}

/// Slab cache for a specific size class
struct SlabCache {
    /// Object size
    obj_size: usize,
    /// Slabs
    slabs: Vec<Slab>,
    /// Slab size
    slab_size: usize,
}

impl SlabCache {
    /// Create a new cache
    fn new(obj_size: usize) -> Self {
        Self {
            obj_size,
            slabs: Vec::new(),
            slab_size: 4096, // One page
        }
    }

    /// Allocate an object
    fn allocate(&mut self) -> Option<*mut u8> {
        // Try to find a slab with free space
        for slab in &mut self.slabs {
            if !slab.is_full() {
                return slab.allocate();
            }
        }
        
        // Need to create a new slab
        // This would normally allocate from the physical memory manager
        // For now, we can't create new slabs without external allocation
        None
    }

    /// Deallocate an object
    fn deallocate(&mut self, ptr: *mut u8) {
        for slab in &mut self.slabs {
            if slab.contains(ptr) {
                slab.deallocate(ptr);
                return;
            }
        }
    }
}

/// Slab allocator
pub struct SlabAllocator {
    /// Caches for each size class
    caches: Mutex<[Option<SlabCache>; 8]>,
    /// Statistics
    allocations: AtomicU64,
    deallocations: AtomicU64,
}

impl SlabAllocator {
    /// Create a new slab allocator
    pub const fn new() -> Self {
        Self {
            caches: Mutex::new([
                None, None, None, None,
                None, None, None, None,
            ]),
            allocations: AtomicU64::new(0),
            deallocations: AtomicU64::new(0),
        }
    }

    /// Initialize with backing memory
    pub fn init(&self, memory: *mut u8, size: usize) {
        let mut caches = self.caches.lock();
        
        // Divide memory among size classes
        let per_class = size / SIZE_CLASSES.len();
        let mut offset = 0;
        
        for (i, &obj_size) in SIZE_CLASSES.iter().enumerate() {
            let mut cache = SlabCache::new(obj_size);
            
            // Create slabs for this cache
            let slab_count = per_class / cache.slab_size;
            for j in 0..slab_count {
                let slab_mem = unsafe { memory.add(offset + j * cache.slab_size) };
                let slab = unsafe { Slab::new(slab_mem, cache.slab_size, obj_size) };
                cache.slabs.push(slab);
            }
            
            caches[i] = Some(cache);
            offset += per_class;
        }
    }

    /// Find appropriate size class
    fn size_class(size: usize) -> Option<usize> {
        SIZE_CLASSES.iter().position(|&s| s >= size)
    }
}

impl Default for SlabAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl HeapAllocator for SlabAllocator {
    fn allocate(&self, layout: Layout) -> *mut u8 {
        let size = layout.size().max(layout.align());
        
        if let Some(class) = Self::size_class(size) {
            let mut caches = self.caches.lock();
            if let Some(ref mut cache) = caches[class] {
                if let Some(ptr) = cache.allocate() {
                    self.allocations.fetch_add(1, Ordering::Relaxed);
                    return ptr;
                }
            }
        }
        
        core::ptr::null_mut()
    }

    fn deallocate(&self, ptr: *mut u8, layout: Layout) {
        let size = layout.size().max(layout.align());
        
        if let Some(class) = Self::size_class(size) {
            let mut caches = self.caches.lock();
            if let Some(ref mut cache) = caches[class] {
                cache.deallocate(ptr);
                self.deallocations.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn name(&self) -> &'static str {
        "Slab Allocator"
    }

    fn stats(&self) -> HeapStats {
        HeapStats {
            total_size: 0,
            used_size: 0,
            free_size: 0,
            allocations: self.allocations.load(Ordering::Relaxed),
            deallocations: self.deallocations.load(Ordering::Relaxed),
        }
    }
}
