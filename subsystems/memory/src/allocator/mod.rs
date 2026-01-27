//! # Heap Allocator Framework
//!
//! Framework for kernel heap allocators.

pub mod slab;

use core::alloc::{GlobalAlloc, Layout};
use alloc::sync::Arc;
use spin::RwLock;

/// Heap allocator trait
pub trait HeapAllocator: Send + Sync {
    /// Allocate memory
    fn allocate(&self, layout: Layout) -> *mut u8;
    
    /// Deallocate memory
    fn deallocate(&self, ptr: *mut u8, layout: Layout);
    
    /// Reallocate memory
    fn reallocate(&self, ptr: *mut u8, old_layout: Layout, new_size: usize) -> *mut u8 {
        // Default implementation: allocate new, copy, deallocate old
        let new_layout = Layout::from_size_align(new_size, old_layout.align())
            .unwrap_or(old_layout);
        
        let new_ptr = self.allocate(new_layout);
        if !new_ptr.is_null() && !ptr.is_null() {
            unsafe {
                core::ptr::copy_nonoverlapping(
                    ptr,
                    new_ptr,
                    old_layout.size().min(new_size),
                );
            }
            self.deallocate(ptr, old_layout);
        }
        new_ptr
    }
    
    /// Get allocator name
    fn name(&self) -> &'static str;
    
    /// Get statistics
    fn stats(&self) -> HeapStats;
}

/// Heap statistics
#[derive(Debug, Clone, Default)]
pub struct HeapStats {
    /// Total heap size
    pub total_size: usize,
    /// Used size
    pub used_size: usize,
    /// Free size
    pub free_size: usize,
    /// Number of allocations
    pub allocations: u64,
    /// Number of deallocations
    pub deallocations: u64,
}

/// Global heap allocator wrapper
pub struct GlobalHeap {
    /// Current allocator
    allocator: RwLock<Option<Arc<dyn HeapAllocator>>>,
}

impl GlobalHeap {
    /// Create a new global heap
    pub const fn new() -> Self {
        Self {
            allocator: RwLock::new(None),
        }
    }

    /// Set the allocator
    pub fn set_allocator(&self, allocator: Arc<dyn HeapAllocator>) {
        *self.allocator.write() = Some(allocator);
    }
}

unsafe impl GlobalAlloc for GlobalHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.allocator.read()
            .as_ref()
            .map(|a| a.allocate(layout))
            .unwrap_or(core::ptr::null_mut())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if let Some(ref allocator) = *self.allocator.read() {
            allocator.deallocate(ptr, layout);
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        self.allocator.read()
            .as_ref()
            .map(|a| a.reallocate(ptr, layout, new_size))
            .unwrap_or(core::ptr::null_mut())
    }
}

/// Simple bump allocator for early boot
pub struct BumpHeap {
    /// Current position
    next: spin::Mutex<usize>,
    /// Heap start
    start: usize,
    /// Heap end
    end: usize,
}

impl BumpHeap {
    /// Create a new bump heap
    pub const fn new() -> Self {
        Self {
            next: spin::Mutex::new(0),
            start: 0,
            end: 0,
        }
    }

    /// Initialize with memory region
    pub fn init(&mut self, start: usize, size: usize) {
        self.start = start;
        self.end = start + size;
        *self.next.lock() = start;
    }
}

impl HeapAllocator for BumpHeap {
    fn allocate(&self, layout: Layout) -> *mut u8 {
        let mut next = self.next.lock();
        
        // Align up
        let aligned = (*next + layout.align() - 1) & !(layout.align() - 1);
        let end = aligned + layout.size();
        
        if end > self.end {
            return core::ptr::null_mut();
        }
        
        *next = end;
        aligned as *mut u8
    }

    fn deallocate(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator doesn't deallocate
    }

    fn name(&self) -> &'static str {
        "Bump Allocator"
    }

    fn stats(&self) -> HeapStats {
        let next = *self.next.lock();
        HeapStats {
            total_size: self.end - self.start,
            used_size: next - self.start,
            free_size: self.end - next,
            allocations: 0,
            deallocations: 0,
        }
    }
}
